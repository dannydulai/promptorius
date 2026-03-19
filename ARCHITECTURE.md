# Architecture

## Overview

Promptorius is structured as a pipeline: parse config, resolve segments, execute scripts concurrently, render output.

```
CLI args
  │
  ▼
Config (TOML)  ──►  Pipeline  ──►  Renderer  ──►  stdout
                       │
                   Script Engine
                   (Rhai + Host API)
```

## Modules

### `cli` — Command-line interface
- Parses args with `clap` (derive API)
- Handles `--right`, `--cmd`, subcommands (`init`, `explain`, `check`, `script new`, `completions`)
- Calls into `pipeline` for prompt rendering
- **Depends on**: `config`, `pipeline`

### `config` — Configuration
- Reads and validates `$XDG_CONFIG_HOME/promptorius/config.toml`
- Produces typed structs: `Config`, `PromptConfig`, `SegmentConfig`, `ColorDef`, `Settings`
- Resolves script paths (user scripts dir > additional `script_path` entries > stdlib)
- **Depends on**: nothing (leaf module)

### `script` — Rhai engine
- Creates and configures the `rhai::Engine`
- Registers string coercion overloads (`String + i64`, etc.)
- Loads `.rhai` files, compiles to AST, caches compiled scripts
- Evaluates scripts and format template expressions
- **Depends on**: `config` (for script paths and segment config)

### `host` — Host API
- Registers all Rhai-callable functions into the engine: `env`, `env_set`, `cwd`, `os`, `file_exists`, `read_file`, `glob_files`, `find_upward`, `exec`, `exec_ok`, `git_*`, `color`, `icon`, `cache_*`, `config`
- Registers `--cmd`-defined functions
- Each API group (environment, filesystem, command, git, color, cache) is a submodule
- **Depends on**: `config` (for color palette), `script` (for engine registration)

### `pipeline` — Segment pipeline
- Resolves which segments to run from config
- Executes segment scripts concurrently (thread pool)
- Evaluates format template, calling `s("name")` to retrieve segment output
- Enforces global timeout
- **Depends on**: `config`, `script`, `host`, `render`

### `render` — ANSI rendering
- Converts color names/definitions to ANSI escape sequences
- Handles kitty underline protocol
- Computes unicode display width for right-prompt alignment
- **Depends on**: nothing (leaf module)

## Dependency graph

```
cli
 └── pipeline
      ├── script
      │    └── config
      ├── host
      │    ├── config
      │    └── script
      ├── config
      └── render
```

Cycles are not permitted. `config` and `render` are leaf modules with no internal dependencies.

## Data flow for a prompt render

1. `cli` parses args, extracts `--cmd` definitions and `--right` flag
2. `config` loads and validates TOML
3. `pipeline` resolves segment list from format template
4. `host` registers all functions (built-in + `--cmd` defined) into `script` engine
5. `pipeline` spawns concurrent script evaluations via `script` engine
6. Each script calls `host` functions as needed, returns a string or `()`
7. `pipeline` evaluates the format template expression, calling `s("name")` for each segment
8. `render` converts the final string's color markers to ANSI escapes
9. `cli` writes to stdout

## Performance constraints

- Total render time budget: 50ms
- Per-`exec` call timeout: 10ms (configurable)
- Git operations use `git2` (libgit2), not subprocess
- Rhai ASTs are compiled once and cached on disk
- `git_status()`, `cwd()`, etc. are computed once per render and shared across segments
