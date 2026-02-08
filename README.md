# agent-sandbox

A secure, embeddable, WASM-based sandbox for AI agents. 40+ built-in CLI tools, a JavaScript runtime, <13ms startup, no Docker/VMs required.

## Installation

### Rust

```bash
cargo add agent-sandbox
```

### Node.js

```bash
npm install @parassharmaa/agent-sandbox
```

Prebuilt binaries are available for macOS (arm64, x64), Linux (x64, arm64), and Windows (x64).

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

// Execute JavaScript inside the sandbox
let js_result = sandbox.exec_js("console.log('Hello from JS!')").await?;
```

### Node.js

```js
import { Sandbox } from "@parassharmaa/agent-sandbox";

const sandbox = new Sandbox({ workDir: "/path/to/workdir" });
const result = await sandbox.exec("grep", ["TODO", "/work/main.rs"]);
console.log(result.stdout.toString());

// Execute JavaScript inside the sandbox
const jsResult = await sandbox.execJs("console.log('Hello from JS!')");

const changes = await sandbox.diff();
await sandbox.destroy();
```

## Features

- 40+ tools: cat, grep, find, sed, awk, jq, git, tar, zip, and more
- Built-in JavaScript runtime (Boa engine) via `node` command or `execJs()` API
- Filesystem sandboxing with path traversal prevention
- Resource limits: fuel, timeout, memory
- Change tracking via filesystem snapshots
- AOT precompiled WASM — <13ms cold start, ~55us cached
- Node.js bindings (NAPI)

## JavaScript Runtime

The sandbox includes a built-in JavaScript engine (Boa) that runs entirely inside the WASM sandbox. Use it via the `node` command or the `execJs()` convenience method.

```js
// Inline evaluation
await sandbox.exec("node", ["-e", "console.log('hello')"]);

// Evaluate and print result
await sandbox.exec("node", ["-p", "2 + 3 * 4"]); // stdout: "14"

// Run a script file (from /work)
await sandbox.writeFile("script.js", Buffer.from(`
  const data = [1, 2, 3, 4, 5];
  console.log(JSON.stringify({ sum: data.reduce((a, b) => a + b) }));
`));
await sandbox.exec("node", ["/work/script.js"]);

// Convenience method
await sandbox.execJs("console.log('quick and easy')");
```

Supported JS features: ES2023+ (variables, arrow functions, destructuring, template literals, Promises, Map/Set, JSON, Math, RegExp, Array methods, and more). No network access or Node.js built-in modules — runs in pure WASM isolation.

## Limitations

- No network access (WASI p1)
- No process spawning or shell pipes
- JS runtime has no Node.js built-in modules (fs, http, etc.)
- Single-threaded execution
- Same-architecture precompiled binary

**Best for:** file-manipulation agents (code analysis, refactoring, git ops), sandboxed JS evaluation.
**Not for:** arbitrary network requests, API calls, databases, GPU.

## License

MIT
