# agent-sandbox

A secure, embeddable, WASM-based sandbox for AI agents. 40+ built-in CLI tools, <13ms startup, no Docker/VMs required.

## Quick Start

### Rust

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

### Node.js

```js
import { Sandbox } from "@parassharmaa/agent-sandbox";

const sandbox = new Sandbox({ workDir: "/path/to/workdir" });
const result = await sandbox.exec("grep", ["TODO", "/work/main.rs"]);
console.log(result.stdout.toString());

const changes = await sandbox.diff();
await sandbox.destroy();
```

## Features

- 40+ tools: cat, grep, find, sed, awk, jq, git, tar, zip, and more
- Filesystem sandboxing with path traversal prevention
- Resource limits: fuel, timeout, memory
- Change tracking via filesystem snapshots
- AOT precompiled WASM — <13ms cold start, ~55µs cached
- Node.js bindings (NAPI)

## Limitations

- No network access (WASI p1)
- No process spawning or shell pipes
- Built-in tools only — can't run Python/Node/Ruby
- Single-threaded execution
- Same-architecture precompiled binary

**Best for:** file-manipulation agents (code analysis, refactoring, git ops).
**Not for:** arbitrary script execution, API calls, databases, GPU.

## License

MIT
