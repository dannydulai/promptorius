//! Format template parser and evaluator.
//!
//! Parses format strings like `{s("directory")} {color("git")}{s("git")}{color("")} > `
//! extracting `{expr}` blocks as Rhai expressions and handling `{{`/`}}` escaping.

use crate::script::{ScriptEngine, ScriptError};

/// A parsed segment of a format template.
#[derive(Debug, PartialEq)]
enum TemplatePart {
    /// Literal text (already unescaped).
    Literal(String),
    /// A Rhai expression to evaluate.
    Expression(String),
}

/// Parse a format template string into parts.
fn parse(template: &str) -> Vec<TemplatePart> {
    let mut parts = Vec::new();
    let chars: Vec<char> = template.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut literal = String::new();

    while i < len {
        if chars[i] == '{' {
            if i + 1 < len && chars[i + 1] == '{' {
                // Escaped opening brace
                literal.push('{');
                i += 2;
            } else {
                // Start of expression — flush literal
                if !literal.is_empty() {
                    parts.push(TemplatePart::Literal(std::mem::take(&mut literal)));
                }
                // Find matching closing brace, respecting nested braces and strings
                i += 1; // skip opening {
                let expr_start = i;
                let mut depth = 1;
                let mut in_double_quote = false;
                let mut in_single_quote = false;

                while i < len && depth > 0 {
                    let c = chars[i];
                    if in_double_quote {
                        if c == '\\' && i + 1 < len {
                            i += 1; // skip escaped char
                        } else if c == '"' {
                            in_double_quote = false;
                        }
                    } else if in_single_quote {
                        if c == '\'' {
                            in_single_quote = false;
                        }
                    } else {
                        match c {
                            '"' => in_double_quote = true,
                            '\'' => in_single_quote = true,
                            '{' => depth += 1,
                            '}' => depth -= 1,
                            _ => {}
                        }
                    }
                    if depth > 0 {
                        i += 1;
                    }
                }

                let expr: String = chars[expr_start..i].iter().collect();
                parts.push(TemplatePart::Expression(expr));
                i += 1; // skip closing }
            }
        } else if chars[i] == '}' {
            if i + 1 < len && chars[i + 1] == '}' {
                // Escaped closing brace
                literal.push('}');
                i += 2;
            } else {
                // Stray closing brace — treat as literal
                literal.push('}');
                i += 1;
            }
        } else {
            literal.push(chars[i]);
            i += 1;
        }
    }

    if !literal.is_empty() {
        parts.push(TemplatePart::Literal(literal));
    }

    parts
}

/// Parse and evaluate a format template string, returning the rendered output.
pub fn evaluate(template: &str, engine: &ScriptEngine) -> Result<String, ScriptError> {
    let parts = parse(template);
    let mut output = String::new();

    for part in parts {
        match part {
            TemplatePart::Literal(s) => output.push_str(&s),
            TemplatePart::Expression(expr) => {
                let result = engine.eval_expression(&expr)?;
                output.push_str(&result);
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_literal_only() {
        let parts = parse("hello world");
        assert_eq!(parts, vec![TemplatePart::Literal("hello world".to_string())]);
    }

    #[test]
    fn parse_expression() {
        let parts = parse("{1 + 2}");
        assert_eq!(parts, vec![TemplatePart::Expression("1 + 2".to_string())]);
    }

    #[test]
    fn parse_mixed() {
        let parts = parse("hello {1 + 2} world");
        assert_eq!(
            parts,
            vec![
                TemplatePart::Literal("hello ".to_string()),
                TemplatePart::Expression("1 + 2".to_string()),
                TemplatePart::Literal(" world".to_string()),
            ]
        );
    }

    #[test]
    fn parse_escaped_braces() {
        let parts = parse("{{literal}}");
        assert_eq!(parts, vec![TemplatePart::Literal("{literal}".to_string())]);
    }

    #[test]
    fn parse_nested_braces() {
        let parts = parse(r#"{if true { "yes" } else { "no" }}"#);
        assert_eq!(
            parts,
            vec![TemplatePart::Expression(
                r#"if true { "yes" } else { "no" }"#.to_string()
            )]
        );
    }

    #[test]
    fn parse_string_with_braces() {
        let parts = parse(r#"{s("directory")}"#);
        assert_eq!(
            parts,
            vec![TemplatePart::Expression(r#"s("directory")"#.to_string())]
        );
    }

    #[test]
    fn evaluate_literal() {
        let engine = ScriptEngine::new();
        let result = evaluate("hello", &engine).unwrap();
        assert_eq!(result, "hello");
    }

    #[test]
    fn evaluate_expression() {
        let engine = ScriptEngine::new();
        let result = evaluate("{1 + 2}", &engine).unwrap();
        assert_eq!(result, "3");
    }

    #[test]
    fn evaluate_mixed() {
        let engine = ScriptEngine::new();
        let result = evaluate("val={1 + 2}!", &engine).unwrap();
        assert_eq!(result, "val=3!");
    }

    #[test]
    fn evaluate_escaped_braces() {
        let engine = ScriptEngine::new();
        let result = evaluate("{{hello}}", &engine).unwrap();
        assert_eq!(result, "{hello}");
    }
}
