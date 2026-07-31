# LLM Wikis Release and Installer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Publish the query-only Rust MVP as four verified GitHub Release assets
without local fake servers or executing any downloaded binary.

**Architecture:** Installers download, checksum, and install but never launch
the downloaded executable. Their local contract tests use only syntax,
side-effect-free plan output, static inspection, and inert checksum fixtures.
The Release workflow executes binaries only in native build jobs before upload;
all later jobs may hash and publish downloaded artifacts but never execute them.

**Tech Stack:** Rust/Cargo, PowerShell 7, POSIX shell, GitHub Actions, GitHub
CLI, and GitHub artifact attestations.

---

## Task 1: Build the safe release distribution

**Files:**

- Create: `install.ps1`
- Create: `install.sh`
- Create: `tests/installers/verify-install-ps1.ps1`
- Create: `tests/installers/verify-install-sh.sh`
- Create: `tests/release/verify-assets.ps1`
- Create: `tests/release/verify-assets.sh`
- Create: `.github/workflows/ci.yml`
- Create: `.github/workflows/release.yml`
- Create: `README.md`
- Modify: `.gitattributes`

- [ ] **Step 1: Write failing side-effect-free installer tests**

`verify-install-ps1.ps1` must:

- parse `install.ps1` with PowerShell's parser and reject syntax errors;
- call only `install.ps1 -PrintPlan`;
- check latest and pinned (`0.1.0` and `v0.1.0`) URLs;
- check the Windows x64 asset and install path;
- reject unsupported Windows architectures in plan mode;
- statically reject fake/test origins, `Start-Process`, and invocation of the
  downloaded path;
- assert the source contains strict mode, checksum verification, fixed
  repository, and one managed user-PATH marker.

`verify-install-sh.sh` must:

- run `sh -n install.sh`;
- call only `install.sh --print-plan`;
- use plan-only OS/architecture inputs to check Linux x64 and Darwin ARM64;
- reject Intel macOS, Linux ARM, and unsupported OS mappings;
- check latest and pinned URLs, install path, and OS-specific profile path;
- statically reject fake/test origins, `http.server`, and execution of the
  downloaded path;
- assert `set -eu`, checksum verification, fixed repository, and one managed
  profile marker.

Neither test may start a server, access the network, write an installation,
write PATH/profile state, create an executable fixture, or execute a binary.

Run:

```powershell
pwsh -NoProfile -File tests/installers/verify-install-ps1.ps1
sh tests/installers/verify-install-sh.sh
```

Expected: both fail because the installers do not exist.

- [ ] **Step 2: Implement the POSIX installer**

`install.sh` must:

- use `set -eu`;
- hard-code `madao02515634/llm-wikis`;
- use latest by default and normalize `LLM_WIKIS_VERSION`;
- map only Linux x64 and Darwin ARM64;
- download the selected raw binary and `SHA256SUMS` with `curl -fsSL`;
- require exactly one manifest entry for the asset;
- fail closed through `sha256sum` or `shasum -a 256`;
- install through a same-directory temporary path and `mv`;
- idempotently add one managed `~/.local/bin` line to `~/.zprofile` on macOS
  or `~/.profile` on Linux;
- never execute the downloaded binary;
- print the installed path and ask the user to run
  `llm-wikis --version` manually.

`--print-plan` exits before `curl`, `mktemp`, or filesystem mutation. Only in
this mode may `LLM_WIKIS_PLAN_OS` and `LLM_WIKIS_PLAN_ARCH` override `uname`;
there is no Release-origin override.

Run the POSIX test until it passes.

- [ ] **Step 3: Implement the Windows installer**

`install.ps1` must:

- enable strict mode and terminating errors;
- hard-code `madao02515634/llm-wikis`;
- use latest by default and normalize `LLM_WIKIS_VERSION`;
- support Windows x64 only;
- download the raw `.exe` and `SHA256SUMS` through `Invoke-WebRequest`;
- require exactly one manifest entry and verify with `Get-FileHash`;
- replace through a same-directory temporary path;
- install to `%LOCALAPPDATA%\llm-wikis\bin\llm-wikis.exe`;
- update the real user PATH once while preserving unrelated entries;
- never execute the downloaded binary;
- print the installed path and ask for a manual `llm-wikis --version`.

`-PrintPlan` exits before network, temp directory, registry, PATH, filesystem,
or child-process mutation. `-PlanArchitecture` is accepted only with
`-PrintPlan`. There is no test origin or user-PATH override.

Run the PowerShell test until it passes.

- [ ] **Step 4: Add release bundle verifiers**

Both verifiers accept a staging directory or a self-test flag. Production
validation requires exactly:

```text
llm-wikis-windows-amd64.exe
llm-wikis-linux-amd64
llm-wikis-darwin-arm64
SHA256SUMS
```

`SHA256SUMS` contains exactly three unique, valid, matching entries for the
three binaries. Self-tests use inert text files only and cover success,
missing/unexpected files, duplicate/malformed entries, and checksum mismatch.
They never execute any fixture.

Run:

```powershell
pwsh -NoProfile -File tests/release/verify-assets.ps1 -SelfTest
sh tests/release/verify-assets.sh --self-test
```

Expected: both pass.

- [ ] **Step 5: Commit installers and safe tests**

Add LF rules for shell scripts and workflow YAML to `.gitattributes`, run all
four script suites, then commit:

```powershell
git add -- install.ps1 install.sh tests .gitattributes
git commit -m "feat: add checksum-only release installers"
```

- [ ] **Step 6: Add CI workflow**

`.github/workflows/ci.yml` runs for pull requests, pushes to `main`, and manual
dispatch with `contents: read`.

It runs:

- Ubuntu: Rust format, Clippy with warnings denied, all Rust tests, POSIX
  installer checks, and both release verifier self-tests;
- Windows 2025: PowerShell installer checks;
- macOS 15 ARM64: POSIX Darwin plan checks.

It must contain no provider invocation, fake server, network installer test,
downloaded executable invocation, or write permission.

- [ ] **Step 7: Add main-dry-run/tag-publish Release workflow**

`.github/workflows/release.yml` triggers on pushes to `main` and `v*` tags.

Use immutable revisions:

```text
actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4
actions/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02 # v4
actions/download-artifact@d3f86a106a0bac45b974a628896c90dbdf5c8093 # v4
actions/attest-build-provenance@96b4a1ef7235a096b17240c259729fdd70c83d45 # v2
```

Jobs:

1. `validate-and-test`: run format, Clippy, all Rust tests, installer checks,
   and verifier self-tests; on tag refs require tag version equals Cargo.
2. `build-windows`: Windows 2025, build
   `x86_64-pc-windows-msvc`, execute Cargo's direct output with `--version`,
   require exact `llm-wikis 0.1.0`, rename, then upload.
3. `build-linux`: Ubuntu 24.04 with `musl-tools`, build
   `x86_64-unknown-linux-musl`, execute direct output before upload, rename,
   then upload.
4. `build-macos`: ARM64 macOS 15, build `aarch64-apple-darwin`, execute direct
   output before upload, rename, then upload.
5. `assemble`: download the three artifacts, never execute them, generate exact
   checksums, run both bundle verifiers, then upload one four-file
   `release-bundle`.
6. `publish`: run only for a `refs/tags/v*` ref; download and re-verify the
   bundle without execution; reject an existing Release; attest the three
   binaries and `SHA256SUMS`; create one Release containing all four assets.

Only `publish` receives:

```yaml
permissions:
  contents: write
  id-token: write
  attestations: write
```

A `main` run must reach and pass `assemble` while skipping `publish`.

- [ ] **Step 8: Add README**

Document config/query usage, latest and pinned raw-repository installer
commands, supported platforms, install paths, checksum-only installer behavior, manual
post-install `--version`, unsigned Gatekeeper/SmartScreen expectations,
attestation verification, and 0.1.0 exclusions.

Do not advise disabling platform security.

- [ ] **Step 9: Run the complete local gate**

```powershell
$env:PATH='C:\Users\User\.cargo\bin;' + $env:PATH
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
pwsh -NoProfile -File tests/installers/verify-install-ps1.ps1
sh tests/installers/verify-install-sh.sh
pwsh -NoProfile -File tests/release/verify-assets.ps1 -SelfTest
sh tests/release/verify-assets.sh --self-test
rg -n -i "http\\.server|Start-Process|LLM_WIKIS_TEST_ORIGIN" install.ps1 install.sh tests .github/workflows
```

Expected: build/test/script commands exit `0`; the final search returns no
matches. No local command accesses a Release URL or executes an installer
download.

- [ ] **Step 10: Commit workflows and documentation**

```powershell
git add -- .github/workflows/ci.yml .github/workflows/release.yml README.md
git commit -m "ci: validate and publish native releases"
```

- [ ] **Step 11: Controller verification and review**

The controller reruns Step 9, confirms scope against `feature/mvp-query`, and
performs spec-compliance and code-quality reviews. Fix only release blockers,
then run one final whole-change review.

- [ ] **Step 12: Merge, tag, push, and monitor**

After all local gates and reviews pass:

```powershell
git -C E:\not_company\llm-wikis merge --ff-only feature/release-automation
git -C E:\not_company\llm-wikis tag -a v0.1.0 -m "llm-wikis v0.1.0"
git -C E:\not_company\llm-wikis push -u origin main
```

Wait for the `main` release dry-run to pass through `assemble`. If it fails,
do not push the tag; fix the branch, re-verify, fast-forward `main`, and recreate
the still-local tag on the corrected commit.

Only after the dry-run succeeds:

```powershell
git -C E:\not_company\llm-wikis push origin v0.1.0
```

Wait for the tag workflow and confirm through GitHub metadata that the Release
contains exactly four assets. Do not download or execute a released binary
during automated verification.
