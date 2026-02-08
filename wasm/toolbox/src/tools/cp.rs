use std::fs;
use std::io;
use std::path::Path;

pub fn run(args: &[String]) -> i32 {
    let mut recursive = false;
    let mut paths = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-r" | "-R" => recursive = true,
            _ if arg.starts_with('-') => {
                eprintln!("cp: invalid option '{}'", arg);
                return 1;
            }
            _ => paths.push(arg.clone()),
        }
    }

    if paths.len() < 2 {
        eprintln!("cp: missing destination operand");
        return 1;
    }

    let dest = paths.last().unwrap().clone();
    let sources = &paths[..paths.len() - 1];

    let dest_is_dir = Path::new(&dest).is_dir();

    if sources.len() > 1 && !dest_is_dir {
        eprintln!("cp: target '{}' is not a directory", dest);
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

        let src_path = Path::new(src);
        if src_path.is_dir() {
            if !recursive {
                eprintln!("cp: -r not specified; omitting directory '{}'", src);
                exit_code = 1;
                continue;
            }
            if let Err(e) = copy_dir_recursive(src_path, Path::new(&target)) {
                eprintln!("cp: {}", e);
                exit_code = 1;
            }
        } else if let Err(e) = fs::copy(src, &target) {
            eprintln!("cp: cannot copy '{}': {}", src, e);
            exit_code = 1;
        }
    }

    exit_code
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> io::Result<()> {
    fs::create_dir_all(dest)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dest.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}
