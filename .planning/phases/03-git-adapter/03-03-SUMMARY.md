---
phase: 03-git-adapter
plan: 03
subsystem: git
tags: [gix, git, ahead-behind, rev-walk, tree-diff, differential-test, tdd]

requires:
  - phase: 03-git-adapter
    provides: "03-01: Repo::discover, Repo::resolve, Repo::merge_base, ObjectId, Error::Gix/GitFailed, the Git spawn helper and the criss-cross fixture"
provides:
  - "Repo::ahead_behind -> AheadBehind { ahead, behind } (two hidden gix walks)"
  - "Repo::range_walk -> BTreeSet<ObjectId> (gix rev_walk with hidden tips; no order promised)"
  - "Repo::tree_diff -> Vec<TreeChange> (gix::diff::tree with a Recorder; no blob-diff feature; files only; sorted by path bytes, then kind)"
  - "ChangeKind { Added, Deleted, Modified, TypeChanged } and TreeChange { kind, path: Vec<u8> }"
  - "14 gix-vs-git differential and edge tests in src/git/reads.rs"
affects: [03-04, 03-05, phase-5, phase-6, phase-7, phase-8, phase-9, phase-10]

actuals:
  tokens: 7228      # chars/4 over the realized diff of src/git/reads.rs (28910 chars)
  tasks: 3
  commits: 5
plan_head_before: 20d88cbbaa784c9fc933bae2c713634adb696bd5
plan_head_after: 4d98d5910f05b7df7433998a20356fc6211cc2dc

tech-stack:
  added: []
  patterns:
    - "A read's answer is compared whole with git plumbing output run through wsx's own Git helper, and every read has a control pair whose answer differs"
    - "Walk results are sets (BTreeSet<ObjectId>), never ordered lists, because gix and git walk in different orders"

key-files:
  created: []
  modified:
    - src/git/reads.rs

key-decisions:
  - "ahead_behind is built on range_walk (two walks, one per direction, counted by set size) rather than a separate counting walk, so the two reads cannot disagree"
  - "A tree-diff record whose entry mode is a tree is dropped; gix-diff 0.70.0 only emits a Modification when both sides are trees or both are not, so a file replaced by a directory arrives as Deletion + Addition and the file half survives the filter"
  - "TypeChanged compares entry kinds with BlobExecutable folded into Blob, so an executable-bit change alone is Modified, as git's diff-tree reports it"

patterns-established:
  - "Pattern: git -z output parsed into wsx types with an unknown status letter failing the test with the raw output"

requirements-completed: [GIT-01]

coverage:
  - id: D1
    description: "Repo::ahead_behind equals git rev-list --left-right --count on (main, feat) = 2/3, (feat, main) and (main~1, feat~2) of the criss-cross, clock-skewed fixture"
    requirement: "GIT-01"
    verification:
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::ahead_behind_matches_git"
        status: pass
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::ahead_behind_changes_with_the_pair"
        status: pass
    human_judgment: false
  - id: D2
    description: "Repo::range_walk equals git rev-list <tip> --not <hidden> as a set: {F2, F3, F4} for feat --not main, {C1, F2, F3, F4} for feat --not feat~3"
    requirement: "GIT-01"
    verification:
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::range_walk_matches_git"
        status: pass
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::range_walk_changes_with_the_hidden_tip"
        status: pass
    human_judgment: false
  - id: D3
    description: "Repo::tree_diff equals git diff-tree -r --no-renames --name-status -z: main/feat is [M b.txt, D f/x, A f/z, D new/y, T p, A q]; main/feat~3 differs; a revision with itself is empty"
    requirement: "GIT-01"
    verification:
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::tree_diff_matches_git"
        status: pass
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::tree_diff_changes_with_the_pair"
        status: pass
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::tree_diff_of_a_revision_with_itself_is_empty"
        status: pass
    human_judgment: false
  - id: D4
    description: "GIT-01 edges pinned against git: a commit with itself, parent and child, unrelated histories (real orphan branch), all tips hidden, no tips, an executable-bit change, and repeated reads"
    requirement: "GIT-01"
    verification:
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::a_commit_with_itself_is_its_own_merge_base_and_level"
        status: pass
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::parent_and_child_share_the_parent_as_merge_base"
        status: pass
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::unrelated_histories_have_no_merge_base"
        status: pass
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::range_walk_with_every_tip_hidden_is_empty"
        status: pass
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::range_walk_without_tips_is_empty"
        status: pass
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::an_executable_bit_change_is_a_modification"
        status: pass
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::repeated_reads_return_equal_values"
        status: pass
    human_judgment: false

duration: 9min
completed: 2026-10-27
status: complete
---

# Phase 3 Plan 03: Ahead/behind, range walk and tree diff Summary

**Ahead/behind, range walk and tree diff now answer from gix, and on the criss-cross, clock-skewed fixture each gives the same result as git's plumbing. Each read has a control pair whose answer moves. The GIT-01 edge cases (a commit with itself, parent and child, unrelated histories, empty walks, repeated reads) are pinned against git.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-10-27T01:55:21Z
- **Completed:** 2026-10-27T02:04:17Z
- **Tasks:** 3 (two TDD RED then GREEN, one test-only)
- **Files modified:** 1 (`src/git/reads.rs`)

## Accomplishments

- `Repo::range_walk(tips, hidden)` is `rev_walk(tips).with_hidden(hidden).all()`, collected into a `BTreeSet<ObjectId>`. An empty `tips` slice returns the empty set without walking. A failure is `Error::Gix { what: "walk <tips> --not <hidden>" }`.
- `Repo::ahead_behind(first, second)` is the size of `range_walk([first], [second])` and of the reverse walk.
- `Repo::tree_diff(from, to)` peels both revisions to trees and runs `gix::diff::tree` with a default `Recorder`. It drops directory records and maps a mode change across blob, symlink and submodule to `TypeChanged`. Results are sorted by path bytes, then kind. `blob-diff` stays off: `rg -q '"blob-diff"' Cargo.toml` exits 1.
- No gix type crosses the public surface. Callers get `AheadBehind`, `BTreeSet<ObjectId>` and `Vec<TreeChange>` with raw-byte paths.

## Task Commits

1. **Task 1 RED: add failing ahead/behind and walk parity tests**: `8d0bd53` (test)
2. **Task 1 GREEN: count ahead/behind and walk ranges with gix**: `1a9ed20` (feat)
3. **Task 2 RED: add failing tree diff parity tests**: `4bf572f` (test)
4. **Task 2 GREEN: diff trees with gix's low-level tree diff**: `cc3d9a6` (feat)
5. **Task 3: pin the read edge cases against git**: `4d98d59` (test)

## TDD Gate Compliance

- **Task 1 RED:** the classifier returned `RED_EVIDENCE_OK` for `git::reads::tests::ahead_behind_matches_git`. The record is `target/gsd/03-03-red-evidence.json` (JUnit, exit 100). Semantic assessment: the target ran (1 test run, 236 skipped) and failed on the planned gix-vs-git assertion for the first pair, `main`/`feat`: `left: AheadBehind { ahead: 0, behind: 0 }`, `right: AheadBehind { ahead: 2, behind: 3 }`. The fixture had built, discovery had succeeded and git's counts had parsed, so the failure was not a build, fixture or parse fault. The other three Task 1 tests also failed against the stubs (4 run, 4 failed).
- **Task 2 RED:** the classifier returned `RED_EVIDENCE_OK` for `git::reads::tests::tree_diff_matches_git`. The record is `target/gsd/03-03-red-evidence-2.json` (JUnit, exit 100). Semantic assessment: the target failed on the gix-vs-git assertion. The stub returned `[]`. Git printed exactly the six expected changes, with no unknown status letter.
- `tree_diff_of_a_revision_with_itself_is_empty` already passed against the RED stub, because the stub returns an empty vector. It pins an edge case, not new behaviour, so its RED pass is expected. It is not evidence that GREEN was needed.
- Each `test(03-03)` commit precedes its `feat(03-03)` commit (`8d0bd53` then `1a9ed20`; `4bf572f` then `cc3d9a6`). No refactor commit was needed. Task 3 is not TDD, per the plan.

## Mutation controls (each reverted, and the file compared byte for byte with the gated version)

- Swapping `ahead` and `behind`: `ahead_behind_matches_git` fails (`left: 3/2`, `right: 2/3`).
- Ignoring the hidden tips in `range_walk`: both `range_walk_matches_git` and `range_walk_changes_with_the_hidden_tip` fail.
- Mapping every kind change to `Modified`: `tree_diff_matches_git` fails (`p` is `T` in git).
- Keeping directory records: `tree_diff_matches_git` fails.
- No longer folding `BlobExecutable` into `Blob`: `an_executable_bit_change_is_a_modification` fails (`TypeChanged` against git's `M`).
- A misnamed filter, `test(=git::reads::tests::no_such_test)`, exits 4. So the verify chain cannot pass on a typo.

## Files Created/Modified

- `src/git/reads.rs`: `AheadBehind`, `ChangeKind`, `TreeChange`; `Repo::{ahead_behind, range_walk, tree_diff}` and the private `tree_of`, `resolve_all`, `file_change`, `entry_class` and `render_revs`; git-side test parsers (`git_ahead_behind`, `git_ids`, `git_tree_diff`); and 14 new tests.

## Decisions Made

See `key-decisions` in the frontmatter. Everything else follows the plan's contract: names, signatures, error `what` strings and the fixture.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing verification] Two tests added beyond the plan's list**
- **Found during:** Task 3
- **Issue:** The plan documents two behaviours that no planned test exercised. An empty `tips` slice returns the empty set without walking (Task 1 action). An executable-bit change alone is `Modified`, not `TypeChanged` (the type contract). The fixture has no mode-only change, so a mapping that treated `100644 -> 100755` as a type change would have passed every planned test.
- **Fix:** Added `range_walk_without_tips_is_empty`, which checks against `git rev-list --not main` printing nothing. Added `an_executable_bit_change_is_a_modification`: the test chmods `a.txt` to 755 on disk and commits, and the diff `main~1`/`main` is `[M a.txt]` in both gix and git.
- **Verification:** Both pass. The exec-bit test fails under the unfolded-kind mutation listed above.
- **Committed in:** `4d98d59`

**2. [Rule 1 - Lint] `map_err(&failed)` rejected by clippy**
- **Found during:** Task 2 GREEN gate
- **Issue:** `clippy::needless_borrows_for_generic_args` failed `scripts/check.sh all` (rc 101).
- **Fix:** `map_err(failed)`. The closure only borrows, so it is `Copy`. The gate passed afterwards, and only then was the change committed.
- **Committed in:** `cc3d9a6`

**Total deviations:** 2 auto-fixed (1 missing verification, 1 lint). **Impact:** 14 tests instead of the planned 12. No production behaviour beyond the plan. Every must_have truth holds as worded. The 03-01 note was checked: no expected value here depends on git's choice of merge-base for `main`/`feat`. The plan's ahead/behind and walk values are reachability sets, and they matched git on the first run.

## Issues Encountered

- My first attempt at loading the `gsd_run` resolver sourced the reference markdown file as a shell script. That started a stray interactive `bash` (PID 1077263), which I stopped by its PID. It wrote nothing to the repository. `gsd-tools.cjs` was then called directly with `node`.

## Verification Results (limits stated)

- `scripts/check.sh all`: exit 0, `==> check.sh: all OK` (205 tests run, 205 passed, 42 skipped). This ran on the exact content of each non-RED commit before that commit was made.
- All 12 planned tests, plus the two added ones, passed when each was run on its own with `--no-tests=fail`. The plan's three `<automated>` chains each exited 0.
- Acceptance greps: Task 1 rc 0; Task 2 rc 0, and `"blob-diff"` absent (rc 1); Task 3 rc 0, including `"--orphan"`.
- **What this does not establish:** parity is shown on one criss-cross fixture with one clock skew, plus the edge fixtures. That is evidence that gix matches git on these shapes, not proof for every history. In particular, histories with commit-graph files, shallow clones, replace refs and grafts were not exercised. The hidden-walk ahead/behind matched git here, but gix's walk with hidden tips has not been compared against git on a large or deep history. Rename detection is deliberately absent, as in `git diff-tree --no-renames`. Submodule (`160000`) entries are mapped as their own kind, but no fixture has one, so the submodule side of `TypeChanged` is untested. Fixture git is 2.56.0. Every gix read runs in-process with the developer's environment (no read here consults config), so hermeticity of the gix side is not proven, as 03-01 already noted.

## Known Stubs

None. The RED stubs were replaced in their GREEN commits, and a scan of `src/git/reads.rs` for TODO, FIXME and placeholder text finds nothing.

## Threat Flags

None. No new network, auth, file-write or schema surface. T-03-08 is mitigated by the differential tests and controls above. T-03-09: walk and diff failures become `Error::Gix` naming the query, and the only `Ok(None)` is gix's own "no merge-base", pinned against git's exit 1 with empty stderr. T-03-SC: no install was made, and every cargo call went through `scripts/check.sh` (`--locked`).

## Cleanup

My scratchpad files are disposable. No `git-unit-*` directory remains under `target/tmp`: each test's `TempDir` was removed on drop. The stray `bash` is stopped, and no background process of mine is running. I kept `target/gsd/03-03-red-evidence.json` and `-2.json` (the plan requires them) and `target/gsd-tools/` (needed by later plans while the host nextest is off-pin). All are gitignored.

## Next Phase Readiness

All four criterion-1 reads (merge-base, ahead/behind, range walk, tree diff) exist with git parity on the fixture. GIT-01 is still declared by other phase-3 plans, so `requirements.ready-ids` reports it blocked, and it is not marked complete in REQUIREMENTS.md by this plan.

## Self-Check: PASSED

- FOUND: src/git/reads.rs, target/gsd/03-03-red-evidence.json, target/gsd/03-03-red-evidence-2.json
- FOUND commits on HEAD: 8d0bd53, 1a9ed20, 4bf572f, cc3d9a6, 4d98d59. `gsd-tools check evaluation-scope --plan 03-03 --commits-only` resolved the plan's commits from their subjects.
