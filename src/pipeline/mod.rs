//! Segment pipeline: resolution, concurrent execution, format template evaluation.
//!
//! Resolves which segments to run from config, executes them concurrently,
//! and evaluates the format template to produce the final prompt string.

mod template;

use crate::config::{Config, SegmentConfig};
use crate::host;
use crate::script::ScriptEngine;
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("script error: {0}")]
    Script(#[from] crate::script::ScriptError),

    #[error("config error: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("timeout: prompt render exceeded {0}ms")]
    Timeout(u64),
}

/// Render a prompt string from config, executing all segments and evaluating the format template.
pub fn render(
    config: &Config,
    cmds: &[host::CmdDef],
    right: bool,
    script_dirs: &[PathBuf],
    stdlib_scripts: &HashMap<String, &str>,
) -> Result<String, PipelineError> {
    let format_str = if right {
        config.prompt.right_format.as_deref().unwrap_or("")
    } else {
        &config.prompt.format
    };

    if format_str.is_empty() {
        return Ok(String::new());
    }

    // Collect all segment names referenced in the format template
    let segment_names = collect_segment_names(format_str, &config.segments);

    // Execute all segments concurrently
    let timeout = Duration::from_millis(config.settings.timeout);
    let start = Instant::now();

    let results: HashMap<String, String> = segment_names
        .par_iter()
        .filter_map(|name| {
            if start.elapsed() >= timeout {
                return None;
            }

            let seg_config = config.segments.get(name.as_str());
            let default_script = format!("{name}.rhai");
            let script_name = seg_config
                .and_then(|s| s.script.as_deref())
                .unwrap_or(&default_script);

            let script_source = resolve_script(script_name, script_dirs, stdlib_scripts)?;

            // Each segment gets its own engine instance with host API
            let mut engine = ScriptEngine::new();
            host::register_all(&mut engine, config, cmds);

            // Build scope with segment-specific config map
            let empty = HashMap::new();
            let extra = seg_config.map(|s| &s.extra).unwrap_or(&empty);
            let mut scope = host::segment_scope(extra);

            let ast = engine.compile_source(&script_source).ok()?;
            let output = engine.eval_ast_with_scope(&ast, &mut scope).ok()?;

            output.map(|s| (name.clone(), s))
        })
        .collect();

    // Now evaluate the format template with segment results available
    let mut eval_engine = ScriptEngine::new();
    host::register_all(&mut eval_engine, config, cmds);

    // Register s() function that looks up segment results
    let results_clone = results.clone();
    eval_engine
        .engine_mut()
        .register_fn("s", move |name: &str| -> String {
            results_clone.get(name).cloned().unwrap_or_default()
        });

    // Parse and evaluate the format template
    let output = template::evaluate(format_str, &eval_engine)?;

    // Add newline prefix if configured
    let output = if !right && config.prompt.add_newline {
        format!("\n{output}")
    } else {
        output
    };

    Ok(output)
}

/// Collect segment names that are referenced in config (we run all defined segments
/// since the format template uses s("name") calls which are opaque to static analysis).
fn collect_segment_names(
    _format_str: &str,
    segments: &HashMap<String, SegmentConfig>,
) -> Vec<String> {
    segments.keys().cloned().collect()
}

/// Resolve a script filename to its source code by searching directories, then stdlib.
fn resolve_script(
    script_name: &str,
    search_dirs: &[PathBuf],
    stdlib_scripts: &HashMap<String, &str>,
) -> Option<String> {
    // Search user dirs first
    for dir in search_dirs {
        let path = dir.join(script_name);
        if path.exists() {
            if let Ok(source) = std::fs::read_to_string(&path) {
                return Some(source);
            }
        }
    }

    // Fall back to bundled stdlib
    stdlib_scripts.get(script_name).map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SegmentConfig;

    #[test]
    fn collect_all_segment_names() {
        let mut segments = HashMap::new();
        segments.insert(
            "directory".to_string(),
            SegmentConfig {
                script: Some("directory.rhai".to_string()),
                extra: HashMap::new(),
            },
        );
        segments.insert(
            "git".to_string(),
            SegmentConfig {
                script: Some("git.rhai".to_string()),
                extra: HashMap::new(),
            },
        );
        let names = collect_segment_names("", &segments);
        assert_eq!(names.len(), 2);
    }
}
