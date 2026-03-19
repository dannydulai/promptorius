//! CLI argument parsing and subcommand dispatch.
//!
//! Handles `--right`, `--cmd`, and subcommands: `init`, `explain`, `check`,
//! `script new`, `completions`.

use clap::Parser;
use std::collections::HashMap;
use thiserror::Error;

use crate::config;
use crate::host;
use crate::pipeline;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("config error: {0}")]
    Config(#[from] crate::config::ConfigError),

    #[error("pipeline error: {0}")]
    Pipeline(#[from] crate::pipeline::PipelineError),
}

/// A fast, Rhai-scriptable shell prompt engine.
#[derive(Parser, Debug)]
#[command(name = "promptorius", version)]
pub struct Args {
    /// Print the right prompt (right_format) instead of the left prompt.
    #[arg(long, global = true)]
    pub right: bool,

    /// Define a function callable from scripts: --cmd :type:name:value
    /// Supported types: str, int, float, bool.
    #[arg(long = "cmd", value_name = "CMD", global = true)]
    pub cmds: Vec<String>,

    #[command(subcommand)]
    pub command: Option<SubCommand>,
}

#[derive(clap::Subcommand, Debug)]
pub enum SubCommand {
    /// Print shell init script.
    Init {
        /// Shell name: bash, zsh, fish, nushell.
        shell: String,
    },
    /// Show what each segment resolved to and timing.
    Explain,
    /// Validate config and scripts, report errors.
    Check,
    /// Scaffold a new segment script.
    #[command(subcommand)]
    Script(ScriptCommand),
    /// Generate shell completions.
    Completions {
        /// Shell name: bash, zsh, fish, nushell.
        shell: String,
    },
}

#[derive(clap::Subcommand, Debug)]
pub enum ScriptCommand {
    /// Create a new segment script.
    New {
        /// Name of the segment.
        name: String,
    },
}

pub fn parse() -> Args {
    Args::parse()
}

pub fn run(args: Args) -> Result<(), CliError> {
    match args.command {
        Some(SubCommand::Init { shell }) => {
            init_shell(&shell);
            Ok(())
        }
        Some(SubCommand::Explain) => {
            run_explain(&args.cmds, args.right)
        }
        Some(SubCommand::Check) => {
            run_check()
        }
        Some(SubCommand::Script(ScriptCommand::New { name })) => {
            run_script_new(&name);
            Ok(())
        }
        Some(SubCommand::Completions { shell }) => {
            run_completions(&shell);
            Ok(())
        }
        None => {
            run_prompt(&args.cmds, args.right)
        }
    }
}

fn run_prompt(cmd_args: &[String], right: bool) -> Result<(), CliError> {
    let cfg = config::load()?;
    let cmds = parse_cmds(cmd_args);
    let mut script_dirs = config::script_search_paths(&cfg.settings);
    append_stdlib_dir(&mut script_dirs);
    let stdlib = stdlib_scripts();

    let output = pipeline::render(&cfg, &cmds, right, &script_dirs, &stdlib)?;
    print!("{output}");
    Ok(())
}

fn run_explain(cmd_args: &[String], right: bool) -> Result<(), CliError> {
    let cfg = config::load()?;
    // Strip shell cmd so explain outputs raw ANSI (not %{...%} wrapped)
    let cmds: Vec<_> = parse_cmds(cmd_args)
        .into_iter()
        .filter(|c| c.name != "shell")
        .collect();
    let mut script_dirs = config::script_search_paths(&cfg.settings);
    append_stdlib_dir(&mut script_dirs);
    let stdlib = stdlib_scripts();

    let (output, stats) =
        pipeline::render_with_stats(&cfg, &cmds, right, &script_dirs, &stdlib)?;

    eprintln!("--- promptorius explain ---");
    eprintln!();

    // Per-segment breakdown, sorted by duration descending
    let mut segs = stats.segments.clone();
    segs.sort_by(|a, b| b.total_us.cmp(&a.total_us));

    for seg in &segs {
        let total_ms = seg.total_us as f64 / 1000.0;
        let compile_ms = seg.compile_us as f64 / 1000.0;
        let exec_ms = seg.exec_us as f64 / 1000.0;
        let status = if let Some(err) = &seg.error {
            format!("ERROR: {err}")
        } else if seg.had_output {
            format!("{} bytes", seg.output_len)
        } else {
            "hidden".to_string()
        };
        eprintln!(
            "  {:>7.2}ms  {:<20} compile={:.2}ms exec={:.2}ms  {}",
            total_ms, seg.name, compile_ms, exec_ms, status
        );
    }

    let seg_total_ms = segs.iter().map(|s| s.total_us).sum::<u128>() as f64 / 1000.0;
    let engine_ms = stats.engine_setup_us as f64 / 1000.0;
    let template_ms = stats.template_eval_us as f64 / 1000.0;
    let total_ms = stats.total_us as f64 / 1000.0;

    eprintln!();
    eprintln!("  {:>7.2}ms  engine setup", engine_ms);
    eprintln!("  {:>7.2}ms  segments total", seg_total_ms);
    eprintln!("  {:>7.2}ms  template eval", template_ms);
    eprintln!("  {:>7.2}ms  total", total_ms);
    eprintln!("---");
    print!("{output}");
    Ok(())
}

fn run_check() -> Result<(), CliError> {
    let cfg = config::load()?;
    let mut script_dirs = config::script_search_paths(&cfg.settings);
    append_stdlib_dir(&mut script_dirs);
    let stdlib = stdlib_scripts();

    eprintln!("config: ok ({})", config::config_path().display());
    eprintln!("segments: {}", cfg.segments.len());

    for (name, seg) in &cfg.segments {
        let default_script = format!("{name}.rhai");
        let script_name = seg.script.as_deref().unwrap_or(&default_script);
        let found = script_dirs.iter().any(|d| d.join(script_name).exists())
            || stdlib.contains_key(script_name);
        if found {
            eprintln!("  {name}: ok ({script_name})");
        } else {
            eprintln!("  {name}: MISSING ({script_name})");
        }
    }

    for color_name in cfg.colors.keys() {
        eprintln!("  color/{color_name}: ok");
    }

    eprintln!("check complete.");
    Ok(())
}

fn run_script_new(name: &str) {
    let config_dir = config::config_path()
        .parent()
        .map(|p| p.join("scripts"))
        .unwrap_or_default();

    let path = config_dir.join(format!("{name}.rhai"));
    if path.exists() {
        eprintln!("promptorius: script already exists: {}", path.display());
        return;
    }

    if let Err(e) = std::fs::create_dir_all(&config_dir) {
        eprintln!("promptorius: failed to create scripts dir: {e}");
        return;
    }

    let template = format!(
        r#"// {name} segment
// Return a string to display, or () to hide this segment.

let result = "";

// TODO: implement {name} segment logic

result
"#
    );

    match std::fs::write(&path, template) {
        Ok(_) => eprintln!("created: {}", path.display()),
        Err(e) => eprintln!("promptorius: failed to write script: {e}"),
    }
}

fn run_completions(shell: &str) {
    use clap::CommandFactory;
    let mut cmd = Args::command();
    let shell = match shell {
        "bash" => clap_complete::Shell::Bash,
        "zsh" => clap_complete::Shell::Zsh,
        "fish" => clap_complete::Shell::Fish,
        _ => {
            eprintln!("promptorius: unsupported shell for completions: {shell}");
            return;
        }
    };
    clap_complete::generate(shell, &mut cmd, "promptorius", &mut std::io::stdout());
}

fn init_shell(shell: &str) {
    match shell {
        "zsh" => print!("{}", include_str!("../shell/zsh.sh")),
        "bash" => print!("{}", include_str!("../shell/bash.sh")),
        "fish" => print!("{}", include_str!("../shell/fish.fish")),
        "nushell" | "nu" => print!("{}", include_str!("../shell/nushell.nu")),
        _ => eprintln!("promptorius: unsupported shell: {shell}"),
    }
}

fn parse_cmds(args: &[String]) -> Vec<host::CmdDef> {
    args.iter()
        .filter_map(|s| host::parse_cmd(s))
        .collect()
}

/// In debug builds, add `./stdlib/` next to the binary as a search path
/// so scripts can be edited without rebuilding.
fn append_stdlib_dir(dirs: &mut Vec<std::path::PathBuf>) {
    #[cfg(debug_assertions)]
    {
        // Check for stdlib/ relative to the cargo manifest dir (repo root)
        let manifest_stdlib = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("stdlib");
        if manifest_stdlib.is_dir() {
            dirs.push(manifest_stdlib);
        }
    }
}

/// Bundled stdlib scripts, embedded at compile time.
/// In debug builds these serve as fallback — the filesystem copy takes priority.
fn stdlib_scripts() -> HashMap<String, &'static str> {
    let mut m = HashMap::new();
    m.insert("directory.rhai".to_string(), include_str!("../../stdlib/directory.rhai"));
    m.insert("git.rhai".to_string(), include_str!("../../stdlib/git.rhai"));
    m.insert("language.rhai".to_string(), include_str!("../../stdlib/language.rhai"));
    m.insert("time.rhai".to_string(), include_str!("../../stdlib/time.rhai"));
    m.insert("duration.rhai".to_string(), include_str!("../../stdlib/duration.rhai"));
    m.insert("exitcode.rhai".to_string(), include_str!("../../stdlib/exitcode.rhai"));
    m.insert("character.rhai".to_string(), include_str!("../../stdlib/character.rhai"));
    m.insert("jobs.rhai".to_string(), include_str!("../../stdlib/jobs.rhai"));
    m.insert("user_host.rhai".to_string(), include_str!("../../stdlib/user_host.rhai"));
    m.insert("battery.rhai".to_string(), include_str!("../../stdlib/battery.rhai"));
    m
}
