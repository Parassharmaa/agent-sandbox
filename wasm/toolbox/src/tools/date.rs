use std::env;

pub fn run(args: &[String]) -> i32 {
    // In WASM sandbox, we don't have real clock access.
    // Check for SANDBOX_TIME env var or return a fixed epoch-based string.
    let timestamp = env::var("SANDBOX_TIME")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .unwrap_or(0);

    if args.is_empty() {
        // Default format
        println!("{}", format_timestamp(timestamp, None));
        return 0;
    }

    // Check for format string starting with +
    for arg in args {
        if let Some(fmt) = arg.strip_prefix('+') {
            println!("{}", format_timestamp(timestamp, Some(fmt)));
            return 0;
        }
    }

    println!("{}", format_timestamp(timestamp, None));
    0
}

fn format_timestamp(epoch_secs: i64, format: Option<&str>) -> String {
    // Simple date formatting without external crate
    let (year, month, day, hour, min, sec) = epoch_to_parts(epoch_secs);

    match format {
        Some(fmt) => {
            let mut result = String::new();
            let chars: Vec<char> = fmt.chars().collect();
            let mut i = 0;
            while i < chars.len() {
                if chars[i] == '%' && i + 1 < chars.len() {
                    i += 1;
                    match chars[i] {
                        'Y' => result.push_str(&format!("{:04}", year)),
                        'm' => result.push_str(&format!("{:02}", month)),
                        'd' => result.push_str(&format!("{:02}", day)),
                        'H' => result.push_str(&format!("{:02}", hour)),
                        'M' => result.push_str(&format!("{:02}", min)),
                        'S' => result.push_str(&format!("{:02}", sec)),
                        's' => result.push_str(&format!("{}", epoch_secs)),
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        '%' => result.push('%'),
                        'F' => result.push_str(&format!("{:04}-{:02}-{:02}", year, month, day)),
                        'T' => result.push_str(&format!("{:02}:{:02}:{:02}", hour, min, sec)),
                        c => {
                            result.push('%');
                            result.push(c);
                        }
                    }
                } else {
                    result.push(chars[i]);
                }
                i += 1;
            }
            result
        }
        None => format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
            year, month, day, hour, min, sec
        ),
    }
}

fn epoch_to_parts(epoch_secs: i64) -> (i64, u32, u32, u32, u32, u32) {
    let secs = epoch_secs;
    let sec = ((secs % 60) + 60) as u32 % 60;
    let mins = secs / 60;
    let min = ((mins % 60) + 60) as u32 % 60;
    let hours = mins / 60;
    let hour = ((hours % 24) + 24) as u32 % 24;
    let days = hours / 24;

    // Days since epoch to Y/M/D
    let mut y = 1970i64;
    let mut remaining_days = days;

    loop {
        let days_in_year = if is_leap(y) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        y += 1;
    }

    let month_days = if is_leap(y) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut m = 0u32;
    for &md in &month_days {
        if remaining_days < md {
            break;
        }
        remaining_days -= md;
        m += 1;
    }

    (y, m + 1, remaining_days as u32 + 1, hour, min, sec)
}

fn is_leap(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}
