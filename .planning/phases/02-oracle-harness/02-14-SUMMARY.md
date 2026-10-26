---
phase: 02-oracle-harness
plan: 14
subsystem: testing
tags: [rust, nextest, oracle, harness, git, pr-cut]
status: complete

requires:
  - phase: 02-oracle-harness (02-05)
    provides: Class, OracleSide, Role, CaseKind, validate and judge
  - phase: 02-oracle-harness (02-07)
    provides: cut::self_test, the Cut adapter (BASE_REMOTE, WORKSPACE_BASE), the recording rule
  - phase: 02-oracle-harness (02-11)
    provides: cut.rs fixture helpers (commit, cut_outcome, refusal, fixed, topic_cut, phase_cut, advance_and_fetch) and module rules
provides:
  - "cut::fork_layout (Parity, subject exit 0 and counterpart exit 128), cut from upstream/develop"
  - "cut::recut_after_squash_merge (Parity on both cases): the landed commit is excluded, the later one replays onto the squash commit"
  - "cut::no_gsd_repo (Parity, subject exit 0 and dirty-tree counterpart exit 1)"
  - "cut::no_gsd_custom_personal_path, Rust-only, with a counterpart"
  - "cut::merge_resolution_only_personal_file (orchestrator-added): the oracle reaches and excludes a conflicting integration merge (Parity); the counterpart whose resolution adds a code file is fixed:"
affects: [02-15, phase-9, phase-10]

actuals:
  tokens: 4700
  tasks: 2
  commits: 3
plan_head_before: e7bccbeeb5b2786862574d1fcb3ff9b4969b2e0d
plan_head_after: 2efa6ae3af4c4c612778ebbc43ba41b1aeda7d95

tech-stack:
  added: []
  patterns:
    - "A merge conflict whose feature side comes from a workspace commit made before the fork: no replayed commit touches the conflicting path, so the oracle's replay reaches the merge"

key-files:
  created: []
  modified:
    - tests/harness/cut.rs

key-decisions:
  - "Every oracle case was classed by its first run as Parity: 7 Parity and 1 fixed: (the orchestrator-added scenario's counterpart). All 4 of the plan's oracle scenarios matched the predicted Parity"
  - "fork_layout's fixture also advances upstream/develop with feat: add d, so the expected PR parent (feat: add d) tells upstream/develop apart from local develop and the initial commit"
  - "merge_resolution_only_personal_file reads PRCUT-01's 'its own resolution changes' as changes against every parent (git's combined diff). A conflict resolved to develop's side is therefore not one; the added personal file is. Phase 9 should confirm this reading"
  - "The fork counterpart's expected exit is the oracle's observed 128 (NO_BASE_REMOTE_EXIT), as the plan directs; IF-01 revises it when phase 9 enables the Rust side"

patterns-established:
  - "A cut refusal with an exit other than 1 is written Outcome { exit, ..refusal() }"

requirements-completed: [HARN-06]

coverage:
  - id: D1
    description: "Phase-1 cut in a fork layout (base upstream, push origin): exit 0, feature/phase-1-pr at feat(1): add b on upstream/develop's feat: add d; counterpart with upstream removed refuses with exit 128, refs, commits and files unchanged. Both Parity"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/cut.rs#cut::fork_layout::oracle"
        status: pass
    human_judgment: false
  - id: D2
    description: "Re-cut after the first commit landed as the squash commit feat: add b (#1): exit 0, one new commit feat(1): add c whose parent is the squash; counterpart before the squash replays both. Both Parity"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/cut.rs#cut::recut_after_squash_merge::oracle"
        status: pass
    human_judgment: false
  - id: D3
    description: "Cut in a no-GSD repo from a branch made from develop: exit 0, feat: add b replayed; counterpart with an uncommitted change to tracked src/a.txt refuses with exit 1. Both Parity"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/cut.rs#cut::no_gsd_repo::oracle"
        status: pass
    human_judgment: false
  - id: D4
    description: "cut::no_gsd_custom_personal_path registered Rust-only: generated as an ignored ::rust test, no ::oracle test, validated by the registry rule"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/registry.rs#registry::every_scenario_is_well_formed"
        status: pass
      - kind: other
        ref: "cargo nextest list --locked --run-ignored only -E 'test(=cut::no_gsd_custom_personal_path::rust)'"
        status: pass
    human_judgment: false
  - id: D5
    description: "Orchestrator-added: an integration merge whose conflict resolution adds only a personal file is reached and excluded by the oracle (Parity, exit 0); the counterpart whose resolution adds src/MERGE-NOTES.md is fixed: (wsx refuses; the oracle exits 0 and drops it)"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/cut.rs#cut::merge_resolution_only_personal_file::oracle"
        status: pass
    human_judgment: false

duration: 10min
completed: 2026-10-26
---

# Phase 2 Plan 14: Cut fork-layout, squash re-cut and no-GSD scenarios Summary

**Five cut scenarios in `tests/harness/cut.rs` (twelve cases), four of them recorded against devflow's `cut-pr-branch.sh`: fork layout, re-cut after a squash merge, no-GSD repo, and a conflicting integration merge the oracle actually reaches. One Rust-only scenario covers a custom personal path.**

## Performance

- **Duration:** about 10 min
- **Started:** 2026-10-26T19:41:06Z
- **Completed:** 2026-10-26T19:51Z
- **Tasks:** 2, plus 1 orchestrator-added scenario
- **Files modified:** 1

## Accomplishments

Every oracle case was first run as Parity. Observed results:

| Scenario | Case | Oracle exit | Class | Predicted class | Matched |
|---|---|---|---|---|---|
| `cut::fork_layout` | subject | 0 | Parity | Parity | yes |
| `cut::fork_layout` | counterpart (no `upstream` remote) | 128 | Parity | Parity, observed exit | yes |
| `cut::recut_after_squash_merge` | subject | 0 | Parity | Parity | yes |
| `cut::recut_after_squash_merge` | counterpart (not merged yet) | 0 | Parity | Parity | yes |
| `cut::no_gsd_repo` | subject | 0 | Parity | Parity | yes |
| `cut::no_gsd_repo` | counterpart (dirty tracked file) | 1 | Parity | Parity | yes |
| `cut::no_gsd_custom_personal_path` | subject and counterpart | none (Rust-only) | NotImplemented | Rust-only | n/a |
| `cut::merge_resolution_only_personal_file` | subject | 0 | Parity | Parity | yes |
| `cut::merge_resolution_only_personal_file` | counterpart (code file added) | 0 | `fixed:` | `fixed:` | yes |

- **Fork layout.** The subject exits 0. stdout: `==> Fetching latest develop from upstream...`, `rooted at upstream/develop`, `Included: 1 code commit(s)`. The counterpart exits **128**: under `set -e` the script stops at `git fetch upstream develop`, whose stderr is `fatal: 'upstream' does not appear to be a git repository`. It creates nothing.
- **Squash re-cut.** The oracle **excluded** the already-landed `feat(1): add b`. stdout: `2 commits scoped to phase 1`, `Included: 1 code commit(s)`, `Excluded: 1 planning/environment-only commit(s)`. The cherry-pick was empty, so the script counted the commit excluded. It did not replay it empty or refuse. The PR branch tip is a new `feat(1): add c` (Ada, tick 3, committed by the harness identity) whose parent is `feat: add b (#1)`. Its tree is the seed plus `src/b.txt` and `src/c.txt`. The counterpart: `Included: 2`.
- **No-GSD repo.** The subject exits 0 with `feat: add b` replayed onto `origin/develop`. The counterpart exits **1** with stderr `error: working tree has uncommitted changes. Commit or stash them before cutting a PR branch.`. Refs, commits and files are unchanged, and the modified `src/a.txt` is left as it was.
- **Custom personal path (Rust-only).** Its oracle side is `NotImplemented`: devflow's cut strips only its own built-in list and the `only` entries of `.workspace-divergence`.
- **Merge reached by the oracle (orchestrator-added).** stdout for subject and counterpart: `3 commits ahead of fork point`, `Included: 1`, `Excluded: 2`. The excluded two are `feat: change a upstream` (empty on the base) and the merge, for which `diff-tree` lists no files. The loop's only early exit is a conflict, which exits 1, so the run processed the merge. First-run mismatch lines of the counterpart, run as Parity (expected: refusal):
  ```
  1. exit code  expected: 1  got: 0
  2. the set of refs  expected: ...without refs/heads/feature/topic-pr  got: ...refs/heads/feature/topic-pr...
  3. the number of new commits  expected: 0 []  got: 1 ["feat: add b\n"]
  ```
  Defect text: `the cut exits 0 and writes feature/topic-pr with one new commit feat: add b, the merge's added src/MERGE-NOTES.md left out`. It names only what those lines and the recorded outcome show, and does not mention the checkout.

## Task Commits

1. **Task 1: Fork layout and re-cut after a squash merge.** `b9296d0` (test)
2. **Task 2: No-GSD cut scenarios.** `d5a7813` (test)
3. **Orchestrator-added: merge resolution adding only a personal file.** `2efa6ae` (test)

## Files Created/Modified

- `tests/harness/cut.rs`: the five scenarios, their fixtures (`fork_phase_with_b`, `fork_phase_without_upstream`, `phase_after_squash_merge`, `topic_from_develop[_dirty]`, `topic_with_declared_note`, `topic_with_undeclared_scratch`, `topic_merge_resolution_adding`), and their registry entries

## Decisions Made

See `key-decisions` in the frontmatter. In brief:
- `fork_layout` advances `upstream/develop`, which strengthens the check.
- The merge scenario relies on a combined-diff reading of PRCUT-01.
- The fork counterpart's expected exit is the oracle's observed 128.

## Deviations from Plan

**1. [Orchestrator-added] `cut::merge_resolution_only_personal_file`**
- **Why:** 02-11's `cut::merge_resolution_adds_personal_file` stops at an earlier replay conflict, so the oracle never reaches its merge.
- **What:** The conflict's feature side is a `workspace/dev` commit made before the fork (`chore(ws): change a locally`), so no replayed commit touches `src/a.txt`. The resolution takes `origin/develop`'s side and adds `.planning/MERGE-NOTES.md`. The counterpart flips one variable: the resolution adds `src/MERGE-NOTES.md` instead.
- **Classification:** subject Parity, counterpart `fixed:`, both by observation.
- **Commit:** `2efa6ae`

**2. [Rule 2 - stronger check] `fork_layout` advances `upstream/develop`**
- **Why:** The plan's fixture has only `feat(1): add b`. There, a cut from local `develop` or from `upstream/develop` would produce the same PR parent, the initial commit.
- **What:** `upstream/develop` gains `feat: add d`, fetched, so the expected parent `feat: add d` exists only on the base remote.
- **Commit:** `b9296d0`

**Total deviations:** 2, one orchestrator-directed and one auto-added. **Impact:** more coverage; no scope change outside `cut.rs`.

## Verification and its limits

- `scripts/check.sh all` exited 0 (`==> check.sh: all OK`) before each of the three commits.
- Each `::oracle` test listed in the plan passes alone (`1 test run: 1 passed`), and so does `registry::every_scenario_is_well_formed`.
- The full test output lists every new scenario in the disabled-side report. Examples: `cut::no_gsd_repo (Rust side not enabled until phase 9)` and `cut::no_gsd_custom_personal_path (Rust side not enabled until phase 9)`.
- `cargo nextest list --run-ignored only` finds `cut::no_gsd_custom_personal_path::rust`. Selecting its `::oracle` exits 4.
- **Negative controls** (each applied to a scratch copy of the file, then restored):
  - Wrong exit codes in `fork_layout`, `recut_after_squash_merge`, `no_gsd_repo` and `merge_resolution_only_personal_file` each made the oracle test fail (rc 100) with an `exit code` mismatch.
  - Registering `no_gsd_custom_personal_path` as `oracle` made `registry::every_scenario_is_well_formed` fail, with one "register it rust_only" rejection per case.
  - Selecting a misnamed test also exits 4. So the "exit 4" check proves only that no test has that name; the registry rule above is what proves the Rust-only registration.

**What this does not establish:**
- Each scenario records the oracle on one fixture. Research ran none of these, so every record here is a first observation.
- The negative controls perturbed exit codes only. They show that the comparison checks the exit code, not that every ref, commit and file field would catch a change. Those fields rely on the comparator's own tests.
- No Rust side runs. The Rust expectations are readings of PRCUT-01, -02, -05, -06, CONF-02, CONF-04 and IF-02, which phases 9 and 10 revise when they enable those sides. The Rust-only scenario has never executed: its expectation is validated for shape only.
- `merge_resolution_only_personal_file`'s subject expectation depends on the combined-diff reading of PRCUT-01. A stricter reading (any conflict resolution counts as an own change) would make wsx refuse the subject.
- No oracle run failed for a reason unrelated to the case under test.

## Issues Encountered

None.

## Known Stubs

None.

## Next Phase Readiness

The cut's HARN-06 fork, squash-merge and no-GSD cases are recorded. 02-15 can count them in its phase-end rules. Ready for 02-15.

## Self-Check: PASSED

- `tests/harness/cut.rs` exists, and `rg -q` finds `ForkLayout`, `SquashMerged`, `NoGsd` and `recut_after_squash_merge` in it.
- Commits `b9296d0`, `d5a7813` and `2efa6ae` are ancestors of HEAD on `feature/phase-2`.
