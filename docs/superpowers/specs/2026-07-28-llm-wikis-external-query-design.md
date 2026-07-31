# LLM Wikis External Query Design

Date: 2026-07-28  
Version: 0.1.0  
Status: user-approved design; independently reviewed by Opus and Fable; implementation not started  
Project root: `<root>`

## 1. Purpose

Provide a distributable, read-only `llm-wikis` command-line interface for querying one registered knowledge base from any development project without opening an interactive Claude or Codex session and manually copying the result.

The CLI is implemented as one Rust binary and installed directly from GitHub Releases without a Python runtime. The MVP externalizes only `query`; wiki initialization, ingestion, update, merge, lint, audit, answer filing, and cross-wiki orchestration remain outside this implementation.

## 2. Current State and Problem

The current Agents knowledge base is split across two roots:

```text
<root>/agents/                    # project_root
├── .claude/skills/
├── .agents/skills/
└── wiki/                         # content_root
    ├── SCHEMA.md
    ├── config/
    ├── bin/
    └── wiki/
        ├── index.md
        └── pages/
```

Interactive Claude and Codex sessions can use the installed `wiki-query` skill. External projects cannot query the knowledge base through a stable machine interface, so users must switch sessions and copy answers manually.

The wrapper cannot hard-code `/wiki-query`. A future knowledge base may expose the same query contract through:

- a differently named project skill;
- a Claude plugin skill such as `/knowledge-tools:ask-wiki`;
- a Codex skill mention such as `$ask-wiki`;
- a provider-specific installation mechanism.

Provider behavior also differs:

- Claude uses `-p` for non-interactive execution, can load skills from an additional directory, and supports one schema-validated JSON result.
- Codex uses `exec`, discovers repository skills relative to its working directory, and emits JSONL events even when its final result follows an output schema.
- Claude `--add-dir` grants additional file access; Codex `--add-dir` grants an additional writable root. They are not interchangeable.

## 3. Scope

### 3.1 In Scope

- an `llm-wikis query` command that accepts exactly one registered wiki;
- a trusted TOML registry for wikis and reusable query profiles;
- Claude Code and Codex CLI provider adapters;
- project skills with configurable names;
- a constrained Claude local-plugin entrypoint;
- a fixed `wiki-query/v1` input and output contract;
- read-only index freshness checking;
- structured JSON success and failure envelopes;
- citation extraction, validation, and wiki namespacing;
- static and optional live `llm-wikis doctor` checks;
- a non-interactive, non-overwriting `llm-wikis config init`;
- one native Rust executable with no Python runtime dependency;
- GitHub Release binaries and checksum-verifying install scripts;
- Windows x64, Linux/WSL x64, and macOS Apple Silicon portability;
- an internal Query Service reusable by a future MCP adapter.

### 3.2 Explicitly Out of Scope

- multi-wiki fan-out, question decomposition, or synthesis;
- an `llm-wikis orchestration` implementation;
- MCP server implementation;
- query answer saving or operation logging;
- index regeneration during query;
- externalizing any other wiki skill;
- direct retrieval APIs such as `search_wiki` or `read_wiki_page`;
- arbitrary wiki paths supplied by query callers;
- arbitrary prompt templates, system prompts, shell commands, or raw provider CLI arguments in configuration;
- automatic discovery of all installed skills or plugins;
- HTTP service, remote execution, or multi-user tenancy;
- changing the current wiki page or citation format;
- Intel macOS and Linux ARM binaries;
- package-manager distribution through Cargo, Homebrew, WinGet, Scoop, apt, or similar channels;
- Codex installed-plugin loading, because it conflicts with the version 0.1.0 `--ignore-user-config` hardening posture until an explicit safe plugin-load mechanism is verified;
- an interactive configuration wizard;
- a built-in self-update command;
- Apple Developer ID signing, Apple notarization, or Windows Authenticode signing in version 0.1.0.

A future `llm-wikis orchestration` command may call the same Query Service once per wiki and synthesize normalized results. It receives a separate design and implementation plan.

## 4. Design Decisions

1. `query` handles exactly one wiki ID per invocation.
2. The public command is provider-neutral; provider-specific behavior stays behind adapters.
3. Configuration records explicit provider entrypoints instead of deriving names.
4. Every entrypoint must implement `wiki-query/v1`; a configured contract is a claim that static and live checks verify, not a security boundary.
5. Read-only enforcement belongs to the harness: tool restriction, sandboxing, no session persistence, trusted roots, and mutation checks.
6. `project_root` and `content_root` are separate required fields.
7. The wrapper owns citation validation and adds the wiki namespace. The model never supplies an authoritative public namespace.
8. External query never regenerates the index. Interactive mutation workflows own index maintenance.
9. The wrapper warns or fails when the index appears stale, according to policy.
10. CLI and a future MCP adapter call the same Query Service. MCP must not reimplement retrieval and must not shell out through the public CLI.
11. Questions go to child processes through stdin. They are never interpolated into a shell command.
12. Configured project roots, content, skills, and plugins are trusted operator inputs. User questions are untrusted.
13. The product is one synchronous Rust CLI. Concurrent pipe readers and a platform-specific process supervisor provide bounded output, timeout, and process-tree termination without an async application runtime.
14. Provider executable locations may be overridden only by a trusted global configuration value containing one command name or one absolute path; raw arguments remain implementation-owned.
15. Version 0.1.0 release assets are raw platform executables plus `SHA256SUMS`, not ZIP or tar archives.
16. macOS version 0.1.0 supports Apple Silicon only and is explicitly published as an unsigned preview.

## 5. Public CLI

### 5.1 Commands

```text
llm-wikis --version
llm-wikis [--config <absolute-path>] [--json] config init
llm-wikis [--config <absolute-path>] [--json] list
llm-wikis [--config <absolute-path>] [--json] doctor [--wiki <id>] [--agent claude|codex] [--live]
llm-wikis [--config <absolute-path>] [--json] query --wiki <id> [--agent claude|codex] -- <question>
```

`llm-wikis --version` prints `llm-wikis 0.1.0`.

`llm-wikis config init` creates the parent directory and a valid, non-interactive starter configuration only when the destination does not exist. It never overwrites or merges an existing file; an existing destination is `CONFIG_EXISTS` and exit `2`. The generated file contains runtime and provider defaults, one reusable `wiki-query/v1` profile, an empty wiki registry, and a commented wiki example. A future wizard or `config add-wiki` command is outside version 0.1.0.

In JSON mode, config initialization emits exactly `{ "schema_version": "1.0", "ok": true, "operation": "config_init", "path": "<absolute config path>", "created": true }` on success. Failure uses the same operation with `created: false` and the public `error` object.

`llm-wikis query` accepts one and only one `--wiki`. Repeating it or supplying `all` is an argument error.

If the positional question is omitted, the command reads the complete question from stdin. When both are supplied, the command fails rather than merging ambiguous inputs.

`--agent` is optional only when `default_agent` exists and the selected wiki enables it.

Normal human output prints the answer followed by gaps and warnings. `--json` emits exactly one JSON document on stdout. Diagnostics go to stderr.

`llm-wikis list` loads the registry and lists every configured wiki without starting a provider. `llm-wikis doctor` defaults to static checks for every configured wiki and every provider enabled by each wiki. `--wiki` and `--agent` narrow that matrix. Because live checks consume model quota, `llm-wikis doctor --live` requires both selectors.

`list` derives each wiki's `default_agent` from the global `default_agent`. The field is the configured value only when that provider is enabled for the wiki; otherwise it is JSON `null`. Omitting `--agent` from `query` is valid only when this derived value is non-null.

### 5.2 Configuration Selection

The installed CLI has one platform-native default configuration path:

```text
Windows:   %APPDATA%\llm-wikis\config.toml
Linux/WSL: ${XDG_CONFIG_HOME:-~/.config}/llm-wikis/config.toml
macOS:     ~/Library/Application Support/llm-wikis/config.toml
```

An optional `--config <absolute-path>` override is an operator/testing feature and establishes a new trust boundary. Relative override paths are rejected. The override is not accepted through MCP or other untrusted callers.

Configuration discovery never walks the caller's current project.

### 5.3 List and Doctor Machine Output

`llm-wikis --json list` returns:

```json
{
  "schema_version": "1.0",
  "ok": true,
  "operation": "list",
  "wikis": [
    {
      "id": "agents",
      "title": "Agents Knowledge Base",
      "default_agent": "claude",
      "agents": ["claude", "codex"]
    }
  ]
}
```

`llm-wikis --json doctor` returns one result per selected wiki/provider pair:

```json
{
  "schema_version": "1.0",
  "ok": true,
  "operation": "doctor",
  "live": false,
  "results": [
    {
      "wiki": "agents",
      "agent": "claude",
      "ok": true,
      "checks": [
        {
          "name": "entrypoint",
          "status": "pass",
          "code": null,
          "message": "Configured project skill is statically addressable."
        }
      ]
    }
  ]
}
```

Check `status` is `pass`, `warn`, or `fail`. Overall `ok` is false when any check fails. A command-level argument or configuration failure adds the same top-level `error` object used by query and emits an empty `wikis` or `results` array. Pair-specific doctor failures remain in `results[].checks`.

List uses exit `0` after a valid config and exit `2` for config failure. Doctor performs all selected static checks before any live check and uses the global error-to-exit mapping in Section 14. For example, invalid config is `2`, missing provider CLI is `3`, live child timeout is `5`, invalid native output is `6`, and a detected mutation is `7`. When a matrix has failures from more than one class, the process selects the first present class from this precedence order: `70`, `7`, `6`, `5`, `4`, `3`, `2`, then `0`.

## 6. Configuration Contract

The file is TOML so it supports comments and a strict Rust deserializer can reject unknown or mistyped keys. The registry written by `config init` is valid with zero configured wikis; `list` returns an empty array, while `query` reports `WIKI_NOT_ALLOWED` until the operator adds a wiki.

Example configured registry:

```toml
config_version = 1
default_agent = "claude"

[providers.claude]
executable = "claude"

[providers.codex]
executable = "codex"

[runtime]
timeout_seconds = 180
max_question_bytes = 65536
max_stdout_bytes = 1048576
max_stderr_bytes = 65536
index_freshness = "warn" # warn | error

[query_profiles.wiki_skill_v1]
contract = "wiki-query/v1"

[query_profiles.wiki_skill_v1.claude]
load = "project_skill"
entrypoint = "/wiki-query"
skill_path = ".claude/skills/wiki-query/SKILL.md"

[query_profiles.wiki_skill_v1.codex]
load = "project_skill"
entrypoint = "$wiki-query"
skill_path = ".agents/skills/wiki-query/SKILL.md"

[wikis.agents]
title = "Agents Knowledge Base"
project_root = "E:/not_company/llm-wikis/agents"
content_root = "E:/not_company/llm-wikis/agents/wiki"
query_profile = "wiki_skill_v1"
agents = ["claude", "codex"]
```

The paths above illustrate one configured Windows machine; the generated commented example says to replace them with operator-owned absolute paths. Relative wiki paths remain supported and resolve from the configuration file, but an example installed under `%APPDATA%`, XDG config, or `~/Library/Application Support` must not imply that `../agents` refers to the CLI repository.

`max_question_bytes`, `max_stdout_bytes`, and `max_stderr_bytes` are positive byte counts. The complete UTF-8 question is limited before provider startup; overflow is `QUESTION_TOO_LARGE`, and invalid stdin UTF-8 is `QUESTION_INVALID_UTF8`. Provider stream caps are enforced independently while the child is running. Crossing either stream cap terminates the process tree and returns `OUTPUT_TOO_LARGE` with the offending stream named in error details.

Required/default rules:

- `config_version` is required and must equal `1`;
- `default_agent` is optional;
- `[runtime]` is optional; its fields independently default to the values in the example;
- `[providers]`, `[query_profiles]`, and `[wikis]` default to empty tables;
- each provider subtable is optional until a wiki enables that provider;
- `providers.<agent>.executable` defaults to the matching command name when its provider table exists;
- a wiki requires `title`, `project_root`, `content_root`, `query_profile`, and a non-empty unique `agents` array;
- an enabled agent missing its provider table is `PROVIDER_CONFIG_MISSING`;
- an enabled agent missing from the referenced query profile is `PROVIDER_PROFILE_MISSING`;
- each provider/profile/load-mode table has its own closed set of required fields as shown in Section 6.2.

### 6.1 Path Rules

- `project_root` and `content_root` resolve relative to the configuration file.
- `content_root` must be a real directory contained by `project_root`.
- `skill_path` for `project_skill` resolves relative to `project_root`.
- `plugin_dir` for `local_plugin` resolves relative to the configuration file.
- A local plugin's `skill_path` resolves relative to `plugin_dir`.
- Every path is resolved canonically.
- While resolving a configured root/path, every component on that path is checked for symlink, junction, reparse, mount, or other special status. Full recursive special-entry scans apply only to the complete `content_root` and the selected project-skill/Claude-local-plugin artifact tree; unrelated portions of `project_root` are not recursively scanned. Any encountered special entry is `UNSAFE_FILESYSTEM_ENTRY`, even when its target would remain contained. Canonical regular paths outside their declared root use `PATH_OUTSIDE_ALLOWED_ROOT`.
- Wiki IDs match `[a-z0-9]+(?:-[a-z0-9]+)*` and are unique.
- Query callers select an allowlisted ID; they cannot supply a path.
- A provider `executable` is either one command name resolved through the current process `PATH`/`PATHEXT`, or one absolute path.
- Provider executable values containing arguments, shell syntax, control characters, or relative path components are rejected.
- `doctor` records and reports the canonical executable selected after resolution.

### 6.2 Supported Provider Load Modes

Claude:

```toml
[query_profiles.example.claude]
load = "project_skill"
entrypoint = "/ask-wiki"
skill_path = ".claude/skills/ask-wiki/SKILL.md"
```

or:

```toml
[query_profiles.example.claude]
load = "local_plugin"
entrypoint = "/knowledge-tools:ask-wiki"
plugin_dir = "../plugins/knowledge-tools"
skill_path = "skills/ask-wiki/SKILL.md"
```

Codex:

```toml
[query_profiles.example.codex]
load = "project_skill"
entrypoint = "$ask-wiki"
skill_path = ".agents/skills/ask-wiki/SKILL.md"
```

Every load mode is disabled for normal query until `doctor --live` succeeds for the exact current machine, provider executable/version, resolved roots, profile, entrypoint, configuration identity, and project-skill/local-plugin fingerprint. This one-time-per-fingerprint gate prevents an APM refresh, Claude local-plugin update, or local skill edit from silently changing the claimed `wiki-query/v1` behavior. Live doctor is never implicit.

### 6.3 Entrypoint Validation

- Claude project skills match `/name`.
- Claude plugin skills match `/plugin-name:skill-name`.
- Codex project skills match one unnamespaced `$name` in version 0.1.0; `$plugin-name:skill-name` is reserved for the deferred installed-plugin design.
- Names use ASCII letters, digits, dots, underscores, and hyphens.
- A Claude plugin entrypoint contains exactly one colon separating two valid names.
- Entrypoints are single tokens. Whitespace, newlines, control characters, quotes, and shell metacharacters are rejected.
- The wrapper does not append provider namespaces, rename the entrypoint, or infer it from `skill_path`.

### 6.4 Forbidden Configuration

The schema rejects unknown keys by default, including:

```text
claude_args
codex_args
shell_command
system_prompt
prompt_template
allowed_tools
sandbox
mcp_config
```

Provider safety flags and the query prompt envelope are implementation-owned.

### 6.5 Current Agents Skill Ownership

The upstream `kfchou/wiki-skills` dependency remains the source of the interactive skill baseline, but version 0.1.0 does not modify or claim ownership of that third-party source. This repository owns an explicit overlay:

```text
agents/overrides/wiki-query/SKILL.md
```

Deterministic deployment scripts copy that overlay byte-for-byte to:

```text
agents/.claude/skills/wiki-query/SKILL.md
agents/.agents/skills/wiki-query/SKILL.md
```

The overlay preserves the upstream interactive workflow and adds the required early `external-readonly` branch. Repository tests require both deployed copies to equal the overlay. Any `apm install` or `apm update` may replace the deployed copies; operator documentation therefore requires rerunning the overlay deployment scripts afterward. Even if that step is missed, the changed skill fingerprint invalidates the required live probe and normal query fails with `ENTRYPOINT_UNVERIFIED` rather than executing the reverted write-capable workflow.

## 7. `wiki-query/v1` Contract

### 7.1 Invocation Envelope

The wrapper builds the complete prompt. The configured entrypoint is the first token, followed by a fixed envelope:

```text
<entrypoint>

EXTERNAL_QUERY:
{
  "contract": "wiki-query/v1",
  "mode": "external-readonly",
  "wiki_id": "agents",
  "content_root": "<canonical absolute path>",
  "question": "<untrusted question>"
}
```

The JSON is serialized by the wrapper. The question is data, not a prompt template fragment.

### 7.2 Required Skill Behavior

In `external-readonly` mode, a compatible skill must:

1. use the explicit `content_root`;
2. read `SCHEMA.md`, link-style configuration, the complete index, relevant pages, and one relevant level of cross-references;
3. treat wiki content as the source of truth and not fill gaps from general knowledge;
4. return grounded answers, inline native wiki citations, disagreements, gaps, and follow-up questions;
5. omit answer-saving offers;
6. never regenerate the index;
7. never write pages, logs, reports, commits, caches, or other files;
8. return the normalized result object required below.

Interactive invocation retains the current behavior. The implementation must add an explicit mode branch to the existing skill instead of relying on a later prompt to contradict mutation instructions.

### 7.3 Model Result Object

Both providers must produce:

```json
{
  "contract": "wiki-query/v1",
  "knowledge_status": "grounded",
  "answer": "Grounded answer with [[wiki-slug]] citations.",
  "citations": ["wiki-slug"],
  "gaps": ["Optional gap"],
  "warnings": []
}
```

Rules:

- `contract` must equal `wiki-query/v1`;
- `knowledge_status` is exactly `grounded` or `no_relevant_material`;
- `answer` is a non-empty string;
- `citations`, `gaps`, and `warnings` are arrays of strings;
- `grounded` requires at least one citation that the wrapper can resolve;
- `no_relevant_material` requires an empty citation array and at least one non-empty `gaps` item;
- provider-native metadata does not replace this object.

## 8. Query Service

The internal Query Service has five bounded components:

1. **Config Loader** — loads and validates the trusted TOML.
2. **Wiki Resolver** — resolves one wiki ID, roots, profile, and provider.
3. **Preflight** — checks wiki structure, entrypoint availability, index freshness, and executable/auth readiness.
4. **Provider Adapter** — runs Claude or Codex and parses its native output.
5. **Result Normalizer** — validates the contract, citations, public envelope, and errors.

The CLI and future MCP server call this service directly.

### 8.1 Query Flow

1. Parse CLI input without starting an agent.
2. Load the fixed or explicitly trusted config.
3. Resolve exactly one wiki ID and enabled provider.
4. Validate the complete UTF-8 question and `max_question_bytes`.
5. Canonicalize all configured paths and verify containment.
6. Require:
   - `SCHEMA.md`;
   - a mechanically parsed supported `link_style` value in `SCHEMA.md`, or the `obsidian` default;
   - the contained `link_style_rules` relative path when `SCHEMA.md` names one;
   - `wiki/index.md`;
   - `wiki/pages/`;
   - the configured project skill or local plugin artifacts when statically addressable.
7. Check index freshness without writing.
8. Resolve the provider executable; run its bounded version and non-billable authentication-status probes.
9. Fingerprint the selected skill/local-plugin artifact and require the one current live-probe record for the logical key/current-verification tuple.
10. Build the fixed prompt envelope.
11. Snapshot the complete canonical `content_root` with content hashes.
12. Invoke the provider with stdin/stdout/stderr separated.
13. Enforce timeout, independent stdout/stderr size limits, and process-tree termination.
14. Parse provider-native output.
15. Validate `wiki-query/v1`.
16. Extract and validate page-provenance citations against actual pages.
17. Add the wiki namespace.
18. Recompute and compare the complete content-root snapshot.
19. Emit exactly one public result envelope.

## 9. Index Freshness

External query never runs `bin/generate-index.py`.

Preflight compares `wiki/index.md` with Markdown pages under `wiki/pages/`:

- missing index is `WIKI_INVALID`;
- when any page modification time is later than the index, the index is stale;
- parse page entries only from generated-list lines matching exact built-in shapes `- [[<slug>]] — ...` or `- [[<slug>](pages/<slug>.md)] — ...`; incidental links on other lines are ignored;
- the compared page set and parsed index-entry set both remove slugs whose filenames match the implementation-owned current-schema exclusion `audit-*.md`;
- the remaining index-entry set must equal the exact filename-stem set of regular `wiki/pages/*.md` files;
- a missing, duplicate, extra, or case-mismatched index entry is stale even when timestamps would not detect it;
- `index_freshness = "warn"` continues with `INDEX_MAY_BE_STALE`;
- `index_freshness = "error"` stops with `INDEX_STALE`.

This is a conservative mechanical check, not proof that the generated index content is correct. Interactive ingest, update, merge, and other mutation workflows remain responsible for regenerating it.

Tests cover equal timestamps and filesystems with coarse timestamp resolution.

## 10. Process Supervision and Provider Adapters

### 10.1 Common Process Supervisor

The Query Service passes a structured argument vector to a synchronous process supervisor. It never renders that vector into a shell command string. The supervisor:

- launches the provider in its own process group or platform-equivalent containment;
- sends the complete prompt through stdin and then closes stdin;
- drains stdout and stderr concurrently so neither pipe can deadlock the child;
- enforces the two byte limits while streaming, not after buffering an unbounded result;
- enforces the monotonic deadline across spawn, write, read, wait, and termination;
- kills the entire process group on timeout, overflow, cancellation, or parser-aborting failure;
- waits for termination and joins both pipe readers before returning;
- preserves stdout, stderr, exit status, elapsed time, and termination reason as separate bounded fields.

On Unix, the supervisor uses a dedicated process group. On Windows, it uses a Job Object or an equivalently tested process-tree primitive. Direct `.exe` providers are started without a shell. When PATH resolution selects a `.cmd` or `.bat` provider shim, only the trusted, fixed provider argv passes through a dedicated Windows batch adapter; the untrusted question remains on stdin. Phase 0 must prove paths with spaces and representative Windows metacharacters do not change the command shape.

Provider flag spellings may change after Phase 0, but the safety invariant may not: the corrected vector exposes only the minimum skill-invocation capability plus non-mutating read/search tools, disables or excludes Write, Edit, shell execution, web access, MCP, subagents, and session persistence, and preserves non-interactive no-escalation behavior. A substitute permission mode that re-enables a forbidden capability is not an acceptable compatibility fix. The capability spike records the minimal tool set that both invokes the configured entrypoint and satisfies this invariant.

Generated empty MCP configuration, the Codex JSON-schema file, and any batch-adapter helper live in a fresh system temporary directory that is canonically outside every configured `project_root` and `content_root`. Files use exclusive creation and user-only permissions where supported, remain owned/open by the wrapper until child startup, and are removed after the child is reaped. Claude's `--json-schema` value is an implementation-generated inline JSON string, not a path. An unsafe temporary location fails before provider startup.

Authentication readiness uses provider status commands, not a paid model request: Claude runs `claude auth status --json`; Codex runs `codex login status`. Phase 0 records sanitized authenticated output and exact exit behavior without logging the operator out. For these status commands only, any nonzero exit or explicit logged-out state is `AUTH_REQUIRED`; exit-zero malformed/unrecognized output is `INVALID_NATIVE_OUTPUT`; timeout and output overflow retain `TIMEOUT`/`OUTPUT_TOO_LARGE`. The later paid provider invocation still maps an ordinary nonzero exit to `NONZERO_EXIT`.

### 10.2 Claude Adapter

The child process working directory is the selected `project_root`, not the caller's development project and not an unverified empty directory.

Target invocation:

```text
claude
  --add-dir <content_root>
  -p
  --input-format text
  --no-session-persistence
  --permission-mode dontAsk
  --tools Read,Grep,Glob
  --strict-mcp-config
  --mcp-config <generated-empty-config>
  --output-format json
  --json-schema <inline-result-schema>
```

For `local_plugin`, add:

```text
--plugin-dir <plugin_dir>
```

The configured entrypoint appears only in the stdin prompt.

Requirements:

- do not use `--dangerously-skip-permissions`;
- do not expose Bash, Edit, Write, Web, MCP, or subagent tools;
- do not persist sessions;
- parse stdout as one JSON document;
- reject `is_error: true` or non-success subtype;
- prefer `structured_output`;
- cap stderr in diagnostics.

Claude's cwd is `project_root`, so read reach is broader than `content_root`. Static doctor and every Claude query emit `CLAUDE_READ_SCOPE_BROAD`: `Claude read tools can inspect the configured project root, not only the selected content root; use an OS sandbox or container for stricter confidentiality.`

A pre-implementation live spike must verify the exact current Agents layout, skill discovery, explicit `content_root`, schema output, zero wiki mutations, and every proposed flag name/value against the implementation machine's recorded Claude Code version. The argument vector in this section is proposed rather than timeless: an unsupported help surface blocks implementation until this specification and plan are corrected and re-reviewed. The successful result becomes a versioned capability fixture; no historical provider version is assumed.

### 10.3 Codex Adapter

Target invocation:

```text
codex
  --ask-for-approval never
  exec
  -C <project_root>
  --sandbox read-only
  --ephemeral
  --skip-git-repo-check
  --ignore-user-config
  --output-schema <temporary-schema-file>
  --json
  -
```

Do not use Codex `--add-dir`; its documented purpose is granting an additional writable root.

`--ask-for-approval` is a top-level Codex option in the currently verified CLI and therefore precedes `exec`. The pre-implementation spike treats the structured argument vector—not a shell command string—as a versioned capability fixture and rejects a provider version whose help surface no longer accepts it.

Requirements:

- send the prompt through stdin;
- parse every non-empty stdout line as JSON;
- reject error and failed-turn events;
- select the last completed agent message;
- validate the final message against `wiki-query/v1`;
- cap event diagnostics and stderr;
- treat the traditional read-only sandbox as write prevention, not a guarantee that unrelated filesystem paths are unreadable.

Because the adapter uses `--ignore-user-config`, an operator-managed Codex user permission profile is not a supported hardening path in version 0.1.0. Sensitive knowledge bases that require a strict read allowlist need an OS sandbox/container exposing only the selected wiki. Static doctor and every Codex query emit warning code `CODEX_READ_SCOPE_BROAD` with the message: `Codex read-only sandbox prevents writes but does not limit reads to the selected wiki; use an OS sandbox or container for strict confidentiality.`

## 11. Citation Normalization

`wiki-query/v1` citations are query-provenance pointers to wiki pages the provider read. They are distinct from the formal source footnotes defined by the wiki's `## Citations` schema. Existing footnotes may legitimately target `raw/<file>`, `assets/<file>`, or an HTTP(S) URL; those drive-by targets are permitted in answer prose but are not entries in the model's `citations` array and do not replace page provenance.

The model returns native bare page slugs. The wrapper:

1. reads `link_style` from `SCHEMA.md`, defaulting to `obsidian` when the field is absent;
2. extracts only exact inline page-provenance forms using the matching built-in parser;
3. merges them with the returned `citations` array;
4. removes one outer link delimiter when present;
5. requires every returned-array slug candidate to match the wiki rule `[a-z0-9-]+`;
6. fails with `CITATION_INVALID` when an element of the model `citations` array contains path separators, traversal, anchors, a URL, or unsafe characters;
7. fails with `CITATION_NOT_FOUND` when a syntactically valid page slug does not resolve to an existing page;
8. preserves all inline-extracted slugs in answer order, appends returned-array slugs in array order, and deduplicates the combined sequence by first occurrence;
9. creates public objects containing the selected wiki ID.

The MVP supports two machine-defined page-provenance parsers:

- `obsidian` recognizes only an exact `[[<slug>]]` whose complete inner text matches `[a-z0-9-]+`;
- `markdown` recognizes that same exact form plus exact `[[<slug>](pages/<slug>.md)]`, where both slug occurrences match `[a-z0-9-]+` and are identical.

All other inline text is outside the page-provenance grammar and is ignored by this extractor. In particular, `raw/<file>`, `assets/<file>`, HTTP(S) URLs, `[[raw/<file>]]`, `[[assets/<file>]]`, ordinary Markdown links, display-label wiki links, and malformed/dangling wiki-link prose are neither public page citations nor `CITATION_INVALID`. Strict unsafe-target rejection applies to the explicit model `citations` array. A syntactically recognized inline page slug that has no exact page file remains `CITATION_NOT_FOUND`.

`SCHEMA.md` is UTF-8 Markdown with a machine-readable subsection. The wrapper locates the single exact level-two heading `## Cross-References`, ending at the next line beginning with `## `. A missing or duplicate heading is `WIKI_INVALID`. Within that subsection it accepts at most one exact field line matching `- **link_style:** <value>` and at most one `- **link_style_rules:** <relative-path>` line. Leading indentation, duplicate fields, or trailing prose on either field line is `WIKI_INVALID`. The value grammar is `[a-z0-9]+(?:-[a-z0-9]+)*`; an absent `link_style` field means `obsidian`. A rules path must be a slash-separated relative path, resolve inside `content_root`, and name a regular `.md` file.

The wrapper does not execute a regular expression or other parsing logic copied from Markdown. The referenced rules file remains human-readable documentation: static doctor verifies its existence but does not parse it as configuration. Any other `link_style` fails with `LINK_STYLE_UNSUPPORTED`.

Slug resolution is normative and independent of filesystem case-folding: enumerate regular files directly under `<content_root>/wiki/pages/`, accept only `.md`, strip that exact suffix, and require one exact ASCII filename-stem match for `<slug>`. The target is `<content_root>/wiki/pages/<slug>.md`; a case-insensitive Windows lookup does not authorize a case-mismatched citation.

Direct `raw/`, `assets/`, and HTTP(S) targets anywhere in answer prose, including correctly formed Markdown footnotes or non-page wiki-link-like text, are ignored by the page-provenance extractor, not rejected or emitted as public page citations. A result whose only evidence is a drive-by target still fails `CONTRACT_VIOLATION`: external query answers must identify at least one wiki page that supplied the knowledge. This does not change which formal citation targets wiki pages may contain.

Citation failures are fail-fast and never filter-then-continue. `CITATION_INVALID` and `CITATION_NOT_FOUND` discard the complete answer. `knowledge_status = "grounded"` reaches `CONTRACT_VIOLATION` only when both sources of page citations are empty after deterministic deduplication; it is not a fallback for a rejected citation. `knowledge_status = "no_relevant_material"` with any page citation or without a non-empty gap is also `CONTRACT_VIOLATION`. The wrapper never infers status from prose.

Public citation:

```json
{
  "wiki": "agents",
  "slug": "harness-engineering"
}
```

The wrapper never trusts a model-provided wiki namespace.

## 12. Read-Only Enforcement and Trust Model

Configured roots, skill files, plugin files, and wiki content are trusted operator-controlled inputs. The question is untrusted.

Enforcement layers:

- fixed wiki allowlist;
- canonical path containment;
- no shell command construction;
- stdin question transport;
- fixed provider arguments;
- Claude read/search-only tools;
- Codex read-only sandbox;
- no session persistence;
- empty Claude MCP configuration;
- disabled Codex user configuration;
- no query-time index generator;
- timeout and output limits;
- a mutation-sensitive content snapshot before and after the child process.

The content snapshot covers every directory and regular file beneath the canonical `content_root`, including `SCHEMA.md`, all configuration, executable helpers/hooks, raw/assets, index/log/overview files, and every page. There are no unmonitored content-root subtrees in version 0.1.0. For each accepted entry it records the normalized relative path and whether it is a directory or regular file; regular files additionally record byte length and a streaming SHA-256 digest. The before/after comparison detects additions, removals, directory/regular-file type changes, and same-size content rewrites even when timestamps are preserved. Any symlink, junction, reparse point, mount point, or other special entry aborts preflight/snapshot with `UNSAFE_FILESYSTEM_ENTRY`.

Any difference is `READ_ONLY_VIOLATION`; the model result is rejected and sorted changed relative paths are reported without content. Snapshot comparison runs in a `finally`-equivalent path after success, provider failure, timeout, or output overflow. When a mutation accompanies another non-internal failure, `READ_ONLY_VIOLATION` and exit `7` dominate; the sanitized original failure appears as `error.details.secondary_error`. `INTERNAL_ERROR` dominates only when the wrapper cannot complete the integrity comparison itself. This is a detection layer, not a replacement for OS sandboxing.

Local plugins that declare hooks, MCP servers, settings, or other executable lifecycle components are rejected by static doctor for MVP. Only the configured query skill is allowed.

## 13. Public Result Envelope

Success:

```json
{
  "schema_version": "1.0",
  "ok": true,
  "operation": "query",
  "wiki": {
    "id": "agents",
    "title": "Agents Knowledge Base"
  },
  "agent": "claude",
  "contract": "wiki-query/v1",
  "knowledge_status": "grounded",
  "answer": "Grounded answer with [[harness-engineering]].",
  "citations": [
    {
      "wiki": "agents",
      "slug": "harness-engineering"
    }
  ],
  "gaps": [],
  "warnings": [
    {
      "source": "wrapper",
      "code": "INDEX_MAY_BE_STALE",
      "message": "One or more wiki pages are newer than the generated index."
    },
    {
      "source": "wrapper",
      "code": "CLAUDE_READ_SCOPE_BROAD",
      "message": "Claude read tools can inspect the configured project root, not only the selected content root; use an OS sandbox or container for stricter confidentiality."
    }
  ],
  "duration_ms": 63352,
  "child_exit_code": 0,
  "raw_format": "claude-json"
}
```

Failure:

```json
{
  "schema_version": "1.0",
  "ok": false,
  "operation": "query",
  "wiki": {
    "id": "agents",
    "title": "Agents Knowledge Base"
  },
  "agent": "codex",
  "contract": "wiki-query/v1",
  "knowledge_status": null,
  "answer": null,
  "citations": [],
  "gaps": [],
  "warnings": [],
  "duration_ms": 812,
  "child_exit_code": null,
  "raw_format": null,
  "error": {
    "code": "ENTRYPOINT_UNVERIFIED",
    "message": "The selected entrypoint fingerprint has not passed a current live doctor probe."
  }
}
```

Argument failures before wiki or agent resolution use `null` for the unresolved fields.

Public `warnings` is an array of closed objects with `source`, `code`, and `message`. Wrapper warnings use stable codes such as `INDEX_MAY_BE_STALE`, `CLAUDE_READ_SCOPE_BROAD`, and `CODEX_READ_SCOPE_BROAD`. Each model-supplied warning string is normalized to `{ "source": "provider", "code": "PROVIDER_WARNING", "message": <string> }`. Wrapper warnings appear first in deterministic generation order, followed by provider warnings in model order. This keeps index/sandbox policy machine-distinguishable without changing the model-result contract.

`raw_format` is exactly `claude-json` after a parsed Claude native envelope, `codex-jsonl` after parsed Codex events, or `null` when no native format was successfully established.

Every failure `error` has required string fields `code` and `message` plus an optional object `details`. Error-specific detail schemas are closed to unknown keys. For `OUTPUT_TOO_LARGE`, details are:

```json
{
  "stream": "stdout",
  "limit_bytes": 1048576,
  "observed_bytes": 1048577
}
```

`stream` is exactly `stdout` or `stderr`; both byte counts are non-negative integers and `observed_bytes` is greater than `limit_bytes`.

For `READ_ONLY_VIOLATION`, details are:

```json
{
  "changed_paths": [
    "wiki/pages/harness-engineering.md"
  ],
  "secondary_error": {
    "code": "TIMEOUT",
    "message": "The provider exceeded the configured deadline."
  }
}
```

`changed_paths` is a sorted, unique, non-empty array of slash-separated paths relative to `content_root`. It never contains file content, an absolute path, or traversal. `secondary_error` is optional and contains only the displaced public `code` and sanitized `message`; it never nests another `details` object.

## 14. Error and Exit Contract

Complete error mapping:

| Code | Exit | Meaning |
|---|---:|---|
| `ARGUMENT_INVALID` | 2 | Invalid or ambiguous CLI input |
| `QUESTION_INVALID_UTF8` | 2 | Stdin question is not valid UTF-8 |
| `QUESTION_TOO_LARGE` | 2 | Complete UTF-8 question exceeds `max_question_bytes` |
| `CONFIG_INVALID` | 2 | Missing, malformed, unsupported, or unknown configuration |
| `CONFIG_EXISTS` | 2 | `config init` refuses to overwrite an existing destination |
| `WIKI_NOT_ALLOWED` | 2 | Wiki ID is absent from the registry |
| `PATH_OUTSIDE_ALLOWED_ROOT` | 2 | A configured path escapes its declared root |
| `UNSAFE_FILESYSTEM_ENTRY` | 2 | A trusted root contains a symlink, junction, reparse point, mount point, or special entry |
| `WIKI_INVALID` | 2 | Required wiki structure is missing |
| `QUERY_PROFILE_NOT_FOUND` | 2 | Referenced profile does not exist |
| `PROVIDER_CONFIG_MISSING` | 2 | A wiki enables a provider without a corresponding global provider table |
| `PROVIDER_PROFILE_MISSING` | 2 | An enabled provider has no table in the referenced profile |
| `AGENT_UNSUPPORTED` | 2 | Provider is not enabled for the wiki |
| `CONTRACT_UNSUPPORTED` | 2 | Query contract is not implemented |
| `ENTRYPOINT_INVALID` | 2 | Entrypoint syntax or statically addressable artifacts are invalid |
| `LINK_STYLE_UNSUPPORTED` | 2 | Wiki link style has no built-in page-reference parser |
| `INDEX_STALE` | 2 | Strict freshness policy rejected a stale index |
| `CITATION_INVALID` | 2 | Page-provenance citation syntax is unsafe |
| `CITATION_NOT_FOUND` | 2 | Page-provenance citation does not map to an existing page |
| `CLI_NOT_FOUND` | 3 | Provider executable is unavailable |
| `AUTH_REQUIRED` | 3 | Provider authentication is unavailable |
| `ENTRYPOINT_UNVERIFIED` | 3 | Current entrypoint/provider/config fingerprint has not passed live doctor |
| `NONZERO_EXIT` | 4 | Child process returned non-zero |
| `TIMEOUT` | 5 | Child process exceeded the deadline |
| `OUTPUT_TOO_LARGE` | 5 | Native output exceeded the configured cap |
| `TERMINATION_FAILED` | 5 | The process tree could not be confirmed terminated/reaped |
| `INVALID_NATIVE_OUTPUT` | 6 | Claude JSON or a Codex JSONL event is malformed |
| `NO_FINAL_MESSAGE` | 6 | Codex produced no completed agent message |
| `CONTRACT_VIOLATION` | 6 | Final result does not satisfy `wiki-query/v1` |
| `READ_ONLY_VIOLATION` | 7 | A protected wiki path, type, or content digest changed |
| `INTERNAL_ERROR` | 70 | Unexpected error or an incomplete integrity comparison at the wrapper boundary |

Exit `0` means success. These mappings apply identically to query and command-level doctor failures. A query-time `READ_ONLY_VIOLATION` dominates every exit `2`–`6` error and records the displaced failure as `secondary_error`; `INTERNAL_ERROR` dominates only when integrity verification itself cannot complete.

The wrapper process exit code equals its documented class. A failed invocation still emits exactly one JSON envelope in `--json` mode.

## 15. Doctor

Static checks:

- supported config version and no unknown keys;
- unique valid wiki IDs;
- canonical contained roots;
- required wiki structure;
- query profile and provider availability;
- entrypoint syntax;
- project skill or local plugin skill path;
- local plugin manifest identity and absence of hooks/MCP/settings;
- provider executable and version;
- index freshness;
- platform sandbox warning.

Doctor `checks[].name` is one of `config`, `roots`, `wiki_structure`, `profile`, `entrypoint`, `executable`, `auth`, `index_freshness`, `read_scope`, `live_contract`, or `mutation`. `checks[].code` is `null` for pass, a stable warning code for warn, or one error code from Section 14 for fail. `CODEX_READ_SCOPE_BROAD` and `CLAUDE_READ_SCOPE_BROAD` use check name `read_scope`.

Live checks, only with `--live`:

1. invoke the selected configured entrypoint with a minimal harmless query;
2. require `contract = "wiki-query/v1"`;
3. validate native and normalized outputs;
4. confirm identical protected-tree content snapshots;
5. record the complete successful-probe identity locally outside the wiki;
6. invalidate the probe when any identity or fingerprint value changes.

Live doctor consumes model quota and is never run implicitly by `query`. Every configured project skill and Claude local plugin requires a current live probe for its complete fingerprint before normal query.

### 15.1 Probe Store

Live doctor writes a machine-local cache outside every configured wiki:

```text
Windows:   %LOCALAPPDATA%\llm-wikis\probes-v1.json
Linux/WSL: ${XDG_CACHE_HOME:-~/.cache}/llm-wikis/probes-v1.json
macOS:     ~/Library/Caches/llm-wikis/probes-v1.json
```

The document has this shape:

```json
{
  "schema_version": 1,
  "records": [
    {
      "wiki_id": "agents",
      "canonical_project_root": "E:\\not_company\\llm-wikis\\agents",
      "canonical_content_root": "E:\\not_company\\llm-wikis\\agents\\wiki",
      "agent": "codex",
      "agent_executable": "C:\\path\\to\\codex.exe",
      "agent_version": "0.145.0",
      "profile": "wiki_skill_v1",
      "contract": "wiki-query/v1",
      "load": "project_skill",
      "entrypoint": "$wiki-query",
      "skill_fingerprint": "sha256:...",
      "compatibility_fingerprint": "sha256:...",
      "verified_at": "2026-07-28T12:00:00Z"
    }
  ]
}
```

Doctor computes `skill_fingerprint` over every directory and regular file under the configured project-skill directory. For a Claude local plugin it additionally includes `.claude-plugin/plugin.json` and rejects hooks, MCP, settings, or other executable lifecycle components. Any special filesystem entry is `UNSAFE_FILESYSTEM_ENTRY`.

Artifact fingerprints use SHA-256 over this deterministic byte stream for files sorted by ordinal normalized relative path: UTF-8 path with `/` separators, one NUL byte, unsigned 64-bit big-endian file length, raw file bytes. The stored form is lowercase `sha256:` plus 64 hexadecimal digits.

The logical record key is exactly canonical content/project roots, selected provider, profile name, load mode, and exact entrypoint. Exactly zero or one record may exist for that logical key.

The record's current-verification tuple additionally contains canonical provider executable path/version, contract, skill fingerprint, and normalized `compatibility_fingerprint`. The compatibility fingerprint is SHA-256 over canonical JSON for the selected wiki, profile, provider executable declaration, and implementation-owned provider safety/contract version after validation and path resolution. Execution-policy and presentation fields such as timeout, byte limits, freshness mode, comments, and TOML key order do not invalidate a successful contract/entrypoint probe.

A successful live doctor removes every existing record for the logical key and atomically writes exactly one record with the current-verification tuple. It never retains historical fingerprints for the same logical key. Query resolves the logical key, requires exactly one record, and compares every current-verification field; an APM/skill rollback therefore fails unless that exact current state passes a new live doctor. A missing, malformed, duplicate, or mismatched record is unverified; there is no independent time-to-live in version 1. Doctor writes through an atomic temporary-file replacement and requests user-only permissions where the platform supports them. Query only reads the cache. Probe prompts, answers, and wiki content are never stored.

## 16. Testing Strategy

Implementation follows test-driven development with Rust unit, integration, CLI, and platform-specific process tests. Offline CI never invokes a paid provider.

### 16.1 Offline Unit Tests

Configuration:

- valid current Agents configuration;
- malformed TOML, wrong version, unknown keys, duplicate IDs;
- missing profiles and unsupported contracts;
- valid differently named skills;
- valid Claude plugin namespace;
- valid Codex `$name` and rejected/deferred `$plugin-name:skill-name` entrypoints;
- invalid entrypoint whitespace, control characters, and metacharacters;
- path traversal and unconditional symlink, junction, reparse, mount, and special-entry rejection;
- content root outside project root;
- forbidden raw CLI and prompt fields.

Preflight:

- required wiki paths;
- deterministic `SCHEMA.md` cross-reference field parsing, including missing/duplicate headings and fields;
- missing and stale index;
- warn and error freshness policies;
- equal/coarse timestamps;
- plugin manifest rejection when executable components exist;
- all-entrypoint live-probe requirement;
- missing, malformed, duplicate, stale, and version/fingerprint-invalidated probe records;
- project-skill and Claude local-plugin artifact fingerprint changes;

Claude parser:

- success JSON with structured output;
- error subtype and `is_error`;
- malformed JSON;
- stderr warning;
- non-zero exit, timeout, and oversized output.

Codex parser:

- valid JSONL and final agent message;
- malformed line;
- failed turn or error event;
- missing final message;
- final schema violation;
- stderr warning, non-zero exit, timeout, and oversized output.

Process supervision:

- concurrent stdout/stderr draining without deadlock;
- independent byte caps with the offending stream reported;
- monotonic timeout across the complete lifecycle;
- process-tree termination and reap on every failure path;
- stdin closure and Traditional Chinese/multiline transport;
- Windows `.exe` and `.cmd` provider resolution;
- paths with spaces and representative shell metacharacters;
- no untrusted question data in provider argv.

Normalization:

- native inline citation extraction;
- returned citation merging and ordering;
- built-in Obsidian and Markdown parsing;
- unsupported link style rejection;
- unsafe slug rejection;
- missing page rejection;
- wrapper-added wiki namespace;
- grounded result with citations;
- structured `no_relevant_material` result with no citations and a non-empty gap;
- invalid status/citation/gap combinations;
- full-content snapshot additions, removals, type changes, and same-size rewrites with preserved timestamps.

CLI:

- `--version` reports the Cargo package version;
- `config init` creates platform-native parents and a valid empty registry;
- `config init` refuses to overwrite an existing file;
- question argument and stdin forms;
- rejection when both are present;
- exactly one wiki requirement;
- human and JSON output;
- stdout/stderr separation;
- per-stream size details and doctor failure-class precedence;
- exit-code mapping;
- Unicode paths and Traditional Chinese questions;
- leading dashes, quotes, shell metacharacters, and multiline input without shell interpretation.

### 16.2 Live Matrix

| Platform | Provider | Load mode | Required assertions |
|---|---|---|---|
| Windows | Claude | current project skill | Correct skill, explicit content root, valid JSON, grounded citations, no mutation |
| Windows | Codex | current project skill | Correct skill, valid JSONL parsing, grounded citations, no mutation |
| Linux/WSL | Claude | current project skill | Same assertions and portable path handling |
| Linux/WSL | Codex | current project skill | Same assertions and sandbox behavior recorded |
| macOS Apple Silicon | Claude | current project skill | Native ARM binary, zsh environment, explicit content root, grounded citations, no mutation |
| macOS Apple Silicon | Codex | current project skill | Native ARM binary, JSONL parsing, sandbox behavior recorded, no mutation |
| Windows or Linux/WSL | Claude | differently named local plugin skill | Configured namespace resolves; plugin contains no rejected components |

Each live test records provider version, duration, raw format, warnings, and before/after protected-tree digests. Live tests are separate from the fast offline suite because they consume time and model quota. Binary distribution support and live provider verification are reported separately: a native build may be released after its offline and installer gates pass, but documentation must not claim live Claude/Codex verification for a platform until its corresponding rows pass.

## 17. Pre-Implementation Gates

No production implementation starts until both gates below pass.

### 17.1 Independent Acceptance Checklist

An independent review session receives only this approved specification and the implementation-plan path. Before any production code is written, it creates `docs/verification/llm-wikis-v0.1.0-checklist.md`.

The main/orchestrating session and implementation workers must not author, weaken, remove, or self-mark checklist requirements. The checklist:

- maps every normative acceptance requirement to one or more observable checks;
- identifies the platform and evidence required for each check;
- separates offline, native-binary, installer, live-provider, and release assertions;
- uses one row per independently verifiable behavior;
- starts with every row unresolved.

The checklist author also creates `docs/verification/llm-wikis-v0.1.0-checklist-baseline.json`, a canonical UTF-8 JSON document containing every immutable row field—ID, requirement, platform, phase, command/inspection, and expected result. Its raw-file SHA-256 and row count are frozen in the execution record. Status/evidence remain in the Markdown checklist. Final verification re-hashes/recounts the untouched baseline file and fails closed on any unapproved change or deleted row.

After implementation, a second independent verification session—distinct from both the checklist author and all implementation workers—checks rows one at a time and records the command, exit status, relevant sanitized output, and artifact path under `docs/verification/evidence/llm-wikis-v0.1.0/`. A failed or unavailable row remains failed or pending; it is never converted to pass through prose. Material checklist defects require user-approved spec/checklist correction rather than implementation-session edits.

### 17.2 Disposable Capability Spikes

Small, disposable Rust spikes must validate the risky assumptions before the production crate is implemented:

1. resolve and invoke the installed Claude executable and Codex executable/shim;
2. transport Traditional Chinese, multiline text, quotes, leading dashes, and shell metacharacters through stdin without question bytes appearing in argv;
3. confirm the exact current Claude JSON and Codex JSONL command surfaces and sanitized output shapes;
4. load a disposable `wiki-query/v1` fixture skill using distinct temporary `project_root` and contained `content_root` values, proving provider discovery without depending on the not-yet-created production overlay;
5. enforce independent stdout/stderr caps and timeout without deadlock;
6. terminate and reap a deliberately spawned child process tree on Windows, Linux, and macOS where the corresponding native environment is available;
7. prove full-content mutation detection catches a same-size rewrite with a restored timestamp;
8. prove platform config/cache directories and executable resolution, including a Windows `.cmd` shim and paths containing spaces;
9. compile and run `llm-wikis --version`-equivalent smoke binaries for all three release targets on native Windows x64, Ubuntu x64, and GitHub-hosted macOS ARM64 environments.

Spike source is isolated from production modules and deleted or retained only under a clearly marked `spikes/` directory; production code must be developed again through failing tests. Sanitized commands, versions, results, and unresolved platform rows are recorded in `docs/verification/llm-wikis-preflight.md`.

The local implementation platform's provider flags/output and process primitive must pass before production code. Native CI may satisfy the Linux/macOS process and target-smoke rows later, but before release. An unavailable native environment is recorded as `PENDING`, not misreported as failure or pass. Any pending native binary/process/installer row blocks the entire three-asset `v0.1.0` release; a pending paid live-provider row blocks only that provider/platform support claim. A demonstrated `FAIL` in any provider flag, output shape, process primitive, target toolchain, or platform assumption stops the affected implementation/release path until the specification and plan are corrected and re-reviewed.

## 18. Release and Installation

### 18.1 Release Assets

Release automation requires a user-approved Git repository with a configured GitHub remote, Actions enabled, and permission to create Releases/attestations. Core implementation may proceed without that external state, but version 0.1.0 cannot be published until the prerequisite and every three-platform native row pass.

A `v0.1.0` tag triggers the GitHub Actions release workflow. The tag must equal the Cargo package version. Quality gates run before any GitHub Release is published:

```text
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

The workflow builds and natively smoke-tests:

| Asset | Rust target | Support |
|---|---|---|
| `llm-wikis-windows-amd64.exe` | `x86_64-pc-windows-msvc` | Windows x64 |
| `llm-wikis-linux-amd64` | `x86_64-unknown-linux-musl` | Linux/WSL x64 |
| `llm-wikis-darwin-arm64` | `aarch64-apple-darwin` | macOS Apple Silicon |

It also publishes `SHA256SUMS` and GitHub artifact attestations for the
executables and checksum manifest. `install.ps1` and `install.sh` remain
source-controlled files served from raw repository URLs; they are not Release
assets.

Release assets are immutable for a version. A failed or pending Windows, Linux, or macOS native matrix row, checksum, smoke test, or installer test prevents the entire `v0.1.0` publication; version 0.1.0 is not released with a partial asset set. The workflow never invokes Claude or Codex. Pending paid live-provider rows block only their explicit support claims, not the binary release.

### 18.2 Installers

`install.ps1`:

- downloads latest unless `LLM_WIKIS_VERSION` selects an explicit version;
- recognizes Windows x64 only;
- downloads and verifies the matching raw `.exe` against `SHA256SUMS`;
- runs the downloaded binary with `--version` before installation;
- installs to `%LOCALAPPDATA%\llm-wikis\bin\llm-wikis.exe`;
- adds that directory to the user PATH only when absent, without duplicating entries.

`install.sh` follows the proven `apm-go` release-download flow:

- downloads latest unless `LLM_WIKIS_VERSION` selects an explicit version;
- accepts Linux x86-64 and macOS ARM64 only, with explicit errors for Intel Mac and unsupported Linux architectures;
- downloads the raw binary and `SHA256SUMS` into a temporary directory;
- fails closed when neither `sha256sum` nor `shasum` is available;
- runs the downloaded binary with `--version` before installation;
- installs to `~/.local/bin/llm-wikis`;
- when needed, idempotently adds that directory to `~/.zprofile` for macOS zsh or `~/.profile` for Linux/bash/sh;
- prints manual PATH instructions for an unsupported shell instead of editing an unknown profile.

Both installers clean temporary files on success and failure. Re-running an installer upgrades, downgrades, or repairs the binary. Version 0.1.0 has no separate uninstaller or self-update command.

The macOS binary is an unsigned preview. Documentation explains Gatekeeper's normal user approval flow without advising users to disable platform security. Windows documentation similarly warns that the unsigned binary may trigger SmartScreen. SHA-256 and GitHub attestations provide integrity/provenance evidence but do not replace platform code signing.

## 19. Rollout

### Phase 0 — Independent Gates

- independent acceptance checklist;
- disposable Rust capability spikes;
- corrected and re-reviewed spec/plan if any assumption fails.

### Phase 1 — Query Core

- Cargo project and strict configuration;
- wiki resolver and preflight;
- project-owned `agents/overrides/wiki-query/SKILL.md`, deterministic deployment scripts, and byte-identical Claude/Codex copies;
- process supervisor and Claude/Codex adapters;
- result normalization, citation validation, mutation hashing, and errors;
- `--version`, `config init`, `list`, static `doctor`, and `query`;
- offline tests.

### Phase 2 — Hardening and Distribution

- live doctor and probe invalidation;
- plugin entrypoint validation;
- platform-native offline and process tests;
- install scripts and GitHub Release workflow;
- operator and security documentation;
- explicitly authorized live-provider rows.

### Phase 3 — Independent Verification

- freeze implementation changes except verified fixes;
- execute the independent checklist one row at a time;
- send failures back through failing regression tests;
- rerun affected rows and the complete offline suite;
- publish only the support claims backed by recorded evidence.

MCP and orchestration require separate approved designs after the query MVP is stable.

## 20. Acceptance Criteria

- `llm-wikis` is a Rust 0.1.0 binary with no Python runtime dependency.
- A command launched outside the wiki project can query the registered Agents wiki without copy/paste.
- The caller selects one allowlisted wiki ID, never a path.
- Current `/wiki-query` and `$wiki-query` entrypoints are configuration, not product constants.
- Provider executables use safe defaults and trusted command-name/absolute-path overrides.
- A differently named compatible project skill works by changing configuration only.
- Every project-skill and Claude local-plugin entrypoint works only after a current fingerprinted live probe.
- The current Agents external contract is owned by the project overlay; APM refresh cannot silently restore write-capable query behavior.
- Both providers receive the same fixed external query contract through stdin.
- Claude and Codex use independently tested argument vectors and native-output parsers.
- Every successful answer has a mechanically validated `knowledge_status`: `grounded` with citations or `no_relevant_material` with a non-empty gap.
- Every accepted citation resolves to an existing page and carries the wrapper-selected wiki ID.
- Query does not regenerate the index, save an answer, update a log, or modify wiki content.
- Full-content snapshots reject additions, removals, type changes, and same-size rewrites.
- The UTF-8 question is rejected before provider startup when invalid or larger than `max_question_bytes`.
- Stale indexes produce the configured warning or failure.
- JSON mode always emits one normalized document.
- Timeout, auth, native-output, contract, citation, and mutation failures are stable and machine-readable.
- `config init` creates a valid platform-native template and never overwrites.
- All three native executables pass offline and installer smoke gates before release.
- macOS support is Apple Silicon only and clearly marked as unsigned preview.
- Live provider claims are limited to platform/provider rows actually executed successfully.
- An independent checklist exists before production implementation and is verified row-by-row afterward.
- No orchestration or MCP code is included in this implementation.

## 21. References

- Project-owned query overlay: `<root>/agents/overrides/wiki-query/SKILL.md`
- Deployed query behavior: `<root>/agents/.agents/skills/wiki-query/SKILL.md` and `<root>/agents/.claude/skills/wiki-query/SKILL.md`
- Upstream interactive baseline: `<root>/agents/apm_modules/kfchou/wiki-skills/skills/wiki-query/SKILL.md`
- Current dependency targets: `<root>/agents/apm.yml`
- Claude CLI reference: <https://docs.anthropic.com/en/docs/claude-code/cli-usage>
- Claude skills and additional-directory behavior: <https://code.claude.com/docs/en/skills>
- Claude plugin namespace behavior: <https://code.claude.com/docs/en/plugins>
- Codex CLI reference: <https://learn.chatgpt.com/docs/developer-commands?surface=cli>
- Codex skill discovery and invocation: <https://learn.chatgpt.com/docs/build-skills>
- Codex permission profiles: <https://learn.chatgpt.com/docs/permissions>
- Rust process API: <https://doc.rust-lang.org/stable/std/process/index.html>
- Cargo build targets and release profiles: <https://doc.rust-lang.org/cargo/commands/cargo-build.html>
- GitHub artifact attestations: <https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/use-artifact-attestations>
- `apm-go` installer reference: <https://raw.githubusercontent.com/gn00678465/apm-go/refs/heads/main/install.sh>
- CLI-Anything principles: <https://raw.githubusercontent.com/HKUDS/CLI-Anything/refs/heads/main/README.md>

## 22. Repository Note

This workspace is not currently a Git repository. Documentation can be written and reviewed here, but implementation must either initialize/use an explicitly approved Git repository and isolated worktree or record that work is intentionally being executed without commit checkpoints. The intentionally removed `development-handoff.md` must not be recreated.
