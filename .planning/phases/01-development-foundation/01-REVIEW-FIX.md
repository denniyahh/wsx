---
phase: 01-development-foundation
fixed_at: 2026-10-23T20:39:33Z
review_path: .planning/phases/01-development-foundation/01-REVIEW.md
iteration: 1
findings_in_scope: 7
fixed: 7
skipped: 0
status: all_fixed
---

# Phase 1: Code Review Fix Report

**Fixed at:** 2026-10-23T20:39:33Z
**Source review:** .planning/phases/01-development-foundation/01-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 7 (CR-02, CR-03, WR-01, WR-02, WR-03, WR-04, IN-01, as set by the orchestrator)
- Fixed: 7
- Skipped: 0
- Out of scope by instruction and untouched: CR-01 (the `.gitconfig` include design, for the owner to decide), IN-02, IN-03, IN-04

Every fix has a negative control: the old code fails it and the new code passes it. Where the code reaches develop, the control is a Rust test. For `scripts/workspace/` it is a shell control in a scratch repository, because tests must not reference workspace-only scripts.

## Verification

- **Where it ran:** directly in the phase checkout `/var/home/denniyahh/Github/wsx/.worktrees/phase-1` on `feature/phase-1`. `workflow.use_worktrees` is `false`, so no isolated agent worktree was created, and the results can be reproduced from this tree.
- **Per-commit gate:** each commit was gated as `scripts/check.sh all && git commit`. The suite grew from 28 tests to 38, all passing.
- **Container gate:** `scripts/check-in-container.sh all` ran once after the last commit and exited 0, with 38/38 tests passing in `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`.
- **Deps gate:** `scripts/check.sh deps` exited 0 on the host. It is not part of `all`, and its pin check changed in this run.

**What this does not establish:**
- The `cargo-deny` side of CR-02 has only a shell control, not a Rust test. CI's Test job does not install cargo-deny, so a test that needed the real binary would fail there.
- Each control proves the exact case it builds. It does not prove the whole class of inputs.

## Fixed Issues

### CR-02: The pin check verifies one binary while cargo runs another

**Files modified:** `scripts/install-tools.sh`, `scripts/check.sh`, `tests/gate_controls.rs`
**Commit:** d179d7a
**Applied fix:**
- **New mode:** `install-tools.sh verify-cargo TOOL` asks cargo itself (`cargo nextest --version`, `cargo deny --version`), so the binary checked is the binary the gate runs. It refuses tools that are not `cargo-*`. The version strings remain only in `install-tools.sh`.
- **check.sh:** both pin checks now call `verify-cargo`, without the `PATH` prepend. The comments that said cargo searches `$CARGO_HOME/bin` first are corrected.
- **New test:** `gate_controls::nextest_pin_is_checked_on_the_binary_cargo_runs`.

**Negative control:**
- **Nextest setup:** the pinned nextest is in `$CARGO_HOME/bin`, which is on `PATH`. An earlier `PATH` entry holds a stub. Run directly with `--version`, the stub hands off to the real pinned binary. Run by cargo, it reports `cargo-nextest 0.0.0-stub`.
- **Old check.sh:** the test failed. The old script printed `cargo-nextest 0.9.148 OK`, then `STUB cargo-nextest ran: nextest run ...`, then `check.sh: test OK`, and exited 0.
- **New check.sh:** the test passes. The script exits 1 with `cargo-nextest run as 'cargo nextest' reports 'cargo-nextest 0.0.0-stub'`.
- **Deps (shell):** with the same kind of stub, the old `check.sh deps` printed `FAKE cargo-deny ran with: deny --locked check` and `deps OK`, exit 0. The new one exits 1 with `reports 'cargo-deny 0.0.0-stub'`.

### CR-03: The PR cut silently drops code commits whose cherry-pick fails

**Files modified:** `scripts/workspace/cut-pr-branch.sh`
**Commit:** e9a25e6
**Applied fix:** a cherry-pick that fails and leaves no unmerged entries now aborts the cut. The script exits 1, names the commit and prints git's error. Conflicts keep the existing handling. Following the minimal-fix guidance, the review's other two suggestions were not applied: refusing untracked files up front, and widening the fidelity check. The abort already catches the reproduced case.

**Negative control (shell, scratch repo):**
- **Old script:** the reviewer's scenario (an untracked `.planning/state.json` blocks the pick of `feat(09-01)`) exited 0 with `Included: 1`. The PR branch held only `A src/b.rs`, so `src/a.rs` was lost.
- **New script:** exits 1 with `cherry-pick of commit <hash> failed without a conflict; the cut is aborted:` and git's `untracked working tree files would be overwritten`.

**Checks that existing behaviour survives:**
- **Forbidden-path conflict:** the pick exits 1 with `.claude/s` unmerged. Old and new both cut successfully, with `A src/c.rs`.
- **Already-applied commit:** a commit whose content is already on develop is still counted as excluded. Old and new behave identically.

### WR-01: `check.sh test` accepts several selectors

**Files modified:** `scripts/check.sh`, `tests/gate.rs`, `CONTRIBUTING.md`
**Commit:** 48b3dd0
**Applied fix:**
- **Selector limit:** `check.sh test` takes at most one `-E`/`--filterset` expression or test name, and exits 2 on a second one.
- **Docs:** the usage text and the CONTRIBUTING.md command table, which advertised `[name...]`, are updated. The CLAUDE.md wording was still accurate and is unchanged.
- **New tests:** `gate::second_filterset_is_refused` and `gate::second_test_name_is_refused`.

**Negative control:**
- **Reviewer's commands:** on the old script, `-E 'test(=version_flag_prints_package_version)' -E 'test(=no_such_xyz)'` and the two-name form each exited 0 with `test OK`. On the new script both exit 2.
- **New tests on the old check.sh:** both fail.
- **Hazard found during the control:** a first version of the name test used the short name `a`. Against the old check.sh, that substring matched the test itself and recursed. The test now uses names that match no test, and a comment explains why.

### WR-02: The parity check passes a job that runs in an unpinned image

**Files modified:** `scripts/assert-image-parity.sh`, `tests/image_parity.rs`
**Commit:** 9246940
**Applied fix:**
- **Direct child only:** an `image:` now counts only when it is a direct child of `container:`. The indent of the first child line is tracked.
- **Unreadable jobs block:** a workflow file from which no jobs were read, such as one with a quoted `"jobs":` key, is now refused.
- **New fixtures:**
  - a nested `env: image:` holding the pin under a drifted `container.image`, which must fail
  - the reverse order, which must pass
  - `FROM ...:latest`
  - an empty workflows directory
  - a reusable-workflow `uses:` job
  - a quoted `"jobs":` file

**Negative control:**
- **Old script:** the nested-image and quoted-jobs tests fail.
- **Mutants for the three refusal branches that had no tests:**
  - With the exact-tag refusal deleted, `non_exact_dockerfile_tag_fails` fails.
  - With the no-workflows block deleted, `no_workflow_files_fails` fails. A first mutant of this block kept the error message, so the test survived it; it was redone with the whole block deleted.
  - With missing-image jobs no longer recorded, `reusable_workflow_job_fails_naming_the_job` fails.

### WR-03: `check-in-container.sh` gates the repo under the current directory

**Files modified:** `scripts/check-in-container.sh`, `tests/gate_controls.rs`
**Commit:** facacf8
**Applied fix:**
- **Repo resolution:** `REPO_ROOT` now comes from `git -C "$SCRIPT_DIR" rev-parse --show-toplevel`, so it is the repository that holds the script.
- **New test:** `gate_controls::container_gate_mounts_its_own_checkout_not_the_callers` uses two fixture repos created by the test, so git trusts them inside the container too. It runs the script from the other repo against a stub `docker` that records its arguments.

**Negative control:**
- **Old script:** the test fails. The mount was `.../container-gate/caller` where `.../container-gate/own` was expected.
- **New script:** the test passes.

### WR-04: The PR cut force-resets any branch it is given

**Files modified:** `scripts/workspace/cut-pr-branch.sh`
**Commit:** ce175cb
**Applied fix:**
- **Protected branches:** the cut refuses `main`, `master`, `develop`, the base branch, `workspace/*` and `personal/*` as the PR branch.
- **Existing PR branch:** it is refused unless `--force` is passed.
- **No upstream:** the PR branch is created with `--no-track`.

**Negative control (shell, scratch repo):**

| Case | Old script | New script |
|---|---|---|
| `cut-pr-branch.sh develop` | exit 0; the unpushed `LOCAL` commit disappeared from `develop` | exit 1, refused; `develop` still holds `LOCAL` |
| `workspace/x` | exit 0 | exit 1, refused |
| Upstream after the first cut | `origin/develop` | none |
| Re-cut without `--force` | silently discarded a fixup commit | exit 1; the fixup is kept |
| `--phase 9 --force` | not a recognised option | exit 0; the branch is re-cut |

### IN-01: The PR cut's "next steps" and usage text are stale

**Files modified:** `scripts/workspace/cut-pr-branch.sh`
**Commit:** 8be1b0d
**Applied fix:** the usage and the examples now name `scripts/workspace/cut-pr-branch.sh`, and the next step now reads `git checkout $PR_BRANCH && scripts/check.sh all`.

**Negative control (shell):** in the same scratch repo, the old script printed `... && ./tests/sandbox.sh` and the new one prints `... && scripts/check.sh all`.

## Notes for the orchestrator

- **Security threats not updated:** `01-SECURITY.md` was not edited. It still marks T-01-07 (CR-02) and T-01-15 (WR-02) as closed on their old mechanisms.
- **How commits were made:** with `git commit` directly, not through the gsd commit tool, so that the required trailers could be added. Scopes follow the instruction:
  - `fix(01): ...` for code that reaches develop
  - `fix(workspace): ...` for `scripts/workspace/`
- **Not addressed:** a positional test name is still a substring match, which the review mentions under WR-01. It was outside the requested fix.

---

_Fixed: 2026-10-23T20:39:33Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
