# LLM Wikis Query MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development to implement this plan. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a Rust binary that reads a wiki registry and queries one selected
wiki through Claude Code.

**Architecture:** A small `clap` boundary loads strict TOML, resolves one wiki,
builds a shell-free Claude invocation, and delegates execution through a tiny
runner interface. Tests replace the runner and assert cwd, argv, stdin, and
exit propagation without launching Claude.

**Tech Stack:** Rust 1.97.1, edition 2024, `clap`, `serde`, `toml`, `thiserror`,
and the standard process API.

---

## Task 1: Build the query-only MVP

**Files:**

- Create: `Cargo.toml`
- Create: `Cargo.lock`
- Create: `rust-toolchain.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`
- Create: `src/cli.rs`
- Create: `src/config.rs`
- Create: `src/query.rs`
- Create: `config.example.toml`

- [ ] **Step 1: Add failing configuration tests**

Test strict parsing, `config_version = 1`, absolute existing wiki paths,
non-empty entrypoints, wiki lookup, explicit `--config`, and Windows/macOS/Linux
default path functions using injected environment values.

Run:

```powershell
$env:PATH='C:\Users\User\.cargo\bin;'+$env:PATH
cargo test config
```

Expected: FAIL because the crate and configuration module do not exist.

- [ ] **Step 2: Implement the crate and configuration loader**

Use package `llm-wikis` version `0.1.0`, Rust `1.97`, edition `2024`.
Dependencies:

```toml
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
thiserror = "2"
toml = "0.9"
```

Configuration types deny unknown fields. `claude_executable` defaults to
`claude`; each wiki stores `path: PathBuf` and `entrypoint: String`.

- [ ] **Step 3: Add failing invocation tests**

Using an in-memory fake `ProviderRunner`, assert:

- selected wiki path is both cwd and the `--add-dir` value;
- argv is exactly `["-p", "--add-dir", <wiki-path>]`;
- stdin is exactly `<entrypoint> <question>\n`;
- question bytes never appear in argv;
- stdout/stderr are inherited by the real runner;
- provider exit code is returned unchanged;
- missing wiki, empty question, invalid config, and missing executable produce
  concise non-zero errors.

Run:

```powershell
cargo test query
```

Expected: FAIL because query dispatch is not implemented.

- [ ] **Step 4: Implement the query command**

Public syntax:

```text
llm-wikis [--config <path>] query --wiki <id> -- <question...>
```

`SystemRunner` uses `std::process::Command`, never a shell. It sets cwd, passes
fixed argv, pipes stdin, inherits stdout/stderr, writes the prompt, closes
stdin, waits, and returns the child exit code.

- [ ] **Step 5: Add the example config and user-facing errors**

`config.example.toml` shows one `agents` wiki and `/wiki-query` entrypoint using
a replace-me absolute path. Errors identify invalid config, unknown wiki,
missing question, missing executable, and child launch/write/wait failure.

- [ ] **Step 6: Run the complete quality gate**

```powershell
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo run -- --help
```

Expected: all commands exit `0`; help exposes only `query`.

- [ ] **Step 7: Commit**

```powershell
git add Cargo.toml Cargo.lock rust-toolchain.toml src config.example.toml
git commit -m "feat: add query-only llm-wikis MVP"
```

- [ ] **Step 8: Run one manual acceptance query**

Create an untracked temporary config pointing at the user's agents wiki and
run one real query. Inspect the answer to confirm it addresses the question
with useful wiki-backed information. Record only the command exit and the
useful/not-useful verdict; do not retain the answer or prompt in the repository.
