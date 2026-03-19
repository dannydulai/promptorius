# Architecture

Promptorius is a compiled prompt engine. A custom scripting language compiles to native Rust binaries via a persistent cargo project.

## Pipeline

```
config script → Lexer → Parser → AST → Codegen → Rust source → cargo build → native binary
```

## Modules

### `lang/` — Language toolchain

- **`token.rs`** — Token enum with span tracking. Handles ASI (automatic semicolon insertion) via `can_end_stmt()`.
- **`lexer.rs`** — Tokenizer. `#` comments, three string types (double/single/backtick), backtick interpolation with `{expr}`, optional semicolons, all operators including `===`/`!==`/`??`/`..`.
- **`ast.rs`** — AST node types. Expressions (literal, ident, binary, unary, ternary, call, member, index, assign, compound assign, array, dict, interpolation, closure, range, null coalesce) and statements (expr, if/else, while, for-in, return, fn def).
- **`parser.rs`** — Recursive descent with precedence climbing. Assignment → ternary → null coalesce → or → and → equality → comparison → addition → multiplication → unary → postfix → primary.

### `codegen/` — Rust code generation

- **`mod.rs`** — Top-level orchestration. Separates fn defs from top-level stmts. Emits runtime + user functions + `script_init()` (registers fns as closures + runs top-level code) + `main()`. Also has `generate_instrumented()` for explain.
- **`runtime.rs`** — The complete Rust runtime as a `const &str`. ~800 lines covering: Value enum, type coercion, operators, Scope, color system, all built-in functions (env, file, dir, git, battery, exec, regex, spawn/wait, string/array/dict methods), shell escape wrapping, and main() template.
- **`expr.rs`** — Expression codegen. Maps AST expressions to Rust code producing `Value`. Handles builtin detection (namespaced like `git.branch()` and global like `env()`), dynamic dispatch for unknown calls, IIFE inlining, member/index assignment.
- **`stmt.rs`** — Statement codegen. If/while/for-in/return/assignment/fn-def.

### `compiler/` — Build orchestration

- **`mod.rs`** — `compile()`, `is_stale()`, `clean()`, path helpers, default config creation.
- **`project.rs`** — Manages the persistent cargo project in `$XDG_DATA_HOME/promptorius/build/`. Writes `Cargo.toml` (with git2, starship-battery, glob, regex deps), generated `src/main.rs`, runs `cargo build --release`, copies binary.

### `cli/` — Command-line interface

- Subcommands: `compile`, `clean`, `init`, `check`, `explain`, `completions`.
- Default (no subcommand) runs `compile`.

### `shell/` — Shell init scripts

- `zsh.sh`, `bash.sh`, `fish.fish`, `nushell.nu`
- Each handles: staleness check, auto-recompile, duration timing, job count, vi keymap, exit code suppression on empty enter.

### Legacy (to be removed)

- `config/`, `host/`, `pipeline/`, `render/`, `script/`, `stdlib/` — the old Rhai-based engine. Still compiles but unused by the new CLI.

## Key invariants

1. **All non-builtin calls are dynamic.** The codegen never generates static `user_fn_*()` calls from script code. User functions are registered as closures in scope during `script_init()`, and all calls go through scope lookup.
2. **IIFEs are inlined.** `fn() { body }()` does NOT generate a Rust closure — the body is emitted as a Rust block directly, preserving scope mutation.
3. **The runtime is self-contained.** The generated `main.rs` is a complete Rust program with no external code beyond crate deps. The runtime is embedded as a string constant in the promptorius compiler.
4. **Named colors use ANSI codes, hex uses truecolor.** `"red"` → `\x1b[31m`, `"#ff0000"` → `\x1b[38;2;255;0;0m`.
