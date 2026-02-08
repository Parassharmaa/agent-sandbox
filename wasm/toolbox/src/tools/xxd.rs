use std::fs;
use std::io::{self, Read, Write};

pub fn run(args: &[String]) -> i32 {
    let mut reverse = false;
    let mut cols: usize = 16;
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-r" => reverse = true,
            "-c" => {
                i += 1;
                if i < args.len() {
                    cols = args[i].parse().unwrap_or(16);
                }
            }
            _ if !args[i].starts_with('-') => files.push(args[i].clone()),
            _ => {}
        }
        i += 1;
    }

    let input = if files.is_empty() {
        let mut buf = Vec::new();
        if let Err(e) = io::stdin().read_to_end(&mut buf) {
            eprintln!("xxd: {}", e);
            return 1;
        }
        buf
    } else {
        match fs::read(&files[0]) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("xxd: {}: {}", files[0], e);
                return 1;
            }
        }
    };

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if reverse {
        // Reverse hex dump
        let text = String::from_utf8_lossy(&input);
        let mut bytes = Vec::new();
        for line in text.lines() {
            // Skip offset (first part before colon)
            let hex_part = if let Some(pos) = line.find(':') {
                let after = &line[pos + 1..];
                // Take hex portion (before ASCII display)
                if let Some(end) = after.rfind("  ") {
                    &after[..end]
                } else {
                    after
                }
            } else {
                continue;
            };

            for hex_byte in hex_part.split_whitespace() {
                if let Ok(b) = u8::from_str_radix(hex_byte, 16) {
                    bytes.push(b);
                }
            }
        }
        out.write_all(&bytes).ok();
    } else {
        // Forward hex dump
        for (offset, chunk) in input.chunks(cols).enumerate() {
            write!(out, "{:08x}: ", offset * cols).ok();

            // Hex bytes
            for (i, byte) in chunk.iter().enumerate() {
                if i > 0 && i % 2 == 0 {
                    write!(out, " ").ok();
                }
                write!(out, "{:02x}", byte).ok();
            }

            // Padding
            let remaining = cols - chunk.len();
            for i in 0..remaining {
                if (chunk.len() + i) % 2 == 0 {
                    write!(out, " ").ok();
                }
                write!(out, "  ").ok();
            }

            write!(out, "  ").ok();

            // ASCII
            for byte in chunk {
                let c = if *byte >= 0x20 && *byte < 0x7f {
                    *byte as char
                } else {
                    '.'
                };
                write!(out, "{}", c).ok();
            }

            writeln!(out).ok();
        }
    }

    0
}
