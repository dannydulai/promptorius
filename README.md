# Promptorius

A compiled shell prompt engine. Write your prompt in a simple scripting language, compile it to a native binary, get sub-millisecond rendering.

## Why

Most prompt tools either ship 100+ hardcoded modules you can't customize (Starship) or interpret scripts on every render (adding latency). Promptorius compiles your prompt script to a native binary — zero interpreter overhead, full scripting power.

## How it works

1. Write your prompt in `~/.config/promptorius/config`
2. Promptorius compiles it to a native binary
3. Your shell calls that binary on every prompt render (~0.1ms)
4. Edit the config, the shell auto-recompiles on next prompt

## Install

```sh
cargo install --path .
```

Then add to your shell rc file:

```sh
# zsh (~/.zshrc)
eval "$(promptorius init zsh)"

# bash (~/.bashrc)
eval "$(promptorius init bash)"

# fish (~/.config/fish/config.fish)
promptorius init fish | source

# nushell
promptorius init nushell | save -f ~/.config/nushell/promptorius.nu
source ~/.config/nushell/promptorius.nu
```

On first prompt, Promptorius creates a default config and compiles it. The first build takes ~30 seconds (downloading and compiling dependencies). After that, recompiles take 1-2 seconds and only happen when you edit the config.

## The language

A simple, dynamically-typed scripting language with JS-like type coercion:

```sh
# ~/.config/promptorius/config

colors({
    directory: "#50b4ff",
    git_branch: "#ddd",
    error: { fg: "red", bold: true },
})

fn left_prompt() {
    result = ""

    if (exit_code != 0) {
        result += `{C("error")}Exited w/ {exit_code}{C("")}\n`
    }

    dir = cwd().replace(env("HOME"), "~")
    result += `{C("directory")}{dir}{C("")} `

    char = env("USER") == "root" ? "#" : "│"
    result += `{char} `

    return result
}

fn right_prompt() {
    if (!git.is_repo()) { return "" }
    return `{C("git_branch")} {git.branch()}{C("")}`
}
```

### Features

- **Variables**: bare assignment (`x = 5`), undefined vars return `null`
- **Strings**: `"double"`, `'single'`, `` `backtick {interpolation}` ``
- **Control flow**: `if`/`else if`/`else`, `while`, `for (x in array)`, `for (i in 0..10)`
- **Functions**: `fn name(args) { }`, closures `fn(x) { return x * 2 }`
- **Operators**: `+`, `-`, `*`, `/`, `%`, `==`, `!=`, `===`, `!==`, `&&`, `||`, `??`, ternary `? :`
- **Types**: string, number, bool, null, array, dict, regex, closure, future
- **Concurrency**: `spawn()` / `wait()` for parallel execution
- **Colors**: `colors()` to set palette, `C("name")` to emit ANSI escapes, `C("")` to reset

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
| Type coercion | `string()`, `number()`, `array()`, `dict()`, `regex()` |

See [SPEC.md](SPEC.md) for the full language reference.

## CLI

```
promptorius compile              # compile config to binary
promptorius init <shell>         # print shell init script
promptorius check                # validate config syntax
promptorius explain              # show timing breakdown
promptorius clean                # remove build cache
```

## Performance

The compiled binary typically renders in under 1ms. Git operations (libgit2) add 2-4ms. The `spawn()`/`wait()` concurrency model lets you run expensive operations in parallel.

```
--- promptorius explain ---

     0.05ms  script_init
     0.06ms  left_prompt()
     0.00ms  shell escape wrapping

     0.15ms  total
       71    output bytes
---
```

## License

MIT
