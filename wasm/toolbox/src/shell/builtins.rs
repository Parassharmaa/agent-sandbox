use std::io::{self, BufRead, Write};

use super::env::ShellEnv;

/// Check if a command name is a shell builtin.
pub fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "cd" | "export"
            | "unset"
            | "set"
            | "read"
            | "exit"
            | "test"
            | "["
            | "true"
            | "false"
            | ":"
            | "shift"
            | "local"
            | "source"
            | "."
            | "return"
            | "break"
            | "continue"
            | "eval"
            | "type"
    )
}

/// Execute a builtin command. Returns (exit_code, should_exit, control_flow).
pub fn run_builtin(
    name: &str,
    args: &[String],
    env: &mut ShellEnv,
) -> BuiltinResult {
    match name {
        "true" | ":" => BuiltinResult::code(0),
        "false" => BuiltinResult::code(1),
        "exit" => {
            let code = args.first().and_then(|a| a.parse().ok()).unwrap_or(env.last_status);
            BuiltinResult::exit(code)
        }
        "cd" => builtin_cd(args, env),
        "export" => builtin_export(args, env),
        "unset" => builtin_unset(args, env),
        "set" => builtin_set(args, env),
        "read" => builtin_read(args, env),
        "test" | "[" => builtin_test(args),
        "shift" => builtin_shift(args, env),
        "local" => builtin_local(args, env),
        "return" => {
            let code = args.first().and_then(|a| a.parse().ok()).unwrap_or(0);
            BuiltinResult::control(ControlFlow::Return(code))
        }
        "break" => {
            let n = args.first().and_then(|a| a.parse().ok()).unwrap_or(1);
            BuiltinResult::control(ControlFlow::Break(n))
        }
        "continue" => {
            let n = args.first().and_then(|a| a.parse().ok()).unwrap_or(1);
            BuiltinResult::control(ControlFlow::Continue(n))
        }
        "type" => builtin_type(args),
        _ => BuiltinResult::code(127),
    }
}

#[derive(Debug)]
pub struct BuiltinResult {
    pub exit_code: i32,
    pub should_exit: bool,
    pub control_flow: Option<ControlFlow>,
}

#[derive(Debug, Clone)]
pub enum ControlFlow {
    Break(usize),
    Continue(usize),
    Return(i32),
}

impl BuiltinResult {
    pub fn code(exit_code: i32) -> Self {
        BuiltinResult {
            exit_code,
            should_exit: false,
            control_flow: None,
        }
    }

    pub fn exit(code: i32) -> Self {
        BuiltinResult {
            exit_code: code,
            should_exit: true,
            control_flow: None,
        }
    }

    pub fn control(cf: ControlFlow) -> Self {
        BuiltinResult {
            exit_code: 0,
            should_exit: false,
            control_flow: Some(cf),
        }
    }
}

fn builtin_cd(args: &[String], env: &mut ShellEnv) -> BuiltinResult {
    let dir = if args.is_empty() {
        env.get("HOME").unwrap_or("/").to_string()
    } else {
        args[0].clone()
    };

    match std::env::set_current_dir(&dir) {
        Ok(()) => {
            if let Ok(cwd) = std::env::current_dir() {
                env.set("PWD", &cwd.to_string_lossy());
            }
            BuiltinResult::code(0)
        }
        Err(e) => {
            eprintln!("cd: {}: {}", dir, e);
            BuiltinResult::code(1)
        }
    }
}

fn builtin_export(args: &[String], env: &mut ShellEnv) -> BuiltinResult {
    if args.is_empty() {
        // Print all exports
        for (k, v) in env.exported_vars() {
            println!("declare -x {}=\"{}\"", k, v);
        }
        return BuiltinResult::code(0);
    }

    for arg in args {
        if let Some(eq_pos) = arg.find('=') {
            let name = &arg[..eq_pos];
            let value = &arg[eq_pos + 1..];
            env.export(name, Some(value));
        } else {
            env.export(arg, None);
        }
    }
    BuiltinResult::code(0)
}

fn builtin_unset(args: &[String], env: &mut ShellEnv) -> BuiltinResult {
    for name in args {
        if name == "-v" || name == "-f" {
            continue;
        }
        env.unset(name);
    }
    BuiltinResult::code(0)
}

fn builtin_set(args: &[String], env: &mut ShellEnv) -> BuiltinResult {
    if args.is_empty() {
        // Print all variables
        return BuiltinResult::code(0);
    }

    // set -- args... sets positional parameters
    if !args.is_empty() && args[0] == "--" {
        env.positional = args[1..].to_vec();
        return BuiltinResult::code(0);
    }

    // set -e, set -x, etc. (flags we don't fully implement but accept)
    BuiltinResult::code(0)
}

fn builtin_read(args: &[String], env: &mut ShellEnv) -> BuiltinResult {
    let mut prompt = String::new();
    let mut var_names = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-p" => {
                i += 1;
                if i < args.len() {
                    prompt = args[i].clone();
                }
            }
            "-r" => {} // raw mode (default for us)
            _ => var_names.push(args[i].clone()),
        }
        i += 1;
    }

    if !prompt.is_empty() {
        eprint!("{}", prompt);
        io::stderr().flush().ok();
    }

    let mut line = String::new();
    match io::stdin().lock().read_line(&mut line) {
        Ok(0) => return BuiltinResult::code(1), // EOF
        Ok(_) => {}
        Err(_) => return BuiltinResult::code(1),
    }

    let line = line.trim_end_matches('\n').trim_end_matches('\r');

    if var_names.is_empty() {
        var_names.push("REPLY".to_string());
    }

    let parts: Vec<&str> = line.splitn(var_names.len(), char::is_whitespace).collect();

    for (i, name) in var_names.iter().enumerate() {
        let value = parts.get(i).unwrap_or(&"");
        env.set(name, value);
    }

    BuiltinResult::code(0)
}

fn builtin_test(args: &[String]) -> BuiltinResult {
    let args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    // Remove trailing ] if invoked as [
    let args = if !args.is_empty() && *args.last().unwrap() == "]" {
        &args[..args.len() - 1]
    } else {
        &args
    };

    let result = eval_test(args);
    BuiltinResult::code(if result { 0 } else { 1 })
}

fn eval_test(args: &[&str]) -> bool {
    match args.len() {
        0 => false,
        1 => !args[0].is_empty(),
        2 => {
            let op = args[0];
            let val = args[1];
            match op {
                "-z" => val.is_empty(),
                "-n" => !val.is_empty(),
                "!" => !eval_test(&args[1..]),
                "-e" | "-f" => std::path::Path::new(val).exists(),
                "-d" => std::path::Path::new(val).is_dir(),
                "-s" => std::fs::metadata(val).map(|m| m.len() > 0).unwrap_or(false),
                "-r" | "-w" | "-x" => std::path::Path::new(val).exists(),
                _ => !args[0].is_empty(),
            }
        }
        _ => {
            // 3+ args: look for binary operators
            if args.len() >= 3 {
                let left = args[0];
                let op = args[1];
                let right = args[2];

                let result = match op {
                    "=" | "==" => left == right,
                    "!=" => left != right,
                    "-eq" => left.parse::<i64>().unwrap_or(0) == right.parse::<i64>().unwrap_or(0),
                    "-ne" => left.parse::<i64>().unwrap_or(0) != right.parse::<i64>().unwrap_or(0),
                    "-lt" => left.parse::<i64>().unwrap_or(0) < right.parse::<i64>().unwrap_or(0),
                    "-le" => left.parse::<i64>().unwrap_or(0) <= right.parse::<i64>().unwrap_or(0),
                    "-gt" => left.parse::<i64>().unwrap_or(0) > right.parse::<i64>().unwrap_or(0),
                    "-ge" => left.parse::<i64>().unwrap_or(0) >= right.parse::<i64>().unwrap_or(0),
                    _ => false,
                };

                // Handle remaining args with -a/-o
                if args.len() > 3 {
                    let rest = &args[3..];
                    if !rest.is_empty() {
                        match rest[0] {
                            "-a" => return result && eval_test(&rest[1..]),
                            "-o" => return result || eval_test(&rest[1..]),
                            _ => {}
                        }
                    }
                }

                return result;
            }

            // Handle ! prefix
            if args[0] == "!" {
                return !eval_test(&args[1..]);
            }

            false
        }
    }
}

fn builtin_shift(args: &[String], env: &mut ShellEnv) -> BuiltinResult {
    let n = args.first().and_then(|a| a.parse().ok()).unwrap_or(1);
    env.shift(n);
    BuiltinResult::code(0)
}

fn builtin_local(args: &[String], env: &mut ShellEnv) -> BuiltinResult {
    for arg in args {
        if let Some(eq_pos) = arg.find('=') {
            let name = &arg[..eq_pos];
            let value = &arg[eq_pos + 1..];
            env.declare_local(name);
            env.set(name, value);
        } else {
            env.declare_local(arg);
        }
    }
    BuiltinResult::code(0)
}

fn builtin_type(args: &[String]) -> BuiltinResult {
    for name in args {
        if is_builtin(name) {
            println!("{} is a shell builtin", name);
        } else {
            println!("{} is /usr/bin/{}", name, name);
        }
    }
    BuiltinResult::code(0)
}
