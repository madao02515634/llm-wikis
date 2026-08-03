# Query Progress and Usage Documentation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `llm-wikis query` discoverable and visibly active in interactive terminals, then ship the change as `v0.1.1` through a reviewed pull request.

**Architecture:** Keep query construction independent from presentation. A terminal-aware progress component owns spinner lifecycle, the provider runner captures Claude output, and the query service always clears progress before an output sink replays stdout and stderr. Progress is enabled only when both stdout and stderr are terminals.

**Tech Stack:** Rust 2024, Clap 4.5, indicatif 0.18.6, standard `IsTerminal`, Cargo tests, GitHub Actions, GitHub CLI.

---

## File map

- Create `src/progress.rs`: terminal policy and idempotent spinner lifecycle.
- Modify `src/lib.rs`: export the progress module.
- Modify `src/query.rs`: capture provider output, clear progress, and replay output in a testable order.
- Modify `src/cli.rs`: concrete help examples and progress/output dependencies.
- Modify `src/main.rs`: construct production runner, progress, and output sink.
- Modify `Cargo.toml` and `Cargo.lock`: add indicatif and bump to `0.1.1`.
- Modify `README.md`: complete user guide and `0.1.1` scope.
- Modify `.github/workflows/release.yml`: expect `llm-wikis 0.1.1` from every platform build.
- Preserve `install.ps1`, `install.sh`, release asset names, and the four-file release contract unchanged.

### Task 0: Commit the reviewed implementation plan

**Files:**
- Create: `docs/superpowers/plans/2026-08-03-query-progress-usage.md`

- [ ] **Step 1: Confirm the planning baseline**

Verify the current branch is `feat/query-progress`, the approved spec commit is
present, and the only worktree change is this plan file.

- [ ] **Step 2: Commit the approved plan**

Stage only this plan, run the commit-message analyzer, and commit:

```text
docs(plan): 新增查詢進度與使用說明實作計畫
```

- [ ] **Step 3: Confirm a clean execution baseline**

Run `git status --porcelain` and require empty output before Task 1.

### Task 1: Add copyable CLI help examples

**Files:**
- Modify: `src/cli.rs`

- [ ] **Step 1: Write failing root-help test**

Add `clap::CommandFactory` to the test module and add:

```rust
#[test]
fn root_help_contains_copyable_query_example() {
    let help = Cli::command().render_long_help().to_string();
    assert!(help.contains(
        "llm-wikis query --wiki agents -- \"What is context engineering?\""
    ));
}
```

- [ ] **Step 2: Verify the root-help test fails for the missing example**

Run:

```text
cargo test cli::tests::root_help_contains_copyable_query_example -- --exact
```

Expected: FAIL because the rendered root help does not contain the command.

- [ ] **Step 3: Write and verify the query-help test**

Add a second test that obtains the `query` subcommand through
`Cli::command().find_subcommand_mut("query")`, renders long help, and expects
the same copyable command. Run it exactly and confirm the same missing-example
failure before changing the Clap declarations.

- [ ] **Step 4: Add minimal Clap help text**

Add static `after_help` text to the root command and `query` subcommand. Keep
the existing grammar and required `--` separator unchanged:

```text
Examples:
  llm-wikis query --wiki agents -- "What is context engineering?"
  llm-wikis --config ./config.toml query --wiki agents -- "How is deployment configured?"
```

- [ ] **Step 5: Verify both help tests and inspect rendered help**

Run:

```text
cargo test cli::tests::root_help_contains_copyable_query_example -- --exact
cargo test cli::tests::query_help_contains_copyable_query_example -- --exact
cargo run --quiet -- --help
cargo run --quiet -- query --help
```

Expected: both tests PASS and both help screens contain copyable commands.

- [ ] **Step 6: Commit the help change**

Stage only `src/cli.rs`, run the commit-message analyzer, and commit:

```text
feat(cli): 新增查詢指令使用範例
```

### Task 2: Add terminal-aware progress rendering

**Files:**
- Create: `src/progress.rs`
- Modify: `src/lib.rs`
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`

- [ ] **Step 1: Write the failing policy and lifecycle tests**

Create `src/progress.rs` with tests that reference the not-yet-implemented
`should_render_progress` function:

```rust
#[test]
fn progress_requires_both_streams_to_be_terminals() {
    assert!(should_render_progress(true, true));
    assert!(!should_render_progress(true, false));
    assert!(!should_render_progress(false, true));
    assert!(!should_render_progress(false, false));
}
```

In the same RED step, add tests for the wished-for
`TerminalQueryProgress` API: disabled progress stays inactive, enabled progress
becomes active after `start`, and two consecutive `finish` calls leave it
inactive. Add a draw-target injection used only inside the module tests, backed
by `indicatif::InMemoryTerm`, and assert that each disabled stream combination
produces empty terminal contents and no recorded terminal moves after
`start`/`finish`. This proves the real indicatif wiring emits no frames or
control operations when stdout or stderr is non-TTY.

Export the module from `src/lib.rs` only far enough for Cargo to compile the
test target.

- [ ] **Step 2: Verify RED**

Run:

```text
cargo test progress::tests::progress_requires_both_streams_to_be_terminals -- --exact
```

Expected: compile failure for the missing policy, progress type, and lifecycle
methods.

- [ ] **Step 3: Add indicatif and implement the policy**

Add the production dependency plus the test-only in-memory renderer feature:

```toml
indicatif = "0.18.6"

[dev-dependencies]
indicatif = { version = "0.18.6", features = ["in_memory"] }
```

Implement the pure policy as `stdout_is_terminal && stderr_is_terminal` and
define:

```rust
pub trait QueryProgress {
    fn start(&mut self, wiki: &str);
    fn finish(&mut self);
}

pub struct TerminalQueryProgress {
    enabled: bool,
    bar: Option<indicatif::ProgressBar>,
}
```

`TerminalQueryProgress::from_stdio()` reads both stream states through
`std::io::IsTerminal`. `start` creates an stderr spinner with the message
`Querying wiki '<id>'...`, a 100 ms steady tick, and no prompt text. `finish`
takes the stored bar and calls `finish_and_clear`, making repeated cleanup safe.
`Drop` calls `finish` as the final cleanup guard. A private constructor accepts
the test draw target; no test-only API is exported.

- [ ] **Step 4: Verify all policy, lifecycle, and zero-output tests are GREEN**

Run the individual tests first. Confirm all four terminal-state combinations,
idempotent cleanup, and empty disabled `InMemoryTerm` output pass.

- [ ] **Step 5: Verify GREEN**

Run:

```text
cargo test progress::tests -- --nocapture
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

Expected: progress tests PASS; formatting and Clippy exit 0.

- [ ] **Step 6: Commit the progress component**

Stage only the four files listed for this task, run the commit-message analyzer,
and commit:

```text
feat(progress): 新增互動式查詢載入動畫
```

### Task 3: Clear progress before replaying Claude output

**Files:**
- Modify: `src/query.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write failing lifecycle-order tests**

In `src/query.rs`, add recording fakes backed by a shared event vector. Add
separate tests for:

```text
success:      progress:start -> runner -> progress:finish -> output
nonzero:      progress:start -> runner -> progress:finish -> output
runner error: progress:start -> runner -> progress:finish
```

The success and nonzero cases must also assert exact stdout/stderr bytes and
unchanged provider exit status. The error case must assert no output replay.

Also add RED tests for the wished-for output replay helper using injected
`Write` implementations: stdout and stderr bytes go only to their matching
writers, and a writer that returns an error produces the corresponding bounded
stdout/stderr replay error. These tests exercise the same helper used by the
production `StdioOutputSink`.

- [ ] **Step 2: Verify RED**

Run each new test with its full `query::tests::<name>` and `--exact` filter.

Expected: compile failure because `ProviderOutput`, `ProviderOutputSink`, and the
progress-aware query signature do not exist.

- [ ] **Step 3: Introduce captured provider output**

Replace inherited output markers with:

```rust
pub struct ProviderOutput {
    pub exit_code: i32,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

pub trait ProviderOutputSink {
    fn replay(&mut self, output: &ProviderOutput) -> Result<(), RunnerError>;
}
```

Change `ProviderRunner::run` to return `ProviderOutput`. `SystemRunner` must pipe
both streams and use `wait_with_output`; it must not print provider output while
the spinner is active.

- [ ] **Step 4: Implement guaranteed ordering**

Extend `run_query` with injected `QueryProgress` and `ProviderOutputSink`
arguments. The required order is:

```rust
progress.start(wiki_id);
let provider_result = runner.run(invocation);
progress.finish();
let output = provider_result?;
sink.replay(&output)?;
Ok(output.exit_code)
```

Add `StdioOutputSink`, writing provider stdout only to stdout and provider stderr
only to stderr through the tested generic replay helper. Add bounded error
variants for stdout/stderr write and flush failures.
Update the existing invocation, unknown-wiki, empty-question, missing-executable,
and exit-code tests to use empty captured output and recording fakes.

- [ ] **Step 5: Wire production dependencies**

Update `execute` to pass progress and output sink into `run_query`. In `main`,
construct `SystemRunner`, `TerminalQueryProgress::from_stdio()`, and
`StdioOutputSink`. Config and unknown-wiki errors must still occur before
`progress.start`.

- [ ] **Step 6: Verify GREEN and regression behavior**

Run:

```text
cargo test query::tests -- --nocapture
cargo test cli::tests -- --nocapture
cargo test --all-targets --all-features
```

Expected: all tests PASS. Existing tests still prove fixed Claude argv, prompt on
stdin, unknown-wiki rejection, and exit-code propagation.

- [ ] **Step 7: Commit the query lifecycle change**

Stage the three files listed for this task, run the commit-message analyzer, and
commit:

```text
feat(query): 清除載入動畫後輸出查詢結果
```

### Task 4: Expand the user guide

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Add the complete quick-start flow**

Document, in order:

1. Claude Code installation, authentication, and `claude --version`.
2. Windows x64, Linux x64, and Apple Silicon macOS installation.
3. `llm-wikis --version` installation verification.
4. Default config paths for all three supported operating systems.
5. A placeholder-only TOML example that explicitly explains
   `config_version`, `claude_executable`, wiki `path`, and configurable
   `entrypoint`.
6. Single-wiki and multiple-wiki examples.
7. Default and explicit-config query commands.
8. Interactive-only progress behavior.
9. Common config, path, wiki, entrypoint, and Claude launch errors.

Keep `install.ps1` and `install.sh` documented as raw-main source files, never
as Release assets. Do not include local knowledge-base contents or machine paths.

- [ ] **Step 2: Run documentation contract checks**

Run:

```text
rg -n "llm-wikis query --wiki agents --" README.md
rg -n "llm-wikis --config .* query --wiki" README.md
rg -n "config_version|claude_executable|entrypoint" README.md
rg -n "raw.githubusercontent.com/.*/install\.(ps1|sh)" README.md
rg -n "releases/.*/install\.(ps1|sh)" README.md
```

Expected: query forms, all required config fields, and both raw installer URLs
are present; the final search returns no matches.

- [ ] **Step 3: Commit the documentation change**

Stage only `README.md`, run the commit-message analyzer, and commit:

```text
docs(readme): 補齊安裝設定與查詢說明
```

### Task 5: Prepare version 0.1.1

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock`
- Modify: `.github/workflows/release.yml`
- Modify: `README.md`

- [ ] **Step 1: Write the failing version-contract check**

Before editing, run:

```text
cargo run --quiet -- --version
```

Expected: `llm-wikis 0.1.0`, demonstrating the new release version is absent.

- [ ] **Step 2: Bump package and workflow expectations**

Set the Cargo package version to `0.1.1`, refresh `Cargo.lock`, replace all three
platform workflow expectations with `llm-wikis 0.1.1`, and rename the README
scope section to `0.1.1 scope`.

- [ ] **Step 3: Verify the version contract**

Run:

```text
cargo run --quiet -- --version
rg -n "llm-wikis 0\.1\.0" .github/workflows/release.yml Cargo.toml Cargo.lock
rg -n "llm-wikis 0\.1\.1" .github/workflows/release.yml
```

Expected: version output is `llm-wikis 0.1.1`; no old runtime expectation
remains; exactly three platform checks expect `0.1.1`.

- [ ] **Step 4: Re-run release contract self-tests**

Run:

```text
pwsh -NoProfile -File tests/release/verify-assets.ps1 -SelfTest
sh tests/release/verify-assets.sh --self-test
```

Expected: both pass and the expected asset set remains three binaries plus
`SHA256SUMS`.

- [ ] **Step 5: Commit the release preparation**

Stage only the four files listed for this task, run the commit-message analyzer,
and commit:

```text
build(release): 準備 0.1.1 發布版本
```

### Task 6: Run the complete quality gate and review

**Files:**
- Modify only files required to resolve verified findings.

- [ ] **Step 1: Run the complete local gate**

Run every command fresh:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
pwsh -NoProfile -File tests/installers/verify-install-ps1.ps1
sh tests/installers/verify-install-sh.sh
pwsh -NoProfile -File tests/release/verify-assets.ps1 -SelfTest
sh tests/release/verify-assets.sh --self-test
```

Expected: every command exits 0 with no failed Rust tests.

- [ ] **Step 2: Verify scope and private-content exclusions**

Run:

```text
git diff --check origin/main...HEAD
git status --short
git ls-files | rg "^(agents|\.agents)/"
git diff --name-only origin/main...HEAD
```

Expected: no whitespace errors, a clean worktree, no tracked knowledge-base
paths, and only files named in this plan/spec.

- [ ] **Step 3: Perform independent code review**

Use the code-review-and-quality skill against `origin/main...HEAD`. Review
correctness, readability, architecture, security, and performance, with special
attention to process output buffering, cleanup on every error path, TTY policy,
prompt secrecy, and Release asset scope.

- [ ] **Step 4: Resolve findings through the bounded loop**

For each confirmed blocking finding: add or adjust a failing regression test,
verify RED, implement the smallest fix, verify GREEN, rerun the affected gate,
and commit atomically. Re-review once. If the same finding fails again, stop and
revise the plan instead of expanding scope.

### Task 7: Push, open, validate, and merge the PR

**Files:**
- No repository file changes unless CI or review identifies a confirmed issue.

- [ ] **Step 1: Run PR pre-flight**

Confirm `git status --porcelain` is empty, derive the default branch with
`gh repo view`, inspect commit/file counts and `origin/<base>...HEAD`, and verify
`gh auth status`.

- [ ] **Step 2: Push the feature branch**

Run:

```text
git push -u origin HEAD
```

Expected: `origin/feat/query-progress` is created and tracks the local branch.

- [ ] **Step 3: Create a draft PR**

Use the pull-request skill and a UTF-8-no-BOM body file. Use a Conventional
Commits title such as:

```text
feat(cli): 新增查詢進度與完整使用說明
```

The body summarizes user behavior, TTY rules, tests, version bump, and confirms
that no local wiki content is included. Remove temporary PR files afterward.

- [ ] **Step 4: Wait for and inspect all PR checks**

Use `gh pr checks --watch` with waits shorter than 60 seconds per poll. Do not
mark ready while Ubuntu, Windows, or macOS checks are pending or failing.

- [ ] **Step 5: Final review and merge**

After all checks pass and final review has no blockers, mark the PR ready and
squash merge it with branch deletion. Confirm the PR state is `MERGED` and its
merge commit belongs to `main`.

### Task 8: Publish and verify v0.1.1

**Files:**
- No repository file changes.

- [ ] **Step 1: Verify merged main before tagging**

Switch to `main`, fetch/pull with fast-forward only, confirm the merged commit,
clean status, and successful CI plus the non-tag Release workflow run on main.

- [ ] **Step 2: Create and push the release tag**

Create annotated tag `v0.1.1` at the verified `main` commit and push only that
tag. The tag must match `Cargo.toml` exactly.

- [ ] **Step 3: Wait for the tag Release workflow**

Poll GitHub Actions without local fake servers, downloading release binaries,
or executing downloaded binaries. Require every build, assembly, attestation,
and publish job to succeed.

- [ ] **Step 4: Verify published metadata and asset contract**

Query the GitHub Release API and assert the asset names are exactly:

```text
llm-wikis-windows-amd64.exe
llm-wikis-linux-amd64
llm-wikis-darwin-arm64
SHA256SUMS
```

Assert `install.ps1` and `install.sh` are absent, the Release is final, the tag
and `main` point at the merge commit, and the public tree contains no tracked
`agents/**`, `.agents/**`, or excluded private skill artifacts.

- [ ] **Step 5: Report evidence**

Report the PR URL, merge SHA, successful CI and Release run URLs, Release URL,
exact asset list, and any residual GitHub server-side history limitation without
claiming it has been purged.
