use std::{
    ffi::OsString,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use thiserror::Error;

use crate::config::{Config, ConfigError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputMode {
    Inherit,
}

#[derive(Debug)]
pub struct ProviderInvocation {
    executable: OsString,
    cwd: PathBuf,
    args: Vec<OsString>,
    stdin: Vec<u8>,
    stdout: OutputMode,
    stderr: OutputMode,
}

pub trait ProviderRunner {
    fn run(&mut self, invocation: ProviderInvocation) -> Result<i32, RunnerError>;
}

#[derive(Debug, Error)]
pub enum RunnerError {
    #[error("could not launch '{executable}': {source}")]
    Launch {
        executable: String,
        #[source]
        source: std::io::Error,
    },
    #[error("could not write to '{executable}' stdin: {source}")]
    Write {
        executable: String,
        #[source]
        source: std::io::Error,
    },
    #[error("could not wait for '{executable}': {source}")]
    Wait {
        executable: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
pub enum QueryError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("question must not be empty")]
    EmptyQuestion,
    #[error(transparent)]
    Runner(#[from] RunnerError),
}

pub fn run_query(
    config: &Config,
    wiki_id: &str,
    question_parts: &[String],
    runner: &mut impl ProviderRunner,
) -> Result<i32, QueryError> {
    let wiki = config.wiki(wiki_id)?;
    let question = question_parts.join(" ");
    if question.trim().is_empty() {
        return Err(QueryError::EmptyQuestion);
    }

    let mut prompt = Vec::new();
    prompt.extend_from_slice(wiki.entrypoint().as_bytes());
    prompt.push(b' ');
    prompt.extend_from_slice(question.as_bytes());
    prompt.push(b'\n');

    let invocation = ProviderInvocation {
        executable: OsString::from(config.claude_executable()),
        cwd: wiki.path().to_path_buf(),
        args: vec![
            OsString::from("-p"),
            OsString::from("--add-dir"),
            wiki.path().as_os_str().to_owned(),
        ],
        stdin: prompt,
        stdout: OutputMode::Inherit,
        stderr: OutputMode::Inherit,
    };

    runner.run(invocation).map_err(QueryError::from)
}

pub struct SystemRunner;

impl ProviderRunner for SystemRunner {
    fn run(&mut self, invocation: ProviderInvocation) -> Result<i32, RunnerError> {
        let executable = invocation.executable.to_string_lossy().into_owned();
        let mut command = Command::new(&invocation.executable);
        command
            .args(&invocation.args)
            .current_dir(&invocation.cwd)
            .stdin(Stdio::piped());
        match invocation.stdout {
            OutputMode::Inherit => {
                command.stdout(Stdio::inherit());
            }
        }
        match invocation.stderr {
            OutputMode::Inherit => {
                command.stderr(Stdio::inherit());
            }
        }

        let mut child = command.spawn().map_err(|source| RunnerError::Launch {
            executable: executable.clone(),
            source,
        })?;

        if let Some(mut stdin) = child.stdin.take() {
            stdin
                .write_all(&invocation.stdin)
                .map_err(|source| RunnerError::Write {
                    executable: executable.clone(),
                    source,
                })?;
        }

        let status = child
            .wait()
            .map_err(|source| RunnerError::Wait { executable, source })?;
        Ok(status.code().unwrap_or(1))
    }
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use crate::config::Config;

    use super::{
        OutputMode, ProviderInvocation, ProviderRunner, RunnerError, SystemRunner, run_query,
    };

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock should be after epoch")
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "llm-wikis-query-test-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&path).expect("test directory should be created");
            Self(path)
        }

        fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).expect("test directory should be removed");
        }
    }

    fn config(wiki_path: &Path, executable: &str) -> Config {
        let input = format!(
            r#"
config_version = 1
claude_executable = {executable:?}

[wikis.agents]
path = {}
entrypoint = "/wiki-query"
"#,
            toml::Value::String(wiki_path.display().to_string())
        );
        Config::parse(&input).expect("fixture config should parse")
    }

    #[derive(Default)]
    struct FakeRunner {
        invocation: Option<ProviderInvocation>,
        exit_code: i32,
    }

    impl ProviderRunner for FakeRunner {
        fn run(&mut self, invocation: ProviderInvocation) -> Result<i32, RunnerError> {
            self.invocation = Some(invocation);
            Ok(self.exit_code)
        }
    }

    #[test]
    fn builds_expected_claude_invocation_and_writes_question_only_to_stdin() {
        let wiki = TestDir::new();
        let config = config(wiki.path(), "custom-claude");
        let question = vec![
            "How".to_owned(),
            "is".to_owned(),
            "deployment".to_owned(),
            "configured?".to_owned(),
        ];
        let mut runner = FakeRunner::default();

        let exit_code =
            run_query(&config, "agents", &question, &mut runner).expect("query should run");

        assert_eq!(exit_code, 0);
        let invocation = runner.invocation.expect("runner should be invoked");
        assert_eq!(invocation.executable, OsString::from("custom-claude"));
        assert_eq!(invocation.cwd, wiki.path());
        assert_eq!(
            invocation.args,
            vec![
                OsString::from("-p"),
                OsString::from("--add-dir"),
                wiki.path().as_os_str().to_owned(),
            ]
        );
        assert_eq!(
            invocation.stdin,
            b"/wiki-query How is deployment configured?\n"
        );
        assert_eq!(invocation.stdout, OutputMode::Inherit);
        assert_eq!(invocation.stderr, OutputMode::Inherit);
        assert!(
            !invocation
                .args
                .iter()
                .any(|argument| argument.to_string_lossy().contains("deployment"))
        );
    }

    #[test]
    fn returns_provider_exit_code_unchanged() {
        let wiki = TestDir::new();
        let config = config(wiki.path(), "claude");
        let mut runner = FakeRunner {
            exit_code: 23,
            ..FakeRunner::default()
        };

        let exit_code = run_query(
            &config,
            "agents",
            &["What happened?".to_owned()],
            &mut runner,
        )
        .expect("query should run");

        assert_eq!(exit_code, 23);
    }

    #[test]
    fn rejects_unknown_wiki_without_invoking_provider() {
        let wiki = TestDir::new();
        let config = config(wiki.path(), "claude");
        let mut runner = FakeRunner::default();

        let error = run_query(
            &config,
            "missing",
            &["What happened?".to_owned()],
            &mut runner,
        )
        .expect_err("unknown wiki should fail");

        assert_eq!(error.to_string(), "unknown wiki 'missing'");
        assert!(runner.invocation.is_none());
    }

    #[test]
    fn rejects_empty_question_without_invoking_provider() {
        let wiki = TestDir::new();
        let config = config(wiki.path(), "claude");
        let mut runner = FakeRunner::default();

        let error =
            run_query(&config, "agents", &[], &mut runner).expect_err("empty question should fail");

        assert_eq!(error.to_string(), "question must not be empty");
        assert!(runner.invocation.is_none());
    }

    #[test]
    fn reports_missing_executable_as_launch_error() {
        let wiki = TestDir::new();
        let config = config(wiki.path(), "llm-wikis-test-executable-that-does-not-exist");
        let mut runner = SystemRunner;

        let error = run_query(
            &config,
            "agents",
            &["What happened?".to_owned()],
            &mut runner,
        )
        .expect_err("missing executable should fail");

        assert!(
            error
                .to_string()
                .contains("could not launch 'llm-wikis-test-executable-that-does-not-exist'")
        );
    }
}
