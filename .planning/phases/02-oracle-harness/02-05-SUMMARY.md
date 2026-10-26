---
phase: 02-oracle-harness
plan: 05
subsystem: testing
tags: [rust, nextest, harness, scenario-rules, oracle]

requires:
  - phase: 02-oracle-harness
    provides: "02-02 runner (run_case, Entry, scenarios! macro, compare/observed), 02-03 registry header and harness_modules!"
provides:
  - "Class, OracleSide, Role, CaseKind, Kind, CaseRef, Control, Rejection, CaseFailure in tests/harness/scenario.rs"
  - "validate(kind, scenario) enforcing success criterion 4 and HARN-05, reporting every rejection"
  - "judge(side, case, run): a fixed: case fails on the oracle when its run meets the expected outcome, whatever its record holds"
  - "oracle_target, is_guard, try_run_with_env (validates, runs subject and each control, returns the report)"
  - "scenarios! arm `name: rust_only, rust = \"<reason>\";` generating only the ignored rust() test"
  - "registry::disabled_report, registry::rust_sides_not_enabled, registry::every_scenario_is_well_formed"
  - ".config/nextest.toml success-output = \"final\" override for registry::rust_sides_not_enabled"
affects: [02-07, 02-08, 02-10, 02-11, 02-12, 02-13, 02-14, 02-15]

actuals:
  tokens: 11946
  tasks: 2
  commits: 5
plan_head_before: 5a9809d6d74f383fe08e3a28918ca08aff5368e2
plan_head_after: 54a24e370a178f5d3cb4671fa5a3f92fccdfac71

tech-stack:
  added: []
  patterns:
    - "Scenario rules as a Rejection enum with plain-words Display; tests assert the exact Vec<Rejection>"
    - "fixed: cases judged at run time against the expected outcome before the record (encoding-independent)"
    - "Every generated test validates through try_run_with_env before any fixture is built"

key-files:
  created: []
  modified:
    - tests/harness/scenario.rs
    - tests/harness/registry.rs
    - .config/nextest.toml

key-decisions:
  - "A Rust-only case 'claims an oracle side' unless its oracle side is OracleSide::NotImplemented (matches 02-10/12/13/14, which set NotImplemented on every case of a rust_only scenario)"
  - "Control direction is judged by expected exit only: a known-positive control must expect a non-zero exit, a clean control exit 0"
  - "A regression: refusal is exit != 0, RefsExpect::Unchanged (literally) and no new commits; an Exactly list of all-Unchanged tips is rejected"
  - "Rejections carry CaseRef::{Subject, Control(n)} with n counted from 1; Harn06WithoutControl carries the CaseKind; OracleKindWithoutOracleSide carries the why"
  - "validate is called inside try_run_with_env, so every generated test (oracle and Rust side) validates before it runs"

patterns-established:
  - "Report entry format: `<module>::<name>, <case><class>: <failure>`, case = `subject` or `<role> control <n>`, class = ` (fixed: <defect>)` / ` (regression: <reason>)` / empty"
  - "Rust-only scenario: register `name: rust_only, rust = \"...\";` and give every case `.oracle(OracleSide::NotImplemented { why })`"

requirements-completed: [HARN-01, HARN-03, HARN-05, HARN-06]

coverage:
  - id: D1
    description: "validate rejects each malformed case by its exact variant and accepts valid counterparts"
    requirement: HARN-05
    verification:
      - kind: unit
        ref: "tests/harness/scenario.rs#scenario::rejects_* (13 tests), accepts_every_harn06_kind_with_a_counterpart, accepts_well_formed_parity_scenario, accepts_guard_with_both_controls_in_either_order, guard_with_no_controls_reports_both_missing_roles"
        status: pass
    human_judgment: false
  - id: D2
    description: "A fixed: case whose oracle run meets its expected outcome fails at run time whatever its record's encoding; a really differing one passes; record drift fails"
    requirement: HARN-01
    verification:
      - kind: integration
        ref: "tests/harness/scenario.rs#scenario::fixed_record_reordering_expected_refs_fails_on_oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/scenario.rs#scenario::fixed_record_using_unchanged_for_listed_refs_fails_on_oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/scenario.rs#scenario::fixed_case_differing_on_oracle_passes"
        status: pass
      - kind: integration
        ref: "tests/harness/scenario.rs#scenario::fixed_record_drift_fails_on_oracle"
        status: pass
      - kind: unit
        ref: "tests/harness/scenario.rs#scenario::fixed_case_is_judged_against_its_recorded_oracle_outcome"
        status: pass
    human_judgment: false
  - id: D3
    description: "The runner runs each control on the same side and names a failing control by role and position"
    requirement: HARN-05
    verification:
      - kind: integration
        ref: "tests/harness/scenario.rs#scenario::runner_runs_each_control_and_names_the_failing_one"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every registered scenario validates; every full run lists each disabled Rust side by name"
    requirement: HARN-06
    verification:
      - kind: unit
        ref: "tests/harness/registry.rs#registry::report_omits_enabled_rust_sides"
        status: pass
      - kind: unit
        ref: "tests/harness/registry.rs#registry::rust_sides_not_enabled"
        status: pass
      - kind: unit
        ref: "tests/harness/registry.rs#registry::every_scenario_is_well_formed"
        status: pass
      - kind: other
        ref: "scripts/check.sh test (full run output contains the report block)"
        status: pass
    human_judgment: false

duration: 11min
completed: 2026-10-26
status: complete
---

# Phase 2 Plan 05: Scenario Rules and Disabled-Side Report Summary

**`validate` enforces success criterion 4 and HARN-05 through a 12-variant `Rejection` enum. `judge` fails any `fixed:` case whose oracle run meets its expectation, however its record is encoded. The runner validates first, then runs the subject and every control. Every full nextest run now prints `Rust side not enabled (N):` with one line per disabled scenario.**

## Performance

- **Duration:** about 11 min
- **Started:** 2026-10-26T17:24:53Z
- **Completed:** 2026-10-26T17:36:18Z
- **Tasks:** 2 (5 commits)
- **Files modified:** 3

## Accomplishments

- **Scenario rules:** `Class`, `OracleSide`, `Role`, `CaseKind`, `Kind`, `Rejection` and `CaseFailure` exist. Every rule has a test that asserts the exact `Vec<Rejection>`.
- **Run-time `fixed:` check:** `judge` compares a `fixed:` case's oracle run with its expected outcome before its record. It returns `FixedPassesOnOracle` when that comparison is clean. Two encodings that derived equality misses are proven to fail on the real oracle: reordered refs, and `Unchanged` against a listed ref set.
- **Runner:** `try_run_with_env` validates the scenario, then runs the subject and each control, each in its own TempDir. It returns a report with one entry per failing case, and `run_with_env` panics with that report.
- **Rust-only scenarios:** the macro arm `name: rust_only, rust = "...";` generates only the ignored `rust()` test.
- **Disabled-side report:** `registry::disabled_report`, `rust_sides_not_enabled` and `every_scenario_is_well_formed` exist. A nextest `success-output = "final"` override shows the report in every full run.

## Task Commits

1. **Task 1 RED:** `2396642` `test(02-05): add failing scenario rule tests`
2. **Task 1 GREEN:** `9552f52` `feat(02-05): enforce scenario rules and run guard controls`
3. **Task 2 RED:** `5c646ed` `test(02-05): add failing disabled-side report test`
4. **Task 2 GREEN:** `5829116` `feat(02-05): report disabled Rust sides by name in every run`
5. **Added coverage (deviation 1):** `54a24e3` `test(02-05): cover control runs in the runner report`

## TDD Gate Compliance

| Task | RED | GREEN | RED evidence |
|------|-----|-------|--------------|
| 1 | `2396642` | `9552f52` | `target/gsd/02-05-red-evidence.json`: `RED_EVIDENCE_OK` (reason `target_test_failed`) for `scenario::rejects_fixed_matching_expected_on_oracle`, exit 100 |
| 2 | `5c646ed` | `5829116` | `target/gsd/02-05-red-evidence-2.json`: `RED_EVIDENCE_OK` (reason `target_test_failed`) for `registry::report_omits_enabled_rust_sides`, exit 100 |

**Semantic assessment, Task 1:** the target compiled and ran. It failed on its own `assert_eq` with `left: Ok(())`, `right: Err([FixedRecordMatchesExpected(Subject)])`, because the stub `validate` accepted everything. In the same RED state, every `rejects_*` test and all five oracle and judge tests failed on their assertions. The oracle tests failed with these lefts:
- reordered and unchanged: `Ok(())`
- differing: `Err(..exit code expected 1 got 0..)`
- drift: mismatch `expected: "1"` where `"2"` was required

No fixture or load fault occurred.

**Semantic assessment, Task 2:** the target failed on `assert_eq` with `left: ""` against the one-line report. The stub returned an empty string.

The RED and GREEN commits are in order: `git log BASE..HEAD`, with `BASE = merge-base HEAD workspace/denniyahh`, lists `test`, `feat`, `test`, `feat`, `test`.

## Report Block (from a full `scripts/check.sh test` run)

```
  stdout ───

    running 1 test
    Rust side not enabled (2):
      sync::already_up_to_date (Rust side not enabled until phase 7)
      sync::self_test (Rust side not enabled until phase 7)
    test registry::rust_sides_not_enabled ... ok

     Summary [   3.067s] 94 tests run: 94 passed, 2 skipped
```

**Control without the override:** a scratch `--config-file` holding only the JUnit table, running `-E 'test(=registry::rust_sides_not_enabled)'`, gave nextest exit 0. Its only output was:

```
        PASS [   0.005s] (1/1) wsx::harness registry::rust_sides_not_enabled
     Summary [   0.006s] 1 test run: 1 passed, 95 skipped
```

The report line was absent. The same filter under the default config, with the override, shows the line. So the override, not the filter or the run size, is what makes it visible. No `nextest.*` directory remains under `WSX_SCRATCH`.

## Negative Controls (each a temporary mutation, restored byte-identical from a scratch backup)

| Mutation | Must go red | Result |
|---|---|---|
| `judge` without the `FixedPassesOnOracle` return | `fixed_record_reordering_*`, `fixed_record_using_unchanged_*` | Both exit 100 (`left: Ok(())`). The opposite control `fixed_case_differing_on_oracle_passes` stayed green. |
| Oracle side of a `fixed:` case compared with the expected outcome, never the record | `fixed_record_drift_fails_on_oracle`, `fixed_case_differing_on_oracle_passes` | Both exit 100 (drift saw `expected: "1"`). `fixed_record_reordering_*` stayed green. |
| `validate` skips the clean-control rejection when the known-positive is also missing | `guard_with_no_controls_reports_both_missing_roles` | Exit 100. `rejects_guard_without_clean_control` stayed green. |
| `sync::already_up_to_date` built without an expectation | `sync::already_up_to_date::oracle` (generated test validates first) and `registry::every_scenario_is_well_formed` | Both exit 100, each naming "the subject states no expected outcome". |
| `already_up_to_date` registered `rust_only` | Only `::rust` listed | `cargo nextest list --run-ignored all` showed only `sync::already_up_to_date::rust`; the original lists `::oracle` and `::rust`. The `::oracle` filter gave exit 4, `no tests to run`. |
| `disabled_report` drops its last line | `registry::rust_sides_not_enabled` | Exit 100 (`Rust side not enabled (1):` against `(2):`). |
| `try_run_with_env` runs only the subject | `runner_runs_each_control_and_names_the_failing_one` | Exit 100 ("a scenario whose control fails must fail"). |

## Files Created/Modified

- `tests/harness/scenario.rs`: the rule types, `validate`, `is_guard`, `oracle_target`, `judge` and `try_run_with_env`, the `rust_only` and `@kind` macro arms, and 22 new tests.
- `tests/harness/registry.rs`: `disabled_report`, `rust_sides_not_enabled`, `every_scenario_is_well_formed` and `report_omits_enabled_rust_sides`.
- `.config/nextest.toml`: the `[[profile.default.overrides]]` block for `registry::rust_sides_not_enabled`. Its comment avoids the word "no-tests"; `rg -n 'no-tests'` still prints only lines 8 and 9.

## Decisions Made

These were taken inside the plan's discretion and are recorded for later plans:
- **Rust-only scenarios:** a case in one must have `OracleSide::NotImplemented`. `SameAsExpected` counts as claiming an oracle side. This matches how 02-10, 02-12, 02-13 and 02-14 already write their Rust-only cases.
- **Control direction:** judged by the expected exit only. A known-positive control needs a non-zero exit; a clean control needs exit 0.
- **What counts as a refusal for `regression:`:** a non-zero exit, `RefsExpect::Unchanged` (that variant, not an equivalent listed set) and no new commits.
- **Rejection order:** first the outcome rules for each case (subject, then controls in order), then the guard's missing roles, then the control directions, then the kind rules, then HARN-06. Every rejection is reported, not only the first.
- **`judge` on a case without an expectation:** it panics with "validate rejects a case without an expected outcome before it runs". Through the runner this cannot happen, because `try_run_with_env` validates first.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2: missing verification] No test exercised the control loop**
- **Found during:** writing this SUMMARY.
- **Issue:** the plan's tests never run a scenario with controls through `try_run_with_env`. The control loop and its `<role> control <n>` label had no runtime coverage.
- **Fix:** added `scenario::runner_runs_each_control_and_names_the_failing_one`, a real-oracle run with a failing counterpart. The report must start with `scenario::with_failing_control, counterpart control 1: ...` and must not mention the passing subject. A mutation that runs only the subject makes it fail.
- **Files modified:** `tests/harness/scenario.rs`
- **Committed in:** `54a24e3`

**2. [Scope: placement] `validate` is called inside `try_run_with_env`, not in each macro arm**
- Every generated test still validates before it runs, on both sides. The rejections come out as `<module>::<name> breaks the scenario rules:` followed by one `  - <rejection>` line each.

**3. [Scope: additive surface]**
- New types: `CaseRef` and `Control`.
- New `Display` impls: `Role`, `CaseKind`, `Rejection` and `CaseFailure`.
- New private helpers: `cases_of`, `is_refusal`, `outcome_rules`, `guard_rules`, `control_direction`, `kind_rule`, `expected_of`, `case_name`, `class_label` and `failure_entry`.
- Two tests carry an extra assertion for the opposite result:
  - `rejects_rust_only_kind_with_an_oracle_side` also checks that a `NotImplemented` case is accepted.
  - `accepts_guard_with_both_controls_in_either_order` checks both orders.

---

**Total deviations:** 1 auto-fixed (missing verification), plus one placement choice and additive surface.
**Impact on plan:** every planned name, signature and test exists as specified. The as-built signatures from 02-02 are kept: `run_case` takes the entry, and fixture steps return `Result`.

## What These Checks Do Not Establish

- **Synthetic scenarios only, so far:** the rules are proven on synthetic scenarios and on the two 02-02 sync scenarios. The guard and HARN-06 rules first meet real scenarios in 02-07 and in 02-10 to 02-14. No registered scenario has controls yet; only the synthetic control test runs that path.
- **Unexercised macro arms:** the `rust_enabled` arm from 02-02 still has no user. The `rust_only` arm was checked only by a temporary expansion, which was not committed; its first real use is in 02-10 and 02-12.
- **What the run-time `fixed:` check proves:** that the oracle run differs from the expectation in at least one field `compare` reports. It does not prove that the difference is the one the defect text names. 02-07's recording rule covers that.
- **Literal copies only, before a fixture is built:** `FixedRecordMatchesExpected` uses derived equality, so it catches only a literal copy. Other encodings are caught only when the scenario runs on the oracle.
- **Control direction looks at exit codes only:** it does not check a known-positive control's refs or commits.

## Issues Encountered

None.

## User Setup Required

None. No external service configuration is required.

## Next Phase Readiness

- 02-07 and later plans can use `.class`, `.oracle`, `Scenario::control`, `Scenario::harn06` and the `rust_only` arm.
- 02-15 can read `Entry.kind`, `Scenario.harn06` and the subject action.

## Self-Check: PASSED

- Files: `tests/harness/scenario.rs`, `tests/harness/registry.rs` and `.config/nextest.toml` exist.
- Commits: `2396642`, `9552f52`, `5c646ed`, `5829116` and `54a24e3` are ancestors of HEAD.
- Measured with `git rev-list --count 5a9809d..HEAD`: 5 commits.
- `scripts/check.sh all` exited 0 after the last code commit: 95 passed, 2 skipped, `==> check.sh: all OK`.

---
*Phase: 02-oracle-harness*
*Completed: 2026-10-26*
