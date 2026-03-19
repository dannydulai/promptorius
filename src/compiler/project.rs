//! Manages the persistent cargo project used to compile generated Rust.

use crate::compiler::CompileError;
use std::path::{Path, PathBuf};
use std::process::Command;

const CARGO_TOML: &str = r#"[package]
name = "promptorius-output"
version = "0.1.0"
edition = "2021"

[dependencies]
git2 = "0.19"
starship-battery = "0.10"
glob = "0.3"
libc = "0.2"
regex = "1"

[profile.release]
opt-level = 2
lto = "thin"
strip = true
"#;

/// Get the build directory path.
pub fn build_dir() -> PathBuf {
    super::data_dir().join("build")
}

/// Ensure the cargo project exists with the right Cargo.toml.
pub fn ensure_build_project() -> Result<PathBuf, CompileError> {
    let dir = build_dir();
    let src_dir = dir.join("src");

    std::fs::create_dir_all(&src_dir)?;

    // Write Cargo.toml (overwrite to ensure deps are up to date)
    let cargo_path = dir.join("Cargo.toml");
    let needs_write = if cargo_path.exists() {
        let existing = std::fs::read_to_string(&cargo_path).unwrap_or_default();
        existing != CARGO_TOML
    } else {
        true
    };

    if needs_write {
        std::fs::write(&cargo_path, CARGO_TOML)?;
    }

    Ok(dir)
}

/// Write the generated Rust source into the build project.
pub fn write_source(build_dir: &Path, source: &str) -> Result<(), CompileError> {
    let main_rs = build_dir.join("src").join("main.rs");
    std::fs::write(&main_rs, source)?;
    Ok(())
}

/// Run cargo build --release in the build project.
pub fn build(build_dir: &Path) -> Result<(), CompileError> {
    let output = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .current_dir(build_dir)
        .output()?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(CompileError::Build(stderr.to_string()));
    }

    Ok(())
}

/// Copy the compiled binary to the output path.
pub fn copy_binary(build_dir: &Path, output_path: &Path) -> Result<(), CompileError> {
    let built = build_dir
        .join("target")
        .join("release")
        .join("promptorius-output");

    if !built.exists() {
        return Err(CompileError::Build(format!(
            "binary not found at {}",
            built.display()
        )));
    }

    if let Some(parent) = output_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    std::fs::copy(&built, output_path)?;

    // Make executable on unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(output_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(output_path, perms)?;
    }

    Ok(())
}
