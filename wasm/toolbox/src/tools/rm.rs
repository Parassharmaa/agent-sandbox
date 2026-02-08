use std::fs;
use std::path::Path;

pub fn run(args: &[String]) -> i32 {
    let mut recursive = false;
    let mut force = false;
    let mut paths = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-r" | "-R" => recursive = true,
            "-f" => force = true,
            "-rf" | "-fr" => {
                recursive = true;
                force = true;
            }
            arg if arg.starts_with('-') && arg.len() > 1 => {
                for ch in arg[1..].chars() {
                    match ch {
                        'r' | 'R' => recursive = true,
                        'f' => force = true,
                        _ => {
                            eprintln!("rm: invalid option -- '{}'", ch);
                            return 1;
                        }
                    }
                }
            }
            _ => paths.push(arg.clone()),
        }
    }

    if paths.is_empty() {
        if !force {
            eprintln!("rm: missing operand");
            return 1;
        }
        return 0;
    }

    let mut exit_code = 0;
    for path in &paths {
        let p = Path::new(path);
        if !p.exists() {
            if !force {
                eprintln!("rm: cannot remove '{}': No such file or directory", path);
                exit_code = 1;
            }
            continue;
        }

        let result = if p.is_dir() {
            if recursive {
                fs::remove_dir_all(path)
            } else {
                eprintln!("rm: cannot remove '{}': Is a directory", path);
                exit_code = 1;
                continue;
            }
        } else {
            fs::remove_file(path)
        };

        if let Err(e) = result
            && !force
        {
            eprintln!("rm: cannot remove '{}': {}", path, e);
            exit_code = 1;
        }
    }

    exit_code
}
