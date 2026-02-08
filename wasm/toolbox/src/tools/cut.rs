use std::fs;
use std::io::{self, BufRead};

pub fn run(args: &[String]) -> i32 {
    let mut delimiter = '\t';
    let mut fields: Option<Vec<usize>> = None;
    let mut chars: Option<Vec<usize>> = None;
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-d" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("cut: option requires an argument -- 'd'");
                    return 1;
                }
                delimiter = args[i].chars().next().unwrap_or('\t');
            }
            "-f" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("cut: option requires an argument -- 'f'");
                    return 1;
                }
                fields = Some(parse_ranges(&args[i]));
            }
            "-c" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("cut: option requires an argument -- 'c'");
                    return 1;
                }
                chars = Some(parse_ranges(&args[i]));
            }
            _ if !args[i].starts_with('-') => files.push(args[i].clone()),
            _ => {
                eprintln!("cut: invalid option '{}'", args[i]);
                return 1;
            }
        }
        i += 1;
    }

    if fields.is_none() && chars.is_none() {
        eprintln!("cut: you must specify a list of fields or character positions");
        return 1;
    }

    let process_line = |line: &str| {
        if let Some(ref f) = fields {
            let parts: Vec<&str> = line.split(delimiter).collect();
            let selected: Vec<&str> = f
                .iter()
                .filter_map(|&idx| parts.get(idx.saturating_sub(1)).copied())
                .collect();
            println!("{}", selected.join(&delimiter.to_string()));
        } else if let Some(ref c) = chars {
            let char_vec: Vec<char> = line.chars().collect();
            let selected: String = c
                .iter()
                .filter_map(|&idx| char_vec.get(idx.saturating_sub(1)).copied())
                .collect();
            println!("{}", selected);
        }
    };

    if files.is_empty() {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => process_line(&l),
                Err(e) => {
                    eprintln!("cut: {}", e);
                    return 1;
                }
            }
        }
    } else {
        for file in &files {
            match fs::read_to_string(file) {
                Ok(content) => {
                    for line in content.lines() {
                        process_line(line);
                    }
                }
                Err(e) => {
                    eprintln!("cut: {}: {}", file, e);
                    return 1;
                }
            }
        }
    }

    0
}

fn parse_ranges(spec: &str) -> Vec<usize> {
    let mut result = Vec::new();
    for part in spec.split(',') {
        if let Some((start, end)) = part.split_once('-') {
            let start: usize = start.parse().unwrap_or(1);
            let end: usize = end.parse().unwrap_or(start);
            for i in start..=end {
                result.push(i);
            }
        } else if let Ok(n) = part.parse::<usize>() {
            result.push(n);
        }
    }
    result
}
