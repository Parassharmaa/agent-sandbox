use std::fs;
use std::io::{self, BufRead, Write};

pub fn run(args: &[String]) -> i32 {
    let mut append = false;
    let mut files = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-a" => append = true,
            _ if arg.starts_with('-') && arg.len() > 1 => {
                eprintln!("tee: invalid option -- '{}'", arg);
                return 1;
            }
            _ => files.push(arg.clone()),
        }
    }

    let mut outputs: Vec<Box<dyn Write>> = Vec::new();
    let mut exit_code = 0;

    for file in &files {
        let f = if append {
            fs::OpenOptions::new().create(true).append(true).open(file)
        } else {
            fs::File::create(file)
        };
        match f {
            Ok(f) => outputs.push(Box::new(f)),
            Err(e) => {
                eprintln!("tee: {}: {}", file, e);
                exit_code = 1;
            }
        }
    }

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    for line in stdin.lock().lines() {
        match line {
            Ok(line) => {
                let line_bytes = format!("{}\n", line);
                let _ = stdout.write_all(line_bytes.as_bytes());
                for out in &mut outputs {
                    let _ = out.write_all(line_bytes.as_bytes());
                }
            }
            Err(e) => {
                eprintln!("tee: {}", e);
                exit_code = 1;
                break;
            }
        }
    }

    exit_code
}
