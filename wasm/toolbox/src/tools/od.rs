use std::fs;
use std::io::{self, Read};

pub fn run(args: &[String]) -> i32 {
    let mut format = 'o'; // octal by default
    let mut files = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-x" | "-h" => format = 'x',
            "-c" => format = 'c',
            "-d" => format = 'd',
            "-b" => format = 'b',
            "-A" | "-An" => {} // address format, we always show octal offsets
            _ => files.push(arg.clone()),
        }
    }

    if files.is_empty() {
        files.push("-".to_string());
    }

    let mut all_data = Vec::new();
    for file in &files {
        let data = if file == "-" {
            let mut buf = Vec::new();
            if io::stdin().read_to_end(&mut buf).is_err() {
                return 1;
            }
            buf
        } else {
            match fs::read(file) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("od: {}: {}", file, e);
                    return 1;
                }
            }
        };
        all_data.extend_from_slice(&data);
    }

    match format {
        'x' => dump_hex(&all_data),
        'c' => dump_chars(&all_data),
        'd' => dump_decimal(&all_data),
        'b' => dump_octal_bytes(&all_data),
        _ => dump_octal(&all_data),
    }

    println!("{:07o}", all_data.len());
    0
}

fn dump_octal(data: &[u8]) {
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("{:07o}", i * 16);
        for word in chunk.chunks(2) {
            if word.len() == 2 {
                let val = u16::from_le_bytes([word[0], word[1]]);
                print!(" {:06o}", val);
            } else {
                print!(" {:06o}", word[0] as u16);
            }
        }
        println!();
    }
}

fn dump_hex(data: &[u8]) {
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("{:07o}", i * 16);
        for word in chunk.chunks(2) {
            if word.len() == 2 {
                let val = u16::from_le_bytes([word[0], word[1]]);
                print!(" {:04x}", val);
            } else {
                print!(" {:04x}", word[0] as u16);
            }
        }
        println!();
    }
}

fn dump_chars(data: &[u8]) {
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("{:07o}", i * 16);
        for &byte in chunk {
            let ch = match byte {
                0 => "  \\0".to_string(),
                7 => "  \\a".to_string(),
                8 => "  \\b".to_string(),
                9 => "  \\t".to_string(),
                10 => "  \\n".to_string(),
                13 => "  \\r".to_string(),
                32..=126 => format!("   {}", byte as char),
                _ => format!(" {:03o}", byte),
            };
            print!("{}", ch);
        }
        println!();
    }
}

fn dump_decimal(data: &[u8]) {
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("{:07o}", i * 16);
        for word in chunk.chunks(2) {
            if word.len() == 2 {
                let val = u16::from_le_bytes([word[0], word[1]]);
                print!(" {:05}", val);
            } else {
                print!(" {:05}", word[0] as u16);
            }
        }
        println!();
    }
}

fn dump_octal_bytes(data: &[u8]) {
    for (i, chunk) in data.chunks(16).enumerate() {
        print!("{:07o}", i * 16);
        for &byte in chunk {
            print!(" {:03o}", byte);
        }
        println!();
    }
}
