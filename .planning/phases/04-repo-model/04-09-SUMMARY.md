---
phase: 04-repo-model
plan: 09
subsystem: repo-model
tags: [rust, gix, git-objects, manifest, personal-paths, tdd]

requires:
  - phase: 04-repo-model
    provides: "04-01: `root_entry`, `RootEntry`, the model contract, `personal_paths`/`is_personal`"
  - phase: 04-repo-model
    provides: "04-04: `manifest::normalize_path`, `Problem`, `parse`"
  - phase: 04-repo-model
    provides: "04-05: `single` (D-18), `state_child`"
  - phase: 04-repo-model
    provides: "04-08: `build`'s `ManifestUnreadable` mapping, `require`, the model test helpers"
provides:
  - "`RootEntry::NotABlob { kind }`: a symlink, directory or gitlink at `.wsx.conf`, its mode checked before any object read"
  - "`git::Error::TreeNotSorted { commit, tree }`: `root_entry` decodes the root tree whole and refuses one out of git's tree order"
  - "`model::WORKTREE_DIR` (`wsx.worktreeDir`), read through `single` and `manifest::normalize_path` into `Model::worktree_dir`"
  - "`resolve`'s doc comment: the read-consistency contract, the replacement-map window included"
  - "`testing::raw_tree_commit`: a commit whose root tree is written byte for byte, in any order"
  - "`model::tests::state_child` prints `personal=`, `env_GIT_DIR=` and `env_GIT_INDEX_FILE=`"
affects: [04-10, 04-11, phase-5-check, phase-6-guard, phase-8-new, phase-9-cut]

actuals:
  tokens: 17700
  tasks: 3
  commits: 5
plan_head_before: 974dd0ffe4c2a67c21edc101122eec8467029397
plan_head_after: 85644733d2b476bc493b4418c6a7323458ec97f8

tech-stack:
  added: []
  patterns:
    - "A tree read decodes the whole tree and checks git's order before trusting a by-name match"
    - "Each tree-entry shape is asserted through git (`git ls-tree`, `git cat-file`, `git fsck`) before wsx's answer is asserted"
    - "A read-only claim is tested with a byte snapshot of `.git` and a control that the snapshot sees a ref change"

key-files:
  created: []
  modified:
    - src/git/refs.rs
    - src/git/mod.rs
    - src/git/testing.rs
    - src/model/mod.rs

key-decisions:
  - "TreeNotSorted carries the peeled commit's id and the tree's id; build needs no new arm, since 04-08 maps every root_entry failure to ManifestUnreadable"
  - "A mode that is not a blob, a symlink or a tree reads as submodule, as gix's EntryMode::kind maps any other mode to Commit"
  - "wsx.worktreeDir is read after the integration branch and before the tip read, so a bad value is refused before any manifest read"
  - "The hook test runs a real pre-commit hook (a partial commit in a linked worktree), not a simulated environment, and asserts the GIT_DIR and GIT_INDEX_FILE git exported"

patterns-established:
  - "refs test helpers: manifest_repo, ls_tree, assert_head_mode, git_manifest, manifest_entry, head_manifest_entry, git_answer"
  - "model test helpers: workspace_manifest_line, assert_workspace_mode, replace_manifest, symlink_manifest, not_a_blob, unreadable_branch, manifest_on_workspace, model_with_manifest, path_texts, worktree_dir_resolved, worktree_dir_problem, personal_marker, personal_list, linked_worktree, repository_snapshot, changed_paths"

requirements-completed: []

coverage:
  - id: D1
    description: "Only a regular or executable file at the tip's .wsx.conf is read; a symlink, directory or gitlink is NotABlob, each shape confirmed by git ls-tree first"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/git/refs.rs#git::refs::tests::{root_entry_reads_a_regular_file,root_entry_reads_an_executable_file,root_entry_reads_an_empty_file,root_entry_reports_a_symlink,root_entry_reports_a_directory,root_entry_reports_a_submodule,root_entry_is_absent_without_the_file,root_entry_ignores_a_nested_file}"
        status: pass
    human_judgment: false
  - id: D2
    description: "A root tree out of git's order is refused as TreeNotSorted, with git cat-file failing, git ls-tree listing the entry and git fsck reporting treeNotSorted; two in-order controls read"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/git/refs.rs#git::refs::tests::{root_entry_refuses_a_tree_out_of_git_order,root_entry_reads_a_tree_in_git_order,root_entry_reads_a_tree_git_wrote_where_name_order_differs}; src/git/mod.rs#git::tests::tree_not_sorted_message_names_the_tree_and_the_fix; src/model/mod.rs#model::tests::unsorted_tree_manifest_fails_closed"
        status: pass
    human_judgment: false
  - id: D3
    description: "A configured repository fails closed with the reason on a missing, symlink, directory, gitlink, missing-blob, missing-tip-commit or invalid manifest; an executable manifest resolves"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::{missing_manifest_fails_closed,symlink_manifest_fails_closed,directory_manifest_fails_closed,submodule_manifest_fails_closed,missing_blob_object_fails_closed,missing_tip_commit_fails_closed,invalid_manifest_fails_closed,executable_manifest_resolves}"
        status: pass
    human_judgment: false
  - id: D4
    description: "With no wsx.sourceBranch, a symlink manifest is NotConfigured { inferred: Err(ManifestNotABlob) } and require says run wsx init; a regular manifest in its place gives the model"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::require_wraps_a_non_blob_manifest_as_not_configured"
        status: pass
    human_judgment: false
  - id: D5
    description: "The personal-path list is exactly .wsx.conf, the worktree dir and the only lines in file order; wsx.worktreeDir replaces .worktrees under D-14's path rules and D-18"
    requirement: CONF-02
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::{undeclared_path_is_not_personal,declared_path_is_personal,modified_path_is_not_personal,built_ins_are_personal_with_an_empty_manifest,custom_worktree_dir_replaces_the_default,worktree_dir_trailing_slash_is_stripped,worktree_dir_with_a_dot_component_is_refused,absolute_worktree_dir_is_refused,repeated_worktree_dir_is_an_error,empty_worktree_dir_is_an_error,comments_only_manifest_leaves_the_built_ins,one_line_manifest_adds_one_path,personal_list_is_built_ins_then_declarations_in_file_order,duplicate_declarations_stay_in_the_list,built_in_declaration_stays_in_the_manifest}"
        status: pass
    human_judgment: false
  - id: D6
    description: "The list comes from the object database: the same from a dirty checkout, a clean checkout, a linked worktree, a bare clone, a SHA-256 repository and a real hook; resolving writes nothing; a branch moved mid-read cannot pair a tip with another commit's manifest"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::{working_tree_edit_changes_nothing,clean_checkout_resolves_the_same_list,linked_worktree_resolves_the_same_list,bare_repo_resolves_the_same_model,sha256_repo_resolves_the_same_model,hook_environment_resolves_the_same_list,model_is_a_snapshot_of_the_tip_it_read,resolve_changes_nothing_in_the_repository,repository_snapshot_sees_a_ref_change}; src/git/refs.rs#git::refs::tests::manifest_read_stays_on_the_commit_it_was_given"
        status: pass
    human_judgment: false

duration: 13min
completed: 2026-11-08
status: complete
---

# Phase 4 Plan 09: Manifest source and personal-path list Summary

**The personal-path list now reads only a regular or executable `.wsx.conf` from the workspace tip's object database, refuses a symlink, directory, gitlink or a root tree out of git's order before reading anything, honours `wsx.worktreeDir` under the manifest's path rules, and is pinned identical across dirty, clean, linked, bare, SHA-256 and hook checkouts.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-11-08T17:05:07Z
- **Completed:** 2026-11-08T17:18:05Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments

- `root_entry` decodes the root tree whole, refuses a tree whose entries are not strictly increasing in git's order (`TreeNotSorted`), and checks the entry's mode before any blob read, so a symlink's link text can never be parsed as a manifest.
- A configured repository fails closed with the reason on every non-file manifest shape, a missing blob, a missing tip commit, an unsorted tree and an invalid manifest; in a repository that is not configured, a non-blob manifest reads as run `wsx init`.
- `wsx.worktreeDir` replaces `.worktrees` in the built-ins, through `manifest::normalize_path` and D-18's single-value read.
- The list is shown to come from the object database from every checkout CONF-07 names, `resolve` is shown to write nothing, and `resolve`'s doc comment states the read-consistency contract.

## Task Commits

1. **Task 1: Only a regular file at the tip is a manifest (RED, GREEN)** - `ddc9f94` (test), `c4f9f4f` (feat)
2. **Task 2: The built-ins, `wsx.worktreeDir`, and what counts as personal (RED, GREEN)** - `bdd15e6` (test), `6f60cc3` (feat)
3. **Task 3: The list comes from the object database, from any checkout** - `8564473` (test)

## TDD Gate Compliance

- **Task 1 RED:** `git::refs::tests::root_entry_reports_a_symlink` ran and failed on its planned assertion (`left: Blob([97, 46, 116, 120, 116])`, the link text `a.txt`, `right: NotABlob { kind: "symlink" }`) after its fixture steps passed (`git ls-tree` printed `120000 blob`, `git cat-file blob` printed `a.txt`). Record `target/gsd/04-09-red-evidence.json`; `gsd-tools check tdd-red-evidence` printed `RED_EVIDENCE_OK` (`target_test_failed`, 1 test, 1 failure, exit 100). `root_entry_refuses_a_tree_out_of_git_order` also failed as planned, with `Ok(Blob(...))` where `Err(TreeNotSorted)` was expected.
- **Task 1 GREEN:** `c4f9f4f`; all eleven `root_entry` tests, ten model tests and the message test pass one by one; `scripts/check.sh all` exit 0.
- **Task 2 RED:** `model::tests::custom_worktree_dir_replaces_the_default` ran and failed on its planned assertion (`left: [".wsx.conf", ".worktrees", "a"]`, `right: [".wsx.conf", ".wt", "a"]`): `build` never read the key. Record `target/gsd/04-09-red-evidence-2.json`; the classifier printed `RED_EVIDENCE_OK` (`target_test_failed`, exit 100).
- **Task 2 GREEN:** `6f60cc3`; all fifteen named tests pass one by one; `scripts/check.sh all` exit 0.
- Task 3 is not tdd (it pins existing behaviour). Every test passed on its first run, so no defect was found behind it.

## Files Created/Modified

- `src/git/refs.rs` - `RootEntry::NotABlob`, the decode, tree-order check and mode check in `root_entry`, and fourteen tests
- `src/git/mod.rs` - `Error::TreeNotSorted`, its message and its message test
- `src/git/testing.rs` - `raw_tree_commit` (and `hex_bytes`, `hashed`), the raw-tree recipe shared by the refs and model tests
- `src/model/mod.rs` - `WORKTREE_DIR`, `worktree_dir`, the `NotABlob` arm in `build`, `resolve`'s contract comment, `state_child`'s new markers, and thirty-four tests

## Decisions Made

- `TreeNotSorted` names the peeled commit and its tree; `build` needed no new arm, since 04-08 already reports any `root_entry` failure as `ManifestUnreadable`.
- A mode that is not a blob, symlink or tree is reported as `submodule`, as the plan specifies; gix's `EntryMode::kind` maps every other mode to `Commit`, so an unusual mode in a hand-written tree is also called a submodule. It is still refused, so this only affects the wording.
- `wsx.worktreeDir` is read after the integration branch and before the tip read.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The raw-tree recipe lives in `src/git/testing.rs`, a file outside the plan's `files_modified`**
- **Found during:** Task 1 RED
- **Issue:** the recipe is needed by both `git::refs::tests` and `model::tests`, and private test modules cannot share helpers.
- **Fix:** `testing::raw_tree_commit` holds it once. It writes the blob and tree bytes to files under the scratch root and passes the file path to `git hash-object` (`--literally -t tree -w <file>`), not `--stdin`; git hashes the same bytes either way.
- **Files modified:** `src/git/testing.rs`
- **Verification:** `root_entry_refuses_a_tree_out_of_git_order` asserts git's own answer on the written tree (`cat-file` fails, `ls-tree` lists both entries, `fsck` reports `treeNotSorted`), and the in-order control reads the blob git prints.
- **Committed in:** `ddc9f94`; a clippy fix (`is_multiple_of`) to its hex check in `c4f9f4f`

**2. [Rule 2 - Stronger check] The hook test runs a real pre-commit hook, not a child with a hand-set environment**
- **Found during:** Task 3
- **Issue:** the plan describes a child with `GIT_DIR` and `GIT_INDEX_FILE` set the way a hook sees them, and lists "simulated with a child, not a real hook" as a limit.
- **Fix:** `hook_environment_resolves_the_same_list` installs a pre-commit hook that runs `state_child`, makes a partial commit in a linked worktree, and asserts the `GIT_DIR` (`<main>/.git/worktrees/linked`) and the `GIT_INDEX_FILE` (a file in that directory) that git exported, then `state=configured` and `personal=` equal to the main worktree's list. To support that, `state_child` now prints `env_GIT_DIR` and `env_GIT_INDEX_FILE` as well as `personal`.
- **Files modified:** `src/model/mod.rs`
- **Committed in:** `8564473`

**3. [Rule 1 - Test fixture] Two model tests check more than the plan lists**
- `modified_path_is_not_personal` also asserts the manifest's one entry is `(Modified, "README.md")`, so the check fails if the line was dropped rather than read.
- `built_in_declaration_stays_in_the_manifest` asserts the entry's kind and path as well as its line number.

---

**Total deviations:** 3 (1 blocking, 1 strengthened check, 1 stronger assertion). **Impact:** no change to production behaviour beyond the plan. One test-support file was added to the scope.

## Limits

- An out-of-order root tree is always refused, including some trees where git's lookup would still reach `.wsx.conf`. That errs on the safe side. Only the root tree is checked, because it is the only tree this read looks in.
- A malformed tree entry now fails the read through `Tree::decode`. That conclusion comes from reading the gix source; no fixture builds such an entry.
- The tree-order rule is gix-object's `name_order`, read from source. It is pinned against git on one git-made tree where git's order differs from byte order (`a.b` before `a/`), not on every possible name.
- The `TreeNotSorted` fix command (`git ls-tree -z <commit> | git mktree -z`) appears only in the message. It was not run end to end in this plan.
- `resolve_changes_nothing_in_the_repository` compares file bytes, so it cannot see a file rewritten with identical bytes. Its control shows it does see a ref update.
- `manifest_read_stays_on_the_commit_it_was_given` moves one ref at one point between reads. It shows the tip and the list cannot come from different commits there; it does not rule out other interleavings, and the other tests run sequentially. The other parts of the contract in `resolve`'s doc comment (two tips need not be from one instant; the replacement-map window) are stated but not tested.
- Measured on git 2.56.0 only.
- RESEARCH line 485's claim that a corrupt tree could read as `Absent` is superseded, as recorded in the plan's context; RESEARCH itself was not edited.
- Edge-coverage item CONF-07 (category `unclassified`, probe "review manually") remains unresolved, as the probe protocol requires: no test builds a missing tree object or a malformed tree entry.

## Issues Encountered

- clippy (`manual_is_multiple_of`) refused `hex.len() % 2 != 0` in the new test helper at the first Task 1 gate run. It was fixed before the GREEN commit, and the gate then exited 0.

## Requirements

- CONF-02 and CONF-07 are listed in this plan, but `requirements.ready-ids` reports both as blocked, because a later plan in this phase also lists them and has no SUMMARY yet. REQUIREMENTS.md is left unchanged for both.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Plan 04-10 is next. Plan 04-11 adds the replace-ref refusal as the first call in this plan's `root_entry`.

## Self-Check: PASSED

- Modified files present: `src/git/refs.rs`, `src/git/mod.rs`, `src/git/testing.rs`, `src/model/mod.rs`.
- Commits `ddc9f94`, `c4f9f4f`, `bdd15e6`, `6f60cc3`, `8564473` are ancestors of HEAD (`git merge-base --is-ancestor`).
- `git rev-list --count 974dd0f..8564473` = 5.
- Every acceptance criterion command in Tasks 1-3 exited 0; at least one baseline check (`second lookup`, `manifest_read_stays_on_the_commit_it_was_given`, `normalize_path`) exited 1 on the commit before its task.
- Final `scripts/check.sh all`: exit 0, last log line `==> check.sh: all OK`, 614 tests run, 614 passed, 61 skipped.

---
*Phase: 04-repo-model*
*Completed: 2026-11-08*
