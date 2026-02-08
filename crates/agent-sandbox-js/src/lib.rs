use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use agent_sandbox::config::{MountPoint as RustMountPoint, SandboxConfig as RustSandboxConfig};
use agent_sandbox::fs::overlay::FsChangeKind;
use napi::bindgen_prelude::*;
use napi_derive::napi;

#[napi(object)]
pub struct SandboxOptions {
    pub work_dir: String,
    pub mounts: Option<Vec<MountPointOption>>,
    pub env_vars: Option<HashMap<String, String>>,
    pub timeout_ms: Option<f64>,
    pub memory_limit_bytes: Option<f64>,
    pub fuel_limit: Option<f64>,
}

#[napi(object)]
pub struct MountPointOption {
    pub host_path: String,
    pub guest_path: String,
    pub writable: Option<bool>,
}

#[napi(object)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: Buffer,
    pub stderr: Buffer,
}

#[napi(object)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub size: f64,
}

#[napi(object)]
pub struct FsChange {
    pub path: String,
    pub kind: String,
}

#[napi]
pub struct Sandbox {
    inner: agent_sandbox::Sandbox,
}

#[napi]
impl Sandbox {
    /// Returns a list of all available tool commands in the sandbox.
    #[napi]
    pub fn available_tools() -> Vec<String> {
        agent_sandbox::toolbox::AVAILABLE_TOOLS
            .iter()
            .map(|s| s.to_string())
            .collect()
    }

    #[napi(constructor)]
    pub fn new(options: SandboxOptions) -> Result<Self> {
        let config = RustSandboxConfig {
            work_dir: PathBuf::from(&options.work_dir),
            mounts: options
                .mounts
                .unwrap_or_default()
                .into_iter()
                .map(|m| RustMountPoint {
                    host_path: PathBuf::from(m.host_path),
                    guest_path: m.guest_path,
                    writable: m.writable.unwrap_or(false),
                })
                .collect(),
            env_vars: options.env_vars.unwrap_or_default(),
            timeout: Duration::from_millis(options.timeout_ms.unwrap_or(30000.0) as u64),
            memory_limit_bytes: options
                .memory_limit_bytes
                .unwrap_or(512.0 * 1024.0 * 1024.0) as u64,
            fuel_limit: options.fuel_limit.unwrap_or(1_000_000_000.0) as u64,
        };

        let inner =
            agent_sandbox::Sandbox::new(config).map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(Self { inner })
    }

    #[napi]
    pub async fn exec(&self, command: String, args: Vec<String>) -> Result<ExecResult> {
        let result = self
            .inner
            .exec(&command, &args)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(ExecResult {
            exit_code: result.exit_code,
            stdout: Buffer::from(result.stdout),
            stderr: Buffer::from(result.stderr),
        })
    }

    /// Execute JavaScript code inside the sandbox using the built-in JS engine.
    #[napi]
    pub async fn exec_js(&self, code: String) -> Result<ExecResult> {
        let result = self
            .inner
            .exec_js(&code)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(ExecResult {
            exit_code: result.exit_code,
            stdout: Buffer::from(result.stdout),
            stderr: Buffer::from(result.stderr),
        })
    }

    #[napi]
    pub async fn read_file(&self, path: String) -> Result<Buffer> {
        let content = self
            .inner
            .read_file(&path)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(Buffer::from(content))
    }

    #[napi]
    pub async fn write_file(&self, path: String, contents: Buffer) -> Result<()> {
        self.inner
            .write_file(&path, &contents)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }

    #[napi]
    pub async fn list_dir(&self, path: String) -> Result<Vec<DirEntry>> {
        let entries = self
            .inner
            .list_dir(&path)
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(entries
            .into_iter()
            .map(|e| DirEntry {
                name: e.name,
                is_dir: e.is_dir,
                is_file: e.is_file,
                size: e.size as f64,
            })
            .collect())
    }

    #[napi]
    pub async fn diff(&self) -> Result<Vec<FsChange>> {
        let changes = self
            .inner
            .diff()
            .await
            .map_err(|e| Error::from_reason(e.to_string()))?;

        Ok(changes
            .into_iter()
            .map(|c| FsChange {
                path: c.path,
                kind: match c.kind {
                    FsChangeKind::Created => "created".to_string(),
                    FsChangeKind::Modified => "modified".to_string(),
                    FsChangeKind::Deleted => "deleted".to_string(),
                },
            })
            .collect())
    }

    #[napi]
    pub async fn destroy(&self) -> Result<()> {
        self.inner
            .destroy()
            .await
            .map_err(|e| Error::from_reason(e.to_string()))
    }
}
