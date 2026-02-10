use std::fs;
use std::io::{self, BufRead};

pub fn run(args: &[String]) -> i32 {
    let mut width = 80;
    let mut break_words = true;
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-s" => break_words = false,
            "-w" => {
                i += 1;
                if i < args.len() {
                    width = args[i].parse().unwrap_or(80);
                }
            }
            arg if arg.starts_with("-w") => {
                width = arg[2..].parse().unwrap_or(80);
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
        let lines: Vec<String> = if file == "-" {
            io::stdin().lock().lines().map_while(|l| l.ok()).collect()
        } else {
            match fs::read_to_string(file) {
                Ok(c) => c.lines().map(String::from).collect(),
                Err(e) => {
                    eprintln!("fold: {}: {}", file, e);
                    exit_code = 1;
                    continue;
                }
            }
        };

        for line in &lines {
            if break_words {
                fold_hard(line, width);
            } else {
                fold_soft(line, width);
            }
        }
    }
    exit_code
}

fn fold_hard(line: &str, width: usize) {
    let chars: Vec<char> = line.chars().collect();
    for chunk in chars.chunks(width) {
        println!("{}", chunk.iter().collect::<String>());
    }
    if chars.is_empty() {
        println!();
    }
}

fn fold_soft(line: &str, width: usize) {
    if line.len() <= width {
        println!("{}", line);
        return;
    }

    let mut start = 0;
    while start < line.len() {
        if start + width >= line.len() {
            println!("{}", &line[start..]);
            break;
        }

        // Find last space within width
        let end = start + width;
        let segment = &line[start..end];
        if let Some(pos) = segment.rfind(' ') {
            println!("{}", &line[start..start + pos]);
            start = start + pos + 1;
        } else {
            // No space found, hard break
            println!("{}", segment);
            start = end;
        }
    }
}
