---
phase: 03-git-adapter
plan: 06
subsystem: git
tags: [gix, reftable, refusal, extensions.refStorage, tdd]
status: complete

requires:
  - phase: 03-git-adapter
    provides: "03-01 Repo::discover, open_gix, from_gix, resolve, Error::Reftable and its message; 03-04 the single sanctioned gix entry point"
provides:
  - "refuse_ref_storage: any extensions.refStorage other than the exact bytes `files` is Error::Reftable, checked in discover after open_gix and before from_gix"
  - "Repo::resolve backstop: a gix error classified unsupported (is_unsupported()) is Error::Reftable, not Error::Gix"
  - "git::repo::tests::reftable_repo_is_refused, the test the phase-5 blocker in STATE.md cites"
affects: [03-09, 03-10, phase-5 check::reftable_refused]

actuals:
  tokens: 2844
  tasks: 2
  commits: 4
plan_head_before: 4aed25c906207c3d21a1155d75929199fe321b12
plan_head_after: 941d00900b91fe0686b2d5f669e63b6ae8f548e6

tech-stack:
  added: []
  patterns:
    - "Refusals read configuration before any ref; gix's error class is the backstop, not message text"
    - "Twin fixtures: the same history in two backends, equal HEAD asserted by git, so the control differs only in the variable under test"

key-files:
  created: []
  modified:
    - src/git/repo.rs

key-decisions:
  - "03-06: the backstop maps on gix_error's is_unsupported(), observed true for the reftable HEAD decode error and false for a missing branch; no message-text match was needed"
  - "03-06: the backstop's refusal path uses Repo's already-canonical worktree (or git dir), so it needs no second canonicalize"

requirements-completed: [GIT-03]

duration: 3 min
completed: 2026-10-27
---

# Phase 3 Plan 06: Reftable refusal Summary

**A repository whose `extensions.refStorage` is anything but exactly `files` is refused as `Error::Reftable` before any ref is read, and gix's unsupported-backend error on ref decode maps to the same refusal as a backstop, both pinned by controls that open.**

## Performance

- **Duration:** 3 min (measured from the plan-start timestamp to SUMMARY)
- **Completed:** 2026-10-27
- **Tasks:** 2 of 2
- **Files modified:** 1 (`src/git/repo.rs`)

## Accomplishments

- `discover` now calls `refuse_ref_storage` between `open_gix` and `from_gix`. Absent or exactly `files` passes; any other value (including `Files`) is `Error::Reftable { path, format }`, path the canonical worktree or, for a bare repository, the canonical git dir.
- `resolve` returns `Error::Reftable` when the gix error's `is_unsupported()` is true, with `format` the configured `extensions.refStorage` or `unsupported` when none is set; everything else stays `Error::Gix`.
- Eight tests and the `twins` helper (plus `git_line`, `ref_storage_repo`, `refused_format`, `open_unchecked`) in `repo.rs`'s test module.

## Task Commits

1. **Task 1 RED:** `d3d544e` test(03-06): add failing reftable refusal tests
2. **Task 1 GREEN:** `9ddef1b` feat(03-06): refuse reftable repositories before reading refs
3. **Task 2 RED:** `3087515` test(03-06): add failing unsupported-backend mapping tests
4. **Task 2 GREEN:** `941d009` feat(03-06): map gix's unsupported ref backend to the reftable refusal

No refactor commits.

## TDD Gate Compliance

- **Task 1 RED:** `RED_EVIDENCE_OK` for `git::repo::tests::reftable_repo_is_refused` (record `target/gsd/03-06-red-evidence.json`, JUnit, exit 100). Semantic assessment: the target ran (1 run, 265 skipped) and failed on the planned refusal assertion: discover returned `Ok` and opened `<table>/.git`. Before that call, `twins` had built both repositories, asserted equal `HEAD`, migrated `table` with `git refs migrate --ref-format=reftable` and asserted `git rev-parse --show-ref-format` printed `reftable` (and `files` for the control), so this was not a fixture or build fault. In the same RED state, `empty_reftable_repo_is_refused` and `miscased_ref_storage_is_refused_as_git_refuses_it` failed the same way; `files_backend_twin_opens` and `explicit_files_ref_storage_opens` passed, as controls must.
- **Task 2 RED:** `RED_EVIDENCE_OK` for `git::repo::tests::reftable_ref_decode_maps_to_the_refusal` (record `target/gsd/03-06-red-evidence-2.json`, JUnit, exit 100). Semantic assessment: `open_gix` then `from_gix` succeeded on the reftable twin, and the test failed on the mapping assertion with `Error::Gix`: `resolve HEAD: couldn't parse revision, input="HEAD"`. The two controls passed in the same state. The RED message shows only the top-level error, not the cause chain, so it did not settle whether the Unsupported class survived wrapping.
- **Which mapping GREEN uses:** `is_unsupported()`. A temporary probe (reverted before commit, not in history) printed `unsupported=true` for the reftable `HEAD` and `unsupported=false` for `refs/heads/no-such-branch`. No message-text fallback was written; `rg -v '^\s*//' src/git/repo.rs | rg -q 'unsupported storage backend'` exits 1.

## Verification

- Task 1 verify chain (five named tests, one command each, `--no-tests=fail`): exit 0. Task 2 verify chain (three named tests): exit 0.
- `scripts/check.sh all` exited 0 with final line `==> check.sh: all OK` before each GREEN commit: 224 passed / 42 skipped after Task 1, 227 passed / 42 skipped after Task 2. Both GREEN commits were gated on that exit code.
- Acceptance greps: `"extensions.refStorage"` and `fn refuse_ref_storage` present (exit 0); `"--ref-format=reftable"` present (exit 0); `is_unsupported()` present outside comments (exit 0).
- Negative control, byte-exact comparison: replacing `value == "files"` with `eq_ignore_ascii_case(b"files")` made `miscased_ref_storage_is_refused_as_git_refuses_it` fail ("miscased Files: expected a Reftable refusal, but it opened"); the file was restored from a copy and the restore checked before the gate.
- All gates ran with `PATH=<worktree>/target/gsd-tools:$PATH` (pinned cargo-nextest 0.9.148) per the project note.

## What this does not establish

- `extensions.refStorage` is read from gix's merged configuration snapshot, so a global or system value would also be read. That fails in the safe direction (a files repository refused), but git reads extensions only from the repository's own config, so wsx and git could disagree there. Not exercised by a test.
- The backstop is reachable only when the configuration check is bypassed; in this phase only `open_unchecked` in a test does that. Its specificity rests on two cases (reftable `HEAD`: mapped; missing branch: not mapped), not on a survey of every gix error that carries the Unsupported class. Another unsupported-class error from `rev_parse_single` would also be reported as `Error::Reftable` with `format` taken from config.
- No test opens a bare repository, so the bare-repository path (canonical git dir) of both refusals is untested.
- The `git rev-parse` exit 128 and `invalid value for 'extensions.refstorage'` assertion is against the host's git 2.56.0; a git that changes that wording fails the test, not wsx.

## Deviations from Plan

None in production code or test names. Two additions inside the plan's scope:
- `twins` also asserts the files twin reports `files` from `git rev-parse --show-ref-format`, so the control is shown to differ from the reftable twin only in the backend.
- The backstop is a separate private method, `Repo::unsupported_backend`, and a shared `ref_storage` reader serves both checks; the plan named only `refuse_ref_storage`.

## Issues Encountered

- `cargo fmt` reflowed two lines of the first RED draft; caught by `scripts/check.sh fmt` before the RED commit and fixed with `cargo fmt`.

## Known Stubs

None. The `unsupported` fallback `format` is a deliberate value for a backstop refusal with no configured backend, not a placeholder.

## Cleanup

- Test scratch repositories live under `target/tmp/git-unit-*` as `TempDir`s and are removed on drop; `ls target/tmp | rg -c git-unit-` found none after the runs.
- Removed the two `repo.rs` backup copies from the session scratchpad.
- Kept: `target/gsd/03-06-red-evidence.json` and `-2.json` (the plan requires them; gitignored).
- `.planning/state.json` was untracked before this plan started; left untouched.

## Requirement tracking

- `requirements.mark-complete GIT-03` checked GIT-03 off in `REQUIREMENTS.md`; I reverted that file before committing. 03-10 still carries GIT-03, its `regression:` scenario's Rust side is enabled only in phase 5, and no earlier phase-3 plan has checked off its GIT requirement. This plan delivers the unit-level refusal toward GIT-03, not all of it. Logged in the GSD manual-fix incident log.

## Next Phase Readiness

- The phase-5 blocker's cited test, `git::repo::tests::reftable_repo_is_refused`, now exists and passes.
- 03-09 inserts the trust check and 03-10 the ceiling check before `refuse_ref_storage` in `discover`, per the plan's key link.

## Self-Check: PASSED

- FOUND: src/git/repo.rs, target/gsd/03-06-red-evidence.json, target/gsd/03-06-red-evidence-2.json
- FOUND commits (ancestors of HEAD): d3d544e, 9ddef1b, 3087515, 941d009
