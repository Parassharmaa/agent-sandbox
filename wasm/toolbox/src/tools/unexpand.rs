use std::fs;
use std::io::{self, BufRead};

pub fn run(args: &[String]) -> i32 {
    let mut tab_size = 8;
    let mut all = false;
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-a" | "--all" => all = true,
            "-t" => {
                i += 1;
                if i < args.len() {
                    tab_size = args[i].parse().unwrap_or(8);
                    all = true;
                }
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
                    eprintln!("unexpand: {}: {}", file, e);
                    exit_code = 1;
                    continue;
                }
            }
        };

        for line in &lines {
            if all {
                print_unexpanded(line, tab_size);
            } else {
                // Only convert leading spaces
                let leading_spaces = line.len() - line.trim_start_matches(' ').len();
                let tabs = leading_spaces / tab_size;
                let remaining_spaces = leading_spaces % tab_size;
                for _ in 0..tabs {
                    print!("\t");
                }
                for _ in 0..remaining_spaces {
                    print!(" ");
                }
                print!("{}", &line[leading_spaces..]);
            }
            println!();
        }
    }
    exit_code
}

fn print_unexpanded(line: &str, tab_size: usize) {
    let mut col = 0;
    let mut space_count = 0;

    for ch in line.chars() {
        if ch == ' ' {
            space_count += 1;
            col += 1;
            if col % tab_size == 0 && space_count > 0 {
                print!("\t");
                space_count = 0;
            }
        } else {
            for _ in 0..space_count {
                print!(" ");
            }
            space_count = 0;
            print!("{}", ch);
            col += 1;
        }
    }
    for _ in 0..space_count {
        print!(" ");
    }
}
