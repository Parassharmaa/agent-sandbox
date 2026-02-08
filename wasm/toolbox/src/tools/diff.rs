use std::fs;

use similar::{ChangeTag, TextDiff};

pub fn run(args: &[String]) -> i32 {
    let mut unified = true;
    let mut context_lines: usize = 3;
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-u" => unified = true,
            "-U" => {
                i += 1;
                if i < args.len() {
                    context_lines = args[i].parse().unwrap_or(3);
                }
            }
            _ if !args[i].starts_with('-') => files.push(args[i].clone()),
            _ => {}
        }
        i += 1;
    }

    if files.len() < 2 {
        eprintln!("diff: missing operand");
        return 2;
    }

    let old_content = match fs::read_to_string(&files[0]) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("diff: {}: {}", files[0], e);
            return 2;
        }
    };

    let new_content = match fs::read_to_string(&files[1]) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("diff: {}: {}", files[1], e);
            return 2;
        }
    };

    if old_content == new_content {
        return 0;
    }

    let diff = TextDiff::from_lines(&old_content, &new_content);

    if unified {
        println!("--- {}", files[0]);
        println!("+++ {}", files[1]);

        for hunk in diff
            .unified_diff()
            .context_radius(context_lines)
            .iter_hunks()
        {
            println!("{}", hunk);
        }
    } else {
        for change in diff.iter_all_changes() {
            match change.tag() {
                ChangeTag::Delete => print!("< {}", change),
                ChangeTag::Insert => print!("> {}", change),
                ChangeTag::Equal => {}
            }
        }
    }

    1
}
