# llm-wikis

`llm-wikis` queries local wiki sources using the configured provider settings.

```sh
llm-wikis --config <path> query --wiki agents -- "What is context engineering?"
```

Copy `config.example.toml` to your preferred configuration location and supply
the provider settings before querying. Run `llm-wikis --help` for the full CLI.

## Install a release

The supported release platforms are Windows x64, Linux x64, and Apple Silicon
(Darwin ARM64). The installers only download, checksum, and place the release
file; they intentionally never run a downloaded program. After installing, run
`llm-wikis --version` yourself.

Latest release:

```powershell
irm https://raw.githubusercontent.com/madao02515634/llm-wikis/refs/heads/main/install.ps1 | iex
```

```sh
curl -fsSL https://raw.githubusercontent.com/madao02515634/llm-wikis/refs/heads/main/install.sh | sh
```

To pin version `0.1.0`:

```powershell
$env:LLM_WIKIS_VERSION = '0.1.0'; irm https://raw.githubusercontent.com/madao02515634/llm-wikis/refs/heads/main/install.ps1 | iex
```

```sh
curl -fsSL https://raw.githubusercontent.com/madao02515634/llm-wikis/refs/heads/main/install.sh | LLM_WIKIS_VERSION=0.1.0 sh
```

The installer scripts are source files served from the repository and are not
GitHub Release assets.

Windows installs to `%LOCALAPPDATA%\llm-wikis\bin\llm-wikis.exe`; Linux installs
to `~/.local/bin/llm-wikis` and adds one managed line to `~/.profile`; macOS
installs to `~/.local/bin/llm-wikis` and adds it to `~/.zprofile`.

For a manual install, download exactly the asset for your platform and
`SHA256SUMS` from the Release. Verify the asset's SHA-256 entry before copying
the file to your chosen executable directory. Releases also include GitHub
artifact attestations; verify them with GitHub CLI before trusting a download.

Windows SmartScreen and macOS Gatekeeper can warn because these binaries are
unsigned. Review the checksum and attestation rather than disabling platform
security controls.

## 0.1.0 scope

Version 0.1.0 provides the query MVP and the three native release targets above.
It does not provide Linux ARM, Intel macOS, signed/notarized platform binaries,
automatic post-install execution, or an installer option to change the release
origin. It also excludes provider hardening, a doctor command, Codex, MCP,
orchestration, package-manager distribution (including Homebrew, WinGet, and
Scoop), an uninstaller, and self-update.
