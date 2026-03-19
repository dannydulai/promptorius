//! TOML configuration parsing and validation.
//!
//! Reads `$XDG_CONFIG_HOME/promptorius/config.toml` and produces typed structs.
//! This module is a leaf — no dependencies on other promptorius modules.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("config file not found: {0}")]
    NotFound(PathBuf),

    #[error("failed to read config: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid config: {0}")]
    Parse(#[from] toml::de::Error),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub prompt: PromptConfig,
    #[serde(default)]
    pub colors: HashMap<String, ColorDef>,
    #[serde(default)]
    pub segments: HashMap<String, SegmentConfig>,
    #[serde(default)]
    pub settings: Settings,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PromptConfig {
    pub format: String,
    #[serde(default)]
    pub right_format: Option<String>,
    #[serde(default = "default_true")]
    pub add_newline: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ColorDef {
    /// Simple string: just a foreground color name/hex.
    Simple(String),
    /// Full definition with fg, bg, bold, italic, etc.
    Full {
        #[serde(default)]
        fg: Option<String>,
        #[serde(default)]
        bg: Option<String>,
        #[serde(default)]
        bold: bool,
        #[serde(default)]
        italic: bool,
        #[serde(default)]
        dim: bool,
        #[serde(default)]
        strikethrough: bool,
        #[serde(default)]
        underline: Option<UnderlineStyle>,
        #[serde(default)]
        underline_color: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UnderlineStyle {
    Single,
    Double,
    Curly,
    Dotted,
    Dashed,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SegmentConfig {
    pub script: Option<String>,
    /// All other key-value pairs are passed to the script as the `config` map.
    #[serde(flatten)]
    pub extra: HashMap<String, toml::Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub script_path: Vec<String>,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            script_path: Vec::new(),
            timeout: default_timeout(),
            concurrency: default_concurrency(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    50
}

fn default_concurrency() -> usize {
    4
}

/// Load config from the default path. Creates a default config if none exists.
pub fn load() -> Result<Config, ConfigError> {
    let path = config_path();
    if !path.exists() {
        create_default_config(&path)?;
    }
    let contents = std::fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

const DEFAULT_CONFIG: &str = r#"[prompt]
format = '{s("directory")} {s("git")}{s("character")}'
right_format = '{s("exitcode")}'
add_newline = true

[colors]
default = "white"
directory = { fg = "cyan", bold = true }
git = "purple"
success = "green"
warning = "yellow"
error = { fg = "red", bold = true }
muted = { fg = "bright_black", dim = true }

[segments.directory]
script = "directory.rhai"

[segments.git]
script = "git.rhai"

[segments.character]
script = "character.rhai"

[segments.exitcode]
script = "exitcode.rhai"

[settings]
timeout = 50
concurrency = 4
"#;

fn create_default_config(path: &std::path::Path) -> Result<(), ConfigError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, DEFAULT_CONFIG)?;
    Ok(())
}

/// Resolve the config directory: `$XDG_CONFIG_HOME/promptorius/`
/// (defaults to `$HOME/.config/promptorius/`).
pub fn config_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".config")
        });
    base.join("promptorius")
}

/// Resolve the config file path.
pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

/// Resolve the cache directory: `$XDG_CACHE_HOME/promptorius/`
/// (defaults to `$HOME/.cache/promptorius/`).
pub fn cache_dir() -> PathBuf {
    let base = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".cache")
        });
    base.join("promptorius")
}

/// Resolve the ordered list of directories to search for .rhai scripts.
pub fn script_search_paths(settings: &Settings) -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 1. User config scripts dir (always first)
    paths.push(config_dir().join("scripts"));

    // 2. Additional paths from settings
    for p in &settings.script_path {
        let expanded = shellexpand_tilde(p);
        paths.push(PathBuf::from(expanded));
    }

    // 3. Stdlib (bundled with binary — resolved at runtime)
    // This will be handled by the script loader, not listed here.

    paths
}

fn shellexpand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).to_string_lossy().into_owned();
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_config() {
        let toml_str = r#"
[prompt]
format = "{s(\"directory\")}> "

[segments.directory]
script = "directory.rhai"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.segments.len(), 1);
        assert!(config.segments.contains_key("directory"));
    }

    #[test]
    fn parse_color_simple() {
        let toml_str = r#"
[prompt]
format = ""

[colors]
error = "red"
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(matches!(config.colors.get("error"), Some(ColorDef::Simple(_))));
    }

    #[test]
    fn parse_color_full() {
        let toml_str = r#"
[prompt]
format = ""

[colors]
error = { fg = "red", bold = true }
"#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(matches!(config.colors.get("error"), Some(ColorDef::Full { .. })));
    }
}
