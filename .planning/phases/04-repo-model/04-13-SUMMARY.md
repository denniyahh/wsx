---
phase: 04-repo-model
plan: 13
subsystem: repo-model
tags: [gix, replace-refs, linked-worktree, fail-closed, tdd]
requires:
  - phase: 04-repo-model
    provides: "D-22 replace-ref guard (Repo::refuse_replace_refs) and the model replacement tests (04-12)"
provides:
  - "Repo::refuse_replace_refs refuses an entry in a linked worktree's own <git_dir>/refs/replace (CR-01, D-32)"
  - "ReplaceBaseSet renders only the remedy for the setting that is set (IN-02)"
  - "resolve's doc names the replace-ref guard (IN-01)"
affects: [04-14, 04-15, 04-16, 04-17, 04-18]
actuals:
  tokens: 4056
  tasks: 2
  commits: 4
plan_head_before: 383c74b90b20e855573065c29f6a533246450e21
plan_head_after: 4e00f17acf002560a898ea3ee9fa29e60186f0e8
tech-stack:
  added: []
  patterns:
    - "A linked worktree's private git dir is a replace-ref store wsx checks, loose only"
key-files:
  created: []
  modified:
    - src/git/refs.rs
    - src/git/mod.rs
    - src/model/mod.rs
key-decisions:
  - "ReplaceBaseSet branches on refs::REPLACE_BASE_VAR, made pub(super), rather than a second copy of the variable name"
  - "The private store is checked only for loose entries; its packed-refs is not scanned (spike 002 B2b: neither reader takes a replace ref from it)"
patterns-established:
  - "Linked-worktree replace tests assert git's answer from the linked worktree first, then move wsx's answer onto it with a control"
requirements-completed: [CONF-07]
coverage:
  - id: D1
    description: "A replace ref in a linked worktree's own refs/replace stops resolve from that worktree as ReplaceRefsPresent naming that dir; removing it, or reading from the main worktree, resolves only .planning"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::private_replace_ref_in_a_linked_worktree_fails_closed"
        status: pass
    human_judgment: false
  - id: D2
    description: "A common-dir replace ref read from a linked worktree is still refused naming <work>/.git/refs/replace; after git replace -d both git and wsx give only .planning"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::common_replace_ref_from_a_linked_worktree_fails_closed"
        status: pass
    human_judgment: false
  - id: D3
    description: "A fifo at the private refs/replace is ReplaceRefsUnreadable naming it, never a .wsx.conf failure; an empty dir there resolves"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::unreadable_private_replace_dir_blames_the_replace_refs"
        status: pass
    human_judgment: false
  - id: D4
    description: "A repo with no wsx.sourceBranch stays not configured with a private replace ref: the refusal is inside inferred and require ends Run wsx init. (D-16)"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::private_replace_ref_leaves_a_not_configured_repo_not_configured"
        status: pass
    human_judgment: false
  - id: D5
    description: "ReplaceBaseSet names only the remedy for its own setting (IN-02)"
    verification:
      - kind: unit
        ref: "src/git/mod.rs#git::tests::replace_base_variable_message_names_only_the_variable"
        status: pass
      - kind: unit
        ref: "src/git/mod.rs#git::tests::replace_base_set_message_names_the_setting_and_the_fix"
        status: pass
    human_judgment: false
  - id: D6
    description: "IN-01 and CR-01 doc findings fixed: refuse_replace_refs's doc names the private store, resolve's doc names the guard"
    verification:
      - kind: other
        ref: "rg -qF 'Only the common dir is read' src/git/refs.rs (exit 1); rg -qF 'that lists replace refs' src/model/mod.rs (exit 1)"
        status: pass
    human_judgment: false
  - id: D7
    description: "The D-01 to D-21 behaviour tests are unchanged and the full gate is green"
    verification:
      - kind: other
        ref: "scripts/check.sh all (last line ==> check.sh: all OK)"
        status: pass
    human_judgment: false
duration: 7min
completed: 2026-11-17
status: complete
---

# Phase 04 Plan 13: Linked-worktree replace refs and the replace-base remedy Summary

**D-22's guard now also refuses an entry in a linked worktree's own `refs/replace`, which gix applied and git ignores (CR-01), and `ReplaceBaseSet` names only the fix for the setting that is set (IN-02).**

## Performance

- **Duration:** 7 min
- **Started:** 2026-11-17T22:17:00Z
- **Completed:** 2026-11-17T22:24:08Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- `Repo::refuse_replace_refs` checks `<git_dir>/refs/replace` when the git dir is not the common dir, returning `ReplaceRefsPresent { packed: false }` naming it; an unreadable one is `ReplaceRefsUnreadable` through the existing `holds_an_entry`.
- At RED, `private_replace_ref_in_a_linked_worktree_fails_closed` showed the live gap: from the linked worktree git printed `only .planning` while wsx resolved `Configured` with manifest `only b`, the replaced blob.
- `ReplaceBaseSet`'s `Display` branches on `name`: `GIT_REPLACE_REF_BASE` gets `Run wsx with GIT_REPLACE_REF_BASE unset.`, the key gets the remove-the-key sentence and its `git config --show-origin` line.
- Docs: `refuse_replace_refs` (new step 4, the private store and why it is read, why its `packed-refs` is not), `ReplaceRefsPresent` (three stores), `resolve` (IN-01).

## Task Commits

1. **Task 1 RED:** `b098656` test(04-13): add failing tests for a linked worktree's own replace refs
2. **Task 1 GREEN:** `6f4954a` fix(04-13): refuse a linked worktree's own replace refs
3. **Task 2 RED:** `9275570` test(04-13): pin the ReplaceBaseSet remedy per setting
4. **Task 2 GREEN:** `4e00f17` fix(04-13): name only the remedy for the replace base setting that is set

## TDD Gate Compliance

RED then GREEN for each task, in that order; no REFACTOR commit.

- **Task 1 RED record:** `gsd_run check tdd-red-evidence target/gsd/04-13-red-evidence.json` verdict `RED_EVIDENCE_OK` (reason `target_test_failed`, JUnit, exit 100). Semantic assessment: the target ran and failed on its planned assertion, `resolve: expected an error, got Configured(... manifest only b ...)`, after git's `only .planning` assertion passed; not a build, load or fixture failure.
- **`unreadable_private_replace_dir_blames_the_replace_refs`:** log `exit 100`, met `resolve: expected an error, got Configured(...)`. `Repo::discover` from the linked worktree did not fail on the fifo, so the premise held.
- **`private_replace_ref_leaves_a_not_configured_repo_not_configured`:** log `exit 100`, met `inferred: expected an error, got Model { ... }`.
- **`common_replace_ref_from_a_linked_worktree_fails_closed`:** passed at RED (`exit 0`), as planned: the guard already read the common dir.
- **Task 2 RED:** `replace_base_variable_message_names_only_the_variable` log `exit 100`, `assert_eq!` left side holding today's combined remedy (`... unset, and remove gitoxide.objects.replaceRefBase ...`); a record `target/gsd/04-13-task2-red-evidence.json` (beyond the plan) also gave `RED_EVIDENCE_OK`.

## Files Created/Modified

- `src/git/refs.rs` - the private-store check in `refuse_replace_refs` and its doc; `REPLACE_BASE_VAR` now `pub(super)`
- `src/git/mod.rs` - `ReplaceBaseSet` remedy per setting, `ReplaceRefsPresent` doc, the new and re-pointed message tests
- `src/model/mod.rs` - `private_replace_dir`, `write_private_replace_ref`, `remove_file` helpers, the four linked-worktree tests, the IN-01 doc sentence

## Decisions Made

- The remedy branch compares `name` against `refs::REPLACE_BASE_VAR` (made `pub(super)`), so the variable's name is spelled once.
- Two small test helpers beyond `private_replace_dir` (`write_private_replace_ref`, `remove_file`), because three tests write or remove the same entry.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Task 1's first `<verify>` cannot pass as written**
- **Found during:** Task 1 verify
- **Issue:** `gsd_run check tdd-red-evidence ... --raw` prints the whole JSON result, so `| rg -qx 'RED_EVIDENCE_OK'` never matches and the command prints `no valid RED record` on a valid record.
- **Fix:** ran the same command with `--raw --pick verdict`, which prints the bare verdict; the rest of the command unchanged. The plan file was not edited.
- **Verification:** the variant exits 0. Controls: a copy of the record naming a nonexistent target test gives `INVALID_RED`; replacing the expected text gives `... failed without the intended text`; pointing the check at the test that passed at RED gives `... did not fail at the RED commit`.

**2. [Rule 1 - Bug] clippy `write_with_newline` on the first `ReplaceBaseSet` draft**
- **Found during:** Task 2 GREEN, `scripts/check.sh all`
- **Fix:** the newline moved from the end of the first sentence to the start of each remedy; rendered text unchanged, both message tests pass.
- **Committed in:** `4e00f17`

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug). **Impact:** none on behaviour; the first is a defect in the plan's verify text that later plans with the same pipe will meet.

## Issues Encountered

None.

## User Setup Required

None.

## Next Phase Readiness

Ready for 04-14 (config through git). Each shape is pinned on one fixture on the host's git (2.56.1); the mutation pass for this change runs in 04-18.

## Self-Check: PASSED

- FOUND: b098656
- FOUND: 6f4954a
- FOUND: 9275570
- FOUND: 4e00f17
- FOUND: src/git/refs.rs
- FOUND: src/git/mod.rs
- FOUND: src/model/mod.rs
- FOUND: target/gsd/04-13-red-evidence.json
- FOUND: target/gsd/04-13-all.log
- ==> check.sh: all OK

---
*Phase: 04-repo-model*
*Completed: 2026-11-17*
