---
phase: "05"
slug: "divergence-check-and-command-contract"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-18"
---

# Phase 05 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo test run by cargo-nextest (pinned 0.9.148, `scripts/install-tools.sh`), proptest 1.12 |
| **Config file** | `scripts/check.sh` (always passes `--no-tests=fail`) |
| **Quick run command** | `scripts/check.sh test -E 'test(=<module::name>)'`, one test per command chained with `&&` |
| **Full suite command** | `scripts/check.sh all` (fmt, clippy, test); phase gate also `scripts/check-in-container.sh all` and `scripts/check.sh deps` |
| **Estimated runtime** | not measured at planning |

---

## Sampling Rate

- **After every task commit:** the task's named tests, one `-E 'test(=...)'` filter each, then `scripts/check.sh all` (every plan's verify ends on it); RED commits are gated on `RED_EVIDENCE_OK`
- **After every plan wave:** `scripts/check.sh all`
- **Before `/gsd-verify-work`:** `scripts/check-in-container.sh all` and `scripts/check.sh deps` green (05-09 Task 2)
- **Max feedback latency:** not measured at planning; one named test per command

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 05-01-01 | 01 | 1 | DIV-01 | T-05-01, T-05-02, T-05-04 | undeclared added path refused; terminal bytes C-quoted; replace refs refused before any tree read | harness scenario | `scripts/check.sh test -E 'test(=check::self_test::rust)'` | ✅ tests/harness/check.rs | ⬜ pending |
| 05-01-02 | 01 | 1 | IF-01 | T-05-03 | every error has one exit class, no wildcard arm (planted-wildcard control) | unit + source check | `scripts/check.sh test -E 'test(=exit::tests::every_git_error_has_its_class)'` | ❌ src/exit.rs, RED step | ⬜ pending |
| 05-02-01 | 02 | 2 | IF-01 | T-05-05, T-05-07 | JSON strings escaped per RFC 8259; non-UTF-8 bytes as hex, never lossy | unit (proptest) | `scripts/check.sh test -E 'test(=json::tests::strings_escape_as_rfc8259)'`, `scripts/check.sh deps` | ❌ src/json.rs, RED step | ⬜ pending |
| 05-02-02 | 02 | 2 | IF-01, DIV-01 | T-05-06 | `--json` stdout is exactly one object for every outcome | contract | `scripts/check.sh test -E 'test(=contract::check_json_reports_problems_as_one_object)'` | ❌ tests/harness/contract.rs, RED step | ⬜ pending |
| 05-03-01 | 03 | 2 | DIV-02, DIV-03 | T-05-09, T-05-11 | NUL-framed tree requests, no pathspec; --no-lazy-fetch | unit (git fixture) | `scripts/check.sh test -E 'test(=git::trees::tests::lookup_paths_answers_each_request_in_order)'` | ❌ src/git/trees.rs, RED step | ⬜ pending |
| 05-03-02 | 03 | 2 | DIV-03, DIV-04 | T-05-10, T-05-11, T-05-12 | reflog read after --end-of-options; git's effective hooksPath with GIT_CONFIG dropped | unit (git fixture) | `scripts/check.sh test -E 'test(=git::staleness::tests::ref_moved_at_is_the_newest_entry)'` | ❌ src/git/staleness.rs, RED step | ⬜ pending |
| 05-04-01 | 04 | 3 | DIV-01, DIV-02 | T-05-13 | no pathspec from a manifest entry | unit (pure evaluator) | `scripts/check.sh test -E 'test(=check::tests::modified_shared_file_must_be_declared_exactly)'` | ❌ src/check/mod.rs tests, RED step | ⬜ pending |
| 05-04-02 | 04 | 3 | DIV-01, DIV-02 | T-05-14, T-05-15 | modifications and deletions judged; workspace branch, not HEAD | harness scenario (both sides) | `scripts/check.sh test -E 'test(=check::stale_only::rust)'`, `scripts/check.sh test -E 'test(=registry::every_scenario_is_well_formed)'` | ✅ tests/harness/check.rs | ⬜ pending |
| 05-05-01 | 05 | 3 | IF-02 | T-05-16, T-05-17, T-05-18 (accept) | no agent-file path literal in src/ (planted-read control) | integration (source scan) | `scripts/check.sh test -E 'test(=planted_read_trips_the_scan)'` | ❌ tests/if02_scan.rs, RED step | ⬜ pending |
| 05-05-02 | 05 | 3 | IF-02 | T-05-16 | every command in the table runs in a repo without agent files | harness registry | `scripts/check.sh test -E 'test(=registry::a_command_without_a_no_agent_files_run_is_named)'` | ✅ tests/harness/registry.rs | ⬜ pending |
| 05-06-01 | 06 | 4 | DIV-03 | T-05-19, T-05-20 | symlinks resolved in-tree, 40-hop budget; escape or absolute target fails | unit (pure resolver) | `scripts/check.sh test -E 'test(=check::hooks::tests::plain_hook_fails_naming_its_mode)'` | ❌ src/check/hooks.rs, RED step | ⬜ pending |
| 05-06-02 | 06 | 4 | DIV-03 | T-05-21, T-05-22 | modes from the committed tree; an unchecked hooksPath always reported | harness scenario + contract | `scripts/check.sh test -E 'test(=check::hook_mode_from_the_commit::rust)'` | ✅ tests/harness/check.rs | ⬜ pending |
| 05-07-01 | 07 | 5 | DIV-04 | T-05-24 | staleness cannot set the exit code; a failed read is a null time with its reason | unit (pure) | `scripts/check.sh test -E 'test(=check::staleness::tests::age_text_at_each_boundary)'`, `scripts/check.sh test -E 'test(=check::staleness::tests::a_failed_read_is_a_null_time_with_its_reason)'` | ❌ src/check/staleness.rs, RED step | ⬜ pending |
| 05-07-02 | 07 | 5 | DIV-04 | T-05-23, T-05-24 | offline: no network subcommand in src/ (planted-literal control); read failure keeps the exit code | harness scenario + contract + source check | `scripts/check.sh test -E 'test(=contract::check_json_staleness_after_a_fetch)'`, `scripts/check.sh test -E 'test(=contract::check_json_staleness_read_failure_keeps_the_exit_code)'` | ✅ tests/harness/check.rs; contract.rs from 05-02 | ⬜ pending |
| 05-08-01 | 08 | 6 | IF-01 | T-05-28 | strict schema (additionalProperties false) rejects drifted output | contract | `scripts/check.sh test -E 'test(=contract::every_command_output_matches_its_schema)'` | ❌ docs/json/check.v1.schema.json, RED step | ⬜ pending |
| 05-08-02 | 08 | 6 | IF-01 | T-05-26, T-05-27 | schema edit without a version bump fails; every command has a pinned schema | contract | `scripts/check.sh test -E 'test(=contract::schema_files_match_their_pins)'` | ✅ tests/harness/contract.rs from 05-02 | ⬜ pending |
| 05-09-01 | 09 | 7 | all phase reqs (gate) | — | no module-level dead-code allowance left | gate | `scripts/check.sh all` after the plan's rg gates | ✅ | ⬜ pending |
| 05-09-02 | 09 | 7 | all phase reqs (gate) | T-05-29, T-05-30 | mutation pass tied to its head; container gate on the final tip | gate | cargo mutants pass, `scripts/check.sh coverage`, `scripts/check.sh deps`, `scripts/check-in-container.sh all` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

No separate Wave 0: every plan is TDD, and each new test file is written in the RED commit of the task that needs it, before the code it tests. The framework (nextest through `scripts/check.sh`) and the harness (`tests/harness/check.rs`, `registry.rs`, `scenario.rs`) already exist.

Created in RED steps: `src/exit.rs` tests (05-01), `src/json.rs` tests and `tests/harness/contract.rs` (05-02), `src/git/trees.rs` and `src/git/staleness.rs` tests (05-03), `src/check/` tests (05-01, 05-04, 05-06, 05-07), `tests/if02_scan.rs` (05-05), `docs/json/check.v1.schema.json` (05-08).

---

## Manual-Only Verifications

All phase behaviors have automated verification.

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies (18 of 18 tasks)
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references (no plan has a MISSING verify)
- [x] No watch-mode flags
- [ ] Feedback latency < 05s (not measured)
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** {pending / approved YYYY-MM-DD}
