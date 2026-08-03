# llm-wikis

`llm-wikis` v0.1.x asks Claude Code a question about one configured local wiki
at a time. Each wiki entry selects an existing local directory and the Claude
Code skill or plugin command that knows how to query it.

## Prerequisite

Install Claude Code by following the official
[Claude Code quickstart](https://code.claude.com/docs/en/quickstart). Confirm
that the executable is available in the same terminal where you will run
queries:

```sh
claude --version
```

Next, launch `claude` and complete the first-run login; from an existing Claude
Code session, use `/login`. Then verify the authenticated state:

```sh
claude auth status
```

## Install a release

The supported release platforms are Windows x64, Linux x64, and Apple Silicon
macOS (Darwin ARM64). The installer scripts below are source files served from
the repository's raw `main` branch; they are not GitHub Release assets. The
scripts download the appropriate binary and `SHA256SUMS` from GitHub Releases,
verify the checksum, and install the binary without running it.
Review an installer script at its raw URL before piping it to a shell.

### Windows x64

Install the latest release from PowerShell:

```powershell
irm https://raw.githubusercontent.com/madao02515634/llm-wikis/refs/heads/main/install.ps1 | iex
```

Pin the currently valid `0.1.3` release:

```powershell
$env:LLM_WIKIS_VERSION = '0.1.3'; irm https://raw.githubusercontent.com/madao02515634/llm-wikis/refs/heads/main/install.ps1 | iex
```

The binary is installed at
`%LOCALAPPDATA%\llm-wikis\bin\llm-wikis.exe`. The installer updates your user
`PATH`, but the current PowerShell session does not automatically receive that
change. Verify immediately with the full path:

```powershell
& "$env:LOCALAPPDATA\llm-wikis\bin\llm-wikis.exe" --version
```

Open a new terminal before using `llm-wikis --version` by command name.

### Linux x64

Install the latest release:

```sh
curl -fsSL https://raw.githubusercontent.com/madao02515634/llm-wikis/refs/heads/main/install.sh | sh
```

Pin the currently valid `0.1.3` release:

```sh
curl -fsSL https://raw.githubusercontent.com/madao02515634/llm-wikis/refs/heads/main/install.sh | LLM_WIKIS_VERSION=0.1.3 sh
```

The binary is installed at `~/.local/bin/llm-wikis`. Reload the profile that the
installer manages, then verify the installation:

```sh
. ~/.profile
llm-wikis --version
```

### Apple Silicon macOS

Install the latest release:

```sh
curl -fsSL https://raw.githubusercontent.com/madao02515634/llm-wikis/refs/heads/main/install.sh | sh
```

Pin the currently valid `0.1.3` release:

```sh
curl -fsSL https://raw.githubusercontent.com/madao02515634/llm-wikis/refs/heads/main/install.sh | LLM_WIKIS_VERSION=0.1.3 sh
```

The binary is installed at `~/.local/bin/llm-wikis`. Reload the zsh profile that
the installer manages, then verify the installation:

```sh
. ~/.zprofile
llm-wikis --version
```

### Manual verification and platform warnings

For a manual install, download exactly the binary for your platform and
`SHA256SUMS` from the GitHub Release. Confirm that the manifest contains exactly
one entry for the selected asset and verify the binary's SHA-256 value before
copying it to an executable directory. Releases also include GitHub artifact
attestations; verify the downloaded binary with GitHub CLI before trusting it.

Windows SmartScreen and macOS Gatekeeper can warn because these binaries are
unsigned. Review the checksum and attestation rather than disabling platform
security controls.

## Configure wikis

Without `--config`, `llm-wikis` reads the platform default:

- Windows: `%APPDATA%\llm-wikis\config.toml`
- Linux: `$XDG_CONFIG_HOME/llm-wikis/config.toml`, or
  `~/.config/llm-wikis/config.toml` when `XDG_CONFIG_HOME` is not set
- macOS: `~/Library/Application Support/llm-wikis/config.toml`

You can keep the file elsewhere and select it with `--config <path>`.

### Single-wiki configuration

Create a TOML file like the following, replacing every placeholder with a value
that exists on your machine:

```toml
# Required configuration schema version. Version 0.1.x accepts only 1.
config_version = 1

# Optional; defaults to "claude". Use a command on PATH or a full executable path.
claude_executable = "claude"

# "agents" is the wiki ID used by --wiki. Add one [wikis.<id>] table per wiki.
[wikis.agents]
# Must be an absolute path to an existing directory.
path = "/absolute/path/to/your/wiki"
# Claude Code command available for this wiki; this name is configurable.
entrypoint = "/your-query-command"
```

On Windows, a placeholder can be replaced with a forward-slash path such as
`C:/absolute/path/to/your/wiki`. A namespaced Claude Code plugin command is also
valid when that plugin is installed and available, for example
`entrypoint = "/your-plugin:your-query-command"`.

Query the configured wiki:

```sh
llm-wikis query --wiki agents -- "What is context engineering?"
```

### Multiple-wiki configuration

Each wiki has an independent ID, path, and entrypoint:

```toml
config_version = 1
claude_executable = "claude"

[wikis.agents]
path = "/absolute/path/to/your/first-wiki"
entrypoint = "/your-first-query-command"

[wikis.docs]
path = "/absolute/path/to/your/second-wiki"
entrypoint = "/your-plugin:your-second-query-command"
```

Choose exactly one wiki for each query. The generic explicit-config syntax is
`llm-wikis --config <path> query --wiki <id> -- "<prompt>"`. For example:

```sh
llm-wikis --config ./config.toml query --wiki agents -- "What is context engineering?"
```

The `--` separator is required: it ends `llm-wikis` option parsing and starts the
question. One invocation accepts one `--wiki` ID and one prompt. `llm-wikis`
sends the configured entrypoint and prompt to Claude through stdin; it does not
display that generated input.

Queries launched through `llm-wikis query` are single-turn, read-only, and
non-interactive. Claude is instructed to answer directly and finish without
asking whether to save, record, index, or log the result, and without offering
or performing persistence to wiki files. The configured entrypoint remains
arbitrary, including namespaced plugin commands, while every query fixes Claude
Code's tool surface to `Read`, `Glob`, `Grep`, and `Skill`. Claude Code is also
run in `dontAsk` permission mode and is explicitly denied `Bash`, `Edit`,
`Write`, `NotebookEdit`, `Agent`, `AskUserQuestion`, and all `mcp__*` tools.
The wrapper loads only Claude Code's `project` setting source, so user and local
settings—and plugins or hooks supplied by those sources—do not enter the
non-interactive query.

This is application-level Claude Code tool-dispatch enforcement, not an OS
sandbox or isolation of project hooks. Project settings and their hooks may
still run, and the Claude Code process and those hooks retain their normal
operating-system permissions. This wrapper contract does not apply when you use
Claude directly in an interactive session.

After Claude finishes, its answer is written to stdout and its diagnostics are
written to stderr. A successful query exits with code `0`; otherwise
`llm-wikis` preserves Claude Code's exit code. Argument, configuration, or
launch failures also return a non-zero code and report the error on stderr. This
separation makes answers safe to pipe or redirect independently of diagnostics.

## Query progress

The `Querying wiki '<id>'...` spinner appears only when **both** stdout and
stderr are interactive terminals. It is disabled when either stream is piped or
redirected, including typical CI execution. Progress status is written only to
stderr, the answer remains on stdout, and the prompt is never shown. The spinner
is cleared before the captured Claude output is emitted.

## Troubleshooting

- **Claude executable is missing:** run `claude --version`. If you use a custom
  installation, set `claude_executable` to its command name or full executable
  path, then start a terminal with the correct `PATH`.
- **Claude authentication fails:** launch Claude Code directly, complete its
  authentication flow, and retry from the same user account and environment.
- **The config cannot be read or parsed:** confirm the selected/default path,
  file permissions, valid TOML syntax, `config_version = 1`, and at least one
  `[wikis.<id>]` table. Use `--config <path>` if the file is elsewhere.
- **A wiki path is rejected:** `path` must be absolute, must already exist, and
  must name a directory. Relative paths and missing directories are rejected.
- **A wiki is unknown:** the value passed to `--wiki` must exactly match the ID
  after `[wikis.` in the config, including case.
- **The entrypoint is missing or wrong:** verify that `entrypoint` is non-empty
  and that the exact skill or namespaced plugin command is available to Claude
  Code when its working directory is the configured wiki path.
- **Claude reports a rate, quota, or provider limit:** its diagnostic and exit
  code are forwarded unchanged. Check the provider account and limit, wait if
  necessary, then retry; changing the wiki configuration does not bypass a
  provider limit.

Run `llm-wikis --help` for the complete command-line reference.

## 0.1.3 scope

Version 0.1.3 provides the query MVP, the fixed Claude Code tool-dispatch policy,
and the three native release targets above. It does not provide an OS sandbox,
hook isolation, Linux ARM, Intel macOS, signed/notarized platform binaries,
automatic post-install execution, or an installer option to change the release
origin. It also excludes additional provider hardening, a doctor command,
Codex, MCP, orchestration, package-manager distribution (including Homebrew,
WinGet, and Scoop), an uninstaller, and self-update.
