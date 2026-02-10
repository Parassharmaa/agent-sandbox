use std::fs;

pub fn run(args: &[String]) -> i32 {
    let mut suppress1 = false;
    let mut suppress2 = false;
    let mut suppress3 = false;
    let mut files = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-1" => suppress1 = true,
            "-2" => suppress2 = true,
            "-3" => suppress3 = true,
            "-12" | "-21" => {
                suppress1 = true;
                suppress2 = true;
            }
            "-13" | "-31" => {
                suppress1 = true;
                suppress3 = true;
            }
            "-23" | "-32" => {
                suppress2 = true;
                suppress3 = true;
            }
            _ => files.push(arg.as_str()),
        }
    }

    if files.len() != 2 {
        eprintln!("comm: expected 2 file arguments, got {}", files.len());
        return 1;
    }

    let lines1 = match fs::read_to_string(files[0]) {
        Ok(c) => c.lines().map(String::from).collect::<Vec<_>>(),
        Err(e) => {
            eprintln!("comm: {}: {}", files[0], e);
            return 1;
        }
    };
    let lines2 = match fs::read_to_string(files[1]) {
        Ok(c) => c.lines().map(String::from).collect::<Vec<_>>(),
        Err(e) => {
            eprintln!("comm: {}: {}", files[1], e);
            return 1;
        }
    };

    let mut i = 0;
    let mut j = 0;

    while i < lines1.len() || j < lines2.len() {
        if i >= lines1.len() {
            if !suppress2 {
                print_col(2, &lines2[j], suppress1);
            }
            j += 1;
        } else if j >= lines2.len() {
            if !suppress1 {
                println!("{}", lines1[i]);
            }
            i += 1;
        } else if lines1[i] < lines2[j] {
            if !suppress1 {
                println!("{}", lines1[i]);
            }
            i += 1;
        } else if lines1[i] > lines2[j] {
            if !suppress2 {
                print_col(2, &lines2[j], suppress1);
            }
            j += 1;
        } else {
            // Equal
            if !suppress3 {
                print_col(3, &lines1[i], suppress1 && suppress2);
                if !suppress1 && !suppress2 {
                    // col3 gets two tabs
                } else if suppress1 || suppress2 {
                    // col3 gets one tab if one suppressed
                }
            }
            i += 1;
            j += 1;
        }
    }

    0
}

fn print_col(col: usize, line: &str, both_suppressed: bool) {
    match col {
        2 => {
            if !both_suppressed {
                println!("\t{}", line);
            } else {
                println!("{}", line);
            }
        }
        3 => {
            if both_suppressed {
                println!("{}", line);
            } else {
                println!("\t\t{}", line);
            }
        }
        _ => println!("{}", line),
    }
}
