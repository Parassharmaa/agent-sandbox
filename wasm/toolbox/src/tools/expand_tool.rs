use std::fs;
use std::io::{self, BufRead};

pub fn run(args: &[String]) -> i32 {
    let mut tab_size = 8;
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-t" => {
                i += 1;
                if i < args.len() {
                    tab_size = args[i].parse().unwrap_or(8);
                }
            }
            arg if arg.starts_with("-t") => {
                tab_size = arg[2..].parse().unwrap_or(8);
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
                    eprintln!("expand: {}: {}", file, e);
                    exit_code = 1;
                    continue;
                }
            }
        };

        for line in &lines {
            let mut col = 0;
            for ch in line.chars() {
                if ch == '\t' {
                    let spaces = tab_size - (col % tab_size);
                    for _ in 0..spaces {
                        print!(" ");
                    }
                    col += spaces;
                } else {
                    print!("{}", ch);
                    col += 1;
                }
            }
            println!();
        }
    }
    exit_code
}
