# agent-sandbox

A secure, WASM-based sandboxed execution environment for AI agents. Runs untrusted code in an isolated Wasmtime/WASI runtime with controlled filesystem access and 40+ built-in CLI tools.

## Features

- Isolated command execution via embedded WASM toolbox (cat, grep, find, sed, git, jq, and more)
- Filesystem sandboxing with path traversal prevention
- Configurable resource limits (fuel, timeout, memory)
- Change tracking (diff) via filesystem snapshots
- Node.js bindings via NAPI

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

await sandbox.writeFile("hello.txt", Buffer.from("Hello from sandbox!"));
const content = await sandbox.readFile("hello.txt");

const changes = await sandbox.diff();
console.log("Changed files:", changes);

await sandbox.destroy();
```

## License

MIT
