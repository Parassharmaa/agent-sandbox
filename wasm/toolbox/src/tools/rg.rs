use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use regex::Regex;
use walkdir::WalkDir;

pub fn run(args: &[String]) -> i32 {
    let mut case_insensitive = false;
    let mut line_numbers = false;
    let mut count_only = false;
    let mut invert = false;
    let mut files_with_matches = false;
    let mut json_output = false;
    let mut context_lines: usize = 0;
    let mut glob_patterns: Vec<String> = Vec::new();
    let mut type_filters: Vec<String> = Vec::new();
    let mut pattern_str: Option<String> = None;
    let mut paths: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-i" | "--ignore-case" => case_insensitive = true,
            "-n" | "--line-number" => line_numbers = true,
            "-c" | "--count" => count_only = true,
            "-v" | "--invert-match" => invert = true,
            "-l" | "--files-with-matches" => files_with_matches = true,
            "--json" => json_output = true,
            "-e" | "--regexp" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("rg: option requires an argument -- 'e'");
                    return 2;
                }
                pattern_str = Some(args[i].clone());
            }
            "-C" | "--context" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("rg: option requires an argument -- 'C'");
                    return 2;
                }
                context_lines = args[i].parse().unwrap_or(0);
            }
            "--glob" | "-g" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("rg: option requires an argument -- 'glob'");
                    return 2;
                }
                glob_patterns.push(args[i].clone());
            }
            "--type" | "-t" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("rg: option requires an argument -- 'type'");
                    return 2;
                }
                type_filters.push(args[i].clone());
            }
            arg if arg.starts_with("-C") => {
                context_lines = arg[2..].parse().unwrap_or(0);
            }
            _ => {
                if pattern_str.is_none() {
                    pattern_str = Some(args[i].clone());
                } else {
                    paths.push(args[i].clone());
                }
            }
        }
        i += 1;
    }

    let pattern_str = match pattern_str {
        Some(p) => p,
        None => {
            eprintln!("rg: no pattern given");
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
            eprintln!("rg: invalid pattern: {}", e);
            return 2;
        }
    };

    if paths.is_empty() {
        paths.push(".".to_string());
    }

    let mut all_files = Vec::new();
    for path in &paths {
        let p = Path::new(path);
        if p.is_file() {
            all_files.push(path.clone());
        } else if p.is_dir() {
            for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    let file_path = entry.path().to_string_lossy().to_string();
                    if matches_filters(&file_path, &glob_patterns, &type_filters) {
                        all_files.push(file_path);
                    }
                }
            }
        }
    }

    let show_filename = all_files.len() > 1;
    let mut found_match = false;

    for file in &all_files {
        let f = match File::open(file) {
            Ok(f) => f,
            Err(_) => continue,
        };

        let reader = BufReader::new(f);
        let lines: Vec<String> = reader.lines().map_while(|l| l.ok()).collect();
        let mut match_count = 0;
        let mut matched_lines: Vec<(usize, &str, bool)> = Vec::new();

        for (line_num, line) in lines.iter().enumerate() {
            let matches = re.is_match(line);
            let matches = if invert { !matches } else { matches };

            if matches {
                match_count += 1;
                if files_with_matches {
                    println!("{}", file);
                    found_match = true;
                    break;
                }
                matched_lines.push((line_num, line, true));
            }
        }

        if files_with_matches && match_count > 0 {
            continue;
        }

        if count_only {
            if show_filename {
                println!("{}:{}", file, match_count);
            } else {
                println!("{}", match_count);
            }
            if match_count > 0 {
                found_match = true;
            }
            continue;
        }

        if match_count > 0 {
            found_match = true;
        }

        // Handle context lines
        if context_lines > 0 {
            let mut display = vec![false; lines.len()];
            for &(line_num, _, _) in &matched_lines {
                let start = line_num.saturating_sub(context_lines);
                let end = (line_num + context_lines + 1).min(lines.len());
                for item in &mut display[start..end] {
                    *item = true;
                }
            }
            let match_set: std::collections::HashSet<usize> =
                matched_lines.iter().map(|&(n, _, _)| n).collect();
            let mut last_printed = None;
            for (idx, line) in lines.iter().enumerate() {
                if !display[idx] {
                    continue;
                }
                if let Some(last) = last_printed
                    && idx > last + 1
                {
                    println!("--");
                }
                last_printed = Some(idx);
                let is_match = match_set.contains(&idx);
                if json_output {
                    print_json_line(file, idx + 1, line, is_match);
                } else {
                    print_line(file, idx + 1, line, show_filename, true, is_match);
                }
            }
        } else {
            for &(line_num, line, _) in &matched_lines {
                if json_output {
                    print_json_line(file, line_num + 1, line, true);
                } else {
                    print_line(file, line_num + 1, line, show_filename, line_numbers, false);
                }
            }
        }
    }

    if found_match { 0 } else { 1 }
}

fn print_line(
    file: &str,
    line_num: usize,
    line: &str,
    show_filename: bool,
    show_line_num: bool,
    _context: bool,
) {
    if show_filename && show_line_num {
        println!("{}:{}:{}", file, line_num, line);
    } else if show_filename {
        println!("{}:{}", file, line);
    } else if show_line_num {
        println!("{}:{}", line_num, line);
    } else {
        println!("{}", line);
    }
}

fn print_json_line(file: &str, line_num: usize, line: &str, is_match: bool) {
    let kind = if is_match { "match" } else { "context" };
    // Simple JSON output compatible with rg --json
    println!(
        r#"{{"type":"{}","data":{{"path":{{"text":"{}"}},"lines":{{"text":"{}"}},"line_number":{}}}}}"#,
        kind,
        file.replace('\\', "\\\\").replace('"', "\\\""),
        line.replace('\\', "\\\\").replace('"', "\\\""),
        line_num
    );
}

fn matches_filters(path: &str, glob_patterns: &[String], type_filters: &[String]) -> bool {
    if glob_patterns.is_empty() && type_filters.is_empty() {
        return true;
    }

    if !type_filters.is_empty() {
        let ext = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");
        let matches_type = type_filters.iter().any(|t| match t.as_str() {
            "rust" | "rs" => ext == "rs",
            "js" | "javascript" => ext == "js" || ext == "mjs" || ext == "cjs",
            "ts" | "typescript" => ext == "ts" || ext == "mts" || ext == "cts",
            "py" | "python" => ext == "py",
            "go" => ext == "go",
            "java" => ext == "java",
            "c" => ext == "c" || ext == "h",
            "cpp" => ext == "cpp" || ext == "cc" || ext == "cxx" || ext == "hpp",
            "md" | "markdown" => ext == "md",
            "json" => ext == "json",
            "yaml" | "yml" => ext == "yaml" || ext == "yml",
            "toml" => ext == "toml",
            "html" => ext == "html" || ext == "htm",
            "css" => ext == "css",
            "sh" | "shell" => ext == "sh" || ext == "bash" || ext == "zsh",
            _ => ext == t.as_str(),
        });
        if !matches_type {
            return false;
        }
    }

    if !glob_patterns.is_empty() {
        let filename = Path::new(path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
        let matches_glob = glob_patterns.iter().any(|g| {
            if let Some(negated) = g.strip_prefix('!') {
                !simple_glob_match(negated, filename) && !simple_glob_match(negated, path)
            } else {
                simple_glob_match(g, filename) || simple_glob_match(g, path)
            }
        });
        if !matches_glob {
            return false;
        }
    }

    true
}

fn simple_glob_match(pattern: &str, text: &str) -> bool {
    // Simple glob: * matches anything, ? matches single char
    let pat_chars: Vec<char> = pattern.chars().collect();
    let text_chars: Vec<char> = text.chars().collect();

    glob_match_recursive(&pat_chars, &text_chars, 0, 0)
}

fn glob_match_recursive(pattern: &[char], text: &[char], pi: usize, ti: usize) -> bool {
    if pi == pattern.len() && ti == text.len() {
        return true;
    }
    if pi == pattern.len() {
        return false;
    }

    match pattern[pi] {
        '*' => {
            // Try matching * with 0 or more characters
            for skip in 0..=(text.len() - ti) {
                if glob_match_recursive(pattern, text, pi + 1, ti + skip) {
                    return true;
                }
            }
            false
        }
        '?' => {
            if ti < text.len() {
                glob_match_recursive(pattern, text, pi + 1, ti + 1)
            } else {
                false
            }
        }
        c => {
            if ti < text.len() && text[ti] == c {
                glob_match_recursive(pattern, text, pi + 1, ti + 1)
            } else {
                false
            }
        }
    }
}
