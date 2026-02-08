use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use agent_fetch::SafeClient;
use wasmtime::{
    Caller, Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, Trap,
};
use wasmtime_wasi::WasiCtx;
use wasmtime_wasi::p2::pipe::{MemoryInputPipe, MemoryOutputPipe};

use crate::config::SandboxConfig;
use crate::error::{Result, SandboxError};

/// Result of executing a command in the sandbox.
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

/// JSON request sent from WASM guest to host for fetch.
#[derive(serde::Deserialize)]
struct GuestFetchRequest {
    url: String,
    #[serde(default = "default_method")]
    method: String,
    #[serde(default)]
    headers: HashMap<String, String>,
    #[serde(default)]
    body: Option<String>,
}

fn default_method() -> String {
    "GET".to_string()
}

/// JSON response sent from host back to WASM guest.
#[derive(serde::Serialize)]
struct GuestFetchResponse {
    status: u16,
    headers: HashMap<String, String>,
    body: String,
    ok: bool,
    error: Option<String>,
}

/// Store data combining WASI context with resource limits and fetch state.
struct SandboxState {
    wasi: wasmtime_wasi::p1::WasiP1Ctx,
    limits: StoreLimits,
    fetch_client: Option<Arc<SafeClient>>,
    fetch_response: Option<Vec<u8>>,
    tokio_handle: Option<tokio::runtime::Handle>,
}

/// Cached WASM engine and compiled module shared across all Sandbox instances.
struct CachedModule {
    engine: Engine,
    module: Module,
}

/// Global cache for the compiled WASM module.
/// Compiling the toolbox WASM binary is expensive, so we do it once.
static MODULE_CACHE: OnceLock<std::result::Result<CachedModule, String>> = OnceLock::new();

fn get_or_compile_module() -> Result<(&'static Engine, &'static Module)> {
    let cached = MODULE_CACHE.get_or_init(|| {
        let precompiled_bytes = include_bytes!(env!("TOOLBOX_CWASM_PATH"));

        if precompiled_bytes.is_empty() {
            return Err("WASM toolbox not available".to_string());
        }

        // Engine config MUST match build.rs exactly
        let mut engine_config = Config::new();
        engine_config.consume_fuel(true);

        let engine =
            Engine::new(&engine_config).map_err(|e| format!("engine creation failed: {e}"))?;

        // SAFETY: The precompiled bytes come from our own build.rs via
        // Engine::precompile_module() with the same engine config and wasmtime version.
        let module = unsafe { Module::deserialize(&engine, precompiled_bytes) }
            .map_err(|e| format!("module deserialization failed: {e}"))?;

        Ok(CachedModule { engine, module })
    });

    match cached {
        Ok(c) => Ok((&c.engine, &c.module)),
        Err(e) => Err(SandboxError::Other(e.clone())),
    }
}

/// The WASI runtime that manages Wasmtime engine and module compilation.
pub struct WasiRuntime {
    engine: &'static Engine,
    module: &'static Module,
    config: Arc<SandboxConfig>,
    fetch_client: Option<Arc<SafeClient>>,
}

impl WasiRuntime {
    /// Create a new WASI runtime with the given sandbox config.
    /// The toolbox WASM binary is compiled once and cached globally.
    pub fn new(config: SandboxConfig, fetch_client: Option<Arc<SafeClient>>) -> Result<Self> {
        let (engine, module) = get_or_compile_module()?;

        Ok(Self {
            engine,
            module,
            config: Arc::new(config),
            fetch_client,
        })
    }

    /// Execute a command inside the WASM sandbox.
    pub async fn exec(&self, command: &str, args: &[String]) -> Result<ExecResult> {
        let config = self.config.clone();
        let engine = self.engine;
        let module = self.module;
        let command = command.to_string();
        let args = args.to_vec();
        let timeout = config.timeout;
        let fetch_client = self.fetch_client.clone();
        let tokio_handle = tokio::runtime::Handle::current();

        // Run in blocking thread since Wasmtime is synchronous, with a wall-clock timeout
        let task = tokio::task::spawn_blocking(move || {
            exec_sync(
                engine,
                module,
                &config,
                &command,
                &args,
                fetch_client,
                tokio_handle,
            )
        });

        match tokio::time::timeout(timeout, task).await {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => Err(SandboxError::Other(format!("task join error: {}", e))),
            Err(_) => Err(SandboxError::Timeout(timeout)),
        }
    }
}

/// Read a byte slice from WASM guest memory.
fn read_guest_memory(caller: &mut Caller<'_, SandboxState>, ptr: i32, len: i32) -> Option<Vec<u8>> {
    if ptr < 0 || len < 0 {
        return None;
    }
    let memory = caller.get_export("memory")?.into_memory()?;
    let data = memory.data(&*caller);
    let start = ptr as usize;
    let end = start.checked_add(len as usize)?;
    if end > data.len() {
        return None;
    }
    Some(data[start..end].to_vec())
}

/// Write a byte slice into WASM guest memory.
fn write_guest_memory(caller: &mut Caller<'_, SandboxState>, ptr: i32, buf: &[u8]) -> bool {
    if ptr < 0 {
        return false;
    }
    let memory = match caller.get_export("memory") {
        Some(ext) => match ext.into_memory() {
            Some(m) => m,
            None => return false,
        },
        None => return false,
    };
    let data = memory.data_mut(caller);
    let start = ptr as usize;
    let end = match start.checked_add(buf.len()) {
        Some(e) => e,
        None => return false,
    };
    if end > data.len() {
        return false;
    }
    data[start..end].copy_from_slice(buf);
    true
}

fn exec_sync(
    engine: &Engine,
    module: &Module,
    config: &SandboxConfig,
    command: &str,
    args: &[String],
    fetch_client: Option<Arc<SafeClient>>,
    tokio_handle: tokio::runtime::Handle,
) -> Result<ExecResult> {
    // Build argv: [command, ...args]
    let mut argv: Vec<String> = vec![command.to_string()];
    argv.extend(args.iter().cloned());

    let argv_refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();

    // Set up stdout/stderr capture via MemoryOutputPipe
    let stdout_pipe = MemoryOutputPipe::new(1024 * 1024); // 1MB capacity
    let stderr_pipe = MemoryOutputPipe::new(1024 * 1024);

    // Build WASI context using WasiCtx::builder()
    let mut builder = WasiCtx::builder();
    builder.args(&argv_refs);
    builder.stdin(MemoryInputPipe::new(b"" as &[u8])); // Empty stdin — prevents blocking on host stdin
    builder.stdout(stdout_pipe.clone());
    builder.stderr(stderr_pipe.clone());

    // Set TOOLBOX_CMD env var for BusyBox-style dispatch
    builder.env("TOOLBOX_CMD", command);

    // Set user-configured env vars
    for (key, value) in &config.env_vars {
        builder.env(key, value);
    }

    // Mount work directory
    let work_dir = config.work_dir.canonicalize().map_err(|e| {
        SandboxError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("work_dir '{}': {}", config.work_dir.display(), e),
        ))
    })?;

    let dir = wasmtime_wasi::DirPerms::all();
    let file = wasmtime_wasi::FilePerms::all();
    builder.preopened_dir(&work_dir, "/work", dir, file)?;

    // Mount additional directories
    for mount in &config.mounts {
        let host = mount.host_path.canonicalize().map_err(|e| {
            SandboxError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("mount '{}': {}", mount.host_path.display(), e),
            ))
        })?;

        let (d, f) = if mount.writable {
            (
                wasmtime_wasi::DirPerms::all(),
                wasmtime_wasi::FilePerms::all(),
            )
        } else {
            (
                wasmtime_wasi::DirPerms::READ,
                wasmtime_wasi::FilePerms::READ,
            )
        };

        builder.preopened_dir(&host, &mount.guest_path, d, f)?;
    }

    // Build the WASIp1 context
    let wasi_p1 = builder.build_p1();

    // Build memory limiter
    let limits = StoreLimitsBuilder::new()
        .memory_size(config.memory_limit_bytes as usize)
        .build();

    let mut store = Store::new(
        engine,
        SandboxState {
            wasi: wasi_p1,
            limits,
            fetch_client,
            fetch_response: None,
            tokio_handle: Some(tokio_handle),
        },
    );
    store.limiter(|state| &mut state.limits);

    // Set fuel limit
    store.set_fuel(config.fuel_limit)?;

    // Link WASI p1 and instantiate
    let mut linker = Linker::new(engine);
    wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |state: &mut SandboxState| &mut state.wasi)?;

    // Link sandbox host functions for fetch bridge
    linker.func_wrap(
        "sandbox",
        "__sandbox_fetch",
        |mut caller: Caller<'_, SandboxState>, req_ptr: i32, req_len: i32| -> i32 {
            // Read request JSON from guest memory
            let req_bytes = match read_guest_memory(&mut caller, req_ptr, req_len) {
                Some(b) => b,
                None => return -1,
            };

            let guest_req: GuestFetchRequest = match serde_json::from_slice(&req_bytes) {
                Ok(r) => r,
                Err(_) => return -1,
            };

            let client = match caller.data().fetch_client.as_ref() {
                Some(c) => c.clone(),
                None => {
                    // Networking disabled — store error response
                    let resp = GuestFetchResponse {
                        status: 0,
                        headers: HashMap::new(),
                        body: String::new(),
                        ok: false,
                        error: Some("networking disabled: configure fetch_policy to enable".into()),
                    };
                    caller.data_mut().fetch_response = Some(serde_json::to_vec(&resp).unwrap());
                    return -2;
                }
            };

            let handle = match caller.data().tokio_handle.as_ref() {
                Some(h) => h.clone(),
                None => return -1,
            };

            let fetch_req = agent_fetch::FetchRequest {
                url: guest_req.url,
                method: guest_req.method,
                headers: guest_req.headers,
                body: guest_req.body.map(|s| s.into_bytes()),
            };

            // Bridge async fetch to sync context via the tokio handle
            let result = std::thread::scope(|_| handle.block_on(client.fetch(fetch_req)));

            let resp = match result {
                Ok(r) => GuestFetchResponse {
                    status: r.status,
                    headers: r.headers,
                    body: String::from_utf8_lossy(&r.body).to_string(),
                    ok: (200..300).contains(&(r.status as u32)),
                    error: None,
                },
                Err(e) => GuestFetchResponse {
                    status: 0,
                    headers: HashMap::new(),
                    body: String::new(),
                    ok: false,
                    error: Some(e.to_string()),
                },
            };

            caller.data_mut().fetch_response = Some(serde_json::to_vec(&resp).unwrap());
            0
        },
    )?;

    linker.func_wrap(
        "sandbox",
        "__sandbox_fetch_response_len",
        |caller: Caller<'_, SandboxState>| -> i32 {
            caller
                .data()
                .fetch_response
                .as_ref()
                .map(|r| r.len() as i32)
                .unwrap_or(0)
        },
    )?;

    linker.func_wrap(
        "sandbox",
        "__sandbox_fetch_response_read",
        |mut caller: Caller<'_, SandboxState>, buf_ptr: i32, buf_len: i32| -> i32 {
            if buf_ptr < 0 || buf_len < 0 {
                return -1;
            }
            let resp = match caller.data().fetch_response.as_ref() {
                Some(r) => r.clone(),
                None => return -1,
            };
            let copy_len = std::cmp::min(resp.len(), buf_len as usize);
            if write_guest_memory(&mut caller, buf_ptr, &resp[..copy_len]) {
                copy_len as i32
            } else {
                -1
            }
        },
    )?;

    linker.module(&mut store, "", module)?;

    // Get the default function (_start) and call it
    let func = linker
        .get_default(&mut store, "")?
        .typed::<(), ()>(&store)?;

    let exit_code = match func.call(&mut store, ()) {
        Ok(()) => 0,
        Err(e) => {
            // Check if it's a normal process exit
            if let Some(exit) = e.downcast_ref::<wasmtime_wasi::I32Exit>() {
                exit.0
            } else if e.downcast_ref::<Trap>() == Some(&Trap::OutOfFuel) {
                return Err(SandboxError::Timeout(config.timeout));
            } else {
                return Err(SandboxError::Runtime(e));
            }
        }
    };

    let stdout_bytes = stdout_pipe.contents().to_vec();
    let stderr_bytes = stderr_pipe.contents().to_vec();

    Ok(ExecResult {
        exit_code,
        stdout: stdout_bytes,
        stderr: stderr_bytes,
    })
}
