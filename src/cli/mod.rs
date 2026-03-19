//! CLI argument parsing and subcommand dispatch.

use clap::Parser;
use thiserror::Error;

use crate::compiler;

#[derive(Error, Debug)]
pub enum CliError {
    #[error("{0}")]
    Compile(#[from] compiler::CompileError),

    #[error("{0}")]
    Parse(#[from] crate::lang::parser::ParseError),

    #[error("{0}")]
    Io(#[from] std::io::Error),
}

/// A compiled, scriptable shell prompt engine.
#[derive(Parser, Debug)]
#[command(name = "promptorius", version)]
pub struct Args {
    #[command(subcommand)]
    pub command: Option<SubCommand>,
}

#[derive(clap::Subcommand, Debug)]
pub enum SubCommand {
    /// Compile the prompt script to a native binary.
    Compile {
        /// Script file (default: $XDG_CONFIG_HOME/promptorius/config).
        script: Option<String>,
        /// Output binary path (default: $XDG_DATA_HOME/promptorius/__promptorius_output).
        output: Option<String>,
    },
    /// Remove the build directory and force a full rebuild.
    Clean,
    /// Print shell init script.
    Init {
        /// Shell name: bash, zsh, fish, nushell.
        shell: String,
    },
    /// Validate script syntax without building.
    Check {
        /// Script file (default: $XDG_CONFIG_HOME/promptorius/config).
        script: Option<String>,
    },
    /// Build an instrumented binary and show timing breakdown.
    Explain {
        /// Additional --var args to pass to the explanation binary.
        #[arg(long = "var", value_name = "VAR")]
        vars: Vec<String>,
        /// Show right prompt timing.
        #[arg(long)]
        right: bool,
    },
    /// Generate shell completions.
    Completions {
        /// Shell name: bash, zsh, fish.
        shell: String,
    },
}

pub fn parse() -> Args {
    Args::parse()
}

pub fn run(args: Args) -> Result<(), CliError> {
    match args.command {
        Some(SubCommand::Compile { script, output }) => {
            let script_path = script
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| {
                    compiler::ensure_default_config().unwrap_or_else(|_| compiler::default_script_path())
                });
            let output_path = output
                .map(std::path::PathBuf::from)
                .unwrap_or_else(compiler::default_output_path);

            match compiler::compile(&script_path, &output_path) {
                Ok(()) => Ok(()),
                Err(compiler::CompileError::Build(msg)) => {
                    eprintln!("promptorius: build failed:\n{msg}");
                    eprintln!();
                    eprintln!("Run 'promptorius clean' to remove the build cache and try again.");
                    Err(compiler::CompileError::Build("build failed".to_string()).into())
                }
                Err(e) => Err(e.into()),
            }
        }
        Some(SubCommand::Clean) => {
            compiler::clean()?;
            Ok(())
        }
        Some(SubCommand::Init { shell }) => {
            init_shell(&shell);
            Ok(())
        }
        Some(SubCommand::Check { script }) => {
            let script_path = script
                .map(std::path::PathBuf::from)
                .unwrap_or_else(compiler::default_script_path);
            run_check(&script_path)
        }
        Some(SubCommand::Explain { vars, right }) => {
            run_explain(&vars, right)
        }
        Some(SubCommand::Completions { shell }) => {
            run_completions(&shell);
            Ok(())
        }
        None => {
            // Default: compile (same as `promptorius compile`)
            let script_path = compiler::ensure_default_config()
                .unwrap_or_else(|_| compiler::default_script_path());
            let output_path = compiler::default_output_path();
            compiler::compile(&script_path, &output_path)?;
            Ok(())
        }
    }
}

fn run_check(script_path: &std::path::Path) -> Result<(), CliError> {
    let source = std::fs::read_to_string(script_path)?;
    match crate::lang::parser::Parser::parse(&source) {
        Ok(program) => {
            let fn_names: Vec<&str> = program.stmts.iter().filter_map(|s| {
                if let crate::lang::ast::Stmt::FnDef { name, .. } = s { Some(name.as_str()) } else { None }
            }).collect();

            println!("syntax: ok");
            println!("file: {}", script_path.display());
            println!("functions: {}", fn_names.join(", "));

            if !fn_names.contains(&"left_prompt") {
                println!("WARNING: missing left_prompt() function");
            }
            if !fn_names.contains(&"right_prompt") {
                println!("WARNING: missing right_prompt() function");
            }

            Ok(())
        }
        Err(e) => {
            println!("syntax error: {e}");
            Err(e.into())
        }
    }
}

fn run_explain(vars: &[String], right: bool) -> Result<(), CliError> {
    let script_path = compiler::ensure_default_config()
        .unwrap_or_else(|_| compiler::default_script_path());
    let output_path = compiler::data_dir().join("__promptorius_explanation");

    // Compile instrumented binary
    let source = std::fs::read_to_string(&script_path)?;
    let program = crate::lang::parser::Parser::parse(&source)?;
    let rust_source = crate::codegen::generate_instrumented(&program);

    let build_dir = crate::compiler::project::ensure_build_project()?;
    crate::compiler::project::write_source(&build_dir, &rust_source)?;

    eprintln!("promptorius: building instrumented binary");
    crate::compiler::project::build(&build_dir)?;
    crate::compiler::project::copy_binary(&build_dir, &output_path)?;

    // Run the instrumented binary
    let mut cmd = std::process::Command::new(&output_path);
    if right {
        cmd.arg("--right");
    }
    for v in vars {
        cmd.arg("--var").arg(v);
    }

    let status = cmd.status()?;
    if !status.success() {
        eprintln!("promptorius: explain binary exited with {status}");
    }

    Ok(())
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
