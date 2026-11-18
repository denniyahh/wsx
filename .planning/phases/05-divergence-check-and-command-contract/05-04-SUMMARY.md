---
phase: 05-divergence-check-and-command-contract
plan: 04
subsystem: check
tags: [rust, divergence, manifest, git, tdd, harness]

requires:
  - phase: 05-divergence-check-and-command-contract
    provides: "05-01 check contract (kinds, line texts) and evaluate; 05-03 Repo::lookup_paths"
provides:
  - "ProblemKind::{UndeclaredModified, UndeclaredDeleted, StaleOnly, StaleModified, RedundantBuiltin} with D-11 names and contract lines"
  - "evaluate(model, changes, carried): every DIV-01 and DIV-02 rule, problems in D-05 order"
  - "run's stale-only read: Repo::lookup_paths at model.integration.tip for the distinct only paths no built-in covers"
  - "tests/harness/check.rs: check_in helper; scenarios undeclared_modified, undeclared_deleted, stale_only, stale_modified, redundant_builtin, feature_worktree"
affects: [05-05, 05-06, 05-07, check]

actuals:
  tokens: 7713
  tasks: 2
  commits: 4
plan_head_before: 22c58620f5aaf06b7b762402aa6ce7ade665ff55
plan_head_after: 8dad4d2c9433407f1dc90d5683f30caaab6a9d4d

tech-stack:
  added: []
  patterns:
    - "Problems sort on (group, path bytes, kind), ProblemKind deriving Ord in D-11 declaration order"
    - "Entry staleness without pathspecs: modified via tree_diff plus manifest::covers, only via NUL-framed lookups"

key-files:
  created: []
  modified:
    - src/check/mod.rs
    - src/check/report.rs
    - tests/harness/check.rs

key-decisions:
  - "The stale-only lookup lives in carried_only_paths, called from run after model::resolve, so run and evaluate stay small and evaluate stays pure"
  - "Entries are deduplicated on (kind, path) keeping the first, before redundancy and staleness are judged"

patterns-established:
  - "check_in(checkout, config, fixture): a check scenario in any checkout, check_in_work delegating to it"

requirements-completed: [DIV-01, DIV-02]

coverage:
  - id: D1
    description: "Undeclared modifications, type changes and deletions of shared files are reported; a modified entry declares exactly its own path"
    requirement: DIV-01
    verification:
      - kind: unit
        ref: "src/check/mod.rs#check::tests::modified_shared_file_must_be_declared_exactly"
        status: pass
      - kind: unit
        ref: "src/check/mod.rs#check::tests::type_change_is_an_undeclared_modification"
        status: pass
      - kind: unit
        ref: "src/check/mod.rs#check::tests::deleted_shared_file_must_be_declared_modified"
        status: pass
      - kind: unit
        ref: "src/check/mod.rs#check::tests::modified_entry_declares_its_own_path_and_covers_below_it"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::undeclared_modified::rust"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::undeclared_modified::oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::undeclared_deleted::rust"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::undeclared_deleted::oracle"
        status: pass
    human_judgment: false
  - id: D2
    description: "Stale only and modified declarations and declarations under a built-in are reported, a redundant one as nothing else"
    requirement: DIV-02
    verification:
      - kind: unit
        ref: "src/check/mod.rs#check::tests::only_entry_the_integration_branch_carries_is_stale"
        status: pass
      - kind: unit
        ref: "src/check/mod.rs#check::tests::modified_entry_that_no_longer_differs_is_stale"
        status: pass
      - kind: unit
        ref: "src/check/mod.rs#check::tests::declaration_under_a_builtin_is_redundant_and_nothing_else"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::stale_only::rust"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::stale_only::oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::stale_modified::rust"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::stale_modified::oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::redundant_builtin::rust"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::redundant_builtin::oracle"
        status: pass
    human_judgment: false
  - id: D3
    description: "D-05 order (undeclared, then stale; path bytes; kind) and exact duplicates reported once"
    requirement: DIV-02
    verification:
      - kind: unit
        ref: "src/check/mod.rs#check::tests::problems_are_grouped_then_sorted"
        status: pass
      - kind: unit
        ref: "src/check/mod.rs#check::tests::exact_duplicate_declarations_report_once"
        status: pass
      - kind: unit
        ref: "src/check/mod.rs#check::tests::problems_are_sorted_by_path_bytes"
        status: pass
    human_judgment: false
  - id: D4
    description: "D-02: run from a feature worktree, wsx checks the workspace branch, not HEAD"
    requirement: DIV-01
    verification:
      - kind: integration
        ref: "tests/harness/check.rs#check::feature_worktree::rust"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::feature_worktree::oracle"
        status: pass
    human_judgment: false
  - id: D5
    description: "Every new kind renders its 05-01 contract line"
    verification:
      - kind: unit
        ref: "src/check/report.rs#check::report::tests::every_kind_renders_its_contract_line"
        status: pass
    human_judgment: false

duration: 13min
completed: 2026-11-18
status: complete
---

# Phase 5 Plan 04: Remaining divergence rules Summary

**wsx check now reports undeclared modifications and deletions, stale `only` and `modified` declarations and declarations a built-in already covers, in D-05's order, with the stale-only read done as NUL-framed tree lookups at the integration tip and six recorded scenarios beside the oracle.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-11-18T20:38:40Z
- **Completed:** 2026-11-18T20:51:30Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- A Modified or TypeChanged shared file needs a `modified` entry equal to its path, a Deleted one likewise; an `only` entry never declares a shared change.
- An `only` entry the integration tree holds is `stale_only`; a `modified` entry covering no change is `stale_modified`; an entry under a built-in is `redundant_builtin` and nothing else.
- Problems sort by group, then path bytes, then kind; exact duplicate entries are judged once.
- Run from a feature worktree, the check judges the workspace branch (D-02), where the oracle judges HEAD.

## Task Commits

1. **Task 1 RED: failing tests for the remaining divergence rules** - `9f9b7eb` (test)
2. **Task 1 GREEN: judge modifications, deletions and stale declarations** - `04fd2ba` (feat)
3. **Task 2 RED: the remaining divergence scenarios** - `5650ead` (test)
4. **Task 2 GREEN: read stale only declarations from the integration tree** - `8dad4d2` (feat)

## TDD Gate Compliance

- **Task 1 RED:** `check::tests::modified_shared_file_must_be_declared_exactly` failed on its first `assert_eq!` (left `[]`, right `[undeclared_modified README.md]`). Classifier: `RED_EVIDENCE_OK` (target/gsd/05-04-red-evidence.json). Semantic assessment: the crate compiled and the target ran; the failure is the planned assertion against the stub that judged only Added changes, not a build, load or fixture fault.
- **Task 2 RED:** `check::stale_only::rust` failed on exit code (expected 1, observed 0) and on its pinned stderr. Classifier: `RED_EVIDENCE_OK` (target/gsd/05-04-red-evidence-2.json). Semantic assessment: the fixture step's own git assertions (`shared/x.md` at `refs/remotes/origin/develop`, a two-parent workspace tip) passed before the action, and the scenario's oracle side passed on the same fixture, so the failure is the empty carried set.
- GREEN commits follow each RED: `04fd2ba` after `9f9b7eb`, `8dad4d2` after `5650ead`. No REFACTOR commit.

## Oracle observations

- `redundant_builtin`: the oracle exits **0** on `only .worktrees/x` (recorded as `OracleSide::Recorded(verdict(0))`; `check::redundant_builtin::oracle` passes against it, and the scenario's known-positive control sees the same oracle exit 1).
- `feature_worktree`: the oracle exits 1 printing `undeclared workspace-only path: feature.txt`, as the plan expected; no other line needed pinning.

## Files Created/Modified

- `src/check/mod.rs` - five new ProblemKind variants, the full `evaluate` with its carried-set input, `carried_only_paths` (the lookup), ten unit tests
- `src/check/report.rs` - contract lines for the new kinds; `problem_line` takes the quoted integration ref; `every_kind_renders_its_contract_line`
- `tests/harness/check.rs` - `check_in` helper and six scenarios, all `oracle, rust_enabled` (13 `rust_enabled` registrations)

## Decisions Made

- The lookup sits in `carried_only_paths`, called from `run` after `model::resolve`; the `lookup_paths` call is in that helper rather than in `run`'s body.
- Entries are deduplicated on (kind, path), first kept, before judging.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Known-positive controls on every new scenario**
- **Found during:** Task 2
- **Issue:** `validate` rejects a scenario without both controls (`GuardWithoutKnownPositive`); the plan listed only a clean control for undeclared_deleted, stale_only, stale_modified and redundant_builtin.
- **Fix:** each gets the shared undeclared `notes/todo.md` known-positive (`undeclared_note_in`), refused on both sides.
- **Files modified:** tests/harness/check.rs
- **Verification:** `registry::every_scenario_is_well_formed` passes; each scenario's sides pass.
- **Committed in:** 5650ead

**2. [Plan note] RED commit 9f9b7eb compiles the bin target with a dead-code warning**
- **Issue:** the five new variants are unconstructed on the non-test bin target in the RED stage. No allowance was added; the plan's alternative (wrong-answer construction in evaluate's arms) does not reach the three entry kinds, which no change kind produces.
- **Fix:** none needed past RED; GREEN `04fd2ba` constructs every variant and `scripts/check.sh all` (clippy `-D warnings`) passes.

---

**Total deviations:** 1 auto-fixed (blocking), 1 plan note. **Impact:** no scope change.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Hooks (05-06) and staleness (05-07) add their kinds and groups after `ProblemKind::RedundantBuiltin`; `group()` takes the hooks group as 2.
- `scripts/check.sh all` passed after each GREEN commit (log: target/gsd/05-04-all.log, last line `==> check.sh: all OK`).

---
*Phase: 05-divergence-check-and-command-contract*
*Completed: 2026-11-18*

## Self-Check: PASSED

All four task commits and the SUMMARY commit are ancestors of HEAD; the three modified source files and this SUMMARY exist.
