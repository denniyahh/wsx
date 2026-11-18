---
phase: 05-divergence-check-and-command-contract
plan: 03
subsystem: git
tags: [git, cat-file, ls-tree, reflog, fetch-head, core-hookspath, nul-framing]

requires:
  - phase: 05-divergence-check-and-command-contract
    provides: "05-01's read order (model::resolve before any tree or staleness read)"
provides:
  - "Repo::lookup_paths, Repo::list_tree, Repo::read_blobs (src/git/trees.rs) with TreeEntry, ListedEntry and the MODE_* constants"
  - "Repo::hooks_path: git's effective core.hooksPath without GIT_CONFIG"
  - "Repo::ref_moved_at and Repo::fetched_at (src/git/staleness.rs), the two D-12 times"
affects: [05-04, 05-06, 05-07]

actuals:
  tokens: 14334
  tasks: 2
  commits: 4
plan_head_before: ea0c6d203c910c88e37e08aa1ad7f2d2c69b0ff3
plan_head_after: 4ec76a82c3e7a35c423d568fab32f2cae9c2f870

tech-stack:
  added: []
  patterns:
    - "Tree paths travel only on stdin as NUL-framed <id>:<path> requests (cat-file --batch-check -Z), never argv or pathspecs"
    - "Each test asserts git's own answer (batch-check text, rev-parse ids, --git-path, reflog exists) before wsx's"

key-files:
  created:
    - src/git/trees.rs
    - src/git/staleness.rs
  modified:
    - src/git/config.rs
    - src/git/spawn.rs
    - src/git/objects.rs
    - src/git/mod.rs
    - src/git/testing.rs

key-decisions:
  - "A gitlink lookup is git's '<id> submodule' answer (its commit absent), read as MODE_GITLINK; the test pins that text"
  - "A lookup answer is None only when it is exactly '<its request> missing', so a path holding ' missing' cannot be misread"
  - "parse_reflog_selector refuses more than one line, so a signature line that slipped onto stdout is GitBadOutput, never a time"
  - "No pub use of the trees types yet: 05-06, the first caller outside src/git, adds exactly the names it uses"

patterns-established:
  - "fetch_head_paths: <common>/FETCH_HEAD then <common>/worktrees/<id>/FETCH_HEAD sorted by id; NotFound skipped"

requirements-completed: [DIV-02, DIV-03, DIV-04]

coverage:
  - id: D1
    description: "Tree entry lookups by path at a commit, in request order, NUL-framed, a newline path included"
    requirement: DIV-02
    verification:
      - kind: integration
        ref: "src/git/trees.rs#git::trees::tests::lookup_paths_answers_each_request_in_order"
        status: pass
      - kind: integration
        ref: "src/git/trees.rs#git::trees::tests::lookup_paths_reads_a_path_holding_a_newline"
        status: pass
      - kind: unit
        ref: "src/git/trees.rs#git::trees::tests::parse_lookup_refuses_fewer_answers_than_requests"
        status: pass
      - kind: unit
        ref: "src/git/trees.rs#git::trees::tests::parse_lookup_refuses_a_mode_that_is_not_octal"
        status: pass
    human_judgment: false
  - id: D2
    description: "Whole-tree listing with modes, directories included; malformed output is GitBadOutput"
    requirement: DIV-03
    verification:
      - kind: integration
        ref: "src/git/trees.rs#git::trees::tests::list_tree_lists_every_entry_with_its_mode"
        status: pass
      - kind: unit
        ref: "src/git/trees.rs#git::trees::tests::parse_listing_refuses_a_record_without_a_tab"
        status: pass
      - kind: unit
        ref: "src/git/trees.rs#git::trees::tests::parse_listing_refuses_output_not_ending_in_nul"
        status: pass
    human_judgment: false
  - id: D3
    description: "Blob reads by id in order; a missing id is ObjectMissing"
    requirement: DIV-02
    verification:
      - kind: integration
        ref: "src/git/trees.rs#git::trees::tests::read_blobs_returns_each_blob_in_order"
        status: pass
    human_judgment: false
  - id: D4
    description: "git's effective core.hooksPath: global seen, ~ expanded as git does, GIT_CONFIG ignored, newline kept"
    requirement: DIV-03
    verification:
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::hooks_path_is_the_effective_value"
        status: pass
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::hooks_path_expands_home_as_git_does"
        status: pass
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::hooks_path_ignores_a_git_config_file"
        status: pass
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::hooks_path_keeps_a_newline_in_the_value"
        status: pass
    human_judgment: false
  - id: D5
    description: "ref_moved_at: newest reflog time, None without a reflog, immune to log.showSignature"
    requirement: DIV-04
    verification:
      - kind: integration
        ref: "src/git/staleness.rs#git::staleness::tests::ref_moved_at_is_none_after_a_clone"
        status: pass
      - kind: integration
        ref: "src/git/staleness.rs#git::staleness::tests::ref_moved_at_is_the_newest_entry"
        status: pass
      - kind: integration
        ref: "src/git/staleness.rs#git::staleness::tests::ref_moved_at_ignores_log_show_signature"
        status: pass
      - kind: unit
        ref: "src/git/staleness.rs#git::staleness::tests::parse_reflog_selector_reads_the_time_after_the_last_at_brace"
        status: pass
      - kind: unit
        ref: "src/git/staleness.rs#git::staleness::tests::parse_reflog_selector_refuses_other_output"
        status: pass
    human_judgment: false
  - id: D6
    description: "fetched_at: newest FETCH_HEAD mtime across the main and every linked worktree"
    requirement: DIV-04
    verification:
      - kind: integration
        ref: "src/git/staleness.rs#git::staleness::tests::fetched_at_is_none_before_any_fetch"
        status: pass
      - kind: integration
        ref: "src/git/staleness.rs#git::staleness::tests::fetched_at_is_the_fetch_head_mtime"
        status: pass
      - kind: integration
        ref: "src/git/staleness.rs#git::staleness::tests::fetch_head_paths_include_each_worktrees_git_path"
        status: pass
      - kind: integration
        ref: "src/git/staleness.rs#git::staleness::tests::fetched_at_reads_a_fetch_in_the_main_worktree_from_a_linked_one"
        status: pass
      - kind: integration
        ref: "src/git/staleness.rs#git::staleness::tests::fetched_at_is_the_newest_across_worktrees"
        status: pass
    human_judgment: false
  - id: D7
    description: "DISCOVERING_SUITES lists git::staleness::tests:: and git::trees::tests::, and the hook-environment rerun passes"
    verification:
      - kind: unit
        ref: "src/git/testing.rs#git::testing::discovering_suites_are_the_modules_that_rerun_scrubbed"
        status: pass
      - kind: integration
        ref: "src/git/testing.rs#git::testing::discovering_tests_pass_with_a_runner_git_dir"
        status: pass
    human_judgment: false

duration: 16min
completed: 2026-11-18
status: complete
---

# Phase 5 Plan 03: Tree, hooksPath and staleness reads Summary

**NUL-framed tree lookups, whole-tree listings and blob reads by commit id, git's effective core.hooksPath without GIT_CONFIG, and the two D-12 staleness times (newest reflog entry, newest FETCH_HEAD across worktrees), each pinned against git 2.56.1's own answer**

## Performance

- **Duration:** 16 min
- **Started:** 2026-11-18T20:21:18Z
- **Completed:** 2026-11-18T20:37:08Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- 05-04, 05-06 and 05-07 can ask git what a commit's tree holds at given paths (`lookup_paths`), at every path (`list_tree`), and what blobs hold (`read_blobs`), with no path ever on argv; each child passes `--no-lazy-fetch` (D-13).
- `Repo::hooks_path` returns the hooks directory git's hook runner uses: a global value is seen, `~` expands as git expands it, and a `GIT_CONFIG` file is not read.
- `Repo::ref_moved_at` and `Repo::fetched_at` give D-12's two times; a fetch made in the main worktree is seen from a linked one, and `log.showSignature=true` cannot corrupt the reflog read.

## Task Commits

1. **Task 1 RED: failing tree lookup and listing tests** - `e6a5efb` (test)
2. **Task 1 GREEN: tree entries, listings and blobs through git** - `7ceecae` (feat)
3. **Task 2 RED: failing hooksPath and staleness read tests** - `e3e933f` (test)
4. **Task 2 GREEN: core.hooksPath and the staleness times through git** - `4ec76a8` (feat)

## TDD Gate Compliance

- **Task 1 RED:** `git::trees::tests::lookup_paths_answers_each_request_in_order` failed on its `assert_eq!` of wsx's answers: the stub returned `[]`. Classifier: `RED_EVIDENCE_OK` (target/gsd/05-03-red-evidence.json). Semantic assessment: the crate compiled and the target ran; git's own batch-check oracle assertion passed before it, so the failure is the planned assertion, not a build, load or fixture fault.
- **Task 2 RED:** `git::staleness::tests::ref_moved_at_is_the_newest_entry` failed on its `assert_eq!`: `None` from the stub where `Some(1767225660)` (T0 + TICK) was expected. Classifier: `RED_EVIDENCE_OK` (target/gsd/05-03-red-evidence-2.json). Semantic assessment: the `git reflog exists` control passed first; the failure is the planned assertion.
- GREEN commits `7ceecae` and `4ec76a8` each follow their RED commit; no REFACTOR commit was needed.

## Files Created/Modified

- `src/git/trees.rs` - TreeEntry, ListedEntry, MODE_* constants, lookup_paths, list_tree, read_blobs and their parsers
- `src/git/staleness.rs` - ref_moved_at, fetched_at, fetch_head_paths, parse_reflog_selector
- `src/git/config.rs` - Repo::hooks_path and HOOKS_PATH_ARGS; module doc names the third read
- `src/git/spawn.rs` - run_without_var's doc names its two children and why core.hooksPath drops GIT_CONFIG
- `src/git/objects.rs` - BATCH_FORMAT, BatchAnswer and parse_batch become pub(super)
- `src/git/mod.rs` - mod staleness; mod trees; (no re-exports)
- `src/git/testing.rs` - DISCOVERING_SUITES has eight entries

## Decisions Made

- git answers a gitlink whose commit is absent as `<id> submodule` under the custom batch-check format; the lookup reads it as MODE_GITLINK, and the test asserts git's exact text first.
- A lookup answer is `None` only when it is exactly `<its own request> missing`.
- `parse_reflog_selector` refuses more than one line, so a signature check on stdout becomes GitBadOutput rather than a wrong time.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Global-value control in hooks_path_is_the_effective_value**
- **Found during:** Task 2 RED
- **Issue:** The plan's control, "Repo::read_config's local-only read does not see it", is vacuous: read_config only asks for `^(wsx|remote)\.` keys, so it never sees any `core.hooksPath`, global or local.
- **Fix:** The control is `git config --local --get core.hooksPath` exiting 1 in the repository whose only value is global, which shows a read of the repository's own scope would miss the value hooks_path returns.
- **Files modified:** src/git/config.rs
- **Verification:** the test passes; the control runs before the hooks_path assertion.
- **Committed in:** e3e933f

---

**Total deviations:** 1 auto-fixed (1 test-control correction). **Impact:** none on the API; the test's control now discriminates.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Ready for 05-04 (stale-only, symlink targets), 05-06 (hooks) and 05-07; they consume these reads without touching src/git, and 05-06 adds the `pub use trees::{...}` it needs.

## Verification

- Every named test in both tasks' `<verify>` passed, one test per command with `--no-tests=fail`.
- `scripts/check.sh all` exited 0 with last line `==> check.sh: all OK` after each GREEN (target/gsd/05-03-all.log).
- Acceptance greps: `rg -c '"git::[a-z]+::tests::",' src/git/testing.rs` printed 6 after Task 1; `rg -c '::tests::",'` printed 8 after Task 2; `core\.hooksPath` is in src/git/spawn.rs; `--end-of-options` is in src/git/staleness.rs; LOOKUP_ARGS, LIST_ARGS and BLOB_ARGS each begin with NO_LAZY_FETCH, as does REFLOG_ARGS.

## Self-Check: PASSED

- src/git/trees.rs and src/git/staleness.rs exist.
- e6a5efb, 7ceecae, e3e933f and 4ec76a8 are ancestors of HEAD; `git rev-list --count ea0c6d2..HEAD` is 4.

---
*Phase: 05-divergence-check-and-command-contract*
*Completed: 2026-11-18*
