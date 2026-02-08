use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};

pub fn run(args: &[String]) -> i32 {
    let mut number_lines = false;
    let mut number_nonblank = false;
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-n" => number_lines = true,
            "-b" => number_nonblank = true,
            arg if arg.starts_with('-') && arg != "-" => {
                eprintln!("cat: invalid option -- '{}'", arg);
                return 1;
            }
            _ => files.push(args[i].clone()),
        }
        i += 1;
    }

    // If no files specified, read from stdin
    if files.is_empty() {
        files.push("-".to_string());
    }

    let mut line_number = 1;
    let mut exit_code = 0;

    for file in files {
        let result = if file == "-" {
            process_reader(
                io::stdin().lock(),
                &mut line_number,
                number_lines,
                number_nonblank,
            )
        } else {
            match File::open(&file) {
                Ok(f) => process_reader(
                    BufReader::new(f),
                    &mut line_number,
                    number_lines,
                    number_nonblank,
                ),
                Err(e) => {
                    eprintln!("cat: {}: {}", file, e);
                    exit_code = 1;
                    continue;
                }
            }
        };

        if let Err(e) = result {
            eprintln!("cat: {}", e);
            exit_code = 1;
        }
    }

    exit_code
}

fn process_reader<R: BufRead>(
    reader: R,
    line_number: &mut usize,
    number_lines: bool,
    number_nonblank: bool,
) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for line in reader.lines() {
        let line = line?;

        if number_lines {
            writeln!(handle, "{:6}\t{}", line_number, line)?;
            *line_number += 1;
        } else if number_nonblank {
            if line.is_empty() {
                writeln!(handle)?;
            } else {
                writeln!(handle, "{:6}\t{}", line_number, line)?;
                *line_number += 1;
            }
        } else {
            writeln!(handle, "{}", line)?;
        }
    }

    Ok(())
}
