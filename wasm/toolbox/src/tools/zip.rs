use std::fs::{self, File};
use std::io::{self, Write};
use std::path::Path;

use zip::ZipArchive;
use zip::ZipWriter;
use zip::write::SimpleFileOptions;

pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("Usage: zip <archive.zip> <files...>  or  zip -d <archive.zip> [output_dir]");
        return 1;
    }

    // Check for extract mode
    if args[0] == "-d" || args[0] == "--extract" {
        return extract(args);
    }

    // Create mode: zip <archive> <files...>
    if args.len() < 2 {
        eprintln!("zip: need at least an archive name and one file");
        return 1;
    }

    let archive_path = &args[0];
    let files = &args[1..];

    let file = match File::create(archive_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("zip: cannot create '{}': {}", archive_path, e);
            return 1;
        }
    };

    let mut writer = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for path in files {
        let p = Path::new(path);
        if p.is_dir() {
            if let Err(e) = add_directory(&mut writer, p, p, options) {
                eprintln!("zip: error adding '{}': {}", path, e);
                return 1;
            }
        } else if let Err(e) = add_file(&mut writer, p, path, options) {
            eprintln!("zip: error adding '{}': {}", path, e);
            return 1;
        }
    }

    if let Err(e) = writer.finish() {
        eprintln!("zip: error finalizing archive: {}", e);
        return 1;
    }

    0
}

fn extract(args: &[String]) -> i32 {
    if args.len() < 2 {
        eprintln!("zip: -d requires an archive path");
        return 1;
    }

    let archive_path = &args[1];
    let output_dir = if args.len() > 2 { &args[2] } else { "." };

    let file = match File::open(archive_path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("zip: cannot open '{}': {}", archive_path, e);
            return 1;
        }
    };

    let mut archive = match ZipArchive::new(file) {
        Ok(a) => a,
        Err(e) => {
            eprintln!("zip: invalid archive '{}': {}", archive_path, e);
            return 1;
        }
    };

    let output_path = Path::new(output_dir);
    let mut count = 0;

    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("zip: error reading entry {}: {}", i, e);
                continue;
            }
        };

        let name = entry.name().to_string();
        let out_path = output_path.join(&name);

        if name.ends_with('/') {
            let _ = fs::create_dir_all(&out_path);
        } else {
            if let Some(parent) = out_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let mut outfile = match File::create(&out_path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("zip: cannot create '{}': {}", out_path.display(), e);
                    continue;
                }
            };
            if let Err(e) = io::copy(&mut entry, &mut outfile) {
                eprintln!("zip: error extracting '{}': {}", name, e);
                continue;
            }
            count += 1;
        }
    }

    eprintln!("zip: extracted {} files", count);
    0
}

fn add_directory(
    writer: &mut ZipWriter<File>,
    base: &Path,
    dir: &Path,
    options: SimpleFileOptions,
) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path
            .strip_prefix(base)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();

        if path.is_dir() {
            add_directory(writer, base, &path, options)?;
        } else {
            add_file(writer, &path, &rel, options)?;
        }
    }
    Ok(())
}

fn add_file(
    writer: &mut ZipWriter<File>,
    path: &Path,
    name: &str,
    options: SimpleFileOptions,
) -> io::Result<()> {
    let data = fs::read(path)?;
    writer.start_file(name, options).map_err(io::Error::other)?;
    writer.write_all(&data)?;
    Ok(())
}
