use std::fs;
use std::io::{self, BufRead};

pub fn run(args: &[String]) -> i32 {
    let mut count_lines = false;
    let mut count_words = false;
    let mut count_bytes = false;
    let mut count_chars = false;
    let mut files = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-l" => count_lines = true,
            "-w" => count_words = true,
            "-c" => count_bytes = true,
            "-m" => count_chars = true,
            _ if arg.starts_with('-') && arg.len() > 1 => {
                for ch in arg[1..].chars() {
                    match ch {
                        'l' => count_lines = true,
                        'w' => count_words = true,
                        'c' => count_bytes = true,
                        'm' => count_chars = true,
                        _ => {
                            eprintln!("wc: invalid option -- '{}'", ch);
                            return 1;
                        }
                    }
                }
            }
            _ => files.push(arg.clone()),
        }
    }

    // If no specific flags, show all
    let show_all = !count_lines && !count_words && !count_bytes && !count_chars;

    let mut total_lines = 0usize;
    let mut total_words = 0usize;
    let mut total_bytes = 0usize;

    if files.is_empty() {
        files.push("-".to_string());
    }

    for file in &files {
        let (lines, words, bytes) = if file == "-" {
            count_stdin()
        } else {
            match fs::read(file) {
                Ok(content) => count_content(&content),
                Err(e) => {
                    eprintln!("wc: {}: {}", file, e);
                    continue;
                }
            }
        };

        total_lines += lines;
        total_words += words;
        total_bytes += bytes;

        print_counts(
            lines,
            words,
            bytes,
            show_all,
            count_lines,
            count_words,
            count_bytes,
            count_chars,
        );
        if file != "-" {
            print!(" {}", file);
        }
        println!();
    }

    if files.len() > 1 {
        print_counts(
            total_lines,
            total_words,
            total_bytes,
            show_all,
            count_lines,
            count_words,
            count_bytes,
            count_chars,
        );
        println!(" total");
    }

    0
}

fn count_content(content: &[u8]) -> (usize, usize, usize) {
    let text = String::from_utf8_lossy(content);
    let lines = text.lines().count();
    let words = text.split_whitespace().count();
    let bytes = content.len();
    (lines, words, bytes)
}

fn count_stdin() -> (usize, usize, usize) {
    let stdin = io::stdin();
    let mut lines = 0;
    let mut words = 0;
    let mut bytes = 0;

    for line in stdin.lock().lines().map_while(|l| l.ok()) {
        lines += 1;
        words += line.split_whitespace().count();
        bytes += line.len() + 1; // +1 for newline
    }

    (lines, words, bytes)
}

#[allow(clippy::too_many_arguments)]
fn print_counts(
    lines: usize,
    words: usize,
    bytes: usize,
    show_all: bool,
    count_lines: bool,
    count_words: bool,
    count_bytes: bool,
    count_chars: bool,
) {
    if show_all || count_lines {
        print!("{:>8}", lines);
    }
    if show_all || count_words {
        print!("{:>8}", words);
    }
    if show_all || count_bytes || count_chars {
        print!("{:>8}", bytes);
    }
}
