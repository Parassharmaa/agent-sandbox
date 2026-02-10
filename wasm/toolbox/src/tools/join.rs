use std::fs;

pub fn run(args: &[String]) -> i32 {
    let mut field1 = 0usize; // 0-indexed
    let mut field2 = 0usize;
    let mut separator = " ".to_string();
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-1" => {
                i += 1;
                if i < args.len() {
                    field1 = args[i].parse::<usize>().unwrap_or(1).saturating_sub(1);
                }
            }
            "-2" => {
                i += 1;
                if i < args.len() {
                    field2 = args[i].parse::<usize>().unwrap_or(1).saturating_sub(1);
                }
            }
            "-t" => {
                i += 1;
                if i < args.len() {
                    separator = args[i].clone();
                }
            }
            _ => files.push(args[i].clone()),
        }
        i += 1;
    }

    if files.len() != 2 {
        eprintln!("join: expected 2 file arguments");
        return 1;
    }

    let lines1 = match fs::read_to_string(&files[0]) {
        Ok(c) => c.lines().map(String::from).collect::<Vec<_>>(),
        Err(e) => {
            eprintln!("join: {}: {}", files[0], e);
            return 1;
        }
    };
    let lines2 = match fs::read_to_string(&files[1]) {
        Ok(c) => c.lines().map(String::from).collect::<Vec<_>>(),
        Err(e) => {
            eprintln!("join: {}: {}", files[1], e);
            return 1;
        }
    };

    let sep_char = separator.chars().next().unwrap_or(' ');

    let mut j = 0;
    for line1 in &lines1 {
        let fields1: Vec<&str> = line1.split(sep_char).collect();
        let key1 = fields1.get(field1).unwrap_or(&"");

        while j < lines2.len() {
            let fields2: Vec<&str> = lines2[j].split(sep_char).collect();
            let key2 = fields2.get(field2).unwrap_or(&"");

            if key2 < key1 {
                j += 1;
                continue;
            }
            if key2 > key1 {
                break;
            }

            // Keys match
            let mut output = key1.to_string();
            for (k, f) in fields1.iter().enumerate() {
                if k != field1 {
                    output.push(sep_char);
                    output.push_str(f);
                }
            }
            for (k, f) in fields2.iter().enumerate() {
                if k != field2 {
                    output.push(sep_char);
                    output.push_str(f);
                }
            }
            println!("{}", output);
            j += 1;
        }
    }

    0
}
