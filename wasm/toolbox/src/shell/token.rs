/// Token types for the shell lexer.

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Word(String),           // A word (may contain quotes, expansions, etc.)
    Pipe,                   // |
    And,                    // &&
    Or,                     // ||
    Semi,                   // ;
    Newline,                // \n
    Amp,                    // &
    LParen,                 // (
    RParen,                 // )

    // Redirections
    Less,                   // <
    Great,                  // >
    DGreat,                 // >>
    LessAnd,               // <&
    GreatAnd,               // >&
    DLess,                  // << (here-doc)
    DLessDash,              // <<- (here-doc strip tabs)
    TLess,                  // <<< (here-string)

    // Numbered redirections
    IoNumber(i32),          // A number before < or >

    // Keywords
    If,
    Then,
    Elif,
    Else,
    Fi,
    For,
    While,
    Until,
    Do,
    Done,
    Case,
    Esac,
    In,
    Function,
    Select,
    LBrace,                 // {
    RBrace,                 // }
    Bang,                   // !

    Eof,
}

impl Token {
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            Token::If
                | Token::Then
                | Token::Elif
                | Token::Else
                | Token::Fi
                | Token::For
                | Token::While
                | Token::Until
                | Token::Do
                | Token::Done
                | Token::Case
                | Token::Esac
                | Token::In
                | Token::Function
                | Token::LBrace
                | Token::RBrace
                | Token::Bang
        )
    }

    /// Checks if this token can start a command.
    pub fn is_command_start(&self) -> bool {
        matches!(
            self,
            Token::Word(_)
                | Token::If
                | Token::For
                | Token::While
                | Token::Until
                | Token::Case
                | Token::LParen
                | Token::LBrace
                | Token::Function
                | Token::Bang
        )
    }
}

/// Convert a keyword string to its token, or return None if it's not a keyword.
pub fn keyword_token(s: &str) -> Option<Token> {
    match s {
        "if" => Some(Token::If),
        "then" => Some(Token::Then),
        "elif" => Some(Token::Elif),
        "else" => Some(Token::Else),
        "fi" => Some(Token::Fi),
        "for" => Some(Token::For),
        "while" => Some(Token::While),
        "until" => Some(Token::Until),
        "do" => Some(Token::Do),
        "done" => Some(Token::Done),
        "case" => Some(Token::Case),
        "esac" => Some(Token::Esac),
        "in" => Some(Token::In),
        "function" => Some(Token::Function),
        "select" => Some(Token::Select),
        "!" => Some(Token::Bang),
        "{" => Some(Token::LBrace),
        "}" => Some(Token::RBrace),
        _ => None,
    }
}
