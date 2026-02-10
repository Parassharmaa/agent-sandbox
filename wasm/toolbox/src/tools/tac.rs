use std::fs;
use std::io::{self, BufRead};

pub fn run(args: &[String]) -> i32 {
    let files: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    if files.is_empty() {
        // Read from stdin
        let stdin = io::stdin();
        let lines: Vec<String> = stdin.lock().lines().map_while(|l| l.ok()).collect();
        for line in lines.iter().rev() {
            println!("{}", line);
        }
        return 0;
    }

    let mut exit_code = 0;
    for file in files {
        match fs::read_to_string(file) {
            Ok(content) => {
                let lines: Vec<&str> = content.lines().collect();
                for line in lines.iter().rev() {
                    println!("{}", line);
                }
            }
            Err(e) => {
                eprintln!("tac: {}: {}", file, e);
                exit_code = 1;
            }
        }
    }
    exit_code
}
