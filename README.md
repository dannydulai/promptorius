# Promptorius

A compiled shell prompt engine. Write your prompt in a simple scripting language, compile it to a native binary, get sub-millisecond rendering.

```
~/work/promptorius
promptorius/ │                                      experiment1 ✎ ?
```

## Why

Most prompt tools either ship 100+ hardcoded modules you can't customize (Starship) or interpret scripts on every render (adding latency). Promptorius compiles your prompt script to a native binary — zero interpreter overhead, full scripting power.

- **Fast**: prompt renders in ~0.1ms (git adds ~3ms, runs in parallel)
- **Simple**: one config file, two functions (`left_prompt`, `right_prompt`)
- **Powerful**: full scripting language with closures, concurrency, regex, type coercion
- **Native git**: uses libgit2, not shell-out to `git`
- **Auto-rebuild**: edit your config, next prompt picks it up automatically

## Install

Requires Rust toolchain (`cargo`, `rustc`).

```sh
cargo install --path .
```

Add to your shell rc file:

```sh
# zsh (~/.zshrc)
eval "$(promptorius init zsh)"

# bash (~/.bashrc)
eval "$(promptorius init bash)"

# fish (~/.config/fish/config.fish)
promptorius init fish | source
```

On first prompt, Promptorius creates a default config and compiles it. The first build takes ~30 seconds (downloading and compiling dependencies). After that, recompiles take 1-2 seconds and only happen when you edit the config.

## Quick start

Your config lives at `~/.config/promptorius/config`. Here's a minimal example:

```sh
colors({
    directory: "#50b4ff",
    error: { fg: "red", bold: true },
    git_branch: "#ddd",
})

fn left_prompt() {
    dir = cwd().replace(env("HOME"), "~")
    char = env("USER") == "root" ? "#" : "│"
    return `{C("directory")}{dir}{C("")} {char} `
}

fn right_prompt() {
    if (!git.is_repo()) { return "" }
    return `{C("git_branch")} {git.branch()}{C("")}`
}
```

The default config is more feature-rich — it includes git status indicators, command duration, exit codes, vi mode, background jobs, parallel git lookups, and commented-out battery display.

## The language

A simple, dynamically-typed scripting language designed for prompt scripting.

### Strings and interpolation

```sh
name = "world"
greeting = `hello {name}`     # backtick strings interpolate {expressions}
escaped = `use {{braces}}`    # {{ and }} for literal braces
```

### Colors

```sh
# Define a palette
colors({
    ok: "green",
    err: { fg: "red", bold: true },
    muted: "#666",
})

# Use in prompts — C("name") emits ANSI escape, C("") resets
result = `{C("ok")}✓{C("")}`
```

Color values: named (`red`, `bright blue`, etc.), hex (`#f00`, `#ff5f00`). Named colors respect your terminal theme.

Color options: `fg`, `bg`, `bold`, `italic`, `dim`, `strikethrough`, `underline` (`"single"`, `"double"`, `"curly"`, `"dotted"`, `"dashed"`), `underline_color`.

### Concurrency

Run expensive operations in parallel:

```sh
w = wait({
    branch: spawn(fn() { return git.branch() }),
    status: spawn(fn() { return git.status() }),
})
w.branch    # "main"
w.status    # { modified: 0, staged: 1, ... }
```

### Everything else

- **Variables**: `x = 5` (undefined vars return `null`)
- **Control flow**: `if`/`else if`/`else`, `while`, `for (x in arr)`, `for (i in 0..10)`
- **Functions**: `fn name(args) { }`, closures `fn(x) { return x * 2 }`
- **Operators**: `+` `-` `*` `/` `%` `==` `!=` `===` `!==` `&&` `||` `??` `? :`
- **Types**: string, number, bool, null, array, dict, regex, closure, future
- **JS-like coercion**: `"hello" + 5` → `"hello5"`, `null == false` → `true`
- **Comments**: `# line comment`

See [SPEC.md](SPEC.md) for the full language reference.

### Built-in functions

| Category | Functions |
|---|---|
| Environment | `env()`, `cwd()`, `os()`, `eprint()` |
| Git | `git.is_repo()`, `git.branch()`, `git.root()`, `git.origin()`, `git.status()` |
| Files | `file.exists()`, `file.read()`, `file.write()`, `file.type()` |
| Directories | `dir.search()`, `dir.search_upwards()` |
| Commands | `exec()`, `exec_ok()` |
| Colors | `colors()`, `C()` |
| Battery | `battery.pct()`, `battery.state()`, `battery.time()` |
| Concurrency | `spawn()`, `wait()` |
| Math | `floor()`, `ceil()`, `round()` |
| Time | `time()` |
| Strings | `.len()`, `.trim()`, `.contains()`, `.replace()`, `.split()`, `.slice()`, `.pad_start()`, ... |
| Type coercion | `string()`, `number()`, `array()`, `dict()`, `regex()` |

## CLI

```
promptorius compile              # compile config to binary
promptorius init <shell>         # print shell init script
promptorius check                # validate config syntax
promptorius explain              # show timing breakdown
promptorius clean                # remove build cache (forces full rebuild)
promptorius completions <shell>  # generate shell completions
```

## How it works under the hood

1. Promptorius parses your config into an AST
2. Generates a complete Rust source file (your script logic + runtime with git/battery/color/etc.)
3. Builds it via a persistent cargo project in `~/.local/share/promptorius/build/`
4. The resulting binary is self-contained — no runtime dependencies on promptorius

The shell init script checks mtimes on every prompt. If your config or promptorius itself is newer than the compiled binary, it triggers a rebuild automatically.

## Performance

```
--- promptorius explain ---

     0.07ms  script_init
     0.06ms  left_prompt()
     0.00ms  shell escape wrapping

     0.15ms  total
       71    output bytes
---
```

Left prompt renders in ~0.1ms. Git operations (libgit2) add ~3ms but run in parallel via `spawn()`/`wait()` so they don't block the prompt.

## License

[MIT](LICENSE)
