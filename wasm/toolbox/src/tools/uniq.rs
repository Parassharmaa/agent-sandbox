use std::fs;
use std::io::{self, BufRead};

pub fn run(args: &[String]) -> i32 {
    let mut count = false;
    let mut repeated = false;
    let mut unique_only = false;
    let mut files = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-c" => count = true,
            "-d" => repeated = true,
            "-u" => unique_only = true,
            _ if arg.starts_with('-') && arg.len() > 1 => {
                eprintln!("uniq: invalid option '{}'", arg);
                return 1;
            }
            _ => files.push(arg.clone()),
        }
    }

    let lines: Vec<String> = if files.is_empty() {
        let stdin = io::stdin();
        stdin.lock().lines().map_while(|l| l.ok()).collect()
    } else {
        let mut all = Vec::new();
        for file in &files {
            match fs::read_to_string(file) {
                Ok(content) => {
                    for line in content.lines() {
                        all.push(line.to_string());
                    }
                }
                Err(e) => {
                    eprintln!("uniq: {}: {}", file, e);
                    return 1;
                }
            }
        }
        all
    };

    let mut groups: Vec<(usize, &str)> = Vec::new();
    for line in &lines {
        if let Some(last) = groups.last_mut()
            && last.1 == line.as_str()
        {
            last.0 += 1;
            continue;
        }
        groups.push((1, line));
    }

    for (cnt, line) in &groups {
        if repeated && *cnt < 2 {
            continue;
        }
        if unique_only && *cnt > 1 {
            continue;
        }
        if count {
            println!("{:7} {}", cnt, line);
        } else {
            println!("{}", line);
        }
    }

    0
}
