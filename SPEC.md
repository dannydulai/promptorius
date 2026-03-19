# Promptorius

A compiled, scriptable shell prompt engine. Write your prompt in a simple scripting language, compile it to a native binary, get sub-millisecond rendering.

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
# This is a comment (# only)

x = 5
name = "world"
greeting = `hello {name}`    # backtick interpolation
```

- `#` comments to end of line
- Semicolons optional everywhere
- Bare assignment creates variables (`x = 5`, no `let` needed)
- `null` keyword for unset values
- `true` / `false` booleans
- Accessing an undefined variable returns `null`
- Functions require explicit `return`; without it they return `null`

### Strings

Three string types:

- `"double quoted"` — supports `\n`, `\t`, `\r`, `\\`, `\"`, `\0`, `\u{1F600}` escapes
- `'single quoted'` — supports same escapes
- `` `backtick` `` — interpolation with `{expr}`, use `{{` / `}}` for literal braces

All strings support inline unicode characters.

### Types

Dynamically typed. Values are: string, number (f64), bool, null, array, dict, regex, closure, future.

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

Assignment works on variables, dict members, and array indices:
```
x = 5          # variable
d.foo = "bar"  # dict member
d.foo += "!"   # compound on member
arr[0] = "x"   # array index
arr[0] += "y"  # compound on index
```

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

Note: bare blocks `{ ... }` are not supported. Use `fn() { ... }()` (IIFE) if you need a scope boundary.

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
doubled = arr.map(fn(x) { return x * 2 })

# IIFE for scoping
fn() {
    temp = expensive_calc()
    result += temp
}()
```

Functions require explicit `return`. Without a `return` statement, they return `null`.

Calling a non-function value prints an error to stderr and exits.

### Arrays

```
arr = [1, 2, 3]
arr.push(4)
arr.pop()
arr.shift()
arr.len()
arr.map(fn(x) { return x * 2 })
arr.filter(fn(x) { return x > 0 })
arr.reduce(fn(acc, x) { return acc + x }, 0)
first = arr[0]
arr[0] = "new"
```

### Dicts

```
d = {
    name: "danny",
    age: 30,
    nested: { foo: "bar" },
}
d["name"]
d.name
d.get("name")    # returns null if missing
d.keys()
d.values()
d.len()
d.foo = "bar"
d.foo += "!"
d["key"] = "val"
```

### Regular expressions

Created from strings using the `regex()` function:

```
re = regex("^hello", "i")
re.test("Hello World")      # true
match = re.exec("Hello World")
# match[0] is the full match, match[1]+ are capture groups

result = "hello world".replace(regex("world"), "promptorius")
parts = "a,b,c".split(",")
```

Flags: `i` (case-insensitive), `g` (global), `m` (multiline).

## Script structure

The script file is `$XDG_CONFIG_HOME/promptorius/config` (default `~/.config/promptorius/config`).

Two functions are required:
- `left_prompt()` — returns the left prompt string
- `right_prompt()` — returns the right prompt string

Everything else (global variables, helper functions, `colors()` call) is top-level code that runs once at startup.

```
colors({
    directory: "#6ec2e8",
    error: { fg: "red", bold: true },
    git_branch: "#e89050",
})

fn git_prompt() {
    if (!git.is_repo()) { return "" }
    return `{C("git_branch")} {git.branch()}{C("")}`
}

fn left_prompt() {
    dir = cwd().replace(env("HOME"), "~")
    return `{C("directory")}{dir}{C("")} > `
}

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
| `spawn(closure)` | future | Run a closure on a thread, returns a future |
| `wait(input)` | dict or array | Wait for futures to resolve. Accepts a dict or array, returns same shape with resolved values. |

```
w = wait({
    branch: spawn(fn() { return git.branch() }),
    status: spawn(fn() { return git.status() }),
})
w.branch   # "main"
w.status   # { modified: 0, staged: 0, ... }
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
| `git.root()` | string | Root of the git repo (no trailing `/`) |
| `git.origin()` | string | URL of the `origin` remote, `""` if none |
| `git.status()` | dict | `{ modified, staged, untracked, conflicts, ahead, behind }` |

### Colors

| Function | Returns | Description |
|---|---|---|
| `colors(dict)` | null | Set the color palette |
| `C(name)` | string | Emit ANSI escape for named color. `C("")` resets all formatting. |

Each entry in the `colors()` dict is either a simple string (foreground only) or a dict:

| Key | Type | Description |
|---|---|---|
| `fg` | string | Foreground color |
| `bg` | string | Background color |
| `bold` | bool | Bold |
| `italic` | bool | Italic |
| `dim` | bool | Dim |
| `strikethrough` | bool | Strikethrough |
| `underline` | string | `"single"`, `"double"`, `"curly"`, `"dotted"`, `"dashed"` (kitty protocol) |
| `underline_color` | string | Underline color (independent of fg, kitty protocol) |

Color values: hex `"#ff5f00"`, short hex `"#f50"`, or named:
`black`, `red`, `green`, `yellow`, `blue`, `magenta`/`purple`, `cyan`, `white`,
`bright black`, `bright red`, `bright green`, `bright yellow`, `bright blue`,
`bright magenta`/`bright purple`, `bright cyan`, `bright white`.

Named colors use standard ANSI codes (30-37, 90-97) so they respect your terminal theme. Hex colors use truecolor (24-bit).

### Battery

| Function | Returns | Description |
|---|---|---|
| `battery.pct()` | number | Charge percentage, `-1` if no battery |
| `battery.state()` | string | `"charging"`, `"discharging"`, `"full"`, `"empty"`, `"none"` |
| `battery.time()` | number | Seconds remaining, `-1` if unavailable |

### Math functions

| Function | Description |
|---|---|
| `floor(n)` | Round down to nearest integer |
| `ceil(n)` | Round up to nearest integer |
| `round(n)` | Round to nearest integer |

### Type coercion functions

| Function | Description |
|---|---|
| `string(val)` | Force convert to string |
| `number(val)` | Force convert to number |
| `array(val)` | Force convert to array |
| `dict(val)` | Force convert to dict |

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
| `s.slice(start, end)` | Substring. `end` can be `null` for "to end of string" |
| `s.to_upper()` | Uppercase |
| `s.to_lower()` | Lowercase |
| `s.to_number()` | Parse as number |
| `s.repeat(n)` | Repeat n times |
| `s.pad_start(width, char)` | Left-pad with char (default space) to at least width |

### Array methods

| Method | Description |
|---|---|
| `a.len()` | Array length |
| `a.push(val)` | Returns new array with val appended |
| `a.pop()` | Returns removed last element |
| `a.shift()` | Returns removed first element |
| `a.map(fn)` | Returns new array with fn applied to each element |
| `a.filter(fn)` | Returns new array with elements where fn returns truthy |
| `a.reduce(fn, init)` | Reduces array to single value |

### Dict methods

| Method | Description |
|---|---|
| `d.len()` | Number of entries |
| `d.keys()` | Array of keys |
| `d.values()` | Array of values |
| `d.get(key)` | Get value by key, returns `null` if missing |

### Regex methods

| Method | Description |
|---|---|
| `re.test(str)` | Returns bool — whether the regex matches |
| `re.exec(str)` | Returns array of captures, or `null` if no match |

## --var arguments

The compiled binary accepts `--var name:value` to inject global variables. Values are always strings; the script's type coercion handles conversion as needed.

```
__promptorius_output --var exit_code:0 --var keymap:vicmd --var shell:zsh
```

All variable names are valid — accessing an undefined variable returns `null`. `--right` flag calls `right_prompt()` instead of `left_prompt()`.

### Standard variables (passed by shell integration)

| Variable | Description |
|---|---|
| `exit_code` | Exit code of the last command. `"0"` if no command ran. |
| `duration` | Duration of last command in milliseconds |
| `jobs` | Number of background jobs |
| `keymap` | Vi keymap: `""` (insert/default), `"vicmd"` (normal) |
| `shell` | Shell name: `"zsh"`, `"bash"`, `"fish"`, `"nu"` |
| `shlvl` | Shell nesting level |

## CLI

```
promptorius compile                        # Compile config → default output
promptorius compile <script> <output>      # Compile specific files
promptorius init <shell>                   # Print shell init script (zsh, bash, fish, nushell)
promptorius check [script]                 # Validate script syntax
promptorius explain [--right] [--var k:v]  # Build instrumented binary and show timing
promptorius clean                          # Remove build directory
promptorius completions <shell>            # Generate shell completions
```

## Compilation

### How it works

1. Parses the script into an AST
2. Generates a complete Rust source file (runtime + compiled script + main)
3. Builds using a persistent cargo project in `$XDG_DATA_HOME/promptorius/build/`
   - First build: downloads and compiles dependencies (~30s one-time)
   - Subsequent builds: only recompiles the generated source (~1-2s)
4. Copies the binary to `$XDG_DATA_HOME/promptorius/__promptorius_output`

### Staleness

The binary is stale if:
- The script file is newer than the binary (mtime)
- The `promptorius` compiler binary is newer than the output binary

### Dependencies (in the generated project)

| Crate | Purpose |
|---|---|
| `git2` | Native git operations (libgit2) |
| `starship-battery` | Battery status |
| `glob` | File globbing |
| `regex` | Regular expressions |

## Shell integration

Supported: zsh, bash, fish, nushell.

The shell init script handles:
- Staleness check and auto-recompile
- Duration timing (millisecond precision)
- Background job counting
- Vi keymap change detection (zsh: re-renders immediately)
- Exit code suppression on empty Enter (only shows once after a real command)
- ANSI escape wrapping (`%{...%}` for zsh, `\[...\]` for bash)

## Known limitations

- No `break` / `continue` in loops
- No `try` / `catch`
- No multi-line string literals (use `\n` or backtick interpolation)
- No spread operator or destructuring
- Closures capture scope by clone; mutations after creation don't propagate

## Non-goals

- **No interpreter at runtime.** The compiled binary has zero scripting overhead.
- **No plugin system.** One script file, compiled to one binary.
- **No package manager.** Copy-paste functions between scripts.
- **No Windows support for v1.** macOS and Linux only.
