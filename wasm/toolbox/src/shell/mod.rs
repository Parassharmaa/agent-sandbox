pub mod ast;
pub mod builtins;
pub mod env;
pub mod exec;
pub mod expand;
pub mod lexer;
pub mod parser;
pub mod pipeline;
pub mod redirect;
pub mod token;

use std::fs;

use self::env::ShellEnv;
use self::exec::exec_program;
use self::parser::Parser;

/// Entry point for `sh` command.
/// Supports `sh -c "script"`, `sh script.sh [args...]`, and `sh` (interactive-ish, reads stdin).
pub fn run(args: &[String], dispatch: fn(&str, &[String]) -> i32) -> i32 {
    let mut script = None;
    let mut script_args = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-c" => {
                i += 1;
                if i < args.len() {
                    script = Some(args[i].clone());
                    // Remaining args become $0, $1, ...
                    i += 1;
                    while i < args.len() {
                        script_args.push(args[i].clone());
                        i += 1;
                    }
                } else {
                    eprintln!("sh: -c: option requires an argument");
                    return 2;
                }
            }
            _ => {
                // First non-flag arg is script file
                let file = &args[i];
                match fs::read_to_string(file) {
                    Ok(content) => {
                        script = Some(content);
                        // Remaining args become positional params
                        i += 1;
                        while i < args.len() {
                            script_args.push(args[i].clone());
                            i += 1;
                        }
                    }
                    Err(e) => {
                        eprintln!("sh: {}: {}", file, e);
                        return 1;
                    }
                }
            }
        }
        i += 1;
    }

    let script = match script {
        Some(s) => s,
        None => {
            // Read from stdin
            use std::io::Read;
            let mut buf = String::new();
            if std::io::stdin().read_to_string(&mut buf).is_ok() {
                buf
            } else {
                return 0;
            }
        }
    };

    let mut env = ShellEnv::new();
    env.positional = script_args;

    let mut parser = Parser::new(&script);
    let program = match parser.parse_program() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("sh: syntax error: {}", e);
            return 2;
        }
    };

    let result = exec_program(&program, &mut env, dispatch);

    if result.should_exit {
        return result.exit_code;
    }

    result.exit_code
}
