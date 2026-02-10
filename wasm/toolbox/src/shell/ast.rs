/// AST nodes for the shell interpreter.

#[derive(Debug, Clone)]
pub struct Program {
    pub commands: Vec<CompleteCommand>,
}

/// A complete command is a list of pipelines connected by && or ||.
#[derive(Debug, Clone)]
pub struct CompleteCommand {
    pub first: Pipeline,
    pub rest: Vec<(ListOp, Pipeline)>,
    pub background: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ListOp {
    And, // &&
    Or,  // ||
}

/// A pipeline is a sequence of simple commands connected by |.
#[derive(Debug, Clone)]
pub struct Pipeline {
    pub commands: Vec<Command>,
    pub negated: bool,
}

/// A single command — could be simple, compound, or a function definition.
#[derive(Debug, Clone)]
pub enum Command {
    Simple(SimpleCommand),
    If(IfClause),
    For(ForClause),
    While(WhileClause),
    Until(UntilClause),
    Case(CaseClause),
    Subshell(Program),
    BraceGroup(Program),
    FuncDef(FuncDef),
}

/// A simple command: optional assignments, words, and redirections.
#[derive(Debug, Clone)]
pub struct SimpleCommand {
    pub assignments: Vec<Assignment>,
    pub words: Vec<Word>,
    pub redirections: Vec<Redirect>,
}

#[derive(Debug, Clone)]
pub struct Assignment {
    pub name: String,
    pub value: Word,
}

/// A word is a sequence of parts that get concatenated after expansion.
#[derive(Debug, Clone)]
pub struct Word {
    pub parts: Vec<WordPart>,
}

impl Word {
    pub fn literal(s: &str) -> Self {
        Word {
            parts: vec![WordPart::Literal(s.to_string())],
        }
    }

    pub fn is_empty(&self) -> bool {
        self.parts.is_empty()
    }
}

#[derive(Debug, Clone)]
pub enum WordPart {
    Literal(String),
    SingleQuoted(String),
    DoubleQuoted(Vec<WordPart>),
    Variable(String),                    // $VAR or ${VAR}
    VarDefault(String, Word),            // ${VAR:-default}
    VarAssignDefault(String, Word),      // ${VAR:=default}
    VarLength(String),                   // ${#VAR}
    CommandSub(String),                  // $(cmd) or `cmd`
    ArithmeticSub(String),              // $((expr))
    Glob(String),                        // *, ?, [...]
    SpecialVar(SpecialVar),
}

#[derive(Debug, Clone)]
pub enum SpecialVar {
    ExitStatus,     // $?
    NumArgs,        // $#
    AllArgs,        // $@
    AllArgsStar,    // $*
    ProcessId,      // $$
    Positional(u32), // $0, $1, ...
}

#[derive(Debug, Clone)]
pub struct Redirect {
    pub fd: Option<i32>,
    pub kind: RedirectKind,
    pub target: Word,
}

#[derive(Debug, Clone)]
pub enum RedirectKind {
    Output,         // >
    Append,         // >>
    Input,          // <
    HereDoc(String), // <<EOF ... EOF (stores the content)
    HereString,     // <<<
    DupOutput,      // >&
    DupInput,       // <&
}

#[derive(Debug, Clone)]
pub struct IfClause {
    pub condition: Program,
    pub then_body: Program,
    pub elifs: Vec<(Program, Program)>,
    pub else_body: Option<Program>,
}

#[derive(Debug, Clone)]
pub struct ForClause {
    pub var: String,
    pub words: Option<Vec<Word>>,
    pub body: Program,
}

#[derive(Debug, Clone)]
pub struct WhileClause {
    pub condition: Program,
    pub body: Program,
}

#[derive(Debug, Clone)]
pub struct UntilClause {
    pub condition: Program,
    pub body: Program,
}

#[derive(Debug, Clone)]
pub struct CaseClause {
    pub word: Word,
    pub arms: Vec<CaseArm>,
}

#[derive(Debug, Clone)]
pub struct CaseArm {
    pub patterns: Vec<Word>,
    pub body: Program,
}

#[derive(Debug, Clone)]
pub struct FuncDef {
    pub name: String,
    pub body: Box<Command>,
}
