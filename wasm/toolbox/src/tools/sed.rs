use std::fs;
use std::io::{self, BufRead, Write};

use regex::Regex;

pub fn run(args: &[String]) -> i32 {
    let mut in_place = false;
    let mut expressions: Vec<String> = Vec::new();
    let mut files: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-i" => in_place = true,
            "-e" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("sed: option requires an argument -- 'e'");
                    return 1;
                }
                expressions.push(args[i].clone());
            }
            arg if !arg.starts_with('-') => {
                if expressions.is_empty() {
                    expressions.push(arg.to_string());
                } else {
                    files.push(arg.to_string());
                }
            }
            _ => {
                eprintln!("sed: invalid option '{}'", args[i]);
                return 1;
            }
        }
        i += 1;
    }

    if expressions.is_empty() {
        eprintln!("sed: no expression provided");
        return 1;
    }

    // Parse s/pattern/replacement/flags expressions
    let mut replacements = Vec::new();
    for expr in &expressions {
        if let Some(r) = parse_substitution(expr) {
            replacements.push(r);
        } else {
            eprintln!("sed: unsupported expression: '{}'", expr);
            return 1;
        }
    }

    if files.is_empty() {
        let stdin = io::stdin();
        let stdout = io::stdout();
        let mut out = stdout.lock();
        for line in stdin.lock().lines() {
            match line {
                Ok(line) => {
                    let result = apply_replacements(&line, &replacements);
                    writeln!(out, "{}", result).ok();
                }
                Err(e) => {
                    eprintln!("sed: {}", e);
                    return 1;
                }
            }
        }
    } else {
        for file in &files {
            match fs::read_to_string(file) {
                Ok(content) => {
                    let mut output = String::new();
                    for line in content.lines() {
                        let result = apply_replacements(line, &replacements);
                        output.push_str(&result);
                        output.push('\n');
                    }
                    // Remove trailing newline if original didn't have one
                    if !content.ends_with('\n') && output.ends_with('\n') {
                        output.pop();
                    }
                    if in_place {
                        if let Err(e) = fs::write(file, &output) {
                            eprintln!("sed: {}: {}", file, e);
                            return 1;
                        }
                    } else {
                        print!("{}", output);
                    }
                }
                Err(e) => {
                    eprintln!("sed: {}: {}", file, e);
                    return 1;
                }
            }
        }
    }

    0
}

struct Substitution {
    re: Regex,
    replacement: String,
    global: bool,
}

fn parse_substitution(expr: &str) -> Option<Substitution> {
    if !expr.starts_with('s') || expr.len() < 4 {
        return None;
    }

    let delim = expr.as_bytes()[1] as char;
    let rest = &expr[2..];

    let parts: Vec<&str> = rest.splitn(3, delim).collect();
    if parts.len() < 2 {
        return None;
    }

    let pattern = parts[0];
    let replacement = parts[1];
    let flags = if parts.len() > 2 { parts[2] } else { "" };
    let global = flags.contains('g');
    let case_insensitive = flags.contains('i');

    let pattern = if case_insensitive {
        format!("(?i){}", pattern)
    } else {
        pattern.to_string()
    };

    let re = Regex::new(&pattern).ok()?;

    Some(Substitution {
        re,
        replacement: replacement.to_string(),
        global,
    })
}

fn apply_replacements(line: &str, replacements: &[Substitution]) -> String {
    let mut result = line.to_string();
    for sub in replacements {
        if sub.global {
            result = sub
                .re
                .replace_all(&result, sub.replacement.as_str())
                .to_string();
        } else {
            result = sub
                .re
                .replace(&result, sub.replacement.as_str())
                .to_string();
        }
    }
    result
}
