---
phase: 05-divergence-check-and-command-contract
plan: 07
subsystem: check
tags: [rust, git, reflog, fetch-head, staleness, json]
requires:
  - phase: 05-03
    provides: Repo::ref_moved_at and Repo::fetched_at (Result<Option<i64>, Error>)
  - phase: 05-06
    provides: Report.hooks and the JSON key order through hooks
provides:
  - check::staleness (Staleness, TimeRead, Source, time_of, read, applies, age_text)
  - Report.staleness; the staleness diagnostic line and the staleness JSON object before hooks
  - scenario check::remote_unreachable (Parity, rust_enabled)
  - five contract tests for the staleness object
affects: [05-08, 05-09, check, json-contract]
actuals:
  tokens: 9544
  tasks: 2
  commits: 4
plan_head_before: 0d76e9f0954d6bdb43c33c2fe79cc0bcb8d2556d
plan_head_after: 5a6c2993f1511f53c2d25492641d48f987ba2508
tech-stack:
  added: []
  patterns:
    - "A read that must never fail the run goes through one pure function (time_of) that maps Err to a null value with its reason"
key-files:
  created:
    - src/check/staleness.rs
  modified:
    - src/check/mod.rs
    - src/check/report.rs
    - src/git/mod.rs
    - src/git/staleness.rs
    - tests/harness/check.rs
    - tests/harness/contract.rs
key-decisions:
  - "run reads the clock once through git::epoch_seconds (made pub(crate)), so now and FETCH_HEAD mtimes share one conversion"
  - "staleness::read owns the applies branch: a local integration branch returns Staleness::unread and runs neither read"
  - "The staleness line sits after the not-configured and hooks notices and before the problems header"
patterns-established:
  - "Staleness never reaches ExitClass: staleness.rs names no exit item, and the exit class still comes from problems alone"
requirements-completed: [DIV-04]
coverage:
  - id: D1
    description: "Staleness decided and rendered from two optional times and one clock reading (age_text, applies, time_of, line, JSON)"
    requirement: DIV-04
    verification:
      - kind: unit
        ref: "src/check/staleness.rs#check::staleness::tests::age_text_at_each_boundary"
        status: pass
      - kind: unit
        ref: "src/check/staleness.rs#check::staleness::tests::only_a_remote_tracking_ref_is_fetched"
        status: pass
      - kind: unit
        ref: "src/check/staleness.rs#check::staleness::tests::a_failed_read_is_a_null_time_with_its_reason"
        status: pass
      - kind: unit
        ref: "src/check/report.rs#check::report::tests::staleness_line_names_both_times"
        status: pass
      - kind: unit
        ref: "src/check/report.rs#check::report::tests::staleness_line_says_what_is_missing"
        status: pass
      - kind: unit
        ref: "src/check/report.rs#check::report::tests::local_branch_line_says_staleness_does_not_apply"
        status: pass
      - kind: unit
        ref: "src/check/report.rs#check::report::tests::staleness_json_reports_a_future_time_as_a_negative_age"
        status: pass
      - kind: unit
        ref: "src/check/report.rs#check::report::tests::staleness_never_changes_the_verdict"
        status: pass
    human_judgment: false
  - id: D2
    description: "wsx check reads both times from git offline and reports them; the remote unreachable still exits 0 (success criterion 1, offline half)"
    requirement: DIV-04
    verification:
      - kind: integration
        ref: "tests/harness/check.rs#check::remote_unreachable::rust"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::remote_unreachable::oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/contract.rs#contract::check_json_staleness_after_a_clone"
        status: pass
      - kind: integration
        ref: "tests/harness/contract.rs#contract::check_json_staleness_after_a_fetch"
        status: pass
      - kind: integration
        ref: "tests/harness/contract.rs#contract::check_json_staleness_does_not_apply_to_a_local_branch"
        status: pass
      - kind: integration
        ref: "tests/harness/contract.rs#contract::check_json_staleness_with_the_remote_unreachable"
        status: pass
      - kind: integration
        ref: "tests/harness/contract.rs#contract::check_json_staleness_read_failure_keeps_the_exit_code"
        status: pass
    human_judgment: false
  - id: D3
    description: "No production code in src/ names a network git subcommand"
    verification:
      - kind: other
        ref: "plan Task 2 source scan (awk cut + rg network literals), with the planted late/early controls"
        status: pass
    human_judgment: false
duration: 13min
completed: 2026-11-18
status: complete
---

# Phase 5 Plan 07: Integration ref staleness Summary

**`wsx check` reports when the integration ref last moved (its reflog) and when any worktree last fetched (newest FETCH_HEAD), offline, as a stderr line and a `staleness` JSON object, and no staleness value or read error ever changes the exit code**

## Performance

- **Duration:** 13 min
- **Started:** 2026-11-18T21:31:09Z
- **Completed:** 2026-11-18T21:44:16Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- `src/check/staleness.rs`: `TimeRead { NotRead, At, Unknown }`, `Staleness`, `time_of` (Err and absent become `Unknown` with the contract's reason), `applies` (`refs/remotes/` only), `age_text`, `read`.
- Every run prints `wsx check: integration ref R: last moved <age> ago (reflog); last fetch <age> ago (FETCH_HEAD)` (or each missing time's reason, or the local-branch line) before the problems header.
- JSON payload gains `staleness` between `integration` and `hooks`, with the eight contract fields; a future time gives a negative age.
- Scenario `remote_unreachable` (18 scenarios registered `rust_enabled`): with origin's URL pointing at a missing path, both sides exit 0 and the Rust side prints both times.

## Task Commits

1. **Task 1: Staleness decided and rendered, pure** - `196a78a` (test, RED), `67dac88` (feat, GREEN)
2. **Task 2: Times read from git, unreachable-remote scenario, JSON forms** - `696af8f` (test, RED), `5a6c299` (feat, GREEN)

## TDD Gate Compliance

- **Task 1 RED:** `check::staleness::tests::age_text_at_each_boundary` failed on its planned `assert_eq!` (left every text empty from the stub, right the eight boundary texts). Classifier: `RED_EVIDENCE_OK` (target/gsd/05-07-red-evidence.json, JUnit report). Semantic assessment: the target compiled and ran; the failure is the planned assertion, not a build, load or fixture fault.
- **Task 2 RED:** `contract::check_json_staleness_after_a_fetch` failed on `assert_eq!` for `ref_moved_at` (left null, right the reflog's unix time 1795037960) after its fixture built and `git reflog exists` exited 0. Classifier: `RED_EVIDENCE_OK` (target/gsd/05-07-red-evidence-2.json). Semantic assessment: the planned failure, with run still leaving both times `NotRead`. `check::remote_unreachable::rust` failed the same way (stderr `last moved not read; last fetch not read`).
- RED precedes GREEN by ancestry in both tasks (196a78a then 67dac88; 696af8f then 5a6c299).

## Files Created/Modified

- `src/check/staleness.rs` - the staleness model, `read` and the pure helpers with their tests
- `src/check/mod.rs` - `Report.staleness`; `run` reads the clock once and calls `staleness::read` after resolve and the hooks reads
- `src/check/report.rs` - the staleness line, the staleness JSON object, five tests; `line_of` now takes the last diagnostic and asserts the header before it
- `src/git/staleness.rs`, `src/git/mod.rs` - `epoch_seconds` made `pub(crate)` and re-exported
- `tests/harness/check.rs` - `fetch_moved_develop`, `lose_origin`, scenario `remote_unreachable`
- `tests/harness/contract.rs` - five staleness contract tests

## Decisions Made

- The clock goes through the existing `git::epoch_seconds` (whole seconds, truncated toward zero) instead of a second conversion.
- `staleness::read` decides `applies` itself and returns `Staleness::unread` for a local branch, so `run` has one call.
- The `TimeRead::NotRead` arm of the line prints `<what> not read`; after Task 2 it is reachable only when staleness does not apply, which prints the local-branch line instead.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `epoch_seconds` exposed from `src/git`**
- **Found during:** Task 1
- **Issue:** `run` needs the clock as whole unix seconds; the conversion already lived privately in `src/git/staleness.rs`, a file outside the plan's list.
- **Fix:** `pub(crate) fn epoch_seconds` and `pub(crate) use staleness::epoch_seconds` in `src/git/mod.rs`; no behaviour change.
- **Files modified:** src/git/staleness.rs, src/git/mod.rs
- **Committed in:** 196a78a

**2. [Rule 3 - Blocking] Interim dead-code expectation in Task 1**
- **Found during:** Task 1 GREEN
- **Issue:** with the times left `NotRead` until Task 2, `time_of` and `Source` had no non-test caller and `clippy -D warnings` would fail the gate.
- **Fix:** a module-level `cfg_attr(not(test), expect(dead_code, ...))` in 67dac88, removed in 5a6c299 when `read` gained its caller.
- **Committed in:** 67dac88, 5a6c299

**3. [Rule 1 - Test setup] The local-branch contract test**
- **Found during:** Task 2 RED
- **Issue:** the preset already has a local `develop` (`git branch develop` failed), and removing `origin` alone leaves `wsx.baseRemote` and `wsx.pushRemote` naming it, which the model refuses (exit 4).
- **Fix:** the test asserts the preset's `refs/heads/develop`, removes `origin`, and unsets both keys; it then resolves to `refs/heads/develop` (asserted from `integration.ref`).
- **Committed in:** 696af8f

---

**Total deviations:** 3 auto-fixed (2 blocking, 1 test setup). **Impact:** none on the contract; the local-branch test passed already at Task 2 RED, since Task 1 rendered that case.

## Issues Encountered

None.

## Verification

- All eight Task 1 tests, the two scenario sides and the five contract tests pass, each run as its own `--no-tests=fail` filter.
- Network-subcommand source scan prints nothing; the planted late and early controls are both found (`PLANTED_BOTH_FOUND`).
- `rg -c 'rust_enabled;' tests/harness/check.rs` prints 18; `src/check/staleness.rs` names no `ExitClass` or `exit::`; `staleness::read` (holding the `ref_moved_at` call) is called in `run` at line 129, after `model::resolve` at line 118.
- `scripts/check.sh all` exited 0 with last line `==> check.sh: all OK` after each GREEN commit (target/gsd/05-07-all.log).
- Evidence scope: the clone, fetch, local-branch and unreachable-remote shapes and one failed read (a stray file under worktrees/) end to end; a failing reflog child is pinned only by the pure `time_of` test.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for 05-08.

---
*Phase: 05-divergence-check-and-command-contract*
*Completed: 2026-11-18*

## Self-Check: PASSED
