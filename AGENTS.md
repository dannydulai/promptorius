# Agents Guide

Promptorius is a Rhai-scriptable shell prompt engine written in Rust. See [SPEC.md](SPEC.md) for the full product spec.

## Quick orientation

- **What it does**: Replaces shell prompts. Users define prompt segments as `.rhai` scripts. One TOML config for layout, Rhai scripts for logic.
- **Language**: Rust (2021 edition)
- **Key dependency**: `rhai` (embedded scripting engine)

## Repository map

```
SPEC.md                     # Product spec — the source of truth for what we're building
ARCHITECTURE.md             # Module structure, dependency rules, data flow
docs/
├── design-docs/            # Design decisions and rationale
│   ├── index.md            # Index of all design docs
│   └── core-beliefs.md     # Principles that guide agent decisions
├── exec-plans/             # Implementation plans
│   ├── active/             # Currently in progress
│   └── completed/          # Done
└── references/             # External docs pulled into repo for agent legibility
src/
├── main.rs                 # Entry point — CLI parsing, dispatch
├── cli/                    # CLI argument parsing and subcommands
├── config/                 # TOML config parsing and validation
├── script/                 # Rhai engine setup, script loading, AST caching
├── host/                   # Host API functions registered into Rhai
├── pipeline/               # Segment resolution, concurrent execution, assembly
└── render/                 # ANSI escape generation, width calculation
stdlib/                     # Default .rhai segment scripts shipped with the binary
```

## Architecture rules

See [ARCHITECTURE.md](ARCHITECTURE.md) for full details. The short version:

1. **Dependency direction**: `cli` -> `pipeline` -> `script` + `config` + `render`. `host` is registered into `script`. No cycles.
2. **`host/` is the boundary**: All Rhai-callable functions live in `host/`. No Rhai registration happens elsewhere.
3. **`config/` is pure data**: Parses TOML into typed structs. No side effects, no IO beyond reading the config file.
4. **`render/` is terminal-only**: Knows about ANSI escapes and unicode width. Does not know about Rhai or config.

## How to work in this repo

- `cargo build` — build
- `cargo test` — run all tests
- `cargo clippy -- -D warnings` — lint (must pass, warnings are errors)
- `cargo fmt --check` — format check

## Conventions

- No `unwrap()` or `expect()` in library code. Use `Result`/`Option` propagation.
- Public types and functions get doc comments.
- Each module (`config`, `script`, `host`, `pipeline`, `render`, `cli`) has a `mod.rs` that re-exports its public API.
- Tests live in the same file as the code they test (`#[cfg(test)] mod tests`), not in a separate `tests/` directory, unless they are integration tests.
- Error types are defined per module using `thiserror`.
