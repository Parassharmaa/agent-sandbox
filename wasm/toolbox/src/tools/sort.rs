use std::fs;
use std::io::{self, BufRead};

pub fn run(args: &[String]) -> i32 {
    let mut reverse = false;
    let mut numeric = false;
    let mut unique = false;
    let mut files = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-r" => reverse = true,
            "-n" => numeric = true,
            "-u" => unique = true,
            arg if arg.starts_with('-') && arg.len() > 1 => {
                for ch in arg[1..].chars() {
                    match ch {
                        'r' => reverse = true,
                        'n' => numeric = true,
                        'u' => unique = true,
                        _ => {
                            eprintln!("sort: invalid option -- '{}'", ch);
                            return 1;
                        }
                    }
                }
            }
            _ => files.push(arg.clone()),
        }
    }

    let mut lines: Vec<String> = Vec::new();

    if files.is_empty() {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => lines.push(l),
                Err(e) => {
                    eprintln!("sort: {}", e);
                    return 1;
                }
            }
        }
    } else {
        for file in &files {
            match fs::read_to_string(file) {
                Ok(content) => {
                    for line in content.lines() {
                        lines.push(line.to_string());
                    }
                }
                Err(e) => {
                    eprintln!("sort: {}: {}", file, e);
                    return 1;
                }
            }
        }
    }

    if numeric {
        lines.sort_by(|a, b| {
            let na: f64 = a
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            let nb: f64 = b
                .split_whitespace()
                .next()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0);
            na.partial_cmp(&nb).unwrap_or(std::cmp::Ordering::Equal)
        });
    } else {
        lines.sort();
    }

    if reverse {
        lines.reverse();
    }

    if unique {
        lines.dedup();
    }

    for line in &lines {
        println!("{}", line);
    }

    0
}
