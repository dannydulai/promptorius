//! Lexer/tokenizer for the Promptorius language.

use crate::lang::token::{Span, Spanned, Token};
use thiserror::Error;

#[derive(Error, Debug)]
#[error("{msg} at line {line}, column {col}")]
pub struct LexError {
    pub msg: String,
    pub line: usize,
    pub col: usize,
}

pub struct Lexer {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    /// The last significant (non-newline, non-whitespace) token emitted.
    /// Used for regex vs division disambiguation.
    prev_token: Option<Token>,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            prev_token: None,
        }
    }

    pub fn tokenize(source: &str) -> Result<Vec<Spanned>, LexError> {
        let mut lexer = Lexer::new(source);
        let mut tokens = Vec::new();
        loop {
            let tok = lexer.next_token()?;
            let is_eof = tok.token == Token::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn span(&self) -> Span {
        Span {
            line: self.line,
            col: self.col,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn advance(&mut self) -> Option<char> {
        let ch = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        Some(ch)
    }

    fn skip_whitespace_and_comments(&mut self) -> Option<Spanned> {
        let mut saw_newline = false;
        let mut newline_span = self.span();

        while let Some(ch) = self.peek() {
            if ch == '#' {
                // Comment — skip to end of line
                while let Some(c) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.advance();
                }
            } else if ch == '\n' {
                if !saw_newline {
                    newline_span = self.span();
                }
                saw_newline = true;
                self.advance();
            } else if ch.is_ascii_whitespace() {
                self.advance();
            } else {
                break;
            }
        }

        if saw_newline {
            Some(Spanned {
                token: Token::Newline,
                span: newline_span,
            })
        } else {
            None
        }
    }

    pub fn next_token(&mut self) -> Result<Spanned, LexError> {
        // Skip whitespace/comments, collecting newlines
        let newline = self.skip_whitespace_and_comments();

        // If we saw a newline and the previous token can end a statement, emit it
        if let Some(nl) = newline {
            if let Some(ref prev) = self.prev_token {
                if prev.can_end_stmt() {
                    return Ok(nl);
                }
            }
            // Otherwise discard the newline and continue
        }

        let Some(ch) = self.peek() else {
            return Ok(Spanned {
                token: Token::Eof,
                span: self.span(),
            });
        };

        let spanned = match ch {
            '0'..='9' => self.lex_number()?,
            '"' | '\'' => self.lex_string()?,
            '`' => self.lex_backtick()?,
            '/' => {
                // Regex or division?
                let is_div = self
                    .prev_token
                    .as_ref()
                    .map(|t| t.is_division_context())
                    .unwrap_or(false);
                if is_div {
                    self.lex_operator()?
                } else {
                    self.lex_regex()?
                }
            }
            'a'..='z' | 'A'..='Z' | '_' => self.lex_ident_or_keyword()?,
            '+' | '-' | '*' | '%' | '=' | '!' | '<' | '>' | '&' | '|' | '?' | '.' => {
                self.lex_operator()?
            }
            '(' => {
                let span = self.span();
                self.advance();
                Spanned {
                    token: Token::LParen,
                    span,
                }
            }
            ')' => {
                let span = self.span();
                self.advance();
                Spanned {
                    token: Token::RParen,
                    span,
                }
            }
            '{' => {
                let span = self.span();
                self.advance();
                Spanned {
                    token: Token::LBrace,
                    span,
                }
            }
            '}' => {
                let span = self.span();
                self.advance();
                Spanned {
                    token: Token::RBrace,
                    span,
                }
            }
            '[' => {
                let span = self.span();
                self.advance();
                Spanned {
                    token: Token::LBracket,
                    span,
                }
            }
            ']' => {
                let span = self.span();
                self.advance();
                Spanned {
                    token: Token::RBracket,
                    span,
                }
            }
            ',' => {
                let span = self.span();
                self.advance();
                Spanned {
                    token: Token::Comma,
                    span,
                }
            }
            ':' => {
                let span = self.span();
                self.advance();
                Spanned {
                    token: Token::Colon,
                    span,
                }
            }
            ';' => {
                let span = self.span();
                self.advance();
                Spanned {
                    token: Token::Semicolon,
                    span,
                }
            }
            _ => {
                return Err(LexError {
                    msg: format!("unexpected character: '{ch}'"),
                    line: self.line,
                    col: self.col,
                });
            }
        };

        // Track previous significant token
        if !matches!(spanned.token, Token::Newline) {
            self.prev_token = Some(spanned.token.clone());
        }

        Ok(spanned)
    }

    fn lex_number(&mut self) -> Result<Spanned, LexError> {
        let span = self.span();
        let start = self.pos;
        let mut has_dot = false;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_digit() {
                self.advance();
            } else if ch == '.' && !has_dot && self.peek_at(1).map(|c| c.is_ascii_digit()).unwrap_or(false) {
                has_dot = true;
                self.advance();
            } else {
                break;
            }
        }

        let text: String = self.chars[start..self.pos].iter().collect();
        let value: f64 = text.parse().map_err(|_| LexError {
            msg: format!("invalid number: {text}"),
            line: span.line,
            col: span.col,
        })?;

        Ok(Spanned {
            token: Token::Number(value),
            span,
        })
    }

    fn lex_string(&mut self) -> Result<Spanned, LexError> {
        let span = self.span();
        let quote = self.advance().unwrap(); // consume opening quote
        let mut s = String::new();

        loop {
            match self.advance() {
                None => {
                    return Err(LexError {
                        msg: "unterminated string".to_string(),
                        line: span.line,
                        col: span.col,
                    });
                }
                Some(c) if c == quote => break,
                Some('\\') => {
                    s.push(self.lex_escape()?);
                }
                Some(c) => s.push(c),
            }
        }

        Ok(Spanned {
            token: Token::String(s),
            span,
        })
    }

    fn lex_escape(&mut self) -> Result<char, LexError> {
        match self.advance() {
            Some('n') => Ok('\n'),
            Some('t') => Ok('\t'),
            Some('r') => Ok('\r'),
            Some('\\') => Ok('\\'),
            Some('\'') => Ok('\''),
            Some('"') => Ok('"'),
            Some('`') => Ok('`'),
            Some('{') => Ok('{'),
            Some('}') => Ok('}'),
            Some('0') => Ok('\0'),
            Some('u') => self.lex_unicode_escape(),
            Some(c) => Err(LexError {
                msg: format!("invalid escape: \\{c}"),
                line: self.line,
                col: self.col,
            }),
            None => Err(LexError {
                msg: "unterminated escape".to_string(),
                line: self.line,
                col: self.col,
            }),
        }
    }

    fn lex_unicode_escape(&mut self) -> Result<char, LexError> {
        // Expect \u{XXXX}
        if self.peek() != Some('{') {
            return Err(LexError {
                msg: "expected '{' after \\u".to_string(),
                line: self.line,
                col: self.col,
            });
        }
        self.advance(); // consume {

        let mut hex = String::new();
        loop {
            match self.peek() {
                Some('}') => {
                    self.advance();
                    break;
                }
                Some(c) if c.is_ascii_hexdigit() => {
                    hex.push(c);
                    self.advance();
                }
                _ => {
                    return Err(LexError {
                        msg: "invalid unicode escape".to_string(),
                        line: self.line,
                        col: self.col,
                    });
                }
            }
        }

        let code = u32::from_str_radix(&hex, 16).map_err(|_| LexError {
            msg: format!("invalid unicode codepoint: {hex}"),
            line: self.line,
            col: self.col,
        })?;

        char::from_u32(code).ok_or(LexError {
            msg: format!("invalid unicode codepoint: {hex}"),
            line: self.line,
            col: self.col,
        })
    }

    fn lex_backtick(&mut self) -> Result<Spanned, LexError> {
        let span = self.span();
        self.advance(); // consume opening `

        let mut parts = Vec::new();
        let mut current = String::new();

        loop {
            match self.peek() {
                None => {
                    return Err(LexError {
                        msg: "unterminated backtick string".to_string(),
                        line: span.line,
                        col: span.col,
                    });
                }
                Some('`') => {
                    self.advance();
                    if !current.is_empty() {
                        parts.push(InterpLexPart::Literal(current));
                    }
                    break;
                }
                Some('{') => {
                    if self.peek_at(1) == Some('{') {
                        // Escaped brace
                        self.advance();
                        self.advance();
                        current.push('{');
                    } else {
                        // Expression start
                        self.advance(); // consume {
                        if !current.is_empty() {
                            parts.push(InterpLexPart::Literal(std::mem::take(&mut current)));
                        }
                        // Collect tokens until matching }
                        let expr_source = self.collect_interp_expr()?;
                        parts.push(InterpLexPart::Expr(expr_source));
                    }
                }
                Some('}') => {
                    if self.peek_at(1) == Some('}') {
                        self.advance();
                        self.advance();
                        current.push('}');
                    } else {
                        // Stray } — treat as literal
                        self.advance();
                        current.push('}');
                    }
                }
                Some('\\') => {
                    self.advance();
                    current.push(self.lex_escape()?);
                }
                Some(c) => {
                    self.advance();
                    current.push(c);
                }
            }
        }

        // Convert to a single String token if no interpolation, otherwise store the raw
        // parts for the parser to handle.
        if parts.len() == 1 {
            if let InterpLexPart::Literal(s) = &parts[0] {
                return Ok(Spanned {
                    token: Token::String(s.clone()),
                    span,
                });
            }
        }
        if parts.is_empty() {
            return Ok(Spanned {
                token: Token::String(String::new()),
                span,
            });
        }

        // For now, encode interpolation parts into the String token with markers.
        // The parser will re-lex expression parts.
        // We use a special token for this.
        Ok(Spanned {
            token: Token::String(encode_interp_parts(&parts)),
            span,
        })
    }

    /// Collect source text for an interpolation expression inside backtick string.
    /// Handles nested braces.
    fn collect_interp_expr(&mut self) -> Result<String, LexError> {
        let mut depth = 1;
        let mut expr = String::new();

        loop {
            match self.peek() {
                None => {
                    return Err(LexError {
                        msg: "unterminated interpolation expression".to_string(),
                        line: self.line,
                        col: self.col,
                    });
                }
                Some('{') => {
                    depth += 1;
                    expr.push(self.advance().unwrap());
                }
                Some('}') => {
                    depth -= 1;
                    if depth == 0 {
                        self.advance(); // consume closing }
                        break;
                    }
                    expr.push(self.advance().unwrap());
                }
                Some('"') | Some('\'') => {
                    // String inside expression — consume it whole
                    let q = self.advance().unwrap();
                    expr.push(q);
                    loop {
                        match self.advance() {
                            None => {
                                return Err(LexError {
                                    msg: "unterminated string in interpolation".to_string(),
                                    line: self.line,
                                    col: self.col,
                                });
                            }
                            Some('\\') => {
                                expr.push('\\');
                                if let Some(c) = self.advance() {
                                    expr.push(c);
                                }
                            }
                            Some(c) if c == q => {
                                expr.push(c);
                                break;
                            }
                            Some(c) => expr.push(c),
                        }
                    }
                }
                Some(c) => {
                    expr.push(self.advance().unwrap());
                }
            }
        }

        Ok(expr)
    }

    fn lex_regex(&mut self) -> Result<Spanned, LexError> {
        let span = self.span();
        self.advance(); // consume opening /

        let mut pattern = String::new();
        let mut in_class = false; // inside [...] character class

        loop {
            match self.advance() {
                None => {
                    return Err(LexError {
                        msg: "unterminated regex".to_string(),
                        line: span.line,
                        col: span.col,
                    });
                }
                Some('\\') => {
                    pattern.push('\\');
                    if let Some(c) = self.advance() {
                        pattern.push(c);
                    }
                }
                Some('[') => {
                    in_class = true;
                    pattern.push('[');
                }
                Some(']') => {
                    in_class = false;
                    pattern.push(']');
                }
                Some('/') if !in_class => break,
                Some(c) => pattern.push(c),
            }
        }

        // Collect flags
        let mut flags = String::new();
        while let Some(ch) = self.peek() {
            if matches!(ch, 'i' | 'g' | 'm') {
                flags.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        Ok(Spanned {
            token: Token::Regex(pattern, flags),
            span,
        })
    }

    fn lex_ident_or_keyword(&mut self) -> Result<Spanned, LexError> {
        let span = self.span();
        let start = self.pos;

        while let Some(ch) = self.peek() {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let text: String = self.chars[start..self.pos].iter().collect();
        let token = match text.as_str() {
            "fn" => Token::Fn,
            "if" => Token::If,
            "else" => Token::Else,
            "while" => Token::While,
            "for" => Token::For,
            "in" => Token::In,
            "return" => Token::Return,
            "true" => Token::True,
            "false" => Token::False,
            "null" => Token::Null,
            _ => Token::Ident(text),
        };

        Ok(Spanned { token, span })
    }

    fn lex_operator(&mut self) -> Result<Spanned, LexError> {
        let span = self.span();
        let ch = self.advance().unwrap();

        let token = match ch {
            '+' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::PlusAssign
                } else {
                    Token::Plus
                }
            }
            '-' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::MinusAssign
                } else {
                    Token::Minus
                }
            }
            '*' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::StarAssign
                } else {
                    Token::Star
                }
            }
            '/' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::SlashAssign
                } else {
                    Token::Slash
                }
            }
            '%' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::PercentAssign
                } else {
                    Token::Percent
                }
            }
            '=' => {
                if self.peek() == Some('=') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::StrictEq
                    } else {
                        Token::Eq
                    }
                } else {
                    Token::Assign
                }
            }
            '!' => {
                if self.peek() == Some('=') {
                    self.advance();
                    if self.peek() == Some('=') {
                        self.advance();
                        Token::StrictNotEq
                    } else {
                        Token::NotEq
                    }
                } else {
                    Token::Not
                }
            }
            '<' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::LtEq
                } else {
                    Token::Lt
                }
            }
            '>' => {
                if self.peek() == Some('=') {
                    self.advance();
                    Token::GtEq
                } else {
                    Token::Gt
                }
            }
            '&' => {
                if self.peek() == Some('&') {
                    self.advance();
                    Token::And
                } else {
                    return Err(LexError {
                        msg: "expected '&&'".to_string(),
                        line: span.line,
                        col: span.col,
                    });
                }
            }
            '|' => {
                if self.peek() == Some('|') {
                    self.advance();
                    Token::Or
                } else {
                    return Err(LexError {
                        msg: "expected '||'".to_string(),
                        line: span.line,
                        col: span.col,
                    });
                }
            }
            '?' => {
                if self.peek() == Some('?') {
                    self.advance();
                    Token::NullCoalesce
                } else {
                    Token::Question
                }
            }
            '.' => {
                if self.peek() == Some('.') {
                    self.advance();
                    Token::DotDot
                } else {
                    Token::Dot
                }
            }
            _ => unreachable!(),
        };

        Ok(Spanned { token, span })
    }
}

/// Internal representation of backtick interpolation parts during lexing.
#[derive(Debug)]
enum InterpLexPart {
    Literal(String),
    Expr(String),
}

/// Encode interpolation parts into a string with special markers.
/// The parser will detect the marker prefix and split accordingly.
/// Format: \x00L<literal>\x00E<expr>\x00L<literal>...
fn encode_interp_parts(parts: &[InterpLexPart]) -> String {
    let mut result = String::new();
    result.push('\x01'); // marker: this is an interpolation string
    for part in parts {
        match part {
            InterpLexPart::Literal(s) => {
                result.push('L');
                result.push_str(s);
                result.push('\x00');
            }
            InterpLexPart::Expr(s) => {
                result.push('E');
                result.push_str(s);
                result.push('\x00');
            }
        }
    }
    result
}

/// Check if a string token is actually an encoded interpolation.
pub fn is_interpolation(s: &str) -> bool {
    s.starts_with('\x01')
}

/// Decode interpolation parts from an encoded string.
pub fn decode_interp_parts(s: &str) -> Vec<InterpLexPart> {
    let s = &s[1..]; // skip marker
    let mut parts = Vec::new();
    for chunk in s.split('\x00') {
        if chunk.is_empty() {
            continue;
        }
        let tag = chunk.as_bytes()[0];
        let content = &chunk[1..];
        match tag {
            b'L' => parts.push(InterpLexPart::Literal(content.to_string())),
            b'E' => parts.push(InterpLexPart::Expr(content.to_string())),
            _ => {}
        }
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(src: &str) -> Vec<Token> {
        Lexer::tokenize(src)
            .unwrap()
            .into_iter()
            .map(|s| s.token)
            .filter(|t| !matches!(t, Token::Newline | Token::Eof))
            .collect()
    }

    #[test]
    fn lex_number_int() {
        assert_eq!(tokens("42"), vec![Token::Number(42.0)]);
    }

    #[test]
    fn lex_number_float() {
        assert_eq!(tokens("3.14"), vec![Token::Number(3.14)]);
    }

    #[test]
    fn lex_string_double() {
        assert_eq!(tokens(r#""hello""#), vec![Token::String("hello".to_string())]);
    }

    #[test]
    fn lex_string_single() {
        assert_eq!(tokens("'world'"), vec![Token::String("world".to_string())]);
    }

    #[test]
    fn lex_string_escapes() {
        assert_eq!(
            tokens(r#""hello\nworld""#),
            vec![Token::String("hello\nworld".to_string())]
        );
    }

    #[test]
    fn lex_string_unicode() {
        assert_eq!(
            tokens(r#""\u{1F600}""#),
            vec![Token::String("😀".to_string())]
        );
    }

    #[test]
    fn lex_backtick_no_interp() {
        assert_eq!(tokens("`hello`"), vec![Token::String("hello".to_string())]);
    }

    #[test]
    fn lex_backtick_with_interp() {
        let toks = tokens("`hello {name}`");
        assert_eq!(toks.len(), 1);
        if let Token::String(s) = &toks[0] {
            assert!(is_interpolation(s));
        } else {
            panic!("expected interpolation string");
        }
    }

    #[test]
    fn lex_backtick_escaped_braces() {
        assert_eq!(
            tokens("`{{literal}}`"),
            vec![Token::String("{literal}".to_string())]
        );
    }

    #[test]
    fn lex_regex() {
        assert_eq!(
            tokens("/^hello/i"),
            vec![Token::Regex("^hello".to_string(), "i".to_string())]
        );
    }

    #[test]
    fn lex_regex_vs_division() {
        // After a number, / is division
        let toks = tokens("5 / 2");
        assert_eq!(
            toks,
            vec![Token::Number(5.0), Token::Slash, Token::Number(2.0)]
        );
    }

    #[test]
    fn lex_keywords() {
        let toks = tokens("fn if else while for in return true false null");
        assert_eq!(
            toks,
            vec![
                Token::Fn,
                Token::If,
                Token::Else,
                Token::While,
                Token::For,
                Token::In,
                Token::Return,
                Token::True,
                Token::False,
                Token::Null,
            ]
        );
    }

    #[test]
    fn lex_arithmetic_operators() {
        let toks = tokens("a + b - c * d");
        assert!(toks.contains(&Token::Plus));
        assert!(toks.contains(&Token::Minus));
        assert!(toks.contains(&Token::Star));
    }

    #[test]
    fn lex_division() {
        let toks = tokens("a / b");
        assert_eq!(
            toks,
            vec![
                Token::Ident("a".to_string()),
                Token::Slash,
                Token::Ident("b".to_string()),
            ]
        );
    }

    #[test]
    fn lex_comparison_operators() {
        let toks = tokens("a == b != c === d !== e < f > g <= h >= i");
        assert!(toks.contains(&Token::Eq));
        assert!(toks.contains(&Token::NotEq));
        assert!(toks.contains(&Token::StrictEq));
        assert!(toks.contains(&Token::StrictNotEq));
        assert!(toks.contains(&Token::Lt));
        assert!(toks.contains(&Token::Gt));
        assert!(toks.contains(&Token::LtEq));
        assert!(toks.contains(&Token::GtEq));
    }

    #[test]
    fn lex_logical_operators() {
        let toks = tokens("a && b || !c");
        assert!(toks.contains(&Token::And));
        assert!(toks.contains(&Token::Or));
        assert!(toks.contains(&Token::Not));
    }

    #[test]
    fn lex_null_coalesce_and_ternary() {
        let toks = tokens("a ?? b ? c : d");
        assert!(toks.contains(&Token::NullCoalesce));
        assert!(toks.contains(&Token::Question));
    }

    #[test]
    fn lex_range() {
        let toks = tokens("0..10");
        assert_eq!(
            toks,
            vec![Token::Number(0.0), Token::DotDot, Token::Number(10.0)]
        );
    }

    #[test]
    fn lex_assignment_operators() {
        let toks = tokens("a += 1; b -= 2; c *= 3; d /= 4; e %= 5");
        assert!(toks.contains(&Token::PlusAssign));
        assert!(toks.contains(&Token::MinusAssign));
        assert!(toks.contains(&Token::StarAssign));
        assert!(toks.contains(&Token::SlashAssign));
        assert!(toks.contains(&Token::PercentAssign));
    }

    #[test]
    fn lex_delimiters() {
        let toks = tokens("( ) { } [ ] , : ;");
        assert_eq!(
            toks,
            vec![
                Token::LParen,
                Token::RParen,
                Token::LBrace,
                Token::RBrace,
                Token::LBracket,
                Token::RBracket,
                Token::Comma,
                Token::Colon,
                Token::Semicolon,
            ]
        );
    }

    #[test]
    fn lex_comment() {
        let toks = tokens("x # this is a comment\ny");
        assert_eq!(
            toks,
            vec![Token::Ident("x".to_string()), Token::Ident("y".to_string())]
        );
    }

    #[test]
    fn lex_newline_as_stmt_terminator() {
        let all = Lexer::tokenize("x = 5\ny = 10")
            .unwrap()
            .into_iter()
            .map(|s| s.token)
            .filter(|t| !matches!(t, Token::Eof))
            .collect::<Vec<_>>();
        // After `5` (a number, can end stmt), newline should appear
        assert!(all.contains(&Token::Newline));
    }

    #[test]
    fn lex_simple_program() {
        let src = r#"
fn left_prompt() {
    return "hello"
}
"#;
        let toks = tokens(src);
        assert_eq!(
            toks,
            vec![
                Token::Fn,
                Token::Ident("left_prompt".to_string()),
                Token::LParen,
                Token::RParen,
                Token::LBrace,
                Token::Return,
                Token::String("hello".to_string()),
                Token::RBrace,
            ]
        );
    }
}
