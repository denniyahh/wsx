---
phase: 05-divergence-check-and-command-contract
plan: 01
subsystem: cli
tags: [rust, cli, exit-codes, divergence-check, gix]

requires:
  - phase: 04-repo-model
    provides: model::resolve, RepoModel::require, Model, manifest covers/encode_path
  - phase: 03-git-adapter
    provides: Repo::discover, Repo::tree_diff, git::Error, c_quote
provides:
  - "wsx check: argv -> COMMANDS table -> Repo::discover -> model::resolve -> tree_diff -> evaluate -> stderr report and exit code"
  - "COMMANDS table (src/cli/commands.rs), data only, includable with #[path]"
  - "exit::classify / of_git: every git::Error (20) and model::Error (16) variant to one D-06 class and snake_case kind"
  - "docs/exit-codes.md: class table and kind table"
  - "check::Report, Problem, ProblemKind::UndeclaredAdded, report::human"
  - "Model::builtins"
affects: [05-02, 05-04, 05-06, 05-07, 05-08, 05-09]

actuals:
  tokens: 14036
  tasks: 2
  commits: 4

plan_head_before: 0e615013198ea10c2ebff19e4795e2774648766a
plan_head_after: cef4dcdf4f5f14439f769c88929cd2d1a461cfaf

tech-stack:
  added: []
  patterns:
    - "Exhaustive error classification: one arm per variant with { .. }, wildcard lints denied on the function"
    - "check::run reads model::resolve first, then only the two tips it recorded"

key-files:
  created:
    - src/cli/mod.rs
    - src/cli/commands.rs
    - src/exit.rs
    - src/check/mod.rs
    - src/check/report.rs
    - docs/exit-codes.md
  modified:
    - src/main.rs
    - src/git/mod.rs
    - src/model/mod.rs
    - tests/cli.rs
    - tests/harness/check.rs
    - tests/harness/rust.rs

key-decisions:
  - "Report and CommandSpec fields need no dead_code expectation: their derived PartialEq reads them; ExitClass::name and ProblemKind::name carry cfg_attr(not(test)) expectations until --json reads them (05-02)"
  - "The exit-codes doc test checks a row opening with each kind in backticks; each kind's class is pinned by the variant tests, not by the doc test"
  - "replace_refs_refused: the oracle was observed to exit 0 (git applies the replace ref, so the tips compare equal) and is recorded so"

patterns-established:
  - "A usage error prints 'wsx: <problem>' then one usage line per form, exit 2, nothing on stdout"
  - "Every error of a command prints 'wsx <command>: <Display>' and exits exit::classify(&err).class.code()"

requirements-completed: [DIV-01, IF-01]

coverage:
  - id: D1
    description: "wsx check reports an undeclared added path end to end and passes a fully declared workspace branch"
    requirement: DIV-01
    verification:
      - kind: e2e
        ref: "tests/harness/check.rs#check::self_test::rust"
        status: pass
      - kind: e2e
        ref: "tests/harness/check.rs#check::quoted_personal_names::rust"
        status: pass
      - kind: e2e
        ref: "tests/harness/check.rs#check::fork_layout::rust"
        status: pass
      - kind: e2e
        ref: "tests/harness/check.rs#check::no_gsd_repo::rust"
        status: pass
      - kind: e2e
        ref: "tests/harness/check.rs#check::no_gsd_custom_personal_path::rust"
        status: pass
      - kind: unit
        ref: "src/check/mod.rs#check::tests::added_path_under_a_builtin_is_not_a_problem"
        status: pass
      - kind: unit
        ref: "src/check/mod.rs#check::tests::added_path_is_declared_only_by_a_covering_only_entry"
        status: pass
      - kind: unit
        ref: "src/check/mod.rs#check::tests::problems_are_sorted_by_path_bytes"
        status: pass
    human_judgment: false
  - id: D2
    description: "Usage errors exit 2 with nothing on stdout and the usage lines on stderr; --version unchanged"
    requirement: IF-01
    verification:
      - kind: integration
        ref: "tests/cli.rs#missing_argument_exits_with_usage"
        status: pass
      - kind: integration
        ref: "tests/cli.rs#check_with_an_extra_argument_exits_with_usage"
        status: pass
      - kind: integration
        ref: "tests/cli.rs#version_flag_prints_package_version"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every git::Error and model::Error variant maps to one D-06 class and kind, replacement refusals to 5 wherever wrapped, documented in docs/exit-codes.md"
    requirement: IF-01
    verification:
      - kind: unit
        ref: "src/exit.rs#exit::tests::every_git_error_has_its_class"
        status: pass
      - kind: unit
        ref: "src/exit.rs#exit::tests::every_model_error_has_its_class"
        status: pass
      - kind: unit
        ref: "src/exit.rs#exit::tests::replacement_refusals_are_refused_wherever_wrapped"
        status: pass
      - kind: unit
        ref: "src/exit.rs#exit::tests::exit_codes_doc_lists_every_class_and_kind"
        status: pass
      - kind: unit
        ref: "src/exit.rs#exit::tests::class_codes_are_the_d06_table"
        status: pass
      - kind: other
        ref: "planted-wildcard control: clippy on a copy with NotARepo's arm replaced by _ fails with match_wildcard_for_single_variants"
        status: pass
    human_judgment: false
  - id: D4
    description: "Reftable and replace-ref repositories are refused with exit 5 end to end"
    requirement: IF-01
    verification:
      - kind: e2e
        ref: "tests/harness/check.rs#check::reftable_refused::rust"
        status: pass
      - kind: e2e
        ref: "tests/harness/check.rs#check::replace_refs_refused::rust"
        status: pass
      - kind: e2e
        ref: "tests/harness/check.rs#check::replace_refs_refused::oracle"
        status: pass
    human_judgment: false

duration: 17min
completed: 2026-11-18
status: complete
---

# Phase 5 Plan 01: wsx check tracer and exit classes Summary

**`wsx check` runs end to end (command table, model-first read order, integration-to-workspace tree diff, pure evaluator) and refuses undeclared workspace-only paths; every git and model error exits through an exhaustive D-06 classifier documented in docs/exit-codes.md.**

## Performance

- **Duration:** 17 min
- **Started:** 2026-11-18T19:42:23Z
- **Completed:** 2026-11-18T19:58:55Z
- **Tasks:** 2
- **Files modified:** 12

## Accomplishments
- `wsx check` judges added paths on real repositories; five existing scenarios now run their Rust side and agree with the oracle where it has one.
- Every error class and kind is pinned per variant; a new variant cannot compile unclassified, and a planted wildcard is refused by clippy.
- Reftable and replace-ref repositories exit 5 with a message naming the cause.

## Task Commits

1. **Task 1 RED: enable the added-path scenarios' Rust side** - `ca6c005` (test)
2. **Task 1 GREEN: run wsx check end to end for added paths** - `da76afb` (feat)
3. **Task 2 RED: pin every error's exit class and kind** - `d459ef8` (test)
4. **Task 2 GREEN: classify every error into the D-06 exit classes** - `cef4dcd` (feat)

## TDD Gate Compliance

- **Task 1 RED:** `check::self_test::rust` failed on its outcome comparison: exit 2 (the usage line, no check command) where 0 was expected. Classifier: `RED_EVIDENCE_OK` (target/gsd/05-01-red-evidence.json). Semantic assessment: the target ran, fixtures built, wsx spawned in `work`; the failure is the planned exit-code assertion, not a build or fixture fault.
- **Task 1 GREEN:** all twelve named tests pass; `scripts/check.sh all` OK.
- **Task 2 RED:** `exit::tests::every_git_error_has_its_class` failed on its class assertion: `(GitFailed, "git_failed")` against `(NotConfigured, "not_a_repo")`, the stub answering git_failed for every error. Classifier: `RED_EVIDENCE_OK` (target/gsd/05-01-red-evidence-2.json).
- **Task 2 GREEN:** the five exit tests and both sides of reftable_refused and replace_refs_refused pass; `scripts/check.sh all` OK.
- No REFACTOR commits.

## Files Created/Modified
- `src/cli/commands.rs` - COMMANDS table (CommandId::Check, "check", schema 1), data only
- `src/cli/mod.rs` - argv grammar, dispatch, usage text, check rendering, exit codes
- `src/exit.rs` - ExitClass, Classified, classify, of_git and their tests
- `src/check/mod.rs` - Report, Problem, ProblemKind, run, evaluate and pure tests
- `src/check/report.rs` - human header, problem lines and verdict
- `docs/exit-codes.md` - class table and kind table
- `src/main.rs` - declares the new modules; main returns cli::run
- `src/model/mod.rs` - Model::builtins; personal_paths builds from it
- `src/git/mod.rs` - re-exports ChangeKind, TreeChange; test-only ObjectId::for_test
- `tests/cli.rs` - check_with_an_extra_argument_exits_with_usage
- `tests/harness/check.rs` - wsx-side reasons, seven Rust sides enabled, replace_refs_refused
- `tests/harness/rust.rs` - module doc notes the check line is final

## Decisions Made
- Dead-code expectations: no field-level expectation was needed (derived PartialEq reads the fields); `ExitClass::name` and `ProblemKind::name` carry `cfg_attr(not(test), expect(dead_code))` until 05-02's JSON reads them. The module-level expectations in src/git/mod.rs and src/model/mod.rs stayed fulfilled, so none was removed.
- The doc test checks a `| <code> | <name> |` row per class and a row opening with each kind in backticks; the kind's class is pinned by the variant tests. `manifest_unreadable` is documented as taking the class of the failed read.
- replace_refs_refused's oracle side was run once and observed to exit 0 with nothing changed; recorded as `OracleSide::Recorded(verdict(0))`. Its known-positive control (exit 1) passed in the same run.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Title-only docs/exit-codes.md in the Task 2 RED commit**
- **Found during:** Task 2 RED
- **Issue:** `include_str!("../docs/exit-codes.md")` does not compile without the file, which would make the RED a build failure rather than an assertion failure.
- **Fix:** committed the doc with its title only in RED; GREEN wrote the tables.
- **Files modified:** docs/exit-codes.md
- **Committed in:** d459ef8, cef4dcd

**2. [Rule 3 - Blocking] Temporary dead_code expectation on ExitClass in Task 1**
- **Found during:** Task 1 GREEN
- **Issue:** with the stub classify, NotConfigured, InvalidConfig and Refused were never constructed, which `-D warnings` refuses.
- **Fix:** an expectation on the enum in Task 1 (`cfg_attr(not(test))` in Task 2 RED, where the tests construct them), removed in Task 2 GREEN.
- **Files modified:** src/exit.rs
- **Committed in:** da76afb, d459ef8, cef4dcd

**3. [Rule 1 - Bug] commands.rs module doc named `crate::` and `super::`**
- **Found during:** Task 1 acceptance criteria
- **Issue:** the doc sentence explaining the no-path rule tripped the `! rg -q 'crate::|super::'` criterion.
- **Fix:** reworded it without the path tokens.
- **Files modified:** src/cli/commands.rs
- **Committed in:** da76afb

---

**Total deviations:** 3 auto-fixed (2 blocking, 1 bug). **Impact:** none on scope; each kept a gate meaningful.

## Issues Encountered
None.

## Known Stubs
- `src/check/mod.rs`, `evaluate`: the `Modified | Deleted | TypeChanged` arm yields no problem; 05-04 judges changes to shared files (planned scope boundary, comment names 05-04).

## Verification
- Task 1 verify: the twelve named tests, one per `scripts/check.sh test -E 'test(=...)'` command, all exit 0.
- Task 2 verify: the five exit tests and both sides of reftable_refused and replace_refs_refused exit 0; the source scan prints no wildcard arm and finds NotARepo's own line; a negative control planting `other =>` in a copy makes the scan exit 1 naming the bare-binding arm.
- Planted-wildcard control: clippy on a copy with `_ =>` in place of NotARepo's arm exits 101 with `match_wildcard_for_single_variants`, no unreachable pattern.
- `scripts/check.sh all`: last log line `==> check.sh: all OK` after each GREEN commit.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
Ready for 05-02 (JSON output over Report, Classified.kind and ExitClass::name).

## Self-Check: PASSED
- FOUND: src/cli/mod.rs, src/cli/commands.rs, src/exit.rs, src/check/mod.rs, src/check/report.rs, docs/exit-codes.md
- FOUND commits: ca6c005, da76afb, d459ef8, cef4dcd

---
*Phase: 05-divergence-check-and-command-contract*
*Completed: 2026-11-18*
