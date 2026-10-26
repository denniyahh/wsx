---
phase: 02-oracle-harness
plan: 11
subsystem: testing
tags: [rust, nextest, oracle, harness, git, pr-cut]

requires:
  - phase: 02-oracle-harness (02-05)
    provides: Class, OracleSide, Role, CaseKind, validate and judge
  - phase: 02-oracle-harness (02-07)
    provides: Action::Cut oracle mapping (numeric scopes, explicit PR name), cut::self_test, the recording rule
  - phase: 02-oracle-harness (02-08)
    provides: byte-exact comparator (tree as a map, refs as a set)
provides:
  - "Seven cut scenarios with oracle records: quoted_personal_names, topic_merge_dropped, integration_merged_into_feature, merge_resolution_adds_personal_file, existing_pr_branch_with_extra_commits, conflict_leaves_partial_pr_branch, scoped_and_unscoped_commits"
  - "cut.rs helpers for later cut plans (02-14): phase_worktree, topic_worktree, topic_cut, phase_cut, commit, cut_outcome, refusal, fixed, advance_and_fetch, commit_files"
affects: [02-14, 02-15, phase-9, phase-10]

actuals:
  tokens: 7692
  tasks: 2
  commits: 2
plan_head_before: 3ba78c3436d36949b04a7a1570922fb89758f440
plan_head_after: f6833f97b7b00d1baee0d4384b2eaa6450246e62

tech-stack:
  added: []
  patterns:
    - "A cut with no scope runs from feature/topic in work/.worktrees/topic with an explicit pr_branch feature/topic-pr; a scoped cut runs from feature/phase-1 with scope 1"
    - "A fixture that advances origin/develop also fetches it in work, so the cut's own fetch changes no ref and a refusal is RefsExpect::Unchanged"

key-files:
  created: []
  modified:
    - tests/harness/cut.rs

key-decisions:
  - "Every case was classed by its first run as Parity: 6 of the 14 cases are fixed: (all subjects except integration_merged_into_feature), 8 are Parity (that subject and all 7 counterparts); every class matches the plan's prediction"
  - "merge_resolution_adds_personal_file is fixed: only for the leftover feature/topic-pr: the oracle refuses on a replay conflict before it reaches the merge, so this record says nothing about how devflow treats a merge's resolution changes"
  - "A pre-existing PR branch in a fixture is built in the source worktree and switched away from, never left checked out in another worktree, because git refuses branch -f and checkout of a branch checked out elsewhere, which would change what is recorded"

patterns-established:
  - "fixed(case, expected, recorded, defect) in cut.rs: the defect text names only what the first Parity run's mismatch lines show; the mechanism goes in the scenario's doc comment"

requirements-completed: [HARN-03, HARN-06]

coverage:
  - id: D1
    description: "Quoted personal names: the oracle's PR commit keeps .planning/é.md, a\\nb.md and \\xFF.md (fixed:); a plain-named personal file is stripped (Parity counterpart)"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/cut.rs#cut::quoted_personal_names::oracle"
        status: pass
    human_judgment: false
  - id: D2
    description: "Merge commits in the cut: topic merge dropped (fixed:), integration merge skipped (Parity), resolution-carrying integration merge (fixed: leftover ref), each with a Parity counterpart"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/cut.rs#cut::topic_merge_dropped::oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/cut.rs#cut::integration_merged_into_feature::oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/cut.rs#cut::merge_resolution_adds_personal_file::oracle"
        status: pass
    human_judgment: false
  - id: D3
    description: "Existing PR branch with a reviewer commit is reset by the oracle (fixed:); counterpart with no PR branch is Parity"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/cut.rs#cut::existing_pr_branch_with_extra_commits::oracle"
        status: pass
    human_judgment: false
  - id: D4
    description: "Conflict mid-replay leaves feature/phase-1-pr at the first replay (fixed:); counterpart without the conflicting commit is Parity"
    requirement: HARN-03
    verification:
      - kind: integration
        ref: "tests/harness/cut.rs#cut::conflict_leaves_partial_pr_branch::oracle"
        status: pass
    human_judgment: false
  - id: D5
    description: "Scoped plus unscoped commits: the oracle replays the two scoped commits in order and drops the unscoped one with no warning (fixed:); landed-work counterpart is Parity"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/cut.rs#cut::scoped_and_unscoped_commits::oracle"
        status: pass
    human_judgment: false
  - id: D6
    description: "All seven scenarios pass the scenario rules and list their disabled Rust sides with phase 9 or 10"
    requirement: HARN-03
    verification:
      - kind: unit
        ref: "tests/harness/registry.rs#registry::every_scenario_is_well_formed"
        status: pass
      - kind: other
        ref: "scripts/check.sh all (132 passed, 25 skipped, ==> check.sh: all OK)"
        status: pass
    human_judgment: false

duration: 8min
completed: 2026-10-26
status: complete
---

# Phase 2 Plan 11: Cut HARN-06 and defect scenarios Summary

**Seven PR-cut scenarios now record devflow's cut against the oracle. Six subjects are `fixed:`: the cut leaks quoted personal names, drops a topic merge, resets an existing PR branch, leaves a partial or stray PR branch on a refusal, and silently drops an unscoped commit. The integration-merge subject and all seven counterparts are Parity. Every class matches the plan's prediction.**

## Performance

- **Duration:** about 8 min (2026-10-26T19:07:34Z to 19:15Z)
- **Tasks:** 2 of 2
- **Files modified:** 1 (`tests/harness/cut.rs`)

## Accomplishments

- Shared helpers in `cut.rs` that 02-14 can reuse:
  - fixture steps: `phase_worktree`, `topic_worktree`, `commit_files`, `advance_and_fetch`;
  - case builders: `topic_cut`, `phase_cut`;
  - expectation builders: `commit`, `cut_outcome`, `refusal`, `fixed`.
- `phase_branch` (the self-test's fixture) now calls `phase_worktree`. Its behaviour is unchanged, and `cut::self_test::oracle` still passes.
- Seven scenarios, each with a counterpart control, registered with `rust = "Rust side not enabled until phase 9"` or `phase 10` as the plan assigns.

## Classification of each case (first run as Parity)

| Scenario | Case | Rust expects (requirement) | Oracle observed | Class |
|---|---|---|---|---|
| `quoted_personal_names` | subject: quoted names | exit 0, PR tree = seed + `src/b.txt` (PRCUT-02) | exit 0, PR tree also has the 3 quoted names | `fixed:` |
| | counterpart: `.planning/plain.md` | the same | as expected | Parity |
| `topic_merge_dropped` | subject: merge of `feature/other` | exit 1, unchanged (PRCUT-01) | exit 0, PR with `feat: add b`, `feat: add c` | `fixed:` |
| | counterpart: linear | exit 0, both replayed | as expected | Parity |
| `integration_merged_into_feature` | subject: `origin/develop` merged in | exit 0, `feat: add b` on new develop (PRCUT-01) | as expected | Parity |
| | counterpart: no merge | the same | as expected | Parity |
| `merge_resolution_adds_personal_file` | subject: resolution changes `src/a.txt`, adds `.planning/MERGE-NOTES.md` | exit 1, unchanged (PRCUT-01) | exit 1, `feature/topic-pr` left at `origin/develop` | `fixed:` |
| | counterpart: clean integration merge | exit 0, `feat: change a` replayed | as expected | Parity |
| `existing_pr_branch_with_extra_commits` | subject: PR branch with `fix(1): review fixup` | exit 1, unchanged (PRCUT-08) | exit 0, PR branch reset | `fixed:` |
| | counterpart: no PR branch | exit 0 | as expected | Parity |
| `conflict_leaves_partial_pr_branch` | subject: second commit conflicts | exit 1, unchanged, no PR ref (PRCUT-03) | exit 1, PR branch at first replay | `fixed:` |
| | counterpart: no conflicting commit | exit 0 | as expected | Parity |
| `scoped_and_unscoped_commits` | subject: scoped, unscoped, scoped | exit 1, unchanged (PRCUT-05) | exit 0, scoped commits replayed | `fixed:` |
| | counterpart: landed `fix: landed` + scoped | exit 0 (PRCUT-05's passing case) | as expected | Parity |

The two cases research had not executed (`integration_merged_into_feature`, `merge_resolution_adds_personal_file`) came out as the code reading predicted.

### Mismatch lines of each `fixed:` case's first Parity run, and its defect text

**`quoted_personal_names`** (the oracle's recorded PR tree, which shows which names leaked):
```
1. refs/heads/feature/topic-pr: tree
    expected: ".planning/a\nb.md" absent; ".planning/\xc3\xa9.md" absent; ".planning/\xff.md" absent
    got:      ".planning/a\nb.md" = mode 100644, "personal\n"; ".planning/\xc3\xa9.md" = mode 100644, "personal\n"; ".planning/\xff.md" = mode 100644, "personal\n"
2. new commit 1: tree   (same two lines)
```
Recorded PR tree: `README.md`, `src/a.txt`, `src/b.txt`, `.planning/\xC3\xA9.md`, `.planning/a\nb.md`, `.planning/\xFF.md`. All three quoted names leaked. The script's stdout read `Audit passed: 'feature/topic-pr' carries ZERO personal/agent artifacts.`
Defect text: "feature/topic-pr's commit keeps the quoted-name personal files .planning/\xC3\xA9.md, .planning/a\nb.md and .planning/\xFF.md in its tree".

**`topic_merge_dropped`** (oracle exit 0):
```
1. exit code            expected: 1   got: 0
2. the set of refs      got adds refs/heads/feature/topic-pr
3. the number of new commits   expected: 0 []   got: 2 ["feat: add b\n", "feat: add c\n"]
```
The stdout read `Included: 2 code commit(s)` and `Excluded: 1 planning/environment-only commit(s)`; the merge was the excluded commit.
Defect text: "the cut exits 0 and writes feature/topic-pr with two new commits, feat: add b and feat: add c, the merge of feature/other left out".

**`merge_resolution_adds_personal_file`** (oracle exit 1, the same exit as expected):
```
1. the set of refs   got adds refs/heads/feature/topic-pr
```
The stderr read `error: unresolved code conflict in commit dfda995... on: src/a.txt`. The recorded tip of `feature/topic-pr` is `feat: change a upstream`, which is `origin/develop`'s tip.
Defect text: "the refused cut leaves a new branch feature/topic-pr behind".

**`existing_pr_branch_with_extra_commits`** (the recorded tip of `feature/phase-1-pr` is a new commit `feat(1): add b`, parent `chore: initial commit`):
```
1. exit code   expected: 1   got: 0
2. the tip of refs/heads/feature/phase-1-pr
    expected: unchanged (commit "fix(1): review fixup")
    got:      commit "feat(1): add b"
3. the number of new commits   expected: 0 []   got: 1 ["feat(1): add b\n"]
```
Defect text: "the cut exits 0 and moves feature/phase-1-pr from fix(1): review fixup to a new commit feat(1): add b".

**`conflict_leaves_partial_pr_branch`** (oracle exit 1; the recorded tip of `feature/phase-1-pr` is a new commit `feat(1): add b`, parent `feat: change a upstream`, tree `README.md`, `src/a.txt` = `theirs`, `src/b.txt`):
```
1. the set of refs   got adds refs/heads/feature/phase-1-pr
2. the number of new commits   expected: 0 []   got: 1 ["feat(1): add b\n"]
```
No files difference was reported, so the source checkout ended back on `feature/phase-1` with its files unchanged.
Defect text: "the refused cut leaves a new branch feature/phase-1-pr behind, holding a new commit feat(1): add b".

**`scoped_and_unscoped_commits`** (the recorded replay order is `feat(1): scoped`, then `feat(1-02): second scoped`; the PR tip is the second, with parent `feat(1): scoped`):
```
1. exit code   expected: 1   got: 0
2. the set of refs   got adds refs/heads/feature/phase-1-pr
3. the number of new commits   expected: 0 []   got: 2 ["feat(1): scoped\n", "feat(1-02): second scoped\n"]
```
The full stdout and stderr hold no warning about `fix: unscoped code`. The only diagnostic lines are `Included: 2`, `Excluded: 0` and `Fidelity: 2 path(s) differ from origin/develop, all from replayed commits.` That confirms REQUIREMENTS PRCUT-05 and ROADMAP phase 10 criterion 2 as written; neither was changed.
Defect text: "the cut exits 0 and writes feature/phase-1-pr with two new commits, feat(1): scoped and feat(1-02): second scoped, fix: unscoped code left out".

No defect text names the checkout.

## Negative controls (all run, all reverted byte-identical by sha256, none committed)

- **Quoting is the leak mechanism.** `core.quotePath=false` was set in the quoted-name fixture. The record then failed: `.planning/\xc3\xa9.md` and `.planning/\xff.md` were expected and absent, because they were stripped. `a\nb.md` still leaked, since git quotes a newline regardless of that setting.
- **Records and counterpart expectations are compared, not vacuous.** Every expected author date was shifted by one second (`at()` + 1). All 14 cases, subjects and counterparts, then failed. Their printed stdout also confirmed each class's mechanism:
  - The integration-merge subject showed `Included: 1`, `Excluded: 2`: `feat: add d` replayed to nothing, and the merge listed no files.
  - The topic-merge subject showed `Included: 2`, `Excluded: 1`.
  - The resolution counterpart showed `Included: 1`, `Excluded: 2`.
  - The existing-PR subject showed `Included: 1`.
  - The scoped subject showed `Included: 2`.
- **Misnamed filters and the registry.** Every verify command used `--no-tests=fail` with a single `test(=...)` filter. `registry::every_scenario_is_well_formed` passed with all seven scenarios registered.

## Verification

- Task 1 verify chain (4 oracle tests, `&&`-chained) exited 0. Task 2 verify chain (3 oracle tests, plus the registry test and `scripts/check.sh all`) exited 0.
- `scripts/check.sh all` exited 0 after each commit. The final run had 132 passed and 25 skipped, ending `==> check.sh: all OK`. The fmt gate ran `rustfmt --check` on `cut.rs`, and clippy ran with `-D warnings`.
- The disabled-side report lists all eight `cut::` scenarios:
  - phase 9: `quoted_personal_names`, `topic_merge_dropped`, `integration_merged_into_feature`, `merge_resolution_adds_personal_file`, `conflict_leaves_partial_pr_branch` and `self_test`;
  - phase 10: `existing_pr_branch_with_extra_commits` and `scoped_and_unscoped_commits`.
- Acceptance greps on `cut.rs`:
  - `PathNames`, `Merges`, `ExistingPrBranch`, `ScopedAndUnscoped` and `scoped_and_unscoped_commits` each exit 0.
  - Each exited 1 at the pre-plan tip `3ba78c3`.
- `Cargo.toml` and `Cargo.lock` are unchanged (T-02-SC). The plan's commits touch only `tests/harness/cut.rs`.
- Not run: `scripts/check-in-container.sh`. The container's behaviour on these fixtures is unverified.

## What these checks do not establish

- **One fixture per case.** Each scenario records the oracle on one fixture. The Rust expectations are this plan's readings of PRCUT-01, -02, -03, -05, -06 and -08. They have never run against wsx, and phases 9 and 10 revise them when they enable the Rust sides.
- **`merge_resolution_adds_personal_file` does not record how devflow treats the merge.** The oracle stops at an earlier replay conflict (`feat: change a` against `origin/develop`'s `theirs`), so it never reaches the merge. The matching exit 1 is coincidence, and the only recorded difference is the stray ref. Recording devflow's handling of a resolution-carrying merge would need a fixture whose first replay does not conflict, which this plan's fixture does not provide.
- **Two counterparts do not isolate a difference on the oracle side.**
  - `integration_merged_into_feature`'s counterpart produces the same expected outcome as its subject. It shows that the merge is skipped without effect, not that the outcome changes.
  - `scoped_and_unscoped_commits`'s counterpart passes on the oracle because phase mode skips every unscoped commit. It does not show that the oracle recognises `fix: landed` as already landed.
- **Scope of the quoted-name control.** It shows that quoting causes the leak for the non-ASCII and non-UTF-8 names. For the newline name, it shows only that the leak persists when quoting cannot be turned off.
- **Messages are not pinned.** The `fixed:` records pin refs, commits and files. They do not pin the script's messages. The "no warning" statement rests on reading one run's full stdout and stderr, which are not asserted.

## Decisions Made

- Classes were decided by the first Parity run. All 14 matched the plan's predictions.
- The defect texts name only the differences in the mismatch lines. The mechanism (C-quoting, `diff-tree` listing nothing for a merge, `git branch -f`, the conflict) is in each scenario's doc comment.
- `phase_cut` was added in Task 2's commit rather than Task 1's: unused in Task 1, it would have failed clippy's `-D warnings`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `conflict_leaves_partial_pr_branch` fetches `origin/develop` in the fixture**
- **Found during:** Task 2.
- **Issue:** the plan says `origin/develop` is "advanced" there, but not "fetched", unlike in the merge cases. Unfetched, the cut's own fetch would move `refs/remotes/origin/develop` and add the upstream commit as new. The Rust refusal could then not be `RefsExpect::Unchanged`, which the `refusal()` shape and PRCUT-03's "target ref keeps its pre-cut value" need.
- **Fix:** `advance_and_fetch` is used for the subject and its counterpart, and also for the scoped counterpart's landed commit. The oracle's replay base is the same either way, since it fetches first.
- **Committed in:** `f6833f9`

**2. [Rule 3 - Blocking] The existing PR branch is built in the phase worktree, then left**
- **Found during:** Task 2.
- **Issue:** the plan does not say where `feature/phase-1-pr` is built. If it stays checked out in another worktree, git refuses the oracle's `git branch -f` and `git checkout`, so a different failure would be recorded.
- **Fix:** `checkout --no-track -b feature/phase-1-pr origin/develop` runs in `work/.worktrees/phase-1`, followed by the cherry-pick (`git_tick`) and the reviewer commit. The worktree then goes back to `feature/phase-1`. `--no-track` keeps upstream config out of the fixture.
- **Committed in:** `f6833f9`

**3. [Scope] `phase_branch` reuses the new `phase_worktree`**
- The behaviour is identical, and `cut::self_test::oracle` passes in every full run.
- **Committed in:** `c062443`

**Total deviations:** 3, all within the plan's intent. **Impact:** none on what the scenarios assert.

## Issues Encountered

None.

## Known Stubs

None.

## User Setup Required

None.

## Next Phase Readiness

- 02-14 adds the fork-layout, no-GSD and squash-merge cut scenarios to the same file, and can reuse the helpers above.
- Phases 9 and 10 enable the Rust sides. The `fixed:` records are their negative controls.
- Cleanup: no `target/tmp/harness-*` tree remains. The scratch directory `/tmp/claude-1000/-var-home-denniyahh-Github-wsx/97fae308-abb4-46c4-af94-89ae86614ec1/scratchpad/exec-02-11/` (run logs and file backups) is removed after the final commit.

---
*Phase: 02-oracle-harness*
*Completed: 2026-10-26*

## Self-Check: PASSED

- `tests/harness/cut.rs` exists.
- Commits `c062443` and `f6833f9` are ancestors of HEAD. A control hash, `deadbee`, reports MISSING.
- The commit count measured from the plan ledger is 2 (`3ba78c3..f6833f9`).
