use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead};

use regex::Regex;

pub fn run(args: &[String]) -> i32 {
    let mut field_sep = " ".to_string();
    let mut program = String::new();
    let mut files = Vec::new();

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "-F" => {
                i += 1;
                if i < args.len() {
                    field_sep = args[i].clone();
                }
            }
            arg if arg.starts_with("-F") => {
                field_sep = arg[2..].to_string();
            }
            _ => {
                if program.is_empty() {
                    program = args[i].clone();
                } else {
                    files.push(args[i].clone());
                }
            }
        }
        i += 1;
    }

    if program.is_empty() {
        eprintln!("awk: no program text");
        return 1;
    }

    let rules = match parse_program(&program) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("awk: {}", e);
            return 2;
        }
    };

    let mut vars: HashMap<String, String> = HashMap::new();
    vars.insert("FS".to_string(), field_sep.clone());
    vars.insert("OFS".to_string(), " ".to_string());
    vars.insert("ORS".to_string(), "\n".to_string());
    vars.insert("NR".to_string(), "0".to_string());
    vars.insert("NF".to_string(), "0".to_string());

    // Execute BEGIN rules
    for rule in &rules {
        if rule.pattern == AwkPattern::Begin {
            execute_actions(&rule.actions, &[], &mut vars);
        }
    }

    let mut nr = 0u64;

    if files.is_empty() {
        let stdin = io::stdin();
        for line in stdin.lock().lines().map_while(|l| l.ok()) {
            nr += 1;
            process_line(&line, &field_sep, &rules, &mut vars, nr);
        }
    } else {
        for file in &files {
            match fs::read_to_string(file) {
                Ok(content) => {
                    for line in content.lines() {
                        nr += 1;
                        process_line(line, &field_sep, &rules, &mut vars, nr);
                    }
                }
                Err(e) => {
                    eprintln!("awk: {}: {}", file, e);
                    return 1;
                }
            }
        }
    }

    // Execute END rules
    for rule in &rules {
        if rule.pattern == AwkPattern::End {
            execute_actions(&rule.actions, &[], &mut vars);
        }
    }

    0
}

#[derive(Debug, PartialEq)]
enum AwkPattern {
    Begin,
    End,
    Always,
    Regex(String),
    Expression(String),
}

#[derive(Debug)]
struct AwkRule {
    pattern: AwkPattern,
    actions: Vec<AwkAction>,
}

#[derive(Debug)]
enum AwkAction {
    Print(Vec<AwkExpr>),
    Assign(String, AwkExpr),
    If(AwkExpr, Vec<AwkAction>, Vec<AwkAction>),
    For(Box<AwkAction>, AwkExpr, Box<AwkAction>, Vec<AwkAction>),
    Noop,
}

#[derive(Debug, Clone)]
enum AwkExpr {
    Field(Box<AwkExpr>),
    Literal(String),
    Var(String),
    BinOp(Box<AwkExpr>, String, Box<AwkExpr>),
    Concat(Box<AwkExpr>, Box<AwkExpr>),
    FuncCall(String, Vec<AwkExpr>),
    Match(Box<AwkExpr>, String),
    UnaryOp(String, Box<AwkExpr>),
    PostIncr(String),
    PostDecr(String),
}

fn parse_program(prog: &str) -> Result<Vec<AwkRule>, String> {
    let mut rules = Vec::new();
    let prog = prog.trim();

    if prog.is_empty() {
        return Err("empty program".to_string());
    }

    let mut pos = 0;
    let chars: Vec<char> = prog.chars().collect();

    while pos < chars.len() {
        skip_whitespace(&chars, &mut pos);
        if pos >= chars.len() {
            break;
        }

        // Parse pattern
        let pattern = if starts_with_at(&chars, pos, "BEGIN") && !is_ident_char_at(&chars, pos + 5)
        {
            pos += 5;
            AwkPattern::Begin
        } else if starts_with_at(&chars, pos, "END") && !is_ident_char_at(&chars, pos + 3) {
            pos += 3;
            AwkPattern::End
        } else if chars[pos] == '/' {
            // Regex pattern
            pos += 1;
            let start = pos;
            while pos < chars.len() && chars[pos] != '/' {
                if chars[pos] == '\\' {
                    pos += 1;
                }
                pos += 1;
            }
            let regex = chars[start..pos].iter().collect::<String>();
            if pos < chars.len() {
                pos += 1; // skip closing /
            }
            AwkPattern::Regex(regex)
        } else if chars[pos] == '{' {
            AwkPattern::Always
        } else {
            // Expression pattern
            let start = pos;
            let mut depth = 0;
            while pos < chars.len() && !(chars[pos] == '{' && depth == 0) {
                if chars[pos] == '(' {
                    depth += 1;
                }
                if chars[pos] == ')' {
                    depth -= 1;
                }
                pos += 1;
            }
            let expr = chars[start..pos].iter().collect::<String>().trim().to_string();
            AwkPattern::Expression(expr)
        };

        skip_whitespace(&chars, &mut pos);

        // Parse action block
        let actions = if pos < chars.len() && chars[pos] == '{' {
            pos += 1; // skip {
            let actions = parse_action_block(&chars, &mut pos)?;
            actions
        } else {
            // Default action is print $0
            vec![AwkAction::Print(vec![AwkExpr::Field(Box::new(
                AwkExpr::Literal("0".to_string()),
            ))])]
        };

        skip_whitespace(&chars, &mut pos);
        // Skip optional semicolons
        while pos < chars.len() && (chars[pos] == ';' || chars[pos] == '\n') {
            pos += 1;
        }

        rules.push(AwkRule { pattern, actions });
    }

    Ok(rules)
}

fn parse_action_block(chars: &[char], pos: &mut usize) -> Result<Vec<AwkAction>, String> {
    let mut actions = Vec::new();

    loop {
        skip_whitespace(chars, pos);
        if *pos >= chars.len() || chars[*pos] == '}' {
            if *pos < chars.len() {
                *pos += 1; // skip }
            }
            break;
        }

        let action = parse_statement(chars, pos)?;
        actions.push(action);

        skip_whitespace(chars, pos);
        if *pos < chars.len() && (chars[*pos] == ';' || chars[*pos] == '\n') {
            *pos += 1;
        }
    }

    Ok(actions)
}

fn parse_statement(chars: &[char], pos: &mut usize) -> Result<AwkAction, String> {
    skip_whitespace(chars, pos);
    if *pos >= chars.len() {
        return Ok(AwkAction::Noop);
    }

    // print statement
    if starts_with_at(chars, *pos, "print") && !is_ident_char_at(chars, *pos + 5) {
        *pos += 5;
        skip_spaces(chars, pos);
        let mut exprs = Vec::new();

        while *pos < chars.len() && chars[*pos] != ';' && chars[*pos] != '}' && chars[*pos] != '\n'
        {
            let expr = parse_expr(chars, pos)?;
            exprs.push(expr);
            skip_spaces(chars, pos);
            if *pos < chars.len() && chars[*pos] == ',' {
                *pos += 1;
                skip_spaces(chars, pos);
            }
        }

        if exprs.is_empty() {
            exprs.push(AwkExpr::Field(Box::new(AwkExpr::Literal("0".to_string()))));
        }

        return Ok(AwkAction::Print(exprs));
    }

    // if statement
    if starts_with_at(chars, *pos, "if") && !is_ident_char_at(chars, *pos + 2) {
        *pos += 2;
        skip_whitespace(chars, pos);
        if *pos < chars.len() && chars[*pos] == '(' {
            *pos += 1;
        }
        let cond = parse_expr(chars, pos)?;
        if *pos < chars.len() && chars[*pos] == ')' {
            *pos += 1;
        }
        skip_whitespace(chars, pos);
        let then_block = if *pos < chars.len() && chars[*pos] == '{' {
            *pos += 1;
            parse_action_block(chars, pos)?
        } else {
            vec![parse_statement(chars, pos)?]
        };
        skip_whitespace(chars, pos);
        let else_block = if starts_with_at(chars, *pos, "else") && !is_ident_char_at(chars, *pos + 4) {
            *pos += 4;
            skip_whitespace(chars, pos);
            if *pos < chars.len() && chars[*pos] == '{' {
                *pos += 1;
                parse_action_block(chars, pos)?
            } else {
                vec![parse_statement(chars, pos)?]
            }
        } else {
            Vec::new()
        };
        return Ok(AwkAction::If(cond, then_block, else_block));
    }

    // for statement
    if starts_with_at(chars, *pos, "for") && !is_ident_char_at(chars, *pos + 3) {
        *pos += 3;
        skip_whitespace(chars, pos);
        if *pos < chars.len() && chars[*pos] == '(' {
            *pos += 1;
        }
        let init = parse_statement(chars, pos)?;
        if *pos < chars.len() && chars[*pos] == ';' {
            *pos += 1;
        }
        let cond = parse_expr(chars, pos)?;
        if *pos < chars.len() && chars[*pos] == ';' {
            *pos += 1;
        }
        let update = parse_statement(chars, pos)?;
        if *pos < chars.len() && chars[*pos] == ')' {
            *pos += 1;
        }
        skip_whitespace(chars, pos);
        let body = if *pos < chars.len() && chars[*pos] == '{' {
            *pos += 1;
            parse_action_block(chars, pos)?
        } else {
            vec![parse_statement(chars, pos)?]
        };
        return Ok(AwkAction::For(
            Box::new(init),
            cond,
            Box::new(update),
            body,
        ));
    }

    // Assignment or expression (variable = expr, or $field = expr)
    let expr = parse_expr(chars, pos)?;
    if let AwkExpr::Var(ref name) = expr {
        skip_spaces(chars, pos);
        if *pos < chars.len() && chars[*pos] == '=' && (*pos + 1 >= chars.len() || chars[*pos + 1] != '=') {
            *pos += 1;
            skip_spaces(chars, pos);
            let val = parse_expr(chars, pos)?;
            return Ok(AwkAction::Assign(name.clone(), val));
        }
    }

    // If it's a standalone expression (like i++), wrap as noop-ish
    Ok(AwkAction::Noop)
}

fn parse_expr(chars: &[char], pos: &mut usize) -> Result<AwkExpr, String> {
    parse_comparison(chars, pos)
}

fn parse_comparison(chars: &[char], pos: &mut usize) -> Result<AwkExpr, String> {
    let left = parse_concat(chars, pos)?;
    skip_spaces(chars, pos);

    if *pos < chars.len() {
        let op = if *pos + 1 < chars.len() && chars[*pos] == '!' && chars[*pos + 1] == '=' {
            Some("!=")
        } else if *pos + 1 < chars.len() && chars[*pos] == '=' && chars[*pos + 1] == '=' {
            Some("==")
        } else if *pos + 1 < chars.len() && chars[*pos] == '>' && chars[*pos + 1] == '=' {
            Some(">=")
        } else if *pos + 1 < chars.len() && chars[*pos] == '<' && chars[*pos + 1] == '=' {
            Some("<=")
        } else if chars[*pos] == '>' {
            Some(">")
        } else if chars[*pos] == '<' {
            Some("<")
        } else if chars[*pos] == '~' {
            Some("~")
        } else if *pos + 1 < chars.len() && chars[*pos] == '!' && chars[*pos + 1] == '~' {
            Some("!~")
        } else {
            None
        };

        if let Some(op) = op {
            *pos += op.len();
            skip_spaces(chars, pos);
            let right = parse_concat(chars, pos)?;
            if op == "~" || op == "!~" {
                if let AwkExpr::Literal(ref pat) = right {
                    return Ok(AwkExpr::Match(Box::new(left), pat.clone()));
                }
            }
            return Ok(AwkExpr::BinOp(
                Box::new(left),
                op.to_string(),
                Box::new(right),
            ));
        }
    }

    Ok(left)
}

fn parse_concat(chars: &[char], pos: &mut usize) -> Result<AwkExpr, String> {
    let mut left = parse_addition(chars, pos)?;

    loop {
        skip_spaces(chars, pos);
        if *pos >= chars.len() {
            break;
        }
        // String concatenation: two adjacent non-operator tokens
        let ch = chars[*pos];
        if ch == '"' || ch == '$' || ch.is_alphanumeric() || ch == '_' {
            // Check it's not an operator keyword
            if starts_with_at(chars, *pos, "==")
                || starts_with_at(chars, *pos, "!=")
                || ch == '}'
                || ch == ';'
                || ch == ','
                || ch == ')'
            {
                break;
            }
            let right = parse_addition(chars, pos)?;
            left = AwkExpr::Concat(Box::new(left), Box::new(right));
        } else {
            break;
        }
    }

    Ok(left)
}

fn parse_addition(chars: &[char], pos: &mut usize) -> Result<AwkExpr, String> {
    let mut left = parse_multiplication(chars, pos)?;
    loop {
        skip_spaces(chars, pos);
        if *pos >= chars.len() {
            break;
        }
        if chars[*pos] == '+' || chars[*pos] == '-' {
            // Check for ++ and --
            if *pos + 1 < chars.len() && chars[*pos + 1] == chars[*pos] {
                break;
            }
            let op = chars[*pos].to_string();
            *pos += 1;
            skip_spaces(chars, pos);
            let right = parse_multiplication(chars, pos)?;
            left = AwkExpr::BinOp(Box::new(left), op, Box::new(right));
        } else {
            break;
        }
    }
    Ok(left)
}

fn parse_multiplication(chars: &[char], pos: &mut usize) -> Result<AwkExpr, String> {
    let mut left = parse_unary(chars, pos)?;
    loop {
        skip_spaces(chars, pos);
        if *pos >= chars.len() {
            break;
        }
        if chars[*pos] == '*' || chars[*pos] == '/' || chars[*pos] == '%' {
            let op = chars[*pos].to_string();
            *pos += 1;
            skip_spaces(chars, pos);
            let right = parse_unary(chars, pos)?;
            left = AwkExpr::BinOp(Box::new(left), op, Box::new(right));
        } else {
            break;
        }
    }
    Ok(left)
}

fn parse_unary(chars: &[char], pos: &mut usize) -> Result<AwkExpr, String> {
    skip_spaces(chars, pos);
    if *pos < chars.len() && chars[*pos] == '!' {
        *pos += 1;
        let expr = parse_primary(chars, pos)?;
        return Ok(AwkExpr::UnaryOp("!".to_string(), Box::new(expr)));
    }
    if *pos < chars.len() && chars[*pos] == '-' && (*pos + 1 < chars.len() && chars[*pos + 1] != '-') {
        *pos += 1;
        let expr = parse_primary(chars, pos)?;
        return Ok(AwkExpr::UnaryOp("-".to_string(), Box::new(expr)));
    }
    parse_primary(chars, pos)
}

fn parse_primary(chars: &[char], pos: &mut usize) -> Result<AwkExpr, String> {
    skip_spaces(chars, pos);

    if *pos >= chars.len() {
        return Ok(AwkExpr::Literal("".to_string()));
    }

    // $field
    if chars[*pos] == '$' {
        *pos += 1;
        let inner = parse_primary(chars, pos)?;
        return Ok(AwkExpr::Field(Box::new(inner)));
    }

    // String literal
    if chars[*pos] == '"' {
        *pos += 1;
        let mut s = String::new();
        while *pos < chars.len() && chars[*pos] != '"' {
            if chars[*pos] == '\\' && *pos + 1 < chars.len() {
                *pos += 1;
                match chars[*pos] {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    '\\' => s.push('\\'),
                    '"' => s.push('"'),
                    c => {
                        s.push('\\');
                        s.push(c);
                    }
                }
            } else {
                s.push(chars[*pos]);
            }
            *pos += 1;
        }
        if *pos < chars.len() {
            *pos += 1; // skip closing "
        }
        return Ok(AwkExpr::Literal(s));
    }

    // Regex literal
    if chars[*pos] == '/' {
        *pos += 1;
        let mut s = String::new();
        while *pos < chars.len() && chars[*pos] != '/' {
            if chars[*pos] == '\\' && *pos + 1 < chars.len() {
                s.push(chars[*pos]);
                *pos += 1;
                s.push(chars[*pos]);
            } else {
                s.push(chars[*pos]);
            }
            *pos += 1;
        }
        if *pos < chars.len() {
            *pos += 1;
        }
        return Ok(AwkExpr::Literal(s));
    }

    // Parenthesized expression
    if chars[*pos] == '(' {
        *pos += 1;
        let expr = parse_expr(chars, pos)?;
        skip_spaces(chars, pos);
        if *pos < chars.len() && chars[*pos] == ')' {
            *pos += 1;
        }
        return Ok(expr);
    }

    // Number or identifier
    if chars[*pos].is_ascii_digit() {
        let start = *pos;
        while *pos < chars.len() && (chars[*pos].is_ascii_digit() || chars[*pos] == '.') {
            *pos += 1;
        }
        let num: String = chars[start..*pos].iter().collect();
        return Ok(AwkExpr::Literal(num));
    }

    if chars[*pos].is_alphabetic() || chars[*pos] == '_' {
        let start = *pos;
        while *pos < chars.len() && (chars[*pos].is_alphanumeric() || chars[*pos] == '_') {
            *pos += 1;
        }
        let name: String = chars[start..*pos].iter().collect();

        // Check for function call
        if *pos < chars.len() && chars[*pos] == '(' {
            *pos += 1;
            let mut call_args = Vec::new();
            skip_spaces(chars, pos);
            while *pos < chars.len() && chars[*pos] != ')' {
                let arg = parse_expr(chars, pos)?;
                call_args.push(arg);
                skip_spaces(chars, pos);
                if *pos < chars.len() && chars[*pos] == ',' {
                    *pos += 1;
                    skip_spaces(chars, pos);
                }
            }
            if *pos < chars.len() {
                *pos += 1; // skip )
            }
            return Ok(AwkExpr::FuncCall(name, call_args));
        }

        // Check for post-increment/decrement
        if *pos + 1 < chars.len() && chars[*pos] == '+' && chars[*pos + 1] == '+' {
            *pos += 2;
            return Ok(AwkExpr::PostIncr(name));
        }
        if *pos + 1 < chars.len() && chars[*pos] == '-' && chars[*pos + 1] == '-' {
            *pos += 2;
            return Ok(AwkExpr::PostDecr(name));
        }

        return Ok(AwkExpr::Var(name));
    }

    Ok(AwkExpr::Literal("".to_string()))
}

fn skip_whitespace(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && (chars[*pos] == ' ' || chars[*pos] == '\t' || chars[*pos] == '\n' || chars[*pos] == '\r') {
        *pos += 1;
    }
}

fn skip_spaces(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && (chars[*pos] == ' ' || chars[*pos] == '\t') {
        *pos += 1;
    }
}

fn starts_with_at(chars: &[char], pos: usize, s: &str) -> bool {
    let sc: Vec<char> = s.chars().collect();
    if pos + sc.len() > chars.len() {
        return false;
    }
    chars[pos..pos + sc.len()] == sc[..]
}

fn is_ident_char_at(chars: &[char], pos: usize) -> bool {
    pos < chars.len() && (chars[pos].is_alphanumeric() || chars[pos] == '_')
}

// Execution

fn process_line(
    line: &str,
    field_sep: &str,
    rules: &[AwkRule],
    vars: &mut HashMap<String, String>,
    nr: u64,
) {
    let fields: Vec<&str> = if field_sep == " " {
        line.split_whitespace().collect()
    } else {
        line.split(field_sep).collect()
    };

    vars.insert("NR".to_string(), nr.to_string());
    vars.insert("NF".to_string(), fields.len().to_string());
    vars.insert("0".to_string(), line.to_string());
    for (i, f) in fields.iter().enumerate() {
        vars.insert((i + 1).to_string(), f.to_string());
    }

    for rule in rules {
        let should_exec = match &rule.pattern {
            AwkPattern::Always => true,
            AwkPattern::Begin | AwkPattern::End => false,
            AwkPattern::Regex(re) => {
                Regex::new(re).map(|r| r.is_match(line)).unwrap_or(false)
            }
            AwkPattern::Expression(expr) => eval_condition(expr, &fields, vars),
        };

        if should_exec {
            execute_actions(&rule.actions, &fields, vars);
        }
    }
}

fn eval_condition(expr: &str, fields: &[&str], vars: &HashMap<String, String>) -> bool {
    // Simple condition evaluation for pattern expressions like NR>1, $1=="foo"
    let chars: Vec<char> = expr.chars().collect();
    let mut pos = 0;
    if let Ok(e) = parse_expr(&chars, &mut pos) {
        let val = eval_expr(&e, fields, vars);
        !val.is_empty() && val != "0"
    } else {
        false
    }
}

fn execute_actions(actions: &[AwkAction], fields: &[&str], vars: &mut HashMap<String, String>) {
    for action in actions {
        match action {
            AwkAction::Print(exprs) => {
                let ofs = vars.get("OFS").cloned().unwrap_or_else(|| " ".to_string());
                let vals: Vec<String> = exprs.iter().map(|e| eval_expr(e, fields, vars)).collect();
                println!("{}", vals.join(&ofs));
            }
            AwkAction::Assign(name, expr) => {
                let val = eval_expr(expr, fields, vars);
                vars.insert(name.clone(), val);
            }
            AwkAction::If(cond, then_block, else_block) => {
                let val = eval_expr(cond, fields, vars);
                if !val.is_empty() && val != "0" {
                    execute_actions(then_block, fields, vars);
                } else {
                    execute_actions(else_block, fields, vars);
                }
            }
            AwkAction::For(init, cond, update, body) => {
                execute_actions(&[(**init).clone()], fields, vars);
                loop {
                    let c = eval_expr(cond, fields, vars);
                    if c.is_empty() || c == "0" {
                        break;
                    }
                    execute_actions(body, fields, vars);
                    execute_actions(&[(**update).clone()], fields, vars);
                }
            }
            AwkAction::Noop => {}
        }
    }
}

impl Clone for AwkAction {
    fn clone(&self) -> Self {
        match self {
            AwkAction::Print(exprs) => AwkAction::Print(exprs.clone()),
            AwkAction::Assign(name, expr) => AwkAction::Assign(name.clone(), expr.clone()),
            AwkAction::If(cond, t, e) => AwkAction::If(cond.clone(), t.clone(), e.clone()),
            AwkAction::For(init, cond, update, body) => {
                AwkAction::For(init.clone(), cond.clone(), update.clone(), body.clone())
            }
            AwkAction::Noop => AwkAction::Noop,
        }
    }
}

fn eval_expr(expr: &AwkExpr, fields: &[&str], vars: &HashMap<String, String>) -> String {
    match expr {
        AwkExpr::Literal(s) => s.clone(),
        AwkExpr::Var(name) => vars.get(name).cloned().unwrap_or_default(),
        AwkExpr::Field(idx_expr) => {
            let idx_str = eval_expr(idx_expr, fields, vars);
            let idx: usize = idx_str.parse().unwrap_or(0);
            if idx == 0 {
                vars.get("0").cloned().unwrap_or_default()
            } else if idx <= fields.len() {
                fields[idx - 1].to_string()
            } else {
                String::new()
            }
        }
        AwkExpr::BinOp(left, op, right) => {
            let l = eval_expr(left, fields, vars);
            let r = eval_expr(right, fields, vars);
            match op.as_str() {
                "+" => {
                    let ln: f64 = l.parse().unwrap_or(0.0);
                    let rn: f64 = r.parse().unwrap_or(0.0);
                    format_num(ln + rn)
                }
                "-" => {
                    let ln: f64 = l.parse().unwrap_or(0.0);
                    let rn: f64 = r.parse().unwrap_or(0.0);
                    format_num(ln - rn)
                }
                "*" => {
                    let ln: f64 = l.parse().unwrap_or(0.0);
                    let rn: f64 = r.parse().unwrap_or(0.0);
                    format_num(ln * rn)
                }
                "/" => {
                    let ln: f64 = l.parse().unwrap_or(0.0);
                    let rn: f64 = r.parse().unwrap_or(0.0);
                    if rn == 0.0 {
                        "inf".to_string()
                    } else {
                        format_num(ln / rn)
                    }
                }
                "%" => {
                    let ln: f64 = l.parse().unwrap_or(0.0);
                    let rn: f64 = r.parse().unwrap_or(0.0);
                    format_num(ln % rn)
                }
                "==" => bool_str(l == r),
                "!=" => bool_str(l != r),
                "<" => {
                    let cmp = num_or_str_cmp(&l, &r);
                    bool_str(cmp == std::cmp::Ordering::Less)
                }
                "<=" => {
                    let cmp = num_or_str_cmp(&l, &r);
                    bool_str(cmp != std::cmp::Ordering::Greater)
                }
                ">" => {
                    let cmp = num_or_str_cmp(&l, &r);
                    bool_str(cmp == std::cmp::Ordering::Greater)
                }
                ">=" => {
                    let cmp = num_or_str_cmp(&l, &r);
                    bool_str(cmp != std::cmp::Ordering::Less)
                }
                _ => String::new(),
            }
        }
        AwkExpr::Concat(left, right) => {
            let l = eval_expr(left, fields, vars);
            let r = eval_expr(right, fields, vars);
            format!("{}{}", l, r)
        }
        AwkExpr::FuncCall(name, call_args) => {
            let vals: Vec<String> = call_args.iter().map(|a| eval_expr(a, fields, vars)).collect();
            match name.as_str() {
                "length" => {
                    if vals.is_empty() {
                        vars.get("0")
                            .map(|s| s.len().to_string())
                            .unwrap_or_else(|| "0".to_string())
                    } else {
                        vals[0].len().to_string()
                    }
                }
                "substr" => {
                    if vals.len() >= 2 {
                        let s = &vals[0];
                        let start = vals[1].parse::<usize>().unwrap_or(1).saturating_sub(1);
                        if vals.len() >= 3 {
                            let len = vals[2].parse::<usize>().unwrap_or(s.len());
                            s.chars().skip(start).take(len).collect()
                        } else {
                            s.chars().skip(start).collect()
                        }
                    } else {
                        String::new()
                    }
                }
                "index" => {
                    if vals.len() >= 2 {
                        match vals[0].find(&vals[1]) {
                            Some(i) => (i + 1).to_string(),
                            None => "0".to_string(),
                        }
                    } else {
                        "0".to_string()
                    }
                }
                "split" => {
                    // split(s, a, fs) — not fully implementable without mutable array support
                    "0".to_string()
                }
                "tolower" => vals.first().map(|s| s.to_lowercase()).unwrap_or_default(),
                "toupper" => vals.first().map(|s| s.to_uppercase()).unwrap_or_default(),
                "sprintf" => {
                    if vals.is_empty() {
                        String::new()
                    } else {
                        // Basic sprintf — just handle %s and %d
                        let fmt = &vals[0];
                        let mut result = String::new();
                        let mut arg_idx = 1;
                        let fchars: Vec<char> = fmt.chars().collect();
                        let mut fi = 0;
                        while fi < fchars.len() {
                            if fchars[fi] == '%' && fi + 1 < fchars.len() {
                                fi += 1;
                                match fchars[fi] {
                                    's' => {
                                        if arg_idx < vals.len() {
                                            result.push_str(&vals[arg_idx]);
                                            arg_idx += 1;
                                        }
                                    }
                                    'd' => {
                                        if arg_idx < vals.len() {
                                            let n: f64 =
                                                vals[arg_idx].parse().unwrap_or(0.0);
                                            result.push_str(&format!("{}", n as i64));
                                            arg_idx += 1;
                                        }
                                    }
                                    '%' => result.push('%'),
                                    _ => {
                                        result.push('%');
                                        result.push(fchars[fi]);
                                    }
                                }
                            } else {
                                result.push(fchars[fi]);
                            }
                            fi += 1;
                        }
                        result
                    }
                }
                "int" => {
                    let n: f64 = vals.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    format!("{}", n as i64)
                }
                "sqrt" => {
                    let n: f64 = vals.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    format_num(n.sqrt())
                }
                "sin" => {
                    let n: f64 = vals.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    format_num(n.sin())
                }
                "cos" => {
                    let n: f64 = vals.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    format_num(n.cos())
                }
                "log" => {
                    let n: f64 = vals.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    format_num(n.ln())
                }
                "exp" => {
                    let n: f64 = vals.first().and_then(|s| s.parse().ok()).unwrap_or(0.0);
                    format_num(n.exp())
                }
                "gsub" | "sub" => {
                    // Not fully implementable without mutable references
                    "0".to_string()
                }
                _ => String::new(),
            }
        }
        AwkExpr::Match(expr, pattern) => {
            let val = eval_expr(expr, fields, vars);
            let matched = Regex::new(pattern)
                .map(|r| r.is_match(&val))
                .unwrap_or(false);
            bool_str(matched)
        }
        AwkExpr::UnaryOp(op, expr) => {
            let val = eval_expr(expr, fields, vars);
            match op.as_str() {
                "!" => bool_str(val.is_empty() || val == "0"),
                "-" => {
                    let n: f64 = val.parse().unwrap_or(0.0);
                    format_num(-n)
                }
                _ => val,
            }
        }
        AwkExpr::PostIncr(name) => {
            let val = vars.get(name).cloned().unwrap_or_default();
            let n: f64 = val.parse().unwrap_or(0.0);
            // We can't mutate vars here since we only have immutable ref in eval_expr
            // Return current value; mutation happens in execute_actions
            format_num(n)
        }
        AwkExpr::PostDecr(name) => {
            let val = vars.get(name).cloned().unwrap_or_default();
            let n: f64 = val.parse().unwrap_or(0.0);
            format_num(n)
        }
    }
}

fn format_num(n: f64) -> String {
    if n == n.floor() && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{}", n)
    }
}

fn bool_str(b: bool) -> String {
    if b { "1" } else { "0" }.to_string()
}

fn num_or_str_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    if let (Ok(an), Ok(bn)) = (a.parse::<f64>(), b.parse::<f64>()) {
        an.partial_cmp(&bn).unwrap_or(std::cmp::Ordering::Equal)
    } else {
        a.cmp(b)
    }
}
