---
phase: 02-oracle-harness
plan: 07
subsystem: testing
tags: [rust, nextest, oracle, harness, git, bash]

requires:
  - phase: 02-oracle-harness (02-02)
    provides: oracle adapter, run_script, sync::self_test shape
  - phase: 02-oracle-harness (02-03)
    provides: Action, push_input, fixture presets, observer and comparator
  - phase: 02-oracle-harness (02-05)
    provides: Class, OracleSide, Role, controls, validate and judge
provides:
  - Oracle mappings for Action::Check, Action::NewWorktree (feature/phase-N only), Action::Cut (numeric scopes) and Action::PrePush
  - check::self_test (guard, known-positive and clean controls, Parity)
  - worktree::self_test (fixed: oracle names the directory after the phase number)
  - cut::self_test (Parity by observation)
  - guard::self_test (guard, known-positive and clean controls, Parity)
affects: [02-08, 02-09, 02-10, 02-11, 02-12, 02-13, 02-14, phase-5, phase-6, phase-8, phase-9]

actuals:
  tokens: 5800
  tasks: 2
  commits: 4
plan_head_before: 19713413d6405031be0d1b8647999ddf1be7d10c
plan_head_after: 1e740b03feda9f876a7cce61433953e7fdbb0131

tech-stack:
  added: []
  patterns:
    - "Per-child script variables (WORKSPACE_BASE, BASE_REMOTE) go through oracle::with_vars on a cloned Env, never the process environment"
    - "An oracle mapping returns NotImplemented with the reason when the script's CLI cannot express the action (non-phase branch, non-numeric scope, non-develop integration branch without a PR name)"

key-files:
  created: []
  modified:
    - tests/harness/oracle.rs
    - tests/harness/check.rs
    - tests/harness/worktree.rs
    - tests/harness/cut.rs
    - tests/harness/guard.rs
    - tests/harness/sync.rs

key-decisions:
  - "worktree::self_test is fixed: by observation; its only difference is the directory: the oracle uses .worktrees/phase-1, wsx (CONF-06) .worktrees/feature/phase-1"
  - "cut::self_test is Parity by observation: the first oracle run met the PRCUT-01/02/06 expectation, as the plan's 2026-10-26 measurement predicted"
  - "sync.rs's fixture files and helpers (SEED_FILES, WORKSPACE_FILES, B_FILE, advance_develop, tree, date) are pub and reused by worktree.rs and cut.rs instead of being duplicated"

patterns-established:
  - "A guard self-test proves its refusal reason, not just its exit code: the negative-control runs printed the oracle's own refusal text"

requirements-completed: [HARN-01, HARN-05]

coverage:
  - id: D1
    description: "Divergence check mapped into the oracle adapter; check::self_test passes, with the check seen refusing (known-positive, exit 1) and passing (subject and clean control, exit 0)"
    requirement: HARN-05
    verification:
      - kind: integration
        ref: "tests/harness/check.rs#check::self_test::oracle"
        status: pass
    human_judgment: false
  - id: D2
    description: "phase-worktree.sh mapped for feature/phase-N; worktree::self_test passes against its fixed: record, and a skipped sync fails it"
    requirement: HARN-01
    verification:
      - kind: integration
        ref: "tests/harness/worktree.rs#worktree::self_test::oracle"
        status: pass
    human_judgment: false
  - id: D3
    description: "Pre-push hook driven the way git drives it; guard::self_test passes, with the hook seen refusing .planning/x.md (exit 1) and passing src/-only and workspace/dev pushes"
    requirement: HARN-05
    verification:
      - kind: integration
        ref: "tests/harness/guard.rs#guard::self_test::oracle"
        status: pass
    human_judgment: false
  - id: D4
    description: "PR cut mapped for numeric scopes; cut::self_test passes as Parity"
    requirement: HARN-01
    verification:
      - kind: integration
        ref: "tests/harness/cut.rs#cut::self_test::oracle"
        status: pass
    human_judgment: false
  - id: D5
    description: "All five self-tests pass in the pinned container image"
    requirement: HARN-01
    verification:
      - kind: integration
        ref: "scripts/check-in-container.sh test (mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie): 101 passed, 6 skipped"
        status: pass
    human_judgment: false

duration: 37min
completed: 2026-10-26
status: complete
---

# Phase 2 Plan 07: Remaining oracle mappings and self-tests Summary

**The divergence check, phase-worktree, PR cut and pre-push hook now run through the oracle adapter, each with a passing self-test. Both guards are seen refusing their known-positive control and passing their clean control, on the host and in the pinned container image.**

## Performance

- **Duration:** 37 min wall clock (2026-10-26T17:47:54Z to 18:25:14Z), about 29 min of it an API rate-limit pause between the pre-push controls and the cut scenario
- **Tasks:** 2 of 2
- **Files modified:** 6

## Accomplishments

- `oracle::run` maps the four remaining actions:
  - `Check` runs `check-workspace-divergence.sh <base_remote>/<integration>`.
  - `NewWorktree` runs `phase-worktree.sh <N> [--no-sync]` with `WORKSPACE_BASE`, for `feature/phase-<N>` only.
  - `Cut` runs `cut-pr-branch.sh` with `BASE_REMOTE` and `WORKSPACE_BASE`, plus `--phase <N>` and the positionals as needed.
  - `PrePush` runs `bash hooks/pre-push <remote> <url>`, with the stdin from `Action::push_input`.
- Four self-tests, each registered with an ignored Rust side and listed in the disabled-side report. Together with `sync::self_test`, every vendored script now has one (HARN-01).
- HARN-05 on both guards: `check::self_test` and `guard::self_test` each carry a known-positive control that must be refused and a clean control that must pass.

## Task Commits

1. **Task 1 RED:** `0ebddce`, `test(02-07): add failing divergence-check self-test`
2. **Task 1 GREEN:** `c3d5973`, `feat(02-07): map the divergence check and phase-worktree into the oracle adapter`
3. **Task 2 RED:** `ad242b7`, `test(02-07): add failing pre-push self-test`
4. **Task 2 GREEN:** `1e740b0`, `feat(02-07): map the pre-push hook and the PR cut into the oracle adapter`

No REFACTOR commits.

## TDD Gate Compliance

- **RED, Task 1** (`0ebddce`):
  - Command and exit: `scripts/check.sh test -E 'test(=check::self_test::oracle)'` exited 100.
  - Report: JUnit `target/nextest/default/junit.xml`, written 2026-10-26T17:50:05.666Z, after the run started at 17:50:04.230Z.
  - Record: `target/gsd/02-07-red-evidence.json` (gitignored).
  - Classifier verdict: **`RED_EVIDENCE_OK`** (`target_test_failed`, matched `wsx::harness#check::self_test::oracle`, 1 test, 1 fail).
  - Semantic assessment: the test compiled with no warnings and validation passed. All three cases built their fixtures, then failed with `not implemented: divergence check in work`. That is the planned missing mapping.
- **RED, Task 2** (`ad242b7`):
  - Command and exit: `scripts/check.sh test -E 'test(=guard::self_test::oracle)'` exited 100.
  - Report: JUnit, written 17:53:11.539Z, after the run started at 17:53:09.925Z.
  - Record: `target/gsd/02-07-red-evidence-2.json`.
  - Classifier verdict: **`RED_EVIDENCE_OK`** (`target_test_failed`, matched `wsx::harness#guard::self_test::oracle`).
  - Semantic assessment: all three cases built their fixtures, including the develop worktree and both branches, then failed with `not implemented: pre-push guard ...`.
- Each `test(02-07)` commit precedes its `feat(02-07)` commit.
- `worktree::self_test` and `cut::self_test` were written in the GREEN commits, as the plan's actions specify. They have no RED step of their own.

## Classification of each case (recording rule)

Each case was run once as Parity, and its class follows from that run, not from the plan's prediction.

| Scenario | Class | Observation |
|---|---|---|
| `check::self_test` (3 cases) | Parity | Passed on the first run. |
| `guard::self_test` (3 cases) | Parity | Passed on the first run. |
| `worktree::self_test` | `fixed:` | The first run reported exactly one difference, in the files on disk: every file expected under `work/.worktrees/feature/phase-1` was under `work/.worktrees/phase-1` instead. Refs, both new commits (`feat: add b`, then the sync merge) and the exit code matched. The record is the expectation with that checkout name swapped. Defect text: "the oracle names the worktree directory after the phase number (.worktrees/phase-1), not after the branch as given (.worktrees/feature/phase-1)". The class matches the plan's prediction. |
| `cut::self_test` | **Parity** | Passed on the first run. Reason: devflow's script checks `feature/phase-1-pr` out in the source worktree only while it replays, and returns to `feature/phase-1` before it exits. Its final state therefore equals the expectation: a new `feature/phase-1-pr` at one commit, every other ref unchanged and the files unchanged. The class matches the plan's prediction. |

The passing `cut::self_test::oracle` run:

```
        PASS [   0.333s] (1/1) wsx::harness cut::self_test::oracle
────────────
     Summary [   0.334s] 1 test run: 1 passed, 106 skipped
==> check.sh: test OK
```

## Negative controls (all run, all reverted, none committed)

- **Divergence check:** the subject was left without its `only notes` declaration but still expected exit 0. The run failed on its exit code, and the check's stderr read `undeclared workspace-only path: notes/todo.md (declare: only notes/todo.md)`. The known-positive's exit 1 therefore comes from the missing declaration, not from an infrastructure error.
- **Sync trap (RESEARCH Pitfall 3):** `--no-sync` was forced in the adapter. `worktree::self_test::oracle` then failed with 20 differences, among them the missing merge on `workspace/dev` and `feature/phase-1` and the new-commit count. A run that skips its sync cannot pass. On the real run, the script's stdout shows `==> syncing workspace/dev`.
- **Pre-push:** two expectations were perturbed so the report would print each case's output.
  - The known-positive control printed `pre-push: REFUSING to push personal/agent artifacts to 'refs/heads/feature/leak': .planning/x.md`.
  - The clean control printed `check-workspace-divergence: every difference from origin/develop is declared.`, which proves the patched divergence-check tail ran.
- **Cut:** the replayed commit's expected author date was moved forward one tick. The run failed on exactly that field: `refs/heads/feature/phase-1-pr: author date` and `new commit 1: author date`, expected `1767225780`, got `1767225720`. Its stdout showed `Included: 1`, `Excluded: 1` and `Audit passed`.
- **Ignored-test listing:** the same `cargo nextest list` pipeline exits 1 for the misnamed `check::self_test::nonexistent`.
- **Disabled-side report:** a search for a wrong reason (`guard::self_test (... phase 9)`) exits 1.
- **First-run validation:** an attempt to flip the known-positive control's expected exit to 0 was rejected by `validate` before anything ran. That confirms the 02-05 rule fires.

## Verification

- Task 1: both oracle tests exit 0, and both Rust sides appear in the ignored-test listing. `scripts/check.sh all` passed with 99 tests run and 4 skipped.
- Task 2: both oracle tests exit 0. The disabled-side report lists all four new self-tests with their phases (5, 6, 8 and 9). `scripts/check.sh all` passed with 101 tests run and 6 skipped, ending `==> check.sh: all OK`.
- Container: `scripts/check-in-container.sh test` exited 0 in `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`, the image the `.devcontainer/Dockerfile` FROM line names. 101 passed and 6 skipped, the last line was `==> check.sh: test OK`, and all five `*::self_test::oracle` tests passed. This was the first run of all five scripts there. The image's mawk, as opposed to the host's gawk, caused no failure.
- Acceptance greps all returned as specified. `Recorded` and `Fixed` are in `worktree.rs`, `KnownPositive` and `Clean` are in `check.rs`, `push_input` and `BASE_REMOTE` are in `oracle.rs`, and `KnownPositive` is in `guard.rs`. `Action::Cut` is in `cut.rs`, and `Recorded` is absent from `cut.rs` (exit 1).

## What these checks do not establish

- Each self-test proves that its script runs, through the adapter, on **one fixture shape**. The oracle's behaviour on the HARN-06 cases is recorded in plans 02-10 to 02-14.
- The Parity classification of `cut::self_test` rests on the files-unchanged expectation. A checkout left on the PR branch would drop the source branch's `.planning` files from the checkout. That a missing file is reported is proven by 02-08's mutation test, not here.
- The `fixed:` record of `worktree::self_test` covers only the directory name. The phase-worktree behaviours that need no phase number in the roadmap, or a branch not named after a phase, are NotImplemented on the oracle and Rust-only by design.
- The subject of `guard::self_test` passes because nothing in the scanned range is forbidden. Evidence that the hook actually scanned it is indirect: the known-positive control uses the same stdin plumbing with a different branch and is refused.
- One container run is a single sample, not a reliability bound. Hermeticity under a planted host environment is 02-09's job.

## Decisions Made

- `worktree::self_test` is `fixed:`, and `cut::self_test` is Parity. Both were decided by observation, and both match the plan's prediction.
- A guard self-test uses the same fixture steps for all three cases where possible (`guard`). In the divergence check, the known-positive control reuses the subject's fixture step without the declaration.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Exposed sync.rs fixture items for reuse**
- **Found during:** Task 1, while writing `worktree::self_test`.
- **Issue:** the worktree and cut expectations need the same seed, workspace and `src/b.txt` files, the `advance_develop` step, and the `tree`/`date` helpers that `sync.rs` holds privately. `sync.rs` is not in the plan's `files_modified`.
- **Fix:** made those seven items `pub` (a visibility change only), so nothing is duplicated.
- **Files modified:** `tests/harness/sync.rs`
- **Committed in:** `c3d5973`

**2. [Rule 3 - Blocking] The divergence-check fixture adds `notes/todo.md` in its own workspace commit**
- **Found during:** Task 1.
- **Issue:** the behaviour list says "the workspace commit also adding `notes/todo.md`". The fixture API cannot add files to the preset's commit 1.
- **Fix:** a fixture step adds a second workspace commit, `chore(ws): add a note`. The check compares `origin/develop` with `HEAD`, so its diff is identical either way.
- **Committed in:** `0ebddce`

**3. [Rule 1 - Structure] The `not_implemented` closure in `oracle::run` became a free function**
- **Found during:** Task 2.
- **Issue:** the cut mapping is long enough to live in its own `run_cut`, which needs the same NotImplemented constructor.
- **Fix:** a module-level `not_implemented(action, why)`, used by every arm. Behaviour is unchanged.
- **Committed in:** `1e740b0`

**Total deviations:** 3, all handled within the plan's intent.
**Impact on plan:** none on what the scenarios assert.

## Issues Encountered

- An API rate limit paused execution for about 29 minutes after the pre-push negative controls. The orchestrator confirmed the state on resume. No work was lost or redone.

## Known Stubs

None.

## User Setup Required

None.

## Next Phase Readiness

- Every vendored script now has an adapter mapping and a self-test, so 02-09 can run all five under a planted host.
- The mapping limits that later plans must respect:
  - `NewWorktree` works only for `feature/phase-<N>` branches.
  - `Cut` accepts only numeric scopes, and needs a PR branch name to target an integration branch other than `develop`.
  - A cut with no scope from a `feature/phase-<N>` branch still runs in phase mode.
- Cleanup: no `target/tmp/harness-*` trees remain. The scratch directory `/tmp/claude-1000/-var-home-denniyahh-Github-wsx/97fae308-abb4-46c4-af94-89ae86614ec1/scratchpad/exec-02-07/` holds this plan's logs and helper scripts and is removed after the final commit.

---
*Phase: 02-oracle-harness*
*Completed: 2026-10-26*

## Self-Check: PASSED

- The four modified harness files and `tests/harness/sync.rs` exist, and all are changed in `19713413..1e740b0`.
- Commits `0ebddce`, `c3d5973`, `ad242b7` and `1e740b0` are all ancestors of HEAD, and the commit count measured from the plan ledger is 4.
