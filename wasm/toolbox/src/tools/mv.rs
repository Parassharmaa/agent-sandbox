use std::fs;
use std::path::Path;

pub fn run(args: &[String]) -> i32 {
    let mut paths = Vec::new();

    for arg in args {
        if arg.starts_with('-') {
            continue; // Ignore flags for now
        }
        paths.push(arg.clone());
    }

    if paths.len() < 2 {
        eprintln!("mv: missing destination operand");
        return 1;
    }

    let dest = paths.last().unwrap().clone();
    let sources = &paths[..paths.len() - 1];
    let dest_is_dir = Path::new(&dest).is_dir();

    if sources.len() > 1 && !dest_is_dir {
        eprintln!("mv: target '{}' is not a directory", dest);
        return 1;
    }

    let mut exit_code = 0;
    for src in sources {
        let target = if dest_is_dir {
            let name = Path::new(src).file_name().unwrap_or_default();
            Path::new(&dest).join(name).to_string_lossy().to_string()
        } else {
            dest.clone()
        };

        if let Err(e) = fs::rename(src, &target) {
            eprintln!("mv: cannot move '{}' to '{}': {}", src, target, e);
            exit_code = 1;
        }
    }

    exit_code
}
