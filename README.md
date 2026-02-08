# agent-sandbox

A secure, WASM-based sandboxed execution environment for AI agents. Runs untrusted code in an isolated Wasmtime/WASI runtime with controlled filesystem access and 40+ built-in CLI tools.

## Features

- Isolated command execution via embedded WASM toolbox (cat, grep, find, sed, git, jq, and more)
- Filesystem sandboxing with path traversal prevention
- Configurable resource limits (fuel, timeout, memory)
- Change tracking (diff) via filesystem snapshots
- Node.js bindings via NAPI

## Quick Start

```rust
use agent_sandbox::{Sandbox, config::SandboxConfig};

let config = SandboxConfig {
    work_dir: "/path/to/workdir".into(),
    ..Default::default()
};
let sandbox = Sandbox::new(config)?;

let result = sandbox.exec("grep", &["TODO".into(), "/work/main.rs".into()]).await?;
println!("{}", String::from_utf8_lossy(&result.stdout));
```

## License

MIT
