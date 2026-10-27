---
phase: 03-git-adapter
plan: 08
subsystem: git
tags: [git-hooks, worktrees, local-env-vars, scrub, gix, re-exec, leak-control]
status: complete

requires:
  - phase: 03-git-adapter
    provides: "03-02/03-06: Repo::discover, from_gix, worktree/git_dir/common_dir; 03-05: the --local-env-vars removal rule, linked_worktrees; 03-07: Git::from_process, reexec, marker, child_print"
provides:
  - "Repo::index_path: the index wsx uses, which a hook's GIT_INDEX_FILE overrides (relative one resolved against the cwd), parent directory canonical"
  - "Repo::siblings: every other worktree, opened only through gix main_repo() and worktrees() -> into_repo_with_possibly_inaccessible_worktree(), sorted by canonical worktree path then git dir"
  - "Git::with_removal_disabled (cfg(test) only) and the Git::remove_local_env field, for the leak control"
  - "testing::{install_hook, git_in_env, linked_hook_commit, canonical_parent, failed_run, env_or_unset, SIBLING_VAR, UNSET}"
  - "Criterion 2 proven with real pre-commit hooks: resolution, sibling open, sibling child scrub, and a control that detects the leak"
affects: [03-09, 03-10, phase-8]

actuals:
  tokens: 7535      # chars/4 over `git diff 18e9543..HEAD -- src` (30140 chars; 3 files, 583 insertions, 4 deletions)
  tasks: 3
  commits: 6
plan_head_before: 18e95431a863fd8e59779b440d9a720344888812
plan_head_after: 889d4e68d074435d1e8d5f49d3eb7d9e3b1a7148

tech-stack:
  added: []
  patterns:
    - "Hook children: install_hook writes a pre-commit that execs the unit-test binary as a named ignored child, so git itself supplies GIT_DIR/GIT_INDEX_FILE; the parent asserts the commit's exit 0, then reads markers from git's stderr"
    - "Index paths compared as canonical parent + file name, because a hook's next-index-<pid>.lock is gone once the commit ends"

key-files:
  created: []
  modified:
    - src/git/repo.rs
    - src/git/spawn.rs
    - src/git/testing.rs

key-decisions:
  - "with_removal_disabled bypasses the whole removal branch in command(), so the leak control also skips the --local-env-vars query; the control needs only the unscrubbed child"
  - "The sibling self-match is canonical worktree equality, or git-dir equality when this repository has no worktree; main_repo() from the main worktree is a clone carrying the hook's index, and it is always the dropped self"
  - "Hook test plumbing (linked_hook_commit, canonical_parent) lives in testing.rs, shared by repo.rs and spawn.rs, instead of being duplicated per module"

patterns-established:
  - "Mutation controls for every green-first test: removal off by default turns sibling_child_env_scrubbed red (wt1's git dir instead of wt2's); scrubbing the own worktree turns own_worktree_child_keeps_hook_index red (wt1/index instead of next-index-*); deleting the sort turns siblings_are_sorted_by_worktree_path red (main before aa)"

requirements-completed: []

duration: 8min
completed: 2026-10-27
---

# Phase 3 Plan 08: Hook-environment resolution, sibling worktrees and the real-spawn scrub proof Summary

**Inside a real pre-commit hook, wsx resolves its own worktree, common dir and the hook's index, opens sibling worktrees through gix's worktree list with their own git dirs and indexes, and a git child aimed at a sibling is scrubbed, with a control that shows the leak when the removal is off.**

## Performance

- **Duration:** about 8 min
- **Started:** 2026-10-27T02:42:57Z
- **Completed:** 2026-10-27T02:51:15Z
- **Tasks:** 3 (plus one follow-up commit, see Deviations)
- **Files modified:** 3

## Accomplishments

- `Repo::index_path` follows gix's `index_path()`, so inside wt1's partial-commit pre-commit it is the hook's `next-index-<pid>.lock`. In the main worktree's hook (`GIT_DIR` unset, `GIT_INDEX_FILE=.git/index` relative), it is `<main>/.git/index`.
- `Repo::siblings` returns the main worktree and wt2 from inside wt1's hook, each with its own git dir and index. The control shows that `Repo::discover(<wt2>)` in the same hook resolves wt1's git dir.
- From inside wt1's hook, a child aimed at wt2 through `Git::from_process` reports wt2's git dir and index. With removal disabled, the same helper reports wt1's git dir and the hook's index. A child aimed at wt1 keeps the `next-index-*` index.

## Task Commits

1. **Task 1 RED: hook resolution tests:** `da32c22` (test)
2. **Task 1 GREEN: resolve the hook's index:** `736e38f` (feat)
3. **Task 2 RED: sibling worktree tests:** `06f2df1` (test)
4. **Task 2 GREEN: open siblings through gix's worktree list:** `2b311a1` (feat)
5. **Task 3: sibling child scrub and leak control:** `1654b73` (test)
6. **Follow-up: sibling order test in a hermetic re-exec child:** `889d4e6` (test)

## TDD Gate Compliance

- **Task 1 RED:** `RED_EVIDENCE_OK` for `git::repo::tests::hook_resolves_linked_worktree`. Record: `target/gsd/03-08-red-evidence.json` (JUnit, exit 100). Semantic assessment: the target ran, and the hook child ran and printed its markers. The worktree, git_dir and common_dir assertions passed. The test then failed on the planned index assertion: left `<wt1 git dir>/index`, right `<wt1 git dir>/next-index-<pid>.lock`.
- **Task 2 RED:** `RED_EVIDENCE_OK` for `git::repo::tests::siblings_in_a_hook_use_their_own_git_dirs`. Record: `target/gsd/03-08-red-evidence-2.json` (JUnit, exit 100). Semantic assessment: the test failed on its first assertion, the count (left `"0"`, right `"2"`). In the same RED state, the control `discovering_a_sibling_inside_a_hook_follows_the_hook_env` passed.
- Each GREEN commit was gated on `scripts/check.sh all` exiting 0. The final run printed `244 tests run: 244 passed, 48 skipped` and `==> check.sh: all OK`.
- Task 3 is not TDD (as planned). Its three tests passed on the first run, so each was checked by a mutation instead (see patterns-established).

## Files Created/Modified

- `src/git/repo.rs`: `index_path` field and accessor, `canonical_parent`, `siblings`, `list_failed`; 5 tests and 3 ignored child halves
- `src/git/spawn.rs`: `remove_local_env` field, `#[cfg(test)] with_removal_disabled`; 3 tests and 1 ignored child half
- `src/git/testing.rs`: `install_hook`, `git_in_env`, `linked_hook_commit`, `canonical_parent`, `failed_run`, `env_or_unset`, `SIBLING_VAR`, `UNSET`

## Decisions Made

See `key-decisions` in the frontmatter. All three fall within the plan's instructions.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing test] The planned ordering proof could not detect a missing sort**
- **Found during:** Task 2 GREEN
- **Issue:** The planned assertion of the ordering edge is `siblings_in_a_hook_use_their_own_git_dirs`. In its fixture, gix's own order (main first, then linked worktrees by git dir) already equals path order. When the sort was deleted, that test stayed green (checked).
- **Fix:** Added `git::repo::tests::siblings_are_sorted_by_worktree_path`. A linked worktree at `<root>/aa` sorts before `<root>/main`, so the expected order from wt2 is aa, main, wt1. With the sort deleted, this test goes red (main before aa).
- **Files modified:** src/git/repo.rs
- **Commit:** `2b311a1`

**2. [Rule 1 - Bug] The new ordering test added a false red under a caller `GIT_DIR`**
- **Found during:** a recount of the GIT_DIR deferred item after Task 3
- **Issue:** The test ran `Repo::discover` in the test process. With this checkout's `GIT_DIR`/`GIT_INDEX_FILE` exported, the suite went from 24 failures (03-07's recount) to 25.
- **Fix:** The ordering test now re-executes the sibling child with the scratch environment (`reexec` starts from `env_clear()`). The recount is back to `244 tests run: 220 passed, 24 failed, 48 skipped`, and none of this plan's tests are among the failures.
- **Files modified:** src/git/repo.rs
- **Commit:** `889d4e6`

**3. Extra shared test support**
- `linked_hook_commit`, `canonical_parent`, `failed_run`, `env_or_unset`, `SIBLING_VAR` and `UNSET` were added to `testing.rs` beyond the two functions the plan names. Six parents use one hook fixture, so sharing it avoids two diverging copies. The plan's file list did not change.

## Limits of what this proves

- **Only pre-commit was exercised:** a partial commit in a linked worktree, and a staged commit in the main worktree. RESEARCH observed the same variables in commit-msg and post-commit, and no `GIT_INDEX_FILE` in reference-transaction. None of those hooks are run here.
- **The main-worktree test cannot catch an ignored `GIT_INDEX_FILE`.** `hook_resolves_main_worktree_with_relative_index` also passed against the RED stub, because `<git dir>/index` equals the expected path. It proves that git really exported a relative `.git/index` with `GIT_DIR` unset, and that wsx resolves the right worktree, git dir and index there. Only the linked-worktree test proves that the variable is honoured.
- **Fail-closed sibling paths have no tests.** These are a sibling whose directory is gone (`Error::Io`) and a gix listing failure (`Error::Gix`).
- **Unverified edge:** a sibling whose `gitdir` file is unreadable. gix then opens it with no worktree base. It may then come back as a worktree-less sibling instead of being refused. Prunable worktrees belong to phase 8.
- Tested against git 2.56.0 on this host only. Not run in the pinned container.

## Issues Encountered

None.

## Cleanup

- No `git-unit-hook-*` or `git-unit-sibling*` scratch directories remain under `target/tmp`. TempDirs are removed on drop, including in the panicking mutation runs.
- The mutation backups (`target/gsd/*.bak`), gate logs and `.console` captures have been removed. Only the two RED records remain in `target/gsd/`, which is gitignored.
- No background processes were started.

## Next Phase Readiness

- 03-09/03-10 can use `Repo::siblings` and `index_path`, plus the hook fixture `testing::linked_hook_commit`.
- The deferred `GIT_DIR`-in-test-env item (assigned to 03-10) still stands at 24 failures, as at 03-07.
- GIT-02 is left Pending, because 03-10 also carries it.

## Self-Check: PASSED
