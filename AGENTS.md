# Agents Guide

Promptorius compiles a custom scripting language to native Rust binaries for sub-millisecond shell prompt rendering.

## Repository map

```
SPEC.md                     # Language + product spec
ARCHITECTURE.md             # Module structure, invariants
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
│   ├── mod.rs              # Orchestration
│   ├── runtime.rs          # Inline Rust runtime (~800 lines as const string)
│   ├── expr.rs             # Expression codegen
│   └── stmt.rs             # Statement codegen
├── compiler/               # Build orchestration
│   ├── mod.rs              # compile(), is_stale(), clean(), paths
│   └── project.rs          # Manages cargo project in XDG_DATA_HOME
└── shell/                  # Shell init scripts
```

Legacy code (to be removed): `config/`, `host/`, `pipeline/`, `render/`, `script/`, `stdlib/`

## How to work

- `cargo build` — build the compiler
- `cargo test` — run tests
- `target/debug/promptorius compile` — compile the user's config
- `target/debug/promptorius check` — validate syntax
- `target/debug/promptorius explain --var exit_code:0` — timing breakdown
- `target/debug/promptorius clean` — nuke the build cache

## Gotchas

- The runtime in `runtime.rs` is a raw string (`r##"..."##`). Bulk find-and-replace can cause partial matches — always verify.
- The generated Rust lives at `$XDG_DATA_HOME/promptorius/build/src/main.rs` — inspect it when debugging codegen.
- IIFEs (`fn() { ... }()`) MUST be inlined, not Rust closures, because closures capture scope by clone.
- After any API rename: update codegen, runtime, spec, default_config, and the user's config.
