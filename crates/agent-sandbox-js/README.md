# @parassharmaa/agent-sandbox

Node.js bindings for [agent-sandbox](https://github.com/Parassharmaa/agent-sandbox) — a secure, WASM-based sandbox for AI agents with 40+ built-in CLI tools.

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
// ["cat", "grep", "find", "sed", "awk", "jq", "git", "tar", ...]
```

## Security

- Commands run inside a WASM sandbox — no access to host filesystem outside the work directory
- Path traversal attacks are blocked
- Environment variables are isolated from the host
- Each sandbox instance is fully isolated from others

## License

MIT
