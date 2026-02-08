use std::fs;

pub fn run(args: &[String]) -> i32 {
    let mut parents = false;
    let mut dirs = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-p" => parents = true,
            _ if arg.starts_with('-') => {
                eprintln!("mkdir: invalid option '{}'", arg);
                return 1;
            }
            _ => dirs.push(arg.clone()),
        }
    }

    if dirs.is_empty() {
        eprintln!("mkdir: missing operand");
        return 1;
    }

    let mut exit_code = 0;
    for dir in &dirs {
        let result = if parents {
            fs::create_dir_all(dir)
        } else {
            fs::create_dir(dir)
        };
        if let Err(e) = result {
            eprintln!("mkdir: cannot create directory '{}': {}", dir, e);
            exit_code = 1;
        }
    }

    exit_code
}
