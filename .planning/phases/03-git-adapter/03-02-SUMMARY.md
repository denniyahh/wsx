---
phase: 03-git-adapter
plan: 02
subsystem: testing
tags: [harness, reftable, regression, oracle, git-refs-migrate]

requires:
  - phase: 02-oracle-harness
    provides: scenario rules (Class::Regression, OracleSide::Recorded, guard controls), the check scenario helpers and the scenarios! registration
provides:
  - check::to_reftable fixture step that migrates work to reftable and proves the backend with git rev-parse --show-ref-format
  - check::reftable_refused regression scenario with its oracle side recorded from a real harness run, both guard controls, Rust side disabled until phase 5
  - STATE.md Blockers/Concerns line holding phase 5 to enable the scenario's Rust side
affects: [03-06, phase-5]

actuals:
  tokens: 1210      # chars/4 over the realized diff (4840 chars: check.rs + one STATE.md line)
  tasks: 2
  commits: 2
plan_head_before: ca7b1b4530b8bd7de3df6ec05e0bde99b826f879
plan_head_after: 5d34bd6c5b4dcbdc4af59a12143e24f5e729a140

tech-stack:
  added: []
  patterns:
    - "A fixture step that changes a repository property checks the property afterwards and errors naming what it saw (its own negative control)"

key-files:
  created: []
  modified:
    - tests/harness/check.rs
    - .planning/STATE.md

key-decisions:
  - "03-02: the oracle's record on reftable is verdict(0), observed through the harness (exit 0, 'every difference from origin/develop is declared.', refs unchanged); it matches RESEARCH's scratch prediction"

patterns-established: []

requirements-completed: [GIT-03]

coverage:
  - id: D1
    description: "check::reftable_refused::oracle runs the vendored check on the devflow preset migrated to reftable and matches the recorded outcome (exit 0, refs unchanged, no new commit); the known-positive (undeclared notes/todo.md, exit 1, pinned stderr) and clean files-backend (exit 0) controls run in the same test"
    requirement: "GIT-03"
    verification:
      - kind: integration
        ref: "scripts/check.sh test -E 'test(=check::reftable_refused::oracle)'"
        status: pass
      - kind: integration
        ref: "negative control: record changed to verdict(1) + oracle_stderr(b\"reftable\") fails with 'exit code expected: 1 got: 0' (reverted)"
        status: pass
    human_judgment: false
  - id: D2
    description: "to_reftable proves the subject is reftable: without the migrate call the step fails with 'work reports ref format \"files\\n\", not reftable'"
    requirement: "GIT-03"
    verification:
      - kind: integration
        ref: "negative control: migrate line deleted, scripts/check.sh test -E 'test(=check::reftable_refused::oracle)' rc 100 naming files (reverted)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The Rust side is registered disabled: selecting only check::reftable_refused::rust exits 4 with 0 tests run, nextest lists it as ignored under wsx::harness, and registry::rust_sides_not_enabled names it"
    requirement: "GIT-03"
    verification:
      - kind: integration
        ref: "scripts/check.sh test -E 'test(=check::reftable_refused::rust)' (rc 4)"
        status: pass
      - kind: integration
        ref: "scripts/check.sh test -E 'test(=registry::rust_sides_not_enabled)'"
        status: pass
      - kind: integration
        ref: "scripts/check.sh test -E 'test(=registry::every_scenario_is_well_formed)'"
        status: pass
    human_judgment: false
  - id: D4
    description: "STATE.md Blockers/Concerns holds the phase 5 obligation for check::reftable_refused, written by gsd state.add-blocker"
    verification:
      - kind: other
        ref: "awk Blockers/Concerns section | rg -qF 'check::reftable_refused' && rg -qF 'Phase 12: MIG-03' .planning/STATE.md"
        status: pass
    human_judgment: false

duration: 2min
completed: 2026-10-27
status: complete
---

# Phase 3 Plan 02: Reftable regression scenario Summary

**The harness now runs the vendored divergence check on a reftable repository on every gate: the oracle passes it (exit 0, recorded from a real run), wsx is expected to refuse it (placeholder exit 1 until IF-01), and the Rust side stays disabled until phase 5, which STATE.md now holds it to.**

## Performance

- **Duration:** 2 min, measured from the start mark taken after context loading (2026-10-27T01:49:58Z to 01:52:01Z)
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `to_reftable` runs `git refs migrate --ref-format=reftable` in `work`, then fails unless `git rev-parse --show-ref-format` prints exactly `reftable\n`.
- `reftable_refused`: subject `check_in_work(RepoConfig::devflow(), to_reftable)`, expected `verdict(1)`, `Class::Regression { reason: REFTABLE_REASON }`, oracle `Recorded(verdict(0))`. The known-positive control is `self_test`'s (files backend, undeclared `notes/todo.md`, exit 1, `UNDECLARED_NOTE`). The clean control is the unmodified files-backend preset (exit 0).
- Registered as `reftable_refused: oracle, rust = "Rust side not enabled until phase 5";`. The module doc names plans 02-07, 02-13 and 03-02.
- RESEARCH assumption A3 is now settled by a run: on reftable the observer listed all four refs (`refs/heads/develop`, `refs/heads/workspace/dev`, `refs/remotes/origin/HEAD`, `refs/remotes/origin/develop`) through `git for-each-ref`. The listing was not empty.

## Task Commits

1. **Task 1: reftable regression scenario with the oracle side recorded from a real run**: `d07d44c` (test)
2. **Task 2: phase 5 obligation in STATE.md through GSD**: `5d34bd6` (docs)

## Files Created/Modified

- `tests/harness/check.rs`: `to_reftable`, `REFTABLE_REASON`, `reftable_refused`, its registration line, and the module doc line
- `.planning/STATE.md`: one Blockers/Concerns line, plus the `last_updated`/`state_head` frontmatter refresh that `state.add-blocker` writes

## Decisions Made

The record was not designed. The harness observed `verdict(0)`, the same as RESEARCH's scratch prediction, so the plan's step 4 transcription branch did not trigger.

## Deviations from Plan

None. The plan was executed as written. Two notes:
- `check.rs` was formatted with `rustfmt --edition 2024` directly, because `cargo fmt` does not reach scenario modules (the phase 2 decision).
- My first negative control (record `verdict(1)` with no stderr pin) was rejected by `validate` before the oracle ran, because a non-zero record must pin its stderr. It did not test the run. The control was redone with `.oracle_stderr(b"reftable")` and then failed on the run itself (`exit code expected: 1 got: 0`).

## Verification Results (limits stated)

- `scripts/check.sh all`: OK, `191 tests run: 191 passed, 42 skipped`. The baseline at 03-01 was 190 and 41: +1 oracle test, +1 ignored Rust side.
- Named tests, each run on its own: `check::reftable_refused::oracle` (`1 test run: 1 passed`), `registry::every_scenario_is_well_formed` and `registry::rust_sides_not_enabled`, whose output lists `check::reftable_refused (Rust side not enabled until phase 5)`. `check::reftable_refused::rust` alone exits 4 with `0 tests run`. `cargo nextest list --run-ignored only` shows it under `wsx::harness`.
- Acceptance greps pass. At HEAD~2 the `Class::Regression` grep exits 1. No `(03-02)` commit touches `tests/oracle`.
- Negative controls, both reverted (the final diff was checked): a wrong record fails on the oracle run, and the subject without the migration fails in the fixture step, naming `files`.
- **What this does not establish:** the scenario pins only what the oracle does on reftable. It says nothing about wsx refusing reftable: the Rust side never ran, and the expected exit 1 is a placeholder. The unit-level refusal belongs to 03-06. The STATE.md line names the test `git::repo::tests::reftable_repo_is_refused` as the plan worded it, and that test does not exist yet (checked: no match in `src/`). 03-06 must keep that name, or the line points at nothing. The run covers one reftable repository on git 2.56.0. Converting a clone with `git refs migrate` is one way to reach reftable; `git init --ref-format=reftable` and `git clone --ref-format=reftable` are not exercised.

## Known Stubs

None. The placeholder exit 1 is deliberate and tracked by the STATE.md blocker (RESEARCH Open Question 3).

## Threat Flags

None. T-03-06 (`to_reftable`'s format check) and T-03-07 (the record judged by `oracle_target` on every gate) are implemented, and each was shown to bite by a negative control. No install was run (T-03-SC).

## Cleanup

Removed: the blocker scratch directory under my session scratchpad and the `check.rs` backup under `target/`. Kept, all gitignored under `target/`: the run logs `target/gsd-03-02-*.log` and `target/gsd-03-02-start` (deleted after this SUMMARY's commit), and `target/gsd-tools/` (pinned tools for later plans). No background process was started.

## Next Phase Readiness

Ready for 03-03. Phase 5 inherits the STATE.md obligation.

## Self-Check: PASSED

- FOUND: tests/harness/check.rs contains `reftable_refused: oracle, rust = "Rust side not enabled until phase 5";`
- FOUND commits on HEAD: d07d44c, 5d34bd6 (`git rev-list --count ca7b1b4..HEAD` = 2 before this SUMMARY's commit)
