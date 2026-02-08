# agent-sandbox

A secure, embeddable, WASM-based sandbox for AI agents. 40+ built-in CLI tools, a JavaScript runtime, safe HTTP networking, <13ms startup, no Docker/VMs required.

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
use agent_sandbox::{Sandbox, config::SandboxConfig, FetchPolicy};

let config = SandboxConfig {
    work_dir: "/path/to/workdir".into(),
    fetch_policy: Some(FetchPolicy::default()), // enable networking
    ..Default::default()
};
let sandbox = Sandbox::new(config)?;

let result = sandbox.exec("grep", &["TODO".into(), "/work/main.rs".into()]).await?;
println!("{}", String::from_utf8_lossy(&result.stdout));

// Execute JavaScript inside the sandbox
let js_result = sandbox.exec_js("console.log('Hello from JS!')").await?;

// HTTP fetch with SSRF protection
let response = sandbox.fetch(FetchRequest {
    url: "https://api.example.com/data".into(),
    method: "GET".into(),
    headers: Default::default(),
    body: None,
}).await?;
println!("Status: {}", response.status);
```

### Node.js

```js
import { Sandbox } from "@parassharmaa/agent-sandbox";

const sandbox = new Sandbox({
  workDir: "/path/to/workdir",
  fetchPolicy: { denyPrivateIps: true },
});

const result = await sandbox.exec("grep", ["TODO", "/work/main.rs"]);
console.log(result.stdout.toString());

// Execute JavaScript inside the sandbox
const jsResult = await sandbox.execJs("console.log('Hello from JS!')");

// HTTP fetch
const response = await sandbox.fetch({ url: "https://api.example.com/data" });
console.log(response.status, response.body.toString());

// curl interception — routed through the safe client
const curlResult = await sandbox.exec("curl", ["https://api.example.com/data"]);

const changes = await sandbox.diff();
await sandbox.destroy();
```

## Features

- 40+ tools: cat, grep, find, sed, awk, jq, git, tar, zip, curl, and more
- Built-in JavaScript runtime (Boa engine) via `node` command or `execJs()` API
- Safe HTTP networking with SSRF protection, domain policies, and rate limiting
- `fetch()` available in JS runtime, as a direct API, and via `curl` command interception
- Filesystem sandboxing with path traversal prevention
- Resource limits: fuel, timeout, memory
- Change tracking via filesystem snapshots
- AOT precompiled WASM — <13ms cold start, ~55us cached
- Node.js bindings (NAPI)

## Networking

The sandbox provides safe HTTP access via [agent-fetch](https://crates.io/crates/agent-fetch), with built-in SSRF protection, domain allowlists/blocklists, DNS rebinding prevention, and rate limiting.

Networking is **disabled by default**. Enable it by providing a `fetchPolicy`:

```js
const sandbox = new Sandbox({
  workDir: "/tmp/work",
  fetchPolicy: {
    allowedDomains: ["api.example.com", "*.github.com"], // optional allowlist
    blockedDomains: ["evil.com"],                        // optional blocklist
    denyPrivateIps: true,                                // block 127.0.0.1, 10.x, etc.
    requestTimeoutMs: 30000,
    maxRedirects: 10,
  },
});
```

Three ways to make HTTP requests:

```js
// 1. Direct fetch API
const res = await sandbox.fetch({
  url: "https://api.example.com/data",
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: Buffer.from(JSON.stringify({ key: "value" })),
});

// 2. curl command (intercepted and routed through the safe client)
await sandbox.exec("curl", ["-X", "POST", "-d", '{"key":"value"}', "https://api.example.com"]);

// 3. fetch() inside the JS runtime
await sandbox.execJs(`
  var r = fetch('https://api.example.com/data');
  console.log(r.status, r.text());
`);
```

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

// fetch() is available when networking is enabled
await sandbox.execJs("var r = fetch('https://example.com'); console.log(r.ok)");
```

Supported JS features: ES2023+ (variables, arrow functions, destructuring, template literals, Promises, Map/Set, JSON, Math, RegExp, Array methods, and more).

## Limitations

- No process spawning or shell pipes
- JS runtime has no Node.js built-in modules (fs, http, etc.) — `fetch()` is the only network API
- Single-threaded execution
- Same-architecture precompiled binary

**Best for:** file-manipulation agents (code analysis, refactoring, git ops), sandboxed JS evaluation, safe API calls.
**Not for:** databases, GPU, long-running servers.

## License

MIT
