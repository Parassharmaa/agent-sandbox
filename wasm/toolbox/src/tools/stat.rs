use std::fs;

pub fn run(args: &[String]) -> i32 {
    let files: Vec<&str> = args
        .iter()
        .map(|s| s.as_str())
        .filter(|s| !s.starts_with('-'))
        .collect();

    if files.is_empty() {
        eprintln!("stat: missing operand");
        return 1;
    }

    let mut exit_code = 0;
    for file in &files {
        match fs::metadata(file) {
            Ok(meta) => {
                let file_type = if meta.is_dir() {
                    "directory"
                } else if meta.is_symlink() {
                    "symbolic link"
                } else {
                    "regular file"
                };

                println!("  File: {}", file);
                println!("  Size: {}\tType: {}", meta.len(), file_type);
                println!("  Read-only: {}", meta.permissions().readonly());
            }
            Err(e) => {
                eprintln!("stat: cannot stat '{}': {}", file, e);
                exit_code = 1;
            }
        }
    }

    exit_code
}
