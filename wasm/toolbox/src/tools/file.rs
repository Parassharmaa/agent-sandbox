use std::fs;

pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("file: missing file operand");
        return 1;
    }

    let mut exit_code = 0;
    for file in args {
        if file.starts_with('-') {
            continue;
        }

        let meta = match fs::metadata(file) {
            Ok(m) => m,
            Err(e) => {
                eprintln!("file: {}: {}", file, e);
                exit_code = 1;
                continue;
            }
        };

        if meta.is_dir() {
            println!("{}: directory", file);
            continue;
        }

        if meta.is_symlink() {
            match fs::read_link(file) {
                Ok(target) => println!("{}: symbolic link to {}", file, target.display()),
                Err(_) => println!("{}: symbolic link", file),
            }
            continue;
        }

        if meta.len() == 0 {
            println!("{}: empty", file);
            continue;
        }

        // Read first bytes for magic detection
        let data = match fs::read(file) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("file: {}: {}", file, e);
                exit_code = 1;
                continue;
            }
        };

        let file_type = detect_type(&data, file);
        println!("{}: {}", file, file_type);
    }
    exit_code
}

fn detect_type(data: &[u8], filename: &str) -> &'static str {
    if data.len() < 4 {
        return if data.iter().all(|&b| b.is_ascii()) {
            "ASCII text"
        } else {
            "data"
        };
    }

    // Magic number detection
    match &data[..4] {
        [0x7f, b'E', b'L', b'F'] => return "ELF executable",
        [0x89, b'P', b'N', b'G'] => return "PNG image data",
        [0xff, 0xd8, 0xff, _] => return "JPEG image data",
        [b'G', b'I', b'F', b'8'] => return "GIF image data",
        [b'P', b'K', 0x03, 0x04] => return "Zip archive data",
        [0x1f, 0x8b, _, _] => return "gzip compressed data",
        [b'B', b'Z', b'h', _] => return "bzip2 compressed data",
        [0xfd, b'7', b'z', b'X'] => return "XZ compressed data",
        [b'%', b'P', b'D', b'F'] => return "PDF document",
        [0x00, b'a', b's', b'm'] => return "WebAssembly binary",
        _ => {}
    }

    // Check for tar
    if data.len() > 262 && &data[257..262] == b"ustar" {
        return "POSIX tar archive";
    }

    // Check for shebang
    if data.starts_with(b"#!") {
        return "script, ASCII text executable";
    }

    // Check for JSON
    if data.starts_with(b"{") || data.starts_with(b"[") {
        if serde_json::from_slice::<serde_json::Value>(data).is_ok() {
            return "JSON data";
        }
    }

    // Check for XML/HTML
    if data.starts_with(b"<?xml") || data.starts_with(b"<xml") {
        return "XML document";
    }
    if data.starts_with(b"<!DOCTYPE html") || data.starts_with(b"<html") {
        return "HTML document";
    }

    // Extension-based hints
    if filename.ends_with(".rs") {
        return "Rust source, ASCII text";
    }
    if filename.ends_with(".js") || filename.ends_with(".mjs") {
        return "JavaScript source, ASCII text";
    }
    if filename.ends_with(".ts") {
        return "TypeScript source, ASCII text";
    }
    if filename.ends_with(".py") {
        return "Python script, ASCII text";
    }

    // Check if text
    let sample = &data[..data.len().min(8192)];
    let text_chars = sample
        .iter()
        .filter(|&&b| b.is_ascii_graphic() || b.is_ascii_whitespace())
        .count();

    if text_chars * 100 / sample.len() > 95 {
        if data.iter().any(|&b| b == b'\n') {
            "ASCII text"
        } else {
            "ASCII text, with no line terminators"
        }
    } else {
        "data"
    }
}
