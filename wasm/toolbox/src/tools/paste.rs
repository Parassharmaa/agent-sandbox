use std::fs;
use std::io::{self, BufRead};

pub fn run(args: &[String]) -> i32 {
    let mut delimiter = "\t".to_string();
    let mut serial = false;
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-d" => {
                i += 1;
                if i < args.len() {
                    delimiter = args[i].clone();
                }
            }
            "-s" | "--serial" => serial = true,
            arg if arg.starts_with("-d") => {
                delimiter = arg[2..].to_string();
            }
            _ => files.push(args[i].clone()),
        }
        i += 1;
    }

    if files.is_empty() {
        files.push("-".to_string());
    }

    if serial {
        for file in &files {
            let lines = read_lines(file);
            match lines {
                Ok(lines) => println!("{}", lines.join(&delimiter)),
                Err(e) => {
                    eprintln!("paste: {}: {}", file, e);
                    return 1;
                }
            }
        }
    } else {
        let all_lines: Vec<Vec<String>> = files
            .iter()
            .map(|f| read_lines(f).unwrap_or_default())
            .collect();

        let max_lines = all_lines.iter().map(|l| l.len()).max().unwrap_or(0);

        for i in 0..max_lines {
            let parts: Vec<&str> = all_lines
                .iter()
                .map(|lines| {
                    if i < lines.len() {
                        lines[i].as_str()
                    } else {
                        ""
                    }
                })
                .collect();
            println!("{}", parts.join(&delimiter));
        }
    }

    0
}

fn read_lines(file: &str) -> io::Result<Vec<String>> {
    if file == "-" {
        Ok(io::stdin().lock().lines().map_while(|l| l.ok()).collect())
    } else {
        let content = fs::read_to_string(file)?;
        Ok(content.lines().map(String::from).collect())
    }
}
