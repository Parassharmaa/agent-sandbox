pub mod config;
pub mod error;
pub mod exec;
pub mod fs;
pub mod runtime;
pub mod toolbox;

use std::sync::Arc;

use tokio::sync::Mutex;

use crate::config::SandboxConfig;
use crate::error::{Result, SandboxError};
use crate::fs::overlay::{FsChange, FsOverlay};
use crate::runtime::{ExecResult, WasiRuntime};

/// A sandboxed execution environment backed by WASM (Wasmtime + WASI).
pub struct Sandbox {
    runtime: WasiRuntime,
    overlay: Arc<Mutex<Option<FsOverlay>>>,
    config: SandboxConfig,
    destroyed: Arc<std::sync::atomic::AtomicBool>,
}

impl Sandbox {
    /// Create a new sandbox with the given configuration.
    pub fn new(config: SandboxConfig) -> Result<Self> {
        let overlay = FsOverlay::new(&config.work_dir)?;
        let runtime = WasiRuntime::new(config.clone())?;

        Ok(Self {
            runtime,
            overlay: Arc::new(Mutex::new(Some(overlay))),
            config,
            destroyed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        })
    }

    /// Execute a command inside the sandbox.
    pub async fn exec(&self, command: &str, args: &[String]) -> Result<ExecResult> {
        self.check_destroyed()?;

        if !toolbox::is_available(command) {
            return Err(SandboxError::CommandNotFound(command.to_string()));
        }

        self.runtime.exec(command, args).await
    }

    /// Read a file from the sandbox's work directory.
    pub async fn read_file(&self, path: &str) -> Result<Vec<u8>> {
        self.check_destroyed()?;

        let full_path = fs::validate_path(&self.config.work_dir, path)?;
        let content = tokio::fs::read(&full_path).await?;
        Ok(content)
    }

    /// Write a file to the sandbox's work directory.
    pub async fn write_file(&self, path: &str, contents: &[u8]) -> Result<()> {
        self.check_destroyed()?;

        let full_path = fs::validate_path(&self.config.work_dir, path)?;

        // Ensure parent directory exists
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        tokio::fs::write(&full_path, contents).await?;
        Ok(())
    }

    /// List entries in a directory within the sandbox.
    pub async fn list_dir(&self, path: &str) -> Result<Vec<DirEntry>> {
        self.check_destroyed()?;

        let full_path = fs::validate_path(&self.config.work_dir, path)?;
        let mut entries = Vec::new();

        let mut rd = tokio::fs::read_dir(&full_path).await?;
        while let Some(entry) = rd.next_entry().await? {
            let metadata = entry.metadata().await?;
            entries.push(DirEntry {
                name: entry.file_name().to_string_lossy().to_string(),
                is_dir: metadata.is_dir(),
                is_file: metadata.is_file(),
                size: metadata.len(),
            });
        }

        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    /// Get filesystem changes since the sandbox was created.
    pub async fn diff(&self) -> Result<Vec<FsChange>> {
        self.check_destroyed()?;

        let overlay = self.overlay.lock().await;
        match overlay.as_ref() {
            Some(o) => o.diff(),
            None => Err(SandboxError::Destroyed),
        }
    }

    /// Destroy the sandbox, cleaning up temporary resources.
    pub async fn destroy(&self) -> Result<()> {
        self.destroyed
            .store(true, std::sync::atomic::Ordering::SeqCst);
        let mut overlay = self.overlay.lock().await;
        *overlay = None;
        Ok(())
    }

    fn check_destroyed(&self) -> Result<()> {
        if self.destroyed.load(std::sync::atomic::Ordering::SeqCst) {
            Err(SandboxError::Destroyed)
        } else {
            Ok(())
        }
    }
}

/// A directory entry returned by `list_dir`.
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub size: u64,
}
