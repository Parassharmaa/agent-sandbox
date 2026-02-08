use std::fs;

pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("touch: missing file operand");
        return 1;
    }

    let mut exit_code = 0;
    for file in args {
        if file.starts_with('-') {
            continue;
        }
        if let Err(e) = fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(file)
        {
            eprintln!("touch: cannot touch '{}': {}", file, e);
            exit_code = 1;
        }
    }

    exit_code
}
