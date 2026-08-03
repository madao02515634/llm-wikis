use std::path::PathBuf;

use clap::{Parser, Subcommand};
use thiserror::Error;

use crate::{
    config::{Config, ConfigError, Environment, Platform, default_config_path},
    progress::QueryProgress,
    query::{ProviderOutputSink, ProviderRunner, QueryError, run_query},
};

#[derive(Debug, Parser)]
#[command(
    name = "llm-wikis",
    version,
    about = "Query configured wikis through Claude Code",
    after_long_help = "Examples:\n  llm-wikis query --wiki agents -- \"What is context engineering?\"\n  llm-wikis --config ./config.toml query --wiki agents -- \"How is deployment configured?\""
)]
pub struct Cli {
    #[arg(long, global = true, value_name = "PATH")]
    pub config: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Query one configured wiki
    #[command(
        after_long_help = "Example:\n  llm-wikis query --wiki agents -- \"What is context engineering?\""
    )]
    Query {
        /// Configured wiki ID
        #[arg(long)]
        wiki: String,

        /// Question to ask
        #[arg(last = true, required = true)]
        question: Vec<String>,
    },
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error(transparent)]
    Query(#[from] QueryError),
}

pub fn execute(
    cli: Cli,
    runner: &mut impl ProviderRunner,
    progress: &mut impl QueryProgress,
    output_sink: &mut impl ProviderOutputSink,
    platform: Platform,
    environment: &Environment,
) -> Result<i32, AppError> {
    let config_path = match cli.config {
        Some(path) => path,
        None => default_config_path(platform, environment)?,
    };
    let config = Config::load(&config_path)?;

    match cli.command {
        Command::Query { wiki, question } => {
            run_query(&config, &wiki, &question, runner, progress, output_sink)
                .map_err(AppError::from)
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use clap::{CommandFactory, Parser};

    use crate::{
        config::{Environment, Platform},
        progress::QueryProgress,
        query::{
            ProviderInvocation, ProviderOutput, ProviderOutputSink, ProviderRunner, RunnerError,
        },
    };

    use super::{Cli, Command, execute};

    #[derive(Default)]
    struct FakeRunner {
        invoked: bool,
    }

    impl ProviderRunner for FakeRunner {
        fn run(&mut self, _invocation: ProviderInvocation) -> Result<ProviderOutput, RunnerError> {
            self.invoked = true;
            Ok(ProviderOutput {
                exit_code: 0,
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        }
    }

    #[derive(Default)]
    struct FakeProgress {
        invoked: bool,
    }

    impl QueryProgress for FakeProgress {
        fn start(&mut self, _wiki: &str) {
            self.invoked = true;
        }

        fn finish(&mut self) {}
    }

    #[derive(Default)]
    struct FakeOutputSink {
        invoked: bool,
    }

    impl ProviderOutputSink for FakeOutputSink {
        fn replay(&mut self, _output: &ProviderOutput) -> Result<(), RunnerError> {
            self.invoked = true;
            Ok(())
        }
    }

    fn temp_file() -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after epoch")
            .as_nanos();
        std::env::temp_dir().join(format!(
            "llm-wikis-cli-test-{}-{nonce}.toml",
            std::process::id()
        ))
    }

    #[test]
    fn root_help_contains_copyable_query_example() {
        let help = Cli::command().render_long_help().to_string();

        assert!(
            help.contains(r#"llm-wikis query --wiki agents -- "What is context engineering?""#)
        );
    }

    #[test]
    fn query_help_contains_copyable_query_example() {
        let mut command = Cli::command();
        let query = command
            .find_subcommand_mut("query")
            .expect("query subcommand should exist");
        let help = query.render_long_help().to_string();

        assert!(
            help.contains(r#"llm-wikis query --wiki agents -- "What is context engineering?""#)
        );
    }

    #[test]
    fn parses_explicit_config_and_query_command() {
        let cli = Cli::try_parse_from([
            "llm-wikis",
            "--config",
            "chosen.toml",
            "query",
            "--wiki",
            "agents",
            "--",
            "How",
            "is",
            "deployment",
            "configured?",
        ])
        .expect("CLI should parse");

        assert_eq!(cli.config, Some(PathBuf::from("chosen.toml")));
        let Command::Query { wiki, question } = cli.command;
        assert_eq!(wiki, "agents");
        assert_eq!(
            question,
            ["How", "is", "deployment", "configured?"].map(str::to_owned)
        );
    }

    #[test]
    fn reports_invalid_explicit_config_without_invoking_provider() {
        let config_path = temp_file();
        fs::write(&config_path, "not valid toml = [").expect("fixture should be written");
        let cli = Cli::try_parse_from([
            "llm-wikis",
            "--config",
            config_path.to_str().expect("test path should be UTF-8"),
            "query",
            "--wiki",
            "agents",
            "--",
            "question",
        ])
        .expect("CLI should parse");
        let mut runner = FakeRunner::default();
        let mut progress = FakeProgress::default();
        let mut output_sink = FakeOutputSink::default();

        let error = execute(
            cli,
            &mut runner,
            &mut progress,
            &mut output_sink,
            Platform::Linux,
            &Environment::default(),
        )
        .expect_err("invalid config should fail");

        fs::remove_file(config_path).expect("fixture should be removed");
        assert!(error.to_string().contains("invalid config"));
        assert!(!runner.invoked);
        assert!(!progress.invoked);
        assert!(!output_sink.invoked);
    }
}
