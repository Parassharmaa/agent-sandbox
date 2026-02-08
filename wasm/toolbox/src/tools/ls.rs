use std::fs;
use std::path::Path;

pub fn run(args: &[String]) -> i32 {
    let mut long_format = false;
    let mut all = false;
    let mut one_per_line = false;
    let mut paths = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-l" => long_format = true,
            "-a" => all = true,
            "-1" => one_per_line = true,
            arg if arg.starts_with('-') && arg.len() > 1 => {
                for ch in arg[1..].chars() {
                    match ch {
                        'l' => long_format = true,
                        'a' => all = true,
                        '1' => one_per_line = true,
                        _ => {
                            eprintln!("ls: invalid option -- '{}'", ch);
                            return 1;
                        }
                    }
                }
            }
            _ => paths.push(arg.clone()),
        }
    }

    if paths.is_empty() {
        paths.push(".".to_string());
    }

    let multiple = paths.len() > 1;
    let mut exit_code = 0;

    for (idx, path) in paths.iter().enumerate() {
        if multiple {
            if idx > 0 {
                println!();
            }
            println!("{}:", path);
        }

        let p = Path::new(path);
        if p.is_file() {
            print_entry(p, long_format);
            continue;
        }

        let entries = match fs::read_dir(path) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("ls: cannot access '{}': {}", path, e);
                exit_code = 1;
                continue;
            }
        };

        let mut items: Vec<_> = entries.filter_map(|e| e.ok()).collect();
        items.sort_by_key(|e| e.file_name());

        for entry in &items {
            let name = entry.file_name();
            let name = name.to_string_lossy();

            if !all && name.starts_with('.') {
                continue;
            }

            if long_format {
                print_entry(&entry.path(), true);
            } else if one_per_line {
                println!("{}", name);
            } else {
                print!("{}  ", name);
            }
        }

        if !long_format && !one_per_line {
            println!();
        }
    }

    exit_code
}

fn print_entry(path: &Path, long_format: bool) {
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    if !long_format {
        println!("{}", name);
        return;
    }

    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => {
            println!("?????????? ? ? ? ? ? {}", name);
            return;
        }
    };

    let file_type = if metadata.is_dir() {
        "d"
    } else if metadata.is_symlink() {
        "l"
    } else {
        "-"
    };

    let size = metadata.len();
    let suffix = if metadata.is_dir() { "/" } else { "" };

    println!("{}rwxr-xr-x {:>10} {}{}", file_type, size, name, suffix);
}
