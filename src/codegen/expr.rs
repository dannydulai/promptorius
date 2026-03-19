//! Expression code generation: AST expressions → Rust code producing Value.

use crate::codegen::mangle_ident;
use crate::lang::ast::*;

/// Generate Rust code for an expression that produces a Value.
pub fn gen_expr(expr: &Expr) -> String {
    match expr {
        Expr::Literal(lit, _) => gen_literal(lit),

        Expr::Ident(name, _) => {
            format!("scope.get(\"{}\")", name)
        }

        Expr::BinaryOp { op, left, right, .. } => {
            let l = gen_expr(left);
            let r = gen_expr(right);
            let func = match op {
                BinOp::Add => "value_add",
                BinOp::Sub => "value_sub",
                BinOp::Mul => "value_mul",
                BinOp::Div => "value_div",
                BinOp::Mod => "value_mod",
                BinOp::Eq => "value_eq",
                BinOp::NotEq => "value_neq",
                BinOp::StrictEq => "value_strict_eq",
                BinOp::StrictNotEq => "value_strict_neq",
                BinOp::Lt => "value_lt",
                BinOp::Gt => "value_gt",
                BinOp::LtEq => "value_lte",
                BinOp::GtEq => "value_gte",
                BinOp::And => {
                    return format!(
                        "if ({l}).to_bool() {{ {r} }} else {{ Value::Bool(false) }}"
                    );
                }
                BinOp::Or => {
                    return format!(
                        "{{ let __or_l = {l}; if __or_l.to_bool() {{ __or_l }} else {{ {r} }} }}"
                    );
                }
            };
            format!("{func}(&{l}, &{r})")
        }

        Expr::UnaryOp { op, operand, .. } => {
            let e = gen_expr(operand);
            match op {
                UnaryOp::Not => format!("Value::Bool(!({e}).to_bool())"),
                UnaryOp::Neg => format!("value_neg(&{e})"),
            }
        }

        Expr::Ternary { condition, then_expr, else_expr, .. } => {
            let c = gen_expr(condition);
            let t = gen_expr(then_expr);
            let e = gen_expr(else_expr);
            format!("if ({c}).to_bool() {{ {t} }} else {{ {e} }}")
        }

        Expr::NullCoalesce { left, right, .. } => {
            let l = gen_expr(left);
            let r = gen_expr(right);
            format!("{{ let __nc = {l}; if __nc.is_null() {{ {r} }} else {{ __nc }} }}")
        }

        Expr::Call { callee, args, .. } => {
            gen_call(callee, args)
        }

        Expr::Member { object, field, .. } => {
            // Check for namespaced builtins: git.field, file.field, etc.
            if let Expr::Ident(ns, _) = object.as_ref() {
                match ns.as_str() {
                    "git" | "file" | "dir" | "battery" => {
                        // This is a namespace member access — will be a call, not a value
                        return format!("value_member_access(&{}, \"{}\")", gen_expr(object), field);
                    }
                    _ => {}
                }
            }
            format!("value_member_access(&{}, \"{}\")", gen_expr(object), field)
        }

        Expr::Index { object, index, .. } => {
            format!("value_index(&{}, &{})", gen_expr(object), gen_expr(index))
        }

        Expr::Assign { target, value, .. } => {
            gen_assignment(target, value)
        }

        Expr::CompoundAssign { op, target, value, .. } => {
            gen_compound_assignment(op, target, value)
        }

        Expr::Array(elements, _) => {
            let elems: Vec<String> = elements.iter().map(gen_expr).collect();
            format!("Value::Array(vec![{}])", elems.join(", "))
        }

        Expr::Dict(entries, _) => {
            let pairs: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("(\"{k}\".to_string(), {})", gen_expr(v)))
                .collect();
            format!(
                "Value::Dict(HashMap::from([{}]))",
                pairs.join(", ")
            )
        }

        Expr::Interpolation(parts, _) => {
            let mut exprs = Vec::new();
            for part in parts {
                match part {
                    InterpPart::Literal(s) => {
                        let escaped = s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
                        exprs.push(format!("Value::Str(\"{escaped}\".to_string())"));
                    }
                    InterpPart::Expr(e) => {
                        exprs.push(gen_expr(e));
                    }
                }
            }
            if exprs.is_empty() {
                "Value::Str(String::new())".to_string()
            } else if exprs.len() == 1 {
                format!("Value::Str(({}).to_str())", exprs[0])
            } else {
                let concat: Vec<String> = exprs
                    .iter()
                    .map(|e| format!("({e}).to_str()"))
                    .collect();
                format!(
                    "Value::Str(vec![{}].concat())",
                    concat.join(", ")
                )
            }
        }

        Expr::Closure { params, body, .. } => {
            gen_closure(params, body)
        }

        Expr::Range { start, end, .. } => {
            let s = gen_expr(start);
            let e = gen_expr(end);
            format!(
                "Value::Array((({s}).to_num() as i64..({e}).to_num() as i64).map(|i| Value::Number(i as f64)).collect())"
            )
        }
    }
}

fn gen_literal(lit: &Literal) -> String {
    match lit {
        Literal::Null => "Value::Null".to_string(),
        Literal::Bool(b) => format!("Value::Bool({b})"),
        Literal::Number(n) => {
            if n.is_finite() {
                format!("Value::Number({n}_f64)")
            } else {
                "Value::Number(f64::NAN)".to_string()
            }
        }
        Literal::String(s) => {
            let escaped = s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n").replace('\r', "\\r").replace('\t', "\\t");
            format!("Value::Str(\"{escaped}\".to_string())")
        }
    }
}

fn gen_call(callee: &Expr, args: &[Expr]) -> String {
    // Detect IIFE: fn(...) { body }() — inline the body as a block
    if let Expr::Closure { params, body, .. } = callee {
        let mut s = "{ ".to_string();
        // Bind args to param names in scope
        for (i, p) in params.iter().enumerate() {
            if let Some(arg) = args.get(i) {
                s.push_str(&format!(
                    "scope.set(\"{p}\", {});",
                    gen_expr(arg)
                ));
            }
        }
        for stmt in body {
            s.push_str(&crate::codegen::stmt::gen_stmt(stmt));
            s.push(' ');
        }
        s.push_str("Value::Null }");
        return s;
    }

    // Check for namespaced builtins: git.branch(), file.exists(), etc.
    if let Expr::Member { object, field, .. } = callee {
        if let Expr::Ident(ns, _) = object.as_ref() {
            let builtin = match (ns.as_str(), field.as_str()) {
                ("git", "is_repo") => Some(("builtin_git_is_repo", 0)),
                ("git", "branch") => Some(("builtin_git_branch", 0)),
                ("git", "root") => Some(("builtin_git_root", 0)),
                ("git", "status") => Some(("builtin_git_status", 0)),
                ("git", "origin") => Some(("builtin_git_origin", 0)),
                ("file", "exists") => Some(("builtin_file_exists", 1)),
                ("file", "read") => Some(("builtin_file_read", 1)),
                ("file", "write") => Some(("builtin_file_write", 2)),
                ("file", "type") => Some(("builtin_file_type", 1)),
                ("dir", "search") => Some(("builtin_dir_search", 1)),
                ("dir", "search_upwards") => Some(("builtin_dir_search_upwards", 1)),
                ("battery", "pct") => Some(("builtin_battery_pct", 0)),
                ("battery", "state") => Some(("builtin_battery_state", 0)),
                ("battery", "time") => Some(("builtin_battery_time", 0)),
                _ => None,
            };

            if let Some((func_name, _expected_args)) = builtin {
                let arg_exprs: Vec<String> = args.iter().map(|a| format!("&{}", gen_expr(a))).collect();
                return format!("{func_name}({})", arg_exprs.join(", "));
            }
        }

        // Not a namespace builtin — method call on a value
        let obj = gen_expr(object);
        let arg_exprs: Vec<String> = args.iter().map(gen_expr).collect();
        return format!(
            "value_method_call(&{obj}, \"{field}\", vec![{}])",
            arg_exprs.join(", ")
        );
    }

    // Check for global built-in functions
    if let Expr::Ident(name, _) = callee {
        let arg_exprs: Vec<String> = args.iter().map(gen_expr).collect();
        let ref_args: Vec<String> = args.iter().map(|a| format!("&{}", gen_expr(a))).collect();

        match name.as_str() {
            "env" => return format!("builtin_env({})", ref_args.join(", ")),
            "cwd" => return "builtin_cwd()".to_string(),
            "os" => return "builtin_os()".to_string(),
            "eprint" => return format!("{{ builtin_eprint({}); Value::Null }}", ref_args.join(", ")),
            "colors" => return format!("{{ builtin_colors({}); Value::Null }}", ref_args.join(", ")),
            "C" => return format!("builtin_c({})", ref_args.join(", ")),
            "exec" => return format!("builtin_exec({})", ref_args.join(", ")),
            "exec_ok" => return format!("builtin_exec_ok({})", ref_args.join(", ")),
            "regex" => {
                if args.len() == 1 {
                    return format!("builtin_regex({}, &Value::Str(String::new()))", ref_args[0]);
                }
                return format!("builtin_regex({})", ref_args.join(", "));
            }
            "time" => {
                let fmt = arg_exprs.first().map(|a| a.clone()).unwrap_or_else(|| "Value::Str(\"%H:%M\".to_string())".to_string());
                return format!(
                    "Value::Str({{ let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs(); let tm = time_to_parts(now); format_time(&tm, &({fmt}).to_str()) }})"
                );
            }
            "floor" => return format!("Value::Number(({}).to_num().floor())", arg_exprs.first().unwrap_or(&"Value::Null".to_string())),
            "ceil" => return format!("Value::Number(({}).to_num().ceil())", arg_exprs.first().unwrap_or(&"Value::Null".to_string())),
            "round" => return format!("Value::Number(({}).to_num().round())", arg_exprs.first().unwrap_or(&"Value::Null".to_string())),
            "string" => return format!("builtin_string({})", ref_args.join(", ")),
            "number" => return format!("builtin_number({})", ref_args.join(", ")),
            "array" => return format!("builtin_array({})", ref_args.join(", ")),
            "dict" => return format!("builtin_dict({})", ref_args.join(", ")),
            "spawn" => return format!("builtin_spawn({})", ref_args.join(", ")),
            "wait" => return format!("builtin_wait({})", ref_args.join(", ")),
            _ => {
                // Dynamic dispatch: look up in scope, call if closure
                let arg_list = arg_exprs.join(", ");
                return format!(
                    "{{ let __callee = scope.get(\"{name}\"); match __callee {{ Value::Closure(ref f) => f(vec![{arg_list}]), _ => {{ eprintln!(\"promptorius: '{{}}' is not a function\", \"{name}\"); std::process::exit(1); }} }} }}"
                );
            }
        }
    }

    // Dynamic call (closure in a variable)
    let callee_code = gen_expr(callee);
    let arg_exprs: Vec<String> = args.iter().map(gen_expr).collect();
    format!(
        "match &{callee_code} {{ Value::Closure(f) => f(vec![{}]), _ => Value::Null }}",
        arg_exprs.join(", ")
    )
}

fn gen_assignment(target: &Expr, value: &Expr) -> String {
    let val = gen_expr(value);
    match target {
        Expr::Ident(name, _) => {
            format!("{{ let __v = {val}; scope.set(\"{name}\", __v.clone()); __v }}")
        }
        Expr::Member { object, field, .. } => {
            if let Expr::Ident(obj_name, _) = object.as_ref() {
                format!(
                    "{{ let __v = {val}; let mut __obj = scope.get(\"{obj_name}\"); value_set_member(&mut __obj, \"{field}\", __v.clone()); scope.set(\"{obj_name}\", __obj); __v }}"
                )
            } else {
                format!("{{ {val} }}")
            }
        }
        Expr::Index { object, index, .. } => {
            if let Expr::Ident(obj_name, _) = object.as_ref() {
                let idx = gen_expr(index);
                format!(
                    "{{ let __v = {val}; let __idx = {idx}; let mut __obj = scope.get(\"{obj_name}\"); value_set_index(&mut __obj, &__idx, __v.clone()); scope.set(\"{obj_name}\", __obj); __v }}"
                )
            } else {
                format!("{{ {val} }}")
            }
        }
        _ => format!("{{ {val} }}"),
    }
}

fn gen_compound_assignment(op: &BinOp, target: &Expr, value: &Expr) -> String {
    let val = gen_expr(value);
    let func = match op {
        BinOp::Add => "value_add",
        BinOp::Sub => "value_sub",
        BinOp::Mul => "value_mul",
        BinOp::Div => "value_div",
        BinOp::Mod => "value_mod",
        _ => "value_add",
    };

    match target {
        Expr::Ident(name, _) => {
            format!(
                "{{ let __v = {func}(&scope.get(\"{name}\"), &{val}); scope.set(\"{name}\", __v.clone()); __v }}"
            )
        }
        Expr::Member { object, field, .. } => {
            if let Expr::Ident(obj_name, _) = object.as_ref() {
                format!(
                    "{{ let mut __obj = scope.get(\"{obj_name}\"); let __old = value_member_access(&__obj, \"{field}\"); let __v = {func}(&__old, &{val}); value_set_member(&mut __obj, \"{field}\", __v.clone()); scope.set(\"{obj_name}\", __obj); __v }}"
                )
            } else {
                format!("{{ {val} }}")
            }
        }
        Expr::Index { object, index, .. } => {
            if let Expr::Ident(obj_name, _) = object.as_ref() {
                let idx = gen_expr(index);
                format!(
                    "{{ let mut __obj = scope.get(\"{obj_name}\"); let __idx = {idx}; let __old = value_index(&__obj, &__idx); let __v = {func}(&__old, &{val}); value_set_index(&mut __obj, &__idx, __v.clone()); scope.set(\"{obj_name}\", __obj); __v }}"
                )
            } else {
                format!("{{ {val} }}")
            }
        }
        _ => format!("{{ {val} }}"),
    }
}

fn gen_closure(params: &[String], body: &[Stmt]) -> String {
    use crate::codegen::stmt;

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

    let body_code: Vec<String> = body.iter().map(|s| stmt::gen_stmt(s)).collect();

    format!(
        "Value::Closure(Arc::new(move |__args: Vec<Value>| -> Value {{ {} {} Value::Null }}))",
        param_extracts.join(" "),
        body_code.join(" "),
    )
}
