mod cli;
mod codegen;
mod compiler;
mod lang;

fn main() {
    let args = cli::parse();
    if let Err(e) = cli::run(args) {
        eprintln!("promptorius: {e}");
        std::process::exit(1);
    }
}
