use super::token::{keyword_token, Token};

/// Lexer for shell scripts.
pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    /// Whether to recognize keywords in the next word position
    keyword_ok: bool,
    /// Pending here-doc delimiters to read after newline
    heredoc_pending: Vec<(String, bool)>, // (delimiter, strip_tabs)
    /// Stored here-doc bodies read after newline
    pub heredoc_bodies: Vec<String>,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            chars: input.chars().collect(),
            pos: 0,
            keyword_ok: true,
            heredoc_pending: Vec::new(),
            heredoc_bodies: Vec::new(),
        }
    }

    pub fn pos(&self) -> usize {
        self.pos
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn skip_whitespace_no_newline(&mut self) {
        while let Some(ch) = self.peek() {
            if ch == ' ' || ch == '\t' {
                self.advance();
            } else if ch == '\\' && self.chars.get(self.pos + 1) == Some(&'\n') {
                // Line continuation
                self.pos += 2;
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        if self.peek() == Some('#') {
            while let Some(ch) = self.peek() {
                if ch == '\n' {
                    break;
                }
                self.advance();
            }
        }
    }

    /// Read here-doc bodies after a newline when here-docs are pending.
    fn read_pending_heredocs(&mut self) {
        let pending = std::mem::take(&mut self.heredoc_pending);
        for (delim, strip_tabs) in pending {
            let mut body = String::new();
            loop {
                // Read a line
                let mut line = String::new();
                loop {
                    match self.advance() {
                        Some('\n') => break,
                        Some(ch) => line.push(ch),
                        None => break,
                    }
                }

                let trimmed = if strip_tabs {
                    line.trim_start_matches('\t').to_string()
                } else {
                    line.clone()
                };

                if trimmed == delim {
                    break;
                }

                body.push_str(&line);
                body.push('\n');
            }
            self.heredoc_bodies.push(body);
        }
    }

    /// Get the next token.
    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_no_newline();
        self.skip_comment();

        let ch = match self.peek() {
            Some(ch) => ch,
            None => return Token::Eof,
        };

        match ch {
            '\n' => {
                self.advance();
                if !self.heredoc_pending.is_empty() {
                    self.read_pending_heredocs();
                }
                self.keyword_ok = true;
                Token::Newline
            }

            '|' => {
                self.advance();
                if self.peek() == Some('|') {
                    self.advance();
                    self.keyword_ok = true;
                    Token::Or
                } else {
                    self.keyword_ok = true;
                    Token::Pipe
                }
            }

            '&' => {
                self.advance();
                if self.peek() == Some('&') {
                    self.advance();
                    self.keyword_ok = true;
                    Token::And
                } else {
                    self.keyword_ok = true;
                    Token::Amp
                }
            }

            ';' => {
                self.advance();
                self.keyword_ok = true;
                Token::Semi
            }

            '(' => {
                self.advance();
                self.keyword_ok = true;
                Token::LParen
            }

            ')' => {
                self.advance();
                self.keyword_ok = true;
                Token::RParen
            }

            '<' => {
                self.advance();
                match self.peek() {
                    Some('<') => {
                        self.advance();
                        if self.peek() == Some('<') {
                            self.advance();
                            self.keyword_ok = false;
                            Token::TLess
                        } else if self.peek() == Some('-') {
                            self.advance();
                            // Read here-doc delimiter
                            self.skip_whitespace_no_newline();
                            let delim = self.read_heredoc_delimiter();
                            self.heredoc_pending.push((delim, true));
                            self.keyword_ok = false;
                            Token::DLessDash
                        } else {
                            self.skip_whitespace_no_newline();
                            let delim = self.read_heredoc_delimiter();
                            self.heredoc_pending.push((delim, false));
                            self.keyword_ok = false;
                            Token::DLess
                        }
                    }
                    Some('&') => {
                        self.advance();
                        self.keyword_ok = false;
                        Token::LessAnd
                    }
                    _ => {
                        self.keyword_ok = false;
                        Token::Less
                    }
                }
            }

            '>' => {
                self.advance();
                match self.peek() {
                    Some('>') => {
                        self.advance();
                        self.keyword_ok = false;
                        Token::DGreat
                    }
                    Some('&') => {
                        self.advance();
                        self.keyword_ok = false;
                        Token::GreatAnd
                    }
                    _ => {
                        self.keyword_ok = false;
                        Token::Great
                    }
                }
            }

            // Could be an IO number (like 2> or 1<)
            '0'..='9' if self.is_io_number() => {
                let num = self.read_number();
                Token::IoNumber(num)
            }

            _ => {
                // Read a word
                let word = self.read_word();
                if self.keyword_ok {
                    if let Some(kw) = keyword_token(&word) {
                        self.keyword_ok = matches!(
                            kw,
                            Token::Do
                                | Token::Then
                                | Token::Else
                                | Token::Elif
                                | Token::LBrace
                                | Token::Bang
                                | Token::In
                        );
                        return kw;
                    }
                }
                self.keyword_ok = false;
                Token::Word(word)
            }
        }
    }

    fn is_io_number(&self) -> bool {
        let start = self.pos;
        let mut p = start;
        while p < self.chars.len() && self.chars[p].is_ascii_digit() {
            p += 1;
        }
        if p > start && p < self.chars.len() {
            let next = self.chars[p];
            next == '<' || next == '>'
        } else {
            false
        }
    }

    fn read_number(&mut self) -> i32 {
        let mut num = 0;
        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                num = num * 10 + (ch as i32 - '0' as i32);
                self.advance();
            } else {
                break;
            }
        }
        num
    }

    fn read_heredoc_delimiter(&mut self) -> String {
        let mut delim = String::new();
        let mut quoted = false;

        if self.peek() == Some('\'') {
            quoted = true;
            self.advance();
            while let Some(ch) = self.advance() {
                if ch == '\'' {
                    break;
                }
                delim.push(ch);
            }
        } else if self.peek() == Some('"') {
            quoted = true;
            self.advance();
            while let Some(ch) = self.advance() {
                if ch == '"' {
                    break;
                }
                delim.push(ch);
            }
        } else {
            while let Some(ch) = self.peek() {
                if ch.is_whitespace() || ch == '\n' {
                    break;
                }
                delim.push(ch);
                self.advance();
            }
        }

        let _ = quoted; // quoted here-docs suppress expansion (handled in expand phase)
        delim
    }

    fn read_word(&mut self) -> String {
        let mut word = String::new();

        loop {
            match self.peek() {
                None => break,
                Some(ch) => match ch {
                    // Word terminators
                    ' ' | '\t' | '\n' | '|' | '&' | ';' | '(' | ')' => break,
                    '<' | '>' if word.is_empty() || !word.ends_with('$') => break,

                    // Single quotes
                    '\'' => {
                        word.push('\'');
                        self.advance();
                        loop {
                            match self.advance() {
                                Some('\'') => {
                                    word.push('\'');
                                    break;
                                }
                                Some(c) => word.push(c),
                                None => break,
                            }
                        }
                    }

                    // Double quotes
                    '"' => {
                        word.push('"');
                        self.advance();
                        loop {
                            match self.advance() {
                                Some('"') => {
                                    word.push('"');
                                    break;
                                }
                                Some('\\') => {
                                    word.push('\\');
                                    if let Some(c) = self.advance() {
                                        word.push(c);
                                    }
                                }
                                Some('$') => {
                                    word.push('$');
                                    self.read_dollar_into(&mut word);
                                }
                                Some('`') => {
                                    word.push('`');
                                    self.read_backtick_into(&mut word);
                                }
                                Some(c) => word.push(c),
                                None => break,
                            }
                        }
                    }

                    // Backslash escape
                    '\\' => {
                        self.advance();
                        if let Some(c) = self.advance() {
                            if c != '\n' {
                                word.push('\\');
                                word.push(c);
                            }
                            // else: line continuation, skip both
                        }
                    }

                    // Dollar expansion
                    '$' => {
                        word.push('$');
                        self.advance();
                        self.read_dollar_into(&mut word);
                    }

                    // Backtick command substitution
                    '`' => {
                        word.push('`');
                        self.advance();
                        self.read_backtick_into(&mut word);
                    }

                    // Regular character
                    _ => {
                        word.push(ch);
                        self.advance();
                    }
                },
            }
        }

        word
    }

    fn read_dollar_into(&mut self, word: &mut String) {
        match self.peek() {
            Some('(') => {
                self.advance();
                if self.peek() == Some('(') {
                    // $(( arithmetic ))
                    self.advance();
                    word.push('(');
                    word.push('(');
                    let mut depth = 2;
                    while depth > 0 {
                        match self.advance() {
                            Some('(') => {
                                depth += 1;
                                word.push('(');
                            }
                            Some(')') => {
                                depth -= 1;
                                word.push(')');
                            }
                            Some(c) => word.push(c),
                            None => break,
                        }
                    }
                } else {
                    // $( command substitution )
                    word.push('(');
                    let mut depth = 1;
                    while depth > 0 {
                        match self.advance() {
                            Some('(') => {
                                depth += 1;
                                word.push('(');
                            }
                            Some(')') => {
                                depth -= 1;
                                if depth > 0 {
                                    word.push(')');
                                }
                            }
                            Some('\'') => {
                                word.push('\'');
                                loop {
                                    match self.advance() {
                                        Some('\'') => {
                                            word.push('\'');
                                            break;
                                        }
                                        Some(c) => word.push(c),
                                        None => break,
                                    }
                                }
                            }
                            Some('"') => {
                                word.push('"');
                                loop {
                                    match self.advance() {
                                        Some('"') => {
                                            word.push('"');
                                            break;
                                        }
                                        Some('\\') => {
                                            word.push('\\');
                                            if let Some(c) = self.advance() {
                                                word.push(c);
                                            }
                                        }
                                        Some(c) => word.push(c),
                                        None => break,
                                    }
                                }
                            }
                            Some(c) => word.push(c),
                            None => break,
                        }
                    }
                    word.push(')');
                }
            }
            Some('{') => {
                self.advance();
                word.push('{');
                let mut depth = 1;
                while depth > 0 {
                    match self.advance() {
                        Some('{') => {
                            depth += 1;
                            word.push('{');
                        }
                        Some('}') => {
                            depth -= 1;
                            if depth > 0 {
                                word.push('}');
                            }
                        }
                        Some(c) => word.push(c),
                        None => break,
                    }
                }
                word.push('}');
            }
            Some(c) if c.is_alphanumeric() || c == '_' || c == '?' || c == '#' || c == '@' || c == '*' || c == '$' || c == '!' => {
                // Simple variable like $VAR, $?, $#, etc.
                if c == '?' || c == '#' || c == '@' || c == '*' || c == '$' || c == '!' {
                    word.push(c);
                    self.advance();
                } else {
                    while let Some(c) = self.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            word.push(c);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                }
            }
            _ => {
                // Lone $ — treat as literal
            }
        }
    }

    fn read_backtick_into(&mut self, word: &mut String) {
        loop {
            match self.advance() {
                Some('`') => {
                    word.push('`');
                    break;
                }
                Some('\\') => {
                    if let Some(c) = self.advance() {
                        if c != '`' && c != '\\' && c != '$' {
                            word.push('\\');
                        }
                        word.push(c);
                    }
                }
                Some(c) => word.push(c),
                None => break,
            }
        }
    }

    /// Get a slice of the internal chars buffer.
    pub fn chars_slice(&self) -> &[char] {
        &self.chars
    }

    /// Peek at the next token without consuming it.
    pub fn peek_token(&mut self) -> Token {
        let saved_pos = self.pos;
        let saved_keyword_ok = self.keyword_ok;
        let tok = self.next_token();
        self.pos = saved_pos;
        self.keyword_ok = saved_keyword_ok;
        tok
    }
}
