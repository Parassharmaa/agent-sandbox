use std::fs;
use std::io::{self, Read};

use sha2::{Digest, Sha256};

pub fn run(args: &[String]) -> i32 {
    let mut check = false;
    let mut files = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-c" | "--check" => check = true,
            _ if !arg.starts_with('-') => files.push(arg.clone()),
            _ => {
                eprintln!("sha256sum: invalid option '{}'", arg);
                return 1;
            }
        }
    }

    if check {
        return check_sums(&files);
    }

    if files.is_empty() {
        files.push("-".to_string());
    }

    let mut exit_code = 0;

    for file in &files {
        let data = if file == "-" {
            let mut buf = Vec::new();
            if let Err(e) = io::stdin().read_to_end(&mut buf) {
                eprintln!("sha256sum: {}", e);
                exit_code = 1;
                continue;
            }
            buf
        } else {
            match fs::read(file) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("sha256sum: {}: {}", file, e);
                    exit_code = 1;
                    continue;
                }
            }
        };

        let hash = Sha256::digest(&data);
        let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
        println!("{}  {}", hex, if file == "-" { "-" } else { file.as_str() });
    }

    exit_code
}

fn check_sums(files: &[String]) -> i32 {
    let mut exit_code = 0;

    for file in files {
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("sha256sum: {}: {}", file, e);
                return 1;
            }
        };

        for line in content.lines() {
            let parts: Vec<&str> = line.splitn(2, "  ").collect();
            if parts.len() != 2 {
                continue;
            }
            let expected_hash = parts[0];
            let target_file = parts[1];

            match fs::read(target_file) {
                Ok(data) => {
                    let hash = Sha256::digest(&data);
                    let hex: String = hash.iter().map(|b| format!("{:02x}", b)).collect();
                    if hex == expected_hash {
                        println!("{}: OK", target_file);
                    } else {
                        println!("{}: FAILED", target_file);
                        exit_code = 1;
                    }
                }
                Err(e) => {
                    eprintln!("sha256sum: {}: {}", target_file, e);
                    exit_code = 1;
                }
            }
        }
    }

    exit_code
}
