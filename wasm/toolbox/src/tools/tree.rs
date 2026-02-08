use std::fs;
use std::path::Path;

pub fn run(args: &[String]) -> i32 {
    let mut max_depth: Option<usize> = None;
    let mut dirs_only = false;
    let mut path = ".".to_string();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-L" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("tree: missing argument to '-L'");
                    return 1;
                }
                match args[i].parse() {
                    Ok(d) => max_depth = Some(d),
                    Err(_) => {
                        eprintln!("tree: invalid level: '{}'", args[i]);
                        return 1;
                    }
                }
            }
            "-d" => dirs_only = true,
            arg if !arg.starts_with('-') => path = arg.to_string(),
            _ => {
                eprintln!("tree: unknown option '{}'", args[i]);
                return 1;
            }
        }
        i += 1;
    }

    println!("{}", path);
    let mut dir_count = 0;
    let mut file_count = 0;
    print_tree(
        Path::new(&path),
        "",
        max_depth,
        0,
        dirs_only,
        &mut dir_count,
        &mut file_count,
    );
    if dirs_only {
        println!("\n{} directories", dir_count);
    } else {
        println!("\n{} directories, {} files", dir_count, file_count);
    }

    0
}

fn print_tree(
    path: &Path,
    prefix: &str,
    max_depth: Option<usize>,
    depth: usize,
    dirs_only: bool,
    dir_count: &mut usize,
    file_count: &mut usize,
) {
    if let Some(max) = max_depth
        && depth >= max
    {
        return;
    }

    let mut entries: Vec<_> = match fs::read_dir(path) {
        Ok(rd) => rd.filter_map(|e| e.ok()).collect(),
        Err(_) => return,
    };

    entries.sort_by_key(|e| e.file_name());

    let count = entries.len();
    for (idx, entry) in entries.iter().enumerate() {
        let is_last = idx == count - 1;
        let connector = if is_last { "└── " } else { "├── " };
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        if dirs_only && !is_dir {
            continue;
        }

        if is_dir {
            *dir_count += 1;
            println!("{}{}{}", prefix, connector, name);
            let new_prefix = if is_last {
                format!("{}    ", prefix)
            } else {
                format!("{}│   ", prefix)
            };
            print_tree(
                &entry.path(),
                &new_prefix,
                max_depth,
                depth + 1,
                dirs_only,
                dir_count,
                file_count,
            );
        } else {
            *file_count += 1;
            println!("{}{}{}", prefix, connector, name);
        }
    }
}
