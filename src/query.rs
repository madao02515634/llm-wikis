use std::{
    ffi::OsString,
    io::{self, Read, Write},
    path::PathBuf,
    process::{Command, Stdio},
    thread::{self, JoinHandle},
};

use thiserror::Error;

use crate::{
    config::{Config, ConfigError},
    progress::QueryProgress,
};

const CLAUDE_QUERY_SYSTEM_PROMPT: &str = concat!(
    "This is a single-turn, read-only, non-interactive knowledge query. ",
    "Answer completely and directly. Do not ask follow-up questions. ",
    "Do not ask whether to save, record, index, or log the result. ",
    "Do not offer or perform any wiki write-back. ",
    "Do not create, edit, delete, index, or log files. ",
    "Ignore workflow instructions that require recording or confirmation. ",
    "End after the answer."
);

#[derive(Debug, PartialEq, Eq)]
pub struct ProviderOutput {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

#[derive(Debug)]
pub struct ProviderInvocation {
    executable: OsString,
    cwd: PathBuf,
    args: Vec<OsString>,
    stdin: Vec<u8>,
}

pub trait ProviderRunner {
    fn run(&mut self, invocation: ProviderInvocation) -> Result<ProviderOutput, RunnerError>;
}

pub trait ProviderOutputSink {
    fn replay(&mut self, output: &ProviderOutput) -> Result<(), RunnerError>;
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
    #[error("could not replay provider stdout")]
    Stdout {
        #[source]
        source: std::io::Error,
    },
    #[error("could not replay provider stderr")]
    Stderr {
        #[source]
        source: std::io::Error,
    },
    #[error("could not capture provider stdout")]
    CaptureStdout {
        #[source]
        source: std::io::Error,
    },
    #[error("could not capture provider stderr")]
    CaptureStderr {
        #[source]
        source: std::io::Error,
    },
    #[error("could not join provider stdout capture")]
    JoinStdout,
    #[error("could not join provider stderr capture")]
    JoinStderr,
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
    progress: &mut impl QueryProgress,
    output_sink: &mut impl ProviderOutputSink,
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
            OsString::from("--append-system-prompt"),
            OsString::from(CLAUDE_QUERY_SYSTEM_PROMPT),
            OsString::from("--add-dir"),
            wiki.path().as_os_str().to_owned(),
        ],
        stdin: prompt,
    };

    progress.start(wiki_id);
    let provider_result = runner.run(invocation);
    progress.finish();
    let output = provider_result?;
    output_sink.replay(&output)?;
    Ok(output.exit_code)
}

pub struct SystemRunner;

impl ProviderRunner for SystemRunner {
    fn run(&mut self, invocation: ProviderInvocation) -> Result<ProviderOutput, RunnerError> {
        let executable = invocation.executable.to_string_lossy().into_owned();
        let mut command = Command::new(&invocation.executable);
        command
            .args(&invocation.args)
            .current_dir(&invocation.cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|source| RunnerError::Launch {
            executable: executable.clone(),
            source,
        })?;

        let stdout = child
            .stdout
            .take()
            .expect("stdout must be available after configuring a pipe");
        let stdout_capture = match thread::Builder::new()
            .name("provider-stdout-capture".to_owned())
            .spawn(move || capture_provider_stream(stdout))
        {
            Ok(capture) => capture,
            Err(source) => {
                terminate_child(&mut child);
                return Err(RunnerError::CaptureStdout { source });
            }
        };
        let stderr = child
            .stderr
            .take()
            .expect("stderr must be available after configuring a pipe");
        let stderr_capture = match thread::Builder::new()
            .name("provider-stderr-capture".to_owned())
            .spawn(move || capture_provider_stream(stderr))
        {
            Ok(capture) => capture,
            Err(source) => {
                terminate_child(&mut child);
                let _ = finish_provider_capture(stdout_capture, ProviderStream::Stdout);
                return Err(RunnerError::CaptureStderr { source });
            }
        };

        let stdin_result = child
            .stdin
            .take()
            .map(|mut stdin| stdin.write_all(&invocation.stdin))
            .unwrap_or(Ok(()));
        if let Err(source) = stdin_result {
            terminate_child(&mut child);
            let _ = finish_provider_capture(stdout_capture, ProviderStream::Stdout);
            let _ = finish_provider_capture(stderr_capture, ProviderStream::Stderr);
            return Err(RunnerError::Write { executable, source });
        }

        let status = match child.wait() {
            Ok(status) => status,
            Err(source) => {
                terminate_child(&mut child);
                let _ = finish_provider_capture(stdout_capture, ProviderStream::Stdout);
                let _ = finish_provider_capture(stderr_capture, ProviderStream::Stderr);
                return Err(RunnerError::Wait { executable, source });
            }
        };
        let stdout = finish_provider_capture(stdout_capture, ProviderStream::Stdout);
        let stderr = finish_provider_capture(stderr_capture, ProviderStream::Stderr);
        Ok(ProviderOutput {
            exit_code: status.code().unwrap_or(1),
            stdout: stdout?,
            stderr: stderr?,
        })
    }
}

#[derive(Clone, Copy)]
enum ProviderStream {
    Stdout,
    Stderr,
}

fn capture_provider_stream(mut stream: impl Read) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    stream.read_to_end(&mut bytes)?;
    Ok(bytes)
}

fn finish_provider_capture(
    capture: JoinHandle<io::Result<Vec<u8>>>,
    stream: ProviderStream,
) -> Result<Vec<u8>, RunnerError> {
    match capture.join() {
        Ok(Ok(bytes)) => Ok(bytes),
        Ok(Err(source)) => match stream {
            ProviderStream::Stdout => Err(RunnerError::CaptureStdout { source }),
            ProviderStream::Stderr => Err(RunnerError::CaptureStderr { source }),
        },
        Err(_) => match stream {
            ProviderStream::Stdout => Err(RunnerError::JoinStdout),
            ProviderStream::Stderr => Err(RunnerError::JoinStderr),
        },
    }
}

fn terminate_child(child: &mut std::process::Child) {
    let _ = child.kill();
    let _ = child.wait();
}

pub struct StdioOutputSink;

impl ProviderOutputSink for StdioOutputSink {
    fn replay(&mut self, output: &ProviderOutput) -> Result<(), RunnerError> {
        let stdout = io::stdout();
        let stderr = io::stderr();
        replay_provider_output(output, &mut stdout.lock(), &mut stderr.lock())
    }
}

fn replay_provider_output<Stdout, Stderr>(
    output: &ProviderOutput,
    stdout: &mut Stdout,
    stderr: &mut Stderr,
) -> Result<(), RunnerError>
where
    Stdout: Write,
    Stderr: Write,
{
    stdout
        .write_all(&output.stdout)
        .and_then(|()| stdout.flush())
        .map_err(|source| RunnerError::Stdout { source })?;
    stderr
        .write_all(&output.stderr)
        .and_then(|()| stderr.flush())
        .map_err(|source| RunnerError::Stderr { source })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::{
        cell::RefCell,
        ffi::OsString,
        fs,
        io::{self, Read, Write},
        path::{Path, PathBuf},
        rc::Rc,
        sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        },
        thread,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };

    use crate::config::Config;
    use crate::progress::QueryProgress;

    use super::{
        ProviderInvocation, ProviderOutput, ProviderOutputSink, ProviderRunner, ProviderStream,
        RunnerError, SystemRunner, capture_provider_stream, finish_provider_capture,
        replay_provider_output, run_query,
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
        fn run(&mut self, invocation: ProviderInvocation) -> Result<ProviderOutput, RunnerError> {
            self.invocation = Some(invocation);
            Ok(ProviderOutput {
                exit_code: self.exit_code,
                stdout: Vec::new(),
                stderr: Vec::new(),
            })
        }
    }

    #[derive(Default)]
    struct FakeProgress {
        starts: Vec<String>,
        finish_count: usize,
    }

    impl QueryProgress for FakeProgress {
        fn start(&mut self, wiki: &str) {
            self.starts.push(wiki.to_owned());
        }

        fn finish(&mut self) {
            self.finish_count += 1;
        }
    }

    #[derive(Default)]
    struct FakeOutputSink {
        replayed: bool,
    }

    impl ProviderOutputSink for FakeOutputSink {
        fn replay(&mut self, _output: &ProviderOutput) -> Result<(), RunnerError> {
            self.replayed = true;
            Ok(())
        }
    }

    type Events = Rc<RefCell<Vec<&'static str>>>;

    const DUPLEX_BYTES: usize = 8 * 1024 * 1024;
    const DUPLEX_STDOUT_SENTINEL: &[u8] = b"LLM_WIKIS_DUPLEX_STDOUT_COMPLETE";
    const DUPLEX_STDERR_SENTINEL: &[u8] = b"LLM_WIKIS_DUPLEX_STDERR_COMPLETE";
    const SUCCESS_STDIN: &[u8] = b"LLM_WIKIS_SUCCESS_STDIN";
    const SUCCESS_STDOUT_SENTINEL: &[u8] = b"LLM_WIKIS_SUCCESS_STDOUT";
    const SUCCESS_STDERR_SENTINEL: &[u8] = b"LLM_WIKIS_SUCCESS_STDERR";
    const NONZERO_STDOUT_SENTINEL: &[u8] = b"LLM_WIKIS_NONZERO_STDOUT";
    const NONZERO_STDERR_SENTINEL: &[u8] = b"LLM_WIKIS_NONZERO_STDERR";

    fn subprocess_invocation(helper_name: &str, stdin: Vec<u8>) -> ProviderInvocation {
        let executable = std::env::current_exe().expect("current test executable should exist");
        ProviderInvocation {
            executable: executable.into_os_string(),
            cwd: std::env::current_dir().expect("current test directory should exist"),
            args: vec![
                OsString::from("--ignored"),
                OsString::from("--exact"),
                OsString::from(format!("query::tests::{helper_name}")),
                OsString::from("--nocapture"),
            ],
            stdin,
        }
    }

    fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
        haystack
            .windows(needle.len())
            .any(|window| window == needle)
    }

    struct RecordingProgress {
        events: Events,
    }

    impl QueryProgress for RecordingProgress {
        fn start(&mut self, _wiki: &str) {
            self.events.borrow_mut().push("progress:start");
        }

        fn finish(&mut self) {
            self.events.borrow_mut().push("progress:finish");
        }
    }

    struct RecordingRunner {
        events: Events,
        result: Result<ProviderOutput, RunnerError>,
    }

    impl ProviderRunner for RecordingRunner {
        fn run(&mut self, _invocation: ProviderInvocation) -> Result<ProviderOutput, RunnerError> {
            self.events.borrow_mut().push("runner");
            std::mem::replace(
                &mut self.result,
                Ok(ProviderOutput {
                    exit_code: 0,
                    stdout: Vec::new(),
                    stderr: Vec::new(),
                }),
            )
        }
    }

    struct RecordingOutputSink {
        events: Events,
        stdout: Vec<u8>,
        stderr: Vec<u8>,
    }

    impl ProviderOutputSink for RecordingOutputSink {
        fn replay(&mut self, output: &ProviderOutput) -> Result<(), RunnerError> {
            self.events.borrow_mut().push("output");
            self.stdout.extend_from_slice(&output.stdout);
            self.stderr.extend_from_slice(&output.stderr);
            Ok(())
        }
    }

    struct FailingWriter {
        fail_on: FailurePoint,
    }

    enum FailurePoint {
        Write,
        Flush,
    }

    impl Write for FailingWriter {
        fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
            match self.fail_on {
                FailurePoint::Write => Err(io::Error::other("injected write failure")),
                FailurePoint::Flush => Ok(buffer.len()),
            }
        }

        fn flush(&mut self) -> io::Result<()> {
            match self.fail_on {
                FailurePoint::Write => Ok(()),
                FailurePoint::Flush => Err(io::Error::other("injected flush failure")),
            }
        }
    }

    struct FailingReader;

    impl Read for FailingReader {
        fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
            Err(io::Error::other("injected capture failure"))
        }
    }

    fn recording_components(
        result: Result<ProviderOutput, RunnerError>,
    ) -> (
        RecordingRunner,
        RecordingProgress,
        RecordingOutputSink,
        Events,
    ) {
        let events = Rc::new(RefCell::new(Vec::new()));
        (
            RecordingRunner {
                events: Rc::clone(&events),
                result,
            },
            RecordingProgress {
                events: Rc::clone(&events),
            },
            RecordingOutputSink {
                events: Rc::clone(&events),
                stdout: Vec::new(),
                stderr: Vec::new(),
            },
            events,
        )
    }

    #[test]
    fn clears_progress_before_replaying_successful_provider_output() {
        let wiki = TestDir::new();
        let config = config(wiki.path(), "claude");
        let output = ProviderOutput {
            exit_code: 0,
            stdout: b"provider answer\n".to_vec(),
            stderr: b"provider diagnostic\n".to_vec(),
        };
        let (mut runner, mut progress, mut sink, events) = recording_components(Ok(output));

        let exit_code = run_query(
            &config,
            "agents",
            &["What happened?".to_owned()],
            &mut runner,
            &mut progress,
            &mut sink,
        )
        .expect("query should run");

        assert_eq!(exit_code, 0);
        assert_eq!(sink.stdout, b"provider answer\n");
        assert_eq!(sink.stderr, b"provider diagnostic\n");
        assert_eq!(
            &*events.borrow(),
            &["progress:start", "runner", "progress:finish", "output"]
        );
    }

    #[test]
    fn clears_progress_before_replaying_nonzero_provider_output() {
        let wiki = TestDir::new();
        let config = config(wiki.path(), "claude");
        let output = ProviderOutput {
            exit_code: 23,
            stdout: b"partial answer\n".to_vec(),
            stderr: b"provider failed\n".to_vec(),
        };
        let (mut runner, mut progress, mut sink, events) = recording_components(Ok(output));

        let exit_code = run_query(
            &config,
            "agents",
            &["What happened?".to_owned()],
            &mut runner,
            &mut progress,
            &mut sink,
        )
        .expect("provider nonzero status should still replay output");

        assert_eq!(exit_code, 23);
        assert_eq!(sink.stdout, b"partial answer\n");
        assert_eq!(sink.stderr, b"provider failed\n");
        assert_eq!(
            &*events.borrow(),
            &["progress:start", "runner", "progress:finish", "output"]
        );
    }

    #[test]
    fn clears_progress_before_exposing_runner_error_without_replaying_output() {
        let wiki = TestDir::new();
        let config = config(wiki.path(), "claude");
        let runner_error = RunnerError::Launch {
            executable: "claude".to_owned(),
            source: io::Error::new(io::ErrorKind::NotFound, "injected launch failure"),
        };
        let (mut runner, mut progress, mut sink, events) = recording_components(Err(runner_error));

        let error = run_query(
            &config,
            "agents",
            &["What happened?".to_owned()],
            &mut runner,
            &mut progress,
            &mut sink,
        )
        .expect_err("runner error should be returned");

        assert!(error.to_string().contains("injected launch failure"));
        assert!(sink.stdout.is_empty());
        assert!(sink.stderr.is_empty());
        assert_eq!(
            &*events.borrow(),
            &["progress:start", "runner", "progress:finish"]
        );
    }

    #[test]
    fn replay_provider_output_routes_bytes_to_their_original_streams() {
        let output = ProviderOutput {
            exit_code: 0,
            stdout: b"stdout bytes\n".to_vec(),
            stderr: b"stderr bytes\n".to_vec(),
        };
        let mut stdout = Vec::new();
        let mut stderr = Vec::new();

        replay_provider_output(&output, &mut stdout, &mut stderr)
            .expect("output should be replayed");

        assert_eq!(stdout, b"stdout bytes\n");
        assert_eq!(stderr, b"stderr bytes\n");
    }

    #[test]
    fn replay_provider_output_bounds_stdout_write_errors() {
        let secret_output = b"sensitive stdout that must not appear in errors";
        let output = ProviderOutput {
            exit_code: 0,
            stdout: secret_output.to_vec(),
            stderr: Vec::new(),
        };
        let mut stdout = FailingWriter {
            fail_on: FailurePoint::Write,
        };
        let mut stderr = Vec::new();

        let error = replay_provider_output(&output, &mut stdout, &mut stderr)
            .expect_err("stdout write should fail");

        assert!(matches!(error, RunnerError::Stdout { .. }));
        assert_eq!(error.to_string(), "could not replay provider stdout");
    }

    #[test]
    fn replay_provider_output_bounds_stdout_flush_errors() {
        let output = ProviderOutput {
            exit_code: 0,
            stdout: b"stdout bytes".to_vec(),
            stderr: Vec::new(),
        };
        let mut stdout = FailingWriter {
            fail_on: FailurePoint::Flush,
        };
        let mut stderr = Vec::new();

        let error = replay_provider_output(&output, &mut stdout, &mut stderr)
            .expect_err("stdout flush should fail");

        assert!(matches!(error, RunnerError::Stdout { .. }));
        assert_eq!(error.to_string(), "could not replay provider stdout");
    }

    #[test]
    fn replay_provider_output_bounds_stderr_write_errors() {
        let secret_output = b"sensitive stderr that must not appear in errors";
        let output = ProviderOutput {
            exit_code: 0,
            stdout: Vec::new(),
            stderr: secret_output.to_vec(),
        };
        let mut stdout = Vec::new();
        let mut stderr = FailingWriter {
            fail_on: FailurePoint::Write,
        };

        let error = replay_provider_output(&output, &mut stdout, &mut stderr)
            .expect_err("stderr write should fail");

        assert!(matches!(error, RunnerError::Stderr { .. }));
        assert_eq!(error.to_string(), "could not replay provider stderr");
    }

    #[test]
    fn replay_provider_output_bounds_stderr_flush_errors() {
        let output = ProviderOutput {
            exit_code: 0,
            stdout: Vec::new(),
            stderr: b"stderr bytes".to_vec(),
        };
        let mut stdout = Vec::new();
        let mut stderr = FailingWriter {
            fail_on: FailurePoint::Flush,
        };

        let error = replay_provider_output(&output, &mut stdout, &mut stderr)
            .expect_err("stderr flush should fail");

        assert!(matches!(error, RunnerError::Stderr { .. }));
        assert_eq!(error.to_string(), "could not replay provider stderr");
    }

    #[test]
    fn provider_capture_bounds_stdout_read_errors() {
        let capture = thread::spawn(|| capture_provider_stream(FailingReader));

        let error = finish_provider_capture(capture, ProviderStream::Stdout)
            .expect_err("stdout capture should fail");

        assert!(matches!(error, RunnerError::CaptureStdout { .. }));
        assert_eq!(error.to_string(), "could not capture provider stdout");
    }

    #[test]
    fn provider_capture_bounds_stderr_read_errors() {
        let capture = thread::spawn(|| capture_provider_stream(FailingReader));

        let error = finish_provider_capture(capture, ProviderStream::Stderr)
            .expect_err("stderr capture should fail");

        assert!(matches!(error, RunnerError::CaptureStderr { .. }));
        assert_eq!(error.to_string(), "could not capture provider stderr");
    }

    #[test]
    fn provider_capture_bounds_stdout_thread_panics() {
        let capture =
            thread::spawn(|| -> io::Result<Vec<u8>> { panic!("injected stdout capture panic") });

        let error = finish_provider_capture(capture, ProviderStream::Stdout)
            .expect_err("stdout capture panic should fail");

        assert!(matches!(error, RunnerError::JoinStdout));
        assert_eq!(error.to_string(), "could not join provider stdout capture");
    }

    #[test]
    fn provider_capture_bounds_stderr_thread_panics() {
        let capture =
            thread::spawn(|| -> io::Result<Vec<u8>> { panic!("injected stderr capture panic") });

        let error = finish_provider_capture(capture, ProviderStream::Stderr)
            .expect_err("stderr capture panic should fail");

        assert!(matches!(error, RunnerError::JoinStderr));
        assert_eq!(error.to_string(), "could not join provider stderr capture");
    }

    #[test]
    fn system_runner_drains_duplex_pipes_while_writing_large_stdin() {
        let invocation = subprocess_invocation(
            "system_runner_fixture_duplex_backpressure",
            vec![b'I'; DUPLEX_BYTES],
        );
        let mut runner = SystemRunner;

        let output = runner
            .run(invocation)
            .expect("duplex subprocess should not deadlock or fail");

        assert_eq!(output.exit_code, 0);
        assert!(contains_bytes(&output.stdout, DUPLEX_STDOUT_SENTINEL));
        assert!(contains_bytes(&output.stderr, DUPLEX_STDERR_SENTINEL));
    }

    #[test]
    fn system_runner_captures_successful_subprocess_output_after_stdin_eof() {
        let invocation = subprocess_invocation(
            "system_runner_fixture_successful_capture",
            SUCCESS_STDIN.to_vec(),
        );
        let mut runner = SystemRunner;

        let output = runner
            .run(invocation)
            .expect("successful subprocess should be captured");

        assert_eq!(output.exit_code, 0);
        assert!(contains_bytes(&output.stdout, SUCCESS_STDOUT_SENTINEL));
        assert!(contains_bytes(&output.stderr, SUCCESS_STDERR_SENTINEL));
        assert!(!contains_bytes(&output.stdout, SUCCESS_STDERR_SENTINEL));
        assert!(!contains_bytes(&output.stderr, SUCCESS_STDOUT_SENTINEL));
    }

    #[test]
    fn system_runner_preserves_nonzero_exit_and_captured_streams() {
        let invocation = subprocess_invocation(
            "system_runner_fixture_nonzero_capture",
            SUCCESS_STDIN.to_vec(),
        );
        let mut runner = SystemRunner;

        let output = runner
            .run(invocation)
            .expect("nonzero subprocess output should be captured");

        assert_eq!(output.exit_code, 23);
        assert!(contains_bytes(&output.stdout, NONZERO_STDOUT_SENTINEL));
        assert!(contains_bytes(&output.stderr, NONZERO_STDERR_SENTINEL));
    }

    #[test]
    fn system_runner_reaps_child_after_stdin_write_failure() {
        let invocation = subprocess_invocation(
            "system_runner_fixture_exits_without_reading_stdin",
            vec![b'I'; DUPLEX_BYTES],
        );
        let mut runner = SystemRunner;
        let started = Instant::now();

        let error = runner
            .run(invocation)
            .expect_err("closed child stdin should fail the parent write");

        assert!(matches!(error, RunnerError::Write { .. }));
        assert!(started.elapsed() < Duration::from_secs(5));
    }

    #[test]
    #[ignore = "subprocess fixture invoked by SystemRunner tests"]
    fn system_runner_fixture_successful_capture() {
        let mut stdin = Vec::new();
        io::stdin()
            .read_to_end(&mut stdin)
            .expect("success stdin should be readable through EOF");
        assert_eq!(stdin, SUCCESS_STDIN);

        io::stdout()
            .write_all(SUCCESS_STDOUT_SENTINEL)
            .expect("success stdout sentinel should be writable");
        io::stdout()
            .flush()
            .expect("success stdout sentinel should flush");
        io::stderr()
            .write_all(SUCCESS_STDERR_SENTINEL)
            .expect("success stderr sentinel should be writable");
        io::stderr()
            .flush()
            .expect("success stderr sentinel should flush");
    }

    #[test]
    #[ignore = "subprocess fixture invoked by SystemRunner tests"]
    fn system_runner_fixture_nonzero_capture() {
        let mut stdin = Vec::new();
        io::stdin()
            .read_to_end(&mut stdin)
            .expect("nonzero stdin should be readable through EOF");
        assert_eq!(stdin, SUCCESS_STDIN);

        io::stdout()
            .write_all(NONZERO_STDOUT_SENTINEL)
            .expect("nonzero stdout sentinel should be writable");
        io::stdout()
            .flush()
            .expect("nonzero stdout sentinel should flush");
        io::stderr()
            .write_all(NONZERO_STDERR_SENTINEL)
            .expect("nonzero stderr sentinel should be writable");
        io::stderr()
            .flush()
            .expect("nonzero stderr sentinel should flush");
        std::process::exit(23);
    }

    #[test]
    #[ignore = "subprocess fixture invoked by SystemRunner tests"]
    fn system_runner_fixture_exits_without_reading_stdin() {
        std::process::exit(0);
    }

    #[test]
    #[ignore = "subprocess fixture invoked by SystemRunner tests"]
    fn system_runner_fixture_duplex_backpressure() {
        let completed = Arc::new(AtomicBool::new(false));
        let watchdog_completed = Arc::clone(&completed);
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(3));
            if !watchdog_completed.load(Ordering::SeqCst) {
                std::process::exit(91);
            }
        });

        let mut stdout = io::stdout().lock();
        stdout
            .write_all(&vec![b'O'; DUPLEX_BYTES])
            .expect("duplex stdout should be writable");
        stdout
            .write_all(DUPLEX_STDOUT_SENTINEL)
            .expect("duplex stdout sentinel should be writable");
        stdout.flush().expect("duplex stdout should flush");
        drop(stdout);

        let mut stderr = io::stderr().lock();
        stderr
            .write_all(&vec![b'E'; DUPLEX_BYTES])
            .expect("duplex stderr should be writable");
        stderr
            .write_all(DUPLEX_STDERR_SENTINEL)
            .expect("duplex stderr sentinel should be writable");
        stderr.flush().expect("duplex stderr should flush");
        drop(stderr);

        let mut stdin = Vec::new();
        io::stdin()
            .read_to_end(&mut stdin)
            .expect("duplex stdin should be readable");
        assert_eq!(stdin.len(), DUPLEX_BYTES);
        assert!(stdin.iter().all(|byte| *byte == b'I'));
        completed.store(true, Ordering::SeqCst);
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
        let mut progress = FakeProgress::default();
        let mut sink = FakeOutputSink::default();

        let exit_code = run_query(
            &config,
            "agents",
            &question,
            &mut runner,
            &mut progress,
            &mut sink,
        )
        .expect("query should run");

        assert_eq!(exit_code, 0);
        let invocation = runner.invocation.expect("runner should be invoked");
        assert_eq!(invocation.executable, OsString::from("custom-claude"));
        assert_eq!(invocation.cwd, wiki.path());
        assert_eq!(
            invocation.args,
            vec![
                OsString::from("-p"),
                OsString::from("--append-system-prompt"),
                OsString::from(
                    "This is a single-turn, read-only, non-interactive knowledge query. Answer completely and directly. Do not ask follow-up questions. Do not ask whether to save, record, index, or log the result. Do not offer or perform any wiki write-back. Do not create, edit, delete, index, or log files. Ignore workflow instructions that require recording or confirmation. End after the answer.",
                ),
                OsString::from("--add-dir"),
                wiki.path().as_os_str().to_owned(),
            ]
        );
        assert_eq!(
            invocation.stdin,
            b"/wiki-query How is deployment configured?\n"
        );
        assert!(
            !invocation
                .args
                .iter()
                .any(|argument| argument.to_string_lossy().contains("deployment"))
        );
        assert_eq!(progress.starts, ["agents"]);
        assert_eq!(progress.finish_count, 1);
        assert!(sink.replayed);
    }

    #[test]
    fn returns_provider_exit_code_unchanged() {
        let wiki = TestDir::new();
        let config = config(wiki.path(), "claude");
        let mut runner = FakeRunner {
            exit_code: 23,
            ..FakeRunner::default()
        };
        let mut progress = FakeProgress::default();
        let mut sink = FakeOutputSink::default();

        let exit_code = run_query(
            &config,
            "agents",
            &["What happened?".to_owned()],
            &mut runner,
            &mut progress,
            &mut sink,
        )
        .expect("query should run");

        assert_eq!(exit_code, 23);
    }

    #[test]
    fn rejects_unknown_wiki_without_invoking_provider() {
        let wiki = TestDir::new();
        let config = config(wiki.path(), "claude");
        let mut runner = FakeRunner::default();
        let mut progress = FakeProgress::default();
        let mut sink = FakeOutputSink::default();

        let error = run_query(
            &config,
            "missing",
            &["What happened?".to_owned()],
            &mut runner,
            &mut progress,
            &mut sink,
        )
        .expect_err("unknown wiki should fail");

        assert_eq!(error.to_string(), "unknown wiki 'missing'");
        assert!(runner.invocation.is_none());
        assert!(progress.starts.is_empty());
        assert_eq!(progress.finish_count, 0);
        assert!(!sink.replayed);
    }

    #[test]
    fn rejects_empty_question_without_invoking_provider() {
        let wiki = TestDir::new();
        let config = config(wiki.path(), "claude");
        let mut runner = FakeRunner::default();
        let mut progress = FakeProgress::default();
        let mut sink = FakeOutputSink::default();

        let error = run_query(
            &config,
            "agents",
            &[],
            &mut runner,
            &mut progress,
            &mut sink,
        )
        .expect_err("empty question should fail");

        assert_eq!(error.to_string(), "question must not be empty");
        assert!(runner.invocation.is_none());
        assert!(progress.starts.is_empty());
        assert_eq!(progress.finish_count, 0);
        assert!(!sink.replayed);
    }

    #[test]
    fn reports_missing_executable_as_launch_error() {
        let wiki = TestDir::new();
        let config = config(wiki.path(), "llm-wikis-test-executable-that-does-not-exist");
        let mut runner = SystemRunner;
        let mut progress = FakeProgress::default();
        let mut sink = FakeOutputSink::default();

        let error = run_query(
            &config,
            "agents",
            &["What happened?".to_owned()],
            &mut runner,
            &mut progress,
            &mut sink,
        )
        .expect_err("missing executable should fail");

        assert!(
            error
                .to_string()
                .contains("could not launch 'llm-wikis-test-executable-that-does-not-exist'")
        );
        assert_eq!(progress.starts, ["agents"]);
        assert_eq!(progress.finish_count, 1);
        assert!(!sink.replayed);
    }
}
