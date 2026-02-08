use std::io::{self, Read, Write};

pub fn run(args: &[String]) -> i32 {
    let mut delete = false;
    let mut squeeze = false;
    let mut positional: Vec<&str> = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-d" => delete = true,
            "-s" => squeeze = true,
            _ => positional.push(arg),
        }
    }

    if positional.is_empty() {
        eprintln!("tr: missing operand");
        return 1;
    }

    let set1 = expand_set(positional[0]);

    let mut input = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("tr: {}", e);
        return 1;
    }

    let stdout = io::stdout();
    let mut out = stdout.lock();

    if delete {
        let result: String = input.chars().filter(|c| !set1.contains(c)).collect();
        write!(out, "{}", result).ok();
    } else if positional.len() >= 2 {
        let set2 = expand_set(positional[1]);
        let mut result = String::with_capacity(input.len());
        let mut last_char: Option<char> = None;

        for c in input.chars() {
            let translated = if let Some(pos) = set1.iter().position(|&s| s == c) {
                *set2.get(pos).or(set2.last()).unwrap_or(&c)
            } else {
                c
            };

            if squeeze
                && let Some(last) = last_char
                && last == translated
                && set2.contains(&translated)
            {
                continue;
            }

            result.push(translated);
            last_char = Some(translated);
        }
        write!(out, "{}", result).ok();
    } else {
        eprintln!("tr: missing operand after '{}'", positional[0]);
        return 1;
    }

    0
}

fn expand_set(spec: &str) -> Vec<char> {
    let mut result = Vec::new();
    let chars: Vec<char> = spec.chars().collect();
    let mut i = 0;

    while i < chars.len() {
        if i + 2 < chars.len() && chars[i + 1] == '-' {
            let start = chars[i];
            let end = chars[i + 2];
            for c in start..=end {
                result.push(c);
            }
            i += 3;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    result
}
