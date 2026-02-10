use super::ast::*;
use super::lexer::Lexer;
use super::token::Token;

/// Shell parser: recursive descent.
pub struct Parser {
    lexer: Lexer,
    current: Token,
}

impl Parser {
    pub fn new(input: &str) -> Self {
        let mut lexer = Lexer::new(input);
        let current = lexer.next_token();
        Parser { lexer, current }
    }

    fn advance(&mut self) -> Token {
        let old = std::mem::replace(&mut self.current, self.lexer.next_token());
        old
    }

    fn peek(&self) -> &Token {
        &self.current
    }

    fn eat(&mut self, expected: &Token) -> bool {
        if std::mem::discriminant(&self.current) == std::mem::discriminant(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.current, Token::Newline) {
            self.advance();
        }
    }

    fn expect_word(&mut self) -> Result<String, String> {
        match &self.current {
            Token::Word(w) => {
                let w = w.clone();
                self.advance();
                Ok(w)
            }
            other => Err(format!("expected word, got {:?}", other)),
        }
    }

    /// Parse the entire input as a program.
    pub fn parse_program(&mut self) -> Result<Program, String> {
        self.skip_newlines();
        let mut commands = Vec::new();

        while self.current != Token::Eof
            && self.current != Token::RBrace
            && self.current != Token::RParen
            && self.current != Token::Fi
            && self.current != Token::Done
            && self.current != Token::Else
            && self.current != Token::Elif
            && self.current != Token::Esac
            && self.current != Token::Then
            && self.current != Token::Do
        {
            let cmd = self.parse_complete_command()?;
            commands.push(cmd);

            // Skip terminators
            while matches!(self.current, Token::Semi | Token::Newline | Token::Amp) {
                self.advance();
            }
        }

        Ok(Program { commands })
    }

    /// Parse a list of commands until a keyword boundary (used for if/while bodies).
    fn parse_compound_list(&mut self) -> Result<Program, String> {
        self.skip_newlines();
        let mut commands = Vec::new();

        loop {
            // Check for terminators
            if matches!(
                self.current,
                Token::Eof
                    | Token::Fi
                    | Token::Done
                    | Token::Else
                    | Token::Elif
                    | Token::Then
                    | Token::Do
                    | Token::Esac
                    | Token::RBrace
                    | Token::RParen
            ) {
                break;
            }

            if !self.current.is_command_start() && !matches!(self.current, Token::Word(_)) {
                break;
            }

            let cmd = self.parse_complete_command()?;
            commands.push(cmd);

            while matches!(self.current, Token::Semi | Token::Newline | Token::Amp) {
                self.advance();
            }
        }

        Ok(Program { commands })
    }

    /// Parse a complete command (pipeline && pipeline || pipeline ...)
    fn parse_complete_command(&mut self) -> Result<CompleteCommand, String> {
        let first = self.parse_pipeline()?;
        let mut rest = Vec::new();

        loop {
            let op = match &self.current {
                Token::And => ListOp::And,
                Token::Or => ListOp::Or,
                _ => break,
            };
            self.advance();
            self.skip_newlines();
            let pipeline = self.parse_pipeline()?;
            rest.push((op, pipeline));
        }

        let background = matches!(self.current, Token::Amp);

        Ok(CompleteCommand {
            first,
            rest,
            background,
        })
    }

    /// Parse a pipeline: [!] command [| command ...]
    fn parse_pipeline(&mut self) -> Result<Pipeline, String> {
        let negated = if matches!(self.current, Token::Bang) {
            self.advance();
            true
        } else {
            false
        };

        let mut commands = Vec::new();
        let cmd = self.parse_command()?;
        commands.push(cmd);

        while matches!(self.current, Token::Pipe) {
            self.advance();
            self.skip_newlines();
            let cmd = self.parse_command()?;
            commands.push(cmd);
        }

        Ok(Pipeline { commands, negated })
    }

    /// Parse a single command.
    fn parse_command(&mut self) -> Result<Command, String> {
        match &self.current {
            Token::If => self.parse_if(),
            Token::For => self.parse_for(),
            Token::While => self.parse_while(),
            Token::Until => self.parse_until(),
            Token::Case => self.parse_case(),
            Token::LParen => self.parse_subshell(),
            Token::LBrace => self.parse_brace_group(),
            Token::Function => self.parse_function_keyword(),
            Token::Word(_) => {
                // Could be a function definition: name() { ... }
                // Or a simple command
                self.parse_simple_or_func()
            }
            other => Err(format!("unexpected token: {:?}", other)),
        }
    }

    fn parse_simple_or_func(&mut self) -> Result<Command, String> {
        // Peek ahead: if we see word followed by (, it might be a function def
        let first_word = match &self.current {
            Token::Word(w) => w.clone(),
            _ => return self.parse_simple_command(),
        };

        // Check for function definition: name() { body }
        let saved_pos = self.lexer.pos();
        let saved_current = self.current.clone();

        self.advance();
        if matches!(self.current, Token::LParen) {
            self.advance();
            if matches!(self.current, Token::RParen) {
                self.advance();
                self.skip_newlines();
                let body = self.parse_command()?;
                return Ok(Command::FuncDef(FuncDef {
                    name: first_word,
                    body: Box::new(body),
                }));
            }
        }

        // Not a function def — restore and parse as simple command
        // We can't easily restore the lexer, so we'll just re-parse
        // Restore lexer state and parse as simple command with first word
        self.current = saved_current;
        self.lexer = Lexer::new(&self.lexer_remaining_from_chars(saved_pos));
        self.current = Token::Word(first_word.clone());

        self.parse_simple_command_starting_with(first_word)
    }

    fn lexer_remaining_from_chars(&self, pos: usize) -> String {
        self.lexer.chars_slice()[pos..].iter().collect()
    }

    fn parse_simple_command(&mut self) -> Result<Command, String> {
        let mut cmd = SimpleCommand {
            assignments: Vec::new(),
            words: Vec::new(),
            redirections: Vec::new(),
        };

        loop {
            match &self.current {
                Token::Word(w) => {
                    let w = w.clone();

                    // Check if it's an assignment (name=value)
                    if cmd.words.is_empty() {
                        if let Some(eq_pos) = w.find('=') {
                            let name = &w[..eq_pos];
                            if !name.is_empty()
                                && name
                                    .chars()
                                    .all(|c| c.is_alphanumeric() || c == '_')
                                && name.chars().next().map(|c| !c.is_ascii_digit()).unwrap_or(false)
                            {
                                let value = &w[eq_pos + 1..];
                                cmd.assignments.push(Assignment {
                                    name: name.to_string(),
                                    value: parse_word_from_str(value),
                                });
                                self.advance();
                                continue;
                            }
                        }
                    }

                    cmd.words.push(parse_word_from_str(&w));
                    self.advance();
                }

                Token::IoNumber(n) => {
                    let fd = *n;
                    self.advance();
                    let redir = self.parse_redirect(Some(fd))?;
                    cmd.redirections.push(redir);
                }

                Token::Less | Token::Great | Token::DGreat | Token::GreatAnd | Token::LessAnd | Token::DLess | Token::DLessDash | Token::TLess => {
                    let redir = self.parse_redirect(None)?;
                    cmd.redirections.push(redir);
                }

                _ => break,
            }
        }

        Ok(Command::Simple(cmd))
    }

    fn parse_simple_command_starting_with(&mut self, first_word: String) -> Result<Command, String> {
        let mut cmd = SimpleCommand {
            assignments: Vec::new(),
            words: Vec::new(),
            redirections: Vec::new(),
        };

        // Check if first word is an assignment (name=value)
        if let Some(eq_pos) = first_word.find('=') {
            let name = &first_word[..eq_pos];
            if !name.is_empty()
                && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                && name.chars().next().map(|c| !c.is_ascii_digit()).unwrap_or(false)
            {
                let value = &first_word[eq_pos + 1..];
                cmd.assignments.push(Assignment {
                    name: name.to_string(),
                    value: parse_word_from_str(value),
                });
            } else {
                cmd.words.push(parse_word_from_str(&first_word));
            }
        } else {
            cmd.words.push(parse_word_from_str(&first_word));
        }

        self.advance(); // skip the first word we already consumed

        loop {
            match &self.current {
                Token::Word(w) => {
                    let w = w.clone();
                    cmd.words.push(parse_word_from_str(&w));
                    self.advance();
                }

                Token::IoNumber(n) => {
                    let fd = *n;
                    self.advance();
                    let redir = self.parse_redirect(Some(fd))?;
                    cmd.redirections.push(redir);
                }

                Token::Less | Token::Great | Token::DGreat | Token::GreatAnd | Token::LessAnd | Token::DLess | Token::DLessDash | Token::TLess => {
                    let redir = self.parse_redirect(None)?;
                    cmd.redirections.push(redir);
                }

                _ => break,
            }
        }

        Ok(Command::Simple(cmd))
    }

    fn parse_redirect(&mut self, fd: Option<i32>) -> Result<Redirect, String> {
        let kind = match &self.current {
            Token::Less => RedirectKind::Input,
            Token::Great => RedirectKind::Output,
            Token::DGreat => RedirectKind::Append,
            Token::GreatAnd => RedirectKind::DupOutput,
            Token::LessAnd => RedirectKind::DupInput,
            Token::TLess => RedirectKind::HereString,
            Token::DLess | Token::DLessDash => {
                // Here-doc: body was already read by the lexer
                let body = self.lexer.heredoc_bodies.pop().unwrap_or_default();
                self.advance();
                return Ok(Redirect {
                    fd,
                    kind: RedirectKind::HereDoc(body),
                    target: Word::literal(""),
                });
            }
            other => return Err(format!("expected redirection operator, got {:?}", other)),
        };
        self.advance();

        let target = match &self.current {
            Token::Word(w) => {
                let w = parse_word_from_str(w);
                self.advance();
                w
            }
            other => return Err(format!("expected redirect target, got {:?}", other)),
        };

        Ok(Redirect {
            fd,
            kind,
            target,
        })
    }

    fn parse_if(&mut self) -> Result<Command, String> {
        self.advance(); // eat 'if'
        self.skip_newlines();

        let condition = self.parse_compound_list()?;

        if !matches!(self.current, Token::Then) {
            return Err(format!("expected 'then', got {:?}", self.current));
        }
        self.advance();
        self.skip_newlines();

        let then_body = self.parse_compound_list()?;

        let mut elifs = Vec::new();
        let mut else_body = None;

        loop {
            match &self.current {
                Token::Elif => {
                    self.advance();
                    self.skip_newlines();
                    let elif_cond = self.parse_compound_list()?;
                    if !matches!(self.current, Token::Then) {
                        return Err(format!("expected 'then' after elif, got {:?}", self.current));
                    }
                    self.advance();
                    self.skip_newlines();
                    let elif_body = self.parse_compound_list()?;
                    elifs.push((elif_cond, elif_body));
                }
                Token::Else => {
                    self.advance();
                    self.skip_newlines();
                    else_body = Some(self.parse_compound_list()?);
                    break;
                }
                _ => break,
            }
        }

        if !matches!(self.current, Token::Fi) {
            return Err(format!("expected 'fi', got {:?}", self.current));
        }
        self.advance();

        Ok(Command::If(IfClause {
            condition,
            then_body,
            elifs,
            else_body,
        }))
    }

    fn parse_for(&mut self) -> Result<Command, String> {
        self.advance(); // eat 'for'
        let var = self.expect_word()?;

        self.skip_newlines();

        let words = if matches!(self.current, Token::In) || matches!(&self.current, Token::Word(w) if w == "in") {
            self.advance(); // eat 'in'
            let mut words = Vec::new();
            while matches!(self.current, Token::Word(_)) {
                if let Token::Word(w) = &self.current {
                    words.push(parse_word_from_str(w));
                }
                self.advance();
            }
            Some(words)
        } else {
            None // for var; do => iterate over positional params
        };

        // Skip ; or newline before do
        while matches!(self.current, Token::Semi | Token::Newline) {
            self.advance();
        }

        if !matches!(self.current, Token::Do) {
            return Err(format!("expected 'do', got {:?}", self.current));
        }
        self.advance();
        self.skip_newlines();

        let body = self.parse_compound_list()?;

        if !matches!(self.current, Token::Done) {
            return Err(format!("expected 'done', got {:?}", self.current));
        }
        self.advance();

        Ok(Command::For(ForClause { var, words, body }))
    }

    fn parse_while(&mut self) -> Result<Command, String> {
        self.advance(); // eat 'while'
        self.skip_newlines();

        let condition = self.parse_compound_list()?;

        if !matches!(self.current, Token::Do) {
            return Err(format!("expected 'do', got {:?}", self.current));
        }
        self.advance();
        self.skip_newlines();

        let body = self.parse_compound_list()?;

        if !matches!(self.current, Token::Done) {
            return Err(format!("expected 'done', got {:?}", self.current));
        }
        self.advance();

        Ok(Command::While(WhileClause { condition, body }))
    }

    fn parse_until(&mut self) -> Result<Command, String> {
        self.advance(); // eat 'until'
        self.skip_newlines();

        let condition = self.parse_compound_list()?;

        if !matches!(self.current, Token::Do) {
            return Err(format!("expected 'do', got {:?}", self.current));
        }
        self.advance();
        self.skip_newlines();

        let body = self.parse_compound_list()?;

        if !matches!(self.current, Token::Done) {
            return Err(format!("expected 'done', got {:?}", self.current));
        }
        self.advance();

        Ok(Command::Until(UntilClause { condition, body }))
    }

    fn parse_case(&mut self) -> Result<Command, String> {
        self.advance(); // eat 'case'
        let word_str = self.expect_word()?;
        let word = parse_word_from_str(&word_str);

        self.skip_newlines();
        if !matches!(self.current, Token::In) {
            return Err(format!("expected 'in', got {:?}", self.current));
        }
        self.advance();
        self.skip_newlines();

        let mut arms = Vec::new();
        while !matches!(self.current, Token::Esac | Token::Eof) {
            // Parse pattern list
            let mut patterns = Vec::new();
            // Skip optional (
            if matches!(self.current, Token::LParen) {
                self.advance();
            }
            loop {
                let pat_str = self.expect_word()?;
                patterns.push(parse_word_from_str(&pat_str));
                if matches!(self.current, Token::Pipe) {
                    self.advance();
                } else {
                    break;
                }
            }

            // Expect )
            if matches!(self.current, Token::RParen) {
                self.advance();
            }
            self.skip_newlines();

            let body = self.parse_compound_list()?;

            // Expect ;; or newline before esac
            while matches!(self.current, Token::Semi | Token::Newline) {
                self.advance();
            }

            arms.push(CaseArm { patterns, body });
        }

        if !matches!(self.current, Token::Esac) {
            return Err(format!("expected 'esac', got {:?}", self.current));
        }
        self.advance();

        Ok(Command::Case(CaseClause { word, arms }))
    }

    fn parse_subshell(&mut self) -> Result<Command, String> {
        self.advance(); // eat (
        self.skip_newlines();
        let program = self.parse_program()?;
        if !matches!(self.current, Token::RParen) {
            return Err(format!("expected ')', got {:?}", self.current));
        }
        self.advance();
        Ok(Command::Subshell(program))
    }

    fn parse_brace_group(&mut self) -> Result<Command, String> {
        self.advance(); // eat {
        self.skip_newlines();
        let program = self.parse_program()?;
        if !matches!(self.current, Token::RBrace) {
            return Err(format!("expected '}}', got {:?}", self.current));
        }
        self.advance();
        Ok(Command::BraceGroup(program))
    }

    fn parse_function_keyword(&mut self) -> Result<Command, String> {
        self.advance(); // eat 'function'
        let name = self.expect_word()?;
        // Optional ()
        if matches!(self.current, Token::LParen) {
            self.advance();
            if matches!(self.current, Token::RParen) {
                self.advance();
            }
        }
        self.skip_newlines();
        let body = self.parse_command()?;
        Ok(Command::FuncDef(FuncDef {
            name,
            body: Box::new(body),
        }))
    }
}

/// Parse a raw word string from the lexer into a Word AST node.
pub fn parse_word_from_str(s: &str) -> Word {
    let mut parts = Vec::new();
    let chars: Vec<char> = s.chars().collect();
    let mut pos = 0;

    while pos < chars.len() {
        match chars[pos] {
            '\'' => {
                pos += 1;
                let start = pos;
                while pos < chars.len() && chars[pos] != '\'' {
                    pos += 1;
                }
                let content: String = chars[start..pos].iter().collect();
                parts.push(WordPart::SingleQuoted(content));
                if pos < chars.len() {
                    pos += 1; // skip closing '
                }
            }

            '"' => {
                pos += 1;
                let mut inner_parts = Vec::new();
                while pos < chars.len() && chars[pos] != '"' {
                    if chars[pos] == '\\' && pos + 1 < chars.len() {
                        let next = chars[pos + 1];
                        if matches!(next, '$' | '`' | '"' | '\\' | '\n') {
                            if next != '\n' {
                                inner_parts.push(WordPart::Literal(next.to_string()));
                            }
                            pos += 2;
                        } else {
                            inner_parts.push(WordPart::Literal("\\".to_string()));
                            pos += 1;
                        }
                    } else if chars[pos] == '$' {
                        pos += 1;
                        let part = parse_dollar(&chars, &mut pos);
                        inner_parts.push(part);
                    } else if chars[pos] == '`' {
                        pos += 1;
                        let start = pos;
                        while pos < chars.len() && chars[pos] != '`' {
                            pos += 1;
                        }
                        let cmd: String = chars[start..pos].iter().collect();
                        inner_parts.push(WordPart::CommandSub(cmd));
                        if pos < chars.len() {
                            pos += 1;
                        }
                    } else {
                        let start = pos;
                        while pos < chars.len() && !matches!(chars[pos], '"' | '$' | '`' | '\\') {
                            pos += 1;
                        }
                        let lit: String = chars[start..pos].iter().collect();
                        inner_parts.push(WordPart::Literal(lit));
                    }
                }
                if pos < chars.len() {
                    pos += 1; // skip closing "
                }
                parts.push(WordPart::DoubleQuoted(inner_parts));
            }

            '\\' => {
                pos += 1;
                if pos < chars.len() {
                    parts.push(WordPart::Literal(chars[pos].to_string()));
                    pos += 1;
                }
            }

            '$' => {
                pos += 1;
                let part = parse_dollar(&chars, &mut pos);
                parts.push(part);
            }

            '`' => {
                pos += 1;
                let start = pos;
                while pos < chars.len() && chars[pos] != '`' {
                    pos += 1;
                }
                let cmd: String = chars[start..pos].iter().collect();
                parts.push(WordPart::CommandSub(cmd));
                if pos < chars.len() {
                    pos += 1;
                }
            }

            '*' | '?' => {
                parts.push(WordPart::Glob(chars[pos].to_string()));
                pos += 1;
            }

            '[' => {
                let start = pos;
                pos += 1;
                while pos < chars.len() && chars[pos] != ']' {
                    pos += 1;
                }
                if pos < chars.len() {
                    pos += 1;
                }
                let glob: String = chars[start..pos].iter().collect();
                parts.push(WordPart::Glob(glob));
            }

            _ => {
                let start = pos;
                while pos < chars.len()
                    && !matches!(chars[pos], '\'' | '"' | '\\' | '$' | '`' | '*' | '?' | '[')
                {
                    pos += 1;
                }
                let lit: String = chars[start..pos].iter().collect();
                parts.push(WordPart::Literal(lit));
            }
        }
    }

    Word { parts }
}

fn parse_dollar(chars: &[char], pos: &mut usize) -> WordPart {
    if *pos >= chars.len() {
        return WordPart::Literal("$".to_string());
    }

    match chars[*pos] {
        '(' => {
            *pos += 1;
            if *pos < chars.len() && chars[*pos] == '(' {
                // $(( arithmetic ))
                *pos += 1;
                let start = *pos;
                let mut depth = 2;
                while *pos < chars.len() && depth > 0 {
                    if chars[*pos] == '(' {
                        depth += 1;
                    }
                    if chars[*pos] == ')' {
                        depth -= 1;
                    }
                    if depth > 0 {
                        *pos += 1;
                    }
                }
                let expr: String = chars[start..*pos].iter().collect();
                *pos += 1; // skip last )
                if *pos < chars.len() && chars[*pos] == ')' {
                    *pos += 1;
                }
                WordPart::ArithmeticSub(expr)
            } else {
                // $( command substitution )
                let start = *pos;
                let mut depth = 1;
                while *pos < chars.len() && depth > 0 {
                    if chars[*pos] == '(' {
                        depth += 1;
                    }
                    if chars[*pos] == ')' {
                        depth -= 1;
                    }
                    if depth > 0 {
                        *pos += 1;
                    }
                }
                let cmd: String = chars[start..*pos].iter().collect();
                if *pos < chars.len() {
                    *pos += 1; // skip )
                }
                WordPart::CommandSub(cmd)
            }
        }

        '{' => {
            *pos += 1;

            // ${#VAR} — length
            if *pos < chars.len() && chars[*pos] == '#' {
                *pos += 1;
                let start = *pos;
                while *pos < chars.len() && chars[*pos] != '}' {
                    *pos += 1;
                }
                let name: String = chars[start..*pos].iter().collect();
                if *pos < chars.len() {
                    *pos += 1; // skip }
                }
                return WordPart::VarLength(name);
            }

            let start = *pos;
            // Read variable name
            while *pos < chars.len()
                && (chars[*pos].is_alphanumeric() || chars[*pos] == '_' || chars[*pos] == '?' || chars[*pos] == '#')
            {
                *pos += 1;
            }
            let name: String = chars[start..*pos].iter().collect();

            if *pos < chars.len() && chars[*pos] == '}' {
                *pos += 1;
                return WordPart::Variable(name);
            }

            // Check for modifiers
            if *pos + 1 < chars.len() && chars[*pos] == ':' && chars[*pos + 1] == '-' {
                *pos += 2; // skip :-
                let def_start = *pos;
                let mut depth = 1;
                while *pos < chars.len() && depth > 0 {
                    if chars[*pos] == '{' {
                        depth += 1;
                    }
                    if chars[*pos] == '}' {
                        depth -= 1;
                    }
                    if depth > 0 {
                        *pos += 1;
                    }
                }
                let default: String = chars[def_start..*pos].iter().collect();
                if *pos < chars.len() {
                    *pos += 1; // skip }
                }
                return WordPart::VarDefault(name, parse_word_from_str(&default));
            }

            if *pos + 1 < chars.len() && chars[*pos] == ':' && chars[*pos + 1] == '=' {
                *pos += 2;
                let def_start = *pos;
                let mut depth = 1;
                while *pos < chars.len() && depth > 0 {
                    if chars[*pos] == '{' {
                        depth += 1;
                    }
                    if chars[*pos] == '}' {
                        depth -= 1;
                    }
                    if depth > 0 {
                        *pos += 1;
                    }
                }
                let default: String = chars[def_start..*pos].iter().collect();
                if *pos < chars.len() {
                    *pos += 1;
                }
                return WordPart::VarAssignDefault(name, parse_word_from_str(&default));
            }

            // Skip to closing }
            while *pos < chars.len() && chars[*pos] != '}' {
                *pos += 1;
            }
            if *pos < chars.len() {
                *pos += 1;
            }
            WordPart::Variable(name)
        }

        '?' => {
            *pos += 1;
            WordPart::SpecialVar(SpecialVar::ExitStatus)
        }
        '#' => {
            *pos += 1;
            WordPart::SpecialVar(SpecialVar::NumArgs)
        }
        '@' => {
            *pos += 1;
            WordPart::SpecialVar(SpecialVar::AllArgs)
        }
        '*' => {
            *pos += 1;
            WordPart::SpecialVar(SpecialVar::AllArgsStar)
        }
        '$' => {
            *pos += 1;
            WordPart::SpecialVar(SpecialVar::ProcessId)
        }
        '!' => {
            *pos += 1;
            WordPart::SpecialVar(SpecialVar::ProcessId) // $! — last bg pid (approximate)
        }

        '0'..='9' => {
            let n = chars[*pos] as u32 - '0' as u32;
            *pos += 1;
            WordPart::SpecialVar(SpecialVar::Positional(n))
        }

        c if c.is_alphabetic() || c == '_' => {
            let start = *pos;
            while *pos < chars.len() && (chars[*pos].is_alphanumeric() || chars[*pos] == '_') {
                *pos += 1;
            }
            let name: String = chars[start..*pos].iter().collect();
            WordPart::Variable(name)
        }

        _ => WordPart::Literal("$".to_string()),
    }
}

