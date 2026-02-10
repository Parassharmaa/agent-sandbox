use std::fs;

pub fn run(args: &[String]) -> i32 {
    let mut parents = false;
    let mut dirs = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-p" | "--parents" => parents = true,
            _ => dirs.push(arg.as_str()),
        }
    }

    if dirs.is_empty() {
        eprintln!("rmdir: missing operand");
        return 1;
    }

    let mut exit_code = 0;
    for dir in dirs {
        if parents {
            let mut path = std::path::PathBuf::from(dir);
            loop {
                match fs::remove_dir(&path) {
                    Ok(()) => {}
                    Err(e) => {
                        eprintln!("rmdir: {}: {}", path.display(), e);
                        exit_code = 1;
                        break;
                    }
                }
                if !path.pop() || path.as_os_str().is_empty() {
                    break;
                }
            }
        } else {
            match fs::remove_dir(dir) {
                Ok(()) => {}
                Err(e) => {
                    eprintln!("rmdir: {}: {}", dir, e);
                    exit_code = 1;
                }
            }
        }
    }
    exit_code
}
