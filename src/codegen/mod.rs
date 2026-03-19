pub mod expr;
pub mod runtime;
pub mod stmt;

use crate::lang::ast::*;

/// Generate a complete Rust source file from a parsed program.
pub fn generate(program: &Program) -> String {
    let mut output = String::new();

    let mut fn_defs = Vec::new();
    let mut top_stmts = Vec::new();

    for s in &program.stmts {
        match s {
            Stmt::FnDef { .. } => fn_defs.push(s),
            _ => top_stmts.push(s),
        }
    }

    // Emit runtime
    output.push_str(runtime::RUNTIME);
    output.push('\n');

    // Emit user functions
    for f in &fn_defs {
        if let Stmt::FnDef {
            name, params, body, ..
        } = f
        {
            output.push_str(&gen_fn_def(name, params, body));
            output.push('\n');
        }
    }

    // Emit script_init (top-level statements + register user functions in scope)
    output.push_str("fn script_init(scope: &mut Scope) {\n");

    // Register user-defined functions as closures in scope
    for f in &fn_defs {
        if let Stmt::FnDef { name, params, .. } = f {
            let n = params.len();
            let mangled = mangle_ident(name);
            output.push_str(&format!(
                "    scope.set(\"{name}\", Value::Closure(Arc::new(move |__args: Vec<Value>| -> Value {{ \
                    let mut __scope = Scope::new(); \
                    user_fn_{mangled}(&mut __scope, {arg_unpack}) \
                }})));\n",
                arg_unpack = (0..n)
                    .map(|i| format!("__args.get({i}).cloned().unwrap_or(Value::Null)"))
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        }
    }

    for s in &top_stmts {
        output.push_str("    ");
        output.push_str(&stmt::gen_stmt(s));
        output.push('\n');
    }
    output.push_str("}\n\n");

    // Emit main
    output.push_str(runtime::MAIN_FN);

    output
}

/// Generate an instrumented version for `promptorius explain`.
/// Wraps script_init, left_prompt, right_prompt with timing.
pub fn generate_instrumented(program: &Program) -> String {
    let mut output = String::new();

    let mut fn_defs = Vec::new();
    let mut top_stmts = Vec::new();

    for s in &program.stmts {
        match s {
            Stmt::FnDef { .. } => fn_defs.push(s),
            _ => top_stmts.push(s),
        }
    }

    // Emit runtime
    output.push_str(runtime::RUNTIME);
    output.push('\n');

    // Emit user functions (same as normal)
    for f in &fn_defs {
        if let Stmt::FnDef {
            name, params, body, ..
        } = f
        {
            output.push_str(&gen_fn_def(name, params, body));
            output.push('\n');
        }
    }

    // Emit script_init (same as normal)
    output.push_str("fn script_init(scope: &mut Scope) {\n");
    for f in &fn_defs {
        if let Stmt::FnDef { name, params, .. } = f {
            let n = params.len();
            let mangled = mangle_ident(name);
            output.push_str(&format!(
                "    scope.set(\"{name}\", Value::Closure(Arc::new(move |__args: Vec<Value>| -> Value {{ \
                    let mut __scope = Scope::new(); \
                    user_fn_{mangled}(&mut __scope, {arg_unpack}) \
                }})));\n",
                arg_unpack = (0..n)
                    .map(|i| format!("__args.get({i}).cloned().unwrap_or(Value::Null)"))
                    .collect::<Vec<_>>()
                    .join(", "),
            ));
        }
    }
    for s in &top_stmts {
        output.push_str("    ");
        output.push_str(&stmt::gen_stmt(s));
        output.push('\n');
    }
    output.push_str("}\n\n");

    // Emit instrumented main
    output.push_str(runtime::EXPLAIN_MAIN_FN);

    output
}

/// Generate a Rust function from a script function definition.
fn gen_fn_def(name: &str, params: &[String], body: &[Stmt]) -> String {
    let param_list: Vec<String> = params
        .iter()
        .map(|p| format!("{}: Value", mangle_ident(p)))
        .collect();

    let mut s = format!(
        "fn user_fn_{}(scope: &mut Scope, {}) -> Value {{\n",
        mangle_ident(name),
        param_list.join(", ")
    );

    // Push params into a child scope
    s.push_str("    let mut scope = scope.child();\n");
    for p in params {
        s.push_str(&format!(
            "    scope.set(\"{p}\", {}.clone());\n",
            mangle_ident(p)
        ));
    }

    for stmt in body {
        s.push_str("    ");
        s.push_str(&stmt::gen_stmt(stmt));
        s.push('\n');
    }

    // Only append Value::Null if the last statement isn't a return
    let ends_with_return = body.last().map(|s| matches!(s, Stmt::Return { .. })).unwrap_or(false);
    if !ends_with_return {
        s.push_str("    Value::Null\n");
    }
    s.push_str("}\n");
    s
}

/// Mangle an identifier to be a valid Rust identifier.
pub fn mangle_ident(name: &str) -> String {
    // Prefix with _ if it's a Rust keyword
    let rust_keywords = [
        "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn",
        "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
        "return", "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe",
        "use", "where", "while", "async", "await", "dyn",
    ];
    let mangled = name.replace('-', "_");
    if rust_keywords.contains(&mangled.as_str()) {
        format!("r#{mangled}")
    } else {
        mangled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::parser::Parser;

    #[test]
    fn codegen_minimal_script() {
        let src = r#"
fn left_prompt() {
    return "hello"
}
fn right_prompt() {
    return ""
}
"#;
        let program = Parser::parse(src).unwrap();
        let output = generate(&program);

        assert!(output.contains("fn user_fn_left_prompt"));
        assert!(output.contains("fn user_fn_right_prompt"));
        assert!(output.contains("fn main()"));
        assert!(output.contains("fn script_init"));
        assert!(output.contains("enum Value"));
    }

    #[test]
    fn codegen_with_globals_and_builtins() {
        let src = r##"
colors = {
    error: { fg: "red", bold: true },
}
setcolors(colors)

fn left_prompt() {
    if (exit_code != 0) {
        return color("error") + "err" + color("")
    }
    return cwd()
}
fn right_prompt() {
    if (!git.is_repo()) { return "" }
    return git.branch()
}
"##;
        let program = Parser::parse(src).unwrap();
        let output = generate(&program);

        assert!(output.contains("builtin_setcolors"));
        assert!(output.contains("builtin_color"));
        assert!(output.contains("builtin_cwd"));
        assert!(output.contains("builtin_git_is_repo"));
        assert!(output.contains("builtin_git_branch"));
    }
}
