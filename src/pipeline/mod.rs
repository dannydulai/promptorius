//! Segment pipeline: two-pass execution and format template evaluation.
//!
//! Pass 1: Parse the format template to find s("name") calls, then execute
//!         those segments using a shared engine (one engine setup for all segments).
//! Pass 2: Evaluate the format template with segment results available via s().

mod template;

use crate::config::Config;
use crate::host;
use crate::script::ScriptEngine;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("script error: {0}")]
    Script(#[from] crate::script::ScriptError),

    #[error("config error: {0}")]
    Config(#[from] crate::config::ConfigError),
}

/// Timing info for a single segment execution.
#[derive(Debug, Clone)]
pub struct SegmentTiming {
    pub name: String,
    pub compile_us: u128,
    pub exec_us: u128,
    pub total_us: u128,
    pub output_len: usize,
    pub had_output: bool,
    pub error: Option<String>,
}

/// Stats collected during a render.
#[derive(Debug, Clone)]
pub struct RenderStats {
    pub engine_setup_us: u128,
    pub segments: Vec<SegmentTiming>,
    pub template_eval_us: u128,
    pub total_us: u128,
}

/// Render a prompt string.
pub fn render(
    config: &Config,
    cmds: &[host::CmdDef],
    right: bool,
    script_dirs: &[PathBuf],
    stdlib_scripts: &HashMap<String, &str>,
) -> Result<String, PipelineError> {
    let (output, _) = render_with_stats(config, cmds, right, script_dirs, stdlib_scripts)?;
    Ok(output)
}

/// Render a prompt and return timing stats.
pub fn render_with_stats(
    config: &Config,
    cmds: &[host::CmdDef],
    right: bool,
    script_dirs: &[PathBuf],
    stdlib_scripts: &HashMap<String, &str>,
) -> Result<(String, RenderStats), PipelineError> {
    let total_start = Instant::now();

    let format_str = if right {
        config.prompt.right_format.as_deref().unwrap_or("")
    } else {
        &config.prompt.format
    };

    if format_str.is_empty() {
        return Ok((String::new(), RenderStats {
            engine_setup_us: 0,
            segments: vec![],
            template_eval_us: 0,
            total_us: 0,
        }));
    }

    // Pre-resolve stdlib into owned strings
    let stdlib_owned: HashMap<String, String> = stdlib_scripts
        .iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();

    // Pass 1: Extract segment names from the format template
    let segment_names = template::extract_segment_names(format_str);

    // Set up one shared engine with all host functions
    let engine_start = Instant::now();
    let mut engine = ScriptEngine::new();
    host::register_all(&mut engine, config, cmds);
    let engine_setup_us = engine_start.elapsed().as_micros();

    // Execute each segment with the shared engine
    let mut results: HashMap<String, String> = HashMap::new();
    let mut timings: Vec<SegmentTiming> = Vec::new();

    for name in &segment_names {
        let seg_config = config.segments.get(name.as_str());
        let default_script = format!("{name}.rhai");
        let script_name = seg_config
            .and_then(|s| s.script.as_deref())
            .unwrap_or(&default_script);

        let source = match resolve_script(script_name, script_dirs, &stdlib_owned) {
            Some(s) => s,
            None => {
                let msg = format!("script not found: {script_name}");
                eprintln!("promptorius: {msg}");
                timings.push(SegmentTiming {
                    name: name.clone(),
                    compile_us: 0,
                    exec_us: 0,
                    total_us: 0,
                    output_len: 0,
                    had_output: false,
                    error: Some(msg),
                });
                continue;
            }
        };

        // Compile
        let compile_start = Instant::now();
        let ast = match engine.compile_source(&source) {
            Ok(ast) => ast,
            Err(e) => {
                let msg = format!("{script_name}: {e}");
                eprintln!("promptorius: {msg}");
                timings.push(SegmentTiming {
                    name: name.clone(),
                    compile_us: compile_start.elapsed().as_micros(),
                    exec_us: 0,
                    total_us: compile_start.elapsed().as_micros(),
                    output_len: 0,
                    had_output: false,
                    error: Some(msg),
                });
                continue;
            }
        };
        let compile_us = compile_start.elapsed().as_micros();

        // Execute with per-segment scope
        let empty = HashMap::new();
        let extra = seg_config.map(|s| &s.extra).unwrap_or(&empty);
        let mut scope = host::segment_scope(extra);

        let exec_start = Instant::now();
        let (output, error) = match engine.eval_ast_with_scope(&ast, &mut scope) {
            Ok(Some(s)) => (s, None),
            Ok(None) => (String::new(), None),
            Err(e) => {
                let msg = format!("{script_name}: {e}");
                eprintln!("promptorius: {msg}");
                (String::new(), Some(msg))
            }
        };
        let exec_us = exec_start.elapsed().as_micros();

        let had_output = !output.is_empty();
        let output_len = output.len();
        results.insert(name.clone(), output);

        timings.push(SegmentTiming {
            name: name.clone(),
            compile_us,
            exec_us,
            total_us: compile_us + exec_us,
            output_len,
            had_output,
            error,
        });
    }

    // Pass 2: Evaluate the format template with s() returning cached results
    let template_start = Instant::now();

    // Register s() that just does a lookup
    let results_clone = results.clone();
    engine
        .engine_mut()
        .register_fn("s", move |name: &str| -> String {
            results_clone.get(name).cloned().unwrap_or_default()
        });

    let output = template::evaluate(format_str, &engine)?;
    let template_eval_us = template_start.elapsed().as_micros();

    // Add newline prefix if configured
    let output = if !right && config.prompt.add_newline {
        format!("\n{output}")
    } else {
        output
    };

    // Wrap ANSI escapes for shell compatibility
    let shell = cmds.iter().find(|c| c.name == "shell");
    let output = match shell.map(|c| &c.value) {
        Some(host::CmdValue::Str(s)) => wrap_escapes_for_shell(&output, s),
        _ => output,
    };

    let total_us = total_start.elapsed().as_micros();

    let stats = RenderStats {
        engine_setup_us,
        segments: timings,
        template_eval_us,
        total_us,
    };

    Ok((output, stats))
}

/// Wrap ANSI escape sequences for shell-specific prompt rendering.
fn wrap_escapes_for_shell(s: &str, shell: &str) -> String {
    match shell {
        "zsh" => wrap_ansi_escapes(s, "%{", "%}"),
        "bash" => wrap_ansi_escapes(s, "\\[", "\\]"),
        _ => s.to_string(),
    }
}

/// Wrap all ANSI escape sequences (\x1b[...m) with the given prefix/suffix.
fn wrap_ansi_escapes(s: &str, prefix: &str, suffix: &str) -> String {
    let mut result = String::with_capacity(s.len() * 2);
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == 0x1b && i + 1 < len && bytes[i + 1] == b'[' {
            result.push_str(prefix);
            result.push('\x1b');
            result.push('[');
            i += 2;
            while i < len {
                result.push(bytes[i] as char);
                if bytes[i] == b'm' {
                    i += 1;
                    break;
                }
                i += 1;
            }
            result.push_str(suffix);
        } else {
            let c = s[i..].chars().next().unwrap();
            result.push(c);
            i += c.len_utf8();
        }
    }

    result
}

/// Resolve a script filename to its source code by searching directories, then stdlib.
fn resolve_script(
    script_name: &str,
    search_dirs: &[PathBuf],
    stdlib_scripts: &HashMap<String, String>,
) -> Option<String> {
    for dir in search_dirs {
        let path = dir.join(script_name);
        if path.exists() {
            if let Ok(source) = std::fs::read_to_string(&path) {
                return Some(source);
            }
        }
    }
    stdlib_scripts.get(script_name).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_stdlib_fallback() {
        let mut stdlib = HashMap::new();
        stdlib.insert("test.rhai".to_string(), "42".to_string());

        let result = resolve_script("test.rhai", &[], &stdlib);
        assert_eq!(result, Some("42".to_string()));
    }

    #[test]
    fn resolve_missing_script() {
        let stdlib = HashMap::new();
        let result = resolve_script("nonexistent.rhai", &[], &stdlib);
        assert!(result.is_none());
    }
}
