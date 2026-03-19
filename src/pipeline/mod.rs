//! Segment pipeline: lazy segment execution and format template evaluation.
//!
//! Segments are only executed when `s("name")` is called from the format template.
//! Results are cached so repeated `s()` calls for the same segment are free.

mod template;

use crate::config::Config;
use crate::host;
use crate::script::ScriptEngine;
use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
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
    pub duration_us: u128,
    pub output_len: usize,
    pub had_output: bool,
    pub error: Option<String>,
}

/// Stats collected during a render.
#[derive(Debug, Clone)]
pub struct RenderStats {
    pub segments: Vec<SegmentTiming>,
    pub template_eval_us: u128,
    pub total_us: u128,
}

/// Render a prompt string from config, lazily executing segments as `s()` is called.
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

/// Render a prompt and return timing stats for each segment.
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
            segments: vec![],
            template_eval_us: 0,
            total_us: 0,
        }));
    }

    // Set up the eval engine for the format template
    let mut eval_engine = ScriptEngine::new();
    host::register_all(&mut eval_engine, config, cmds);

    // Cache for lazily-evaluated segment results
    let cache: Rc<RefCell<HashMap<String, String>>> =
        Rc::new(RefCell::new(HashMap::new()));

    // Timing log
    let timings: Rc<RefCell<Vec<SegmentTiming>>> =
        Rc::new(RefCell::new(Vec::new()));

    // Clone what s() needs into its closure
    let segments = config.segments.clone();
    let colors = config.colors.clone();
    let cmds_owned: Vec<host::CmdDef> = cmds.to_vec();
    let dirs_owned: Vec<PathBuf> = script_dirs.to_vec();
    let stdlib_owned: HashMap<String, String> = stdlib_scripts
        .iter()
        .map(|(k, v)| (k.clone(), v.to_string()))
        .collect();
    let cache_clone = cache.clone();
    let timings_clone = timings.clone();

    eval_engine
        .engine_mut()
        .register_fn("s", move |name: &str| -> String {
            // Return cached result if already executed
            if let Some(result) = cache_clone.borrow().get(name) {
                return result.clone();
            }

            let start = Instant::now();

            // Resolve and execute the segment
            let (output, error) = execute_segment_timed(
                name,
                &segments,
                &colors,
                &cmds_owned,
                &dirs_owned,
                &stdlib_owned,
            );

            let elapsed = start.elapsed().as_micros();
            let result = output.unwrap_or_default();

            timings_clone.borrow_mut().push(SegmentTiming {
                name: name.to_string(),
                duration_us: elapsed,
                output_len: result.len(),
                had_output: !result.is_empty(),
                error,
            });

            cache_clone
                .borrow_mut()
                .insert(name.to_string(), result.clone());
            result
        });

    // Parse and evaluate the format template
    let template_start = Instant::now();
    let output = template::evaluate(format_str, &eval_engine)?;
    let template_us = template_start.elapsed().as_micros();

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

    // Subtract segment times from template time to get pure template eval
    let segment_total_us: u128 = timings.borrow().iter().map(|t| t.duration_us).sum();
    let pure_template_us = template_us.saturating_sub(segment_total_us);

    let stats = RenderStats {
        segments: timings.borrow().clone(),
        template_eval_us: pure_template_us,
        total_us,
    };

    Ok((output, stats))
}

/// Execute a single segment, returning output and optional error message.
fn execute_segment_timed(
    name: &str,
    segments: &HashMap<String, crate::config::SegmentConfig>,
    colors: &HashMap<String, crate::config::ColorDef>,
    cmds: &[host::CmdDef],
    script_dirs: &[PathBuf],
    stdlib_scripts: &HashMap<String, String>,
) -> (Option<String>, Option<String>) {
    let seg_config = segments.get(name);
    let default_script = format!("{name}.rhai");
    let script_name = seg_config
        .and_then(|s| s.script.as_deref())
        .unwrap_or(&default_script);

    let script_source = match resolve_script(script_name, script_dirs, stdlib_scripts) {
        Some(s) => s,
        None => return (None, Some(format!("script not found: {script_name}"))),
    };

    // Build a minimal config just for host registration
    let config_for_host = crate::config::Config {
        prompt: crate::config::PromptConfig {
            format: String::new(),
            right_format: None,
            add_newline: false,
        },
        colors: colors.clone(),
        segments: HashMap::new(),
        settings: crate::config::Settings::default(),
    };

    let mut engine = ScriptEngine::new();
    host::register_all(&mut engine, &config_for_host, cmds);

    // Build scope with segment-specific config map
    let empty = HashMap::new();
    let extra = seg_config.map(|s| &s.extra).unwrap_or(&empty);
    let mut scope = host::segment_scope(extra);

    let ast = match engine.compile_source(&script_source) {
        Ok(ast) => ast,
        Err(e) => {
            let msg = format!("{script_name}: {e}");
            eprintln!("promptorius: {msg}");
            return (None, Some(msg));
        }
    };
    match engine.eval_ast_with_scope(&ast, &mut scope) {
        Ok(result) => (result, None),
        Err(e) => {
            let msg = format!("{script_name}: {e}");
            eprintln!("promptorius: {msg}");
            (None, Some(msg))
        }
    }
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
