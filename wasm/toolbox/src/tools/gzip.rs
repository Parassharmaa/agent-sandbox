use std::fs::{self, File};
use std::io::{self, Read, Write};

pub fn run(args: &[String]) -> i32 {
    let mut decompress = false;
    let mut keep = false;
    let mut stdout_mode = false;
    let mut files = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-d" => decompress = true,
            "-k" => keep = true,
            "-c" => stdout_mode = true,
            _ if !arg.starts_with('-') => files.push(arg.clone()),
            _ => {}
        }
    }

    if files.is_empty() {
        // Read from stdin, write to stdout
        if decompress {
            let mut decoder = flate2::read::GzDecoder::new(io::stdin());
            let mut buf = Vec::new();
            if let Err(e) = decoder.read_to_end(&mut buf) {
                eprintln!("gzip: {}", e);
                return 1;
            }
            io::stdout().write_all(&buf).ok();
        } else {
            let mut input = Vec::new();
            if let Err(e) = io::stdin().read_to_end(&mut input) {
                eprintln!("gzip: {}", e);
                return 1;
            }
            let mut encoder =
                flate2::write::GzEncoder::new(io::stdout(), flate2::Compression::default());
            encoder.write_all(&input).ok();
            encoder.finish().ok();
        }
        return 0;
    }

    let mut exit_code = 0;
    for file in &files {
        if decompress {
            if !file.ends_with(".gz") {
                eprintln!("gzip: {}: unknown suffix -- ignored", file);
                exit_code = 1;
                continue;
            }
            let output_name = &file[..file.len() - 3];

            match File::open(file) {
                Ok(f) => {
                    let mut decoder = flate2::read::GzDecoder::new(f);
                    let mut buf = Vec::new();
                    if let Err(e) = decoder.read_to_end(&mut buf) {
                        eprintln!("gzip: {}: {}", file, e);
                        exit_code = 1;
                        continue;
                    }
                    if stdout_mode {
                        io::stdout().write_all(&buf).ok();
                    } else {
                        if let Err(e) = fs::write(output_name, &buf) {
                            eprintln!("gzip: {}: {}", output_name, e);
                            exit_code = 1;
                            continue;
                        }
                        if !keep {
                            fs::remove_file(file).ok();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("gzip: {}: {}", file, e);
                    exit_code = 1;
                }
            }
        } else {
            let output_name = format!("{}.gz", file);

            match fs::read(file) {
                Ok(data) => {
                    if stdout_mode {
                        let mut encoder = flate2::write::GzEncoder::new(
                            io::stdout(),
                            flate2::Compression::default(),
                        );
                        encoder.write_all(&data).ok();
                        encoder.finish().ok();
                    } else {
                        match File::create(&output_name) {
                            Ok(out) => {
                                let mut encoder = flate2::write::GzEncoder::new(
                                    out,
                                    flate2::Compression::default(),
                                );
                                if let Err(e) = encoder.write_all(&data) {
                                    eprintln!("gzip: {}", e);
                                    exit_code = 1;
                                    continue;
                                }
                                if let Err(e) = encoder.finish() {
                                    eprintln!("gzip: {}", e);
                                    exit_code = 1;
                                    continue;
                                }
                                if !keep {
                                    fs::remove_file(file).ok();
                                }
                            }
                            Err(e) => {
                                eprintln!("gzip: {}: {}", output_name, e);
                                exit_code = 1;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("gzip: {}: {}", file, e);
                    exit_code = 1;
                }
            }
        }
    }

    exit_code
}
