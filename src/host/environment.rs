//! Host API: environment functions (env, env_set, cwd, os).

use rhai::Engine;

pub fn register(engine: &mut Engine) {
    engine.register_fn("env", |name: &str| -> String {
        std::env::var(name).unwrap_or_default()
    });

    engine.register_fn("env_set", |name: &str, value: &str| {
        std::env::set_var(name, value);
    });

    engine.register_fn("cwd", || -> String {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default()
    });

    engine.register_fn("os", || -> String {
        if cfg!(target_os = "macos") {
            "macos".to_string()
        } else if cfg!(target_os = "linux") {
            "linux".to_string()
        } else if cfg!(target_os = "windows") {
            "windows".to_string()
        } else {
            "unknown".to_string()
        }
    });
}
