use clap::Parser;
use llm_wikis::{
    cli::{Cli, execute},
    config::{Environment, Platform},
    progress::TerminalQueryProgress,
    query::{StdioOutputSink, SystemRunner},
};

fn main() {
    let cli = Cli::parse();
    let mut runner = SystemRunner;
    let mut progress = TerminalQueryProgress::from_stdio();
    let mut output_sink = StdioOutputSink;
    match execute(
        cli,
        &mut runner,
        &mut progress,
        &mut output_sink,
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
