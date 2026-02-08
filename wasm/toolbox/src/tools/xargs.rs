use std::io::{self, BufRead};

pub fn run(args: &[String]) -> i32 {
    // In WASM context, xargs just reads stdin lines and prints them as args
    // Real execution would need to be handled by the sandbox runtime
    let mut delimiter = None;
    let mut max_args: Option<usize> = None;
    let mut cmd_args = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-d" => {
                i += 1;
                if i < args.len() {
                    delimiter = args[i].chars().next();
                }
            }
            "-n" => {
                i += 1;
                if i < args.len() {
                    max_args = args[i].parse().ok();
                }
            }
            _ => cmd_args.push(args[i].clone()),
        }
        i += 1;
    }

    let stdin = io::stdin();
    let mut items = Vec::new();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if let Some(delim) = delimiter {
            for part in line.split(delim) {
                let trimmed = part.trim();
                if !trimmed.is_empty() {
                    items.push(trimmed.to_string());
                }
            }
        } else {
            for part in line.split_whitespace() {
                items.push(part.to_string());
            }
        }
    }

    // In the sandbox, we just output what would be executed
    if let Some(n) = max_args {
        for chunk in items.chunks(n) {
            if cmd_args.is_empty() {
                println!("echo {}", chunk.join(" "));
            } else {
                println!("{} {}", cmd_args.join(" "), chunk.join(" "));
            }
        }
    } else if cmd_args.is_empty() {
        println!("echo {}", items.join(" "));
    } else {
        println!("{} {}", cmd_args.join(" "), items.join(" "));
    }

    0
}
