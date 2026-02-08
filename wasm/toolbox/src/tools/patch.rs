use std::fs;
use std::io::{self, BufRead};

pub fn run(args: &[String]) -> i32 {
    let mut input_file: Option<String> = None;
    let mut strip_level: usize = 0;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-i" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("patch: option requires an argument -- 'i'");
                    return 1;
                }
                input_file = Some(args[i].clone());
            }
            "-p" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("patch: option requires an argument -- 'p'");
                    return 1;
                }
                strip_level = args[i].parse().unwrap_or(0);
            }
            arg if arg.starts_with("-p") => {
                strip_level = arg[2..].parse().unwrap_or(0);
            }
            _ => {
                if input_file.is_none() {
                    input_file = Some(args[i].clone());
                }
            }
        }
        i += 1;
    }

    let patch_content = if let Some(ref file) = input_file {
        match fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("patch: {}: {}", file, e);
                return 1;
            }
        }
    } else {
        let stdin = io::stdin();
        let mut content = String::new();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => {
                    content.push_str(&l);
                    content.push('\n');
                }
                Err(e) => {
                    eprintln!("patch: {}", e);
                    return 1;
                }
            }
        }
        content
    };

    apply_unified_diff(&patch_content, strip_level)
}

fn strip_path(path: &str, level: usize) -> String {
    let parts: Vec<&str> = path.split('/').collect();
    if level >= parts.len() {
        parts.last().unwrap_or(&"").to_string()
    } else {
        parts[level..].join("/")
    }
}

fn apply_unified_diff(patch: &str, strip_level: usize) -> i32 {
    let lines: Vec<&str> = patch.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        // Find --- and +++ headers
        if i + 1 < lines.len() && lines[i].starts_with("--- ") && lines[i + 1].starts_with("+++ ") {
            let target_path = lines[i + 1]
                .strip_prefix("+++ ")
                .unwrap()
                .split('\t')
                .next()
                .unwrap();
            let target_path = strip_path(target_path, strip_level);
            i += 2;

            let original = match fs::read_to_string(&target_path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("patch: {}: {}", target_path, e);
                    return 1;
                }
            };

            let mut orig_lines: Vec<&str> = original.lines().collect();
            let mut offset: isize = 0;

            // Process hunks
            while i < lines.len() && lines[i].starts_with("@@ ") {
                // Parse @@ -old_start,old_count +new_start,new_count @@
                let hunk_header = lines[i];
                let (old_start, _old_count) = parse_hunk_range(hunk_header, '-');
                i += 1;

                let start = ((old_start as isize - 1) + offset) as usize;
                let mut pos = start;
                let mut removals = 0;
                let mut additions = Vec::new();

                while i < lines.len()
                    && !lines[i].starts_with("@@ ")
                    && !lines[i].starts_with("--- ")
                {
                    if let Some(_content) = lines[i].strip_prefix('-') {
                        removals += 1;
                        i += 1;
                    } else if let Some(content) = lines[i].strip_prefix('+') {
                        additions.push(content);
                        i += 1;
                    } else if lines[i].starts_with(' ') || lines[i].is_empty() {
                        // Context line - flush pending changes
                        if removals > 0 || !additions.is_empty() {
                            let drain_start = pos.saturating_sub(removals);
                            for _ in 0..removals {
                                if drain_start < orig_lines.len() {
                                    orig_lines.remove(drain_start);
                                }
                            }
                            for (j, add) in additions.iter().enumerate() {
                                orig_lines.insert(drain_start + j, add);
                            }
                            offset += additions.len() as isize - removals as isize;
                            pos = drain_start + additions.len();
                            removals = 0;
                            additions = Vec::new();
                        }
                        pos += 1;
                        i += 1;
                    } else {
                        break;
                    }
                }

                // Flush remaining changes
                if removals > 0 || !additions.is_empty() {
                    let drain_start = pos.saturating_sub(removals);
                    for _ in 0..removals {
                        if drain_start < orig_lines.len() {
                            orig_lines.remove(drain_start);
                        }
                    }
                    for (j, add) in additions.iter().enumerate() {
                        orig_lines.insert(drain_start + j, add);
                    }
                    offset += additions.len() as isize - removals as isize;
                }
            }

            let mut result = orig_lines.join("\n");
            if original.ends_with('\n') {
                result.push('\n');
            }

            if let Err(e) = fs::write(&target_path, &result) {
                eprintln!("patch: {}: {}", target_path, e);
                return 1;
            }
            eprintln!("patching file {}", target_path);
        } else {
            i += 1;
        }
    }

    0
}

fn parse_hunk_range(header: &str, marker: char) -> (usize, usize) {
    let search = format!("{}", marker);
    if let Some(pos) = header.find(&search) {
        let rest = &header[pos + 1..];
        let end = rest.find([' ', ',']).unwrap_or(rest.len());
        let start: usize = rest[..end].parse().unwrap_or(1);
        let count = if let Some(comma) = rest.find(',') {
            let after = &rest[comma + 1..];
            let end = after.find([' ', '+', '@']).unwrap_or(after.len());
            after[..end].parse().unwrap_or(1)
        } else {
            1
        };
        (start, count)
    } else {
        (1, 1)
    }
}
