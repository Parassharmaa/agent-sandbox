use std::fs;
use std::path::Path;

use boa_engine::{Context, Source};
use boa_runtime::console::{Console, DefaultLogger};

pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Usage: node [options] [script.js] [arguments]");
        eprintln!("Options:");
        eprintln!("  -e, --eval <code>  Evaluate JavaScript code");
        eprintln!("  -p, --print <code> Evaluate and print result");
        eprintln!("  --version          Print version");
        return 1;
    }

    // Handle --version
    if args[0] == "--version" || args[0] == "-v" {
        println!("node v0.1.0 (boa-engine/wasm-sandbox)");
        return 0;
    }

    let mut context = Context::default();

    // Register console object globally (console.log, console.error, etc.)
    if let Err(e) = Console::register_with_logger(DefaultLogger, &mut context) {
        eprintln!("node: failed to register console: {e}");
        return 1;
    }

    match args[0].as_str() {
        "-e" | "--eval" => {
            if args.len() < 2 {
                eprintln!("node: -e requires an argument");
                return 1;
            }
            execute(&mut context, &args[1])
        }
        "-p" | "--print" => {
            if args.len() < 2 {
                eprintln!("node: -p requires an argument");
                return 1;
            }
            execute_and_print(&mut context, &args[1])
        }
        _ => {
            // Treat as a file path
            let file_path = &args[0];
            match fs::read_to_string(file_path) {
                Ok(content) => {
                    let source =
                        Source::from_bytes(&content).with_path(Path::new(file_path));
                    match context.eval(source) {
                        Ok(_) => 0,
                        Err(err) => {
                            eprintln!("{err}");
                            1
                        }
                    }
                }
                Err(e) => {
                    eprintln!("node: cannot open '{}': {}", file_path, e);
                    1
                }
            }
        }
    }
}

fn execute(context: &mut Context, code: &str) -> i32 {
    let source = Source::from_bytes(code);
    match context.eval(source) {
        Ok(_) => 0,
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}

fn execute_and_print(context: &mut Context, code: &str) -> i32 {
    let source = Source::from_bytes(code);
    match context.eval(source) {
        Ok(val) => {
            match val.to_string(context) {
                Ok(output) => {
                    println!("{}", output.to_std_string_escaped());
                    0
                }
                Err(e) => {
                    eprintln!("{e}");
                    1
                }
            }
        }
        Err(err) => {
            eprintln!("{err}");
            1
        }
    }
}
