---
phase: 04-repo-model
plan: 17
subsystem: git-adapter
tags: [git, git-config, environment, fail-closed, tdd]

requires:
  - phase: 04-repo-model
    provides: "04-14's single git config read (read_config) and 04-16's spawn helpers"
provides:
  - "Git::run_without_var: one child run with one named variable removed, even in wsx's own worktree"
  - "read_config runs without GIT_CONFIG, so a configured repository stays configured while it is exported"
affects: [phase-06-guard, git-adapter, repo-model]

actuals:
  tokens: 2000
  tasks: 2
  commits: 2
plan_head_before: b68aea9f8efd6b4d389b5b86b22013dc8e6ff7cf
plan_head_after: 7b707cc4e31c9c3ba6d53a1ede470f5bea3b3b7e

tech-stack:
  added: []
  patterns:
    - "A per-child variable removal (Command::env_remove) is the only exception to Phase 3's own-worktree rule, used by the config read alone"

key-files:
  created: []
  modified:
    - src/git/spawn.rs
    - src/git/config.rs
    - src/model/mod.rs

key-decisions:
  - "Task 1 answered by the operator before execution: drop (04-CONTEXT D-38; STATE decision line, commit 383c74b); not re-recorded"

patterns-established:
  - "Git's own answer asserted first: the model test checks git config under GIT_CONFIG prints only command-scope records before asserting wsx's state"

requirements-completed: [CONF-04, CONF-07]

coverage:
  - id: D1
    description: "A configured repository resolves configured under GIT_CONFIG=<alt>, and not-configured once wsx.sourceBranch is unset (control), with git's command-only answer asserted first"
    requirement: CONF-07
    verification:
      - kind: integration
        ref: "src/model/mod.rs#model::tests::git_config_variable_does_not_unconfigure_a_repo"
        status: pass
    human_judgment: false
  - id: D2
    description: "Git::run_without_var hides only the named variable from an own-worktree child; run keeps both (control)"
    requirement: CONF-04
    verification:
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::run_without_var_hides_only_that_variable"
        status: pass
    human_judgment: false
  - id: D3
    description: "Full gate green after the change"
    verification:
      - kind: other
        ref: "scripts/check.sh all (target/gsd/04-17-all.log, 683 passed, 63 skipped)"
        status: pass
    human_judgment: false

duration: 6min
completed: 2026-11-17
status: complete
---

# Phase 4 Plan 17: GIT_CONFIG on the config read Summary

**wsx's one `git config` read now runs without `GIT_CONFIG`, so an exported `GIT_CONFIG` no longer makes a configured repository read as not configured (the fail-open the Phase 6 guard would have exited 0 on).**

## Performance

- **Duration:** 6 min
- **Started:** 2026-11-17T23:31:00Z
- **Completed:** 2026-11-17T23:37:00Z
- **Tasks:** 2 (Task 1 pre-answered, Task 2 implemented)
- **Files modified:** 3

## Task 1: the operator's answer

Answered before this run, so no checkpoint was returned. Recorded answer, quoted:

- STATE.md Decisions: `- [Phase 04]: 04-17: GIT_CONFIG in wsx's environment: drop, per the operator on 2026-11-17` (commit 383c74b).
- 04-CONTEXT.md D-38 (operator, 2026-11-17): "When `GIT_CONFIG` is set, the config read drops it for that one child, so `git config` reads the repository's own config as every other git command does. This amends Phase 3's rule that a child aimed at wsx's own worktree gets the whole environment, for `GIT_CONFIG` on the config child only."

The decision line was present before any code (Task 2's precondition held) and was not added again.

## Accomplishments

- `Git::run_without_var(worktree, args, var)` in src/git/spawn.rs: `run` with `Command::env_remove(var)` on the child; its doc says only the config read uses it and why.
- `Repo::read_config` calls it with `"GIT_CONFIG"`; its doc says why. `rg run_without_var src` finds no other production caller, so Phase 3's own-worktree rule is unchanged for every other child and variable.
- Git's answer pinned beside wsx's: under `GIT_CONFIG=<alt>`, `git config -z --show-scope --show-origin --get-regexp '^(wsx|remote)\.'` prints only `command` records and no `.git/config` origin.

## Task Commits

1. **Task 2 RED:** `14745d8` test(04-17): add failing tests for GIT_CONFIG on the config read
2. **Task 2 GREEN:** `7b707cc` fix(04-17): read the repository's config as every other git command does

## TDD Gate Compliance

- **RED:** `model::tests::git_config_variable_does_not_unconfigure_a_repo` failed at src/model/mod.rs:5929 with `assertion \`left == right\` failed: the child under GIT_CONFIG=<root>/alt.conf`, `left: "not-configured"`, `right: "configured"`, after both git-first assertions passed; log `target/gsd/04-17-red/git_config_variable_does_not_unconfigure_a_repo.log` ends `exit 100`. `gsd_run check tdd-red-evidence target/gsd/04-17-red-evidence.json --raw --pick verdict` printed `RED_EVIDENCE_OK`. Semantic assessment: the target ran and failed on its planned assertion for the planned reason (the config child kept `GIT_CONFIG` in wsx's own worktree); not a build, load or fixture failure.
- **RED (spawn):** `git::spawn::tests::run_without_var_hides_only_that_variable` failed at src/git/spawn.rs:853 with `the child of run_without_var(GIT_CONFIG) saw GIT_CONFIG` (env listed `GIT_CONFIG=x`); the RED stub forwarded to `run`. Log ends `exit 100`; `target/gsd/04-17-red-evidence-spawn.json` gave `RED_EVIDENCE_OK`.
- **Negative control on the classifier:** the spawn report with the model test named as target gave `INVALID_RED` (`no_target_test_failure`); that control file was deleted after the run.
- **GREEN:** both tests pass; `scripts/check.sh all` exit 0, last line `==> check.sh: all OK` (683 passed, 63 skipped).
- **Commit type:** GREEN is `fix(04-17)` as the plan prescribes, not `feat(04-17)`, so a gate check that greps only for `feat(` will report no GREEN for this plan; the GREEN commit `7b707cc` follows the RED commit `14745d8`.
- **REFACTOR:** none.

## Files Created/Modified

- `src/git/spawn.rs` - `run_without_var` and its test
- `src/git/config.rs` - `read_config` drops `GIT_CONFIG`; doc
- `src/model/mod.rs` - `git_config_variable_does_not_unconfigure_a_repo`

## Decisions Made

None beyond the operator's D-38.

## Deviations from Plan

**1. [Rule 3 - Blocking] RED-gate classifier run with `--raw --pick verdict`**
- **Found during:** Task 2 RED
- **Issue:** `--raw` alone prints the whole JSON, so piping it into an exact `RED_EVIDENCE_OK` match cannot pass (known plan-text defect, per the orchestrator).
- **Fix:** ran `--raw --pick verdict`; negative control above shows the gate still returns `INVALID_RED` when it should.
- **Files modified:** none (evidence files under gitignored `target/gsd/`)

**2. [Rule 3 - Blocking] Fake git answers `--version`**
- **Found during:** Task 2 RED
- **Issue:** the plan's fake body `env` would fail `ensure_version` (GitTooOld) before the child under test ran.
- **Fix:** the body answers `--version` with `git version 2.54.0` and prints `env` for every other call; the helper is built with the scratch root as its own worktree, the case the drop changes.
- **Files modified:** src/git/spawn.rs (test only), commit 14745d8

**Total deviations:** 2 auto-fixed (both blocking). **Impact:** none on scope.

## Issues Encountered

None.

## User Setup Required

None.

## Next Phase Readiness

Ready for 04-18. Not measured here: whether gix's reads before 04-14 honoured `GIT_CONFIG` (the plan lists it as unmeasured; no current read goes through gix's config for `wsx.*`).

---
*Phase: 04-repo-model*
*Completed: 2026-11-17*

## Self-Check: PASSED

- FOUND 14745d8
- FOUND 7b707cc
- FOUND src/git/spawn.rs
- FOUND src/git/config.rs
- FOUND src/model/mod.rs
- FOUND target/gsd/04-17-red-evidence.json
- FOUND target/gsd/04-17-all.log
