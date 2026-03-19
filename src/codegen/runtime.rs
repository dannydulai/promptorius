/// The Rust runtime code embedded in every compiled binary.
/// This provides the Value type, coercion, built-in functions, and scope management.
pub const RUNTIME: &str = r##"
use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};
use std::thread;

// ============================================================
// Value type
// ============================================================

#[derive(Clone)]
enum Value {
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Array(Vec<Value>),
    Dict(HashMap<String, Value>),
    Regex(regex::Regex, String), // (compiled, flags)
    Closure(Arc<dyn Fn(Vec<Value>) -> Value + Send + Sync>),
    Future(Arc<Mutex<Option<Value>>>), // result of spawn()
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Number(n) => write!(f, "{n}"),
            Value::Str(s) => write!(f, "\"{s}\""),
            Value::Array(a) => write!(f, "{a:?}"),
            Value::Dict(d) => write!(f, "{d:?}"),
            Value::Regex(_, fl) => write!(f, "/<regex>/{fl}"),
            Value::Closure(_) => write!(f, "<closure>"),
            Value::Future(_) => write!(f, "<future>"),
        }
    }
}

impl Value {
    fn to_str(&self) -> String {
        match self {
            Value::Null => String::new(),
            Value::Bool(b) => if *b { "true".to_string() } else { "false".to_string() },
            Value::Number(n) => {
                if *n == (*n as i64) as f64 && n.is_finite() {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
            Value::Str(s) => s.clone(),
            Value::Array(a) => {
                let parts: Vec<String> = a.iter().map(|v| v.to_str()).collect();
                parts.join(",")
            }
            Value::Dict(_) => "[object Dict]".to_string(),
            Value::Regex(_, _) => "[object Regex]".to_string(),
            Value::Closure(_) => "[object Closure]".to_string(),
            Value::Future(_) => "[object Future]".to_string(),
        }
    }

    fn to_num(&self) -> f64 {
        match self {
            Value::Null => 0.0,
            Value::Bool(b) => if *b { 1.0 } else { 0.0 },
            Value::Number(n) => *n,
            Value::Str(s) => s.trim().parse::<f64>().unwrap_or(0.0),
            _ => 0.0,
        }
    }

    fn to_bool(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Number(n) => *n != 0.0 && !n.is_nan(),
            Value::Str(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            _ => true,
        }
    }

    fn to_array(&self) -> Vec<Value> {
        match self {
            Value::Array(a) => a.clone(),
            Value::Null => vec![],
            other => vec![other.clone()],
        }
    }

    fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Number(_) => "number",
            Value::Str(_) => "string",
            Value::Array(_) => "array",
            Value::Dict(_) => "dict",
            Value::Regex(_, _) => "regex",
            Value::Closure(_) => "closure",
            Value::Future(_) => "future",
        }
    }
}

// ============================================================
// Operators (JS-like coercion)
// ============================================================

fn value_add(a: &Value, b: &Value) -> Value {
    // String wins in concatenation
    if matches!(a, Value::Str(_)) || matches!(b, Value::Str(_)) {
        return Value::Str(format!("{}{}", a.to_str(), b.to_str()));
    }
    Value::Number(a.to_num() + b.to_num())
}

fn value_sub(a: &Value, b: &Value) -> Value { Value::Number(a.to_num() - b.to_num()) }
fn value_mul(a: &Value, b: &Value) -> Value { Value::Number(a.to_num() * b.to_num()) }
fn value_div(a: &Value, b: &Value) -> Value { Value::Number(a.to_num() / b.to_num()) }
fn value_mod(a: &Value, b: &Value) -> Value { Value::Number(a.to_num() % b.to_num()) }
fn value_neg(a: &Value) -> Value { Value::Number(-a.to_num()) }

fn value_eq(a: &Value, b: &Value) -> Value {
    Value::Bool(values_equal_coerced(a, b))
}

fn value_neq(a: &Value, b: &Value) -> Value {
    Value::Bool(!values_equal_coerced(a, b))
}

fn value_strict_eq(a: &Value, b: &Value) -> Value {
    Value::Bool(values_strict_equal(a, b))
}

fn value_strict_neq(a: &Value, b: &Value) -> Value {
    Value::Bool(!values_strict_equal(a, b))
}

fn values_equal_coerced(a: &Value, b: &Value) -> bool {
    // null == null, null == false
    if a.is_null() && b.is_null() { return true; }
    if a.is_null() { return !b.to_bool(); }
    if b.is_null() { return !a.to_bool(); }

    match (a, b) {
        (Value::Bool(_), _) | (_, Value::Bool(_)) => a.to_bool() == b.to_bool(),
        (Value::Number(_), _) | (_, Value::Number(_)) => a.to_num() == b.to_num(),
        (Value::Str(sa), Value::Str(sb)) => sa == sb,
        _ => false,
    }
}

fn values_strict_equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Null, Value::Null) => true,
        (Value::Bool(a), Value::Bool(b)) => a == b,
        (Value::Number(a), Value::Number(b)) => a == b,
        (Value::Str(a), Value::Str(b)) => a == b,
        _ => false,
    }
}

fn value_lt(a: &Value, b: &Value) -> Value { Value::Bool(a.to_num() < b.to_num()) }
fn value_gt(a: &Value, b: &Value) -> Value { Value::Bool(a.to_num() > b.to_num()) }
fn value_lte(a: &Value, b: &Value) -> Value { Value::Bool(a.to_num() <= b.to_num()) }
fn value_gte(a: &Value, b: &Value) -> Value { Value::Bool(a.to_num() >= b.to_num()) }

// ============================================================
// Scope
// ============================================================

#[derive(Clone)]
struct Scope {
    vars: HashMap<String, Value>,
    parent: Option<Box<Scope>>,
}

impl Scope {
    fn new() -> Self {
        Self { vars: HashMap::new(), parent: None }
    }

    fn child(&self) -> Self {
        // Shallow clone — child gets a copy of parent's vars for reading
        // We use a flat model: clone all vars from parent into child,
        // so mutations to parent vars in child propagate back via the
        // generated code calling scope.set() on the right scope.
        Self { vars: self.vars.clone(), parent: None }
    }

    fn get(&self, name: &str) -> Value {
        self.vars.get(name).cloned().unwrap_or(Value::Null)
    }

    fn set(&mut self, name: &str, value: Value) {
        self.vars.insert(name.to_string(), value);
    }
}

// ============================================================
// Colors
// ============================================================

static mut COLOR_MAP: Option<HashMap<String, String>> = None;

fn builtin_colors(val: &Value) {
    if let Value::Dict(d) = val {
        let mut map = HashMap::new();
        for (name, def) in d {
            map.insert(name.clone(), color_def_to_ansi(def));
        }
        unsafe { COLOR_MAP = Some(map); }
    }
}

fn builtin_c(name: &Value) -> Value {
    let name = name.to_str();
    if name.is_empty() {
        return Value::Str("\x1b[0m".to_string());
    }
    let ansi = unsafe {
        COLOR_MAP.as_ref().and_then(|m| m.get(&name)).cloned().unwrap_or_default()
    };
    Value::Str(ansi)
}

fn color_def_to_ansi(def: &Value) -> String {
    match def {
        Value::Str(s) => {
            parse_color_to_sgr(s, true)
        }
        Value::Dict(d) => {
            let mut codes = Vec::new();
            if let Some(fg) = d.get("fg") {
                codes.push(parse_color_to_sgr_code(&fg.to_str(), true));
            }
            if let Some(bg) = d.get("bg") {
                codes.push(parse_color_to_sgr_code(&bg.to_str(), false));
            }
            if d.get("bold").map(|v| v.to_bool()).unwrap_or(false) { codes.push("1".to_string()); }
            if d.get("italic").map(|v| v.to_bool()).unwrap_or(false) { codes.push("3".to_string()); }
            if d.get("dim").map(|v| v.to_bool()).unwrap_or(false) { codes.push("2".to_string()); }
            if d.get("strikethrough").map(|v| v.to_bool()).unwrap_or(false) { codes.push("9".to_string()); }
            if let Some(ul) = d.get("underline") {
                match ul.to_str().as_str() {
                    "single" | "true" => codes.push("4".to_string()),
                    "double" => codes.push("4:2".to_string()),
                    "curly" => codes.push("4:3".to_string()),
                    "dotted" => codes.push("4:4".to_string()),
                    "dashed" => codes.push("4:5".to_string()),
                    _ => if ul.to_bool() { codes.push("4".to_string()); }
                }
            }
            if let Some(uc) = d.get("underline_color") {
                let (r, g, b) = color_to_rgb(&uc.to_str());
                codes.push(format!("58;2;{r};{g};{b}"));
            }
            if codes.is_empty() { String::new() } else { format!("\x1b[{}m", codes.join(";")) }
        }
        _ => String::new(),
    }
}

/// Convert any color string to RGB (for underline_color which requires truecolor).
fn color_to_rgb(s: &str) -> (u8, u8, u8) {
    let s = s.trim();
    if let Some(hex) = s.strip_prefix('#') {
        return match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0) * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0) * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(0) * 17;
                (r, g, b)
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                (r, g, b)
            }
            _ => (255, 255, 255),
        };
    }
    match s {
        "black" => (0, 0, 0), "red" => (205, 0, 0), "green" => (0, 205, 0),
        "yellow" => (205, 205, 0), "blue" => (0, 0, 238), "magenta" | "purple" => (205, 0, 205),
        "cyan" => (0, 205, 205), "white" => (229, 229, 229),
        "bright black" => (127, 127, 127), "bright red" => (255, 0, 0),
        "bright green" => (0, 255, 0), "bright yellow" => (255, 255, 0),
        "bright blue" => (92, 92, 255), "bright magenta" | "bright purple" => (255, 0, 255),
        "bright cyan" => (0, 255, 255), "bright white" => (255, 255, 255),
        _ => (255, 255, 255),
    }
}

/// Parse a color string and return the full SGR escape sequence.
fn parse_color_to_sgr(s: &str, is_fg: bool) -> String {
    let code = parse_color_to_sgr_code(s, is_fg);
    if code.is_empty() { String::new() } else { format!("\x1b[{code}m") }
}

/// Parse a color string and return just the SGR code (no \x1b[ or m).
fn parse_color_to_sgr_code(s: &str, is_fg: bool) -> String {
    let s = s.trim();

    // Hex colors → truecolor
    if let Some(hex) = s.strip_prefix('#') {
        let (r, g, b) = match hex.len() {
            3 => {
                let r = u8::from_str_radix(&hex[0..1], 16).unwrap_or(0) * 17;
                let g = u8::from_str_radix(&hex[1..2], 16).unwrap_or(0) * 17;
                let b = u8::from_str_radix(&hex[2..3], 16).unwrap_or(0) * 17;
                (r, g, b)
            }
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
                let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
                let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
                (r, g, b)
            }
            _ => (255, 255, 255),
        };
        return if is_fg { format!("38;2;{r};{g};{b}") } else { format!("48;2;{r};{g};{b}") };
    }

    // Named colors → standard ANSI codes
    let (base_fg, base_bg) = match s {
        "black"          => (30, 40),
        "red"            => (31, 41),
        "green"          => (32, 42),
        "yellow"         => (33, 43),
        "blue"           => (34, 44),
        "magenta" | "purple" => (35, 45),
        "cyan"           => (36, 46),
        "white"          => (37, 47),
        "bright black"   => (90, 100),
        "bright red"     => (91, 101),
        "bright green"   => (92, 102),
        "bright yellow"  => (93, 103),
        "bright blue"    => (94, 104),
        "bright magenta" | "bright purple" => (95, 105),
        "bright cyan"    => (96, 106),
        "bright white"   => (97, 107),
        _ => return if is_fg { "37".to_string() } else { "47".to_string() }, // default white
    };
    (if is_fg { base_fg } else { base_bg }).to_string()
}

// ============================================================
// Built-in functions
// ============================================================

fn builtin_env(name: &Value) -> Value {
    Value::Str(std::env::var(name.to_str()).unwrap_or_default())
}

fn builtin_cwd() -> Value {
    Value::Str(std::env::current_dir()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_default())
}

fn builtin_os() -> Value {
    Value::Str(if cfg!(target_os = "macos") { "macos" }
        else if cfg!(target_os = "linux") { "linux" }
        else if cfg!(target_os = "windows") { "windows" }
        else { "unknown" }.to_string())
}

fn builtin_eprint(msg: &Value) {
    eprintln!("{}", msg.to_str());
}

fn builtin_exec(cmd: &Value, args: &Value) -> Value {
    let cmd_str = cmd.to_str();
    let arg_vec: Vec<String> = args.to_array().iter().map(|a| a.to_str()).collect();
    match std::process::Command::new(&cmd_str).args(&arg_vec).output() {
        Ok(out) => Value::Str(String::from_utf8_lossy(&out.stdout).trim().to_string()),
        Err(_) => Value::Null,
    }
}

fn builtin_exec_ok(cmd: &Value, args: &Value) -> Value {
    let cmd_str = cmd.to_str();
    let arg_vec: Vec<String> = args.to_array().iter().map(|a| a.to_str()).collect();
    Value::Bool(std::process::Command::new(&cmd_str).args(&arg_vec).output()
        .map(|o| o.status.success()).unwrap_or(false))
}

fn builtin_regex(pattern: &Value, flags: &Value) -> Value {
    let p = pattern.to_str();
    let f = flags.to_str();
    let pat = if f.contains('i') { format!("(?i){p}") } else { p };
    match regex::Regex::new(&pat) {
        Ok(re) => Value::Regex(re, f),
        Err(_) => Value::Null,
    }
}

// --- Type coercion functions ---
fn builtin_string(val: &Value) -> Value { Value::Str(val.to_str()) }
fn builtin_number(val: &Value) -> Value { Value::Number(val.to_num()) }
fn builtin_array(val: &Value) -> Value { Value::Array(val.to_array()) }
fn builtin_dict(val: &Value) -> Value {
    match val {
        Value::Dict(_) => val.clone(),
        Value::Null => Value::Dict(HashMap::new()),
        Value::Array(arr) => {
            let mut map = HashMap::new();
            for item in arr {
                if let Value::Array(pair) = item {
                    if pair.len() >= 2 {
                        map.insert(pair[0].to_str(), pair[1].clone());
                    }
                }
            }
            Value::Dict(map)
        }
        _ => Value::Dict(HashMap::new()),
    }
}

// --- File operations ---
fn builtin_file_exists(path: &Value) -> Value {
    Value::Bool(std::path::Path::new(&path.to_str()).exists())
}

fn builtin_file_read(path: &Value) -> Value {
    match std::fs::read_to_string(path.to_str()) {
        Ok(s) => Value::Str(if s.len() > 65536 { s[..65536].to_string() } else { s }),
        Err(_) => Value::Null,
    }
}

fn builtin_file_write(path: &Value, content: &Value) -> Value {
    let _ = std::fs::write(path.to_str(), content.to_str());
    Value::Null
}

fn builtin_file_type(path: &Value) -> Value {
    let path_str = path.to_str();
    let p = std::path::Path::new(&path_str);
    if !p.exists() { return Value::Str("none".to_string()); }
    if p.is_symlink() { return Value::Str("symlink".to_string()); }
    if p.is_dir() { return Value::Str("dir".to_string()); }
    Value::Str("file".to_string())
}

// --- Dir operations ---
fn builtin_dir_search(pattern: &Value) -> Value {
    let arr: Vec<Value> = glob::glob(&pattern.to_str())
        .map(|paths| paths.filter_map(|p| p.ok())
            .map(|p| Value::Str(p.to_string_lossy().into_owned()))
            .collect())
        .unwrap_or_default();
    Value::Array(arr)
}

fn builtin_dir_search_upwards(name: &Value) -> Value {
    let filename = name.to_str();
    let mut dir = std::env::current_dir().ok();
    while let Some(d) = dir {
        let candidate = d.join(&filename);
        if candidate.exists() {
            return Value::Str(candidate.to_string_lossy().into_owned());
        }
        dir = d.parent().map(|p| p.to_path_buf());
    }
    Value::Str(String::new())
}

// --- Git (via libgit2) ---
fn builtin_git_is_repo() -> Value {
    Value::Bool(git2::Repository::discover(".").is_ok())
}

fn builtin_git_branch() -> Value {
    let repo = match git2::Repository::discover(".") {
        Ok(r) => r, Err(_) => return Value::Str(String::new()),
    };
    if let Ok(head) = repo.head() {
        if let Some(name) = head.shorthand() {
            return Value::Str(name.to_string());
        }
        if let Some(oid) = head.target() {
            let hex = oid.to_string();
            return Value::Str(hex[..7.min(hex.len())].to_string());
        }
    }
    Value::Str(String::new())
}

fn builtin_git_root() -> Value {
    Value::Str(git2::Repository::discover(".")
        .ok().and_then(|r| r.workdir().map(|p| {
            let s = p.to_string_lossy().into_owned();
            s.strip_suffix('/').unwrap_or(&s).to_string()
        }))
        .unwrap_or_default())
}

fn builtin_git_origin() -> Value {
    let repo = match git2::Repository::discover(".") {
        Ok(r) => r, Err(_) => return Value::Str(String::new()),
    };
    let url = repo.find_remote("origin")
        .ok()
        .and_then(|r| r.url().map(|s| s.to_string()));
    Value::Str(url.unwrap_or_default())
}

fn builtin_git_status() -> Value {
    let mut map = HashMap::new();
    for k in &["modified", "staged", "untracked", "conflicts", "ahead", "behind"] {
        map.insert(k.to_string(), Value::Number(0.0));
    }
    let repo = match git2::Repository::discover(".") {
        Ok(r) => r, Err(_) => return Value::Dict(map),
    };
    let statuses = match repo.statuses(None) {
        Ok(s) => s, Err(_) => return Value::Dict(map),
    };
    let (mut modified, mut staged, mut untracked, mut conflicts) = (0.0, 0.0, 0.0, 0.0);
    for entry in statuses.iter() {
        let s = entry.status();
        if s.is_conflicted() { conflicts += 1.0; }
        else if s.is_wt_new() { untracked += 1.0; }
        else {
            if s.intersects(git2::Status::INDEX_NEW | git2::Status::INDEX_MODIFIED
                | git2::Status::INDEX_DELETED | git2::Status::INDEX_RENAMED
                | git2::Status::INDEX_TYPECHANGE) { staged += 1.0; }
            if s.intersects(git2::Status::WT_MODIFIED | git2::Status::WT_DELETED
                | git2::Status::WT_TYPECHANGE | git2::Status::WT_RENAMED) { modified += 1.0; }
        }
    }
    map.insert("modified".to_string(), Value::Number(modified));
    map.insert("staged".to_string(), Value::Number(staged));
    map.insert("untracked".to_string(), Value::Number(untracked));
    map.insert("conflicts".to_string(), Value::Number(conflicts));
    if let Ok(head) = repo.head() {
        if let Some(local_oid) = head.target() {
            if let Ok(branch) = repo.find_branch(head.shorthand().unwrap_or(""), git2::BranchType::Local) {
                if let Ok(upstream) = branch.upstream() {
                    if let Some(upstream_oid) = upstream.get().target() {
                        if let Ok((ahead, behind)) = repo.graph_ahead_behind(local_oid, upstream_oid) {
                            map.insert("ahead".to_string(), Value::Number(ahead as f64));
                            map.insert("behind".to_string(), Value::Number(behind as f64));
                        }
                    }
                }
            }
        }
    }
    Value::Dict(map)
}

// --- Battery ---
fn builtin_battery_pct() -> Value {
    let manager = match starship_battery::Manager::new() {
        Ok(m) => m, Err(_) => return Value::Number(-1.0),
    };
    match manager.batteries() {
        Ok(mut batts) => match batts.next() {
            Some(Ok(b)) => Value::Number((b.state_of_charge().value * 100.0) as f64),
            _ => Value::Number(-1.0),
        },
        Err(_) => Value::Number(-1.0),
    }
}

fn builtin_battery_state() -> Value {
    let manager = match starship_battery::Manager::new() {
        Ok(m) => m, Err(_) => return Value::Str("none".to_string()),
    };
    match manager.batteries() {
        Ok(mut batts) => match batts.next() {
            Some(Ok(b)) => Value::Str(match b.state() {
                starship_battery::State::Charging => "charging",
                starship_battery::State::Discharging => "discharging",
                starship_battery::State::Full => "full",
                starship_battery::State::Empty => "empty",
                _ => "unknown",
            }.to_string()),
            _ => Value::Str("none".to_string()),
        },
        Err(_) => Value::Str("none".to_string()),
    }
}

fn builtin_battery_time() -> Value {
    let manager = match starship_battery::Manager::new() {
        Ok(m) => m, Err(_) => return Value::Number(-1.0),
    };
    match manager.batteries() {
        Ok(mut batts) => match batts.next() {
            Some(Ok(b)) => {
                let time = match b.state() {
                    starship_battery::State::Discharging => b.time_to_empty(),
                    starship_battery::State::Charging => b.time_to_full(),
                    _ => None,
                };
                time.map(|t| Value::Number(t.value as f64)).unwrap_or(Value::Number(-1.0))
            },
            _ => Value::Number(-1.0),
        },
        Err(_) => Value::Number(-1.0),
    }
}

// --- Concurrency ---
fn builtin_spawn(closure: &Value) -> Value {
    let result = Arc::new(Mutex::new(None));
    let result_clone = result.clone();
    if let Value::Closure(f) = closure {
        let f = f.clone();
        thread::spawn(move || {
            let val = f(vec![]);
            *result_clone.lock().unwrap() = Some(val);
        });
    }
    Value::Future(result)
}

fn builtin_wait(input: &Value) -> Value {
    match input {
        Value::Dict(d) => {
            let mut results = HashMap::new();
            for (key, val) in d {
                if let Value::Future(result) = val {
                    loop {
                        let guard = result.lock().unwrap();
                        if guard.is_some() {
                            results.insert(key.clone(), guard.clone().unwrap());
                            break;
                        }
                        drop(guard);
                        thread::yield_now();
                    }
                } else {
                    results.insert(key.clone(), val.clone());
                }
            }
            Value::Dict(results)
        }
        Value::Array(arr) => {
            let mut results = Vec::new();
            for f in arr {
                if let Value::Future(result) = f {
                    loop {
                        let guard = result.lock().unwrap();
                        if guard.is_some() {
                            results.push(guard.clone().unwrap());
                            break;
                        }
                        drop(guard);
                        thread::yield_now();
                    }
                } else {
                    results.push(f.clone());
                }
            }
            Value::Array(results)
        }
        _ => input.clone(),
    }
}

// --- Method dispatch ---
fn value_method_call(obj: &Value, method: &str, args: Vec<Value>) -> Value {
    match obj {
        Value::Str(s) => match method {
            "len" => Value::Number(s.len() as f64),
            "trim" => Value::Str(s.trim().to_string()),
            "starts_with" => Value::Bool(args.first().map(|a| s.starts_with(&a.to_str())).unwrap_or(false)),
            "ends_with" => Value::Bool(args.first().map(|a| s.ends_with(&a.to_str())).unwrap_or(false)),
            "contains" => Value::Bool(args.first().map(|a| s.contains(&a.to_str())).unwrap_or(false)),
            "to_upper" => Value::Str(s.to_uppercase()),
            "to_lower" => Value::Str(s.to_lowercase()),
            "to_number" => Value::Number(s.parse::<f64>().unwrap_or(0.0)),
            "repeat" => {
                let n = args.first().map(|a| a.to_num() as usize).unwrap_or(1);
                Value::Str(s.repeat(n))
            }
            "pad_start" => {
                let width = args.first().map(|a| a.to_num() as usize).unwrap_or(0);
                let fill = args.get(1).map(|a| a.to_str()).unwrap_or_else(|| " ".to_string());
                let fill_char = fill.chars().next().unwrap_or(' ');
                if s.len() >= width {
                    Value::Str(s.clone())
                } else {
                    let padding: String = std::iter::repeat(fill_char).take(width - s.len()).collect();
                    Value::Str(format!("{padding}{s}"))
                }
            }
            "replace" => {
                let from = args.first().cloned().unwrap_or(Value::Null);
                let to = args.get(1).map(|a| a.to_str()).unwrap_or_default();
                match &from {
                    Value::Regex(re, _) => Value::Str(re.replace_all(s, to.as_str()).to_string()),
                    _ => Value::Str(s.replace(&from.to_str(), &to)),
                }
            }
            "split" => {
                let sep = args.first().cloned().unwrap_or(Value::Null);
                let parts: Vec<Value> = match &sep {
                    Value::Regex(re, _) => re.split(s).map(|p| Value::Str(p.to_string())).collect(),
                    _ => s.split(&sep.to_str()).map(|p| Value::Str(p.to_string())).collect(),
                };
                Value::Array(parts)
            }
            "slice" => {
                let start = args.first().map(|a| a.to_num() as usize).unwrap_or(0);
                let end = args.get(1)
                    .filter(|a| !a.is_null())
                    .map(|a| a.to_num() as usize)
                    .unwrap_or(s.len());
                let start = start.min(s.len());
                let end = end.min(s.len());
                Value::Str(s[start..end].to_string())
            }
            _ => Value::Null,
        },
        Value::Array(arr) => match method {
            "len" => Value::Number(arr.len() as f64),
            "push" => {
                let mut a = arr.clone();
                if let Some(val) = args.into_iter().next() { a.push(val); }
                Value::Array(a)
            }
            "pop" => {
                let mut a = arr.clone();
                a.pop().unwrap_or(Value::Null)
            }
            "shift" => {
                if arr.is_empty() { Value::Null } else {
                    let mut a = arr.clone();
                    a.remove(0)
                }
            }
            "map" => {
                if let Some(Value::Closure(f)) = args.first() {
                    Value::Array(arr.iter().map(|v| f(vec![v.clone()])).collect())
                } else { obj.clone() }
            }
            "filter" => {
                if let Some(Value::Closure(f)) = args.first() {
                    Value::Array(arr.iter().filter(|v| f(vec![(*v).clone()]).to_bool()).cloned().collect())
                } else { obj.clone() }
            }
            "reduce" => {
                let closure = args.first().cloned();
                let init = args.get(1).cloned().unwrap_or(Value::Null);
                if let Some(Value::Closure(f)) = closure {
                    arr.iter().fold(init, |acc, v| f(vec![acc, v.clone()]))
                } else { init }
            }
            _ => Value::Null,
        },
        Value::Dict(d) => match method {
            "len" => Value::Number(d.len() as f64),
            "keys" => Value::Array(d.keys().map(|k| Value::Str(k.clone())).collect()),
            "values" => Value::Array(d.values().cloned().collect()),
            "get" => {
                let key = args.first().map(|a| a.to_str()).unwrap_or_default();
                d.get(&key).cloned().unwrap_or(Value::Null)
            }
            _ => Value::Null,
        },
        Value::Regex(re, flags) => match method {
            "test" => {
                let s = args.first().map(|a| a.to_str()).unwrap_or_default();
                Value::Bool(re.is_match(&s))
            }
            "exec" => {
                let s = args.first().map(|a| a.to_str()).unwrap_or_default();
                match re.captures(&s) {
                    Some(caps) => {
                        let arr: Vec<Value> = (0..caps.len())
                            .map(|i| caps.get(i).map(|m| Value::Str(m.as_str().to_string())).unwrap_or(Value::Null))
                            .collect();
                        Value::Array(arr)
                    }
                    None => Value::Null,
                }
            }
            _ => Value::Null,
        },
        _ => Value::Null,
    }
}

fn value_member_access(obj: &Value, field: &str) -> Value {
    match obj {
        Value::Dict(d) => d.get(field).cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    }
}

fn value_set_member(obj: &mut Value, field: &str, val: Value) {
    if let Value::Dict(d) = obj {
        d.insert(field.to_string(), val);
    }
}

fn value_set_index(obj: &mut Value, idx: &Value, val: Value) {
    match obj {
        Value::Array(arr) => {
            let i = idx.to_num() as usize;
            if i < arr.len() {
                arr[i] = val;
            } else {
                while arr.len() < i { arr.push(Value::Null); }
                arr.push(val);
            }
        }
        Value::Dict(d) => {
            d.insert(idx.to_str(), val);
        }
        _ => {}
    }
}

fn value_index(obj: &Value, idx: &Value) -> Value {
    match obj {
        Value::Array(arr) => {
            let i = idx.to_num() as usize;
            arr.get(i).cloned().unwrap_or(Value::Null)
        }
        Value::Dict(d) => {
            let key = idx.to_str();
            d.get(&key).cloned().unwrap_or(Value::Null)
        }
        Value::Str(s) => {
            let i = idx.to_num() as usize;
            s.chars().nth(i).map(|c| Value::Str(c.to_string())).unwrap_or(Value::Null)
        }
        _ => Value::Null,
    }
}

// --- Shell escape wrapping ---
fn wrap_escapes_for_shell(s: &str, shell: &str) -> String {
    match shell {
        "zsh" => wrap_ansi_zsh(s),
        "bash" => wrap_ansi(s, "\\[", "\\]", false),
        _ => s.to_string(),
    }
}

fn wrap_ansi_zsh(s: &str) -> String {
    wrap_ansi(s, "%{", "%}", true)
}

fn wrap_ansi(s: &str, prefix: &str, suffix: &str, escape_pct: bool) -> String {
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
                if bytes[i] == b'm' { i += 1; break; }
                i += 1;
            }
            result.push_str(suffix);
        } else {
            let c = s[i..].chars().next().unwrap();
            if escape_pct && c == '%' {
                result.push_str("%%");
            } else {
                result.push(c);
            }
            i += c.len_utf8();
        }
    }
    result
}
"##;

/// The main() function template for compiled binaries.
pub const MAIN_FN: &str = r##"
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut scope = Scope::new();
    let mut right = false;

    // Parse --var name:value and --right
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--right" {
            right = true;
        } else if args[i] == "--var" && i + 1 < args.len() {
            i += 1;
            if let Some((name, value)) = args[i].split_once(':') {
                scope.set(name, Value::Str(value.to_string()));
            }
        }
        i += 1;
    }

    // Run top-level init code
    script_init(&mut scope);

    // Call the appropriate prompt function
    let result = if right {
        user_fn_right_prompt(&mut scope)
    } else {
        user_fn_left_prompt(&mut scope)
    };

    // Wrap ANSI escapes for shell compatibility
    let output = result.to_str();
    let shell = scope.get("shell").to_str();
    let output = if !shell.is_empty() {
        wrap_escapes_for_shell(&output, &shell)
    } else {
        output
    };

    print!("{output}");
}
"##;

/// Instrumented main() for explain — wraps each phase with timing.
pub const EXPLAIN_MAIN_FN: &str = r##"
fn main() {
    use std::time::Instant;

    let total_start = Instant::now();
    let args: Vec<String> = std::env::args().collect();
    let mut scope = Scope::new();
    let mut right = false;

    // Parse --var name:value and --right
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--right" {
            right = true;
        } else if args[i] == "--var" && i + 1 < args.len() {
            i += 1;
            if let Some((name, value)) = args[i].split_once(':') {
                scope.set(name, Value::Str(value.to_string()));
            }
        }
        i += 1;
    }

    // Time script_init
    let t = Instant::now();
    script_init(&mut scope);
    let init_us = t.elapsed().as_micros();

    // Time prompt function
    let t = Instant::now();
    let result = if right {
        user_fn_right_prompt(&mut scope)
    } else {
        user_fn_left_prompt(&mut scope)
    };
    let prompt_us = t.elapsed().as_micros();

    // Time shell escape wrapping
    let t = Instant::now();
    let output = result.to_str();
    let shell = scope.get("shell").to_str();
    let output = if !shell.is_empty() {
        wrap_escapes_for_shell(&output, &shell)
    } else {
        output
    };
    let wrap_us = t.elapsed().as_micros();

    let total_us = total_start.elapsed().as_micros();

    // Print timing breakdown
    println!("--- promptorius explain ---");
    println!();
    println!("  {:>7.2}ms  script_init", init_us as f64 / 1000.0);
    println!("  {:>7.2}ms  {} ", prompt_us as f64 / 1000.0,
        if right { "right_prompt()" } else { "left_prompt()" });
    println!("  {:>7.2}ms  shell escape wrapping", wrap_us as f64 / 1000.0);
    println!();
    println!("  {:>7.2}ms  total", total_us as f64 / 1000.0);
    println!("  {:>7}    output bytes", output.len());
    println!("---");
}
"##;
