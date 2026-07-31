# LLM Wikis Query MVP Design

## Goal

Deliver a usable Rust CLI that lets another project query one selected wiki
per invocation through Claude Code. This MVP validates user value before
investing in process containment, doctor probes, installers, CI, release
automation, or MCP.

## Public interface

```text
llm-wikis [--config <path>] query --wiki <id> -- <question...>
```

Example:

```text
llm-wikis query --wiki agents -- How is deployment configured?
```

The CLI prints Claude's answer to stdout, forwards Claude stderr, and returns
Claude's exit code. Configuration and argument errors return a non-zero exit
with a concise message.

## Configuration

```toml
config_version = 1
claude_executable = "claude"

[wikis.agents]
path = "E:/knowledge/agents"
entrypoint = "/wiki-query"
```

- `config_version` must equal `1`.
- `claude_executable` is optional and defaults to `claude`.
- Each wiki has an absolute existing directory and a non-empty entrypoint.
- Unknown fields, duplicate IDs, missing wiki IDs, and relative wiki paths fail.
- `--config` overrides the platform default:
  - Windows: `%APPDATA%\llm-wikis\config.toml`
  - macOS: `~/Library/Application Support/llm-wikis/config.toml`
  - Linux: `$XDG_CONFIG_HOME/llm-wikis/config.toml`, otherwise
    `~/.config/llm-wikis/config.toml`

## Query flow

1. Parse the command and load strict TOML.
2. Resolve the requested wiki.
3. Start the configured Claude executable directly, without a shell.
4. Use the wiki path as Claude's working directory and pass it with
   `--add-dir`.
5. Invoke Claude in print mode with `-p`.
6. Send `<entrypoint> <question>` to Claude only through stdin, then close it.
7. Inherit stdout and stderr so the user sees Claude's normal print-mode result
   and diagnostics.
8. Wait for Claude and return its exit code.

## MVP boundaries

Included:

- one `query` command;
- multiple configured wiki IDs;
- configurable skill/plugin entrypoint per wiki;
- Claude Code only;
- Windows, macOS, and Linux path resolution;
- direct native Rust binary with no Python dependency.

Deferred until users validate the query experience:

- `config init`, `list`, and `doctor`;
- JSON normalization and citation validation;
- timeout, bounded pipes, process-tree containment, mutation snapshots;
- installers, CI, GitHub Releases, signing, MCP, and orchestration.

## Acceptance

The MVP is acceptable when:

1. `cargo test` passes;
2. `llm-wikis --config <fixture> query --wiki <id> -- <question>` selects the
   configured wiki and constructs the expected Claude invocation;
3. one manual query against the user's agents wiki returns a useful answer;
4. the question is written to child stdin and never placed in Claude argv.

The older full version 0.1.0 specification and plan remain future-quality
backlog; they do not gate this MVP branch.
