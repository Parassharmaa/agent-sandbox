use std::fs;
use std::io::{self, BufRead};

pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        let stdin = io::stdin();
        for line in stdin.lock().lines().map_while(|l| l.ok()) {
            println!("{}", line.chars().rev().collect::<String>());
        }
        return 0;
    }

    let mut exit_code = 0;
    for file in args {
        match fs::read_to_string(file) {
            Ok(content) => {
                for line in content.lines() {
                    println!("{}", line.chars().rev().collect::<String>());
                }
            }
            Err(e) => {
                eprintln!("rev: {}: {}", file, e);
                exit_code = 1;
            }
        }
    }
    exit_code
}
