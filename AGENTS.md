# Agents Guide

Promptorius is a compiled prompt engine. Users write a config script in a custom language, which compiles to a native Rust binary for sub-millisecond prompt rendering.

## Quick orientation

- **What it does**: Shell prompt replacement. Script compiles to native binary.
- **Language**: Rust (compiler), custom scripting language (user config)
- **Branch**: `experiment1` is the compiled version. `main` has the old Rhai-based interpreter.

## Repository map

```
SPEC.md                     # Full language + product spec
ARCHITECTURE.md             # Module structure, invariants, pipeline
default_config              # Default config script, shipped in binary
src/
├── main.rs                 # Entry point
├── cli/mod.rs              # CLI subcommands
├── lang/                   # Language toolchain
│   ├── token.rs            # Token types
│   ├── lexer.rs            # Tokenizer
│   ├── ast.rs              # AST nodes
│   └── parser.rs           # Recursive descent parser
├── codegen/                # Rust code generation
│   ├── mod.rs              # Orchestration (generate, generate_instrumented)
│   ├── runtime.rs          # Inline Rust runtime (~800 lines as const string)
│   ├── expr.rs             # Expression codegen
│   └── stmt.rs             # Statement codegen
├── compiler/               # Build orchestration
│   ├── mod.rs              # compile(), is_stale(), clean(), paths
│   └── project.rs          # Manages cargo project in XDG_DATA_HOME
└── shell/                  # Shell init scripts
    ├── zsh.sh
    ├── bash.sh
    ├── fish.fish
    └── nushell.nu
```

Legacy code (to be removed): `config/`, `host/`, `pipeline/`, `render/`, `script/`, `stdlib/`

## How to work

- `cargo build` — build the compiler
- `cargo test` — run all tests (79 passing)
- `target/debug/promptorius compile` — compile the user's config to a binary
- `target/debug/promptorius check` — validate config syntax
- `target/debug/promptorius explain --var exit_code:0` — timing breakdown
- `target/debug/promptorius clean` — nuke the build cache

## Important notes

- The runtime in `codegen/runtime.rs` is a raw Rust string (`r##"..."##`). Be careful with string escaping — `{` and `}` in format strings need doubling.
- Bulk find-and-replace on function names is dangerous — `color` → `c` once turned `builtin_colors` into `builtin_cs`. Always check for partial matches.
- The generated Rust lives at `$XDG_DATA_HOME/promptorius/build/src/main.rs` — inspect it when debugging codegen issues.
- IIFEs (`fn() { ... }()`) MUST be inlined, not generated as Rust closures, because closures capture scope by clone and mutations don't propagate back.
