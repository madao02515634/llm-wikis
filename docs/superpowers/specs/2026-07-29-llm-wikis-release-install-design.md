# LLM Wikis Release and Installer Design

## Purpose

Ship the accepted query-only `llm-wikis` 0.1.0 MVP as native GitHub Release
assets, then install those assets without Python, `cargo install`, or a package
manager.

This design replaces only the release and installer portions of the older
full-product specification. It does not restore deferred query features,
provider hardening, `doctor`, Codex, MCP, orchestration, or package-manager
distribution.

## Supported platforms and assets

Version 0.1.0 publishes exactly:

| Release asset | Rust target | Supported host |
|---|---|---|
| `llm-wikis-windows-amd64.exe` | `x86_64-pc-windows-msvc` | Windows x64 |
| `llm-wikis-linux-amd64` | `x86_64-unknown-linux-musl` | Linux/WSL x64 |
| `llm-wikis-darwin-arm64` | `aarch64-apple-darwin` | macOS Apple Silicon |
| `SHA256SUMS` | n/a | Checksums for the three binaries |

`install.ps1` and `install.sh` remain source-controlled repository files served
from raw `main` URLs. They are not GitHub Release assets.

There is no Intel macOS, Linux ARM, signing, notarization, Authenticode,
Homebrew, WinGet, Scoop, uninstaller, or self-update support in 0.1.0. The
macOS and Windows binaries are unsigned previews.

Here, "signing" means platform code signing with Apple Developer ID or Windows
Authenticode. GitHub build-provenance attestation remains included as a
supply-chain statement; it does not make either binary platform-signed.

## Continuous integration

`.github/workflows/ci.yml` runs for pull requests, pushes to `main`, and manual
dispatch. It has read-only repository permissions and never invokes Claude.

The workflow:

1. runs `cargo fmt --check`, Clippy with warnings denied, and all Rust tests;
2. parses the PowerShell installer AST and checks POSIX shell syntax;
3. exercises side-effect-free installer plan output for every supported and
   rejected platform mapping;
4. performs static contract checks for the fixed Release origin, checksum
   requirement, install paths, and idempotent PATH/profile markers.

CI never starts a local HTTP server, downloads a binary, or executes a
downloaded binary.

GitHub-owned actions use reviewed immutable commit SHAs with their major
versions recorded in comments.

## Release workflow

`.github/workflows/release.yml` runs for pushes to `main` and tags matching
`v*`. A `main` run is a complete non-publishing dry run. A tag run repeats the
same build and assembly path, then publishes.
It defaults to `contents: read`. Only `publish` receives `contents: write`,
`id-token: write`, and `attestations: write`, which are required to create the
Release and its GitHub provenance attestations.

The workflow graph is:

```text
validate-and-test
       |
       +--> build-windows --+
       +--> build-linux ----+--> assemble --> publish (tag only)
       +--> build-macos ----+
```

On a tag, `validate-and-test` rejects a version whose leading `v`-stripped
value differs from the Cargo package version. It also runs the Rust format,
Clippy, and test gates so the tag is independently gated.

Each build runs on its native GitHub-hosted architecture, renames the raw
binary to its public asset name, and requires exact `llm-wikis 0.1.0`
`--version` output before upload. This is the only binary execution in the
distribution pipeline: it executes the binary directly from Cargo's build
output, before any artifact upload or download.

The `assemble` job:

1. downloads all three build artifacts;
2. rejects missing or unexpected assets;
3. creates `SHA256SUMS` for exactly the three binaries;
4. verifies the manifest before publication;
5. uploads one complete four-file Release bundle artifact.

It may read and hash downloaded artifacts, but it never executes them.

`publish` runs only when `github.ref` is a `v*` tag. It downloads the complete
bundle, verifies it again without executing any binary, creates GitHub
build-provenance attestations for the binaries and checksum manifest, and runs
one `gh release create "$GITHUB_REF_NAME" ... --verify-tag
   --generate-notes` command containing all four assets.

No draft or partial Release is created before all platform jobs succeed.
Existing Releases are not edited or overwritten.

## POSIX installer

`install.sh` follows the established `apm-go` flow with stricter platform and
manifest checks.

It:

- uses `set -eu`, `curl -fsSL`, `mktemp -d`, and a cleanup trap;
- uses the latest Release by default;
- accepts `LLM_WIKIS_VERSION=0.1.0` or `v0.1.0` for a pinned Release;
- accepts only Linux `x86_64`/`amd64` and Darwin `arm64`/`aarch64`;
- downloads the selected raw binary and `SHA256SUMS`;
- requires exactly one checksum entry for the selected asset;
- verifies with `sha256sum` or `shasum -a 256`, failing if neither exists;
- installs atomically where practical to `~/.local/bin/llm-wikis`;
- adds one managed PATH line to `~/.zprofile` on macOS or `~/.profile` on
  Linux only when `~/.local/bin` is absent;
- prints the installed path and tells the user to run `llm-wikis --version`
  manually.

Re-running repairs, upgrades, or downgrades the binary without duplicating the
profile entry.

`--print-plan` resolves the version, asset URL, destination, and profile path,
then exits before network or filesystem mutation. Test-only OS/architecture
inputs are accepted only in this side-effect-free mode.

## Windows installer

`install.ps1` uses strict mode and terminating errors.

It:

- supports Windows x64 only;
- uses the latest Release by default;
- accepts `LLM_WIKIS_VERSION=0.1.0` or `v0.1.0`;
- downloads `llm-wikis-windows-amd64.exe` and `SHA256SUMS`;
- requires exactly one matching checksum entry and verifies it with
  `Get-FileHash`;
- installs through a same-directory temporary file to
  `%LOCALAPPDATA%\llm-wikis\bin\llm-wikis.exe`;
- adds that exact directory to the user PATH once, preserving unrelated
  entries;
- prints the installed path and tells the user to run
  `llm-wikis --version` manually.

`-PrintPlan` emits version, asset URL, install directory, and user-PATH action,
then exits before network, file, registry, or process mutation.

## Test boundary

Installer verification is deliberately side-effect-free. It never starts a
fake server, contacts a Release origin, downloads an executable, writes an
installation, or executes a downloaded executable. It covers:

- latest and pinned version URL selection;
- supported and rejected OS/architecture pairs;
- fixed production Release origin;
- PowerShell AST and POSIX shell syntax;
- absence of automatic downloaded-binary execution;
- exact install destinations and managed PATH/profile markers;
- checksum parser behavior against inert text fixtures;
- Release bundle success, missing file, duplicate entry, malformed entry,
  mismatch, and unexpected file cases.

Release asset verification has both PowerShell and POSIX implementations and
rejects any manifest other than the exact three checksummed binaries.

There is no test Release-origin override. Production installation always uses
`https://github.com/madao02515634/llm-wikis/releases`; plan mode may vary only
OS/architecture inputs needed to prove asset selection.

## Documentation

The repository README documents:

- configuration and `query` usage;
- current supported platforms;
- pinned and latest installer commands;
- Windows install location and POSIX install location;
- unsigned macOS Gatekeeper and Windows SmartScreen expectations without
  disabling platform security;
- checksum and attestation verification;
- the features explicitly deferred from 0.1.0.

## Publication sequence

After implementation and local verification:

1. fast-forward `main` to the completed release branch;
2. create annotated tag `v0.1.0` on that exact commit;
3. push `main` so the Release workflow performs its full non-publishing dry
   run on the default branch;
4. wait for the `main` Release workflow to pass;
5. push `v0.1.0` to repeat the verified path and trigger publication;
6. wait for the tag workflow and inspect all four published assets without
   downloading or executing a binary.

A failed workflow or missing asset leaves publication incomplete and must not
be reported as a successful 0.1.0 release.
