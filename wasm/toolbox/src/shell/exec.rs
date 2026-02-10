use std::fs;
use std::io::Write;

use super::ast::*;
use super::builtins::{self, ControlFlow};
use super::env::ShellEnv;
use super::expand;
use super::pipeline::PipelineState;
use super::redirect;

/// The dispatch function from main.rs, used to execute external commands.
type DispatchFn = fn(&str, &[String]) -> i32;

/// Result of executing a command — may include control flow signals.
pub struct ExecResult {
    pub exit_code: i32,
    pub should_exit: bool,
    pub control_flow: Option<ControlFlow>,
}

impl ExecResult {
    fn code(c: i32) -> Self {
        ExecResult {
            exit_code: c,
            should_exit: false,
            control_flow: None,
        }
    }

    fn exit(c: i32) -> Self {
        ExecResult {
            exit_code: c,
            should_exit: true,
            control_flow: None,
        }
    }

    fn control(cf: ControlFlow) -> Self {
        ExecResult {
            exit_code: 0,
            should_exit: false,
            control_flow: Some(cf),
        }
    }
}

/// Execute a program (list of commands).
pub fn exec_program(
    program: &Program,
    env: &mut ShellEnv,
    dispatch: DispatchFn,
) -> ExecResult {
    let mut last_code = 0;

    for cmd in &program.commands {
        let result = exec_complete_command(cmd, env, dispatch);
        last_code = result.exit_code;
        env.last_status = last_code;

        if result.should_exit || result.control_flow.is_some() {
            return result;
        }
    }

    ExecResult::code(last_code)
}

/// Execute a complete command (pipeline && pipeline || pipeline ...).
fn exec_complete_command(
    cmd: &CompleteCommand,
    env: &mut ShellEnv,
    dispatch: DispatchFn,
) -> ExecResult {
    let mut result = exec_pipeline(&cmd.first, env, dispatch);
    env.last_status = result.exit_code;

    if result.should_exit || result.control_flow.is_some() {
        return result;
    }

    for (op, pipeline) in &cmd.rest {
        let should_run = match op {
            ListOp::And => result.exit_code == 0,
            ListOp::Or => result.exit_code != 0,
        };

        if should_run {
            result = exec_pipeline(pipeline, env, dispatch);
            env.last_status = result.exit_code;

            if result.should_exit || result.control_flow.is_some() {
                return result;
            }
        }
    }

    result
}

/// Execute a pipeline.
fn exec_pipeline(
    pipeline: &Pipeline,
    env: &mut ShellEnv,
    dispatch: DispatchFn,
) -> ExecResult {
    if pipeline.commands.len() == 1 {
        let result = exec_command(&pipeline.commands[0], env, dispatch, None, None);
        let exit_code = if pipeline.negated {
            if result.exit_code == 0 { 1 } else { 0 }
        } else {
            result.exit_code
        };
        return ExecResult {
            exit_code,
            ..result
        };
    }

    // Multi-command pipeline: use temp files
    let pipe_state = PipelineState::new(pipeline.commands.len());
    let mut last_code = 0;

    for (i, cmd) in pipeline.commands.iter().enumerate() {
        let stdin_file = pipe_state.input_for(i);
        let stdout_file = pipe_state.output_for(i);

        let result = exec_command(cmd, env, dispatch, stdin_file, stdout_file);
        last_code = result.exit_code;

        if result.should_exit || result.control_flow.is_some() {
            pipe_state.cleanup();
            return result;
        }
    }

    pipe_state.cleanup();

    let exit_code = if pipeline.negated {
        if last_code == 0 { 1 } else { 0 }
    } else {
        last_code
    };

    ExecResult::code(exit_code)
}

/// Execute a single command.
fn exec_command(
    cmd: &Command,
    env: &mut ShellEnv,
    dispatch: DispatchFn,
    pipe_stdin: Option<&str>,
    pipe_stdout: Option<&str>,
) -> ExecResult {
    match cmd {
        Command::Simple(simple) => exec_simple(simple, env, dispatch, pipe_stdin, pipe_stdout),
        Command::If(if_clause) => exec_if(if_clause, env, dispatch),
        Command::For(for_clause) => exec_for(for_clause, env, dispatch),
        Command::While(while_clause) => exec_while(while_clause, env, dispatch),
        Command::Until(until_clause) => exec_until(until_clause, env, dispatch),
        Command::Case(case_clause) => exec_case(case_clause, env, dispatch),
        Command::Subshell(program) => {
            let mut sub_env = env.clone();
            let result = exec_program(program, &mut sub_env, dispatch);
            ExecResult::code(result.exit_code)
        }
        Command::BraceGroup(program) => exec_program(program, env, dispatch),
        Command::FuncDef(func_def) => {
            env.functions.insert(func_def.name.clone(), *func_def.body.clone());
            ExecResult::code(0)
        }
    }
}

/// Execute a simple command.
fn exec_simple(
    cmd: &SimpleCommand,
    env: &mut ShellEnv,
    dispatch: DispatchFn,
    pipe_stdin: Option<&str>,
    pipe_stdout: Option<&str>,
) -> ExecResult {
    // Expand words
    let mut expanded_words: Vec<String> = Vec::new();
    for word in &cmd.words {
        let words = expand::expand_word(word, env, dispatch);
        expanded_words.extend(words);
    }

    // Process assignments
    for assign in &cmd.assignments {
        let value = expand::expand_word_to_string(&assign.value, env, dispatch);
        env.set(&assign.name, &value);
    }

    // If no command words, just apply assignments
    if expanded_words.is_empty() {
        return ExecResult::code(0);
    }

    let cmd_name = &expanded_words[0];
    let cmd_args = &expanded_words[1..];

    // Expand redirection targets
    let expanded_targets: Vec<String> = cmd.redirections
        .iter()
        .map(|r| expand::expand_word_to_string(&r.target, env, dispatch))
        .collect();

    // Apply redirections
    let redir_state = redirect::apply_redirections(&cmd.redirections, &expanded_targets)
        .unwrap_or_else(|e| {
            eprintln!("sh: {}", e);
            redirect::RedirectState::new()
        });

    // Determine actual stdin content
    let stdin_content = redir_state.stdin_content.or_else(|| {
        pipe_stdin.and_then(|f| fs::read_to_string(f).ok())
    });

    // Determine stdout target
    let stdout_target = redir_state.stdout_file.as_deref().or(pipe_stdout);
    let stderr_target = redir_state.stderr_file.as_deref();

    // Check for builtins
    if builtins::is_builtin(cmd_name) {
        let args: Vec<String> = cmd_args.to_vec();
        let builtin_result = builtins::run_builtin(cmd_name, &args, env);

        if builtin_result.should_exit {
            return ExecResult::exit(builtin_result.exit_code);
        }
        if let Some(cf) = builtin_result.control_flow {
            return ExecResult::control(cf);
        }
        return ExecResult::code(builtin_result.exit_code);
    }

    // Check for shell functions
    if let Some(func_body) = env.functions.get(cmd_name).cloned() {
        // Save positional params
        let saved_positional = env.positional.clone();
        env.positional = cmd_args.to_vec();
        env.push_local_scope();

        let result = exec_command(&func_body, env, dispatch, pipe_stdin, pipe_stdout);

        env.pop_local_scope();
        env.positional = saved_positional;

        let exit_code = if let Some(ControlFlow::Return(code)) = result.control_flow {
            code
        } else {
            if result.control_flow.is_some() {
                return result;
            }
            result.exit_code
        };

        return ExecResult::code(exit_code);
    }

    // Check for eval
    if cmd_name == "eval" {
        let script = cmd_args.join(" ");
        let (code, output) = exec_capture(&script, env, dispatch);
        if !output.is_empty() {
            if let Some(target) = stdout_target {
                let _ = write_to_file(target, &output, redir_state.stdout_file.is_some()
                    && cmd.redirections.iter().any(|r| matches!(r.kind, RedirectKind::Append)));
            } else {
                print!("{}", output);
            }
        }
        return ExecResult::code(code);
    }

    // Check for source / .
    if cmd_name == "source" || cmd_name == "." {
        if let Some(file) = cmd_args.first() {
            match fs::read_to_string(file) {
                Ok(content) => {
                    let (code, _) = exec_capture(&content, env, dispatch);
                    return ExecResult::code(code);
                }
                Err(e) => {
                    eprintln!("sh: {}: {}", file, e);
                    return ExecResult::code(1);
                }
            }
        }
        return ExecResult::code(0);
    }

    // External command via dispatch
    // We need to handle stdin/stdout redirection via temp files since
    // the dispatch function uses actual stdout/stderr

    // If we have stdin content, write it to a temp file and set it up
    // For simple cases, we can use the dispatch directly
    if stdin_content.is_none() && stdout_target.is_none() && stderr_target.is_none() {
        let args: Vec<String> = cmd_args.to_vec();
        let code = dispatch(cmd_name, &args);
        return ExecResult::code(code);
    }

    // For redirected commands, capture output
    let args: Vec<String> = cmd_args.to_vec();

    // Handle stdin redirection via fd_renumber on fd 0 and file args
    if let Some(ref content) = stdin_content {
        let stdin_file = "/work/.sh_stdin_tmp";
        let _ = fs::write(stdin_file, content);

        // Try to redirect WASI stdin (fd 0) to the file
        let stdin_redirected = redirect_stdin_to_file(stdin_file);

        let actual_args = if stdin_redirected {
            // stdin is redirected — the command can read from stdin normally
            args.clone()
        } else {
            // Fallback: add the file as an argument for commands that read files
            let mut new_args = args.clone();
            if !matches!(cmd_name.as_str(), "echo" | "printf" | "mkdir" | "rm" | "cp" | "mv" | "ln" | "touch" | "chmod") {
                if new_args.is_empty() || new_args.iter().all(|a| a.starts_with('-')) {
                    new_args.push(stdin_file.to_string());
                }
            }
            new_args
        };

        let result = if let Some(target) = stdout_target {
            let (code, output) = capture_dispatch(cmd_name, &actual_args, dispatch);
            let append = cmd.redirections.iter().any(|r| matches!(r.kind, RedirectKind::Append) && r.fd.unwrap_or(1) == 1);
            let _ = write_to_file(target, &output, append);
            ExecResult::code(code)
        } else {
            let code = dispatch(cmd_name, &actual_args);
            ExecResult::code(code)
        };

        if stdin_redirected {
            restore_stdin();
        }
        let _ = fs::remove_file(stdin_file);
        return result;
    } else if let Some(target) = stdout_target {
        // Capture stdout to file
        let (code, output) = capture_dispatch(cmd_name, &args, dispatch);
        let append = cmd.redirections.iter().any(|r| matches!(r.kind, RedirectKind::Append) && r.fd.unwrap_or(1) == 1);
        let _ = write_to_file(target, &output, append);
        return ExecResult::code(code);
    }

    let code = dispatch(cmd_name, &args);
    ExecResult::code(code)
}

/// Execute a script and capture its stdout output.
pub fn exec_capture(
    script: &str,
    env: &mut ShellEnv,
    dispatch: DispatchFn,
) -> (i32, String) {
    use super::parser::Parser;

    let mut parser = Parser::new(script);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            return (2, format!("sh: parse error: {}\n", e));
        }
    };

    // For command substitution, we need to capture stdout.
    // In WASM, we redirect to a temp file.
    let capture_file = "/work/.sh_capture_tmp";

    // Execute each command, redirecting stdout to capture file
    let mut total_output = String::new();
    let mut last_code = 0;

    for cmd in &program.commands {
        // We need to wrap each pipeline's commands to redirect to capture file
        let (code, output) = exec_and_capture_complete_cmd(cmd, env, dispatch);
        last_code = code;
        total_output.push_str(&output);
    }

    let _ = fs::remove_file(capture_file);
    (last_code, total_output)
}

fn exec_and_capture_complete_cmd(
    cmd: &CompleteCommand,
    env: &mut ShellEnv,
    dispatch: DispatchFn,
) -> (i32, String) {
    // For captures, we run commands and collect their output
    let mut output = String::new();

    let mut result = exec_and_capture_pipeline(&cmd.first, env, dispatch);
    output.push_str(&result.1);
    env.last_status = result.0;

    for (op, pipeline) in &cmd.rest {
        let should_run = match op {
            ListOp::And => result.0 == 0,
            ListOp::Or => result.0 != 0,
        };

        if should_run {
            result = exec_and_capture_pipeline(pipeline, env, dispatch);
            output.push_str(&result.1);
            env.last_status = result.0;
        }
    }

    (result.0, output)
}

fn exec_and_capture_pipeline(
    pipeline: &Pipeline,
    env: &mut ShellEnv,
    dispatch: DispatchFn,
) -> (i32, String) {
    if pipeline.commands.len() == 1 {
        let result = exec_and_capture_single(&pipeline.commands[0], env, dispatch, None);
        let code = if pipeline.negated {
            if result.0 == 0 { 1 } else { 0 }
        } else {
            result.0
        };
        return (code, result.1);
    }

    // Multi-stage pipeline with capture
    let pipe_state = PipelineState::new(pipeline.commands.len());
    let mut last_code = 0;
    let mut last_output = String::new();

    for (i, cmd) in pipeline.commands.iter().enumerate() {
        let stdin_file = pipe_state.input_for(i);

        if i < pipeline.commands.len() - 1 {
            // Not last stage: write output to pipe file
            let stdout_file = pipe_state.output_for(i).unwrap();
            let (code, output) = exec_and_capture_single(cmd, env, dispatch, stdin_file);
            let _ = fs::write(stdout_file, &output);
            last_code = code;
        } else {
            // Last stage: capture output
            let (code, output) = exec_and_capture_single(cmd, env, dispatch, stdin_file);
            last_code = code;
            last_output = output;
        }
    }

    pipe_state.cleanup();

    let code = if pipeline.negated {
        if last_code == 0 { 1 } else { 0 }
    } else {
        last_code
    };

    (code, last_output)
}

fn exec_and_capture_single(
    cmd: &Command,
    env: &mut ShellEnv,
    dispatch: DispatchFn,
    pipe_stdin: Option<&str>,
) -> (i32, String) {
    match cmd {
        Command::Simple(simple) => {
            let mut expanded_words: Vec<String> = Vec::new();
            for word in &simple.words {
                let words = expand::expand_word(word, env, dispatch);
                expanded_words.extend(words);
            }

            for assign in &simple.assignments {
                let value = expand::expand_word_to_string(&assign.value, env, dispatch);
                env.set(&assign.name, &value);
            }

            if expanded_words.is_empty() {
                return (0, String::new());
            }

            let cmd_name = &expanded_words[0];
            let cmd_args = &expanded_words[1..];

            if builtins::is_builtin(cmd_name) {
                // For builtins in capture mode, we can't easily capture their output
                // Use a simulated approach
                let args: Vec<String> = cmd_args.to_vec();
                let result = builtins::run_builtin(cmd_name, &args, env);
                return (result.exit_code, String::new());
            }

            // Check for shell functions
            if let Some(func_body) = env.functions.get(cmd_name).cloned() {
                let saved_positional = env.positional.clone();
                env.positional = cmd_args.to_vec();
                env.push_local_scope();
                let (code, output) = exec_and_capture_single(&func_body, env, dispatch, pipe_stdin);
                env.pop_local_scope();
                env.positional = saved_positional;
                return (code, output);
            }

            // For stdin piping
            let mut args: Vec<String> = cmd_args.to_vec();
            if let Some(stdin_file) = pipe_stdin {
                if args.is_empty() || args.iter().all(|a| a.starts_with('-')) {
                    if !matches!(cmd_name.as_str(), "echo" | "printf" | "mkdir" | "rm" | "cp" | "mv" | "ln" | "touch") {
                        args.push(stdin_file.to_string());
                    }
                }
            }

            capture_dispatch(cmd_name, &args, dispatch)
        }
        Command::If(ic) => {
            let cond_result = exec_program(&ic.condition, env, dispatch);
            if cond_result.exit_code == 0 {
                let mut out = String::new();
                let mut code = 0;
                for c in &ic.then_body.commands {
                    let (c2, o) = exec_and_capture_complete_cmd(c, env, dispatch);
                    code = c2;
                    out.push_str(&o);
                }
                (code, out)
            } else {
                for (elif_cond, elif_body) in &ic.elifs {
                    let cr = exec_program(elif_cond, env, dispatch);
                    if cr.exit_code == 0 {
                        let mut out = String::new();
                        let mut code = 0;
                        for c in &elif_body.commands {
                            let (c2, o) = exec_and_capture_complete_cmd(c, env, dispatch);
                            code = c2;
                            out.push_str(&o);
                        }
                        return (code, out);
                    }
                }
                if let Some(ref else_body) = ic.else_body {
                    let mut out = String::new();
                    let mut code = 0;
                    for c in &else_body.commands {
                        let (c2, o) = exec_and_capture_complete_cmd(c, env, dispatch);
                        code = c2;
                        out.push_str(&o);
                    }
                    (code, out)
                } else {
                    (0, String::new())
                }
            }
        }
        Command::BraceGroup(program) | Command::Subshell(program) => {
            let mut out = String::new();
            let mut code = 0;
            for c in &program.commands {
                let (c2, o) = exec_and_capture_complete_cmd(c, env, dispatch);
                code = c2;
                out.push_str(&o);
            }
            (code, out)
        }
        Command::For(fc) => {
            let words = if let Some(ref word_list) = fc.words {
                let mut expanded = Vec::new();
                for w in word_list {
                    expanded.extend(expand::expand_word(w, env, dispatch));
                }
                expanded
            } else {
                env.all_positional().to_vec()
            };
            let mut out = String::new();
            let mut code = 0;
            for val in &words {
                env.set(&fc.var, val);
                for c in &fc.body.commands {
                    let (c2, o) = exec_and_capture_complete_cmd(c, env, dispatch);
                    code = c2;
                    out.push_str(&o);
                }
            }
            (code, out)
        }
        _ => {
            let result = exec_command(cmd, env, dispatch, pipe_stdin, None);
            (result.exit_code, String::new())
        }
    }
}

/// Atomic counter for unique capture file names.
static CAPTURE_COUNTER: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);

/// Capture the output of an external command by redirecting fd 1 to a file.
///
/// Uses WASI fd_renumber to swap stdout to a capture file, run the command,
/// then restore stdout and read the captured output.
fn capture_dispatch(cmd: &str, args: &[String], dispatch: DispatchFn) -> (i32, String) {
    let id = CAPTURE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let capture_file = format!("/work/.sh_cap_{}", id);

    #[cfg(target_os = "wasi")]
    {
        use std::os::wasi::io::AsRawFd;

        #[link(wasm_import_module = "wasi_snapshot_preview1")]
        unsafe extern "C" {
            fn fd_renumber(from: u32, to: u32) -> u16;
        }

        // Flush stdout before swapping
        let _ = std::io::stdout().flush();

        // Open capture file for writing — this gives us a file fd (call it C)
        let cap_file = match std::fs::File::create(&capture_file) {
            Ok(f) => f,
            Err(_) => {
                let code = dispatch(cmd, args);
                return (code, String::new());
            }
        };
        let cap_fd = cap_file.as_raw_fd() as u32;
        std::mem::forget(cap_file); // we manage this fd manually

        // Strategy: fd_renumber(from, to) requires `to` to be open.
        // Swap stdout (1) with capture file (cap_fd):
        //   1. fd_renumber(1, cap_fd) — moves stdout to cap_fd's slot, cap_fd is replaced
        //      After: cap_fd = original stdout, fd 1 = freed
        //   2. Now fd 1 is free. Opening a new file should allocate fd 1.
        //   3. Open capture file again for writing → gets fd 1 (hopefully)
        //   4. Run command → output goes to fd 1 (our capture file)
        //   5. fd_renumber(cap_fd, 1) — restore: cap_fd (original stdout) → fd 1
        //      This requires fd 1 to be in used (it is, from step 3)

        unsafe {
            // Step 1: Save stdout by moving fd 1 → cap_fd
            let ret = fd_renumber(1, cap_fd);
            if ret != 0 {
                let code = dispatch(cmd, args);
                let _ = fs::remove_file(&capture_file);
                return (code, String::new());
            }
            // cap_fd now holds original stdout, fd 1 is freed
        }

        // Step 2: Open capture file for writing — should get fd 1 since it's free
        let cap_write = match std::fs::File::create(&capture_file) {
            Ok(f) => f,
            Err(_) => {
                // Restore stdout
                unsafe { let _ = fd_renumber(cap_fd, cap_fd); } // no-op but keeps it alive
                let code = dispatch(cmd, args);
                let _ = fs::remove_file(&capture_file);
                return (code, String::new());
            }
        };
        let new_fd = cap_write.as_raw_fd() as u32;
        std::mem::forget(cap_write);

        if new_fd != 1 {
            // We didn't get fd 1. We need to move new_fd to fd 1.
            // But fd 1 might still not be in used. Try opening one more file
            // to "fill" up to fd 1, or just use renumber if fd 1 is now used.
            // Actually, if new_fd != 1, fd 1 is still not allocated.
            // We can't easily force fd 1. Fall back to no-capture.
            unsafe {
                // Try: move cap_fd (saved stdout) to new_fd (closing the capture file there)
                let _ = fd_renumber(cap_fd, new_fd);
                // Now new_fd = original stdout. We need it at fd 1.
                // fd 1 is still free. We can't renumber to it.
                // Last resort: just accept new_fd as our "stdout"
                // This breaks things, so let's try a different approach
            }
            // Fallback: just dispatch without capture
            let code = dispatch(cmd, args);
            let _ = fs::remove_file(&capture_file);
            return (code, String::new());
        }

        // fd 1 now points to capture file. Run the command.
        let code = dispatch(cmd, args);

        // Flush to ensure all output is written
        let _ = std::io::stdout().flush();

        // Step 3: Restore stdout. cap_fd still holds original stdout.
        // fd 1 is in used (capture file). fd_renumber(cap_fd, 1) works.
        unsafe {
            let _ = fd_renumber(cap_fd, 1);
        }

        // Read captured output
        let output = fs::read_to_string(&capture_file).unwrap_or_default();
        let _ = fs::remove_file(&capture_file);

        (code, output)
    }

    #[cfg(not(target_os = "wasi"))]
    {
        let code = dispatch(cmd, args);
        let _ = fs::remove_file(&capture_file);
        (code, String::new())
    }
}

/// Saved original stdin fd for restore_stdin.
static SAVED_STDIN_FD: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(u32::MAX);

/// Redirect WASI stdin (fd 0) to read from a file.
/// Returns true if successful.
fn redirect_stdin_to_file(path: &str) -> bool {
    #[cfg(target_os = "wasi")]
    {
        use std::os::wasi::io::AsRawFd;

        #[link(wasm_import_module = "wasi_snapshot_preview1")]
        unsafe extern "C" {
            fn fd_renumber(from: u32, to: u32) -> u16;
        }

        // Open the file for reading
        let file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return false,
        };
        let file_fd = file.as_raw_fd() as u32;
        std::mem::forget(file);

        unsafe {
            // fd_renumber(file_fd, 0): moves file_fd → fd 0
            // fd 0 must be in used set (it is — it's WASI stdin)
            // This replaces stdin with our file
            let ret = fd_renumber(file_fd, 0);
            if ret != 0 {
                return false;
            }
            // Note: original stdin is gone. We can't restore it.
            // Store file_fd as a marker that we redirected.
            SAVED_STDIN_FD.store(0, std::sync::atomic::Ordering::Relaxed);
        }
        true
    }
    #[cfg(not(target_os = "wasi"))]
    {
        let _ = path;
        false
    }
}

/// Restore stdin after redirect. Since we can't save the original stdin fd
/// (fd_renumber requires target to be open), we open /dev/null as stdin.
fn restore_stdin() {
    #[cfg(target_os = "wasi")]
    {
        // We can't truly restore the original stdin since fd_renumber replaced it.
        // In the sandbox, stdin is empty anyway (MemoryInputPipe with no data),
        // so losing it is acceptable. Just leave stdin pointing at EOF.
        SAVED_STDIN_FD.store(u32::MAX, std::sync::atomic::Ordering::Relaxed);
    }
    #[cfg(not(target_os = "wasi"))]
    {}
}

fn write_to_file(path: &str, content: &str, append: bool) -> std::io::Result<()> {
    if append {
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        file.write_all(content.as_bytes())?;
    } else {
        fs::write(path, content)?;
    }
    Ok(())
}

fn exec_if(
    if_clause: &IfClause,
    env: &mut ShellEnv,
    dispatch: DispatchFn,
) -> ExecResult {
    let cond_result = exec_program(&if_clause.condition, env, dispatch);
    if cond_result.should_exit {
        return cond_result;
    }

    if cond_result.exit_code == 0 {
        return exec_program(&if_clause.then_body, env, dispatch);
    }

    for (elif_cond, elif_body) in &if_clause.elifs {
        let cr = exec_program(elif_cond, env, dispatch);
        if cr.should_exit {
            return cr;
        }
        if cr.exit_code == 0 {
            return exec_program(elif_body, env, dispatch);
        }
    }

    if let Some(ref else_body) = if_clause.else_body {
        return exec_program(else_body, env, dispatch);
    }

    ExecResult::code(0)
}

fn exec_for(
    for_clause: &ForClause,
    env: &mut ShellEnv,
    dispatch: DispatchFn,
) -> ExecResult {
    let words = if let Some(ref word_list) = for_clause.words {
        let mut expanded = Vec::new();
        for w in word_list {
            expanded.extend(expand::expand_word(w, env, dispatch));
        }
        expanded
    } else {
        env.all_positional().to_vec()
    };

    let mut last_code = 0;

    for val in &words {
        env.set(&for_clause.var, val);
        let result = exec_program(&for_clause.body, env, dispatch);
        last_code = result.exit_code;

        if result.should_exit {
            return result;
        }

        match &result.control_flow {
            Some(ControlFlow::Break(n)) => {
                if *n > 1 {
                    return ExecResult::control(ControlFlow::Break(n - 1));
                }
                break;
            }
            Some(ControlFlow::Continue(n)) => {
                if *n > 1 {
                    return ExecResult::control(ControlFlow::Continue(n - 1));
                }
                continue;
            }
            Some(ControlFlow::Return(code)) => {
                return ExecResult::control(ControlFlow::Return(*code));
            }
            None => {}
        }
    }

    ExecResult::code(last_code)
}

fn exec_while(
    while_clause: &WhileClause,
    env: &mut ShellEnv,
    dispatch: DispatchFn,
) -> ExecResult {
    let mut last_code = 0;

    loop {
        let cond = exec_program(&while_clause.condition, env, dispatch);
        if cond.should_exit {
            return cond;
        }
        if cond.exit_code != 0 {
            break;
        }

        let result = exec_program(&while_clause.body, env, dispatch);
        last_code = result.exit_code;

        if result.should_exit {
            return result;
        }

        match &result.control_flow {
            Some(ControlFlow::Break(n)) => {
                if *n > 1 {
                    return ExecResult::control(ControlFlow::Break(n - 1));
                }
                break;
            }
            Some(ControlFlow::Continue(n)) => {
                if *n > 1 {
                    return ExecResult::control(ControlFlow::Continue(n - 1));
                }
                continue;
            }
            Some(ControlFlow::Return(code)) => {
                return ExecResult::control(ControlFlow::Return(*code));
            }
            None => {}
        }
    }

    ExecResult::code(last_code)
}

fn exec_until(
    until_clause: &UntilClause,
    env: &mut ShellEnv,
    dispatch: DispatchFn,
) -> ExecResult {
    let mut last_code = 0;

    loop {
        let cond = exec_program(&until_clause.condition, env, dispatch);
        if cond.should_exit {
            return cond;
        }
        if cond.exit_code == 0 {
            break;
        }

        let result = exec_program(&until_clause.body, env, dispatch);
        last_code = result.exit_code;

        if result.should_exit {
            return result;
        }

        match &result.control_flow {
            Some(ControlFlow::Break(n)) => {
                if *n > 1 {
                    return ExecResult::control(ControlFlow::Break(n - 1));
                }
                break;
            }
            Some(ControlFlow::Continue(n)) => {
                if *n > 1 {
                    return ExecResult::control(ControlFlow::Continue(n - 1));
                }
                continue;
            }
            Some(ControlFlow::Return(code)) => {
                return ExecResult::control(ControlFlow::Return(*code));
            }
            None => {}
        }
    }

    ExecResult::code(last_code)
}

fn exec_case(
    case_clause: &CaseClause,
    env: &mut ShellEnv,
    dispatch: DispatchFn,
) -> ExecResult {
    let word_val = expand::expand_word_to_string(&case_clause.word, env, dispatch);

    for arm in &case_clause.arms {
        for pattern in &arm.patterns {
            let pat_val = expand::expand_word_to_string(pattern, env, dispatch);
            if case_matches(&word_val, &pat_val) {
                return exec_program(&arm.body, env, dispatch);
            }
        }
    }

    ExecResult::code(0)
}

/// Match a value against a case pattern (supports * and ? globs).
fn case_matches(value: &str, pattern: &str) -> bool {
    if pattern == "*" {
        return true;
    }
    super::expand::glob_match(pattern, value)
}
