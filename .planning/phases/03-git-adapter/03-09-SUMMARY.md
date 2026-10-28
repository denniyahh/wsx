---
phase: 03-git-adapter
plan: 09
subsystem: git
tags: [trust, safe-directory, foreign-owner, user-namespace, unshare, gix, re-exec]
status: complete

requires:
  - phase: 03-git-adapter
    provides: "03-01: Error::ForeignOwner and open_gix's no-bail trust mapping; 03-06: refuse_ref_storage, refusal_path; 03-07/03-08: reexec, marker, child_print, failed_run, git_in_env"
provides:
  - "testing::ForeignOwned: makes a fixture really owned by another uid (root: chown 65534; user: /usr/bin/unshare user namespace, inner uid 1), restores it, Drop restores, panics naming the missing mechanism"
  - "refuse_untrusted in Repo::discover: Error::ForeignOwner unless gix's git_dir_trust() is Trust::Full, before refuse_ref_storage"
  - "Criterion 4 proven against a really foreign-owned repository, matching git's refusal and fix line, with the global safe.directory control and the pinned GIT_CONFIG_COUNT divergence"
affects: [03-10]

actuals:
  tokens: 4599      # chars/4 over `git diff a19fcce..HEAD -- src` (18395 chars; 2 files, 424 insertions, 1 deletion)
  tasks: 2
  commits: 3
plan_head_before: a19fcce844227b9ecd4aa6becd097773a9a9cb80
plan_head_after: 68780e860df639c512de5b05eb521f7dce60168d

tech-stack:
  added: []
  patterns:
    - "Foreign-owner fixtures: chmod -R go+rX, then chown to another uid through a guard bound after the Scratch, so it drops first and gives the tree back before the TempDir is removed"
    - "Trust is read as the typed gix::sec::Trust, never matched from gix's bail message"

key-files:
  created: []
  modified:
    - src/git/testing.rs
    - src/git/repo.rs

key-decisions:
  - "ForeignOwner carries the canonical worktree (git dir for a bare repo), via the existing refusal_path; git prints the same path, so the fix lines are equal without changing which path is carried"
  - "ForeignOwned runs chmod/chown/unshare through testing::command with only PATH=/usr/local/bin:/usr/bin:/bin and LC_ALL=C, cwd /; the guard is built before the chown so a partial chown is also restored"
  - "The missing-mechanism diagnosis checks /usr/bin/unshare, /usr/bin/newuidmap, /usr/bin/newgidmap and an /etc/subuid and /etc/subgid entry (by uid or by `id -un`), and says so when all are present (user namespaces restricted)"

patterns-established:
  - "Mutation controls for the guard: a missing unshare path fails with `missing: /usr/bin/unshare-missing`; a no-op chown (inner 0:0) fails with `still owned by uid 1000 after the chown`"

requirements-completed: []

duration: 5min
completed: 2026-10-27
---

# Phase 3 Plan 09: Foreign-owner refusal Summary

**A repository really owned by another uid is refused by `Repo::discover` as `Error::ForeignOwner` exactly where git refuses it (exit 128, dubious ownership), carrying git's own `git config --global --add safe.directory <path>` line; a global `safe.directory` entry opens it for both, and `safe.directory` given only through `GIT_CONFIG_COUNT` opens it for git while wsx still refuses.**

## Performance

- **Duration:** about 5 min
- **Started:** 2026-10-27T02:53:42Z
- **Completed:** 2026-10-27T02:58:25Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `testing::ForeignOwned` makes a fixture really foreign-owned and always gives it back. Its self-test `git::testing::foreign_guard_changes_and_restores_the_owner` is not ignored and runs in every gate.
- `refuse_untrusted` runs in `discover` right after `open_gix` and before `refuse_ref_storage`, so no configuration of an untrusted repository is read first. Its doc records the two accepted divergences: `-c`/`GIT_CONFIG_PARAMETERS`/`GIT_CONFIG_COUNT` `safe.directory` is ignored, and gix honours `SUDO_UID` for any euid.
- Three parents plus one ignored child half: `foreign_owner_refused`, `safe_directory_in_global_config_opens_a_foreign_repo`, `command_line_safe_directory_does_not_trust_a_foreign_repo`, child `foreign_owner_child`. Each calls `restore()` and asserts `Ok`.
- The fix line matched git's on the first GREEN run, so `ForeignOwner` keeps carrying the canonical worktree. git prints the worktree path too.

## Task Commits

1. **Task 1: ForeignOwned guard and self-test:** `c666b8d` (test)
2. **Task 2 RED: failing foreign-owner refusal tests:** `bdb05ed` (test)
3. **Task 2 GREEN: refuse repositories owned by another user:** `68780e8` (feat)

## TDD Gate Compliance

- **Task 2 RED:** `RED_EVIDENCE_OK` for `git::repo::tests::foreign_owner_refused`. Record: `target/gsd/03-09-red-evidence.json` (JUnit, exit 100). Semantic assessment: the target ran. The fixture was really foreign-owned: git's exit 128 and `detected dubious ownership` assertions passed first. The re-executed child ran and printed its markers. The test failed on the planned result assertion: left `"opened"`, right `"foreign_owner"`. In the same RED state, the control `safe_directory_in_global_config_opens_a_foreign_repo` passed, and `command_line_safe_directory_does_not_trust_a_foreign_repo` failed the same way (`opened`).
- **GREEN:** gated on `scripts/check.sh all` exiting 0: `248 tests run: 248 passed, 49 skipped`, `==> check.sh: all OK`.
- Task 1 is not TDD, as planned. Its failing direction was checked by mutation instead (see patterns-established).

## Mechanism used on this host

The **user-namespace path** (`/usr/bin/unshare --map-root-user --map-users=auto --map-groups=auto chown -R 1:1`): the test runs as uid 1000, so inner uid 1 maps to the first subordinate uid. The root path (`chown -R 65534:65534`) did not run here.

## Files Created/Modified

- `src/git/testing.rs`: `ForeignOwned` (`new`, `restore`, `Drop`), private `owner`, `unshare_chown`, `run_tool`, `missing_mechanism`, and constants `UNSHARE`, `UNSHARE_ARGS`, `INNER_FOREIGN`, `ROOT_FOREIGN`; the self-test
- `src/git/repo.rs`: `refuse_untrusted` and its call in `discover`; test helpers `foreign_repo`, `discover_in_child`, `line_with`, `child_result`, `REPO_VAR`, `GIT_FIX`; 3 tests and 1 ignored child half

## Decisions Made

See `key-decisions` in the frontmatter. All fall within the plan's instructions.

## Deviations from Plan

None. The plan was executed as written.

## Limits of what this proves

- **Only the user-namespace path ran.** The root path runs in 03-10's `scripts/check-in-container.sh all`. GitHub's job container is not exercised by any agent run (RESEARCH A1: that it can `chown` to 65534 is unverified until a CI run after push. If it cannot, the test fails loudly there by design).
- **One ownership shape was tested:** worktree and git dir both owned by one foreign uid. Split ownership (worktree and git dir owned by different uids) is not tested, as planned. The GIT-04 edge-coverage item stays unresolved.
- **`SUDO_UID` is documented, not tested.** The test children start from `env_clear()`, so it never reaches them.
- **Only `GIT_CONFIG_COUNT` was tested** for the command-line divergence, not `-c` or `GIT_CONFIG_PARAMETERS`. RESEARCH observed all three refused by gix.
- **Only git 2.56.0** on this host was tested, and only `git rev-parse --absolute-git-dir` as the git side.
- **The fix-line equality covers a path without spaces or quoting.** git's and wsx's rendering of an unusual path was not compared.

## Issues Encountered

None.

## Deferred item recount (`GIT_DIR` in the test environment, assigned to 03-10)

With this checkout's `GIT_DIR` and `GIT_INDEX_FILE` exported, `scripts/check.sh test` gives `248 tests run: 224 passed, 24 failed, 49 skipped`. That is the same 24 failures as at 03-07 and 03-08, and none of this plan's tests are among them. The new tests discover only in re-executed children, and every git call uses the scratch environment.

## Cleanup

- No file under `target/` is owned by another uid (`find target -not -user "$(id -un)"` printed nothing), including after the two panicking mutation runs.
- No `git-unit-foreign*` scratch directories remain under `target/tmp`.
- Mutation backups, gate logs and the RED-record builder lived in the session scratchpad and have been removed. Only `target/gsd/03-09-red-evidence.json` (gitignored) remains, kept as the RED record.
- No background processes were started.

## Next Phase Readiness

- 03-10 adds the ceiling check before `refuse_untrusted` in `discover`, and its container gate runs the root path of `ForeignOwned`.
- GIT-04 is left Pending, because 03-10 also carries it.

## Self-Check: PASSED
