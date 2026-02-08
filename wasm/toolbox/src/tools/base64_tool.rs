use std::fs;
use std::io::{self, Read, Write};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

pub fn run(args: &[String]) -> i32 {
    let mut decode = false;
    let mut files = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-d" | "--decode" => decode = true,
            _ if !arg.starts_with('-') => files.push(arg.clone()),
            _ => {
                eprintln!("base64: invalid option '{}'", arg);
                return 1;
            }
        }
    }

    let input = if files.is_empty() {
        let mut buf = Vec::new();
        if let Err(e) = io::stdin().read_to_end(&mut buf) {
            eprintln!("base64: {}", e);
            return 1;
        }
        buf
    } else {
        match fs::read(&files[0]) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("base64: {}: {}", files[0], e);
                return 1;
            }
        }
    };

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if decode {
        // Strip whitespace before decoding
        let input_str: String = String::from_utf8_lossy(&input)
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect();
        match STANDARD.decode(&input_str) {
            Ok(decoded) => {
                out.write_all(&decoded).ok();
            }
            Err(e) => {
                eprintln!("base64: invalid input: {}", e);
                return 1;
            }
        }
    } else {
        let encoded = STANDARD.encode(&input);
        // Wrap at 76 chars
        for chunk in encoded.as_bytes().chunks(76) {
            out.write_all(chunk).ok();
            out.write_all(b"\n").ok();
        }
    }

    0
}
