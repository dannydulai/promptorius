# Architecture

Promptorius compiles a custom scripting language to native Rust binaries via a persistent cargo project.

## Pipeline

```
config script → Lexer → Parser → AST → Codegen → Rust source → cargo build → native binary
```

## Modules

### `lang/` — Language toolchain

- **`token.rs`** — Token enum with span tracking. Handles ASI via `can_end_stmt()`.
- **`lexer.rs`** — Tokenizer. `#` comments, three string types, backtick interpolation with `{expr}`, optional semicolons, all operators.
- **`ast.rs`** — AST node types for expressions and statements.
- **`parser.rs`** — Recursive descent with precedence climbing.

### `codegen/` — Rust code generation

- **`mod.rs`** — Orchestration. Emits runtime + user functions + `script_init()` + `main()`. Has `generate()` and `generate_instrumented()`.
- **`runtime.rs`** — Complete Rust runtime as `const &str`. Value type, coercion, operators, Scope, colors, all built-in functions, shell escape wrapping, main template.
- **`expr.rs`** — Expression → Rust. Builtin detection, dynamic dispatch, IIFE inlining, member/index assignment.
- **`stmt.rs`** — Statement → Rust.

### `compiler/` — Build orchestration

- **`mod.rs`** — `compile()`, `is_stale()`, `clean()`, path helpers, default config creation.
- **`project.rs`** — Manages persistent cargo project in `$XDG_DATA_HOME/promptorius/build/`.

### `cli/` — Command-line interface

Subcommands: `compile`, `clean`, `init`, `check`, `explain`, `completions`.

### `shell/` — Shell init scripts

`zsh.sh`, `bash.sh`, `fish.fish`, `nushell.nu` — staleness check, auto-recompile, duration timing, job count, vi keymap, exit code suppression.

## Key invariants

1. **All non-builtin calls are dynamic.** User functions are registered as closures in scope. Calls go through scope lookup.
2. **IIFEs are inlined.** `fn() { body }()` emits a Rust block, not a closure. Preserves scope mutation.
3. **The runtime is self-contained.** Generated `main.rs` is a complete Rust program. Runtime embedded as string constant.
4. **Named colors use ANSI codes, hex uses truecolor.** `"red"` → `\x1b[31m`, `"#ff0000"` → `\x1b[38;2;255;0;0m`.
