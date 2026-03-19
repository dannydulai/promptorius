//! Statement code generation: AST statements → Rust code.

use crate::codegen::expr::gen_expr;
use crate::codegen::mangle_ident;
use crate::lang::ast::*;

/// Generate Rust code for a statement.
pub fn gen_stmt(stmt: &Stmt) -> String {
    match stmt {
        Stmt::Expr(expr) => {
            format!("{{ {}; }}", gen_expr(expr))
        }

        Stmt::If {
            condition,
            then_body,
            else_ifs,
            else_body,
            ..
        } => {
            let mut s = format!("if ({}).to_bool() {{\n", gen_expr(condition));
            for stmt in then_body {
                s.push_str("    ");
                s.push_str(&gen_stmt(stmt));
                s.push('\n');
            }
            s.push('}');

            for (cond, body) in else_ifs {
                s.push_str(&format!(" else if ({}).to_bool() {{\n", gen_expr(cond)));
                for stmt in body {
                    s.push_str("    ");
                    s.push_str(&gen_stmt(stmt));
                    s.push('\n');
                }
                s.push('}');
            }

            if let Some(body) = else_body {
                s.push_str(" else {\n");
                for stmt in body {
                    s.push_str("    ");
                    s.push_str(&gen_stmt(stmt));
                    s.push('\n');
                }
                s.push('}');
            }

            s
        }

        Stmt::While { condition, body, .. } => {
            let mut s = format!("while ({}).to_bool() {{\n", gen_expr(condition));
            for stmt in body {
                s.push_str("    ");
                s.push_str(&gen_stmt(stmt));
                s.push('\n');
            }
            s.push('}');
            s
        }

        Stmt::ForIn { var, iter, body, .. } => {
            let iter_code = gen_expr(iter);
            let mut s = format!(
                "for __item in ({iter_code}).to_array().iter() {{\n"
            );
            s.push_str(&format!(
                "    scope.set(\"{var}\", __item.clone());\n"
            ));
            for stmt in body {
                s.push_str("    ");
                s.push_str(&gen_stmt(stmt));
                s.push('\n');
            }
            s.push('}');
            s
        }

        Stmt::Return { value, .. } => match value {
            Some(expr) => format!("return {};", gen_expr(expr)),
            None => "return Value::Null;".to_string(),
        },

        Stmt::FnDef { .. } => {
            // Top-level fn defs are handled by the main generate() function.
            // Nested fn defs inside blocks become closures stored in scope.
            if let Stmt::FnDef {
                name, params, body, ..
            } = stmt
            {
                let closure = gen_local_fn(params, body);
                format!("scope.set(\"{name}\", {closure});")
            } else {
                String::new()
            }
        }
    }
}

/// Generate a closure for a function defined inside another function.
fn gen_local_fn(params: &[String], body: &[Stmt]) -> String {
    let param_extracts: Vec<String> = params
        .iter()
        .enumerate()
        .map(|(i, p)| {
            format!(
                "let {} = __args.get({i}).cloned().unwrap_or(Value::Null);",
                mangle_ident(p)
            )
        })
        .collect();

    let body_code: Vec<String> = body.iter().map(gen_stmt).collect();

    format!(
        "Value::Closure(Arc::new(move |__args: Vec<Value>| -> Value {{ {} {} Value::Null }}))",
        param_extracts.join(" "),
        body_code.join(" "),
    )
}
