//! Compiler orchestration: parse → codegen → cargo build.

pub mod project;

use crate::codegen;
use crate::lang::parser::Parser;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CompileError {
    #[error("parse error: {0}")]
    Parse(#[from] crate::lang::parser::ParseError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("build failed: {0}")]
    Build(String),
}

/// Compile a script file to a native binary.
pub fn compile(script_path: &Path, output_path: &Path) -> Result<(), CompileError> {
    eprint!("Promptorius needs to rebuild. Please wait a few seconds while this happens…");

    let result = (|| -> Result<(), CompileError> {
        let source = std::fs::read_to_string(script_path)?;
        let program = Parser::parse(&source)?;
        let rust_source = codegen::generate(&program);
        let build_dir = project::ensure_build_project()?;
        project::write_source(&build_dir, &rust_source)?;
        project::build(&build_dir)?;
        project::copy_binary(&build_dir, output_path)?;
        Ok(())
    })();

    match result {
        Ok(()) => {
            eprintln!(" done.");
            Ok(())
        }
        Err(e) => {
            eprintln!("\n\nBuild failed.\n\n{e}");
            Err(e)
        }
    }
}

#[allow(dead_code)]
/// Check if the output binary is stale relative to the script and compiler.
pub fn is_stale(script_path: &Path, output_path: &Path) -> bool {
    if !output_path.exists() {
        return true;
    }

    let binary_mtime = match std::fs::metadata(output_path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return true,
    };

    // Check script mtime
    if let Ok(meta) = std::fs::metadata(script_path) {
        if let Ok(script_mtime) = meta.modified() {
            if script_mtime > binary_mtime {
                return true;
            }
        }
    }

    // Check compiler binary mtime
    if let Ok(compiler_path) = std::env::current_exe() {
        if let Ok(meta) = std::fs::metadata(&compiler_path) {
            if let Ok(compiler_mtime) = meta.modified() {
                if compiler_mtime > binary_mtime {
                    return true;
                }
            }
        }
    }

    false
}

/// Remove the build directory.
pub fn clean() -> Result<(), CompileError> {
    let build_dir = project::build_dir();
    if build_dir.exists() {
        std::fs::remove_dir_all(&build_dir)?;
        eprintln!("promptorius: cleaned {}", build_dir.display());
    } else {
        eprintln!("promptorius: nothing to clean");
    }
    Ok(())
}

/// Default script file path.
pub fn default_script_path() -> PathBuf {
    config_dir().join("config")
}

/// Default output binary path.
pub fn default_output_path() -> PathBuf {
    data_dir().join("__promptorius_output")
}

/// XDG config dir for promptorius.
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

/// XDG data dir for promptorius.
pub fn data_dir() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".local")
                .join("share")
        });
    base.join("promptorius")
}

/// Default config script content, shipped in the binary.
pub const DEFAULT_CONFIG: &str = include_str!("../../default_config");

/// Ensure the default config exists, creating it if needed.
pub fn ensure_default_config() -> Result<PathBuf, CompileError> {
    let path = default_script_path();
    if !path.exists() {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, DEFAULT_CONFIG)?;
        eprintln!("promptorius: created default config at {}", path.display());
    }
    Ok(path)
}
