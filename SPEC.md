# Promptorius

A compiled, scriptable shell prompt engine. Write your prompt in a simple scripting language, compile it to a native binary, get sub-millisecond rendering.

## Problem

Starship has ~100+ hardcoded modules. Promptorius v1 replaced them with Rhai scripts, but Rhai's engine initialization (~1ms) and interpretation overhead remained. This version eliminates the interpreter entirely: your prompt script compiles to native Rust, producing a standalone binary with zero runtime overhead.

## How it works

```
config.prompt          (your script)
       │
       ▼
promptorius compile    (parses script, generates Rust, builds binary)
       │
       ▼
__promptorius_output   (native binary in XDG_DATA_HOME, sub-ms execution)
```

1. You write `config.prompt` — a single script file in the Promptorius language.
2. `promptorius compile` transpiles it to Rust source and builds a native binary.
3. The shell calls `__promptorius_output` on every prompt render.
4. If the script or promptorius itself is newer than the binary, the shell integration auto-recompiles.

## The Promptorius Language

A dynamically-typed scripting language with JavaScript-like coercion, designed for prompt scripting.

### Basics

```
# This is a comment

x = 5
name = "world"
greeting = `hello {name}`    # backtick interpolation, no $ needed
```

- `#` comments to end of line
- Semicolons optional everywhere
- Bare assignment creates variables
- `null` keyword for unset values
- `true` / `false` booleans

### Strings

Three string types:

- `"double quoted"` — supports `\n`, `\t`, `\\`, `\"`, `\u{1F600}` escapes
- `'single quoted'` — supports same escapes
- `` `backtick` `` — interpolation with `{expr}`, use `{{` / `}}` for literal braces

All strings support inline unicode characters.

### Types

Dynamically typed. Values are: string, number (f64), bool, null, array, dict, regex, closure.

### Null coercion

`null` coerces naturally in all contexts:

| Context | null becomes |
|---|---|
| String | `""` (empty string) |
| Number | `0` |
| Boolean | `false` |

`null == false` is `true`. Use `===` / `!==` for strict comparison without coercion.

### Type coercion (JS-like)

- `"hello" + 5` → `"hello5"` (string wins in concatenation)
- `"5" * 2` → `10` (arithmetic coerces strings to numbers)
- `"" == false` → `true`
- `0 == false` → `true`
- `"0" == false` → `true`
- `null == false` → `true`
- `null === false` → `false`

### Operators

Arithmetic: `+`, `-`, `*`, `/`, `%`
Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
Strict: `===`, `!==`
Logical: `&&`, `||`, `!`
Assignment: `=`, `+=`, `-=`, `*=`, `/=`, `%=`
Ternary: `cond ? a : b`
Member: `.`, `[]`

### Control flow

```
if (condition) {
    # ...
} else if (other) {
    # ...
} else {
    # ...
}

while (condition) {
    # ...
}

for (item in array) {
    # ...
}

return value
```

### Functions

```
fn greet(name) {
    return `hello {name}`
}

# Closures / anonymous functions
doubled = arr.map(fn(x) { x * 2 })

# Short closure
filtered = arr.filter(fn(x) { x > 0 })
```

### Arrays

```
arr = [1, 2, 3]
arr.push(4)
arr.pop()
arr.shift()
arr.len()
arr.map(fn(x) { x * 2 })
arr.filter(fn(x) { x > 0 })
arr.reduce(fn(acc, x) { acc + x }, 0)
first = arr[0]
```

### Dicts

```
d = {
    name: "danny",
    age: 30,
    nested: { foo: "bar" },
}
d["name"]
d.name        # same thing
d.get("name") # same thing, returns null if missing
d.keys()
d.values()
d.len()
```

### Regular expressions

First-class regex literals, JavaScript-style:

```
re = /^hello/i
if (re.test("Hello World")) {
    # ...
}
match = re.exec("Hello World")
# match[0] is the full match, match[1]+ are capture groups

result = "hello world".replace(/world/, "promptorius")
parts = "a,b,c".split(/,/)
```

Flags: `i` (case-insensitive), `g` (global), `m` (multiline).

## Script structure

The script file is `$XDG_CONFIG_HOME/promptorius/config.prompt`.

```
# Colors — a dict of named colors.
# Each value is a string (fg only) or a dict with fg, bg, bold, italic, etc.
colors = {
    directory: "#6ec2e8",
    error: { fg: "red", bold: true },
    char_normal: "#666",
    char_vicmd: "#ff40c0",
    git_branch: "#e89050",
}
setcolors(colors)

# Settings
settings = {
    timeout: 50,
}

# Helper functions
fn git_prompt() {
    if (!git_is_repo()) { return "" }
    return color("git_branch") + " " + git_branch() + color("")
}

# Required: return the left prompt string
fn left_prompt() {
    dir = color("directory") + cwd_short() + color("")
    char = env("USER") == "root" ? "#" : "│"
    col = keymap === "vicmd" ? "char_vicmd" : "char_normal"

    result = ""
    if (exit_code != 0) {
        result += color("error") + "Exited w/ " + exit_code + color("") + "\n"
    }
    result += dir + " " + color(col) + char + color("") + " "
    return result
}

# Required: return the right prompt string
fn right_prompt() {
    return git_prompt()
}
```

## Built-in functions

### Environment

| Function | Returns | Description |
|---|---|---|
| `env(name)` | string | Get env var, `""` if unset |
| `env_set(name, val)` | null | Set env var for this render |
| `cwd()` | string | Current working directory |
| `cwd_short()` | string | cwd with `$HOME` replaced by `~` |
| `os()` | string | `"macos"`, `"linux"`, `"windows"` |
| `hostname()` | string | Machine hostname |
| `eprint(msg)` | null | Print to stderr (debug) |

### Filesystem

| Function | Returns | Description |
|---|---|---|
| `file_exists(path)` | bool | Check if file/dir exists |
| `read_file(path)` | string | Read file contents (capped 64KB) |
| `glob_files(pattern)` | array | Glob match from cwd |
| `find_upward(name)` | string | Walk up from cwd looking for file, `""` if not found |

### Command execution

| Function | Returns | Description |
|---|---|---|
| `exec(cmd, args)` | string | Run command, return stdout trimmed |
| `exec_ok(cmd, args)` | bool | Run command, true if exit 0 |

### Git (native via libgit2)

| Function | Returns | Description |
|---|---|---|
| `git_is_repo()` | bool | Whether cwd is inside a git repo |
| `git_branch()` | string | Branch name or short SHA if detached |
| `git_root()` | string | Root of the git repo |
| `git_status()` | dict | `{ modified, staged, untracked, conflicts, ahead, behind }` |

### Colors

| Function | Returns | Description |
|---|---|---|
| `setcolors(dict)` | null | Set the color palette |
| `color(name)` | string | Emit ANSI escape for named color, `""` to reset |

### Battery

| Function | Returns | Description |
|---|---|---|
| `battery_pct()` | number | Charge percentage, `-1` if no battery |
| `battery_state()` | string | `"charging"`, `"discharging"`, `"full"`, `"empty"`, `"none"` |
| `battery_time()` | number | Seconds remaining, `-1` if unavailable |

### String methods

| Method | Description |
|---|---|
| `s.len()` | String length |
| `s.trim()` | Strip whitespace |
| `s.starts_with(prefix)` | Bool |
| `s.ends_with(suffix)` | Bool |
| `s.contains(substr)` | Bool |
| `s.replace(from, to)` | Replace (from can be string or regex) |
| `s.split(sep)` | Split into array (sep can be string or regex) |
| `s.slice(start, end)` | Substring |
| `s.to_upper()` | Uppercase |
| `s.to_lower()` | Lowercase |
| `s.to_number()` | Parse as number |
| `s.repeat(n)` | Repeat n times |

## --var arguments

The compiled binary accepts `--var name:type:value` to inject global variables.

```
__promptorius_output --var exit_code:int:0 --var keymap:str:vicmd --var shell:str:zsh
```

Types: `str`, `int`, `float`, `bool`.

All variable names are valid — accessing an undefined variable returns `null` instead of an error. This means `--var` declarations are optional; scripts can check any variable name safely.

`--right` flag tells the binary to call `right_prompt()` instead of `left_prompt()`.

## Compilation

### `promptorius compile`

```
promptorius compile                    # compile config.prompt → __promptorius_output
promptorius compile <script> <output>  # compile specific script to specific output
```

What it does:
1. Parses the `.prompt` script into an AST
2. Generates a complete Rust source file containing:
   - The runtime (git, env, color, battery, exec, etc.) as inline Rust code
   - The compiled script logic as Rust functions
   - A `main()` that parses `--var` / `--right` and calls `left_prompt()` or `right_prompt()`
3. Builds the binary using a persistent cargo project in `$XDG_DATA_HOME/promptorius/build/`
   - First build: downloads and compiles dependencies (~30s one-time)
   - Subsequent builds: only recompiles the generated source (~1-2s incremental)
4. Copies the binary to `$XDG_DATA_HOME/promptorius/__promptorius_output`

### Staleness check

The binary is stale if:
- The script file is newer than the binary (mtime comparison)
- The `promptorius` compiler binary is newer than the output binary (runtime update)

### Dependencies (in the build project's Cargo.toml)

| Crate | Purpose |
|---|---|
| `git2` | Native git operations |
| `starship-battery` | Battery status |
| `glob` | File globbing |
| `regex` | Regular expressions |

## CLI

```
promptorius compile                     # Compile config.prompt → default output
promptorius compile <script> <output>   # Compile specific files
promptorius init <shell>                # Print shell init script
promptorius check                       # Validate script syntax
promptorius explain                     # Show timing breakdown
promptorius completions <shell>         # Generate shell completions

__promptorius_output                    # Render left prompt
__promptorius_output --right            # Render right prompt
__promptorius_output --var k:type:val   # Inject variables
```

## Shell integration

`promptorius init zsh` outputs a script that:

1. On `precmd`: checks if `__promptorius_output` is stale
2. If stale: runs `promptorius compile` (shows status messages during build)
3. Calls `__promptorius_output` with `--var` args for exit code, duration, keymap, jobs, shell
4. Sets `PROMPT` and `RPROMPT`

```zsh
promptorius_precmd() {
    local exit_code=$?
    # ... duration, jobs calculation ...

    # Auto-recompile if stale
    local script="$XDG_CONFIG_HOME/promptorius/config.prompt"
    local binary="$XDG_DATA_HOME/promptorius/__promptorius_output"
    local compiler="$(command -v promptorius)"

    if [[ ! -f "$binary" || "$script" -nt "$binary" || "$compiler" -nt "$binary" ]]; then
        promptorius compile
    fi

    local -a vars=(
        --var "exit_code:int:${exit_code}"
        --var "duration:int:${duration_ms}"
        --var "jobs:int:${job_count}"
        --var "keymap:str:${KEYMAP:-}"
        --var "shell:str:zsh"
        --var "shlvl:int:${SHLVL}"
    )
    PROMPT="$($binary "${vars[@]}")"
    RPROMPT="$($binary --right "${vars[@]}")"
}
```

## Default script

On first run, if `config.prompt` doesn't exist, promptorius creates it:

```
# Promptorius prompt script
# See: https://github.com/user/promptorius

# --- Colors ---
colors = {
    directory: "#6ec2e8",
    error: { fg: "red", bold: true },
    char_normal: "#666",
    char_vicmd: "#ff40c0",
    git_branch: "#e89050",
    # git_staged: "green",
    # git_modified: "yellow",
    # git_untracked: "red",
    # battery_normal: "green",
    # battery_low: "#ff8800",
    # battery_critical: { fg: "red", bold: true },
}
setcolors(colors)

# --- Left prompt ---
fn left_prompt() {
    result = ""

    # Show non-zero exit code
    if (exit_code != 0) {
        result += color("error") + "Exited w/ " + exit_code + color("") + "\n"
    }

    # Directory
    result += color("directory") + cwd_short() + color("")

    # Prompt character: │ for user, # for root, repeated SHLVL times
    char = env("USER") == "root" ? "#" : "│"
    col = keymap === "vicmd" ? "char_vicmd" : "char_normal"
    result += " " + color(col) + char.repeat(shlvl) + color("") + " "

    return result
}

# --- Right prompt ---
fn right_prompt() {
    if (!git_is_repo()) { return "" }
    return color("git_branch") + " " + git_branch() + color("")
}

# --- Uncomment below for more features ---

# fn git_prompt() {
#     if (!git_is_repo()) { return "" }
#     branch = git_branch()
#     s = git_status()
#     indicators = ""
#     if (s.staged > 0)    { indicators += color("git_staged") + " +" }
#     if (s.modified > 0)  { indicators += color("git_modified") + " ✎" }
#     if (s.untracked > 0) { indicators += color("git_untracked") + " ?" }
#     if (s.conflicts > 0) { indicators += color("error") + " ✘" }
#     if (indicators != "") { indicators += color("") }
#     remote = ""
#     if (s.ahead > 0)  { remote += " ↑" + s.ahead }
#     if (s.behind > 0) { remote += " ↓" + s.behind }
#     return color("git_branch") + " " + branch + indicators + remote + color("")
# }

# fn battery_prompt() {
#     pct = battery_pct()
#     if (pct < 0) { return "" }
#     state = battery_state()
#     if (state == "full") { return "" }
#     col = pct <= 10 ? "battery_critical" : pct <= 25 ? "battery_low" : "battery_normal"
#     icon = pct > 75 ? "█" : pct > 50 ? "▆" : pct > 25 ? "▄" : pct > 10 ? "▂" : "▁"
#     suffix = state == "charging" ? "⚡" : ""
#     return color(col) + icon + " " + pct + "%" + suffix + color("")
# }
```

## Non-goals

- **No interpreter at runtime.** The compiled binary has zero scripting overhead.
- **No plugin system.** One script file, compiled to one binary.
- **No package manager.** Copy-paste functions between scripts.
- **No Windows support for v1.** macOS and Linux only.

## Open questions

1. **WASM target?** Could compile to WASM for portability, but adds complexity.
2. **LSP / editor support?** Syntax highlighting and error checking for `.prompt` files.
3. **Hot reload?** Watch mode that recompiles on script change.
