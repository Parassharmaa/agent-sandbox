use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

pub fn run(args: &[String]) -> i32 {
    let mut num_lines: usize = 10;
    let mut num_bytes: Option<usize> = None;
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("head: option requires an argument -- 'n'");
                    return 1;
                }
                match args[i].parse() {
                    Ok(n) => num_lines = n,
                    Err(_) => {
                        eprintln!("head: invalid number of lines: '{}'", args[i]);
                        return 1;
                    }
                }
            }
            "-c" => {
                i += 1;
                if i >= args.len() {
                    eprintln!("head: option requires an argument -- 'c'");
                    return 1;
                }
                match args[i].parse() {
                    Ok(n) => num_bytes = Some(n),
                    Err(_) => {
                        eprintln!("head: invalid number of bytes: '{}'", args[i]);
                        return 1;
                    }
                }
            }
            arg if arg.starts_with('-') && arg.len() > 1 => {
                // Try parsing as -N (number)
                if let Ok(n) = arg[1..].parse::<usize>() {
                    num_lines = n;
                } else {
                    eprintln!("head: invalid option -- '{}'", arg);
                    return 1;
                }
            }
            _ => files.push(args[i].clone()),
        }
        i += 1;
    }

    if files.is_empty() {
        files.push("-".to_string());
    }

    let multiple = files.len() > 1;
    let mut exit_code = 0;

    for (idx, file) in files.iter().enumerate() {
        if multiple {
            if idx > 0 {
                println!();
            }
            println!(
                "==> {} <==",
                if file == "-" { "standard input" } else { file }
            );
        }

        let result = if file == "-" {
            process_head(io::stdin().lock(), num_lines, num_bytes)
        } else {
            match File::open(file) {
                Ok(f) => process_head(BufReader::new(f), num_lines, num_bytes),
                Err(e) => {
                    eprintln!("head: {}: {}", file, e);
                    exit_code = 1;
                    continue;
                }
            }
        };

        if let Err(e) = result {
            eprintln!("head: {}", e);
            exit_code = 1;
        }
    }

    exit_code
}

fn process_head<R: BufRead>(
    mut reader: R,
    num_lines: usize,
    num_bytes: Option<usize>,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut out = stdout.lock();

    if let Some(bytes) = num_bytes {
        let mut buf = vec![0u8; bytes];
        let n = reader.read(&mut buf)?;
        out.write_all(&buf[..n])?;
    } else {
        for _ in 0..num_lines {
            let mut line = String::new();
            let n = reader.read_line(&mut line)?;
            if n == 0 {
                break;
            }
            write!(out, "{}", line)?;
        }
    }

    Ok(())
}
