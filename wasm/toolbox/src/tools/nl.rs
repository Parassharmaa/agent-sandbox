use std::fs;
use std::io::{self, BufRead};

pub fn run(args: &[String]) -> i32 {
    let mut body_numbering = 't'; // t=non-empty, a=all, n=none
    let mut separator = "\t".to_string();
    let mut width = 6;
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-ba" => body_numbering = 'a',
            "-bt" => body_numbering = 't',
            "-bn" => body_numbering = 'n',
            "-b" => {
                i += 1;
                if i < args.len() {
                    body_numbering = args[i].chars().next().unwrap_or('t');
                }
            }
            "-s" => {
                i += 1;
                if i < args.len() {
                    separator = args[i].clone();
                }
            }
            "-w" => {
                i += 1;
                if i < args.len() {
                    width = args[i].parse().unwrap_or(6);
                }
            }
            arg if arg.starts_with("-w") => {
                width = arg[2..].parse().unwrap_or(6);
            }
            arg if arg.starts_with("-s") => {
                separator = arg[2..].to_string();
            }
            _ => files.push(args[i].clone()),
        }
        i += 1;
    }

    if files.is_empty() {
        files.push("-".to_string());
    }

    let mut line_num = 1usize;
    let mut exit_code = 0;

    for file in &files {
        let lines: Vec<String> = if file == "-" {
            io::stdin().lock().lines().map_while(|l| l.ok()).collect()
        } else {
            match fs::read_to_string(file) {
                Ok(content) => content.lines().map(String::from).collect(),
                Err(e) => {
                    eprintln!("nl: {}: {}", file, e);
                    exit_code = 1;
                    continue;
                }
            }
        };

        for line in &lines {
            let should_number = match body_numbering {
                'a' => true,
                't' => !line.is_empty(),
                _ => false,
            };

            if should_number {
                println!("{:>width$}{}{}", line_num, separator, line, width = width);
                line_num += 1;
            } else {
                println!("{:>width$}{}{}", "", separator, line, width = width);
            }
        }
    }

    exit_code
}
