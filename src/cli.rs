use std::path::PathBuf;

use clap::{Parser, Subcommand};
use thiserror::Error;

use crate::{
    config::{Config, ConfigError, Environment, Platform, default_config_path},
    query::{ProviderRunner, QueryError, run_query},
};

#[derive(Debug, Parser)]
#[command(
    name = "llm-wikis",
    version,
    about = "Query configured wikis through Claude Code"
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
            run_query(&config, &wiki, &question, runner).map_err(AppError::from)
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

    use clap::Parser;

    use crate::{
        config::{Environment, Platform},
        query::{ProviderInvocation, ProviderRunner, RunnerError},
    };

    use super::{Cli, Command, execute};

    #[derive(Default)]
    struct FakeRunner {
        invoked: bool,
    }

    impl ProviderRunner for FakeRunner {
        fn run(&mut self, _invocation: ProviderInvocation) -> Result<i32, RunnerError> {
            self.invoked = true;
            Ok(0)
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

        let error = execute(cli, &mut runner, Platform::Linux, &Environment::default())
            .expect_err("invalid config should fail");

        fs::remove_file(config_path).expect("fixture should be removed");
        assert!(error.to_string().contains("invalid config"));
        assert!(!runner.invoked);
    }
}
