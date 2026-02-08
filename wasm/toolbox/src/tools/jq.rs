use std::fs;
use std::io::{self, Read};

use serde_json::Value;

pub fn run(args: &[String]) -> i32 {
    let mut raw_output = false;
    let mut compact = false;
    let mut filter = String::new();
    let mut files: Vec<String> = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-r" | "--raw-output" => raw_output = true,
            "-c" | "--compact-output" => compact = true,
            "-e" | "--exit-status" => {} // ignored for compatibility
            "--" => {
                i += 1;
                while i < args.len() {
                    files.push(args[i].clone());
                    i += 1;
                }
                break;
            }
            arg if arg.starts_with('-') && arg.len() > 1 => {
                eprintln!("jq: unknown option '{}'", arg);
                return 2;
            }
            _ => {
                if filter.is_empty() {
                    filter = args[i].clone();
                } else {
                    files.push(args[i].clone());
                }
            }
        }
        i += 1;
    }

    if filter.is_empty() {
        filter = ".".to_string();
    }

    let inputs: Vec<String> = if files.is_empty() {
        let mut buf = String::new();
        if io::stdin().read_to_string(&mut buf).is_ok() && !buf.is_empty() {
            vec![buf]
        } else {
            eprintln!("jq: no input");
            return 1;
        }
    } else {
        let mut result = Vec::new();
        for file in &files {
            match fs::read_to_string(file) {
                Ok(content) => result.push(content),
                Err(e) => {
                    eprintln!("jq: {}: {}", file, e);
                    return 1;
                }
            }
        }
        result
    };

    for input in &inputs {
        let value: Value = match serde_json::from_str(input.trim()) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("jq: invalid JSON: {}", e);
                return 1;
            }
        };

        match apply_filter(&value, &filter) {
            Ok(results) => {
                for result in results {
                    let output = if raw_output {
                        match &result {
                            Value::String(s) => s.clone(),
                            other => format_value(other, compact),
                        }
                    } else {
                        format_value(&result, compact)
                    };
                    println!("{}", output);
                }
            }
            Err(e) => {
                eprintln!("jq: {}", e);
                return 1;
            }
        }
    }

    0
}

fn format_value(value: &Value, compact: bool) -> String {
    if compact {
        serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
    } else {
        serde_json::to_string_pretty(value).unwrap_or_else(|_| "null".to_string())
    }
}

fn apply_filter(value: &Value, filter: &str) -> Result<Vec<Value>, String> {
    let filter = filter.trim();

    if filter == "." {
        return Ok(vec![value.clone()]);
    }

    if filter == ".[]" {
        return match value {
            Value::Array(arr) => Ok(arr.clone()),
            Value::Object(obj) => Ok(obj.values().cloned().collect()),
            _ => Err("cannot iterate over non-iterable".to_string()),
        };
    }

    // Handle pipe: .foo | .bar
    if let Some(pipe_pos) = find_pipe(filter) {
        let left = filter[..pipe_pos].trim();
        let right = filter[pipe_pos + 1..].trim();
        let intermediate = apply_filter(value, left)?;
        let mut results = Vec::new();
        for val in &intermediate {
            results.extend(apply_filter(val, right)?);
        }
        return Ok(results);
    }

    // Field access: .foo or .foo.bar
    if let Some(rest) = filter.strip_prefix('.') {
        if rest.is_empty() {
            return Ok(vec![value.clone()]);
        }

        // Handle .foo[0] or .foo[]
        if let Some(bracket_pos) = rest.find('[') {
            let field = &rest[..bracket_pos];
            let bracket_expr = &rest[bracket_pos..];

            let intermediate = if field.is_empty() {
                value.clone()
            } else {
                get_field(value, field)
            };

            if bracket_expr == "[]" {
                return apply_filter(&intermediate, ".[]");
            }

            if bracket_expr.starts_with('[') && bracket_expr.ends_with(']') {
                let idx_str = &bracket_expr[1..bracket_expr.len() - 1];
                if let Ok(idx) = idx_str.parse::<usize>() {
                    return match &intermediate {
                        Value::Array(arr) => Ok(vec![arr.get(idx).cloned().unwrap_or(Value::Null)]),
                        _ => Err(format!("cannot index non-array with {}", idx)),
                    };
                }
            }

            return Err(format!("unsupported bracket expression: {}", bracket_expr));
        }

        // Handle nested field access: .foo.bar
        if let Some(dot_pos) = rest.find('.') {
            let field = &rest[..dot_pos];
            let remaining = &rest[dot_pos..];
            let intermediate = get_field(value, field);
            return apply_filter(&intermediate, remaining);
        }

        return Ok(vec![get_field(value, rest)]);
    }

    // keys
    if filter == "keys" {
        return match value {
            Value::Object(obj) => {
                let keys: Vec<Value> = obj.keys().map(|k| Value::String(k.clone())).collect();
                Ok(vec![Value::Array(keys)])
            }
            Value::Array(arr) => {
                let keys: Vec<Value> = (0..arr.len())
                    .map(|i| Value::Number(serde_json::Number::from(i)))
                    .collect();
                Ok(vec![Value::Array(keys)])
            }
            _ => Err("keys requires object or array".to_string()),
        };
    }

    // length
    if filter == "length" {
        return match value {
            Value::Array(arr) => Ok(vec![Value::Number(arr.len().into())]),
            Value::Object(obj) => Ok(vec![Value::Number(obj.len().into())]),
            Value::String(s) => Ok(vec![Value::Number(s.len().into())]),
            Value::Null => Ok(vec![Value::Number(0.into())]),
            _ => Err("length not supported for this type".to_string()),
        };
    }

    // type
    if filter == "type" {
        let t = match value {
            Value::Null => "null",
            Value::Bool(_) => "boolean",
            Value::Number(_) => "number",
            Value::String(_) => "string",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
        };
        return Ok(vec![Value::String(t.to_string())]);
    }

    // select(expr)
    if filter.starts_with("select(") && filter.ends_with(')') {
        let inner = &filter[7..filter.len() - 1];
        let results = apply_filter(value, inner)?;
        if let Some(r) = results.first() {
            match r {
                Value::Bool(true) => return Ok(vec![value.clone()]),
                Value::Bool(false) => return Ok(vec![]),
                _ => return Ok(vec![value.clone()]),
            }
        }
        return Ok(vec![]);
    }

    Err(format!("unsupported filter: {}", filter))
}

fn get_field(value: &Value, field: &str) -> Value {
    match value {
        Value::Object(obj) => obj.get(field).cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

fn find_pipe(filter: &str) -> Option<usize> {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape = false;
    for (i, ch) in filter.chars().enumerate() {
        if escape {
            escape = false;
            continue;
        }
        if ch == '\\' {
            escape = true;
            continue;
        }
        if ch == '"' {
            in_string = !in_string;
            continue;
        }
        if in_string {
            continue;
        }
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth -= 1,
            '|' if depth == 0 => return Some(i),
            _ => {}
        }
    }
    None
}
