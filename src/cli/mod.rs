//! CLI argument parsing and subcommand dispatch.
//!
//! Handles `--right`, `--cmd`, and subcommands: `init`, `explain`, `check`,
//! `script new`, `completions`.

use clap::Parser;
use thiserror::Error;

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
    #[arg(long)]
    pub right: bool,

    /// Define a function callable from scripts: --cmd :type:name:value
    /// Supported types: str, int, float, bool.
    #[arg(long = "cmd", value_name = "CMD")]
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
            // TODO: implement explain
            Ok(())
        }
        Some(SubCommand::Check) => {
            // TODO: implement check
            Ok(())
        }
        Some(SubCommand::Script(ScriptCommand::New { name })) => {
            // TODO: implement script new
            let _ = name;
            Ok(())
        }
        Some(SubCommand::Completions { shell }) => {
            // TODO: implement completions
            let _ = shell;
            Ok(())
        }
        None => {
            // Default: render prompt
            // TODO: wire up pipeline
            let _ = args.right;
            let _ = args.cmds;
            Ok(())
        }
    }
}

fn init_shell(shell: &str) {
    match shell {
        "zsh" => print!("{}", include_str!("../shell/zsh.sh")),
        "bash" => print!("{}", include_str!("../shell/bash.sh")),
        "fish" => print!("{}", include_str!("../shell/fish.fish")),
        _ => eprintln!("promptorius: unsupported shell: {shell}"),
    }
}
