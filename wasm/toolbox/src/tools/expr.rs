pub fn run(args: &[String]) -> i32 {
    if args.is_empty() {
        eprintln!("expr: missing operand");
        return 2;
    }

    let tokens: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    match evaluate(&tokens, &mut 0) {
        Ok(val) => {
            println!("{}", val);
            if val == "0" || val.is_empty() {
                1
            } else {
                0
            }
        }
        Err(e) => {
            eprintln!("expr: {}", e);
            2
        }
    }
}

fn evaluate<'a>(tokens: &[&'a str], pos: &mut usize) -> Result<String, String> {
    eval_or(tokens, pos)
}

fn eval_or(tokens: &[&str], pos: &mut usize) -> Result<String, String> {
    let mut left = eval_and(tokens, pos)?;
    while *pos < tokens.len() && tokens[*pos] == "|" {
        *pos += 1;
        let right = eval_and(tokens, pos)?;
        left = if !left.is_empty() && left != "0" {
            left
        } else {
            right
        };
    }
    Ok(left)
}

fn eval_and(tokens: &[&str], pos: &mut usize) -> Result<String, String> {
    let mut left = eval_compare(tokens, pos)?;
    while *pos < tokens.len() && tokens[*pos] == "&" {
        *pos += 1;
        let right = eval_compare(tokens, pos)?;
        left = if (!left.is_empty() && left != "0") && (!right.is_empty() && right != "0") {
            left
        } else {
            "0".to_string()
        };
    }
    Ok(left)
}

fn eval_compare(tokens: &[&str], pos: &mut usize) -> Result<String, String> {
    let left = eval_add(tokens, pos)?;
    if *pos < tokens.len() {
        let op = tokens[*pos];
        if matches!(op, "=" | "!=" | "<" | "<=" | ">" | ">=") {
            *pos += 1;
            let right = eval_add(tokens, pos)?;

            let result = match (left.parse::<i64>(), right.parse::<i64>()) {
                (Ok(l), Ok(r)) => match op {
                    "=" => l == r,
                    "!=" => l != r,
                    "<" => l < r,
                    "<=" => l <= r,
                    ">" => l > r,
                    ">=" => l >= r,
                    _ => false,
                },
                _ => match op {
                    "=" => left == right,
                    "!=" => left != right,
                    "<" => left < right,
                    "<=" => left <= right,
                    ">" => left > right,
                    ">=" => left >= right,
                    _ => false,
                },
            };
            return Ok(if result { "1" } else { "0" }.to_string());
        }
    }
    Ok(left)
}

fn eval_add(tokens: &[&str], pos: &mut usize) -> Result<String, String> {
    let mut left = eval_mul(tokens, pos)?;
    while *pos < tokens.len() && matches!(tokens[*pos], "+" | "-") {
        let op = tokens[*pos];
        *pos += 1;
        let right = eval_mul(tokens, pos)?;
        let l: i64 = left
            .parse()
            .map_err(|_| format!("non-integer argument '{}'", left))?;
        let r: i64 = right
            .parse()
            .map_err(|_| format!("non-integer argument '{}'", right))?;
        left = match op {
            "+" => (l + r).to_string(),
            "-" => (l - r).to_string(),
            _ => unreachable!(),
        };
    }
    Ok(left)
}

fn eval_mul(tokens: &[&str], pos: &mut usize) -> Result<String, String> {
    let mut left = eval_match(tokens, pos)?;
    while *pos < tokens.len() && matches!(tokens[*pos], "*" | "/" | "%") {
        let op = tokens[*pos];
        *pos += 1;
        let right = eval_match(tokens, pos)?;
        let l: i64 = left
            .parse()
            .map_err(|_| format!("non-integer argument '{}'", left))?;
        let r: i64 = right
            .parse()
            .map_err(|_| format!("non-integer argument '{}'", right))?;
        left = match op {
            "*" => (l * r).to_string(),
            "/" => {
                if r == 0 {
                    return Err("division by zero".to_string());
                }
                (l / r).to_string()
            }
            "%" => {
                if r == 0 {
                    return Err("division by zero".to_string());
                }
                (l % r).to_string()
            }
            _ => unreachable!(),
        };
    }
    Ok(left)
}

fn eval_match(tokens: &[&str], pos: &mut usize) -> Result<String, String> {
    let left = eval_primary(tokens, pos)?;
    if *pos < tokens.len() && tokens[*pos] == ":" {
        *pos += 1;
        let pattern = eval_primary(tokens, pos)?;
        let re_pattern = format!("^{}", pattern);
        match regex::Regex::new(&re_pattern) {
            Ok(re) => {
                if let Some(m) = re.find(&left) {
                    if re.captures_len() > 1 {
                        if let Some(caps) = re.captures(&left) {
                            if let Some(group) = caps.get(1) {
                                return Ok(group.as_str().to_string());
                            }
                        }
                    }
                    Ok(m.len().to_string())
                } else {
                    Ok("0".to_string())
                }
            }
            Err(e) => Err(format!("invalid regex: {}", e)),
        }
    } else {
        Ok(left)
    }
}

fn eval_primary(tokens: &[&str], pos: &mut usize) -> Result<String, String> {
    if *pos >= tokens.len() {
        return Err("syntax error".to_string());
    }

    if tokens[*pos] == "(" {
        *pos += 1;
        let val = evaluate(tokens, pos)?;
        if *pos < tokens.len() && tokens[*pos] == ")" {
            *pos += 1;
        } else {
            return Err("missing ')'".to_string());
        }
        return Ok(val);
    }

    let val = tokens[*pos].to_string();
    *pos += 1;
    Ok(val)
}
