---
phase: 04-repo-model
plan: 07
subsystem: repo-model
tags: [rust, gix, git-refs, integration-branch, config, tdd]

requires:
  - phase: 04-repo-model
    provides: "04-01: the model contract, `find_ref`, `peel_commit`, `Error::IntegrationRefSymbolic`/`BaseRefMissing`/`NoIntegrationBranch` and their messages, `repo_with_remote` with distinct develop/next/main commits"
  - phase: 04-repo-model
    provides: "04-05: `single` with the D-18 refusals"
  - phase: 04-repo-model
    provides: "04-06: `remote()` for the base remote, `with_fork`, `assert_integration`"
provides:
  - "`model::integration`: `wsx.baseRef`, then `develop`, `next` and the base remote's `HEAD` target, read only from the base remote's tracking refs (local branches only with no base remote); D-09's error lists what was tried"
  - "`model::BASE_REF` (`wsx.baseRef`), `model::CHAIN`, private `model::branch_name`"
  - "`git::is_branch_name` agreeing with `git check-ref-format --branch` (refuses `HEAD` and a leading `-`)"
  - "`RefTarget` re-exported from `crate::git`"
  - "Test helper `model::tests::assert_resolved(scratch, work, integration, branch, full_ref, rejected)`"
affects: [04-08, 04-10, phase-5-check, cut]

actuals:
  tokens: 11897
  tasks: 2
  commits: 4
plan_head_before: 5d33405342cc84befc535b5d3a518f69968e4950
plan_head_after: 8aa5453d651174ae3484529da2db64c9da85b0ac

tech-stack:
  added: []
  patterns:
    - "Every rung of the integration chain goes through one closure, `found`: one exact `find_ref` read; a symbolic answer is a refusal; the tip is `peel_commit` of the id that read returned"
    - "Every test that resolves an integration branch asserts the tip against `git show-ref --verify` and unequal to each rejected candidate that exists (`assert_resolved`)"

key-files:
  created: []
  modified:
    - src/model/mod.rs
    - src/git/refs.rs
    - src/git/mod.rs

key-decisions:
  - "A symbolic ref on any rung (develop, next, the HEAD target, wsx.baseRef, local rungs included) is refused with IntegrationRefSymbolic, never followed and never skipped; a chain that stays inside the remote is refused too"
  - "wsx.baseRef is checked with branch_name before any rung and answered by found or BaseRefMissing, never by the chain (D-08)"
  - "is_branch_name refuses HEAD and a leading -, the two names where gix's full-name check disagrees with git check-ref-format --branch on the 13-name table (A5); no further disagreement was found"

patterns-established:
  - "assert_resolved(scratch, work, integration, branch, full_ref, rejected): label, full_ref, tip equal to git show-ref, tip unequal to each rejected ref (which must exist); assert_integration now delegates to it with branch develop"

requirements-completed: []

coverage:
  - id: D1
    description: "The chain resolves develop, then next, then the base remote's HEAD target, from tracking refs only; local develop and next only with no base remote (D-06, D-07)"
    requirement: CONF-03
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::{develop_beats_the_remote_default,next_follows_develop,remote_default_follows_next,local_develop_does_not_redirect_a_remote_base,local_branches_count_only_without_a_base_remote,local_next_follows_local_develop}"
        status: pass
    human_judgment: false
  - id: D2
    description: "With no rung, NoIntegrationBranch lists the refs tried and both fixes; main is never guessed; each broken <remote>/HEAD shape reaches that error; resolution makes no network call (D-09)"
    requirement: CONF-03
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::{no_integration_branch_lists_the_refs_tried,main_is_never_guessed,local_main_is_never_guessed,dangling_remote_head_is_not_a_default,object_remote_head_is_not_a_default,remote_head_outside_the_remote_is_not_a_default,resolution_needs_no_remote_to_be_reachable}"
        status: pass
    human_judgment: false
  - id: D3
    description: "A stale <remote>/HEAD is used as it is (D-10)"
    requirement: CONF-03
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::stale_remote_head_is_accepted"
        status: pass
    human_judgment: false
  - id: D4
    description: "No symbolic ref on any rung can move the base, and a rung that does not peel to a commit fails closed"
    requirement: CONF-03
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::{remote_head_chain_into_a_local_branch_is_refused,remote_head_chain_inside_the_remote_is_refused,remote_head_cycle_is_refused,symbolic_develop_is_refused,symbolic_next_is_refused,symbolic_local_develop_is_refused,symbolic_base_ref_is_refused,integration_ref_holding_a_blob_fails_closed}"
        status: pass
    human_judgment: false
  - id: D5
    description: "wsx.baseRef is a bare branch name on the base remote with no fallback; invalid, repeated and empty values are refused (D-08, D-18, A5)"
    requirement: CONF-03
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::{base_ref_overrides_the_chain,missing_base_ref_names_the_ref,base_ref_with_a_slash_is_a_literal_branch_name,base_ref_resolves_locally_without_a_base_remote,invalid_base_ref_is_an_error,repeated_base_ref_is_an_error,empty_base_ref_is_an_error}"
        status: pass
      - kind: unit
        ref: "src/git/refs.rs#git::refs::tests::branch_names_agree_with_git"
        status: pass
    human_judgment: false
  - id: D6
    description: "A full-name read is exact: a nested name, a loose file under .git/heads/ and a tag never satisfy it, agreeing with git show-ref --verify"
    requirement: CONF-03
    verification:
      - kind: unit
        ref: "src/git/refs.rs#git::refs::tests::{full_name_read_is_exact,tag_does_not_satisfy_a_branch_name,ref_commit_matches_show_ref}; src/model/mod.rs#model::tests::ref_under_another_path_does_not_satisfy_a_rung"
        status: pass
    human_judgment: false
  - id: D7
    description: "Prohibition: wsx never guesses the integration branch (no main or master default, no network lookup, no fallback once wsx.baseRef is set, no local branch redirecting a remote base)"
    verification:
      - kind: unit
        ref: "model::tests::main_is_never_guessed, local_main_is_never_guessed, missing_base_ref_names_the_ref, local_develop_does_not_redirect_a_remote_base, resolution_needs_no_remote_to_be_reachable"
        status: pass
    human_judgment: true
    rationale: "The plan marks this prohibition `verification: judgment`. The tests pin the code paths that exist today; that no later caller adds a guess is a review judgment."

duration: 8min
completed: 2026-11-08
status: complete
---

# Phase 4 Plan 07: Integration chain and `wsx.baseRef` Summary

**`integration` resolves `wsx.baseRef` (a bare branch name, no fallback), else `develop`, `next`, then the base remote's `HEAD` target, each read once as an exact full ref from the base remote's tracking refs. Any symbolic rung is refused, and when nothing matches the error lists every ref tried and both fixes.**

## Performance

- **Duration:** about 8 min
- **Started:** 2026-11-08T16:32:19Z
- **Completed:** 2026-11-08T16:40Z
- **Tasks:** 2 of 2
- **Files modified:** 3 (`src/model/mod.rs`, `src/git/refs.rs`, `src/git/mod.rs`)

## Accomplishments

- `integration` (`src/model/mod.rs:425`) reads each rung through the closure `found`. `found` makes one `find_ref` read of `<prefix><branch>`. `None` moves on to the next rung. `RefTarget::Symbolic` returns `Error::IntegrationRefSymbolic` with no fall-through. `RefTarget::Object(id)` gives a tip of `repo.peel_commit(&id)`, and a peel failure is `Error::Git` with no fall-through. The `<remote>/HEAD` rung is tried only when there is a base remote. It counts only as a symbolic ref whose target starts with the remote's prefix and whose stripped name `found` finds.
- `wsx.baseRef` is read through `single` and checked by the new `branch_name` before any rung. Its answer is `found(&value)` or `BaseRefMissing { value, tried }`, never the chain.
- `is_branch_name` (`src/git/refs.rs:54`) refuses `HEAD` and a leading `-`. Its doc comment records the git 2.56.0 table.
- `RefTarget` is re-exported from `crate::git` in the same commit as `found`, its first user outside `src/git`.

## Task Commits

1. **Task 1: The chain and its error.** RED `57824dc` (test), GREEN `00c4906` (feat)
2. **Task 2: `wsx.baseRef`, a bare branch name with no fallback.** RED `dd41981` (test), GREEN `8aa5453` (feat)

There are no REFACTOR commits.

## TDD Gate Compliance

Each task has a `test(04-07)` commit followed by a `feat(04-07)` commit that descends from it: `57824dc` then `00c4906`, and `dd41981` then `8aa5453`. No violations.

**Task 1 RED:** `gsd_run check tdd-red-evidence target/gsd/04-07-red-evidence.json --raw` printed `RED_EVIDENCE_OK` (`target_test_failed`, JUnit, 1 test, 1 failure, nextest exit 100) for `model::tests::next_follows_develop`.
- **Semantic assessment:** the target compiled and ran. It failed on the planned assertion: `configured()` requires `resolve` to return `Configured`, and 04-01's `integration` tries only `develop`, so it returned `NoIntegrationBranch` with `tried = [refs/remotes/origin/develop]`. This was not a build, fixture or load failure.
- **Other tests in the same RED tree:** 18 of the 25 Task 1 tests failed. Seven passed, because they describe behavior that already held: `develop_beats_the_remote_default`, `local_branches_count_only_without_a_base_remote`, `main_is_never_guessed`, `integration_ref_holding_a_blob_fails_closed`, and the three `git::refs` tests.

**Task 2 RED:** `target/gsd/04-07-red-evidence-2.json` gave `RED_EVIDENCE_OK` (`target_test_failed`, 1 test, 1 failure, exit 100) for `model::tests::missing_base_ref_names_the_ref`.
- **Semantic assessment:** the control passed first (`refs/remotes/origin/develop` exists). The test then failed on the planned assertion: `refusal()` requires an error, and `integration` did not read `wsx.baseRef`, so it resolved `develop`.
- **Other tests in the same RED tree:** all 9 Task 2 tests failed. `branch_names_agree_with_git` failed on `HEAD`, where gix accepts the name and git refuses it. `base_ref_with_a_slash_is_a_literal_branch_name` failed for the wrong reason: its own control, `refs/remotes/upstream/next`, was missing from the fixture. The test was fixed in the GREEN commit; see Deviations.

Both RED records were classified again at SUMMARY time, and each gave `RED_EVIDENCE_OK`.

**Negative controls (run after GREEN):** each mutant was applied to a scratch copy of the file, the named tests were run, and the file was restored. `git diff` was clean afterwards.
- **Mutant B (guess `main` after the remote default, and locally):** `main_is_never_guessed` and `local_main_is_never_guessed` both failed (exit 100).
- **Mutant C (a rung that does not peel falls through):** `integration_ref_holding_a_blob_fails_closed` failed.
- **Mutant D (`find_ref` without its name comparison):** `git::refs::tests::full_name_read_is_exact` and `model::tests::ref_under_another_path_does_not_satisfy_a_rung` both failed.
- **Mutant E (`CHAIN` order swapped):** `develop_beats_the_remote_default` failed. `local_branches_count_only_without_a_base_remote` passed, since it has no local `next`; see Limits.
- **Mutant F (a symbolic rung skipped instead of refused):** `symbolic_develop_is_refused`, `symbolic_next_is_refused` and `symbolic_base_ref_is_refused` all failed.
- **Without the `wsx.baseRef` read:** `base_ref_with_a_slash_is_a_literal_branch_name` failed (exit 100) on its planned assertion.

## Tip helper and the tests that assert a rejected tip

The helper is `assert_resolved` (`src/model/mod.rs:1328`). It checks the label and `full_ref`, checks that the tip equals `git show-ref --verify <full_ref>`, and checks that the tip differs from every ref in `rejected`, each of which must exist. 04-06's `assert_integration` now delegates to it with branch `develop`. These tests assert a rejected candidate's tip:

| Test | Rejected tips |
|------|---------------|
| `develop_beats_the_remote_default` | `origin/next`, `origin/main` |
| `next_follows_develop` | `origin/main` |
| `remote_default_follows_next` | `workspace/dev` |
| `local_develop_does_not_redirect_a_remote_base` | local `develop`, `origin/main` |
| `local_branches_count_only_without_a_base_remote` | local `main` |
| `local_next_follows_local_develop` | local `main` |
| `stale_remote_head_is_accepted` | `workspace/dev` |
| `resolution_needs_no_remote_to_be_reachable` | `workspace/dev` |
| `ref_under_another_path_does_not_satisfy_a_rung` | `workspace/dev`, `origin/main` |
| `base_ref_overrides_the_chain` | `origin/develop`, `origin/next` |
| `base_ref_resolves_locally_without_a_base_remote` | local `develop`, local `main` |

**D-10:** `stale_remote_head_is_accepted` passes. The source's default was renamed to `trunk` and nothing was fetched, so `git symbolic-ref` in the source prints `refs/heads/trunk` while `origin/HEAD` still names `origin/main`. The answer is `main` from `refs/remotes/origin/main`, with the tip equal to `git show-ref --verify`.

## Source-shape check

The `<verify>` source check exits 0 after each GREEN commit. As a negative control, it was run on the pre-plan `src/model/mod.rs` (`git show 5d33405:...`) and exited 1 with `integration calls ref_commit, a second read of a ref`. This check is a lint on the shape of the code. It is not evidence about the T-04-38 race: a read moved into a helper would pass it. T-04-38 stays accepted.

## Gate runs

| Command | Exit |
|---------|------|
| `scripts/check.sh test -E 'test(=model::tests::next_follows_develop)'` (Task 1 RED) | 100 |
| Task 1 `<verify>` loop, 22 model and 3 refs tests, one `-E 'test(=…)'` per command | 0 |
| Task 1 source check | 0 |
| `scripts/check.sh all` after Task 1 GREEN (522 run, 522 passed, 61 skipped; last line `==> check.sh: all OK`) | 0 |
| `scripts/check.sh test -E 'test(=model::tests::missing_base_ref_names_the_ref)'` (Task 2 RED) | 100 |
| Task 2 `<verify>` loop, 8 model tests and `branch_names_agree_with_git` | 0 |
| Task 2 source check | 0 |
| `scripts/check.sh all` after Task 2 GREEN (531 run, 531 passed, 61 skipped; last line `==> check.sh: all OK`) | 0 |

`scripts/check.sh deps` was not run, because no dependency was added.

## Files Created/Modified

- `src/model/mod.rs`:
  - New items: `BASE_REF`, `CHAIN`, `branch_name`, and the finished `integration`.
  - 30 tests: 22 for Task 1 and 8 for Task 2.
  - New test helpers: `assert_resolved`, `chain_repo`, `run`, `exit_code`, `delete_refs`, `integration_of`, `refusal`, `no_integration`, `assert_symbolic`, and the `ORIGIN_*`/`WORKSPACE` test constants.
- `src/git/refs.rs`: the `is_branch_name` checks and their doc comment, plus four tests.
- `src/git/mod.rs`: `RefTarget` added to the `refs` re-export.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The control in `base_ref_with_a_slash_is_a_literal_branch_name` named a ref the fork fixture lacks**
- **Found during:** Task 2 GREEN.
- **Issue:** `with_fork` clones `source`, so the fork has only `main` and `develop`, and `refs/remotes/upstream/next` does not exist. The RED commit's control, `tip(upstream/next)`, therefore panicked before the assertion.
- **Fix:** the test now creates `next` in the fork from `origin/next` and fetches `upstream`. That makes the control real: read as a remote plus a branch, `upstream/next` would have resolved. A mutant without the `wsx.baseRef` read shows that the test now fails on its assertion.
- **Files modified:** `src/model/mod.rs`
- **Commit:** `8aa5453`

### Strengthened beyond the plan's wording (no behavior change to the code under test)

- **`base_ref_resolves_locally_without_a_base_remote`:** creates `trunk` from `origin/next` and a local `develop` before removing the remote. The plan's `git branch trunk` would put `trunk` on local `main`'s commit, so a tip check could not tell them apart. Now the override is visible against a `develop` that the chain would take.
- **`resolution_needs_no_remote_to_be_reachable`:** deletes `develop` and `next` so that the remote-default rung answers. It resolves before and after deleting `<root>/source`, then asserts the two answers are equal and that the tip equals `git show-ref`.
- **`remote_head_outside_the_remote_is_not_a_default`:** asserts the rendered error lacks the substring `exist`. That is stricter than the plan's `exists`, and it also rules out "does not exist".
- **Rejected lists:** `remote_default_follows_next`, `stale_remote_head_is_accepted`, `resolution_needs_no_remote_to_be_reachable` and `ref_under_another_path_does_not_satisfy_a_rung` also reject the `workspace/dev` tip. In those tests no other rung is left to reject.

## Limits (what these tests do not establish)

- **No network call:** shown only by resolving with the remote's directory deleted, plus gix having no network feature (Phase 3's dependency check). It is not a syscall trace.
- **Strictness:** every symbolic ref on a rung is refused. That includes a chain that stays inside the remote and a local `develop` aliased to `main`, which is deliberately stricter than the escape requires; `wsx.baseRef` is the way out. A symbolic workspace branch is still followed, as git follows it; this plan does not test that shape.
- **T-04-38 (accepted):** that a rung is read once has no runtime test. The source check lints the code shape only.
- **Local rung order:** no test has both a local `develop` and a local `next`, so the local order is pinned only through the shared `CHAIN` constant, whose order `develop_beats_the_remote_default` pins on the remote side (Mutant E).
- **`tag_does_not_satisfy_a_branch_name`:** no mutant was run against it. It passes on the existing full-name read and is a differential against `git show-ref --verify`, not a test that a specific check bites.
- **Scope:** each rung is exercised once, on one fixture, on git 2.56.0. The tip assertions tell rungs apart only where the fixtures give them distinct commits, which they do for every candidate rejected here.
- **Requirement:** CONF-03 is also listed by 04-10, which is not complete, so REQUIREMENTS.md is left unchanged for it.

## Known Stubs

None. A scan of the added lines for TODO, FIXME, placeholder and unimplemented found nothing.

## Self-Check: PASSED

- Files FOUND: `src/model/mod.rs`, `src/git/refs.rs`, `src/git/mod.rs`, `target/gsd/04-07-red-evidence.json`, `target/gsd/04-07-red-evidence-2.json`, `target/gsd/04-07-all.log`.
- Commits FOUND in HEAD's history: `57824dc`, `00c4906`, `dd41981`, `8aa5453`.
- `git rev-list --count 5d33405..8aa5453` = 4.
