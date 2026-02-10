use std::fs;
use std::io::{self, Read};

pub fn run(args: &[String]) -> i32 {
    let mut check = false;
    let mut files = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-c" | "--check" => check = true,
            _ => files.push(arg.as_str()),
        }
    }

    if check {
        return check_sums(&files);
    }

    if files.is_empty() {
        files.push("-");
    }

    let mut exit_code = 0;
    for file in files {
        match read_content(file) {
            Ok(data) => {
                let hash = format!("{}", sha1_smol::Sha1::from(&data).digest());
                if file == "-" {
                    println!("{}  -", hash);
                } else {
                    println!("{}  {}", hash, file);
                }
            }
            Err(e) => {
                eprintln!("sha1sum: {}: {}", file, e);
                exit_code = 1;
            }
        }
    }
    exit_code
}

fn read_content(file: &str) -> io::Result<Vec<u8>> {
    if file == "-" {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
        Ok(buf)
    } else {
        fs::read(file)
    }
}

fn check_sums(files: &[&str]) -> i32 {
    let mut exit_code = 0;
    for file in files {
        let content = match fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("sha1sum: {}: {}", file, e);
                return 1;
            }
        };

        for line in content.lines() {
            let parts: Vec<&str> = line.splitn(2, "  ").collect();
            if parts.len() != 2 {
                continue;
            }
            let expected = parts[0];
            let filename = parts[1];
            match fs::read(filename) {
                Ok(data) => {
                    let actual = format!("{}", sha1_smol::Sha1::from(&data).digest());
                    if actual == expected {
                        println!("{}: OK", filename);
                    } else {
                        println!("{}: FAILED", filename);
                        exit_code = 1;
                    }
                }
                Err(e) => {
                    eprintln!("sha1sum: {}: {}", filename, e);
                    exit_code = 1;
                }
            }
        }
    }
    exit_code
}
