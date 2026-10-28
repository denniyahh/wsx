---
phase: 03-git-adapter
plan: 07
subsystem: git
tags: [git-binary, spawn, re-exec, null-stdin, hooks, ssh-signing, local-env-vars, fail-closed]
status: complete

requires:
  - phase: 03-git-adapter
    provides: "03-01: Git::new/run, test support Scratch, Env, command, git_ok, write, commit_all, T0, TICK; 03-05: with_own_worktree, the --local-env-vars removal list, fake_git, linked_worktrees; 03-02: Repo::discover, Repo::worktree"
provides:
  - "testing::{MARKER, child_print, reexec, marker}: the re-exec harness 03-08, 03-09 and 03-10 use, with its zero-test trap pinned by reexec_of_an_unknown_child_has_no_marker"
  - "Git::from_process(&Repo): the production constructor, a process-environment snapshot taken once plus the repo's worktree as own worktree"
  - "Proof from a re-executed child with a real piped stdin that helper children get a null stdin, with an inheriting-child control"
  - "Proof that a commit and a --no-ff merge through Git::run run the user's hooks and carry good SSH signatures from the user's local config, with an explicit-bypass control"
  - "Orchestrator addition: an exit-0 --local-env-vars query printing no name fails closed as Error::GitEmptyOutput before any child aimed at another worktree is built"
affects: [03-08, 03-09, 03-10]

actuals:
  tokens: 6359      # chars/4 over `git diff 9e291dd..HEAD` (src/git/{spawn,testing,mod}.rs, 525 insertions, 5 deletions)
  tasks: 3          # 2 plan tasks + 1 orchestrator addition
  commits: 4
plan_head_before: 9e291dde01da610a1e54a45f249b8efba4b76fa4
plan_head_after: 3a829dee3bbf4cd9480c6e3e960de1d9567c9a03

tech-stack:
  added: []
  patterns:
    - "Re-exec child halves: #[ignore = \"child half of <parent>; runs only when re-executed\"] tests that read the process environment and cwd and print WSXCHILD key=value; parents assert exit 0 and then the marker, never the exit status alone"
    - "reexec feeds stdin from a scoped thread, so a child that never reads stdin or writes a lot first cannot deadlock the parent"

key-files:
  created: []
  modified:
    - src/git/testing.rs
    - src/git/spawn.rs
    - src/git/mod.rs

key-decisions:
  - "An empty --local-env-vars list is a new variant, Error::GitEmptyOutput { argv, expected }, rendered `git rev-parse --local-env-vars exited 0 but printed no variable names`. GitFailed means `exited unsuccessfully` and would have rendered `... exited 0`, a refusal that does not name its cause"
  - "marker() finds the marker anywhere in a line, not only at line start, so libtest's `test <name> ... ` prefix cannot hide a child's fact line"
  - "A failed write of the child's stdin (for example a broken pipe from a child that exited first) is an error from reexec, a false red rather than a silently unfed child"

patterns-established:
  - "Negative controls by mutation: switching the helper's child stdin to Stdio::inherit() turns children_get_null_stdin red with the marker's blob 7d43155320b1c0ebb5027ceeed291de7e06fdc2e instead of the empty blob"

requirements-completed: []

coverage:
  - id: D1
    description: "The re-exec harness refuses to treat a zero-test child run as a result: an unknown child name exits 0 with `running 0 tests` and no marker"
    requirement: "GIT-01"
    verification:
      - kind: unit
        ref: "src/git/testing.rs#git::testing::reexec_of_an_unknown_child_has_no_marker"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every git child the helper runs gets a null stdin, shown from a re-executed child whose own stdin holds data, with a control that an inheriting child in the same setup sees that data"
    requirement: "GIT-01"
    verification:
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::children_get_null_stdin"
        status: pass
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::piped_stdin_reaches_an_unmanaged_child"
        status: pass
    human_judgment: false
  - id: D3
    description: "Criterion 1, first half: a commit and a merge through the helper run the user's pre-commit, commit-msg and pre-merge-commit hooks and are signed per the user's config (%G? = G); an explicit caller bypass gives N and no hook runs"
    requirement: "GIT-01"
    verification:
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::commit_and_merge_run_hooks_and_sign"
        status: pass
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::explicit_bypass_skips_hooks_and_signing"
        status: pass
    human_judgment: false
  - id: D4
    description: "An exit-0 --local-env-vars query that prints nothing or only whitespace fails closed under its own argv before the requested command spawns; one printed name lets the command run"
    requirement: "GIT-02"
    verification:
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::empty_list_query_fails_closed_before_the_command"
        status: pass
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::whitespace_only_list_query_fails_closed_before_the_command"
        status: pass
      - kind: unit
        ref: "src/git/spawn.rs#git::spawn::tests::one_name_list_query_lets_the_command_run"
        status: pass
      - kind: unit
        ref: "src/git/mod.rs#git::tests::empty_output_message_names_the_query"
        status: pass
    human_judgment: false

duration: 8min
completed: 2026-10-27
---

# Phase 3 Plan 07: Re-exec harness, null stdin, hooks and signing through the helper Summary

**The unit-test binary can now re-execute itself as a child with a chosen environment and a real stdin, which proves that helper children get a null stdin. A commit and a `--no-ff` merge through `Git::run` run the user's hooks and carry good SSH signatures, and an explicit bypass removes both. A `--local-env-vars` query that prints nothing now fails closed instead of removing nothing.**

## Performance

- **Duration:** about 8 min
- **Started:** 2026-10-27T02:32:11Z
- **Completed:** 2026-10-27T02:40Z
- **Tasks:** 2 plan tasks plus 1 orchestrator addition
- **Files modified:** 3 (`src/git/testing.rs`, `src/git/spawn.rs`, `src/git/mod.rs`)

## Accomplishments

- `testing::reexec` runs `current_exe()` with `--exact <child> --include-ignored --nocapture`, `env_clear()` plus the given env, the given cwd, and a piped stdin that is fed and then closed. `marker` reads the first `WSXCHILD <key>=` line from stdout, else from stderr. Re-executing an unknown child name exits 0 with `running 0 tests` and no marker. That pins the trap that parents must never rely on the exit status alone.
- `Git::from_process(&Repo)` snapshots `std::env::vars_os()` once and sets the repo's worktree as the own worktree.
- `children_get_null_stdin`: the child gets `wsx-stdin-marker\n` on its stdin and still gets the empty blob `e69de29b...` from `git hash-object --stdin` through `Git::from_process`. The control child runs a plain `Command` with `Stdio::inherit()` and gets the marker's own blob, which the parent computes independently from a file.
- `commit_and_merge_run_hooks_and_sign`: `%G?` prints `G G`, and the hook log shows pre-commit 1, commit-msg 2 and pre-merge-commit 1. HEAD is asserted to be a two-parent merge. In the control, the test passes `-c commit.gpgsign=false` and `--no-verify` itself, and the result is `N N` with an absent hook log.
- Orchestrator addition: `removal_list` returns `Error::GitEmptyOutput { argv: ["rev-parse", "--local-env-vars"], expected: "variable names" }` when the query exits 0 but prints no name. It does not cache that result. The requested command is never spawned.

## Task Commits

1. **Task 1: re-exec harness and the null-stdin proof with its control:** `d74b947` (test)
2. **Task 2: hooks and signing through the helper, with the bypass control:** `add2589` (test)
3. **Orchestrator addition RED: failing tests for an empty list:** `c065ff4` (test)
4. **Orchestrator addition GREEN: fail closed on an empty list:** `3a829de` (feat)

## TDD Gate Compliance

- Plan tasks 1 and 2 are `type="auto"`, not tdd. The plan says why: null stdin and the argv pass-through already existed, so these tests prove existing behaviour, and each has a control that must give the opposite result.
- **Orchestrator addition RED:** `RED_EVIDENCE_OK` for `git::spawn::tests::empty_list_query_fails_closed_before_the_command`, recorded in `target/gsd/03-07-red-evidence.json` (JUnit, exit 100). Semantic assessment: the target ran alone and failed on the planned assertion, `an empty removal list must fail closed, got Ok(...)`. The fixture built and the fake git answered the list query with nothing. The requested command then ran through the real git and printed wt2's git dir. This was not a build, discovery or fixture fault. In the same RED state, the whitespace-only test also failed, and the control `one_name_list_query_lets_the_command_run` passed.
- The RED commit includes the new `Error::GitEmptyOutput` variant and its pinned message, so the tests compile. Nothing returned the variant until GREEN.
- RED `c065ff4` comes before GREEN `3a829de`. No refactor commit was needed.

## Orchestrator-requested addition (not a plan deviation)

The orchestrator asked that an empty or whitespace-only `--local-env-vars` list fail closed. 03-05's SUMMARY had recorded that limit as a silent fail-open. It is done as described above.

On the error variant, the orchestrator preferred `GitFailed` or the closest existing variant, with a new one only if none fits. None fits: `GitFailed` is documented as "exited unsuccessfully" and would render `git rev-parse --local-env-vars exited 0`. The new variant's message is pinned in its own test, `git::tests::empty_output_message_names_the_query`, so the existing message test stays unedited.

03-05's tests are unedited. Across the whole plan, the only removed lines in `spawn.rs` are the `removal_list` doc and body lines being replaced, plus the test module's `use std::ffi::OsString;`, which became `{OsStr, OsString}`.

## Verification

- `scripts/check.sh all` exited 0 before each of the three green commits, with the pinned nextest from `target/gsd-tools` on PATH. The run gave 230 tests after Task 1, 232 after Task 2 and 236 after GREEN, all passing with 44 skipped (the ignored child halves among them).
- Each new test passed alone under `--no-tests=fail -E 'test(=...)'`. Both child halves are listed by `cargo nextest list --run-ignored only`.
- Every acceptance criterion passed: `MARKER`, `pub fn reexec`, `pub fn from_process`, `Stdio::inherit()`, `gpg.ssh.allowedSignersFile`, `pre-merge-commit` and `%G?`.
- **Negative controls:**
  - Mutating the helper's child stdin to `Stdio::inherit()` makes `children_get_null_stdin` fail, with the marker's blob `7d431553...` where the empty blob was expected. The mutation was reverted.
  - The bypass test flips both signals of the signing test: `N` instead of `G`, and no hook runs instead of four.
  - The RED run above is the control for the empty-list check.
- **Deferred `GIT_DIR` item not made worse:** with a hostile `GIT_DIR` and `GIT_INDEX_FILE` in the test process, 24 of 236 tests fail. All 24 are in `git::reads` and `git::repo`, which this plan did not change, and six of them are 03-06's reftable tests. No `git::spawn` or `git::testing` test fails. The recount is now recorded in `deferred-items.md`.

## What these checks do not establish

- The signing proof covers SSH signing and three hooks. GPG signing, merge drivers and credential helpers go through the same mechanism (argv and environment passed through unchanged), but this plan does not exercise them.
- The null-stdin proof covers the helper's `run` path. A future spawn path outside `Git` would need its own proof.
- `from_process` is exercised only from a re-executed child whose environment the test chose. No test runs it inside a real hook; that is 03-08's job.
- The empty-list tests use a fake git. They prove the decision rule, not that any real git build prints an empty list.
- The hook-run counts are measured on git 2.56.0. If a different git version decided that a merge skips `commit-msg`, `commit_and_merge_run_hooks_and_sign` would fail. That is a false red, not a false green.

## Decisions Made

See `key-decisions` in the frontmatter. All three are inside this plan's remit.

## Deviations from Plan

None. Both plan tasks were executed as written. The empty-list work was requested by the orchestrator and is recorded above.

## Issues Encountered

None.

## Requirements

GIT-01 and GIT-02 are also declared by 03-08 and 03-10. As in 03-03 through 03-06, this plan does not mark them complete in REQUIREMENTS.md.

## Cleanup

- No `git-unit-*` scratch directory remains under `target/tmp`: `ls target/tmp | rg -q git-unit-` exits 1. No background process was started.
- Removed: the mutation backup, the gate logs and RED console under `target/gsd/`, and the per-test logs.
- Kept: `target/gsd/03-07-red-evidence.json` (TDD evidence) and `target/gsd-tools/` (needed by later plans). Both are gitignored.
- My scratchpad files (the pin guard script and the start-time file) are removed at the end of the run.

## Next Phase Readiness

- 03-08, 03-09 and 03-10 can use `testing::{reexec, marker, child_print, MARKER}` as they are. A hook variant can `exec` the test binary with the same arguments, and its marker comes back through git's stderr, which `marker` already searches.

## Self-Check: PASSED

- FOUND: src/git/testing.rs, src/git/spawn.rs, src/git/mod.rs, target/gsd/03-07-red-evidence.json
- FOUND: d74b947, add2589, c065ff4, 3a829de (`git rev-list --count 9e291dd..HEAD` = 4)
