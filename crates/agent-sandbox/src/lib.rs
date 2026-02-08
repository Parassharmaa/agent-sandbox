pub mod config;
pub mod error;
pub mod exec;
pub mod fs;
pub mod runtime;
pub mod toolbox;

use std::collections::HashMap;
use std::sync::Arc;

use agent_fetch::SafeClient;
pub use agent_fetch::{DomainPattern, FetchPolicy, FetchRequest, FetchResponse};
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
    fetch_client: Option<Arc<SafeClient>>,
}

impl Sandbox {
    /// Create a new sandbox with the given configuration.
    pub fn new(config: SandboxConfig) -> Result<Self> {
        let overlay = FsOverlay::new(&config.work_dir)?;

        let fetch_client = config
            .fetch_policy
            .as_ref()
            .map(|policy| Arc::new(SafeClient::new(policy.clone())));

        let runtime = WasiRuntime::new(config.clone(), fetch_client.clone())?;

        Ok(Self {
            runtime,
            overlay: Arc::new(Mutex::new(Some(overlay))),
            config,
            destroyed: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            fetch_client,
        })
    }

    /// Execute a command inside the sandbox.
    pub async fn exec(&self, command: &str, args: &[String]) -> Result<ExecResult> {
        self.check_destroyed()?;

        // Intercept curl commands and route through fetch
        if command == "curl" {
            return self.exec_curl(args).await;
        }

        if !toolbox::is_available(command) {
            return Err(SandboxError::CommandNotFound(command.to_string()));
        }

        self.runtime.exec(command, args).await
    }

    /// Execute JavaScript code inside the sandbox using the built-in JS engine.
    pub async fn exec_js(&self, code: &str) -> Result<ExecResult> {
        self.exec("node", &["-e".to_string(), code.to_string()])
            .await
    }

    /// Perform an HTTP fetch using the sandbox's safe client.
    pub async fn fetch(&self, request: FetchRequest) -> Result<FetchResponse> {
        self.check_destroyed()?;

        let client = self
            .fetch_client
            .as_ref()
            .ok_or(SandboxError::NetworkingDisabled)?;

        client
            .fetch(request)
            .await
            .map_err(|e| SandboxError::Fetch(e.to_string()))
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

    /// Intercept `curl` commands and route through the fetch client.
    async fn exec_curl(&self, args: &[String]) -> Result<ExecResult> {
        let client = self
            .fetch_client
            .as_ref()
            .ok_or(SandboxError::NetworkingDisabled)?;

        let (request, output_file) = parse_curl_args(args)?;

        match client.fetch(request).await {
            Ok(resp) => {
                let body = resp.body.clone();

                // If -o was specified, write to file
                if let Some(out_path) = output_file {
                    let full_path = fs::validate_path(&self.config.work_dir, &out_path)?;
                    if let Some(parent) = full_path.parent() {
                        tokio::fs::create_dir_all(parent).await?;
                    }
                    tokio::fs::write(&full_path, &body).await?;
                }

                let status_line = format!("HTTP {}\n", resp.status);
                Ok(ExecResult {
                    exit_code: 0,
                    stdout: body,
                    stderr: status_line.into_bytes(),
                })
            }
            Err(e) => {
                let err_msg = format!("curl: {}\n", e);
                Ok(ExecResult {
                    exit_code: 1,
                    stdout: Vec::new(),
                    stderr: err_msg.into_bytes(),
                })
            }
        }
    }
}

/// Parse curl-like arguments into a FetchRequest.
fn parse_curl_args(args: &[String]) -> Result<(FetchRequest, Option<String>)> {
    let mut url = None;
    let mut method = "GET".to_string();
    let mut headers = HashMap::new();
    let mut body: Option<Vec<u8>> = None;
    let mut output_file = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-X" | "--request" => {
                i += 1;
                if i < args.len() {
                    method = args[i].clone();
                }
            }
            "-H" | "--header" => {
                i += 1;
                if i < args.len()
                    && let Some((k, v)) = args[i].split_once(':')
                {
                    headers.insert(k.trim().to_string(), v.trim().to_string());
                }
            }
            "-d" | "--data" => {
                i += 1;
                if i < args.len() {
                    body = Some(args[i].as_bytes().to_vec());
                    if method == "GET" {
                        method = "POST".to_string();
                    }
                }
            }
            "-o" | "--output" => {
                i += 1;
                if i < args.len() {
                    output_file = Some(args[i].clone());
                }
            }
            "-s" | "--silent" | "-S" | "--show-error" | "-L" | "--location" | "-f" | "--fail"
            | "-v" | "--verbose" | "-k" | "--insecure" | "-I" | "--head" | "-N" | "--no-buffer"
            | "-g" | "--globoff" => {
                // Silently ignore common boolean flags
            }
            "--max-time" | "--connect-timeout" | "--retry" | "--retry-delay" | "--max-redirs"
            | "-u" | "--user" | "-A" | "--user-agent" | "-e" | "--referer" | "-w"
            | "--write-out" | "--max-filesize" => {
                // Unknown flags that take a value — skip the next argument
                i += 1;
            }
            arg if !arg.starts_with('-') && url.is_none() => {
                url = Some(arg.to_string());
            }
            arg if arg.starts_with('-') => {
                // Unknown flag — ignore (may cause issues with value-taking flags)
            }
            _ => {
                // Non-flag argument after URL — ignore
            }
        }
        i += 1;
    }

    let url = url.ok_or_else(|| SandboxError::Other("curl: no URL specified".into()))?;

    Ok((
        FetchRequest {
            url,
            method,
            headers,
            body,
        },
        output_file,
    ))
}

/// A directory entry returned by `list_dir`.
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: String,
    pub is_dir: bool,
    pub is_file: bool,
    pub size: u64,
}
