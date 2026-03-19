mod cli;
mod config;
mod host;
mod lang;
mod pipeline;
mod render;
mod script;

fn main() {
    let args = cli::parse();
    if let Err(e) = cli::run(args) {
        eprintln!("promptorius: {e}");
        std::process::exit(1);
    }
}
