use walkdir::WalkDir;

pub fn run(args: &[String]) -> i32 {
    let mut paths: Vec<String> = Vec::new();
    let mut name_pattern: Option<String> = None;
    let mut type_filter: Option<char> = None;
    let mut max_depth: Option<usize> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-name" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("find: missing argument to '-name'");
                    return 1;
                }
                name_pattern = Some(args[i].clone());
            }
            "-type" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("find: missing argument to '-type'");
                    return 1;
                }
                type_filter = args[i].chars().next();
            }
            "-maxdepth" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("find: missing argument to '-maxdepth'");
                    return 1;
                }
                match args[i].parse() {
                    Ok(d) => max_depth = Some(d),
                    Err(_) => {
                        eprintln!("find: invalid argument to '-maxdepth': '{}'", args[i]);
                        return 1;
                    }
                }
            }
            arg if !arg.starts_with('-') => {
                paths.push(arg.to_string());
            }
            _ => {
                eprintln!("find: unknown predicate '{}'", args[i]);
                return 1;
            }
        }
        i += 1;
    }

    if paths.is_empty() {
        paths.push(".".to_string());
    }

    for path in &paths {
        let mut walker = WalkDir::new(path);
        if let Some(depth) = max_depth {
            walker = walker.max_depth(depth);
        }

        for entry in walker.into_iter().filter_map(|e| e.ok()) {
            let file_type = entry.file_type();

            // Type filter
            if let Some(tf) = type_filter {
                match tf {
                    'f' if !file_type.is_file() => continue,
                    'd' if !file_type.is_dir() => continue,
                    'l' if !file_type.is_symlink() => continue,
                    _ => {}
                }
            }

            // Name pattern (simple glob matching)
            if let Some(ref pattern) = name_pattern {
                let name = entry.file_name().to_string_lossy();
                if !glob_match(pattern, &name) {
                    continue;
                }
            }

            println!("{}", entry.path().display());
        }
    }

    0
}

fn glob_match(pattern: &str, name: &str) -> bool {
    let pattern = pattern.as_bytes();
    let name = name.as_bytes();
    glob_match_impl(pattern, name, 0, 0)
}

fn glob_match_impl(pattern: &[u8], name: &[u8], pi: usize, ni: usize) -> bool {
    if pi == pattern.len() && ni == name.len() {
        return true;
    }
    if pi == pattern.len() {
        return false;
    }

    match pattern[pi] {
        b'*' => {
            // Try matching zero or more characters
            for j in ni..=name.len() {
                if glob_match_impl(pattern, name, pi + 1, j) {
                    return true;
                }
            }
            false
        }
        b'?' => {
            if ni < name.len() {
                glob_match_impl(pattern, name, pi + 1, ni + 1)
            } else {
                false
            }
        }
        c => {
            if ni < name.len() && name[ni] == c {
                glob_match_impl(pattern, name, pi + 1, ni + 1)
            } else {
                false
            }
        }
    }
}
