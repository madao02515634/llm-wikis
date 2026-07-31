# llm-wikis v0.1.0 Execution Record

Authoritative specification: `docs/superpowers/specs/2026-07-28-llm-wikis-external-query-design.md`  
Authoritative plan: `docs/superpowers/plans/2026-07-28-llm-wikis-query-cli.md`

## Independent Task 1 checkpoint

- Recorded at: `2026-07-29T08:28:03+08:00`
- Checklist author/session: `/root/task01_checklist_author`
- Role: independent acceptance-checklist author only; no production implementation or final-verification role
- Checklist: `docs/verification/llm-wikis-v0.1.0-checklist.md`
- Immutable baseline: `docs/verification/llm-wikis-v0.1.0-checklist-baseline.json`
- Baseline schema version: `1`
- Baseline row count: `205`
- Baseline SHA-256 over raw file bytes: `6fbd12c5b4e25f1b52d867d39c65b7ecfefbfa1ded99532f92c1404af63a60d9`
- Baseline encoding: UTF-8 without BOM, LF line endings, one trailing LF
- Initial mutable row state: every Markdown row is `PENDING` with empty Evidence

The baseline freezes exactly these immutable row fields: ID, requirement, platform, phase, command or inspection, and expected result. Baseline objects use lexicographically sorted keys and two-space JSON indentation. Later verification may update only Status and Evidence in the Markdown checklist.

The main/orchestrating session and implementation workers may not author, edit, weaken, remove, or replace checklist requirements or expected results. The baseline is immutable. Any replacement requires explicit user approval and a new recorded raw SHA-256 and row count that preserves the prior hash history.

Final row-by-row status and evidence updates are reserved for a second independent verifier who is not `/root/task01_checklist_author` and did not implement the feature. That verifier must record sanitized evidence under `docs/verification/evidence/llm-wikis-v0.1.0/`.

## User-approved baseline replacement

- Approval: explicit user approval received in the main/orchestrating session `/root` on `2026-07-29`
- Original approval timestamp provenance: the historical approval date is `2026-07-29`; the exact original user-message timestamp was not captured and is not reconstructed here
- Scope-bound continuation/reaffirmation: on `2026-07-29` the user confirmed continuation of the already-approved Task 1 replacement loop and clarified that Task 1 must not pre-implement or pre-audit future Tasks 3–13 test harnesses
- Scope-bound continuation/reaffirmation result recorded at: `2026-07-29T09:56:33+08:00`
- Scope-bound continuation/reaffirmation provenance: controller-recorded result; the precise original approval time remains unavailable
- Replacement author/session: `/root/task01_checklist_author`
- Recorded at: `2026-07-29T08:45:39+08:00`
- Precise reason: independent Task 1 spec-compliance review findings
- Prior baseline row count: `205`
- Prior baseline SHA-256 over raw file bytes: `6fbd12c5b4e25f1b52d867d39c65b7ecfefbfa1ded99532f92c1404af63a60d9`
- Replacement baseline row count: `221`
- Replacement baseline SHA-256 over raw file bytes: `035d6317b8b5cf14bcf0f77024b49172f227cb99e1d1bf266ae4b16d3cbb9860`
- Replacement encoding: UTF-8 without BOM, LF line endings, one trailing LF
- Replacement mutable row state: every Markdown row is `PENDING` with empty Evidence

The approved replacement preserves every prior checklist row and stable ID, strengthens `OFF-104`, `OFF-112`, and `PROC-023`, and adds `OFF-131` through `OFF-144` plus `PROC-029` and `PROC-030`. The prior hash and row count above remain permanent history; they are not current-baseline values.

## Second user-approved baseline replacement

- Approval: explicit second user approval received in the main/orchestrating session `/root` at `2026-07-29T09:12:43+08:00`
- Replacement author/session: `/root/task01_checklist_author`
- Precise reason: mandatory quality-review corrections before any next implementation-plan task
- Original baseline: row count `205`, raw SHA-256 `6fbd12c5b4e25f1b52d867d39c65b7ecfefbfa1ded99532f92c1404af63a60d9`
- First approved replacement: row count `221`, raw SHA-256 `035d6317b8b5cf14bcf0f77024b49172f227cb99e1d1bf266ae4b16d3cbb9860`
- Replacement encoding: UTF-8 without BOM, LF line endings, one trailing LF
- Replacement mutable row state: every Markdown row is `PENDING` with empty Evidence

### Second-replacement candidate review history

- Candidate 1 frozen at: `2026-07-29T09:16:18+08:00`
- Candidate 1 row count: `230`
- Candidate 1 raw SHA-256: `f9cbe69e3e87e5a761084338b6ca8ec61f2b994a8c9b34ce48e018d84d3f8c81`
- Candidate 1 status: `REJECTED_BY_SAME_SPEC_REVIEWER`
- Candidate 1 rejection recorded at: `2026-07-29T09:28:44+08:00`
- Candidate 1 rejection reason: same spec-review findings—invalid OFF-133 regex syntax; insufficient OFF-128/OFF-145 README/no-copy lineage proof; insufficient PROC-036 deadline/classification/bound proof; weakened grounded/citation assertions in platform live rows; and incorrect REL-010 triggering-tag/asset-ID semantics.
- Candidate 2 frozen at: `2026-07-29T09:28:44+08:00`
- Candidate 2 row count: `230`
- Candidate 2 raw SHA-256: `74ba4a58097a8b1d39c748b6c9ede0e286cfdf2e8c42da046c8865f206ec8232`
- Candidate 2 status: `REJECTED_BY_SAME_SPEC_REVIEWER`
- Candidate 2 rejection recorded at: `2026-07-29T09:36:30+08:00`
- Candidate 2 rejection reason: the OFF-145 Git-mode lineage command used an outer double-quoted child PowerShell script that expanded `$f` before the child received it, causing a parser error; it also passed absolute rather than repository-relative paths to `git log --follow`.
- Candidate 3 frozen at: `2026-07-29T09:36:30+08:00`
- Candidate 3 row count: `230`
- Candidate 3 raw SHA-256: `ce6e32b8eba20c5a0e95d27391cd2b94e01a551d7d2441193a8542418595d732`
- Candidate 3 same-spec review result: `APPROVED_BY_SAME_SPEC_REVIEWER`
- Candidate 3 same-spec reviewer identity: `/root/task01_spec_compliance`
- Candidate 3 same-spec result recorded at: `2026-07-29T09:56:33+08:00`
- Candidate 3 same-spec result provenance: controller-recorded result; original reviewer completion timestamp unavailable
- Candidate 3 same-spec result detail: compliant with zero open spec findings
- Candidate 3 status: `REJECTED_BY_SAME_QUALITY_REVIEWER`
- Candidate 3 quality reviewer identity: `/root/task01_quality_review`
- Candidate 3 quality result recorded at: `2026-07-29T09:56:33+08:00`
- Candidate 3 quality result provenance: controller-recorded result; original reviewer completion timestamp unavailable
- Candidate 3 rejection reason: Important blockers in review-state provenance; non-vacuity governance for future planned Rust-test acceptance mappings; references to plan-unowned test targets; raw Cargo metadata capable of retaining absolute paths; and lineage evidence that was neither structured per relative file nor explicitly oldest-first. The user subsequently clarified that Task 1 must address future-test non-vacuity through owning-task governance, not by globally rewriting commands or requiring unimplemented Tasks 3–13 tests to exist now.
- Candidate 4 frozen at: `2026-07-29T09:56:33+08:00`
- Candidate 4 row count: `230`
- Candidate 4 raw SHA-256: `ee4fbc47a64f6f878f57658025a89a5a43f36a92fd9f69561d10d9dc9b50f42a`
- Candidate 4 same-spec review result: `APPROVED_BY_SAME_SPEC_REVIEWER`
- Candidate 4 same-spec reviewer identity: `/root/task01_spec_compliance`
- Candidate 4 same-spec result recorded at: `2026-07-29T10:05:25+08:00`
- Candidate 4 same-spec result provenance: controller-recorded result; original reviewer completion timestamp unavailable
- Candidate 4 same-spec result detail: spec compliant with zero open findings
- Candidate 4 quality review attempt result: `CORRECTION_REQUESTED_BY_SAME_QUALITY_REVIEWER`
- Candidate 4 quality reviewer identity: `/root/task01_quality_review`
- Candidate 4 quality result recorded at: `2026-07-29T10:05:25+08:00`
- Candidate 4 quality result provenance: controller-recorded result; original reviewer completion timestamp unavailable
- Candidate 4 quality result detail: one Important governance mismatch remained—the completed same-spec approval and the scope of the later checkpoint statement were not recorded coherently
- Candidate 4 post-correction state before re-review: `AWAITING_SAME_QUALITY_REVIEWER_RE-REVIEW`
- Candidate 4 same-quality review result: `APPROVED_BY_SAME_QUALITY_REVIEWER`; `Ready=Yes`
- Candidate 4 same-quality reviewer identity: `/root/task01_quality_review`
- Candidate 4 same-quality result recorded at: `2026-07-29T10:07:22+08:00`
- Candidate 4 same-quality result provenance: controller-recorded result; original reviewer completion timestamp unavailable
- Candidate 4 same-quality result detail: zero open Critical, Important, or Minor findings
- Candidate 4 current status: `READY_FOR_CONTROLLER_EVIDENCE_VERIFICATION`

The explicitly approved scope of this second replacement is:

1. normalize Markdown table escape `\|` to the actual literal `|` logical immutable value in baseline JSON;
2. audit every prior row so command or inspection is an exact row-addressable command or closed numbered procedure and expected result is deterministic;
3. convert compound/self-certifying rows `OFF-091`, `OFF-128`, and `PROC-028` into closed prerequisite roll-ups and add only behavior-split rows `OFF-145`, `OFF-146`, and `PROC-031` through `PROC-037`;
4. require approved synthetic content roots and a closed sanitized schema for all recorded live/preflight evidence;
5. replace `REL-010` retrospective immutability prose with an enforceable no-clobber workflow and manifest/attestation binding;
6. add this active-baseline block and the mandatory per-task review loop while retaining both superseded histories.

No prior ID was deleted or renumbered. The second replacement preserves `OFF-001` through `OFF-144`, `PROC-001` through `PROC-030`, and all `NAT`, `INST`, `LIVE`, and `REL` IDs; it appends only the nine split-behavior IDs listed above.

## Active baseline

<!-- ACTIVE_BASELINE_BEGIN -->
```json
{
  "row_count": 230,
  "schema_version": 1,
  "sha256": "ee4fbc47a64f6f878f57658025a89a5a43f36a92fd9f69561d10d9dc9b50f42a"
}
```
<!-- ACTIVE_BASELINE_END -->

The delimited JSON block above is the only machine-readable active-baseline record. Historical baseline values remain immutable history and are not active.

## Per-task mandatory review loop

For every implementation-plan Task 1 through Task 17, the controller must complete this loop before marking that task complete or starting the next task:

Scope boundary: this loop reviews only artifacts required by the current Task. Task 1 establishes checklist behavior, command/inspection recipes, the immutable baseline, and review governance; it does not require the present existence of production test functions, fixtures, or harnesses assigned to unimplemented Tasks 3–13. For each checklist row mapped to a planned test target, that target's owning implementation Task must create the named test and fixtures through its TDD work, and its implementer/spec/quality review loop must prove the exact checklist command is non-vacuous before the owning Task can be marked complete. Task 17 verifies the rows after implementation; Task 1 does not fail merely because those future tests do not yet exist.

1. the assigned implementer completes the scoped work and fresh self-verification;
2. the assigned spec reviewer reviews the task against the authoritative spec and plan;
3. the same implementer fixes every spec-review finding, then the same spec reviewer re-reviews; repeat until the spec reviewer reports compliant with no open finding;
4. the assigned quality reviewer reviews correctness, readability, architecture, security, performance, evidence quality, and task scope;
5. the same implementer fixes every quality-review finding, then the same quality reviewer re-reviews; repeat until there is no open Critical or Important finding and the quality reviewer records `Ready=Yes`;
6. the main/orchestrating session independently runs fresh evidence verification for the task's required commands, reads the complete results, and records the checkpoint;
7. only after steps 1–6 pass may the main session mark the task complete and authorize the next task.

No next task may start while the implementer reports any concern, any spec/quality review issue remains open, either required same-reviewer re-review is missing, `Ready` is not `Yes`, or fresh evidence verification fails or is incomplete. Task 17 additionally retains the approved specification's distinct independent-checklist-verifier requirements; this loop does not permit its checklist author, implementers, or prior reviewers to replace that verifier.

## Task 1 controller evidence verification

- Recorded at: `2026-07-29T10:09:14.7366301+08:00`
- Controller: `/root`
- Fresh evidence result: `PASS`
- Checklist Markdown rows / baseline rows / declared rows: `230 / 230 / 230`
- Unique IDs / `PENDING` rows / non-empty Evidence cells: `230 / 230 / 0`
- Logical Markdown-to-baseline immutable-field mismatches: `0`
- Baseline encoding: UTF-8 without BOM, LF-only, exactly one trailing LF
- Active baseline schema / row count / SHA-256: `1 / 230 / ee4fbc47a64f6f878f57658025a89a5a43f36a92fd9f69561d10d9dc9b50f42a`
- Candidate 4 same-spec review: approved with zero open findings by `/root/task01_spec_compliance`
- Candidate 4 same-quality review: `Ready=Yes` with zero Critical, Important, or Minor findings by `/root/task01_quality_review`
- Production crate check: `Cargo.toml` absent, as required before Task 2/Task 3 progression
- Result: Task 1 is complete. Task 2 may start under the mandatory review loop.

## Subsequent execution checkpoints

The Candidate 1–4 review events and the controller verification above are Task 1 history. No Task 2-or-later spike, production, implementation, native-platform, live-provider, installer, or release checkpoint is recorded here. Later authorized owners append those checkpoints without altering the frozen Task 1 values above.
