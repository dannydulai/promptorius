//! Rhai engine setup, script loading, AST caching, and evaluation.
//!
//! Configures the Rhai engine with string coercion overloads and delegates
//! host API registration to the `host` module.

use rhai::{Engine, AST};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ScriptError {
    #[error("script not found: {0}")]
    NotFound(String),

    #[error("script compile error: {0}")]
    Compile(String),

    #[error("script runtime error: {0}")]
    Runtime(String),
}

/// Manages the Rhai engine and compiled script cache.
pub struct ScriptEngine {
    engine: Engine,
    cache: HashMap<PathBuf, AST>,
}

impl ScriptEngine {
    /// Create a new engine with string coercion overloads registered.
    pub fn new() -> Self {
        let mut engine = Engine::new();

        // String coercion: allow "text" + 42, true + "text", etc.
        engine.register_fn("+", |s: &str, n: i64| format!("{s}{n}"));
        engine.register_fn("+", |n: i64, s: &str| format!("{n}{s}"));
        engine.register_fn("+", |s: &str, n: f64| format!("{s}{n}"));
        engine.register_fn("+", |n: f64, s: &str| format!("{n}{s}"));
        engine.register_fn("+", |s: &str, b: bool| format!("{s}{b}"));
        engine.register_fn("+", |b: bool, s: &str| format!("{b}{s}"));

        Self {
            engine,
            cache: HashMap::new(),
        }
    }

    /// Get a mutable reference to the underlying Rhai engine for host API registration.
    pub fn engine_mut(&mut self) -> &mut Engine {
        &mut self.engine
    }

    /// Get a reference to the underlying Rhai engine.
    pub fn engine(&self) -> &Engine {
        &self.engine
    }

    /// Compile a script file and cache the AST. Returns the cached AST on subsequent calls.
    pub fn compile_file(&mut self, path: &Path) -> Result<&AST, ScriptError> {
        if !self.cache.contains_key(path) {
            let source = std::fs::read_to_string(path)
                .map_err(|_| ScriptError::NotFound(path.display().to_string()))?;
            let ast = self
                .engine
                .compile(&source)
                .map_err(|e| ScriptError::Compile(e.to_string()))?;
            self.cache.insert(path.to_path_buf(), ast);
        }
        Ok(&self.cache[path])
    }

    /// Evaluate a compiled AST and return the result as a string.
    /// Returns `None` if the script returned `()`.
    pub fn eval_ast(&self, ast: &AST) -> Result<Option<String>, ScriptError> {
        let result: rhai::Dynamic = self
            .engine
            .eval_ast(ast)
            .map_err(|e| ScriptError::Runtime(e.to_string()))?;

        if result.is_unit() {
            Ok(None)
        } else {
            Ok(Some(result.to_string()))
        }
    }

    /// Evaluate a Rhai expression string (used for format template expressions).
    pub fn eval_expression(&self, expr: &str) -> Result<String, ScriptError> {
        let result: rhai::Dynamic = self
            .engine
            .eval_expression::<rhai::Dynamic>(expr)
            .or_else(|_| self.engine.eval::<rhai::Dynamic>(expr))
            .map_err(|e| ScriptError::Runtime(e.to_string()))?;

        if result.is_unit() {
            Ok(String::new())
        } else {
            Ok(result.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn string_coercion_int() {
        let engine = ScriptEngine::new();
        let result: String = engine.engine().eval(r#""count: " + 42"#).unwrap();
        assert_eq!(result, "count: 42");
    }

    #[test]
    fn string_coercion_bool() {
        let engine = ScriptEngine::new();
        let result: String = engine.engine().eval(r#""flag: " + true"#).unwrap();
        assert_eq!(result, "flag: true");
    }
}
