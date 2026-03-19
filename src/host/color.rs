//! Host API: color() and icon() functions.

use crate::config::ColorDef;
use crate::render;
use rhai::Engine;
use std::collections::HashMap;

pub fn register(engine: &mut Engine, colors: &HashMap<String, ColorDef>) {
    let colors: HashMap<String, ColorDef> = colors.clone();

    engine.register_fn("color", move |name: &str| -> String {
        if name.is_empty() {
            return render::ansi_reset();
        }
        match colors.get(name) {
            Some(def) => render::color_to_ansi(def),
            None => String::new(),
        }
    });

}
