use std::fs;
use std::io::{self, BufRead};

pub fn run(args: &[String]) -> i32 {
    let mut table_mode = false;
    let mut separator = " \t".to_string();
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-t" => table_mode = true,
            "-s" => {
                i += 1;
                if i < args.len() {
                    separator = args[i].clone();
                }
            }
            _ => files.push(args[i].clone()),
        }
        i += 1;
    }

    if files.is_empty() {
        files.push("-".to_string());
    }

    let mut all_lines = Vec::new();
    for file in &files {
        let lines: Vec<String> = if file == "-" {
            io::stdin().lock().lines().map_while(|l| l.ok()).collect()
        } else {
            match fs::read_to_string(file) {
                Ok(c) => c.lines().map(String::from).collect(),
                Err(e) => {
                    eprintln!("column: {}: {}", file, e);
                    return 1;
                }
            }
        };
        all_lines.extend(lines);
    }

    if table_mode {
        format_table(&all_lines, &separator);
    } else {
        // Simple column output
        for line in &all_lines {
            println!("{}", line);
        }
    }

    0
}

fn format_table(lines: &[String], separator: &str) {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut max_cols = 0;

    for line in lines {
        let cols: Vec<String> = if separator.len() == 1 {
            line.split(separator.chars().next().unwrap())
                .map(|s| s.trim().to_string())
                .collect()
        } else {
            line.split(|c: char| separator.contains(c))
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect()
        };
        if cols.len() > max_cols {
            max_cols = cols.len();
        }
        rows.push(cols);
    }

    // Calculate column widths
    let mut widths = vec![0usize; max_cols];
    for row in &rows {
        for (i, col) in row.iter().enumerate() {
            if col.len() > widths[i] {
                widths[i] = col.len();
            }
        }
    }

    // Print formatted
    for row in &rows {
        let mut parts = Vec::new();
        for (i, col) in row.iter().enumerate() {
            if i < row.len() - 1 {
                parts.push(format!("{:<width$}", col, width = widths[i]));
            } else {
                parts.push(col.to_string());
            }
        }
        println!("{}", parts.join("  "));
    }
}
