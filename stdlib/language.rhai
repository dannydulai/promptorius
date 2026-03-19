// Detect project language and show version
let result = "";

if find_upward("Cargo.toml") != "" {
    let v = exec("rustc", ["--version"]);
    let parts = v.split(" ");
    if parts.len() >= 2 {
        result = "🦀 " + parts[1];
    }
} else if find_upward("package.json") != "" {
    let v = exec("node", ["--version"]);
    result = " " + v;
} else if find_upward("pyproject.toml") != "" || find_upward("setup.py") != "" {
    let v = exec("python3", ["--version"]);
    let parts = v.split(" ");
    if parts.len() >= 2 {
        result = "🐍 " + parts[1];
    }
}

result
