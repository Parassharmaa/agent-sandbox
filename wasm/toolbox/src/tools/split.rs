use std::fs;
use std::io::{self, Read};

pub fn run(args: &[String]) -> i32 {
    let mut lines_per_file = 1000;
    let mut bytes_per_file: Option<usize> = None;
    let mut prefix = "x".to_string();
    let mut input_file = "-".to_string();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-l" => {
                i += 1;
                if i < args.len() {
                    lines_per_file = args[i].parse().unwrap_or(1000);
                }
            }
            "-b" => {
                i += 1;
                if i < args.len() {
                    bytes_per_file = parse_size(&args[i]);
                }
            }
            arg if arg.starts_with('-') && arg.len() > 1 && arg[1..].parse::<usize>().is_ok() => {
                lines_per_file = arg[1..].parse().unwrap_or(1000);
            }
            _ => {
                if input_file == "-" && !args[i].starts_with('-') {
                    input_file = args[i].clone();
                } else {
                    prefix = args[i].clone();
                }
            }
        }
        i += 1;
    }

    let data = if input_file == "-" {
        let mut buf = Vec::new();
        if io::stdin().read_to_end(&mut buf).is_err() {
            eprintln!("split: error reading stdin");
            return 1;
        }
        buf
    } else {
        match fs::read(&input_file) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("split: {}: {}", input_file, e);
                return 1;
            }
        }
    };

    if let Some(chunk_size) = bytes_per_file {
        split_bytes(&data, chunk_size, &prefix)
    } else {
        let text = String::from_utf8_lossy(&data);
        split_lines(&text, lines_per_file, &prefix)
    }
}

fn split_bytes(data: &[u8], chunk_size: usize, prefix: &str) -> i32 {
    for (idx, chunk) in data.chunks(chunk_size).enumerate() {
        let suffix = index_to_suffix(idx);
        let filename = format!("{}{}", prefix, suffix);
        if let Err(e) = fs::write(&filename, chunk) {
            eprintln!("split: {}: {}", filename, e);
            return 1;
        }
    }
    0
}

fn split_lines(text: &str, lines_per_file: usize, prefix: &str) -> i32 {
    let lines: Vec<&str> = text.lines().collect();
    for (idx, chunk) in lines.chunks(lines_per_file).enumerate() {
        let suffix = index_to_suffix(idx);
        let filename = format!("{}{}", prefix, suffix);
        let content = chunk.join("\n") + "\n";
        if let Err(e) = fs::write(&filename, content) {
            eprintln!("split: {}: {}", filename, e);
            return 1;
        }
    }
    0
}

fn index_to_suffix(idx: usize) -> String {
    let a = (b'a' + (idx / 26) as u8) as char;
    let b = (b'a' + (idx % 26) as u8) as char;
    format!("{}{}", a, b)
}

fn parse_size(s: &str) -> Option<usize> {
    let s = s.trim();
    if let Some(n) = s.strip_suffix('k').or_else(|| s.strip_suffix('K')) {
        n.parse::<usize>().ok().map(|n| n * 1024)
    } else if let Some(n) = s.strip_suffix('m').or_else(|| s.strip_suffix('M')) {
        n.parse::<usize>().ok().map(|n| n * 1024 * 1024)
    } else {
        s.parse().ok()
    }
}
