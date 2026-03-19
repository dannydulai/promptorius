//! Host API: filesystem functions (file_exists, read_file, glob_files, find_upward).

use rhai::{Array, Dynamic, Engine};
use std::path::Path;

pub fn register(engine: &mut Engine) {
    engine.register_fn("file_exists", |path: &str| -> bool {
        Path::new(path).exists()
    });

    engine.register_fn("read_file", |path: &str| -> String {
        std::fs::read_to_string(path)
            .map(|s| {
                // Cap at 64KB
                if s.len() > 65536 {
                    s[..65536].to_string()
                } else {
                    s
                }
            })
            .unwrap_or_default()
    });

    engine.register_fn("glob_files", |pattern: &str| -> Array {
        glob::glob(pattern)
            .map(|paths| {
                paths
                    .filter_map(|p| p.ok())
                    .map(|p| Dynamic::from(p.to_string_lossy().into_owned()))
                    .collect()
            })
            .unwrap_or_default()
    });

    engine.register_fn("find_upward", |filename: &str| -> String {
        let mut dir = std::env::current_dir().ok();
        while let Some(d) = dir {
            let candidate = d.join(filename);
            if candidate.exists() {
                return candidate.to_string_lossy().into_owned();
            }
            dir = d.parent().map(|p| p.to_path_buf());
        }
        String::new()
    });
}
