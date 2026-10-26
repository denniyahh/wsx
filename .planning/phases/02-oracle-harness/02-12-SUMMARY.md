---
phase: 02-oracle-harness
plan: 12
subsystem: testing
tags: [rust, nextest, oracle, harness, git, sync, worktree]

requires:
  - phase: 02-oracle-harness (02-05)
    provides: Class, OracleSide, Role, CaseKind, validate and judge
  - phase: 02-oracle-harness (02-07)
    provides: Action::NewWorktree oracle mapping (feature/phase-N), worktree::self_test, the recording rule
  - phase: 02-oracle-harness (02-08)
    provides: byte-exact comparator (refs as a set, files per checkout)
provides:
  - "sync::after_squash_merged_pr, sync::fork_layout, sync::no_gsd_repo, each with a counterpart, all Parity"
  - "worktree::rerun_with_own_commits (fixed: exit 1, W1), worktree::no_gsd_repo (fixed: exit 2, IF-02's negative control), worktree::fork_layout (fixed: exit 128), each with a counterpart"
  - "worktree::rm_after_squash_merge, Rust-only (WTREE-04), with an unlanded counterpart"
  - "sync.rs helpers for later plans: NO_GSD_FILES, fixture_commit, synced_files, sync_merge, synced, no_change, land_topic, land_topic_then_squash, advance_upstream"
affects: [02-13, 02-14, 02-15, phase-7, phase-8]

actuals:
  tokens: 7500
  tasks: 2
  commits: 2
plan_head_before: 56d392207ebf59848a50b4549f699c3d0a80ee48
plan_head_after: 2be120f686d7bbbe9ab7dc88f181ead80226f6b1

tech-stack:
  added: []
  patterns:
    - "A new-worktree outcome is a sync outcome (sync::synced) plus feature/phase-1 and its checkout (worktree::with_phase_branch)"
    - "A Rust-only scenario's fixture is exercised by force-running its ignored Rust side once: the stub wsx run must fail only on the fields the action would change"

key-files:
  created: []
  modified:
    - tests/harness/sync.rs
    - tests/harness/worktree.rs

key-decisions:
  - "Every oracle case was classed by its first run as Parity: 7 Parity (all 6 sync cases and the re-run counterpart), 5 fixed: (the three worktree subjects and the two directory-name counterparts)"
  - "worktree::no_gsd_repo's oracle exits 2, not the exit 1 RESEARCH lists for a missing ROADMAP heading: the error path's second grep fails on the missing file and pipefail plus set -e pass its 2 through; recorded as observed"
  - "sync::fork_layout's counterpart expects exit 128 because the plan says to take the oracle's observed code; no requirement fixes the exit code of a sync whose base remote is missing"
  - "DIRECTORY_DEFECT is one constant shared by worktree::self_test and the two new directory-name counterparts, so the defect text cannot drift between them"

patterns-established:
  - "An oracle refusal that creates nothing is recorded as no_change(<observed exit>); the defect text names the exit code and what is missing, the mechanism goes in the doc comment"

requirements-completed: [HARN-05, HARN-06]

coverage:
  - id: D1
    description: "Sync after a squash-merged PR: the squash commit merges cleanly into a workspace branch that already landed the same change (Parity); before the squash it is a no-op (Parity)"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/sync.rs#sync::after_squash_merged_pr::oracle"
        status: pass
    human_judgment: false
  - id: D2
    description: "Sync in a fork layout merges upstream/develop (Parity); with upstream removed it refuses with exit 128 (Parity)"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/sync.rs#sync::fork_layout::oracle"
        status: pass
    human_judgment: false
  - id: D3
    description: "Sync in a no-GSD repo merges, tree holding .wsx.conf and notes/todo.md and no GSD file (Parity); up to date is a no-op (Parity)"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/sync.rs#sync::no_gsd_repo::oracle"
        status: pass
    human_judgment: false
  - id: D4
    description: "Worktree re-run with own commits: oracle exit 1 recorded (fixed:), counterpart fast-forwards (Parity)"
    requirement: HARN-05
    verification:
      - kind: integration
        ref: "tests/harness/worktree.rs#worktree::rerun_with_own_commits::oracle"
        status: pass
    human_judgment: false
  - id: D5
    description: "Worktree in a no-GSD repo: oracle refusal (exit 2, nothing created) recorded as IF-02's negative control; devflow counterpart fixed: for the directory"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/worktree.rs#worktree::no_gsd_repo::oracle"
        status: pass
    human_judgment: false
  - id: D6
    description: "Worktree in a fork layout: oracle exit 128 from fetching origin's missing develop (fixed:); --no-sync counterpart fixed: for the directory"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/worktree.rs#worktree::fork_layout::oracle"
        status: pass
    human_judgment: false
  - id: D7
    description: "worktree::rm_after_squash_merge registered Rust-only and generated as an ignored test; validated by the registry rule"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/registry.rs#registry::every_scenario_is_well_formed"
        status: pass
      - kind: command
        ref: "cargo nextest list --locked --run-ignored only -E 'test(=worktree::rm_after_squash_merge::rust)'"
        status: pass
    human_judgment: false

duration: 9min
completed: 2026-10-26
status: complete
---

# Phase 2 Plan 12: Sync and worktree HARN-06 cases Summary

**Sync is recorded at parity with the oracle after a squash merge, in a fork layout and in a no-GSD repo. On the worktree side, the oracle's re-run refusal (exit 1), its no-GSD refusal (exit 2, IF-02's negative control) and its fork-layout failure (exit 128) are recorded as `fixed:`. Removal after a squash merge exists as a Rust-only scenario.**

## Performance

- **Duration:** about 9 min (2026-10-26T19:19:13Z to 19:28:18Z)
- **Tasks:** 2 of 2
- **Files modified:** 2

## Accomplishments

- Three sync scenarios, each with a counterpart. All six cases are Parity with the oracle on their first run.
- Three oracle-recorded worktree scenarios and one Rust-only scenario, each with a counterpart. Each worktree subject is `fixed:` by observation. The re-run counterpart is Parity, and the other two counterparts are `fixed:` only for the directory name.
- The plan's three predictions about the oracle (W1, the no-GSD refusal and the fork-layout failure) were confirmed by real runs. The no-GSD exit code differs from the one RESEARCH lists (see below).

## Classification of each case (first run as Parity)

| Scenario | Case | Rust expects (requirement) | Oracle observed | Class |
|---|---|---|---|---|
| `sync::after_squash_merged_pr` | subject: squash `feat: add b (#1)` on `origin/develop`, same change landed via `feature/topic` | exit 0, merge `chore: sync develop into workspace/dev`, tree has `src/b.txt` once, `develop` at the squash (WSYNC-01) | as expected | Parity |
| | counterpart: no squash yet | exit 0, nothing changes (WSYNC-02) | as expected, `workspace/dev is already up to date` | Parity |
| `sync::fork_layout` | subject: `upstream/develop` advanced | exit 0, merge of `upstream/develop`, `develop` fast-forwarded (WSYNC-01, CONF-04) | as expected | Parity |
| | counterpart: `upstream` removed | refusal, refs/commits/files unchanged, exit 128 (exit code from the oracle, per the plan) | as expected | Parity |
| `sync::no_gsd_repo` | subject: `origin/develop` advanced | exit 0, merge tree = seed, `src/b.txt`, `notes/todo.md`, `.wsx.conf` (WSYNC-01, IF-02) | as expected | Parity |
| | counterpart: already up to date | exit 0, nothing changes (WSYNC-02) | as expected | Parity |
| `worktree::rerun_with_own_commits` | subject: `feature/phase-1` has `feat(1): work` | exit 0, sync merge, branch and checkout untouched (WTREE-01, WTREE-03) | exit 1, everything else as expected | `fixed:` |
| | counterpart: no own commit | exit 0, branch fast-forwarded to the sync merge | as expected | Parity |
| `worktree::no_gsd_repo` | subject: no-GSD repo | exit 0, sync merge, `feature/phase-1` there, checkout `work/.worktrees/feature/phase-1` (WTREE-01, WTREE-02, IF-02, CONF-06) | exit 2, nothing changed | `fixed:` |
| | counterpart: devflow layout | the same in the devflow layout | checkout at `work/.worktrees/phase-1` | `fixed:` |
| `worktree::fork_layout` | subject: fork, sync on | exit 0, sync merge of `upstream/develop`, branch and checkout (WTREE-01, CONF-04) | exit 128, nothing changed | `fixed:` |
| | counterpart: `--no-sync` | exit 0, branch at the `workspace/dev` tip, no merge | checkout at `work/.worktrees/phase-1` | `fixed:` |
| `worktree::rm_after_squash_merge` | subject: landed and squash-merged | exit 0, `feature/topic` and its checkout gone, other refs unchanged (WTREE-04) | NotImplemented | Rust-only |
| | counterpart: not on `origin/develop` | exit 1, nothing changes | NotImplemented | Rust-only |

**Difference from the predictions:** the no-GSD worktree refusal exits **2**, not the exit 1 that RESEARCH lists for a missing `### Phase N:` heading. The plan predicted only "refuses before creating anything", which held.
- **Mechanism:** the `if ! grep -qE ...` test fails on the missing file. The error path then runs `grep -nE ... .planning/ROADMAP.md | tail -5`, and that grep exits 2 on the missing file. `pipefail` makes 2 the pipeline's status, and `set -e` exits with it.
- **Where the 1 comes from:** RESEARCH's exit 1 is the path where `ROADMAP.md` exists but lacks the heading.

All other classes match the plan's predictions.

### Recorded oracle outcomes the acceptance criteria ask for

**`sync::fork_layout` counterpart (`upstream` removed), exit 128.** From a run with the expected exit perturbed to 1:
```
1. exit code   expected: 1   got: 128
stdout: ==> Fetching latest develop from upstream...
stderr: fatal: 'upstream' does not appear to be a git repository
        fatal: Could not read from remote repository.
```

**`worktree::no_gsd_repo` subject (IF-02's negative control), exit 2.** The stdout was empty, and the stderr read:
```
grep: .planning/ROADMAP.md: No such file or directory
error: no '### Phase 1:' heading in .planning/ROADMAP.md.
  Phases are sometimes renumbered (36 -> 35.3); check the heading first:
grep: .planning/ROADMAP.md: No such file or directory
```
It changed 17 fields relative to the Rust expectation: the exit code, the ref set (no `feature/phase-1`), `develop`, `workspace/dev` and `origin/develop` (none moved), the new-commit count (0) and the files. The record is `no_change(2)`.
Defect text: "the oracle exits 2 with no '### Phase 1:' heading in .planning/ROADMAP.md and creates nothing: no sync merge, no feature/phase-1, no worktree".

**`worktree::fork_layout` subject, exit 128.** The script got as far as its sync step:
```
stdout: ==> phase 1: ### Phase 1: Example
        ==> syncing workspace/dev
        ==> Fetching latest develop from origin...
stderr: fatal: couldn't find remote ref develop
```
The record is `no_change(128)`.
Defect text: "the oracle fetches develop from origin, not from the base remote upstream, exits 128 and creates nothing: no sync merge, no feature/phase-1, no worktree".

**`worktree::rerun_with_own_commits` subject, exit 1.** There was exactly 1 difference, the exit code (expected 0, got 1). The sync ran (`==> syncing workspace/dev`, `Merge made by the 'ort' strategy.`). The script then printed `==> worktree already exists: .../work/.worktrees/phase-1`, and its stderr ended with `fatal: Not possible to fast-forward, aborting.` and `not a fast-forward -- the branch has its own commits.`
Defect text: "the oracle exits 1 when feature/phase-1 has a commit of its own".

**Directory-name counterparts** (`worktree::no_gsd_repo` devflow, `worktree::fork_layout` `--no-sync`). Each run had exactly 1 difference, the files on disk: every file expected under `work/.worktrees/feature/phase-1` was under `work/.worktrees/phase-1` instead. The defect text is the shared `DIRECTORY_DEFECT`, word for word the text of `worktree::self_test`.

**WSX_ORACLE_DIR trap (orchestrator note):** this was checked on stdout, not on the exit code alone. Every sync-on run printed `==> syncing workspace/dev` and then `==> Fetching latest develop from ...`. None printed `not found; skipping sync`.

## Negative controls (all run, all reverted byte-identical by sha256, none committed)

- **Sync records are compared, not vacuous:**
  - Moving the squash commit one tick later failed `after_squash_merged_pr` on its author date in three places: `develop`, `origin/develop` and new commit 1.
  - Changing `.wsx.conf` to `only notesX` failed `no_gsd_repo` on the tree of `workspace/dev`, new commit 2 and the files on disk.
  - Flipping each counterpart's expected exit code failed each one on the exit code alone, which printed `already up to date` for both no-op counterparts.
- **The fork sync really uses `upstream`:** naming `origin` as the sync remote in the fork subject's expectation failed on the ref set. The run had `refs/remotes/upstream/{HEAD,develop}` and no `origin/develop`, and its stdout read `Merging upstream/develop into workspace/dev`.
- **Worktree records are compared:** the recorded exits were perturbed (1 to 3, 2 to 1, 128 to 1). Each failed on the exit code alone, against the observed 1, 2 and 128.
- **The Rust-only fixture builds and means what it says:**
  - The ignored `worktree::rm_after_squash_merge::rust` was force-run once with `cargo nextest run --run-ignored only`. Both fixtures built: there was no `a fixture step failed`. The stub `wsx` exited 2 (`usage: wsx --version`).
  - The subject's only differences were the exit code, the leftover `refs/heads/feature/topic` and the leftover checkout files. Its expectation about `work`'s files and the other refs therefore matches the fixture.
  - A probe pinned `refs/remotes/origin/develop` to `feat: add b (#1)` at tick 4 and added no difference, while the same probe at tick 5 failed on the author date. The squash commit is therefore fetched before the action.
- **Listing:** the plan's `cargo nextest list --run-ignored only` check found `worktree::rm_after_squash_merge::rust`. The same check for a misnamed `::nonexistent` exited 1, and no `rm_after_squash_merge::oracle` test exists (`rg` exit 1).

## Task Commits

1. **Task 1:** `470f2c0`, `test(02-12): record sync after a squash merge, in a fork layout and without GSD`
2. **Task 2:** `2be120f`, `test(02-12): record worktree re-run, no-GSD and fork cases, add removal after squash`

## Files Created/Modified

- `tests/harness/sync.rs`: three HARN-06 scenarios, and helpers made `pub` for reuse (`sync_merge`, `synced_files`, `synced`, `no_change`, `fixture_commit`, `advance_upstream`, `land_topic`, `NO_GSD_FILES`).
- `tests/harness/worktree.rs`: four scenarios, the shared `DIRECTORY_DEFECT`, and `with_phase_branch` / `synced_new_worktree`.

## Verification

- Task 1 verify: the three `sync::*::oracle` tests (one command each) and `registry::every_scenario_is_well_formed` all exited 0. `scripts/check.sh all` exited 0 with 135 passed and 28 skipped.
- Task 2 verify:
  - The `&&` chain of the three worktree oracle tests exited 0.
  - The ignored-test listing check exited 0.
  - The registry test followed by `scripts/check.sh all` exited 0, with 138 passed and 32 skipped, ending `==> check.sh: all OK`. That is 3 more run and 4 more skipped than after Task 1: three oracle tests and four ignored Rust sides.
- Acceptance greps: `SquashMerged`, `ForkLayout` and `NoGsd` in `sync.rs`, and `RemoveWorktree`, `NoGsd` and `ForkLayout` in `worktree.rs`, each exit 0. Each exited 1 at the pre-plan tip `56d3922`.
- `Cargo.toml` and `Cargo.lock` are unchanged (T-02-SC), and every cargo call used `--locked`.
- Not run: `scripts/check-in-container.sh`. These fixtures' behaviour in the pinned image is unverified.

## What these checks do not establish

- **One fixture per case.** Each scenario records the oracle on one fixture. The Rust expectations are readings of WSYNC-01/02, WTREE-01 to -04, CONF-04, CONF-06 and IF-02 that have never run against wsx. Phases 7 and 8 revise them when they enable the Rust sides.
- **`worktree::rm_after_squash_merge` is unproven beyond its fixture.** Its expectation was compared only with the stub `wsx`, which shows the fixture is consistent with it. Nothing shows that a real `rm` can tell a squash-landed change from an unlanded one.
- **The `fixed:` records pin state and exit codes, not messages.** The reasons (`not a fast-forward`, the missing heading, `couldn't find remote ref develop`) come from reading one run's stderr each, and nothing asserts them.
  - `rerun_with_own_commits` differs from its expectation only in the exit code. An oracle that exited 1 for another reason, with the same final state, would still match the record.
- **Two exit codes are git's or bash's, not devflow's.** The 128 in both fork cases is git's fetch failure code, and the 2 in `worktree::no_gsd_repo` is grep's. A different git or grep could change them.
  - The sync fork counterpart's expected 128 comes from the oracle run, as the plan directs. No requirement fixes it, so phase 7 may choose another non-zero code.
- **The squash case covers identical content only.** The landed change and the squash add byte-identical `src/b.txt`. A squash whose content differs from what the workspace landed, which would conflict, is not covered.
- **The no-GSD sync shows the oracle needs none of those files for this merge**, not for sync in general (a conflict path, for example).

## Decisions Made

- Classes were decided by the first Parity run, as listed in the table above.
- The defect texts name only what the mismatch lines show. The mechanisms (hard-coded `origin`, `pipefail` on the error-path grep, `merge --ff-only`) are in the doc comments.
- The fixture details the plan left open:
  - The re-run's own commit adds `src/w.txt`.
  - The no-GSD worktree subject advances `origin/develop`, so its expectation exercises the sync as well as the creation.
  - The re-run's existing worktree is at `work/.worktrees/phase-1`, so both sides find it at the same path.

## Deviations from Plan

### Auto-fixed Issues

**1. [Scope] Task 2 also changed `tests/harness/sync.rs`**
- **Found during:** Task 2.
- **Issue:** the worktree outcomes after a sync are the sync outcome plus the new branch and checkout. Building them in `worktree.rs` would have duplicated the merge and ref list a third time.
- **Fix:** `sync_merge` and `synced_files` were extracted from `synced`, and `synced`, `no_change` and `advance_upstream` were made `pub`. The sync scenarios' behaviour is unchanged: all `sync::*` tests pass in the Task 2 `check.sh all` run. The plan's `files_modified` lists `sync.rs`; only Task 2's `<files>` did not.
- **Committed in:** `2be120f`

**2. [Scope] `worktree::self_test` uses the shared `DIRECTORY_DEFECT`**
- The string is identical to the literal it replaces, so the defect text cannot drift between the three directory-name cases. `worktree::self_test::oracle` passes.
- **Committed in:** `2be120f`

**Total deviations:** 2, both within the plan's intent. **Impact:** none on what any scenario asserts.

## Issues Encountered

None.

## Known Stubs

None.

## User Setup Required

None.

## Next Phase Readiness

- 02-13 (divergence check) and 02-14 (cut) can reuse `sync::NO_GSD_FILES`, `fixture_commit`, `advance_upstream`, `land_topic_then_squash` and `no_change`.
- Phase 7 enables the sync Rust sides; all six sync cases are Parity. Phase 8 enables the worktree sides; the three `fixed:` subjects are its negative controls, and `rm_after_squash_merge` is its first `rm` scenario.
- Cleanup: no `target/tmp/harness-*` tree remains (`rg -q '^harness-'` exit 1, with a `^cargo-` control exit 0). The scratch directory `/tmp/claude-1000/-var-home-denniyahh-Github-wsx/97fae308-abb4-46c4-af94-89ae86614ec1/scratchpad/exec-02-12/` (run logs and file backups) is removed after the final commit.

---
*Phase: 02-oracle-harness*
*Completed: 2026-10-26*

## Self-Check: PASSED

- FOUND: `tests/harness/sync.rs`, `tests/harness/worktree.rs`
- FOUND: commits `470f2c0` and `2be120f` (both ancestors of HEAD; `check evaluation-scope --plan 02-12 --commits-only` resolves both)
