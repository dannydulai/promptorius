# Promptorius

A fast, scriptable shell prompt engine written in Rust. An alternative to Starship that replaces hundreds of hardcoded modules with a generic Rhai scripting system.

## Problem

Starship ships ~100+ built-in modules (git, node, python, rust, docker, etc.), each with its own config schema, update cycle, and edge cases. Adding or customizing behavior means waiting for upstream changes or forking. The module system is wide but shallow — you get toggles and format strings, not real logic.

Promptorius takes the opposite approach: ship a small, fast core with powerful primitives, and let users define prompt segments in Rhai scripts.

## Design Principles

1. **Scripts over modules.** Every prompt segment is a Rhai script. No hardcoded modules in the binary.
2. **Ship a good stdlib.** Provide a library of default scripts that cover common use cases (git, language versions, etc.) out of the box. Users can override or ignore them.
3. **Fast by default.** Prompt rendering must complete in under 50ms for typical configurations. Scripts run concurrently where possible.
4. **Shell agnostic.** Support bash, zsh, fish, and nushell via shell-specific init scripts.
5. **Simple config.** One TOML file for layout and global settings. Rhai scripts for logic.

## Architecture

```
┌─────────────────────────────────────────────┐
│                   Shell                     │
│  (bash/zsh/fish/nushell init script)        │
├─────────────────────────────────────────────┤
│              promptorius binary             │
│                                             │
│  ┌──────────┐  ┌──────────┐  ┌───────────┐  │
│  │  Config  │  │  Script  │  │  Renderer │  │
│  │  Parser  │  │  Engine  │  │           │  │
│  │  (TOML)  │  │  (Rhai)  │  │  (ANSI)   │  │
│  └──────────┘  └──────────┘  └───────────┘  │
│        │              │             │       │
│        ▼              ▼             ▼       │
│  ┌──────────────────────────────────────┐   │
│  │          Segment Pipeline            │   │
│  │  resolve → execute → style → render  │   │
│  └──────────────────────────────────────┘   │
└─────────────────────────────────────────────┘
```

### Core Components

**Config Parser** — Reads `$XDG_CONFIG_HOME/promptorius/config.toml` (defaulting to `~/.config/promptorius/config.toml`). Defines which segments appear, their order, layout (left/right prompt, newlines), and global settings. Does *not* contain segment logic.

**Script Engine** — Embedded Rhai runtime. Each segment maps to a `.rhai` script. The engine exposes a set of built-in functions to scripts (see Host API below) and collects the output.

**Segment Pipeline** — Resolves which scripts to run, executes them concurrently, collects results, applies colors, and assembles the final prompt string.

**Renderer** — Converts colored segments into shell-appropriate ANSI escape sequences. Handles width calculation for right-aligned prompts.

## Configuration

### `config.toml`

```toml
# $XDG_CONFIG_HOME/promptorius/config.toml

[prompt]
format = """
{s("directory") + s("git") + s("language")}> \
"""

# Right-side prompt (supported shells only)
right_format = "{s(\"time\")}"

# Newline before prompt
add_newline = true

[colors]
# Named color palette — use these names in scripts via `color("name")`
# Each color can be a simple string (foreground only) or a table with:
#   fg, bg        — color value: named ("red", "cyan"), bright ("bright_red"),
#                   256-palette (196), or hex ("#ff5f00", "#f50")
#   bold          — bool
#   italic        — bool
#   dim           — bool
#   strikethrough — bool
#   underline     — "single", "double", "curly", "dotted", "dashed" (kitty protocol)
#   underline_color — color value for underline (kitty protocol)

default = "white"
directory = { fg = "cyan", bold = true }
git = "purple"
success = "green"
warning = "yellow"
error = { fg = "red", bold = true }
muted = { fg = "bright_black", dim = true }

[segments.directory]
script = "directory.rhai"      # resolved from script_path

[segments.git]
script = "git.rhai"
# Arbitrary key-value pairs passed to the script as `config` map
max_branch_len = 32

[segments.language]
script = "language.rhai"

[segments.time]
script = "time.rhai"
format = "%H:%M"

[settings]
# Additional search paths for .rhai files (checked before stdlib)
# Config dir ($XDG_CONFIG_HOME/promptorius/scripts/) is always searched first.
script_path = ["~/.local/share/promptorius/scripts/"]

# Max time (ms) to wait for all segments before rendering what we have
timeout = 50

# Number of threads for concurrent script execution
concurrency = 4
```

### Format expressions

The `format` and `right_format` values are templates where `{...}` blocks contain Rhai expressions evaluated at render time.

- `{expr}` — evaluate the Rhai expression and insert the result.
- `s("name")` — render segment `name`, returns its output or empty string if the segment produced nothing.
- `{{` / `}}` — literal `{` / `}` characters.
- Everything outside `{}` is literal text.

This means format strings have the full power of Rhai — conditionals, string manipulation, color calls, etc:

```toml
format = """{s("directory")} {color("git")}{s("git")}{color("")} > """
right_format = '{if exit_code() != 0 { color("error") + exit_code() + color("") } else { "" }}'
```

## Rhai Script API

Each segment script is a Rhai script that returns a string (the segment text) or `()` to produce no output.

The engine registers string coercion overloads so `+` automatically converts common types (`i64`, `f64`, `bool`) to strings when concatenated with a string. `"ahead: " + 3` just works — no `.to_string()` needed. Rhai's backtick interpolation (`` `ahead: ${count}` ``) also works out of the box.

### Host API (functions exposed to Rhai)

These are Rust functions registered into the Rhai engine, callable from any script:

#### Environment

| Function | Signature | Description |
|---|---|---|
| `env` | `(name: &str) -> String` | Get environment variable, empty string if unset |
| `env_set` | `(name: &str, value: &str)` | Set an environment variable for the current prompt render |
| `cwd` | `() -> String` | Current working directory |
| `os` | `() -> String` | Operating system (`linux`, `macos`, `windows`) |

#### Filesystem

| Function | Signature | Description |
|---|---|---|
| `file_exists` | `(path: &str) -> bool` | Check if file/dir exists |
| `read_file` | `(path: &str) -> String` | Read file contents (capped at 64KB) |
| `glob_files` | `(pattern: &str) -> Array` | Glob match from cwd, returns file paths |
| `find_upward` | `(filename: &str) -> String` | Walk up from cwd looking for file, returns path or empty |

#### Command execution

| Function | Signature | Description |
|---|---|---|
| `exec` | `(cmd: &str, args: Array) -> String` | Run command, return stdout (trimmed). Times out after 10ms by default. |
| `exec_ok` | `(cmd: &str, args: Array) -> bool` | Run command, return true if exit code 0 |

#### Git (high-frequency, so native for speed)

| Function | Signature | Description |
|---|---|---|
| `git_branch` | `() -> String` | Current branch name or short SHA if detached |
| `git_is_repo` | `() -> bool` | Whether cwd is inside a git repo |
| `git_status` | `() -> Map` | `#{ modified: i64, staged: i64, untracked: i64, conflicts: i64, ahead: i64, behind: i64 }` |
| `git_root` | `() -> String` | Root of the git repo |

#### Colors

| Function | Signature | Description |
|---|---|---|
| `color` | `(name: &str) -> String` | Emit ANSI escape to set the named color from `[colors]`. Pass `""` to reset. |

#### Cache (namespaced per segment)

| Function | Signature | Description |
|---|---|---|
| `cache_set` | `(key: &str, value: Dynamic)` | Store a value in the segment's cache |
| `cache_get` | `(key: &str) -> Dynamic` | Retrieve a cached value, `()` if not found |
| `cache_has` | `(key: &str) -> bool` | Check if a key exists in the segment's cache |
| `cache_del` | `(key: &str)` | Remove a key from the segment's cache |

Cache is persisted across prompt renders and namespaced per segment name, so scripts can store state without colliding with each other.

#### Segment config

| Function | Signature | Description |
|---|---|---|
| `config` | Global `Map` | The key-value pairs from this segment's TOML block |

### Example Scripts

**`directory.rhai`**
```rhai
// Show abbreviated cwd, replacing $HOME with ~
let dir = cwd();
let home = env("HOME");

if dir.starts_with(home) {
    "~" + dir[home.len()..]
} else {
    dir
}
```

**`git.rhai`**
```rhai
if !git_is_repo() {
    return ();  // no output — segment hidden
}

let branch = git_branch();
let max_len = config.get("max_branch_len") ?? 40;

if branch.len() > max_len {
    branch = branch[..max_len] + "…";
}

let s = git_status();
let indicators = "";

if s.staged > 0    { indicators += color("success") + "+"; }
if s.modified > 0  { indicators += color("warning") + "!"; }
if s.untracked > 0 { indicators += color("error") + "?"; }
if s.conflicts > 0 { indicators += color("error") + "✘"; }
if indicators != "" { indicators += color(""); }

let remote = "";
if s.ahead > 0  { remote += "↑" + s.ahead; }
if s.behind > 0 { remote += "↓" + s.behind; }

icon("git-branch") + " " + branch + " " + indicators + remote
```

**`language.rhai`**
```rhai
// Detect project language and show version
let result = "";

if find_upward("Cargo.toml") != "" {
    let v = exec("rustc", ["--version"]);
    // "rustc 1.78.0 (..." -> "1.78.0"
    let parts = v.split(" ");
    if parts.len() >= 2 {
        result = icon("rust") + " " + parts[1];
    }
} else if find_upward("package.json") != "" {
    let v = exec("node", ["--version"]);
    result = icon("nodejs") + " " + v;
} else if find_upward("pyproject.toml") != "" || find_upward("setup.py") != "" {
    let v = exec("python3", ["--version"]);
    let parts = v.split(" ");
    if parts.len() >= 2 {
        result = icon("python") + " " + parts[1];
    }
}

result  // empty string = segment hidden
```

## Standard Library Scripts

Ship a `stdlib/` directory with scripts covering common cases:

- `directory.rhai` — abbreviated cwd with `~` substitution
- `git.rhai` — branch, status indicators, ahead/behind
- `language.rhai` — detect and show version for common languages
- `time.rhai` — current time
- `duration.rhai` — last command duration (if above threshold)
- `exitcode.rhai` — show non-zero exit codes
- `jobs.rhai` — background job count
- `user_host.rhai` — username@hostname (e.g. for SSH sessions)
- `character.rhai` — prompt character, colored by last exit code

Users can override any of these by placing a file with the same name in their scripts directory.

## Performance Strategy

1. **Concurrent execution.** Segments with no dependencies run in parallel on a thread pool.
2. **Native git.** Git operations use `git2` (libgit2 bindings) directly, not shelling out to `git`.
3. **Timeout.** Each script has a per-script timeout (default 10ms for `exec` calls, 50ms global). If a segment times out, it's silently omitted.
4. **Caching.** Results from `git_status()`, `cwd()`, etc. are computed once and shared across segments within a single prompt render.
5. **Rhai compilation.** Scripts are compiled to AST on first run and cached on disk.

## CLI Interface

```
promptorius                     # Print the left prompt (format)
promptorius --right             # Print the right prompt (right_format)
promptorius --cmd :int:exit_code:0 --cmd :str:duration:12ms
                                # Define functions callable from scripts
promptorius init <shell>        # Print shell init script (bash|zsh|fish|nushell)
promptorius explain             # Show what each segment resolved to and timing
promptorius check               # Validate config and scripts, report errors
promptorius script new <name>   # Scaffold a new segment script
promptorius completions <shell> # Generate shell completions
```

### `--cmd` flag

`--cmd :type:name:value` registers a zero-argument function `name()` in the Rhai engine that returns `value` parsed as `type`. Can be repeated any number of times.

Supported types: `str`, `int`, `float`, `bool`.

This is how the shell init script passes runtime context (exit code, command duration, etc.) into scripts — there are no hardcoded "last exit code" functions in the engine.

Example:
```
promptorius --cmd :int:exit_code:127 --cmd :str:duration:1200ms
```

Scripts then call `exit_code()` and `duration()` as regular functions.

## Shell Integration

`promptorius init zsh` outputs something like:

```zsh
promptorius_precmd() {
    local exit_code=$?
    local duration=$((EPOCHREALTIME - ${_promptorius_start:-$EPOCHREALTIME}))
    local cmd_args="--cmd :int:exit_code:${exit_code} --cmd :int:duration:${duration}"
    PROMPT="$(promptorius $cmd_args)"
    RPROMPT="$(promptorius --right $cmd_args)"
    unset _promptorius_start
}

promptorius_preexec() {
    _promptorius_start=$EPOCHREALTIME
}

autoload -Uz add-zsh-hook
add-zsh-hook precmd promptorius_precmd
add-zsh-hook preexec promptorius_preexec
```

## Crate Dependencies (expected)

| Crate | Purpose |
|---|---|
| `rhai` | Embedded scripting engine |
| `git2` | Native git operations |
| `toml` | Config parsing |
| `glob` | File globbing |
| `clap` | CLI argument parsing |
| `rayon` or `tokio` | Concurrent script execution |
| `nu-ansi-term` | ANSI styling |
| `dirs` | XDG / platform directories |
| `unicode-width` | Correct prompt width calculation |

## Non-Goals

- **No built-in modules.** Everything is a script. If it's not in a `.rhai` file, it doesn't render.
- **No remote fetching of scripts.** Scripts are local files. Package management is out of scope for v1.
- **No interactive configuration wizard.** Edit TOML and Rhai files directly.
- **No Windows Terminal integration.** Standard ANSI only for v1.

## Open Questions

1. **Transient prompt?** Starship supports replacing the prompt after command execution with a minimal version. Worth supporting in v1?
2. **Async segments?** Should segments be able to declare async data fetches that populate on re-render (like p10k's instant prompt)?
3. **Script sandboxing?** Rhai is sandboxed by default (no filesystem/network), and we selectively expose capabilities via the host API. Should we add a permission model for `exec`?
4. **Icon sets?** Ship a built-in Nerd Font icon map, or require users to inline unicode?
