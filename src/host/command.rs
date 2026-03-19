//! Host API: command execution (exec, exec_ok).

use rhai::{Array, Engine};
use std::process::Command;

pub fn register(engine: &mut Engine) {
    engine.register_fn("exec", |cmd: &str, args: Array| -> String {
        let string_args: Vec<String> = args
            .into_iter()
            .map(|a| a.to_string())
            .collect();

        let output = Command::new(cmd)
            .args(&string_args)
            .output();

        match output {
            Ok(out) => String::from_utf8_lossy(&out.stdout).trim().to_string(),
            Err(_) => String::new(),
        }
    });

    engine.register_fn("exec_ok", |cmd: &str, args: Array| -> bool {
        let string_args: Vec<String> = args
            .into_iter()
            .map(|a| a.to_string())
            .collect();

        Command::new(cmd)
            .args(&string_args)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false)
    });
}
