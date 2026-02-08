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

**`crates/agent-sandbox/`** — Core Rust library. The `Sandbox` struct is the public API (`new`, `exec`, `exec_js`, `read_file`, `write_file`, `list_dir`, `diff`, `destroy`). Key modules:
- `runtime/mod.rs` — Wasmtime engine with global module cache (`OnceLock`), fuel limits, wall-clock timeout via `tokio::time::timeout`, and `spawn_blocking` for sync WASM execution.
- `fs/capability.rs` — Path validation preventing traversal attacks. All host-side file ops go through `validate_path()`.
- `fs/overlay.rs` — SHA-256 snapshot diffing to detect created/modified/deleted files.
- `toolbox/mod.rs` — Allowlist of available commands checked before any `exec()`.

**`wasm/toolbox/`** — BusyBox-style multi-call WASM binary (target: `wasm32-wasip1`). Single entry point dispatches to 40+ tool implementations via `TOOLBOX_CMD` env var. Includes a built-in JavaScript runtime (`node` command) powered by Boa engine. Compiled once and embedded into the core crate via `include_bytes!(env!("TOOLBOX_WASM_PATH"))` in `build.rs`.

**`crates/agent-sandbox-js/`** — NAPI bindings exposing `Sandbox` class to Node.js. Async methods, cross-platform builds (darwin-arm64/x64, linux-x64-gnu/arm64-gnu, win32-x64-msvc). Tests use AVA + tsx.

### Execution flow

```
JS/Rust caller → Sandbox.exec(cmd, args)
  → toolbox::is_available(cmd) check
  → WasiRuntime.exec() → spawn_blocking + tokio::time::timeout
    → fresh WASI context per call (stdin=empty, stdout/stderr=MemoryPipe)
    → mount work_dir at /work, set TOOLBOX_CMD env var
    → call _start on cached WASM module
    → return ExecResult { exit_code, stdout, stderr }
```

The WASM module is compiled once globally and reused. Each `exec()` creates a new `Store` + `Linker` + WASI context — no state leaks between calls.

## Commit Guidelines

- Keep commit messages short and precise (1 line summary, optional body)
- Do not include email addresses or `Co-Authored-By` lines in commit messages
- Use imperative mood: "fix timeout bug" not "fixed timeout bug"
