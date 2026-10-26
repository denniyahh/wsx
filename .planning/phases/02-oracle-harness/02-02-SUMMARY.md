---
phase: 02-oracle-harness
plan: 02
subsystem: testing
tags: [harness, oracle, nextest, tempfile, git, scenario-macro]

requires:
  - phase: 02-oracle-harness
    provides: "02-01: tests/oracle (devflow ee6ec792 scripts, oracle.patch) and tempfile as a dev-dependency"
provides:
  - "tests/harness: the single `wsx::harness` integration-test target with private modules env, fixture, action, oracle, rust, outcome, scenario and sync"
  - "hermetic_env / env::command: every spawn starts from env_clear() with the harness key table plus LLVM_*"
  - "Fixture with the devflow preset, deterministic clock (T0, TICK, ACTION_TIME), git/git_ok/write/commit/advance_remote"
  - "OracleCopy::prepare (scratch copy, git apply under the ceiling, WSX_ORACLE_DIR check) and the Sync mapping to sync-workspace.sh"
  - "Typed Outcome expectations (Exactly and Unchanged forms), a byte-exact observer that records symbolic refs by target, and a field-naming comparator"
  - "scenarios! (rust = \"<reason>\", rust_enabled, empty form) and harness_modules! with registries() and all_scenarios()"
  - "sync::self_test and sync::already_up_to_date, oracle sides passing, Rust sides ignored until phase 7"
affects: [02-03, 02-04, 02-05, 02-07, 02-08, 02-09, 02-15, phase-7]

actuals:
  tokens: 24396   # chars/4 over `git diff c277346 a8a80c2` (97583 chars)
  tasks: 2
  commits: 4
plan_head_before: c277346560c245f0b1f1422b1253ff1bef6a9eb1
plan_head_after: a8a80c270ab5aa6bdbdd07a310bf550c9df3b428

tech-stack:
  added: []
  patterns:
    - "Scenario = Case { config, fixture fn, abstract Action, expected Outcome }; scenarios! generates <module>::<name>::{oracle, rust} and registry()"
    - "Every harness spawn goes through env::command (env_clear, hermetic env, cwd, stdin null)"
    - "Observer reads raw git forms (for-each-ref %00, cat-file commit, ls-tree -z, worktree list -z); paths and messages stay bytes"
    - "Every TempDir is harness-<module>-<name>-* under CARGO_TARGET_TMPDIR"

key-files:
  created:
    - tests/harness/main.rs
    - tests/harness/env.rs
    - tests/harness/fixture.rs
    - tests/harness/action.rs
    - tests/harness/oracle.rs
    - tests/harness/rust.rs
    - tests/harness/outcome.rs
    - tests/harness/scenario.rs
    - tests/harness/sync.rs
  modified: []

key-decisions:
  - "run_case takes the registry Entry as well as the case, because a Case carries no name and the TempDir prefix needs harness-<module>-<name>-"
  - "Fixture steps return Result<(), String> (FixtureFn = fn(&mut Fixture) -> Result<(), String>), so a failing fixture step reaches run_case's Err instead of panicking"
  - "Adapters return Result<ActionRun, AdapterError>, where AdapterError is NotImplemented or Spawn, so a spawn failure is reported rather than panicked"
  - "Fixture::commit stores the message with --cleanup=verbatim; scenario messages carry their trailing newline"
  - "The observer fails loudly on a symlink in a checkout or a non-blob tree entry instead of recording it inexactly"
  - "Cargo discovers tests/harness/main.rs as target `harness` without a [[test]] entry (RESEARCH A2 confirmed with cargo nextest list)"

patterns-established:
  - "Each generated scenario test calls scenario::run(Side, &entry()); a disabled Rust side carries #[ignore = \"Rust side not enabled until phase N\"]"
  - "A failing scenario prints each mismatch in plain words, the action's stdout and stderr, and the observed outcome in scenario syntax (byte fields as b\"...\")"

requirements-completed: [HARN-01, HARN-02, HARN-03]

coverage:
  - id: D1
    description: "sync::self_test runs devflow's sync-workspace.sh from a patched scratch copy in a hermetic devflow fixture and passes on its explicit outcome"
    requirement: HARN-01
    verification:
      - kind: integration
        ref: "tests/harness/sync.rs#sync::self_test::oracle (scripts/check.sh test -E 'test(=sync::self_test::oracle)')"
        status: pass
      - kind: other
        ref: "RED evidence target/gsd/02-02-red-evidence.json: RED_EVIDENCE_OK, target sync::self_test::oracle"
        status: pass
    human_judgment: false
  - id: D2
    description: "The same definition generates sync::self_test::rust, ignored with `Rust side not enabled until phase 7`; selecting only it exits 4 with `no tests to run`"
    requirement: HARN-01
    verification:
      - kind: other
        ref: "scripts/check.sh test -E 'test(=sync::self_test::rust)' exits 4"
        status: pass
      - kind: other
        ref: "cargo nextest list --locked --run-ignored only -E 'test(=sync::self_test::rust)' lists wsx::harness sync::self_test::rust"
        status: pass
    human_judgment: false
  - id: D3
    description: "sync::already_up_to_date passes with exit 0, every ref unchanged, no new commit and files unchanged"
    requirement: HARN-03
    verification:
      - kind: integration
        ref: "tests/harness/sync.rs#sync::already_up_to_date::oracle"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every spawned process gets only the harness key table plus LLVM_* (pure hermetic_env, single spawn helper)"
    requirement: HARN-02
    verification:
      - kind: unit
        ref: "tests/harness/env.rs#env::hermetic_env_passes_only_the_harness_keys_and_llvm"
        status: pass
    human_judgment: false
  - id: D5
    description: "Symbolic refs are observed by target: Tip::Unchanged holds for origin/HEAD and fails for origin/develop after a fetch"
    requirement: HARN-03
    verification:
      - kind: integration
        ref: "tests/harness/outcome.rs#outcome::symbolic_refs_are_recorded_by_target"
        status: pass
    human_judgment: false
  - id: D6
    description: "A live scenario TempDir lies under CARGO_TARGET_TMPDIR as harness-<module>-<name>-* and is gone after drop"
    requirement: HARN-02
    verification:
      - kind: unit
        ref: "tests/harness/env.rs#env::scenario_dirs_carry_the_harness_prefix"
        status: pass
    human_judgment: false
  - id: D7
    description: "OracleCopy::prepare refuses a copy whose patched scripts lack WSX_ORACLE_DIR (a git apply that skipped silently)"
    requirement: HARN-01
    verification:
      - kind: other
        ref: "one-time control this session: prepare without GIT_CEILING_DIRECTORIES failed with `check-workspace-divergence.sh lacks WSX_ORACLE_DIR after git apply`; no committed test pins it (02-04 adds provenance tests)"
        status: pass
    human_judgment: true
    rationale: "The refusal was shown by a temporary mutation, not by a committed test; until 02-04 lands its provenance tests the verifier must judge whether the one-time control suffices"
  - id: D8
    description: "A scenario interrupted by a panic removes its TempDir on unwind; one killed by a nextest timeout leaves a harness-<module>-<name>-* directory under target/tmp (backstop truth)"
    requirement: HARN-03
    verification: []
    human_judgment: true
    rationale: "Not exercised in this plan: no test panics while ScenarioDirs is alive, and no timeout was induced; the plan marks it a backstop for 02-15's scratch audit"

duration: 18min
completed: 2026-10-26
status: complete
---

# Phase 2 Plan 02: Harness tracer Summary

**One `wsx::harness` test target where `sync::self_test` runs devflow's real `sync-workspace.sh` from a patched scratch copy in a hermetic fixture and passes on a fully stated outcome (two-parent merge by Harness at the action instant, develop fast-forwarded, origin/HEAD unchanged by target, seven files), while its Rust side is generated as an ignored test.**

## Performance

- **Duration:** about 18 min
- **Started:** 2026-10-26T16:42:46Z
- **Completed:** 2026-10-26T17:00:19Z
- **Tasks:** 2 (4 commits: RED, GREEN, Task 2, one added test)
- **Files modified:** 9 created

## Accomplishments

- `sync::self_test::oracle` passes. It asserts exit 0 and exactly five refs. `develop` and `origin/develop` are at `feat: add b`. `workspace/dev` is at a new merge commit: message `chore: sync develop into workspace/dev\n`, author and committer Harness, author date `1767312000 +0000`, parents `chore(ws): add workspace files` then `feat: add b`, a tree of seven files, unsigned. `origin/HEAD` and `origin/workspace/dev` are unchanged. The two new commits come in order, and the seven files on disk in `work` match.
- `sync::self_test::rust` and `sync::already_up_to_date::rust` exist and are ignored (`Rust side not enabled until phase 7`). nextest counts them as skipped, and a filter that selects only one exits 4.
- `sync::already_up_to_date::oracle` passes on `RefsExpect::Unchanged`, no new commits and `FilesExpect::Unchanged`.
- The observer records `refs/remotes/origin/HEAD` as `Symbolic("refs/remotes/origin/develop")`. A fetch that moves `origin/develop` leaves it unchanged, and in the same run the comparator flags `origin/develop` itself.
- The full gate is green: `scripts/check.sh all` gives 58 passed, 2 skipped, `==> check.sh: all OK`.

## Task Commits

1. **Task 1 RED: failing sync self-test scenario**: `048180d` (test)
2. **Task 1 GREEN: sync wired to the patched oracle copy**: `6a75971` (feat)
3. **Task 2: unchanged expectations, observer and scratch-naming tests**: `98d2c7a` (feat)
4. **Added: hermetic env key-table test** (a must-have truth no planned test pinned): `a8a80c2` (test)

## TDD Gate Compliance

- **RED** `048180d`: `scripts/check.sh test -E 'test(=sync::self_test::oracle)'` exited 100. The JUnit report is `target/nextest/default/junit.xml`; the run started at 2026-10-26T16:54:09.635Z and the report was written at 16:54:10.750Z. The record is at `target/gsd/02-02-red-evidence.json` (gitignored), target test `sync::self_test::oracle`, target file `tests/harness/sync.rs`. Classifier verdict: **`RED_EVIDENCE_OK`** (`target_test_failed`, matched `wsx::harness#sync::self_test::oracle`, 1 test, 1 fail).
  - **Semantic assessment:** the test compiled and built its fixture (the observed outcome lists the five preset refs and six files). It prepared the patched copy and failed on the comparator assertion (`scenario.rs:188`), naming 16 fields. The first of those was the exit code (expected 0, got 127), followed by every ref tip field and the new-commit count. It was not a compile error, a fixture panic or zero-test discovery. The run was re-made after `cargo fmt`, so the evidence matches the committed bytes.
  - **Classifier control:** the same record over the passing report gave `INVALID_RED unexpected_green`.
- **GREEN** `6a75971`: the same command passes `1 test run: 1 passed`, and `scripts/check.sh all` exited 0 before the commit.
- **REFACTOR:** none.
- `git log --grep` finds `test(02-02)` (`048180d`) before `feat(02-02)` (`6a75971`). `a8a80c2` is a later test-only commit, not a RED step.
- Backlog 999.4 (the nested nextest run in `tests/gate_controls.rs` shares `target/nextest` with the outer JUnit report) did not affect this evidence. The RED run was filtered to one harness test, so the nested run never executed, and no harness test starts a nested nextest. 999.4 stays in the backlog.

## Verification evidence and negative controls

Every `<verify>` command was run at final HEAD:

| Check | Result |
|---|---|
| `check.sh test -E 'test(=sync::self_test::oracle)'` | rc 0, `1 test run: 1 passed` |
| `check.sh test -E 'test(=sync::self_test::rust)'` | rc 4, `error: no tests to run` |
| `cargo nextest list --run-ignored only ...` | lists `wsx::harness` `sync::self_test::rust` |
| rust argv test, then registry test | rc 0 each |
| `already_up_to_date::oracle`, `symbolic_refs...`, `scenario_dirs...` | rc 0 each |
| `scripts/check.sh all` | rc 0, `==> check.sh: all OK` |

Controls, each a temporary mutation that was reverted afterwards:

| Control | Expected | Observed |
|---|---|---|
| RED adapter returns 127 without running the script | fail on comparison | 16 field mismatches |
| A: swap the expected merge parent order | fail on parents only | 2 mismatches: `refs/heads/workspace/dev: parents`, `new commit 2: parents` |
| B: `prepare` without `GIT_CEILING_DIRECTORIES` | refuse the copy | `check-workspace-divergence.sh lacks WSX_ORACLE_DIR after git apply` |
| C: `Command::new("bash")` with and without the hermetic PATH | different binaries | `/usr/bin/bash` vs `/home/linuxbrew/.linuxbrew/bin/bash` |
| D: an unused fn in a private harness module | clippy fails | `error: function unused_probe_helper is never used` |
| E: observer records symbolic refs as objects | symbolic-ref test fails | fails on the direct assertion. With those assertions removed, the comparator half alone reports `[RefTip(origin/HEAD), RefTip(origin/develop)]` |
| F: `already_up_to_date` with the advancing fixture | unchanged forms fail | 5 mismatches (3 ref tips, new-commit count, files), in plain words |
| G: TempDir `disable_cleanup(true)` | drop check fails | `... must be removed on drop`; the leaked dir was then deleted |
| H: env passthrough widened to every key | key-table test fails | assertion failed |
| Classifier on a green report | refuse | `INVALID_RED unexpected_green` |

Acceptance criteria: `main.rs` exists and contains no `pub mod`, and `outcome.rs` contains `%(symref)`. RED precedes GREEN in `git log $BASE..HEAD` (BASE `536fb37`). The frozen paths have an empty diff, and `tests/oracle` shows only the `(02-01)` subject. Task 2's function names and both `Unchanged` variants exit 0, against baselines of 1 at `6a75971`.

## What these checks do not establish

- **One action and one fixture.** The harness is proven end to end for Sync on the devflow preset only. The other four scripts are mapped and exercised from 02-07.
- **Hermeticity is shown for the key table and the PATH lookup, not against a planted host.** No test plants a host global hook or config and shows it does not fire; that proof is 02-09. Control C was a one-time probe and is not committed.
- **The committer date is not compared.** `CommitExpect` has no committer-date field (the plan's contract has none). So "committer Harness at the action instant" is asserted for the committer's name and email, and for the author date. The committer date is not asserted.
- **The patch-refusal path has no committed test.** Control B showed it once; 02-04 adds the provenance tests.
- **Panic-unwind and timeout cleanup are untested.** This is the backstop truth. No test panics while `ScenarioDirs` is alive (every panic here happens after `run_case` has returned), and no nextest timeout was induced. The only observation is that `target/tmp` held no `harness-*` directory after every run in this session, apart from the one control G leaked on purpose.
- **The symbolic-ref rule is shown only for a remote HEAD moved by a fetch.** No phase-2 action creates or retargets a symbolic ref. `Tip` has no form to expect one, and such a change shows as a ref mismatch.
- **The Rust command lines are guesses.** `rust::every_action_maps_to_a_wsx_command` only documents them, and each enabling phase revises its line.
- **The container gate was not run.** RESEARCH A1 (the CI image's git and coreutils on `/usr/local/bin:/usr/bin:/bin`) is unchecked here. Host git is 2.56.0 at `/usr/bin/git`.

## Files Created/Modified

- `tests/harness/main.rs`: target root, private modules, `harness_modules!(sync)` with `registries()` and `all_scenarios()`
- `tests/harness/env.rs`: `ScenarioDirs`, `Env`, `hermetic_env`, `inherited_env`, `command`, and two tests
- `tests/harness/fixture.rs`: clock constants, `Ident`, `RepoConfig::devflow`, `Layout::Devflow`, `ManifestKind::Only`, and `Fixture` with `checkout`, `git`, `git_ok`, `write`, `commit` and `advance_remote`
- `tests/harness/action.rs`: `Action` (six variants), `PushUpdate`, `push_input`, `Display`, and the pre-push stdin test
- `tests/harness/oracle.rs`: `OracleCopy::prepare`, `run_script`, `run` (Sync mapped), `ActionRun`, `NotImplemented` and `AdapterError`
- `tests/harness/rust.rs`: `wsx_argv`, `run`, `every_action_maps_to_a_wsx_command`
- `tests/harness/outcome.rs`: expectation types, `Snapshot`/`RefValue`, `observe`, `compare`, `Mismatch`/`Field`, `observed` transcription, and the symbolic-ref test
- `tests/harness/scenario.rs`: `Case`, `Scenario`, `Side`, `Entry`, `CaseRun`, `scenarios!`, `run`, `run_with_env`, `run_case`, and the registry and empty-form tests
- `tests/harness/sync.rs`: `self_test` and `already_up_to_date`

## Decisions Made

See `key-decisions` in the frontmatter. Each one is a signature change against the plan's contract, kept small and documented under Deviations.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `run_case` takes the registry entry**
- **Found during:** Task 1
- **Issue:** the contract's `run_case(side, case, make_env)` must name the TempDir `harness-<module>-<name>-`, but a `Case` carries no name.
- **Fix:** the signature is `run_case(side, &Entry, &Case, make_env)`. 02-08 and 02-09 already take their cases from `all_scenarios()`, so they have the entry.
- **Files modified:** `tests/harness/scenario.rs`
- **Committed in:** `048180d`

**2. [Rule 2 - Missing critical] Fixture steps and helpers return `Result`**
- **Found during:** Task 1
- **Issue:** the contract says `run_case` returns "the error a fixture step hit", but its `fixture_fn: fn(&mut Fixture)` can only panic.
- **Fix:** the fixture function type is `FixtureFn = fn(&mut Fixture) -> Result<(), String>`, and `Fixture::{new, git, git_ok, write, commit, advance_remote}`, `push_input`, `wsx_argv` and `observe` return `Result`. A planted-host control in 02-09 can now get an `Err` instead of a panic.
- **Files modified:** `fixture.rs`, `scenario.rs`, `action.rs`, `rust.rs`, `outcome.rs`
- **Committed in:** `048180d`

**3. [Rule 2 - Missing critical] Adapter spawn failures are values**
- **Found during:** Task 1
- **Fix:** `oracle::run` and `rust::run` return `Result<ActionRun, AdapterError>`, where `AdapterError` is `NotImplemented(NotImplemented { action, why })` or `Spawn(String)`. A `NotImplemented` on the oracle side still panics, naming the scenario and the action.
- **Committed in:** `048180d`

**4. [Rule 2 - Missing verification] Added `env::hermetic_env_passes_only_the_harness_keys_and_llvm`**
- **Found during:** writing the SUMMARY's coverage list
- **Issue:** the must-have "receives only the harness key table plus `LLVM_*`" had no test in this plan.
- **Fix:** a test over a planted inherited environment. Control H confirmed that it fails when the filter is widened.
- **Committed in:** `a8a80c2`

**5. [Scope - additive] Contract surface beyond the plan's list:**
- `push_input` returns a named `PushInput { url, stdin }`.
- `observe(fixture, before: Option<&Snapshot>)`, plus the `observed()` transcription.
- `Field::Commit { which, part: CommitPart }`, `FileState`, `TreeEntry::file`, `FileExpect::plain`, `Ident::{ada, harness}`, `fixture::{dated, WORK}`, `scenario::{FixtureFn, declaring_module}`, `oracle::spawn_and_wait`.
- `harness_modules!` also defines `registries()`. The registry test needs it to check `entry.module` against the module name `harness_modules!` lists, which is independent of `module_path!()`.
- Two extra tests: `action::pre_push_input_lists_one_line_per_update` and `scenario::an_empty_scenario_list_registers_nothing`. The second proves that `scenarios! {}` expands without warnings.

---

**Total deviations:** 4 auto-fixed (1 blocking, 3 missing critical or verification) plus additive surface.
**Impact on plan:** later plans that construct `Case::new` must pass a fixture function returning `Result<(), String>` and call `run_case` with the entry. Every contract name is kept.

## Issues Encountered

- My first attempt at the comparator-only half of control E did not apply the mutation, because rustfmt had reflowed the target lines and the script's assert caught it. The rc 0 from that attempt therefore measured the unmodified test and was discarded. The retry checked that the mutation applied (the script prints `mutated`) before trusting the result.

## Known Stubs

These are intentional per the plan, not goal-blocking:
- `tests/harness/oracle.rs`, the `run` arms for `NewWorktree`, `Cut`, `Check` and `PrePush`, return `NotImplemented("mapped to its script in plan 02-07")`. 02-07 maps them.
- `tests/harness/oracle.rs`, the `RemoveWorktree` arm, returns `NotImplemented`, because the oracle has no `rm`. That is permanent for the oracle side.
- `.planning/WINDOWS.md` does not exist, so nothing was appended to the ledger.

## User Setup Required

None.

## Next Phase Readiness

- 02-03 can widen `harness_modules!`, add `Layout::NoGsd`, `ManifestKind::Modified`, the `RepoConfig::with_*` builders and `Fixture::{remove, git_tick}`. `Fixture::new` already creates a separate bare push remote when `push_remote != base_remote`, but nothing exercises that until 02-03.
- To consider before many scenarios exist: `CommitExpect` has no committer date (see the limits above). Adding the field later changes every `CommitExpect` literal.

## Self-Check: PASSED

- FOUND: all 9 files under `tests/harness/` (`[ -f ]`)
- FOUND: commits 048180d, 6a75971, 98d2c7a, a8a80c2 (each an ancestor of HEAD)
- No file deletions in any of the four commits
- No `harness-*` directory left under `target/tmp`, and none of my probes left under the scratch directory

---
*Phase: 02-oracle-harness*
*Completed: 2026-10-26*
