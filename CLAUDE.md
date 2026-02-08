# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
# Build the WASM toolbox (must be done first — embedded into the core crate at compile time)
cargo build --target wasm32-wasip1 --release --manifest-path wasm/toolbox/Cargo.toml

# Build the core sandbox library
cargo build -p agent-sandbox

# Build JS/NAPI bindings (requires npm install first)
cd crates/agent-sandbox-js && npm install && npm run build:debug
```

## Test Commands

```bash
# All Rust tests (unit + integration)
cargo test -p agent-sandbox

# Integration tests only
cargo test -p agent-sandbox --test integration

# Single Rust test by name
cargo test -p agent-sandbox --test integration test_exec_echo

# JS tests (from crates/agent-sandbox-js/)
npm test

# Single JS test by title match
npx ava --match '*symlink*'
```

## Lint & Format

```bash
cargo +nightly fmt --all              # format
cargo +nightly fmt --all -- --check   # check only
cargo +stable clippy --workspace -- -D warnings
```

Rustfmt config: `imports_granularity = "Module"`, `group_imports = "StdExternalCrate"`.

Workspace lints: `deprecated = "deny"`, `unused_imports = "warn"`, `dead_code = "warn"`.

## Architecture

This is a WASM-based sandbox that executes commands in an isolated Wasmtime/WASI environment. Three main components:

**`crates/agent-sandbox/`** — Core Rust library. The `Sandbox` struct is the public API (`new`, `exec`, `exec_js`, `fetch`, `read_file`, `write_file`, `list_dir`, `diff`, `destroy`). Key modules:
- `runtime/mod.rs` — Wasmtime engine with global module cache (`OnceLock`), fuel limits, wall-clock timeout via `tokio::time::timeout`, `spawn_blocking` for sync WASM execution, and host functions for the WASM↔host fetch bridge (`__sandbox_fetch`, `__sandbox_fetch_response_len`, `__sandbox_fetch_response_read`).
- `fs/capability.rs` — Path validation preventing traversal attacks. All host-side file ops go through `validate_path()`.
- `fs/overlay.rs` — SHA-256 snapshot diffing to detect created/modified/deleted files.
- `toolbox/mod.rs` — Allowlist of available commands checked before any `exec()`.
- `lib.rs` — `Sandbox` struct, `fetch()` method, `curl` command interception routed through `agent-fetch::SafeClient`.

**`wasm/toolbox/`** — BusyBox-style multi-call WASM binary (target: `wasm32-wasip1`). Single entry point dispatches to 40+ tool implementations via `TOOLBOX_CMD` env var. Includes a built-in JavaScript runtime (`node` command) powered by Boa engine with `fetch()` global. `fetch.rs` provides the guest-side bridge to host functions. Compiled once and embedded into the core crate via `include_bytes!(env!("TOOLBOX_WASM_PATH"))` in `build.rs`.

**`crates/agent-sandbox-js/`** — NAPI bindings exposing `Sandbox` class to Node.js. Async methods, cross-platform builds (darwin-arm64/x64, linux-x64-gnu/arm64-gnu, win32-x64-msvc). Tests use AVA + tsx, split by category: `basic.spec.ts`, `exec.spec.ts`, `security.spec.ts`, `node.spec.ts`, `fetch.spec.ts`, with shared helpers in `helpers.ts`.

**`agent-fetch`** — External crate ([crates.io](https://crates.io/crates/agent-fetch)) providing `SafeClient` for SSRF-protected HTTP. Used by the sandbox for `fetch()`, `curl` interception, and JS `fetch()` global. Configured via `FetchPolicy` (domain allow/blocklists, private IP blocking, rate limiting, timeouts).

### Execution flow

```
JS/Rust caller → Sandbox.exec(cmd, args)
  → if cmd == "curl": parse args → SafeClient.fetch() → ExecResult
  → toolbox::is_available(cmd) check
  → WasiRuntime.exec() → spawn_blocking + tokio::time::timeout
    → fresh WASI context per call (stdin=empty, stdout/stderr=MemoryPipe)
    → mount work_dir at /work, set TOOLBOX_CMD env var
    → link "sandbox" host functions (fetch bridge)
    → call _start on cached WASM module
    → return ExecResult { exit_code, stdout, stderr }
```

```
JS fetch() inside Boa → wasm/toolbox/src/fetch.rs
  → __sandbox_fetch(req_ptr, req_len) host import
  → host reads JSON request from WASM memory
  → SafeClient.fetch() via tokio Handle::block_on
  → response stored in SandboxState
  → guest reads via __sandbox_fetch_response_len/read
  → Boa returns { status, ok, body, headers, text(), json() }
```

The WASM module is compiled once globally and reused. Each `exec()` creates a new `Store` + `Linker` + WASI context — no state leaks between calls.

## Commit Guidelines

- Keep commit messages short and precise (1 line summary, optional body)
- Do not include email addresses or `Co-Authored-By` lines in commit messages
- Use imperative mood: "fix timeout bug" not "fixed timeout bug"
