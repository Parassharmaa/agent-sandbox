use std::sync::{Arc, OnceLock};

use wasmtime::{Config, Engine, Linker, Module, Store, StoreLimits, StoreLimitsBuilder, Trap};
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

/// Store data combining WASI context with resource limits.
struct SandboxState {
    wasi: wasmtime_wasi::p1::WasiP1Ctx,
    limits: StoreLimits,
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
        let wasm_bytes = include_bytes!(env!("TOOLBOX_WASM_PATH"));

        if wasm_bytes.is_empty() {
            return Err("WASM toolbox not available".to_string());
        }

        let mut engine_config = Config::new();
        engine_config.consume_fuel(true);

        let engine =
            Engine::new(&engine_config).map_err(|e| format!("engine creation failed: {e}"))?;
        let module = Module::new(&engine, wasm_bytes)
            .map_err(|e| format!("module compilation failed: {e}"))?;

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
}

impl WasiRuntime {
    /// Create a new WASI runtime with the given sandbox config.
    /// The toolbox WASM binary is compiled once and cached globally.
    pub fn new(config: SandboxConfig) -> Result<Self> {
        let (engine, module) = get_or_compile_module()?;

        Ok(Self {
            engine,
            module,
            config: Arc::new(config),
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

        // Run in blocking thread since Wasmtime is synchronous, with a wall-clock timeout
        let task = tokio::task::spawn_blocking(move || {
            exec_sync(engine, module, &config, &command, &args)
        });

        match tokio::time::timeout(timeout, task).await {
            Ok(Ok(result)) => result,
            Ok(Err(e)) => Err(SandboxError::Other(format!("task join error: {}", e))),
            Err(_) => Err(SandboxError::Timeout(timeout)),
        }
    }
}

fn exec_sync(
    engine: &Engine,
    module: &Module,
    config: &SandboxConfig,
    command: &str,
    args: &[String],
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
        },
    );
    store.limiter(|state| &mut state.limits);

    // Set fuel limit
    store.set_fuel(config.fuel_limit)?;

    // Link WASI p1 and instantiate
    let mut linker = Linker::new(engine);
    wasmtime_wasi::p1::add_to_linker_sync(&mut linker, |state: &mut SandboxState| &mut state.wasi)?;

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
