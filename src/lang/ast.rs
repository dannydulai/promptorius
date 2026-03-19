//! AST node types for the Promptorius language.

#![allow(dead_code)]
use crate::lang::token::Span;

/// A complete program (top-level statements).
#[derive(Debug, Clone)]
pub struct Program {
    pub stmts: Vec<Stmt>,
}

/// A statement.
#[derive(Debug, Clone)]
pub enum Stmt {
    /// Expression statement (including bare assignments).
    Expr(Expr),

    /// if (cond) { ... } else if (cond) { ... } else { ... }
    If {
        condition: Expr,
        then_body: Vec<Stmt>,
        else_ifs: Vec<(Expr, Vec<Stmt>)>,
        else_body: Option<Vec<Stmt>>,
        span: Span,
    },

    /// while (cond) { ... }
    While {
        condition: Expr,
        body: Vec<Stmt>,
        span: Span,
    },

    /// for (item in expr) { ... }
    ForIn {
        var: String,
        iter: Expr,
        body: Vec<Stmt>,
        span: Span,
    },

    /// return expr
    Return {
        value: Option<Expr>,
        span: Span,
    },

    /// fn name(params) { ... }
    FnDef {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
        span: Span,
    },
}

/// An expression.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Literal value: number, string, bool, null.
    Literal(Literal, Span),

    /// Variable reference.
    Ident(String, Span),

    /// Binary operation: left op right.
    BinaryOp {
        op: BinOp,
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },

    /// Unary operation: op expr.
    UnaryOp {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },

    /// Ternary: cond ? then : else.
    Ternary {
        condition: Box<Expr>,
        then_expr: Box<Expr>,
        else_expr: Box<Expr>,
        span: Span,
    },

    /// Function/method call: callee(args).
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },

    /// Member access: obj.field.
    Member {
        object: Box<Expr>,
        field: String,
        span: Span,
    },

    /// Index access: obj[index].
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
        span: Span,
    },

    /// Assignment: target = value.
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },

    /// Compound assignment: target op= value.
    CompoundAssign {
        op: BinOp,
        target: Box<Expr>,
        value: Box<Expr>,
        span: Span,
    },

    /// Array literal: [a, b, c].
    Array(Vec<Expr>, Span),

    /// Dict literal: { key: value, ... }.
    Dict(Vec<(String, Expr)>, Span),

    /// Backtick interpolation string: `hello {expr} world`.
    Interpolation(Vec<InterpPart>, Span),

    /// Anonymous function / closure: fn(params) { body }.
    Closure {
        params: Vec<String>,
        body: Vec<Stmt>,
        span: Span,
    },

    /// Range expression: start..end.
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
        span: Span,
    },

    /// Null coalescing: left ?? right.
    NullCoalesce {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
}

/// Part of an interpolated string.
#[derive(Debug, Clone)]
pub enum InterpPart {
    Literal(String),
    Expr(Expr),
}

/// Literal values.
#[derive(Debug, Clone)]
pub enum Literal {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
}

/// Binary operators.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    NotEq,
    StrictEq,
    StrictNotEq,
    Lt,
    Gt,
    LtEq,
    GtEq,
    And,
    Or,
}

/// Unary operators.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Not,
    Neg,
}
