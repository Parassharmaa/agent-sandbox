use std::fs::File;
use std::io::{self, BufRead, BufReader};

use regex::Regex;

pub fn run(args: &[String]) -> i32 {
    let mut case_insensitive = false;
    let mut line_numbers = false;
    let mut count_only = false;
    let mut invert = false;
    let mut recursive = false;
    let mut files_with_matches = false;
    let mut pattern_str: Option<String> = None;
    let mut files: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-i" => case_insensitive = true,
            "-n" => line_numbers = true,
            "-c" => count_only = true,
            "-v" => invert = true,
            "-r" | "-R" => recursive = true,
            "-l" => files_with_matches = true,
            "-e" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("grep: option requires an argument -- 'e'");
                    return 2;
                }
                pattern_str = Some(args[i].clone());
            }
            arg if arg.starts_with('-') && arg.len() > 1 => {
                // Handle combined flags like -in, -rn
                for ch in arg[1..].chars() {
                    match ch {
                        'i' => case_insensitive = true,
                        'n' => line_numbers = true,
                        'c' => count_only = true,
                        'v' => invert = true,
                        'r' | 'R' => recursive = true,
                        'l' => files_with_matches = true,
                        _ => {
                            eprintln!("grep: invalid option -- '{}'", ch);
                            return 2;
                        }
                    }
                }
            }
            _ => {
                if pattern_str.is_none() {
                    pattern_str = Some(args[i].clone());
                } else {
                    files.push(args[i].clone());
                }
            }
        }
        i += 1;
    }

    let pattern_str = match pattern_str {
        Some(p) => p,
        None => {
            eprintln!("grep: missing pattern");
            return 2;
        }
    };

    let pattern = if case_insensitive {
        format!("(?i){}", pattern_str)
    } else {
        pattern_str
    };

    let re = match Regex::new(&pattern) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("grep: invalid pattern: {}", e);
            return 2;
        }
    };

    if files.is_empty() {
        files.push("-".to_string());
    }

    // Expand recursive directories
    if recursive {
        let mut expanded = Vec::new();
        for file in &files {
            if file == "-" {
                expanded.push("-".to_string());
                continue;
            }
            let path = std::path::Path::new(file);
            if path.is_dir() {
                expand_dir(path, &mut expanded);
            } else {
                expanded.push(file.clone());
            }
        }
        files = expanded;
    }

    let show_filename = files.len() > 1;
    let mut found_match = false;

    for file in &files {
        if file == "-" {
            let stdin = io::stdin();
            let result = search_reader(
                stdin.lock(),
                "-",
                &re,
                show_filename,
                line_numbers,
                count_only,
                invert,
                files_with_matches,
            );
            if result {
                found_match = true;
            }
        } else {
            let path = std::path::Path::new(file);
            if path.is_dir() {
                continue;
            }
            match File::open(file) {
                Ok(f) => {
                    let result = search_reader(
                        BufReader::new(f),
                        file,
                        &re,
                        show_filename,
                        line_numbers,
                        count_only,
                        invert,
                        files_with_matches,
                    );
                    if result {
                        found_match = true;
                    }
                }
                Err(e) => {
                    eprintln!("grep: {}: {}", file, e);
                }
            }
        }
    }

    if found_match { 0 } else { 1 }
}

fn expand_dir(dir: &std::path::Path, files: &mut Vec<String>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                expand_dir(&path, files);
            } else {
                files.push(path.to_string_lossy().to_string());
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn search_reader<R: BufRead>(
    reader: R,
    filename: &str,
    re: &Regex,
    show_filename: bool,
    line_numbers: bool,
    count_only: bool,
    invert: bool,
    files_with_matches: bool,
) -> bool {
    let mut match_count = 0;

    for (line_num, line) in reader.lines().enumerate() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };

        let matches = re.is_match(&line);
        let matches = if invert { !matches } else { matches };

        if matches {
            match_count += 1;

            if files_with_matches {
                println!("{}", filename);
                return true;
            }

            if !count_only {
                let prefix = match (show_filename, line_numbers) {
                    (true, true) => format!("{}:{}:", filename, line_num + 1),
                    (true, false) => format!("{}:", filename),
                    (false, true) => format!("{}:", line_num + 1),
                    (false, false) => String::new(),
                };
                println!("{}{}", prefix, line);
            }
        }
    }

    if count_only {
        if show_filename {
            println!("{}:{}", filename, match_count);
        } else {
            println!("{}", match_count);
        }
    }

    match_count > 0
}
