use std::fs;
use std::path::Path;

pub fn run(args: &[String]) -> i32 {
    let mut symbolic = false;
    let mut force = false;
    let mut positional = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-s" => symbolic = true,
            "-f" => force = true,
            "-sf" | "-fs" => {
                symbolic = true;
                force = true;
            }
            _ if arg.starts_with('-') && arg.len() > 1 => {
                for ch in arg[1..].chars() {
                    match ch {
                        's' => symbolic = true,
                        'f' => force = true,
                        _ => {
                            eprintln!("ln: invalid option -- '{}'", ch);
                            return 1;
                        }
                    }
                }
            }
            _ => positional.push(arg.clone()),
        }
    }

    if positional.len() < 2 {
        eprintln!("ln: missing file operand");
        eprintln!("Usage: ln [-sf] TARGET LINK_NAME");
        return 1;
    }

    let target = &positional[0];
    let link_name = &positional[1];

    if force
        && Path::new(link_name).exists()
        && let Err(e) = fs::remove_file(link_name)
    {
        eprintln!("ln: cannot remove '{}': {}", link_name, e);
        return 1;
    }

    if symbolic {
        #[cfg(unix)]
        {
            if let Err(e) = std::os::unix::fs::symlink(target, link_name) {
                eprintln!("ln: failed to create symbolic link '{}': {}", link_name, e);
                return 1;
            }
        }
        #[cfg(not(unix))]
        {
            eprintln!("ln: symbolic links not supported on this platform");
            return 1;
        }
    } else if let Err(e) = fs::hard_link(target, link_name) {
        eprintln!("ln: failed to create hard link '{}': {}", link_name, e);
        return 1;
    }

    0
}
