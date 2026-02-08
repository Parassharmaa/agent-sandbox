use std::fs;
use std::path::Path;

pub fn run(args: &[String]) -> i32 {
    let mut summary = false;
    let mut human_readable = false;
    let mut paths = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-s" => summary = true,
            "-h" => human_readable = true,
            "-sh" | "-hs" => {
                summary = true;
                human_readable = true;
            }
            _ if !arg.starts_with('-') => paths.push(arg.clone()),
            _ => {}
        }
    }

    if paths.is_empty() {
        paths.push(".".to_string());
    }

    for path in &paths {
        let size = dir_size(Path::new(path), summary, human_readable);
        print_size(size, human_readable);
        println!("\t{}", path);
    }

    0
}

fn dir_size(path: &Path, summary: bool, human_readable: bool) -> u64 {
    let mut total = 0u64;

    if path.is_file() {
        return fs::metadata(path).map(|m| m.len()).unwrap_or(0);
    }

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let sub_size = dir_size(&p, summary, human_readable);
                total += sub_size;
                if !summary {
                    print_size(sub_size, human_readable);
                    println!("\t{}", p.display());
                }
            } else {
                total += fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            }
        }
    }

    total
}

fn print_size(bytes: u64, human_readable: bool) {
    if human_readable {
        if bytes >= 1_073_741_824 {
            print!("{:.1}G", bytes as f64 / 1_073_741_824.0);
        } else if bytes >= 1_048_576 {
            print!("{:.1}M", bytes as f64 / 1_048_576.0);
        } else if bytes >= 1024 {
            print!("{:.1}K", bytes as f64 / 1024.0);
        } else {
            print!("{}", bytes);
        }
    } else {
        // Print in 1K blocks
        print!("{}", bytes.div_ceil(1024));
    }
}
