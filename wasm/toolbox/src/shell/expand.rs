use super::ast::*;
use super::env::ShellEnv;

/// The dispatch function type.
pub type DispatchFn = fn(&str, &[String]) -> i32;

/// Expand a Word into one or more strings, performing variable expansion,
/// command substitution, and glob expansion.
pub fn expand_word(word: &Word, env: &mut ShellEnv, dispatch: DispatchFn) -> Vec<String> {
    let result = expand_word_to_string(word, env, dispatch);

    // Glob expansion on the result
    if has_glob_parts(word) {
        let expanded = glob_expand(&result);
        if !expanded.is_empty() {
            return expanded;
        }
    }

    vec![result]
}

/// Expand a Word into a single string (no glob expansion).
pub fn expand_word_to_string(word: &Word, env: &mut ShellEnv, dispatch: DispatchFn) -> String {
    let mut result = String::new();
    for part in &word.parts {
        result.push_str(&expand_part(part, env, dispatch));
    }
    result
}

fn expand_part(part: &WordPart, env: &mut ShellEnv, dispatch: DispatchFn) -> String {
    match part {
        WordPart::Literal(s) => s.clone(),

        WordPart::SingleQuoted(s) => s.clone(),

        WordPart::DoubleQuoted(parts) => {
            let mut result = String::new();
            for p in parts {
                result.push_str(&expand_part(p, env, dispatch));
            }
            result
        }

        WordPart::Variable(name) => env.get(name).unwrap_or("").to_string(),

        WordPart::VarDefault(name, default) => {
            let val = env.get(name).unwrap_or("").to_string();
            if val.is_empty() {
                expand_word_to_string(default, env, dispatch)
            } else {
                val
            }
        }

        WordPart::VarAssignDefault(name, default) => {
            let val = env.get(name).unwrap_or("").to_string();
            if val.is_empty() {
                let expanded = expand_word_to_string(default, env, dispatch);
                env.set(name, &expanded);
                expanded
            } else {
                val
            }
        }

        WordPart::VarLength(name) => {
            let val = env.get(name).unwrap_or("").to_string();
            val.len().to_string()
        }

        WordPart::CommandSub(cmd) => {
            let (_, output) = exec_capture_for_expand(cmd, env, dispatch);
            // Trim trailing newlines (shell behavior)
            output.trim_end_matches('\n').to_string()
        }

        WordPart::ArithmeticSub(expr) => eval_arithmetic(expr, env),

        WordPart::Glob(pattern) => {
            // Return the pattern as-is; glob expansion happens at the word level
            pattern.clone()
        }

        WordPart::SpecialVar(var) => match var {
            SpecialVar::ExitStatus => env.last_status.to_string(),
            SpecialVar::NumArgs => env.num_positional().to_string(),
            SpecialVar::AllArgs | SpecialVar::AllArgsStar => {
                env.all_positional().join(" ")
            }
            SpecialVar::ProcessId => "1".to_string(), // Fake PID in WASM
            SpecialVar::Positional(n) => {
                if *n == 0 {
                    "sh".to_string()
                } else {
                    env.all_positional()
                        .get(*n as usize - 1)
                        .cloned()
                        .unwrap_or_default()
                }
            }
        },
    }
}

/// Execute a script and capture its output (for command substitution).
/// This calls into the exec module.
fn exec_capture_for_expand(script: &str, env: &mut ShellEnv, dispatch: DispatchFn) -> (i32, String) {
    super::exec::exec_capture(script, env, dispatch)
}

fn has_glob_parts(word: &Word) -> bool {
    word.parts.iter().any(|p| matches!(p, WordPart::Glob(_)))
}

/// Expand glob patterns against the filesystem.
fn glob_expand(pattern: &str) -> Vec<String> {
    use std::fs;

    if !pattern.contains('*') && !pattern.contains('?') && !pattern.contains('[') {
        return Vec::new();
    }

    let (dir, file_pattern) = if let Some(slash_pos) = pattern.rfind('/') {
        (&pattern[..slash_pos], &pattern[slash_pos + 1..])
    } else {
        (".", pattern)
    };

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return Vec::new(),
    };

    let mut matches: Vec<String> = entries
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().to_string())
        .filter(|name| {
            if name.starts_with('.') && !file_pattern.starts_with('.') {
                return false;
            }
            glob_match(file_pattern, name)
        })
        .map(|name| {
            if dir == "." {
                name
            } else {
                format!("{}/{}", dir, name)
            }
        })
        .collect();

    matches.sort();
    matches
}

/// Match a string against a glob pattern.
pub fn glob_match(pattern: &str, s: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let s: Vec<char> = s.chars().collect();
    glob_match_impl(&p, &s, 0, 0)
}

fn glob_match_impl(pattern: &[char], string: &[char], mut pi: usize, mut si: usize) -> bool {
    while pi < pattern.len() {
        if si >= string.len() {
            while pi < pattern.len() && pattern[pi] == '*' {
                pi += 1;
            }
            return pi >= pattern.len();
        }

        match pattern[pi] {
            '*' => {
                pi += 1;
                for i in si..=string.len() {
                    if glob_match_impl(pattern, string, pi, i) {
                        return true;
                    }
                }
                return false;
            }
            '?' => {
                pi += 1;
                si += 1;
            }
            '[' => {
                pi += 1;
                let negate = pi < pattern.len() && pattern[pi] == '!';
                if negate {
                    pi += 1;
                }
                let mut matched = false;
                while pi < pattern.len() && pattern[pi] != ']' {
                    if pi + 2 < pattern.len() && pattern[pi + 1] == '-' {
                        let lo = pattern[pi];
                        let hi = pattern[pi + 2];
                        if string[si] >= lo && string[si] <= hi {
                            matched = true;
                        }
                        pi += 3;
                    } else {
                        if pattern[pi] == string[si] {
                            matched = true;
                        }
                        pi += 1;
                    }
                }
                if pi < pattern.len() {
                    pi += 1;
                }
                if matched == negate {
                    return false;
                }
                si += 1;
            }
            c => {
                if c != string[si] {
                    return false;
                }
                pi += 1;
                si += 1;
            }
        }
    }

    si >= string.len()
}

/// Simple arithmetic evaluation for $(( expr )).
fn eval_arithmetic(expr: &str, env: &ShellEnv) -> String {
    eval_arith_expr(expr.trim(), env).to_string()
}

fn eval_arith_expr(expr: &str, env: &ShellEnv) -> i64 {
    let expr = expr.trim();
    if expr.is_empty() {
        return 0;
    }

    if expr.starts_with('(') {
        if let Some(closing) = find_matching_paren(expr) {
            if closing == expr.len() - 1 {
                return eval_arith_expr(&expr[1..closing], env);
            }
        }
    }

    if let Some(pos) = find_op(expr, "||") {
        let l = eval_arith_expr(&expr[..pos], env);
        let r = eval_arith_expr(&expr[pos + 2..], env);
        return if l != 0 || r != 0 { 1 } else { 0 };
    }
    if let Some(pos) = find_op(expr, "&&") {
        let l = eval_arith_expr(&expr[..pos], env);
        let r = eval_arith_expr(&expr[pos + 2..], env);
        return if l != 0 && r != 0 { 1 } else { 0 };
    }
    if let Some(pos) = find_op(expr, "==") {
        let l = eval_arith_expr(&expr[..pos], env);
        let r = eval_arith_expr(&expr[pos + 2..], env);
        return if l == r { 1 } else { 0 };
    }
    if let Some(pos) = find_op(expr, "!=") {
        let l = eval_arith_expr(&expr[..pos], env);
        let r = eval_arith_expr(&expr[pos + 2..], env);
        return if l != r { 1 } else { 0 };
    }
    if let Some(pos) = find_op(expr, "<=") {
        let l = eval_arith_expr(&expr[..pos], env);
        let r = eval_arith_expr(&expr[pos + 2..], env);
        return if l <= r { 1 } else { 0 };
    }
    if let Some(pos) = find_op(expr, ">=") {
        let l = eval_arith_expr(&expr[..pos], env);
        let r = eval_arith_expr(&expr[pos + 2..], env);
        return if l >= r { 1 } else { 0 };
    }
    if let Some(pos) = rfind_op_additive(expr) {
        let l = eval_arith_expr(&expr[..pos], env);
        let r = eval_arith_expr(&expr[pos + 1..], env);
        return if expr.as_bytes()[pos] == b'+' { l + r } else { l - r };
    }
    if let Some(pos) = rfind_op_multiplicative(expr) {
        let l = eval_arith_expr(&expr[..pos], env);
        let r = eval_arith_expr(&expr[pos + 1..], env);
        return match expr.as_bytes()[pos] {
            b'*' => l * r,
            b'/' => if r == 0 { 0 } else { l / r },
            b'%' => if r == 0 { 0 } else { l % r },
            _ => 0,
        };
    }

    if expr.starts_with('!') {
        let val = eval_arith_expr(&expr[1..], env);
        return if val == 0 { 1 } else { 0 };
    }
    if expr.starts_with('-') && expr.len() > 1 && expr.as_bytes()[1].is_ascii_digit() {
        if let Ok(n) = expr.parse::<i64>() {
            return n;
        }
    }

    if let Ok(n) = expr.parse::<i64>() {
        return n;
    }

    let var_name = expr.trim_start_matches('$');
    if let Some(val) = env.get(var_name) {
        if let Ok(n) = val.parse::<i64>() {
            return n;
        }
    }

    0
}

fn find_matching_paren(expr: &str) -> Option<usize> {
    let mut depth = 0;
    for (i, ch) in expr.chars().enumerate() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn find_op(expr: &str, op: &str) -> Option<usize> {
    let bytes = expr.as_bytes();
    let op_bytes = op.as_bytes();
    let mut depth = 0i32;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'(' { depth += 1; }
        if bytes[i] == b')' { depth -= 1; }
        if depth == 0 && i + op_bytes.len() <= bytes.len() && &bytes[i..i + op_bytes.len()] == op_bytes {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn rfind_op_additive(expr: &str) -> Option<usize> {
    let bytes = expr.as_bytes();
    let mut depth = 0i32;
    let mut result = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'(' { depth += 1; }
        if b == b')' { depth -= 1; }
        if depth == 0 && (b == b'+' || b == b'-') && i > 0 {
            let prev = bytes[i - 1];
            if prev == b'*' || prev == b'/' || prev == b'%' || prev == b'(' {
                continue;
            }
            result = Some(i);
        }
    }
    result
}

fn rfind_op_multiplicative(expr: &str) -> Option<usize> {
    let bytes = expr.as_bytes();
    let mut depth = 0i32;
    let mut result = None;
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'(' { depth += 1; }
        if b == b')' { depth -= 1; }
        if depth == 0 && (b == b'*' || b == b'/' || b == b'%') && i > 0 {
            result = Some(i);
        }
    }
    result
}
