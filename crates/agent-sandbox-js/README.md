# @parassharmaa/agent-sandbox

Node.js bindings for [agent-sandbox](https://github.com/Parassharmaa/agent-sandbox) — a secure, WASM-based sandbox for AI agents with 40+ built-in CLI tools and a JavaScript runtime.

## Installation

```bash
npm install @parassharmaa/agent-sandbox
```

Prebuilt binaries are available for:
- macOS (arm64, x64)
- Linux (x64 gnu, arm64 gnu)
- Windows (x64 msvc)

## Usage

```js
import { Sandbox } from "@parassharmaa/agent-sandbox";

// Create a sandbox with a working directory
const sandbox = new Sandbox({ workDir: "/path/to/workdir" });

// Execute commands (files are accessible under /work/)
const result = await sandbox.exec("grep", ["TODO", "/work/main.rs"]);
console.log(result.stdout.toString()); // stdout/stderr are Buffers
console.log(result.exitCode);          // 0 on success

// Read and write files (paths relative to workDir)
const content = await sandbox.readFile("config.json");
await sandbox.writeFile("output.txt", Buffer.from("hello"));

// List directory contents
const entries = await sandbox.listDir(".");
// [{ name: "file.txt", isFile: true, isDir: false, size: 42 }, ...]

// Track filesystem changes since sandbox creation
const changes = await sandbox.diff();
// [{ path: "output.txt", kind: "created" }, ...]

// Clean up
await sandbox.destroy();
```

### JavaScript Runtime

Execute JavaScript code inside the WASM sandbox using the built-in Boa engine:

```js
// Inline evaluation
const result = await sandbox.exec("node", ["-e", "console.log('hello from JS')"]);

// Evaluate and print result
const calc = await sandbox.exec("node", ["-p", "2 + 3 * 4"]);
console.log(calc.stdout.toString().trim()); // "14"

// Run a script file
await sandbox.writeFile("script.js", Buffer.from(`
  const data = [1, 2, 3, 4, 5];
  const sum = data.reduce((a, b) => a + b, 0);
  console.log(JSON.stringify({ sum, avg: sum / data.length }));
`));
await sandbox.exec("node", ["/work/script.js"]);

// Convenience method — execJs(code) wraps exec("node", ["-e", code])
const jsResult = await sandbox.execJs("console.log('quick and easy')");
```

Supports ES2023+ features (arrow functions, destructuring, template literals, Promises, JSON, Math, RegExp, Array methods, etc.). No network access or Node.js built-in modules — runs in pure WASM isolation.

### Configuration Options

```js
const sandbox = new Sandbox({
  workDir: "/path/to/workdir",      // Required: host directory mounted at /work
  mounts: [{                         // Optional: additional mount points
    hostPath: "/data",
    guestPath: "/mnt/data",
    writable: false,
  }],
  envVars: { API_KEY: "value" },     // Optional: environment variables
  timeoutMs: 30000,                  // Optional: execution timeout (ms)
  memoryLimitBytes: 256 * 1024 * 1024, // Optional: memory limit
  fuelLimit: 1_000_000_000,         // Optional: WASM fuel limit
});
```

### List Available Tools

```js
const tools = Sandbox.availableTools();
// ["cat", "grep", "find", "sed", "jq", "git", "node", "tar", ...]
```

## Security

- Commands run inside a WASM sandbox — no access to host filesystem outside the work directory
- Path traversal attacks are blocked
- Environment variables are isolated from the host
- Each sandbox instance is fully isolated from others
- JS runtime runs inside WASM — no network, no host access

## License

MIT
