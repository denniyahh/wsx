---
phase: 03-git-adapter
plan: 05
subsystem: git
tags: [git-binary, spawn, version-gate, local-env-vars, hooks, worktrees, oncelock]
status: complete

requires:
  - phase: 03-git-adapter
    provides: "03-01: Git::new/run, Error::{GitTooOld, GitFailed, Io}, render_argv, test support Scratch, Env, git_ok, write, commit_all, T0"
provides:
  - "MIN_GIT_VERSION (2, 54) and parse_git_version; Git::run checks `git --version` once per helper before any other spawn and refuses an older or unreadable git as GitTooOld naming what git printed"
  - "Git::with_own_worktree; a child whose canonical target is not exactly the own worktree loses every name `git rev-parse --local-env-vars` prints (queried once per helper, on first need, after the version check)"
  - "A failed or killed list query fails closed as GitFailed under argv `rev-parse --local-env-vars`; the requested command is never spawned without the list"
  - "testing::Worktrees and testing::linked_worktrees: main, wt1 beside it, wt2 nested at <main>/.worktrees/wt2, all canonical (03-08 builds its hook fixture on it)"
  - "Twelve tests in git::spawn::tests"
affects: [03-07, 03-08, 03-10]

actuals:
  tokens: 7891      # chars/4 over `git diff efb1634..HEAD` (src/git/spawn.rs + src/git/testing.rs, 704 insertions, 28 deletions)
  tasks: 2
  commits: 4
plan_head_before: efb163431d3eeb82e9c2ef719d1910fee11c316b
plan_head_after: 18378bc4ea02835c0a050d85ba098137515f3a49

tech-stack:
  added: []
  patterns:
    - "Per-helper OnceLock caches set only after success, so a failed check is retried by the next call and no caller proceeds before its own check passed"
    - "Fake git on the snapshot's PATH: a /bin/sh script that logs \"$*\" and execs the real git, with assertions written to stay true when the helper adds a spawn"

key-files:
  created: []
  modified:
    - src/git/spawn.rs
    - src/git/testing.rs

key-decisions:
  - "The version check and the --local-env-vars list are cached per helper, not per process: each helper resolves git through its own snapshot's PATH. Once per process holds in production because it builds one helper per process (03-07)"
  - "Own-versus-other is exact canonical equality (Some(canonical target) == own_worktree), never starts_with, because wsx nests sibling worktrees inside its own worktree directory"
  - "A version that cannot be parsed is refused (fail closed), with `found` set to the token after `git version ` or the whole first line"
  - "The list query runs through base_command with the whole snapshot (hook variables included), in the target directory; it needs no repository"

patterns-established:
  - "Mutation controls for decision rules: replacing equality with a prefix test turns nested_sibling_counts_as_another_worktree red, and moving the version check after the list query turns old_git_refused red"

requirements-completed: []

coverage:
  - id: D1
    description: "git older than 2.54, or one whose version cannot be read, is refused once per helper before any requested command runs, naming the version found and required"
    requirement: "GIT-01"
    verification:
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::old_git_refused"
        status: pass
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::git_2_54_is_accepted"
        status: pass
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::unparseable_git_version_is_refused"
        status: pass
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::git_version_output_parses_leniently"
        status: pass
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::version_is_checked_once_per_helper"
        status: pass
  - id: D2
    description: "A failing git child carries its argv, exit status and stderr; a signal-killed child is a failure with no exit status"
    requirement: "GIT-01"
    verification:
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::failing_child_reports_argv_status_and_stderr"
        status: pass
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::signal_killed_child_is_a_failure"
        status: pass
  - id: D3
    description: "A child aimed at another worktree, a nested one included, loses git's repo-local variables; a child aimed at wsx's own worktree, through a symlink too, keeps the hook environment; the helper adds nothing to argv, cwd or env"
    requirement: "GIT-02"
    verification:
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::sibling_target_removes_local_env_vars"
        status: pass
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::nested_sibling_counts_as_another_worktree"
        status: pass
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::own_target_through_a_symlink_keeps_the_env"
        status: pass
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::child_gets_exact_argv_cwd_and_no_bypass"
        status: pass
  - id: D4
    description: "A failed list query fails closed under its own argv before the requested command runs"
    requirement: "GIT-02"
    verification:
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::killed_list_query_is_a_failure_naming_its_argv"
        status: pass

duration: 6min
completed: 2026-10-27
---

# Phase 3 Plan 05: Git version gate and worktree-scoped environment removal Summary

**The spawn helper now refuses git below 2.54 (or an unreadable version) on its first spawn, and strips git's own `--local-env-vars` list from every child whose canonical target is not exactly wsx's own worktree, with nested siblings and symlinked own paths pinned.**

## Performance

- **Duration:** about 6 min
- **Started:** 2026-10-27T02:16:01Z
- **Completed:** 2026-10-27T02:21:54Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `run` checks `git --version` once per `Git` value before anything else. git 2.53.0 is refused as `GitTooOld { found: "2.53.0", required: (2, 54) }` with only `--version` run. `nonsense` is refused as `found: "nonsense"`. 2.54.0 is accepted.
- `command` canonicalises the target. It keeps the whole snapshot only on exact equality with the own worktree. Any other target loses every name git's own `rev-parse --local-env-vars` prints. That list is queried on first need and cached per helper.
- A list query that is killed or exits non-zero fails closed as `GitFailed` under its own argv, and the requested command never runs.
- `testing::linked_worktrees` builds main, wt1 beside it and wt2 nested in main, all canonical.

## Task Commits

1. **Task 1 RED: version gate tests** - `fbaef2f` (test)
2. **Task 1 GREEN: refuse git older than 2.54** - `401bd21` (feat)
3. **Task 2 RED: env removal tests and fixture** - `94d8ed6` (test)
4. **Task 2 GREEN: drop repo-local variables for other worktrees** - `18378bc` (feat)

## TDD Gate Compliance

- **Task 1 RED:** `RED_EVIDENCE_OK` for `git::spawn::tests::old_git_refused`. The record is `target/gsd/03-05-red-evidence.json` (JUnit, exit 100). Semantic assessment: the target ran (1 test run, 255 skipped) and failed on the planned `GitTooOld` assertion. `run` returned `Ok(<repo>/.git)` because the RED helper never ran `git --version`. The fake git was written, executed and passed the requested command to the real git, so this was not a build, fixture or discovery fault. In the same RED state, the five gate tests failed. The two failure-mapping tests passed, because they pin 03-01 behaviour that already existed.
- **Task 2 RED:** `RED_EVIDENCE_OK` for `git::spawn::tests::sibling_target_removes_local_env_vars`. The record is `target/gsd/03-05-red-evidence-2.json` (JUnit, exit 100). Semantic assessment: the target failed on the planned removal assertion. Before that, the fixture built, the independent list query succeeded and all five planted names were confirmed to be on it. The first name reported is `GIT_CONFIG_PARAMETERS`, not `GIT_DIR`, only because the loop walks a `BTreeSet`; every planted name was present. In that RED state, 4 of the 5 new tests failed. `own_target_through_a_symlink_keeps_the_env` passed, as expected, because the RED helper kept the whole snapshot.
- Each RED commit comes before its GREEN commit. No refactor commit was needed.
- **Task 1's tests are unedited:** `git diff 401bd21..HEAD -- src/git/spawn.rs` changes only two `use` lines in the test module and appends the new tests. No line inside Task 1's seven test bodies changed, and all seven pass after Task 2.

## Verification

- `scripts/check.sh all` exited 0 before each GREEN commit, with the pinned nextest from `target/gsd-tools` on PATH: 214 tests after Task 1 and 219 after Task 2, all passed.
- Each of the 12 tests was run alone under `--no-tests=fail -E 'test(=...)'`, as were Task 2's four rerun checks on Task 1 tests. Every run exited 0.
- **Negative controls (mutations, reverted afterwards):**
  - Replacing the equality with `target.starts_with(own)` makes `nested_sibling_counts_as_another_worktree` fail with "reached a nested sibling's child".
  - Moving the version check after `command` makes `old_git_refused` fail with log `["rev-parse --local-env-vars", "--version"]`.
  - The RED runs are the controls that the removal and gate tests fail when the behaviour is absent.
- **Deferred `GIT_DIR` item not made worse:** with a hostile `GIT_DIR` and `GIT_INDEX_FILE` in the test process, the 12 spawn tests still pass. The reads/repo suites fail 17, the same count the deferred item records.

## What these checks do not establish

- The removal tests inspect the built `Command`. They prove the decision rule, not that git then behaves correctly inside a real hook. That is 03-08's real-spawn half.
- The fake-git tests check the gate's logic against git's output format, not against a real git 2.53 install.
- Both caches are per helper. "Once per process" rests on production building one helper per process (03-07's `from_process`), and no test in this plan pins that.
- Concurrent first calls on one `Git` are not exercised. By design, racing callers may each run the check once.
- The worktree-root contract is documented, not enforced. A caller that passes a subdirectory of the own worktree gets the removal (the safe direction), and no test pins that case.
- A list query that exits 0 but prints nothing would make the removal vacuous. Git 2.54 and later print the list, so this plan does not guard against it.

## Decisions Made

See `key-decisions` in the frontmatter. All four sit inside the plan's own instructions; none needed the owner.

## Deviations from Plan

None. The plan was executed as written.

## Issues Encountered

- A wider `git::spawn` test run overwrote `target/nextest/default/junit.xml` before the Task 2 RED record was built. I re-ran the target test alone and built the record from that run, so the record's report, start time and modification time all come from one run.

## Requirements

GIT-01 and GIT-02 are also declared by 03-07, 03-08 and 03-10. As in 03-03 and 03-04, this plan does not mark them complete in REQUIREMENTS.md.

## Cleanup

- Removed: the RED console, rc and start-time files and the two gate logs under `target/gsd/`, and the mutation backup copy.
- No `git-unit-*` scratch directory remains under `target/tmp`. No background process is running.
- Kept: `target/gsd/03-05-red-evidence.json` and `-2.json` (the plan requires them) and `target/gsd-tools/` (needed by later plans). All are gitignored.
- My scratchpad helper scripts are removed at the end of the run.

## Next Phase Readiness

- 03-07 can build `Git::from_process` on `new` plus `with_own_worktree(Repo::worktree())`.
- 03-08 can build its hook fixture on `testing::linked_worktrees`.

## Self-Check: PASSED

- FOUND: src/git/spawn.rs, src/git/testing.rs, target/gsd/03-05-red-evidence.json, target/gsd/03-05-red-evidence-2.json
- FOUND: fbaef2f, 401bd21, 94d8ed6, 18378bc (`git rev-list --count efb1634..HEAD` = 4)
