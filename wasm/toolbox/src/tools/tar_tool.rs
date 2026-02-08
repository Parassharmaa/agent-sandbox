use std::fs::File;
use std::io::{self, BufReader, BufWriter};
use std::path::Path;

pub fn run(args: &[String]) -> i32 {
    let mut create = false;
    let mut extract = false;
    let mut list = false;
    let mut verbose = false;
    let mut gzip = false;
    let mut archive_file: Option<String> = None;
    let mut files: Vec<String> = Vec::new();
    let mut directory: Option<String> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-c" => create = true,
            "-x" => extract = true,
            "-t" => list = true,
            "-v" => verbose = true,
            "-z" => gzip = true,
            "-f" => {
                i += 1;
                if i < args.len() {
                    archive_file = Some(args[i].clone());
                }
            }
            "-C" => {
                i += 1;
                if i < args.len() {
                    directory = Some(args[i].clone());
                }
            }
            arg if arg.starts_with('-') && !arg.starts_with("--") && arg.len() > 1 => {
                let chars: Vec<char> = arg[1..].chars().collect();
                let mut j = 0;
                while j < chars.len() {
                    match chars[j] {
                        'c' => create = true,
                        'x' => extract = true,
                        't' => list = true,
                        'v' => verbose = true,
                        'z' => gzip = true,
                        'f' => {
                            // Rest of the chars or next arg is the filename
                            let rest: String = chars[j + 1..].iter().collect();
                            if !rest.is_empty() {
                                archive_file = Some(rest);
                            } else {
                                i += 1;
                                if i < args.len() {
                                    archive_file = Some(args[i].clone());
                                }
                            }
                            j = chars.len(); // skip rest
                        }
                        'C' => {
                            i += 1;
                            if i < args.len() {
                                directory = Some(args[i].clone());
                            }
                            j = chars.len();
                        }
                        _ => {}
                    }
                    j += 1;
                }
            }
            _ => files.push(args[i].clone()),
        }
        i += 1;
    }

    // Auto-detect gzip from filename
    if let Some(ref f) = archive_file
        && (f.ends_with(".gz") || f.ends_with(".tgz"))
    {
        gzip = true;
    }

    if create {
        create_tar(&archive_file, &files, gzip, verbose)
    } else if extract {
        extract_tar(&archive_file, &directory, gzip, verbose)
    } else if list {
        list_tar(&archive_file, gzip, verbose)
    } else {
        eprintln!("tar: you must specify one of -c, -x, -t");
        1
    }
}

fn create_tar(archive_file: &Option<String>, files: &[String], gzip: bool, verbose: bool) -> i32 {
    let writer: Box<dyn io::Write> = match archive_file {
        Some(f) => match File::create(f) {
            Ok(f) => Box::new(BufWriter::new(f)),
            Err(e) => {
                eprintln!("tar: {}: {}", f, e);
                return 1;
            }
        },
        None => Box::new(io::stdout()),
    };

    let writer: Box<dyn io::Write> = if gzip {
        Box::new(flate2::write::GzEncoder::new(
            writer,
            flate2::Compression::default(),
        ))
    } else {
        writer
    };

    let mut ar = tar::Builder::new(writer);

    for file in files {
        let path = Path::new(file);
        if path.is_dir() {
            if let Err(e) = ar.append_dir_all(file, file) {
                eprintln!("tar: {}: {}", file, e);
                return 1;
            }
        } else {
            match File::open(file) {
                Ok(mut f) => {
                    if let Err(e) = ar.append_file(file, &mut f) {
                        eprintln!("tar: {}: {}", file, e);
                        return 1;
                    }
                }
                Err(e) => {
                    eprintln!("tar: {}: {}", file, e);
                    return 1;
                }
            }
        }
        if verbose {
            eprintln!("{}", file);
        }
    }

    if let Err(e) = ar.finish() {
        eprintln!("tar: {}", e);
        return 1;
    }

    0
}

fn extract_tar(
    archive_file: &Option<String>,
    directory: &Option<String>,
    gzip: bool,
    verbose: bool,
) -> i32 {
    let reader: Box<dyn io::Read> = match archive_file {
        Some(f) => match File::open(f) {
            Ok(f) => Box::new(BufReader::new(f)),
            Err(e) => {
                eprintln!("tar: {}: {}", f, e);
                return 1;
            }
        },
        None => Box::new(io::stdin()),
    };

    let reader: Box<dyn io::Read> = if gzip {
        Box::new(flate2::read::GzDecoder::new(reader))
    } else {
        reader
    };

    let mut ar = tar::Archive::new(reader);
    let dest = directory.as_deref().unwrap_or(".");

    for entry in ar.entries().unwrap() {
        match entry {
            Ok(mut entry) => {
                if verbose && let Ok(path) = entry.path() {
                    eprintln!("{}", path.display());
                }
                if let Err(e) = entry.unpack_in(dest) {
                    eprintln!("tar: {}", e);
                    return 1;
                }
            }
            Err(e) => {
                eprintln!("tar: {}", e);
                return 1;
            }
        }
    }

    0
}

fn list_tar(archive_file: &Option<String>, gzip: bool, verbose: bool) -> i32 {
    let reader: Box<dyn io::Read> = match archive_file {
        Some(f) => match File::open(f) {
            Ok(f) => Box::new(BufReader::new(f)),
            Err(e) => {
                eprintln!("tar: {}: {}", f, e);
                return 1;
            }
        },
        None => Box::new(io::stdin()),
    };

    let reader: Box<dyn io::Read> = if gzip {
        Box::new(flate2::read::GzDecoder::new(reader))
    } else {
        reader
    };

    let mut ar = tar::Archive::new(reader);

    for entry in ar.entries().unwrap() {
        match entry {
            Ok(entry) => {
                if let Ok(path) = entry.path() {
                    if verbose {
                        println!("{:>10} {}", entry.size(), path.display());
                    } else {
                        println!("{}", path.display());
                    }
                }
            }
            Err(e) => {
                eprintln!("tar: {}", e);
                return 1;
            }
        }
    }

    0
}
