use clap::Parser;
use llm_wikis::{
    cli::{Cli, execute},
    config::{Environment, Platform},
    query::SystemRunner,
};

fn main() {
    let cli = Cli::parse();
    let mut runner = SystemRunner;
    match execute(
        cli,
        &mut runner,
        Platform::current(),
        &Environment::current(),
    ) {
        Ok(exit_code) => std::process::exit(exit_code),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(2);
        }
    }
}
