//! Recursive descent parser for the Promptorius language.

use crate::lang::ast::*;
use crate::lang::lexer::{self, Lexer};
use crate::lang::token::{Span, Spanned, Token};
use thiserror::Error;

#[derive(Error, Debug)]
#[error("{msg} at line {line}, column {col}")]
pub struct ParseError {
    pub msg: String,
    pub line: usize,
    pub col: usize,
}

pub struct Parser {
    tokens: Vec<Spanned>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Spanned>) -> Self {
        Self { tokens, pos: 0 }
    }

    pub fn parse(source: &str) -> Result<Program, ParseError> {
        let tokens = Lexer::tokenize(source).map_err(|e| ParseError {
            msg: e.msg,
            line: e.line,
            col: e.col,
        })?;
        let mut parser = Parser::new(tokens);
        parser.parse_program()
    }

    fn span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|t| t.span.clone())
            .unwrap_or(Span { line: 0, col: 0 })
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|t| &t.token)
            .unwrap_or(&Token::Eof)
    }

    fn at(&self, tok: &Token) -> bool {
        self.peek() == tok
    }

    fn advance(&mut self) -> &Token {
        let tok = self.tokens.get(self.pos).map(|t| &t.token).unwrap_or(&Token::Eof);
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        tok
    }

    fn expect(&mut self, expected: &Token) -> Result<(), ParseError> {
        if self.peek() == expected {
            self.advance();
            Ok(())
        } else {
            Err(ParseError {
                msg: format!("expected {expected:?}, got {:?}", self.peek()),
                line: self.span().line,
                col: self.span().col,
            })
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Token::Newline | Token::Semicolon) {
            self.advance();
        }
    }

    fn at_stmt_end(&self) -> bool {
        matches!(
            self.peek(),
            Token::Newline | Token::Semicolon | Token::Eof | Token::RBrace
        )
    }

    fn consume_stmt_end(&mut self) {
        if matches!(self.peek(), Token::Newline | Token::Semicolon) {
            self.advance();
        }
    }

    // --- Program ---

    fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut stmts = Vec::new();
        self.skip_newlines();
        while !self.at(&Token::Eof) {
            stmts.push(self.parse_statement()?);
            self.skip_newlines();
        }
        Ok(Program { stmts })
    }

    // --- Statements ---

    fn parse_statement(&mut self) -> Result<Stmt, ParseError> {
        self.skip_newlines();
        match self.peek().clone() {
            Token::Fn => {
                // fn name(...) = named function def
                // fn(...) = closure expression
                // Check if next non-newline token is ( or an identifier
                let mut lookahead = self.pos + 1;
                while lookahead < self.tokens.len()
                    && matches!(self.tokens[lookahead].token, Token::Newline)
                {
                    lookahead += 1;
                }
                let next = self.tokens.get(lookahead).map(|t| &t.token);
                if matches!(next, Some(Token::LParen)) {
                    // Closure expression
                    let expr = self.parse_expression()?;
                    self.consume_stmt_end();
                    Ok(Stmt::Expr(expr))
                } else {
                    self.parse_fn_def()
                }
            }
            Token::If => self.parse_if(),
            Token::While => self.parse_while(),
            Token::For => self.parse_for(),
            Token::Return => self.parse_return(),
            _ => {
                let expr = self.parse_expression()?;
                self.consume_stmt_end();
                Ok(Stmt::Expr(expr))
            }
        }
    }

    fn parse_fn_def(&mut self) -> Result<Stmt, ParseError> {
        let span = self.span();
        self.advance(); // consume 'fn'

        // Named function: fn name(params) { body }
        // Anonymous: fn(params) { body } — handled in parse_primary as closure
        let name = match self.peek().clone() {
            Token::Ident(name) => {
                self.advance();
                name
            }
            _ => {
                return Err(ParseError {
                    msg: "expected function name".to_string(),
                    line: span.line,
                    col: span.col,
                });
            }
        };

        self.expect(&Token::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&Token::RParen)?;

        self.skip_newlines();
        self.expect(&Token::LBrace)?;
        let body = self.parse_block_body()?;
        self.expect(&Token::RBrace)?;
        self.consume_stmt_end();

        Ok(Stmt::FnDef {
            name,
            params,
            body,
            span,
        })
    }

    fn parse_param_list(&mut self) -> Result<Vec<String>, ParseError> {
        let mut params = Vec::new();
        while let Token::Ident(name) = self.peek().clone() {
            params.push(name);
            self.advance();
            if self.at(&Token::Comma) {
                self.advance();
            } else {
                break;
            }
        }
        Ok(params)
    }

    fn parse_block_body(&mut self) -> Result<Vec<Stmt>, ParseError> {
        let mut stmts = Vec::new();
        self.skip_newlines();
        while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
            stmts.push(self.parse_statement()?);
            self.skip_newlines();
        }
        Ok(stmts)
    }

    fn parse_if(&mut self) -> Result<Stmt, ParseError> {
        let span = self.span();
        self.advance(); // consume 'if'

        self.expect(&Token::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(&Token::RParen)?;

        self.skip_newlines();
        self.expect(&Token::LBrace)?;
        let then_body = self.parse_block_body()?;
        self.expect(&Token::RBrace)?;

        let mut else_ifs = Vec::new();
        let mut else_body = None;

        loop {
            self.skip_newlines();
            if !self.at(&Token::Else) {
                break;
            }
            self.advance(); // consume 'else'

            if self.at(&Token::If) {
                self.advance(); // consume 'if'
                self.expect(&Token::LParen)?;
                let cond = self.parse_expression()?;
                self.expect(&Token::RParen)?;

                self.skip_newlines();
                self.expect(&Token::LBrace)?;
                let body = self.parse_block_body()?;
                self.expect(&Token::RBrace)?;
                else_ifs.push((cond, body));
            } else {
                self.skip_newlines();
                self.expect(&Token::LBrace)?;
                let body = self.parse_block_body()?;
                self.expect(&Token::RBrace)?;
                else_body = Some(body);
                break;
            }
        }

        self.consume_stmt_end();
        Ok(Stmt::If {
            condition,
            then_body,
            else_ifs,
            else_body,
            span,
        })
    }

    fn parse_while(&mut self) -> Result<Stmt, ParseError> {
        let span = self.span();
        self.advance(); // consume 'while'

        self.expect(&Token::LParen)?;
        let condition = self.parse_expression()?;
        self.expect(&Token::RParen)?;

        self.skip_newlines();
        self.expect(&Token::LBrace)?;
        let body = self.parse_block_body()?;
        self.expect(&Token::RBrace)?;
        self.consume_stmt_end();

        Ok(Stmt::While {
            condition,
            body,
            span,
        })
    }

    fn parse_for(&mut self) -> Result<Stmt, ParseError> {
        let span = self.span();
        self.advance(); // consume 'for'

        self.expect(&Token::LParen)?;
        let var = match self.peek().clone() {
            Token::Ident(name) => {
                self.advance();
                name
            }
            _ => {
                return Err(ParseError {
                    msg: "expected variable name in for loop".to_string(),
                    line: self.span().line,
                    col: self.span().col,
                });
            }
        };
        self.expect(&Token::In)?;
        let iter = self.parse_expression()?;
        self.expect(&Token::RParen)?;

        self.skip_newlines();
        self.expect(&Token::LBrace)?;
        let body = self.parse_block_body()?;
        self.expect(&Token::RBrace)?;
        self.consume_stmt_end();

        Ok(Stmt::ForIn {
            var,
            iter,
            body,
            span,
        })
    }

    fn parse_return(&mut self) -> Result<Stmt, ParseError> {
        let span = self.span();
        self.advance(); // consume 'return'

        let value = if self.at_stmt_end() {
            None
        } else {
            Some(self.parse_expression()?)
        };
        self.consume_stmt_end();

        Ok(Stmt::Return { value, span })
    }

    // --- Expressions (precedence climbing) ---

    fn parse_expression(&mut self) -> Result<Expr, ParseError> {
        self.parse_assignment()
    }

    fn parse_assignment(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_ternary()?;

        match self.peek().clone() {
            Token::Assign => {
                let span = self.span();
                self.advance();
                let value = self.parse_assignment()?; // right-associative
                Ok(Expr::Assign {
                    target: Box::new(expr),
                    value: Box::new(value),
                    span,
                })
            }
            Token::PlusAssign => self.parse_compound_assign(expr, BinOp::Add),
            Token::MinusAssign => self.parse_compound_assign(expr, BinOp::Sub),
            Token::StarAssign => self.parse_compound_assign(expr, BinOp::Mul),
            Token::SlashAssign => self.parse_compound_assign(expr, BinOp::Div),
            Token::PercentAssign => self.parse_compound_assign(expr, BinOp::Mod),
            _ => Ok(expr),
        }
    }

    fn parse_compound_assign(&mut self, target: Expr, op: BinOp) -> Result<Expr, ParseError> {
        let span = self.span();
        self.advance();
        let value = self.parse_assignment()?;
        Ok(Expr::CompoundAssign {
            op,
            target: Box::new(target),
            value: Box::new(value),
            span,
        })
    }

    fn parse_ternary(&mut self) -> Result<Expr, ParseError> {
        let expr = self.parse_null_coalesce()?;

        if self.at(&Token::Question) {
            let span = self.span();
            self.advance(); // consume ?
            let then_expr = self.parse_assignment()?;
            self.expect(&Token::Colon)?;
            let else_expr = self.parse_assignment()?;
            Ok(Expr::Ternary {
                condition: Box::new(expr),
                then_expr: Box::new(then_expr),
                else_expr: Box::new(else_expr),
                span,
            })
        } else {
            Ok(expr)
        }
    }

    fn parse_null_coalesce(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_or()?;

        while self.at(&Token::NullCoalesce) {
            let span = self.span();
            self.advance();
            let right = self.parse_or()?;
            expr = Expr::NullCoalesce {
                left: Box::new(expr),
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_and()?;

        while self.at(&Token::Or) {
            let span = self.span();
            self.advance();
            let right = self.parse_and()?;
            expr = Expr::BinaryOp {
                op: BinOp::Or,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_equality()?;

        while self.at(&Token::And) {
            let span = self.span();
            self.advance();
            let right = self.parse_equality()?;
            expr = Expr::BinaryOp {
                op: BinOp::And,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_comparison()?;

        loop {
            let op = match self.peek() {
                Token::Eq => BinOp::Eq,
                Token::NotEq => BinOp::NotEq,
                Token::StrictEq => BinOp::StrictEq,
                Token::StrictNotEq => BinOp::StrictNotEq,
                _ => break,
            };
            let span = self.span();
            self.advance();
            let right = self.parse_comparison()?;
            expr = Expr::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_addition()?;

        loop {
            let op = match self.peek() {
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::LtEq => BinOp::LtEq,
                Token::GtEq => BinOp::GtEq,
                _ => break,
            };
            let span = self.span();
            self.advance();
            let right = self.parse_addition()?;
            expr = Expr::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_addition(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_multiplication()?;

        loop {
            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                _ => break,
            };
            let span = self.span();
            self.advance();
            let right = self.parse_multiplication()?;
            expr = Expr::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_multiplication(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_unary()?;

        loop {
            let op = match self.peek() {
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                _ => break,
            };
            let span = self.span();
            self.advance();
            let right = self.parse_unary()?;
            expr = Expr::BinaryOp {
                op,
                left: Box::new(expr),
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        match self.peek().clone() {
            Token::Not => {
                let span = self.span();
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Not,
                    operand: Box::new(operand),
                    span,
                })
            }
            Token::Minus => {
                let span = self.span();
                self.advance();
                let operand = self.parse_unary()?;
                Ok(Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    operand: Box::new(operand),
                    span,
                })
            }
            _ => self.parse_postfix(),
        }
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            match self.peek().clone() {
                Token::Dot => {
                    let span = self.span();
                    self.advance();

                    // Check for range: expr..expr
                    if self.at(&Token::Dot) {
                        self.advance();
                        let end = self.parse_unary()?;
                        // Oops — DotDot should have been caught by the lexer.
                        // Actually `expr.` then `.` means we consumed Dot then see Dot.
                        // But the lexer emits DotDot for `..`, so this case is
                        // `expr` `.` `field` — normal member access.
                        // This shouldn't happen. Let me just handle member access.
                        return Err(ParseError {
                            msg: "unexpected '.'".to_string(),
                            line: span.line,
                            col: span.col,
                        });
                    }

                    let field = match self.peek().clone() {
                        Token::Ident(name) => {
                            self.advance();
                            name
                        }
                        _ => {
                            return Err(ParseError {
                                msg: "expected field name after '.'".to_string(),
                                line: self.span().line,
                                col: self.span().col,
                            });
                        }
                    };

                    // Check if followed by ( — method call
                    if self.at(&Token::LParen) {
                        self.advance();
                        let args = self.parse_arg_list()?;
                        self.expect(&Token::RParen)?;
                        expr = Expr::Call {
                            callee: Box::new(Expr::Member {
                                object: Box::new(expr),
                                field,
                                span: span.clone(),
                            }),
                            args,
                            span,
                        };
                    } else {
                        expr = Expr::Member {
                            object: Box::new(expr),
                            field,
                            span,
                        };
                    }
                }
                Token::LBracket => {
                    let span = self.span();
                    self.advance();
                    let index = self.parse_expression()?;
                    self.expect(&Token::RBracket)?;
                    expr = Expr::Index {
                        object: Box::new(expr),
                        index: Box::new(index),
                        span,
                    };
                }
                Token::LParen => {
                    let span = self.span();
                    self.advance();
                    let args = self.parse_arg_list()?;
                    self.expect(&Token::RParen)?;
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                        span,
                    };
                }
                Token::DotDot => {
                    let span = self.span();
                    self.advance();
                    let end = self.parse_unary()?;
                    expr = Expr::Range {
                        start: Box::new(expr),
                        end: Box::new(end),
                        span,
                    };
                }
                _ => break,
            }
        }

        Ok(expr)
    }

    fn parse_arg_list(&mut self) -> Result<Vec<Expr>, ParseError> {
        let mut args = Vec::new();
        self.skip_newlines();
        if self.at(&Token::RParen) {
            return Ok(args);
        }
        args.push(self.parse_expression()?);
        while self.at(&Token::Comma) {
            self.advance();
            self.skip_newlines();
            if self.at(&Token::RParen) {
                break; // trailing comma
            }
            args.push(self.parse_expression()?);
        }
        self.skip_newlines();
        Ok(args)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let span = self.span();

        match self.peek().clone() {
            Token::Number(n) => {
                self.advance();
                Ok(Expr::Literal(Literal::Number(n), span))
            }
            Token::String(s) => {
                self.advance();
                if lexer::is_interpolation(&s) {
                    self.parse_interpolation_from_encoded(&s, span)
                } else {
                    Ok(Expr::Literal(Literal::String(s), span))
                }
            }
            Token::True => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(true), span))
            }
            Token::False => {
                self.advance();
                Ok(Expr::Literal(Literal::Bool(false), span))
            }
            Token::Null => {
                self.advance();
                Ok(Expr::Literal(Literal::Null, span))
            }
            Token::Ident(name) => {
                self.advance();
                Ok(Expr::Ident(name, span))
            }
            Token::LParen => {
                self.advance();
                let expr = self.parse_expression()?;
                self.expect(&Token::RParen)?;
                Ok(expr)
            }
            Token::LBracket => self.parse_array_literal(),
            Token::LBrace => self.parse_dict_literal(),
            Token::Fn => self.parse_closure(),
            _ => Err(ParseError {
                msg: format!("unexpected token: {:?}", self.peek()),
                line: span.line,
                col: span.col,
            }),
        }
    }

    fn parse_interpolation_from_encoded(
        &mut self,
        encoded: &str,
        span: Span,
    ) -> Result<Expr, ParseError> {
        let mut interp_parts = Vec::new();
        let s = &encoded[1..]; // skip \x01 marker
        for chunk in s.split('\x00') {
            if chunk.is_empty() {
                continue;
            }
            let tag = chunk.as_bytes()[0];
            let content = &chunk[1..];
            match tag {
                b'L' => {
                    interp_parts.push(InterpPart::Literal(content.to_string()));
                }
                b'E' => {
                    // Parse the expression source
                    let expr = Parser::parse_expression_string(content).map_err(|e| {
                        ParseError {
                            msg: format!("in interpolation: {}", e.msg),
                            line: span.line,
                            col: span.col,
                        }
                    })?;
                    interp_parts.push(InterpPart::Expr(expr));
                }
                _ => {}
            }
        }

        Ok(Expr::Interpolation(interp_parts, span))
    }

    /// Parse a standalone expression from a source string (used for interpolation).
    fn parse_expression_string(source: &str) -> Result<Expr, ParseError> {
        let tokens = Lexer::tokenize(source).map_err(|e| ParseError {
            msg: e.msg,
            line: e.line,
            col: e.col,
        })?;
        let mut parser = Parser::new(tokens);
        parser.parse_expression()
    }

    fn parse_array_literal(&mut self) -> Result<Expr, ParseError> {
        let span = self.span();
        self.advance(); // consume [
        let mut elements = Vec::new();
        self.skip_newlines();
        while !self.at(&Token::RBracket) && !self.at(&Token::Eof) {
            elements.push(self.parse_expression()?);
            self.skip_newlines();
            if self.at(&Token::Comma) {
                self.advance();
                self.skip_newlines();
            } else {
                break;
            }
        }
        self.expect(&Token::RBracket)?;
        Ok(Expr::Array(elements, span))
    }

    fn parse_dict_literal(&mut self) -> Result<Expr, ParseError> {
        let span = self.span();
        self.advance(); // consume {
        let mut entries = Vec::new();
        self.skip_newlines();
        while !self.at(&Token::RBrace) && !self.at(&Token::Eof) {
            let key = match self.peek().clone() {
                Token::Ident(name) => {
                    self.advance();
                    name
                }
                Token::String(s) => {
                    self.advance();
                    s
                }
                _ => {
                    return Err(ParseError {
                        msg: format!("expected dict key, got {:?}", self.peek()),
                        line: self.span().line,
                        col: self.span().col,
                    });
                }
            };
            self.expect(&Token::Colon)?;
            let value = self.parse_expression()?;
            entries.push((key, value));
            self.skip_newlines();
            if self.at(&Token::Comma) {
                self.advance();
                self.skip_newlines();
            } else {
                break;
            }
        }
        self.skip_newlines();
        self.expect(&Token::RBrace)?;
        Ok(Expr::Dict(entries, span))
    }

    fn parse_closure(&mut self) -> Result<Expr, ParseError> {
        let span = self.span();
        self.advance(); // consume 'fn'

        self.expect(&Token::LParen)?;
        let params = self.parse_param_list()?;
        self.expect(&Token::RParen)?;

        self.skip_newlines();
        self.expect(&Token::LBrace)?;
        let body = self.parse_block_body()?;
        self.expect(&Token::RBrace)?;

        Ok(Expr::Closure {
            params,
            body,
            span,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str) -> Program {
        Parser::parse(src).expect(&format!("failed to parse: {src}"))
    }

    fn parse_expr(src: &str) -> Expr {
        let prog = parse(src);
        assert_eq!(prog.stmts.len(), 1, "expected 1 statement, got {}", prog.stmts.len());
        match &prog.stmts[0] {
            Stmt::Expr(e) => e.clone(),
            other => panic!("expected Expr statement, got {other:?}"),
        }
    }

    #[test]
    fn parse_number() {
        let e = parse_expr("42");
        assert!(matches!(e, Expr::Literal(Literal::Number(n), _) if n == 42.0));
    }

    #[test]
    fn parse_string() {
        let e = parse_expr(r#""hello""#);
        assert!(matches!(e, Expr::Literal(Literal::String(s), _) if s == "hello"));
    }

    #[test]
    fn parse_bool() {
        assert!(matches!(parse_expr("true"), Expr::Literal(Literal::Bool(true), _)));
        assert!(matches!(parse_expr("false"), Expr::Literal(Literal::Bool(false), _)));
    }

    #[test]
    fn parse_null() {
        assert!(matches!(parse_expr("null"), Expr::Literal(Literal::Null, _)));
    }

    #[test]
    fn parse_binary_add() {
        let e = parse_expr("1 + 2");
        assert!(matches!(e, Expr::BinaryOp { op: BinOp::Add, .. }));
    }

    #[test]
    fn parse_precedence() {
        // 1 + 2 * 3 should parse as 1 + (2 * 3)
        let e = parse_expr("1 + 2 * 3");
        match e {
            Expr::BinaryOp { op: BinOp::Add, right, .. } => {
                assert!(matches!(*right, Expr::BinaryOp { op: BinOp::Mul, .. }));
            }
            _ => panic!("expected Add at top"),
        }
    }

    #[test]
    fn parse_ternary() {
        let e = parse_expr("x ? 1 : 2");
        assert!(matches!(e, Expr::Ternary { .. }));
    }

    #[test]
    fn parse_null_coalesce() {
        let e = parse_expr("x ?? 5");
        assert!(matches!(e, Expr::NullCoalesce { .. }));
    }

    #[test]
    fn parse_unary_not() {
        let e = parse_expr("!x");
        assert!(matches!(e, Expr::UnaryOp { op: UnaryOp::Not, .. }));
    }

    #[test]
    fn parse_unary_neg() {
        let e = parse_expr("-5");
        assert!(matches!(e, Expr::UnaryOp { op: UnaryOp::Neg, .. }));
    }

    #[test]
    fn parse_assignment() {
        let e = parse_expr("x = 5");
        assert!(matches!(e, Expr::Assign { .. }));
    }

    #[test]
    fn parse_compound_assign() {
        let e = parse_expr("x += 1");
        assert!(matches!(e, Expr::CompoundAssign { op: BinOp::Add, .. }));
    }

    #[test]
    fn parse_function_call() {
        let e = parse_expr("foo(1, 2)");
        assert!(matches!(e, Expr::Call { .. }));
    }

    #[test]
    fn parse_method_call() {
        let e = parse_expr("s.len()");
        match e {
            Expr::Call { callee, args, .. } => {
                assert!(matches!(*callee, Expr::Member { .. }));
                assert!(args.is_empty());
            }
            _ => panic!("expected Call"),
        }
    }

    #[test]
    fn parse_member_access() {
        let e = parse_expr("obj.field");
        assert!(matches!(e, Expr::Member { field, .. } if field == "field"));
    }

    #[test]
    fn parse_index_access() {
        let e = parse_expr("arr[0]");
        assert!(matches!(e, Expr::Index { .. }));
    }

    #[test]
    fn parse_array_literal() {
        let e = parse_expr("[1, 2, 3]");
        match e {
            Expr::Array(elems, _) => assert_eq!(elems.len(), 3),
            _ => panic!("expected Array"),
        }
    }

    #[test]
    fn parse_dict_literal() {
        let e = parse_expr("{ name: \"danny\", age: 30 }");
        match e {
            Expr::Dict(entries, _) => assert_eq!(entries.len(), 2),
            _ => panic!("expected Dict"),
        }
    }

    #[test]
    fn parse_closure() {
        let e = parse_expr("fn(x) { x * 2 }");
        assert!(matches!(e, Expr::Closure { .. }));
    }

    #[test]
    fn parse_range() {
        let e = parse_expr("0..10");
        assert!(matches!(e, Expr::Range { .. }));
    }

    #[test]
    fn parse_chained_member_call() {
        let e = parse_expr("git.branch()");
        match e {
            Expr::Call { callee, .. } => {
                assert!(matches!(*callee, Expr::Member { field, .. } if field == "branch"));
            }
            _ => panic!("expected Call on Member"),
        }
    }

    #[test]
    fn parse_fn_def() {
        let prog = parse("fn foo(a, b) { return a + b }");
        assert!(matches!(&prog.stmts[0], Stmt::FnDef { name, params, .. } if name == "foo" && params.len() == 2));
    }

    #[test]
    fn parse_if_stmt() {
        let prog = parse("if (x > 0) { y = 1 }");
        assert!(matches!(&prog.stmts[0], Stmt::If { .. }));
    }

    #[test]
    fn parse_if_else() {
        let prog = parse("if (x) { 1 } else { 2 }");
        match &prog.stmts[0] {
            Stmt::If { else_body, .. } => assert!(else_body.is_some()),
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn parse_if_else_if() {
        let prog = parse("if (a) { 1 } else if (b) { 2 } else { 3 }");
        match &prog.stmts[0] {
            Stmt::If { else_ifs, else_body, .. } => {
                assert_eq!(else_ifs.len(), 1);
                assert!(else_body.is_some());
            }
            _ => panic!("expected If"),
        }
    }

    #[test]
    fn parse_while_stmt() {
        let prog = parse("while (x > 0) { x -= 1 }");
        assert!(matches!(&prog.stmts[0], Stmt::While { .. }));
    }

    #[test]
    fn parse_for_in() {
        let prog = parse("for (item in arr) { eprint(item) }");
        assert!(matches!(&prog.stmts[0], Stmt::ForIn { var, .. } if var == "item"));
    }

    #[test]
    fn parse_for_range() {
        let prog = parse("for (i in 0..10) { eprint(i) }");
        match &prog.stmts[0] {
            Stmt::ForIn { iter, .. } => {
                assert!(matches!(iter, Expr::Range { .. }));
            }
            _ => panic!("expected ForIn"),
        }
    }

    #[test]
    fn parse_return() {
        let prog = parse("fn f() { return 42 }");
        match &prog.stmts[0] {
            Stmt::FnDef { body, .. } => {
                assert!(matches!(&body[0], Stmt::Return { value: Some(_), .. }));
            }
            _ => panic!("expected FnDef"),
        }
    }

    #[test]
    fn parse_default_config_script() {
        let src = r##"
colors = {
    directory: "#6ec2e8",
    error: { fg: "red", bold: true },
    char_normal: "#666",
}
setcolors(colors)

fn left_prompt() {
    result = ""
    if (exit_code != 0) {
        result += color("error") + "Exited w/ " + exit_code + color("") + "\n"
    }
    result += color("directory") + cwd().replace(env("HOME"), "~") + color("")
    char = env("USER") == "root" ? "#" : "│"
    col = keymap === "vicmd" ? "char_vicmd" : "char_normal"
    result += " " + color(col) + char + color("") + " "
    return result
}

fn right_prompt() {
    if (!git.is_repo()) { return "" }
    return color("git_branch") + " " + git.branch() + color("")
}
"##;
        let prog = parse(src);
        // Should have: colors assignment, setcolors call, left_prompt fn, right_prompt fn
        assert!(prog.stmts.len() >= 4, "expected at least 4 top-level stmts, got {}", prog.stmts.len());
    }
}
