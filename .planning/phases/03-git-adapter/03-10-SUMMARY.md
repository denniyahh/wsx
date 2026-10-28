---
phase: 03-git-adapter
plan: 10
subsystem: git
tags: [ceiling, GIT_CEILING_DIRECTORIES, refStorage, test-hermeticity, phase-gate, container, gix]
status: complete

requires:
  - phase: 03-git-adapter
    provides: "03-01: open_gix with match_ceiling_dir_or_error=false; 03-06: refuse_ref_storage; 03-07/03-08: reexec, marker, child_print, git_in_env; 03-09: refuse_untrusted, ForeignOwned"
provides:
  - "refuse_at_ceiling in Repo::discover: NotARepo where GIT_CEILING_DIRECTORIES hides the repository from git"
  - "testing::rerun_scrubbed: a test that discovers in its own process re-runs without the runner's repository-local git variables"
  - "ref_storage reads extensions.refStorage from the repository's own config (gix Source::Local) only"
  - "Phase gate evidence: host all, host deps and container all green at ae69388"
affects: [phase-3-verification, phase-5]

actuals:
  tokens: 9255      # chars/4 over `git diff 7f51a3d..ae69388 -- src` (3 files, 648 insertions, 3 deletions)
  tasks: 4          # 2 plan tasks + 2 orchestrator additions
  commits: 7
plan_head_before: 7f51a3d59796d14cdbdf2e27726cfebf41f4c154
plan_head_after: 9ce3dfd462e4acb890f9ccab0088bf25433b603e

tech-stack:
  added: []
  patterns:
    - "Self re-execution by thread name: libtest names each test's thread after its full path, so a test can re-run itself as `--exact <name>` without a hand-typed name"
    - "gix config by source: config_snapshot().plumbing().string_filter(key, |meta| meta.source == Source::Local)"

key-files:
  created: []
  modified:
    - src/git/repo.rs
    - src/git/testing.rs
    - src/git/reads.rs
    - .planning/STATE.md
    - .planning/phases/03-git-adapter/deferred-items.md

key-decisions:
  - "The ceiling check compares canonical ceiling entries with the canonical worktree. It is skipped when GIT_DIR is set and for a bare repository, as the plan specifies."
  - "Test hermeticity is fixed in the tests, not in scripts/check.sh: rerun_scrubbed re-runs a dirty-environment test as a child without the variables `git rev-parse --local-env-vars` lists. A direct `cargo nextest run` is covered too, and production discovery is unchanged."
  - "extensions.refStorage is read from Source::Local only. git 2.56.0 ignores the key in global config, in config.worktree (worktreeConfig=true) and in GIT_CONFIG_COUNT (all three measured), so wsx ignores it in all three."

patterns-established:
  - "A suite-level regression test re-runs whole test modules under a hazardous runner environment and asserts a non-zero pass count with zero failures"

requirements-completed: [GIT-02, GIT-01, GIT-03, GIT-04]

coverage:
  - deliverable: "GIT_CEILING_DIRECTORIES hides a repository from wsx exactly where it hides it from git"
    human_judgment: false
    verification:
      - kind: test
        ref: "src/git/repo.rs#ceiling_at_the_repository_root_hides_it_like_git"
        status: pass
      - kind: test
        ref: "src/git/repo.rs#ceiling_at_the_repository_root_hides_it_two_levels_down"
        status: pass
      - kind: test
        ref: "src/git/repo.rs#ceiling_above_the_repository_lets_wsx_and_git_find_it"
        status: pass
      - kind: test
        ref: "src/git/repo.rs#ceiling_equal_to_the_start_directory_does_not_stop_discovery"
        status: pass
      - kind: test
        ref: "src/git/repo.rs#starting_in_a_ceiling_that_holds_the_repository_opens_it"
        status: pass
  - deliverable: "No unit test's result depends on repository-local git variables in the runner's environment"
    human_judgment: false
    verification:
      - kind: test
        ref: "src/git/testing.rs#discovering_tests_pass_with_a_runner_git_dir"
        status: pass
      - kind: test
        ref: "src/git/testing.rs#rerun_scrubbed_discovers_the_fixture_under_a_runner_git_dir"
        status: pass
      - kind: test
        ref: "src/git/testing.rs#a_runner_git_dir_redirects_unscrubbed_discovery"
        status: pass
      - kind: command
        ref: "GIT_DIR=<this checkout's git dir> GIT_INDEX_FILE=<its index> scripts/check.sh test"
        status: pass
  - deliverable: "extensions.refStorage decides the refusal only from repository config"
    human_judgment: false
    verification:
      - kind: test
        ref: "src/git/repo.rs#ref_storage_in_global_config_does_not_refuse_a_files_repo"
        status: pass
      - kind: test
        ref: "src/git/repo.rs#ref_storage_in_worktree_config_does_not_refuse_a_files_repo"
        status: pass
      - kind: test
        ref: "src/git/repo.rs#ref_storage_in_environment_config_does_not_refuse_a_files_repo"
        status: pass
      - kind: test
        ref: "src/git/repo.rs#reftable_repo_is_refused"
        status: pass
  - deliverable: "Phase gate green on the host and in the pinned container"
    human_judgment: false
    verification:
      - kind: command
        ref: "scripts/check.sh all"
        status: pass
      - kind: command
        ref: "scripts/check.sh deps"
        status: pass
      - kind: command
        ref: "scripts/check-in-container.sh all"
        status: pass

duration: 17min
completed: 2026-10-27
---

# Phase 3 Plan 10: Ceiling parity, test hermeticity, refStorage source and phase gate Summary

**wsx now refuses a repository as not-a-repo exactly where `GIT_CEILING_DIRECTORIES` hides it from git. The discovering unit tests no longer follow a runner's `GIT_DIR`. `extensions.refStorage` counts only from the repository's own config, as in git. The phase gate passes on the host and in the pinned container, where the run is root.**

## Performance

- **Duration:** about 17 min
- **Started:** 2026-10-27T03:01:44Z
- **Completed:** 2026-10-27T03:19Z
- **Tasks:** 2 plan tasks and 2 orchestrator-requested additions
- **Files modified:** 3 source files, 2 planning files

## Accomplishments

- `refuse_at_ceiling` is the first check in `discover`, before `refuse_untrusted`. It refuses when a canonical ceiling entry equals the found worktree and the start is not that worktree. Its doc carries RESEARCH's table.
- `testing::rerun_scrubbed()` is called first in all 26 tests that discover in the test process (16 in `git::reads`, 10 in `git::repo`). With a clean environment it costs one `git rev-parse --local-env-vars` spawn and the test runs in place.
- `ref_storage` filters on gix's `Source::Local`, the common dir's `config`. That covers both `refuse_ref_storage` and the unsupported-backend refusal.
- STATE.md no longer lists the gix 0.89 concern. The phase 5 reftable obligation is still there.

## Task Commits

1. **Task 1 RED: ceiling parity tests:** `22646b3` (test)
2. **Task 1 GREEN: refuse_at_ceiling:** `31697de` (feat)
3. **Addition 1 RED: suite run under a runner GIT_DIR:** `99d6a5d` (test)
4. **Addition 1 GREEN: rerun_scrubbed and its 26 call sites:** `142a095` (feat)
5. **Addition 2 RED: ref-storage config source tests:** `aaade17` (test)
6. **Addition 2 GREEN: Source::Local only:** `ae69388` (fix)
7. **Task 2: resolve the gix concern:** `9ce3dfd` (docs)

## TDD Gate Compliance

Every RED commit comes before its GREEN commit. Addition 2's GREEN is a `fix(03-10)` because it corrects a 03-06 defect. The plan-level gate also has two `feat(03-10)` commits after RED commits.

- **Task 1 RED:** `RED_EVIDENCE_OK` for `git::repo::tests::ceiling_at_the_repository_root_hides_it_like_git` (`target/gsd/03-10-red-evidence.json`, JUnit, exit 100). Semantic assessment: the test ran and failed on the planned result assertion (left `opened`, right `not_a_repo`). git's exit-128 assertion had passed first. In the same RED state, `..._two_levels_down` failed the same way and the three controls passed.
- **Addition 1 RED:** `RED_EVIDENCE_OK` for `git::testing::discovering_tests_pass_with_a_runner_git_dir` (`target/gsd/03-10-red-evidence-2.json`). The re-run suites reported `(passed, failed) = (15, 24)`. Those are the same 24 test names as the shell control below, compared with `diff`.
- **Addition 2 RED:** `RED_EVIDENCE_OK` for `git::repo::tests::ref_storage_in_global_config_does_not_refuse_a_files_repo` (`target/gsd/03-10-red-evidence-3.json`). git read `files` (asserted first) and the child printed `result=reftable`. The worktree-config and `GIT_CONFIG_COUNT` variants failed the same way.
- **GREEN gates:** each was committed only after `scripts/check.sh all` exited 0: 253, then 256, then 259 tests passed.

## Orchestrator-requested additions

**1. Test hermeticity against inherited repository-local git variables** (the deferred item, now resolved).
- Control, at `31697de` before the fix: `GIT_DIR=$(git rev-parse --absolute-git-dir) GIT_INDEX_FILE=<that>/index scripts/check.sh test` gave `253 tests run: 229 passed, 24 failed, 50 skipped` (15 `git::reads`, 9 `git::repo`).
- After the fix, the same command gave `256 tests run: 256 passed, 52 skipped`. A clean run gave the same count.
- Under the dirty environment, 26 tests reported `WSXCHILD rerun=`. A clean run had 0.
- Mutation: removing the guard from `sha256_repo_opens` makes the suite test fail with `(38, 1)`.
- Control pair: with a decoy `GIT_DIR`, a child that does not call the guard discovers the decoy, and a child that does call it discovers its fixture.
- No `scripts/check.sh` scrub was added. The fix is in the tests, so a defence-in-depth scrub would add nothing.

**2. `extensions.refStorage` read from the repository's own config only.**
- Measured with git 2.56.0 on a files repository with `extensions.refStorage=reftable` set in three places. In global config, `config.worktree` (v1, `worktreeConfig=true`) and `GIT_CONFIG_COUNT`, git reads `files`. Set in the local config of a v0 repository, git exits 128 (`repo version is 0, but v1-only extension found`).
- So only `Source::Local` decides the refusal. Worktree config is excluded because git ignores it there.
- Controls: the existing reftable tests still pass (`reftable_repo_is_refused`, `empty_reftable_repo_is_refused`, `miscased_...`, `explicit_files_...`, `files_backend_twin_opens`, `reftable_ref_decode_maps_to_the_refusal`). Changing the filter to `Source::Worktree` makes `reftable_repo_is_refused` fail.

## Phase gate (at `ae69388`)

| Gate | Exit | Result |
|------|------|--------|
| `scripts/check.sh all` (host, pinned tools via `target/gsd-tools`) | 0 | `259 tests run: 259 passed, 53 skipped`; `==> check.sh: all OK` |
| `scripts/check.sh deps` (host) | 0 | `advisories ok, bans ok, licenses ok, sources ok`; `==> check.sh: deps OK` |
| `scripts/check-in-container.sh all` (rootless Docker 29.9.2, `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`) | 0 | `259 tests run: 259 passed, 53 skipped`; `==> check.sh: all OK` |

The container run is the one that exercised the root foreign-owner path. The container process is uid 0: the image user is `root`, `id` prints `uid=0(root)`, and the script passes no `--user`. `ForeignOwned::new` takes `chown -R 65534:65534` when uid is 0. `foreign_owner_refused`, `safe_directory_in_global_config_opens_a_foreign_repo`, `command_line_safe_directory_does_not_trust_a_foreign_repo` and `foreign_guard_changes_and_restores_the_owner` all show `PASS` in its log. The branch taken is inferred from uid 0 and the code; no log line names it.

## Files Created/Modified

- `src/git/repo.rs`: `refuse_at_ceiling` and its call; `ref_storage` source filter; five ceiling tests, three ref-storage source tests, the shared `print_discovery`, children `ceiling_child` and `ref_storage_child`; guard calls in 10 tests
- `src/git/testing.rs`: `rerun_scrubbed`, `local_env_vars`, `RERUN_VAR`; the suite test with its helpers; the control pair and two children
- `src/git/reads.rs`: guard calls in all 16 tests
- `.planning/STATE.md`: gix concern resolved via `state.resolve-blocker`
- `.planning/phases/03-git-adapter/deferred-items.md`: item marked resolved; one new item logged

## Decisions Made

See `key-decisions`. All are within the plan and the orchestrator's instructions.

## Deviations from Plan

1. **[Orchestrator addition] Test hermeticity and the refStorage source:** documented above. They added 4 commits and touched `src/git/testing.rs` and `src/git/reads.rs`, which are outside the plan's `files_modified`.
2. **[Acceptance criterion overtaken by close-out] `git log -1 --format=%s -- .planning/STATE.md`:** it printed `docs(03-10): resolve the gix method-name concern` right after `9ce3dfd`. The GSD close-out commit then touches STATE.md again, so after close-out the latest STATE.md subject is the close-out commit. The resolve commit stays in history.
3. **`tdd-red-evidence --raw` prints JSON, not a bare `RED_EVIDENCE_OK`:** its `verdict` field was `RED_EVIDENCE_OK` all three times.

**Total deviations:** 0 auto-fixed bugs. 2 additions were requested by the orchestrator. **Impact:** wider test coverage; production semantics change only for refStorage, where wsx now matches git.

## Limits of what this proves

- **Ceiling:** one git version (2.56.0) and only `rev-parse --absolute-git-dir` as the git side. A bare repository is not checked; git hides a bare repository the same way gix does not, and that divergence remains. With `GIT_WORK_TREE` set and `GIT_DIR` unset, the comparison uses `GIT_WORK_TREE`, not the directory where `.git` was found (untested). git's rule that an empty entry turns off symlink resolution for later entries is not modelled.
- **Hermeticity:** only the repository-local variables are scrubbed. The 26 in-process tests still read the runner's global and system git config; this is logged as a new deferred item. The suite test covers `git::reads` and `git::repo` only. A future in-process discovering test elsewhere is not covered unless it is added to `DISCOVERING_SUITES`.
- **refStorage:** `-c extensions.refStorage` on git's command line and a value from an `[include]` in local config were not tested. gix may attribute included sections to `Source::Local`.
- **Container:** this proves the root path under rootless Docker on this host. A GitHub runner was not used (RESEARCH A1 stays unverified until CI runs after the owner pushes).
- **Gate counts:** each was one run. A single pass of 259 tests is not evidence against flakiness.

## Issues Encountered

None.

## Cleanup

- `target/tmp`: no `git-unit-*` directory and no file owned by another uid after the gates. Both checks were shown to catch a planted case: a `git-unit-control-03-10` directory, and a file chowned to a subordinate uid with `unshare`. Both plants were removed.
- No file under the worktree is owned by another uid (`find . ! -user 1000` printed nothing).
- The probe repository `target/tmp/probe-03-10-refstorage`, the mutation backups and `target/gsd/scratch-03-10` were removed. The gate logs and the pin script are in the session scratchpad. `target/gsd/03-10-red-evidence{,-2,-3}.json` (gitignored) are kept as RED records.
- The only background process was the container gate. It exited 0, and `docker run --rm` removed its container.
- `.planning/state.json` was untracked before this plan started and was left alone.

## Next Phase Readiness

- Every phase 3 plan has a SUMMARY, so the phase is ready for `/gsd-verify-work`.
- GIT-01..04 are marked complete per the shared-ID gate (`requirements.ready-ids` reported all four ready). Phase verification is what confirms them.

## Self-Check: PASSED
