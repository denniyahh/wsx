---
phase: 05-divergence-check-and-command-contract
fixed_at: 2026-11-19T10:54:00.820Z
review_path: .planning/phases/05-divergence-check-and-command-contract/05-REVIEW.md
iteration: 1
findings_in_scope: 4
fixed: 3
skipped: 1
status: partial
---

# Phase 05: Code Review Fix Report

**Fixed at:** 2026-11-19T10:54:00.820Z
**Source review:** .planning/phases/05-divergence-check-and-command-contract/05-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 4 (fix_scope critical_warning: WR-01, WR-02, WR-03, WR-05; WR-04 carries `**Category:** necessity` and is out of scope)
- Fixed: 3
- Skipped: 1

**Verification:** every gate ran in the main checkout (`workflow.use_worktrees: false`, no worktree created). Each fix's new tests were run alone first and failed for the reported reason, then passed after the fix; each commit was gated on `scripts/check.sh all` exit 0.

## Fixed Issues

### WR-01: Writing the verdict to a closed stdout panics with exit 101, outside the exit-code contract

**Files modified:** `src/cli/mod.rs`, `tests/cli.rs`, `tests/harness/contract.rs`
**Commit:** ef4daa5
**Applied fix:** `--version`, the non-`--json` verdict and the JSON object all go through one fallible `stdout_line` that reports a failed write on stderr and exits 6 (git_failed, "the operating system failed"), as `finish` already did. Every stderr line goes through `stderr_line`, which drops a failed write instead of panicking. No documented behaviour changed: exit 6 already covers an operating-system failure. Tests: `version_to_a_closed_stdout_exits_git_failed`, `contract::check_verdict_to_a_closed_stdout_exits_git_failed` (both failed before the fix with the "failed printing to stdout" panic, exit 101).

### WR-02: One missing symlink blob anywhere in the tree fails the whole check with exit 6

**Files modified:** `src/git/trees.rs`, `src/check/mod.rs`, `tests/harness/contract.rs`
**Commit:** ca829f2
**Applied fix:** `Repo::read_blobs` (its only caller is `check_hooks`) answers `None` for an id git reports `missing` instead of `Error::ObjectMissing`; `check_hooks` leaves an unread link text out of `targets`, so only a hook whose resolution needs it becomes a `hook_not_executable` problem ("the link text of ... was not read"). Tests: `contract::check_passes_with_a_missing_symlink_blob_outside_the_hooks` (exit 0, hooks checked) and `contract::check_reports_a_hook_symlink_whose_blob_is_missing` (exit 1, one hook problem); both exited 6 before the fix. `read_blobs_returns_each_blob_in_order` now pins `None` for an absent id.

### WR-03: The replace-ref guard does not cover the phase's new git children, against the comment's claim

**Files modified:** `src/git/mod.rs`, `src/git/trees.rs`, `src/check/mod.rs`
**Commit:** 3a25cca
**Applied fix:** new `NO_REPLACE_OBJECTS` (`--no-replace-objects`) leads the argv of the lookup, listing and blob-read children; the `check::run` comment now states what the guard covers, including that gix's diff keeps the map it loaded at open. Tests: `git::trees::tests::lookup_paths_ignores_a_replace_ref`, `list_tree_ignores_a_replace_ref`, `read_blobs_ignores_a_replace_ref`; each read the replaced object before the fix. `REFLOG_ARGS` in `src/git/staleness.rs` was left unchanged: the reflog read prints only the reflog entry's selector and time, which a replace ref cannot change, so no test could fail without the flag.

## Skipped Issues

### WR-05: The IF-02 scan misses a forbidden path built from separate literals

**File:** `tests/if02_scan.rs:37-46`
**Reason:** already fixed after the review was written. The review is dated 2026-11-19T01:07Z; gap plan 05-10 (commits a9a6ba7, 1805a0b, about two hours later) matches literals and `concat!` by decoded value (`STATE\x2emd` is caught: `escaped_spellings_trip_the_scan`) and joins `.join` chains (`.join(".planning").join("config.json")` is caught: `planted_join_read_trips_the_scan`, `join_spellings_trip_the_scan`). The review's other suggestion, adding `.planning` to `COMPONENTS`, is deliberately not done: the scan's module doc records `.planning` alone as manifest path data (D-14), and changing that is a locked-decision call for the operator.
**Original issue:** `.planning/config.json` was matched only as one literal, so a path built from separate or escaped literals passed the IF-02 gate.

---

_Fixed: 2026-11-19T10:54:00.820Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
