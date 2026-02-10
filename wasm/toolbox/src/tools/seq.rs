pub fn run(args: &[String]) -> i32 {
    let mut separator = "\n".to_string();
    let mut nums = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-s" => {
                i += 1;
                if i < args.len() {
                    separator = args[i].clone();
                }
            }
            "-w" => {
                // Zero-padded — we'll handle width after parsing
            }
            _ => nums.push(args[i].clone()),
        }
        i += 1;
    }

    let (first, increment, last) = match nums.len() {
        1 => (Some(1.0), Some(1.0), parse_num(&nums[0])),
        2 => (parse_num(&nums[0]), Some(1.0), parse_num(&nums[1])),
        3 => (parse_num(&nums[0]), parse_num(&nums[1]), parse_num(&nums[2])),
        _ => {
            eprintln!("seq: missing operand");
            return 1;
        }
    };

    if let (Some(first), Some(increment), Some(last)) = (first, increment, last) {
        if increment == 0.0 {
            eprintln!("seq: zero increment");
            return 1;
        }

        let mut values = Vec::new();
        let mut current = first;

        if increment > 0.0 {
            while current <= last + f64::EPSILON {
                values.push(current);
                current += increment;
            }
        } else {
            while current >= last - f64::EPSILON {
                values.push(current);
                current += increment;
            }
        }

        let output: Vec<String> = values
            .iter()
            .map(|v| {
                if *v == v.floor() {
                    format!("{}", *v as i64)
                } else {
                    format!("{}", v)
                }
            })
            .collect();

        println!("{}", output.join(&separator));
        0
    } else {
        eprintln!("seq: invalid argument");
        1
    }
}

fn parse_num(s: &str) -> Option<f64> {
    s.parse::<f64>().ok()
}
