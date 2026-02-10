use std::fs;
use std::io::{self, Read};

pub fn run(args: &[String]) -> i32 {
    let mut min_len = 4;
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => {
                i += 1;
                if i < args.len() {
                    min_len = args[i].parse().unwrap_or(4);
                }
            }
            arg if arg.starts_with('-') && arg[1..].parse::<usize>().is_ok() => {
                min_len = arg[1..].parse().unwrap_or(4);
            }
            _ => files.push(args[i].clone()),
        }
        i += 1;
    }

    if files.is_empty() {
        files.push("-".to_string());
    }

    let mut exit_code = 0;
    for file in &files {
        let data = if file == "-" {
            let mut buf = Vec::new();
            if io::stdin().read_to_end(&mut buf).is_err() {
                exit_code = 1;
                continue;
            }
            buf
        } else {
            match fs::read(file) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("strings: {}: {}", file, e);
                    exit_code = 1;
                    continue;
                }
            }
        };

        extract_strings(&data, min_len);
    }
    exit_code
}

fn extract_strings(data: &[u8], min_len: usize) {
    let mut current = String::new();

    for &byte in data {
        if byte >= 0x20 && byte < 0x7f || byte == b'\t' {
            current.push(byte as char);
        } else {
            if current.len() >= min_len {
                println!("{}", current);
            }
            current.clear();
        }
    }

    if current.len() >= min_len {
        println!("{}", current);
    }
}
