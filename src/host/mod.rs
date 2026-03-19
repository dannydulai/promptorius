//! Host API functions registered into the Rhai engine.
//!
//! All Rhai-callable functions are defined and registered here.
//! Organized by API group as submodules.

pub mod cache;
pub mod color;
pub mod command;
pub mod environment;
pub mod filesystem;
pub mod git;

use crate::config::Config;
use crate::script::ScriptEngine;
use std::collections::HashMap;

/// Register all host API functions into the script engine.
pub fn register_all(engine: &mut ScriptEngine, config: &Config, cmds: &[CmdDef]) {
    let rhai = engine.engine_mut();

    environment::register(rhai);
    filesystem::register(rhai);
    command::register(rhai);
    git::register(rhai);
    color::register(rhai, &config.colors);
    cache::register(rhai);

    // Register --cmd defined functions
    for cmd in cmds {
        register_cmd(rhai, cmd);
    }
}

/// A function definition from `--cmd :type:name:value`.
#[derive(Debug, Clone)]
pub struct CmdDef {
    pub name: String,
    pub value: CmdValue,
}

#[derive(Debug, Clone)]
pub enum CmdValue {
    Str(String),
    Int(i64),
    Float(f64),
    Bool(bool),
}

/// Parse a `--cmd` argument string like `:int:exit_code:127`.
pub fn parse_cmd(s: &str) -> Option<CmdDef> {
    let s = s.strip_prefix(':')?;
    let (type_str, rest) = s.split_once(':')?;
    let (name, value_str) = rest.split_once(':')?;

    let value = match type_str {
        "str" => CmdValue::Str(value_str.to_string()),
        "int" => CmdValue::Int(value_str.parse().ok()?),
        "float" => CmdValue::Float(value_str.parse().ok()?),
        "bool" => CmdValue::Bool(value_str.parse().ok()?),
        _ => return None,
    };

    Some(CmdDef {
        name: name.to_string(),
        value,
    })
}

fn register_cmd(engine: &mut rhai::Engine, cmd: &CmdDef) {
    let name = cmd.name.clone();
    match &cmd.value {
        CmdValue::Str(v) => {
            let v = v.clone();
            engine.register_fn(&name, move || v.clone());
        }
        CmdValue::Int(v) => {
            let v = *v;
            engine.register_fn(&name, move || v);
        }
        CmdValue::Float(v) => {
            let v = *v;
            engine.register_fn(&name, move || v);
        }
        CmdValue::Bool(v) => {
            let v = *v;
            engine.register_fn(&name, move || v);
        }
    }
}

/// Create a Rhai scope with the segment's `config` map pre-populated.
pub fn segment_scope(extra: &HashMap<String, toml::Value>) -> rhai::Scope<'static> {
    let map = toml_to_rhai_map(extra);
    let mut scope = rhai::Scope::new();
    scope.push_constant("config", map);
    scope
}

fn toml_to_rhai_map(extra: &HashMap<String, toml::Value>) -> rhai::Map {
    let mut map = rhai::Map::new();
    for (k, v) in extra {
        let key = k.clone().into();
        let val = toml_value_to_dynamic(v);
        map.insert(key, val);
    }
    map
}

fn toml_value_to_dynamic(v: &toml::Value) -> rhai::Dynamic {
    match v {
        toml::Value::String(s) => rhai::Dynamic::from(s.clone()),
        toml::Value::Integer(n) => rhai::Dynamic::from(*n),
        toml::Value::Float(f) => rhai::Dynamic::from(*f),
        toml::Value::Boolean(b) => rhai::Dynamic::from(*b),
        _ => rhai::Dynamic::UNIT,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_cmd_int() {
        let cmd = parse_cmd(":int:exit_code:127").unwrap();
        assert_eq!(cmd.name, "exit_code");
        assert!(matches!(cmd.value, CmdValue::Int(127)));
    }

    #[test]
    fn parse_cmd_str() {
        let cmd = parse_cmd(":str:duration:12ms").unwrap();
        assert_eq!(cmd.name, "duration");
        assert!(matches!(cmd.value, CmdValue::Str(ref s) if s == "12ms"));
    }

    #[test]
    fn parse_cmd_bool() {
        let cmd = parse_cmd(":bool:is_ssh:true").unwrap();
        assert_eq!(cmd.name, "is_ssh");
        assert!(matches!(cmd.value, CmdValue::Bool(true)));
    }

    #[test]
    fn parse_cmd_invalid() {
        assert!(parse_cmd("garbage").is_none());
        assert!(parse_cmd(":int:name:notanumber").is_none());
    }
}
