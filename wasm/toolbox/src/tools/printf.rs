use std::io::{self, Write};

pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        return 0;
    }

    let format = &args[0];
    let params = &args[1..];
    let mut param_idx = 0;

    let stdout = io::stdout();
    let mut out = stdout.lock();

    let chars: Vec<char> = format.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if chars[i] == '\\' && i + 1 < chars.len() {
            match chars[i + 1] {
                'n' => {
                    writeln!(out).ok();
                }
                't' => {
                    write!(out, "\t").ok();
                }
                '\\' => {
                    write!(out, "\\").ok();
                }
                '"' => {
                    write!(out, "\"").ok();
                }
                '0' => {
                    write!(out, "\0").ok();
                }
                _ => {
                    write!(out, "\\{}", chars[i + 1]).ok();
                }
            }
            i += 2;
        } else if chars[i] == '%' && i + 1 < chars.len() {
            match chars[i + 1] {
                's' => {
                    if param_idx < params.len() {
                        write!(out, "{}", params[param_idx]).ok();
                        param_idx += 1;
                    }
                    i += 2;
                }
                'd' => {
                    if param_idx < params.len() {
                        let n: i64 = params[param_idx].parse().unwrap_or(0);
                        write!(out, "{}", n).ok();
                        param_idx += 1;
                    }
                    i += 2;
                }
                '%' => {
                    write!(out, "%").ok();
                    i += 2;
                }
                _ => {
                    write!(out, "%{}", chars[i + 1]).ok();
                    i += 2;
                }
            }
        } else {
            write!(out, "{}", chars[i]).ok();
            i += 1;
        }
    }

    0
}
