# agent-sandbox

A secure, embeddable, WASM-based sandbox for AI agents. 80+ built-in CLI tools, a full shell interpreter, a JavaScript runtime, safe HTTP networking, <13ms startup, no Docker/VMs required.

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

- 80+ tools: cat, grep, find, sed, awk, jq, git, tar, zip, curl, seq, md5sum, and more
- **Full shell interpreter** (`sh`/`bash`) with pipes, redirections, variables, loops, functions, and command substitution
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

## Shell Interpreter

The sandbox includes a full shell interpreter accessible via `sh` or `bash`. It supports most common shell constructs, all running entirely inside the WASM sandbox.

### Pipes and Redirections

```js
// Pipes
await sandbox.exec("sh", ["-c", "echo hello world | wc -w"]);           // stdout: "2"
await sandbox.exec("sh", ["-c", "cat /work/data.csv | sort | uniq -c"]);

// Output redirection
await sandbox.exec("sh", ["-c", "echo hello > /work/out.txt"]);
await sandbox.exec("sh", ["-c", "echo world >> /work/out.txt"]);         // append

// Input redirection
await sandbox.exec("sh", ["-c", "wc -l < /work/data.txt"]);
```

### Variables and Expansion

```js
// Variable assignment and expansion
await sandbox.exec("sh", ["-c", "NAME=world; echo hello $NAME"]);        // "hello world"

// Default values
await sandbox.exec("sh", ["-c", 'echo ${UNSET:-fallback}']);              // "fallback"
await sandbox.exec("sh", ["-c", 'X=; echo ${X:=default}; echo $X']);     // "default\ndefault"

// String length
await sandbox.exec("sh", ["-c", 'S=hello; echo ${#S}']);                  // "5"

// Command substitution
await sandbox.exec("sh", ["-c", "echo today is $(date)"]);
await sandbox.exec("sh", ["-c", "FILES=$(ls /work); echo $FILES"]);

// Arithmetic
await sandbox.exec("sh", ["-c", "echo $((3 + 4 * 2))"]);                 // "11"

// Special variables
await sandbox.exec("sh", ["-c", "echo exit=$? args=$# pid=$$"]);
```

### Control Flow

```js
// if/elif/else
await sandbox.exec("sh", ["-c", `
  if [ -f /work/config.json ]; then
    echo "config exists"
  else
    echo "no config"
  fi
`]);

// for loops
await sandbox.exec("sh", ["-c", `
  for f in /work/*.txt; do
    echo "processing $f"
    wc -l "$f"
  done
`]);

// while loops
await sandbox.exec("sh", ["-c", `
  i=1
  while [ $i -le 5 ]; do
    echo $i
    i=$((i + 1))
  done
`]);

// case statements
await sandbox.exec("sh", ["-c", `
  EXT=.rs
  case $EXT in
    .rs)  echo "Rust" ;;
    .js)  echo "JavaScript" ;;
    .py)  echo "Python" ;;
    *)    echo "Unknown" ;;
  esac
`]);
```

### Functions

```js
await sandbox.exec("sh", ["-c", `
  greet() {
    echo "Hello, $1!"
  }
  greet World
  greet Agent
`]);
// stdout: "Hello, World!\nHello, Agent!"
```

### Command Chaining

```js
// AND chain — runs second only if first succeeds
await sandbox.exec("sh", ["-c", "mkdir -p /work/out && echo 'created'"]);

// OR chain — runs second only if first fails
await sandbox.exec("sh", ["-c", "cat /work/missing.txt 2>/dev/null || echo 'not found'"]);

// Semicolons — always runs both
await sandbox.exec("sh", ["-c", "echo one; echo two"]);
```

### Real-World Agent Examples

```js
// Install dependencies and run tests (typical CI pattern)
await sandbox.exec("sh", ["-c", `
  cd /work && \
  cat package.json | jq -r '.dependencies | keys[]' | sort > deps.txt && \
  echo "Found $(wc -l < deps.txt) dependencies"
`]);

// Process data pipeline
await sandbox.exec("sh", ["-c", `
  for file in /work/logs/*.log; do
    grep ERROR "$file" | wc -l
  done | awk '{sum+=$1} END {print "Total errors:", sum}'
`]);

// Generate a report
await sandbox.exec("sh", ["-c", `
  echo "# File Report" > /work/report.md
  echo "" >> /work/report.md
  for f in /work/src/*.rs; do
    lines=$(wc -l < "$f")
    name=$(basename "$f")
    echo "- $name: $lines lines" >> /work/report.md
  done
`]);
```

## Available Commands

**Text Processing:** cat, head, tail, grep, rg, sed, sort, uniq, cut, tr, wc, awk, tac, rev, nl, paste, comm, join, fold, column, expand, unexpand, strings, od

**Search & Files:** find, tree, ls, mkdir, cp, mv, rm, du, ln, stat, touch, tee, readlink, rmdir, split, file

**Data & Hashing:** jq, diff, patch, base64, sha256sum, sha1sum, md5sum, xxd

**Archives:** tar, gzip, zip

**Code & Version Control:** git

**Shell & Utilities:** sh, bash, echo, printf, env, xargs, basename, dirname, seq, sleep, which, whoami, hostname, printenv, date, expr, true, false, test, [

**Networking:** curl (intercepted through safe client)

**JavaScript Runtime:** node

## Limitations

- JS runtime has no Node.js built-in modules (fs, http, etc.) — `fetch()` is the only network API
- Single-threaded execution (pipes run sequentially via temp files)
- Shell does not support job control, signal handling, or process spawning
- Same-architecture precompiled binary

**Best for:** file-manipulation agents (code analysis, refactoring, git ops), sandboxed JS evaluation, safe API calls, multi-step shell scripts.
**Not for:** databases, GPU, long-running servers.

## License

MIT
