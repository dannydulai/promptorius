//! Token types for the Promptorius language.

#[derive(Debug, Clone, PartialEq)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Spanned {
    pub token: Token,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Literals
    Number(f64),
    String(String),
    /// Backtick interpolation string — alternating literal parts and expression parts.
    /// e.g. `hello {name}!` → Interp(["hello ", "!"], [expr_tokens_for_name])
    /// Stored as raw parts; the parser handles the expression tokens.
    InterpStart,          // opening backtick
    InterpLiteral(String), // literal text between expressions
    InterpExprStart,      // `{` inside backtick
    InterpExprEnd,        // `}` inside backtick
    InterpEnd,            // closing backtick
    True,
    False,
    Null,

    // Identifiers and keywords
    Ident(String),
    Fn,
    If,
    Else,
    While,
    For,
    In,
    Return,

    // Operators
    Plus,        // +
    Minus,       // -
    Star,        // *
    Slash,       // /
    Percent,     // %
    Eq,          // ==
    NotEq,       // !=
    StrictEq,    // ===
    StrictNotEq, // !==
    Lt,          // <
    Gt,          // >
    LtEq,       // <=
    GtEq,       // >=
    And,         // &&
    Or,          // ||
    Not,         // !
    NullCoalesce, // ??
    Question,    // ?
    DotDot,      // ..

    // Assignment
    Assign,      // =
    PlusAssign,  // +=
    MinusAssign, // -=
    StarAssign,  // *=
    SlashAssign, // /=
    PercentAssign, // %=

    // Delimiters
    LParen,      // (
    RParen,      // )
    LBrace,      // {
    RBrace,      // }
    LBracket,    // [
    RBracket,    // ]

    // Punctuation
    Dot,         // .
    Comma,       // ,
    Colon,       // :
    Semicolon,   // ;

    // Special
    Newline,
    Eof,
}

impl Token {
    /// Returns true if this token can end a statement (for optional semicolon / ASI).
    pub fn can_end_stmt(&self) -> bool {
        matches!(
            self,
            Token::Ident(_)
                | Token::Number(_)
                | Token::String(_)
                | Token::InterpEnd
                | Token::True
                | Token::False
                | Token::Null
                | Token::RParen
                | Token::RBracket
                | Token::RBrace
                | Token::Return
        )
    }

}
