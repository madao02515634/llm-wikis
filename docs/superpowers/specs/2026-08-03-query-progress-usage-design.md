# Query Progress and Usage Documentation Design

Date: 2026-08-03  
Status: Approved for implementation planning

## Goal

Make the query command self-explanatory and visibly active while Claude is
working. A user should be able to discover and run this command without reading
the source code:

```text
llm-wikis query --wiki agents -- "<prompt>"
```

## Scope

- Expand root and `query` help with concrete examples.
- Expand the README with prerequisites, installation verification, default
  config locations, configuration examples, querying, multiple wikis, and
  troubleshooting.
- Show an animated progress indicator while an interactive user waits for
  Claude.
- Bump the release version to `0.1.1` and publish it only after the feature PR
  is merged and the default-branch checks pass.

## Progress behavior

The progress indicator writes only to stderr and is enabled only when both
stdout and stderr are interactive terminals. Redirecting or piping either
stream, as well as running in CI, therefore disables the spinner and emits no
spinner frames or terminal control sequences.

The spinner begins after the configuration and selected wiki have passed
validation, immediately before the Claude process is launched. Its message
identifies the selected wiki without echoing the user's prompt:

```text
Querying wiki 'agents'...
```

Claude stdout and stderr are captured while the spinner is active. When Claude
finishes, fails, or cannot be launched, the spinner is cleared before captured
provider output or the wrapper error is printed. This prevents the answer and
spinner from overwriting each other. The provider's exit code remains the CLI's
exit code.

The initial implementation uses `indicatif` for cross-platform terminal
detection and rendering. It does not add progress configuration flags: non-TTY
auto-detection is the only policy in this release.

## Components and data flow

1. Clap parses and validates the requested command.
2. The configuration loader resolves an existing absolute wiki directory and
   its configured entrypoint.
3. The query service constructs the same stdin prompt and fixed Claude argv as
   version `0.1.0`.
4. The system runner creates a terminal-aware progress indicator, launches
   Claude, writes stdin, and waits for its output.
5. Guaranteed cleanup clears the progress indicator before replaying provider
   stdout and stderr.
6. The wrapper returns the unchanged provider status, or its existing wrapper
   error status when launch, stdin, or wait handling fails.

Progress rendering is isolated from query construction so tests can verify
whether progress was enabled and whether cleanup occurred without invoking a
real Claude query.

## CLI and documentation

Both root help and query help include at least one copyable command using the
required `--` separator. The README documents:

- Claude Code installation and authentication as prerequisites;
- supported release platforms and installer commands;
- default config paths on Windows, Linux, and Apple Silicon macOS;
- `config_version`, `claude_executable`, wiki path, and configurable
  `entrypoint` fields;
- one-wiki and multiple-wiki examples;
- the default-config form and the explicit form exactly as
  `llm-wikis --config <path> query --wiki <id> -- <question...>`;
- common errors for a missing Claude executable, config, wiki, directory, or
  entrypoint;
- interactive-only progress behavior.

Examples use placeholders or public paths and must not include the repository's
local `agents` contents or other private machine data.

## Error handling

- Config and wiki validation failures occur before progress starts.
- Launch, stdin-write, wait, and output-replay failures stop and clear progress.
- Provider stdout stays on stdout; provider stderr and progress stay on stderr.
- The prompt is never displayed by the progress indicator or diagnostics.
- Non-interactive output remains free of ANSI control bytes.

## Verification

Implementation follows red-green-refactor. Automated tests cover:

- root and query help containing a copyable query example;
- progress enabled only when stdout and stderr are both interactive, and
  disabled when either stream is non-TTY;
- progress cleanup on success, nonzero provider exit, and runner error;
- no spinner bytes in redirected output;
- unchanged prompt, Claude argv, and provider exit-code propagation;
- the complete existing Rust, installer, and release-verifier suites.

The release gate runs formatting, Clippy with warnings denied, all tests, and
both installer and release-asset verifiers. GitHub Actions must build all three
platform binaries successfully.

## Delivery workflow

All work is isolated on `feat/query-progress`, created from the synchronized
default branch. The branch contains the spec, plan, tests, implementation,
documentation, version bump, and release-workflow version updates. After local
verification it is committed, pushed, and opened as a draft PR. The PR becomes
ready only after review findings are resolved and CI passes, then is squash
merged.

After the merged `main` succeeds, tag `v0.1.1` is created from the merge commit.
The resulting Release must contain exactly:

- `llm-wikis-windows-amd64.exe`
- `llm-wikis-linux-amd64`
- `llm-wikis-darwin-arm64`
- `SHA256SUMS`

`install.ps1` and `install.sh` remain raw repository files and are never Release
assets.

## Non-goals

- MCP, Codex, orchestration, multi-wiki fan-out, or streaming provider output.
- User-selectable spinner styles, colors, messages, or a `--no-spinner` flag.
- Signing, notarization, new platforms, package managers, or installer redesign.
- Uploading any local `agents` or `.agents` knowledge-base content.
