# Promptorius

A compiled, scriptable shell prompt engine. Write your prompt in a simple scripting language, compile it to a native binary, get sub-millisecond rendering.

## Problem

Starship has ~100+ hardcoded modules. Promptorius v1 replaced them with Rhai scripts, but Rhai's engine initialization (~1ms) and interpretation overhead remained. This version eliminates the interpreter entirely: your prompt script compiles to native Rust, producing a standalone binary with zero runtime overhead.

## How it works

```
config                 (your script)
       │
       ▼
promptorius compile    (parses script, generates Rust, builds binary)
       │
       ▼
__promptorius_output   (native binary in XDG_DATA_HOME, sub-ms execution)
```

1. You write `$XDG_CONFIG_HOME/promptorius/config` — a single script file in the Promptorius language.
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
- `null ?? "default"` → `"default"` (null coalescing — returns right side only if left is `null`)

### Operators

Arithmetic: `+`, `-`, `*`, `/`, `%`
Comparison: `==`, `!=`, `<`, `>`, `<=`, `>=`
Strict: `===`, `!==`
Logical: `&&`, `||`, `!`
Null coalescing: `??`
Assignment: `=`, `+=`, `-=`, `*=`, `/=`, `%=`
Ternary: `cond ? a : b`
Range: `x..y` (for use in `for` loops)
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

for (i in 0..10) {
    # range-based, exclusive end
}

return value
```

### Scoping

Variables have lexical scope with global fallback:

- Variables created at the top level are **global** — visible everywhere.
- Variables created inside a function, loop, or block are **local** — scoped to that block.
- Reading a variable checks local scope first, then walks up to global scope.
- Assigning to an existing variable updates it in whatever scope it lives in.
- Assigning to a new name creates it in the **current local** scope.

```
x = 10                # global

fn example() {
    y = 20            # local to example()
    x = 30            # updates the global x
    eprint(x)         # 30
}

example()
eprint(x)             # 30
eprint(y)             # null — y is not in global scope
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

Create regex from string with `regex(pattern)` or `regex(pattern, flags)`:

```
pattern = "^hello"
re = regex(pattern, "i")
re.test("Hello World")   # true
```

## Script structure

The script file is `$XDG_CONFIG_HOME/promptorius/config`.

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

# Helper functions
fn git_prompt() {
    if (!git.is_repo()) { return "" }
    return color("git_branch") + " " + git.branch() + color("")
}

# Required: return the left prompt string
fn left_prompt() {
    dir = color("directory") + cwd().replace(env("HOME"), "~") + color("")
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
| `cwd()` | string | Current working directory |
| `os()` | string | `"macos"`, `"linux"`, `"windows"` |
| `eprint(msg)` | null | Print to stderr (debug) |
| `set_config(name, val)` | null | Set a config value (e.g. `"timeout"`, `"concurrency"`) |

### File operations

| Function | Returns | Description |
|---|---|---|
| `file.exists(path)` | bool | Check if file exists |
| `file.read(path)` | string | Read file contents (capped 64KB) |
| `file.write(path, content)` | null | Write string to file |
| `file.type(path)` | string | `"file"`, `"dir"`, `"symlink"`, `"none"` |

### Directory operations

| Function | Returns | Description |
|---|---|---|
| `dir.search(pattern)` | array | Glob match from cwd |
| `dir.search_upwards(name)` | string | Walk up from cwd looking for file, `""` if not found |

### Concurrency

| Function | Returns | Description |
|---|---|---|
| `spawn(closure)` | future | Run a closure on the thread pool, returns a future |
| `wait(futures)` | array | Wait for an array of futures to resolve, returns array of results |

```
# Run git and battery checks in parallel
f1 = spawn(fn() { git.branch() })
f2 = spawn(fn() { battery.pct() })
results = wait([f1, f2])
branch = results[0]
pct = results[1]
```

### Command execution

| Function | Returns | Description |
|---|---|---|
| `exec(cmd, args)` | string | Run command, return stdout trimmed |
| `exec_ok(cmd, args)` | bool | Run command, true if exit 0 |

### Git (native via libgit2)

| Function | Returns | Description |
|---|---|---|
| `git.is_repo()` | bool | Whether cwd is inside a git repo |
| `git.branch()` | string | Branch name or short SHA if detached |
| `git.root()` | string | Root of the git repo |
| `git.status()` | dict | `{ modified, staged, untracked, conflicts, ahead, behind }` |

### Colors

| Function | Returns | Description |
|---|---|---|
| `setcolors(dict)` | null | Set the color palette |
| `color(name)` | string | Emit ANSI escape for named color, `""` to reset |

### Battery

| Function | Returns | Description |
|---|---|---|
| `battery.pct()` | number | Charge percentage, `-1` if no battery |
| `battery.state()` | string | `"charging"`, `"discharging"`, `"full"`, `"empty"`, `"none"` |
| `battery.time()` | number | Seconds remaining, `-1` if unavailable |

### Type coercion functions

| Function | Description |
|---|---|
| `string(val)` | Force convert to string. `null` → `""`, numbers → decimal string, bool → `"true"`/`"false"` |
| `number(val)` | Force convert to number. `null` → `0`, `""` → `0`, `"5.2"` → `5.2`, non-numeric strings → `0`, `true` → `1`, `false` → `0` |
| `array(val)` | Force convert to array. `null` → `[]`, string → `[string]`, array → identity |
| `dict(val)` | Force convert to dict. `null` → `{}`, array of `[key, value]` pairs → dict |

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

The compiled binary accepts `--var name:value` to inject global variables. Values are always strings; the script's type coercion handles conversion as needed.

```
__promptorius_output --var exit_code:0 --var keymap:vicmd --var shell:zsh
```

All variable names are valid — accessing an undefined variable returns `null` instead of an error. This means `--var` declarations are optional; scripts can check any variable name safely.

`--right` flag tells the binary to call `right_prompt()` instead of `left_prompt()`.

## Compilation

### `promptorius compile`

```
promptorius compile                    # compile config → __promptorius_output
promptorius compile <script> <output>  # compile specific script to specific output
```

What it does:
1. Parses the script into an AST
2. Generates a complete Rust source file containing:
   - The runtime (git, env, color, battery, exec, etc.) as inline Rust code
   - The compiled script logic as Rust functions
   - A `main()` that parses `--var` / `--right` and calls `left_prompt()` or `right_prompt()`
   - Generated `--var` arg parser
3. Builds the binary using a persistent cargo project in `$XDG_DATA_HOME/promptorius/build/`
   - First build: downloads and compiles dependencies (~30s one-time)
   - Subsequent builds: only recompiles the generated source (~1-2s incremental)
4. Copies the binary to `$XDG_DATA_HOME/promptorius/__promptorius_output`

### Build failure handling

If the build fails due to a non-script-syntax issue (e.g. missing system library, corrupted build cache), promptorius shows the error and asks the user if they want to run `promptorius clean` and retry.

### `promptorius clean`

Removes the entire build directory at `$XDG_DATA_HOME/promptorius/build/`, forcing a full rebuild on next compile.

### `promptorius explain`

Compiles a special instrumented binary `__promptorius_explanation` that wraps each function call with timing, then runs it and displays a breakdown. The explanation binary lives alongside `__promptorius_output` in XDG_DATA_HOME.

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
promptorius compile                     # Compile config → default output
promptorius compile <script> <output>   # Compile specific files
promptorius init <shell>                # Print shell init script
promptorius check                       # Validate script syntax
promptorius explain                     # Build instrumented binary and show timing
promptorius clean                       # Remove build directory
promptorius completions <shell>         # Generate shell completions

__promptorius_output                    # Render left prompt
__promptorius_output --right            # Render right prompt
__promptorius_output --var name:value   # Inject variables
```

## Shell integration

`promptorius init zsh` outputs a script that:

1. On `precmd`: checks if `__promptorius_output` is stale
2. If stale: runs `promptorius compile` (shows status messages during build)
3. Calls `__promptorius_output` with `--var` args for exit code, duration, keymap, jobs, shell
4. Sets `PROMPT` and `RPROMPT`

```zsh
zmodload zsh/datetime
zmodload zsh/parameter

promptorius_precmd() {
    local exit_code=$?
    local duration_ms=0

    if [[ -n "$_promptorius_cmd_ran" ]]; then
        if [[ -n "$_promptorius_start" ]]; then
            duration_ms=$(( (EPOCHREALTIME - _promptorius_start) * 1000 ))
            duration_ms=${duration_ms%.*}
        fi
        unset _promptorius_cmd_ran
    else
        exit_code=0
        duration_ms=0
    fi
    unset _promptorius_start

    local job_count=${#jobstates[*]}

    # Auto-recompile if stale
    local script="${XDG_CONFIG_HOME:-$HOME/.config}/promptorius/config"
    local binary="${XDG_DATA_HOME:-$HOME/.local/share}/promptorius/__promptorius_output"
    local compiler="$(command -v promptorius)"

    if [[ ! -f "$binary" || "$script" -nt "$binary" || "$compiler" -nt "$binary" ]]; then
        promptorius compile
    fi

    local -a vars=(
        --var "exit_code:${exit_code}"
        --var "duration:${duration_ms}"
        --var "jobs:${job_count}"
        --var "keymap:${_promptorius_keymap}"
        --var "shell:zsh"
        --var "shlvl:${SHLVL}"
    )
    PROMPT="$($binary "${vars[@]}")"
    RPROMPT="$($binary --right "${vars[@]}")"
    _promptorius_keymap="${KEYMAP:-}"
}

promptorius_render() {
    # Re-render on keymap change without re-checking staleness
    local -a vars=(
        --var "exit_code:0"
        --var "keymap:${_promptorius_keymap}"
        --var "shell:zsh"
        --var "shlvl:${SHLVL}"
    )
    local binary="${XDG_DATA_HOME:-$HOME/.local/share}/promptorius/__promptorius_output"
    PROMPT="$($binary "${vars[@]}")"
    RPROMPT="$($binary --right "${vars[@]}")"
    zle reset-prompt 2>/dev/null
}

promptorius_preexec() {
    _promptorius_start=$EPOCHREALTIME
    _promptorius_cmd_ran=1
}

promptorius_zle_keymap_select() {
    _promptorius_keymap="${KEYMAP:-}"
    promptorius_render
}

autoload -Uz add-zsh-hook
add-zsh-hook precmd promptorius_precmd
add-zsh-hook preexec promptorius_preexec

zle -N zle-keymap-select promptorius_zle_keymap_select
```

## Default script

Shipped in the promptorius source tree at `default_config`. Copied to `$XDG_CONFIG_HOME/promptorius/config` on first run if no config exists.

```
# Promptorius prompt script
# https://github.com/user/promptorius

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
    result += color("directory") + cwd().replace(env("HOME"), "~") + color("")

    # Prompt character: │ for user, # for root, repeated SHLVL times
    char = env("USER") == "root" ? "#" : "│"
    col = keymap === "vicmd" ? "char_vicmd" : "char_normal"
    result += " " + color(col) + char.repeat(shlvl) + color("") + " "

    return result
}

# --- Right prompt ---
fn right_prompt() {
    if (!git.is_repo()) { return "" }
    return color("git_branch") + " " + git.branch() + color("")
}

# --- Uncomment below for more features ---

# fn git_prompt() {
#     if (!git.is_repo()) { return "" }
#     branch = git.branch()
#     s = git.status()
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
#     pct = battery.pct()
#     if (pct < 0) { return "" }
#     state = battery.state()
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

