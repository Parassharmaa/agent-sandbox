use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

pub use agent_fetch::{DomainPattern, FetchPolicy};
use serde::{Deserialize, Serialize};

/// Configuration for creating a sandbox instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Host directory to expose as `/work` inside the sandbox.
    pub work_dir: PathBuf,

    /// Additional mount points beyond the work directory.
    #[serde(default)]
    pub mounts: Vec<MountPoint>,

    /// Environment variables to set inside the sandbox.
    #[serde(default)]
    pub env_vars: HashMap<String, String>,

    /// Maximum execution time per command (default: 30s).
    #[serde(default = "default_timeout")]
    pub timeout: Duration,

    /// Maximum memory in bytes the WASM instance can use (default: 512MB).
    #[serde(default = "default_memory_limit")]
    pub memory_limit_bytes: u64,

    /// Fuel limit for execution (higher = more compute allowed, default: 1 billion).
    #[serde(default = "default_fuel_limit")]
    pub fuel_limit: u64,

    /// Fetch policy for HTTP networking. `None` disables all networking (default).
    #[serde(default)]
    pub fetch_policy: Option<FetchPolicy>,
}

/// A directory mount point mapping host path to guest path.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountPoint {
    /// Path on the host filesystem.
    pub host_path: PathBuf,

    /// Path inside the sandbox (e.g., `/data`).
    pub guest_path: String,

    /// Whether the sandbox can write to this mount.
    #[serde(default)]
    pub writable: bool,
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_memory_limit() -> u64 {
    512 * 1024 * 1024 // 512 MB
}

fn default_fuel_limit() -> u64 {
    1_000_000_000 // 1 billion instructions
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            work_dir: PathBuf::from("."),
            mounts: Vec::new(),
            env_vars: HashMap::new(),
            timeout: default_timeout(),
            memory_limit_bytes: default_memory_limit(),
            fuel_limit: default_fuel_limit(),
            fetch_policy: None,
        }
    }
}
