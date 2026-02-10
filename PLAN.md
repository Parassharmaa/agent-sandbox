# Plan: Full Shell Experience + Missing Commands

## Context

Comparing agent-sandbox (40 commands, single-command execution only) with just-bash (97 commands, full shell interpreter), two major gaps exist:

1. **No shell interpreter** — agent-sandbox can only run one command at a time via `exec(cmd, args)`. No pipes, redirections, chaining, variables, loops, or conditionals.
2. **~25 missing commands** commonly needed by AI agents.

The user wants a "full shell-like experience" — a `sh`/`bash` command inside the WASM toolbox that parses and executes shell scripts.

## Implementation Plan

### Part 1: Shell Interpreter (`wasm/toolbox/src/shell/`)

A hand-written shell interpreter inside the WASM toolbox. No external parser libraries — recursive descent parser keeps the WASM binary small.

**Architecture**: `sh -c "script"` → Lexer → Parser → AST → Executor → dispatch existing tools

**Modules to create** (in implementation order):

| # | File | Purpose | ~Lines |
|---|------|---------|--------|
| 1 | `shell/ast.rs` | AST node types (Program, Pipeline, Command, Word, Redirect, If/For/While/Case) | ~200 |
| 2 | `shell/token.rs` | Token enum (Word, Pipe, And, Or, Semi, redirections, keywords) | ~80 |
| 3 | `shell/env.rs` | Shell variables, exports, positional params, functions, `$?`/`$#`/`$$` | ~150 |
| 4 | `shell/lexer.rs` | Tokenizer with quoting (single/double), `$(...)`, backticks, here-docs | ~400 |
| 5 | `shell/parser.rs` | Recursive descent: program → complete_command → pipeline → command | ~500 |
| 6 | `shell/expand.rs` | Variable expansion, `${VAR:-default}`, command substitution, glob expansion | ~300 |
| 7 | `shell/builtins.rs` | cd, export, unset, set, read, exit, test/[, true, false, :, shift, local, source | ~250 |
| 8 | `shell/redirect.rs` | Fd save/restore using WASI `fd_renumber` for `>`, `>>`, `<`, `2>&1` | ~150 |
| 9 | `shell/pipeline.rs` | Multi-stage pipe execution using temp files as buffers between stages | ~120 |
| 10 | `shell/exec.rs` | AST walker: dispatches to builtins, functions, or toolbox `dispatch()` | ~350 |
| 11 | `shell/mod.rs` | Public `run(args)` for `sh -c "..."` and `sh script.sh` | ~60 |

**Key design decisions**:
- **Pipes via temp files**: Each pipe stage writes stdout to `/work/.sh_pipe_N`, next stage reads it as stdin. Cleaned up after each pipeline. No tool signature changes needed.
- **Fd swapping via WASI `fd_renumber`**: Save original fd to high slot, redirect to file, restore after command. Requires `wasi` crate dependency.
- **Sequential pipeline execution**: WASI p1 is single-threaded, so pipe stages run sequentially (not concurrent). Fuel limits prevent infinite producers like `yes`.
- **Subshell isolation**: Clone `ShellEnv` before `(...)` subshells, restore after.
- **`dispatch()` reuse**: The shell calls the existing `dispatch(cmd, args)` function from `main.rs` — all 40+ tools work immediately.

**Shell features supported**:
- Pipes: `|`
- Redirections: `>`, `>>`, `<`, `2>`, `2>&1`
- Command chaining: `&&`, `||`, `;`
- Variables: `$VAR`, `${VAR}`, `${VAR:-default}`, `${VAR:=default}`, `${#VAR}`
- Command substitution: `$(cmd)` and backticks
- Positional params: `$1`, `$@`, `$#`, `$?`
- Glob expansion: `*`, `?`, `[...]`
- Conditionals: `if/then/elif/else/fi`
- Loops: `for`, `while`, `until`, `break`, `continue`
- Functions: `name() { ... }` and `function name { ... }`
- Case: `case/in/esac`
- Here-documents: `<<EOF`
- Quoting: single quotes, double quotes, backslash escapes

### Part 2: Missing Commands (~25 new tools)

Each tool follows the existing pattern: `pub fn run(args: &[String]) -> i32` in `wasm/toolbox/src/tools/`.

**Text Processing** (high priority for AI agents):
| Command | Description | Complexity |
|---------|-------------|------------|
| `awk` | Pattern scanning and text processing | High |
| `tac` | Reverse cat (print lines in reverse) | Low |
| `rev` | Reverse each line | Low |
| `nl` | Number lines | Low |
| `paste` | Merge lines from files | Low |
| `comm` | Compare two sorted files | Low |
| `join` | Join lines on common field | Medium |
| `fold` | Wrap lines to specified width | Low |
| `column` | Columnate output | Medium |
| `expand` | Convert tabs to spaces | Low |
| `unexpand` | Convert spaces to tabs | Low |
| `strings` | Extract printable strings from binary | Low |
| `od` | Octal/hex dump | Medium |

**Hashing**:
| Command | Description | Complexity |
|---------|-------------|------------|
| `md5sum` | MD5 hash (sha1_smol pattern, add md5 crate) | Low |
| `sha1sum` | SHA-1 hash (already have sha1_smol dep) | Low |

**File Operations**:
| Command | Description | Complexity |
|---------|-------------|------------|
| `readlink` | Print resolved symlink target | Low |
| `rmdir` | Remove empty directories | Low |
| `split` | Split file into pieces | Medium |
| `file` | Detect file type by content/magic bytes | Medium |

**Shell Utilities**:
| Command | Description | Complexity |
|---------|-------------|------------|
| `seq` | Print number sequence | Low |
| `sleep` | Delay execution (simulated in WASM) | Low |
| `date` | Display date/time | Medium |
| `expr` | Evaluate expressions | Medium |
| `which` | Locate a command in toolbox | Low |
| `whoami` | Print current user | Low |
| `hostname` | Print hostname | Low |
| `printenv` | Print environment variables | Low |

### Part 3: Integration Changes

**Files to modify**:
- `wasm/toolbox/src/main.rs` — Add new commands to `dispatch()`, make `dispatch` function `pub` so shell module can call it
- `wasm/toolbox/src/tools/mod.rs` — Add `pub mod` for all new tool modules + shell
- `wasm/toolbox/Cargo.toml` — Add `wasi = "0.13"` (for fd_renumber), `md5` crate
- `crates/agent-sandbox/src/toolbox/mod.rs` — Add all new commands + `sh`/`bash` to `AVAILABLE_TOOLS`

### Part 4: Tests

**Rust integration tests** (`crates/agent-sandbox/tests/integration.rs`):
- Shell basics: `sh -c "echo hello"`, exit codes
- Pipes: `sh -c "echo hello world | wc -w"`, multi-stage pipes
- Redirections: `sh -c "echo test > /work/out.txt && cat /work/out.txt"`
- Variables: `sh -c 'X=42; echo $X'`, `${VAR:-default}`
- Command substitution: `sh -c 'echo $(echo inner)'`
- Conditionals: `sh -c 'if true; then echo yes; fi'`
- Loops: `sh -c 'for i in a b c; do echo $i; done'`
- Functions: `sh -c 'f() { echo hi; }; f'`
- Chaining: `sh -c 'true && echo yes'`, `sh -c 'false || echo fallback'`
- Error handling: syntax errors, unknown commands
- Each new command (awk, tac, rev, seq, etc.)

**JS tests** (`crates/agent-sandbox-js/__test__/shell.spec.ts`):
- Mirror the Rust tests via the JS Sandbox API
- Test `exec('sh', ['-c', '...'])` patterns

**Unit tests** (inside WASM toolbox modules with `#[cfg(test)]`):
- Lexer: token sequences for various inputs
- Parser: AST structure verification
- Expansion: variable substitution, glob matching

## Implementation Order

1. **New simple commands first** (tac, rev, nl, seq, sleep, which, whoami, hostname, printenv, readlink, rmdir, expand, unexpand, paste, comm, fold, md5sum, sha1sum) — quick wins
2. **Shell interpreter core** (ast → token → env → lexer → parser → expand → builtins → redirect → pipeline → exec → mod) — the main effort
3. **Medium complexity commands** (awk, column, od, split, file, date, expr, join, strings)
4. **Integration + allowlist updates**
5. **Tests for everything**

## Verification

```bash
# Build WASM toolbox
cargo build --target wasm32-wasip1 --release --manifest-path wasm/toolbox/Cargo.toml

# Build core
cargo build -p agent-sandbox

# Run all Rust tests
cargo test -p agent-sandbox

# Build JS bindings and run JS tests
cd crates/agent-sandbox-js && npm run build:debug && npm test

# Specific shell test
cargo test -p agent-sandbox --test integration test_shell

# Format and lint
cargo +nightly fmt --all
cargo +stable clippy --workspace -- -D warnings
```
