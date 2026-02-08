use thiserror::Error;

#[derive(Error, Debug)]
pub enum SandboxError {
    #[error("WASM runtime error: {0}")]
    Runtime(#[from] wasmtime::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("path traversal blocked: {0}")]
    PathTraversal(String),

    #[error("command not found: {0}")]
    CommandNotFound(String),

    #[error("execution timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("sandbox destroyed")]
    Destroyed,

    #[error(
        "WASM toolbox not available — build with: cargo build --target wasm32-wasip1 -p agent-toolbox --release"
    )]
    ToolboxNotAvailable,

    #[error("networking disabled: configure fetch_policy to enable")]
    NetworkingDisabled,

    #[error("fetch error: {0}")]
    Fetch(String),

    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, SandboxError>;
