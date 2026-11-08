---
phase: 04-repo-model
plan: 06
subsystem: repo-model
tags: [rust, gix, gix-config, git-remote, config, tdd]

requires:
  - phase: 04-repo-model
    provides: "04-05: local-only `local_config_values`, `single` with D-18 refusals, `state_child`, `DISCOVERING_SUITES` covering `git::config::tests::` and `model::tests::`"
  - phase: 04-repo-model
    provides: "04-01: the model contract, `Error::NoSuchRemote`, `repo_with_remote` with distinct develop/next/main commits"
provides:
  - "`Repo::remote_configured` answering as `git remote add` does: a `remote` section with at least one key from `Source::Local` or `Source::Worktree`, include level not filtered"
  - "`model::BASE_REMOTE` (`wsx.baseRemote`) and `model::PUSH_REMOTE` (`wsx.pushRemote`), read independently by private `remote(repo, key)` in `build`"
  - "Test helper `model::tests::with_fork`, a fork fixture whose `upstream/develop` holds its own commit"
  - "`state_child` markers `base_remote=` and `push_remote=`; child half `git::config::tests::remote_child`"
affects: [04-07, 04-08, 04-10, phase-5-check, cut, push]

actuals:
  tokens: 9057
  tasks: 2
  commits: 4
plan_head_before: 9ca771159b033c5b63751aacb5dadafe736cd30f
plan_head_after: c5b3536008488376f0a1218884e3e6ff923be39e

tech-stack:
  added: []
  patterns:
    - "A remote exists when this repository defines it: git remote add's exit 3 is the differential, not git remote's listing or gix remote_names()"
    - "Every resolved-base test asserts the tip against git show-ref --verify and unequal to each rejected candidate that exists (assert_integration)"

key-files:
  created: []
  modified:
    - src/git/config.rs
    - src/model/mod.rs

key-decisions:
  - "remote_configured does not filter include level, unlike local_config_values: git remote add refuses a name defined in an include.path file (exit 3), so an included section counts"
  - "remote(repo, key) resolves base then push after the workspace-branch check; with both keys bad, the base key's error is the one reported"

patterns-established:
  - "assert_integration(scratch, work, integration, full_ref, rejected): label, full_ref, tip equal to git show-ref, tip unequal to each rejected ref (which must exist)"

requirements-completed: [CONF-04]

coverage:
  - id: D1
    description: "A remote exists only when this repository defines it (local, included or config.worktree section with a key); a global section or a keyless local section does not count"
    requirement: CONF-04
    verification:
      - kind: unit
        ref: "src/git/config.rs#git::config::tests::keyless_local_remote_section_is_not_a_remote"
        status: pass
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::global_remote_section_is_not_a_remote"
        status: pass
      - kind: unit
        ref: "src/git/config.rs#git::config::tests::{local_remote_is_a_remote,local_remote_without_a_url_is_a_remote,included_remote_is_a_remote,worktree_config_remote_is_a_remote,undefined_remote_is_not_a_remote}"
        status: pass
    human_judgment: false
  - id: D2
    description: "Base and push remotes resolve from wsx.baseRemote and wsx.pushRemote independently, default to origin only when unset and defined, refuse a named remote that is not defined, and never read remote.pushDefault or auto-detect upstream"
    requirement: CONF-04
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::fork_layout_resolves_each_remote and the twelve others in Task 2's loop"
        status: pass
      - kind: integration
        ref: "src/model/mod.rs#model::tests::global_origin_section_does_not_hide_local_branches"
        status: pass
    human_judgment: false
  - id: D3
    description: "Prohibition: wsx never guesses where a PR branch goes or where the base lives (no remote.pushDefault, no upstream detection, no fallback to another remote)"
    verification:
      - kind: unit
        ref: "model::tests::push_default_is_not_read, upstream_is_not_auto_detected, upstream_is_preferred_nowhere, missing_*_remote_names_the_key_and_the_remote"
        status: pass
    human_judgment: true
    rationale: "The plan marks this prohibition `verification: judgment`. The tests pin the code paths that exist today; whether no later caller reintroduces a guess is a review judgment, not something these tests can prove."

duration: 7min
completed: 2026-11-08
status: complete
---

# Phase 4 Plan 06: Base and push remotes Summary

**`wsx.baseRemote` and `wsx.pushRemote` resolve separately to remotes this repository defines (`git remote add`'s test, read through gix `sections_by_name` on `Source::Local`/`Source::Worktree`), defaulting to `origin` only when it is defined, with no `remote.pushDefault` and no `upstream` detection.**

## Performance

- **Duration:** about 7 min
- **Started:** 2026-11-08T16:09:49Z
- **Completed:** 2026-11-08T16:17Z
- **Tasks:** 2 of 2
- **Files modified:** 2 (`src/git/config.rs`, `src/model/mod.rs`)

## Accomplishments

- `Repo::remote_configured` (`src/git/config.rs:59`) now counts a remote only when a `[remote "<name>"]` section with at least one key comes from the repository's own config or `config.worktree`. A global-only section and a local keyless section no longer count; gix `remote_names()` counted both.
- `model::remote(repo, key)` (`src/model/mod.rs:351`) is called once for `BASE_REMOTE` and once for `PUSH_REMOTE` in `build`. A key naming an undefined remote is `Error::NoSuchRemote { key, remote }`. A repeated key is `KeyRepeated` and an empty one is `KeyEmpty`, both through 04-05's `single`. An unset key resolves to `origin` only when the repository defines it.
- A fork fixture, `with_fork` (`src/model/mod.rs:1198`). Its `upstream/develop` holds a commit unique to the fork, and it asserts four distinct candidate tips before returning. Every test that resolves a base checks the tip against `git show-ref --verify` and checks it differs from each rejected candidate's tip.

## Task Commits

1. **Task 1: A remote exists when this repository defines it.** RED `c51e050` (test), GREEN `f5e3ee7` (feat)
2. **Task 2: Base and push remotes resolve separately, with no guessing.** RED `f7c1dda` (test), GREEN `c5b3536` (feat)

No REFACTOR commits: nothing needed cleanup after GREEN.

## TDD Gate Compliance

RED and GREEN commits for both tasks are present and in order: `test(04-06)` `c51e050` comes before `feat(04-06)` `f5e3ee7`, and `f7c1dda` comes before `c5b3536`. Each GREEN commit descends from its RED commit. No violations.

**Task 1 RED:** `gsd_run check tdd-red-evidence target/gsd/04-06-red-evidence.json --raw` printed `RED_EVIDENCE_OK` (`target_test_failed`, JUnit report, 1 test, 1 failure, nextest exit 100) for `git::config::tests::keyless_local_remote_section_is_not_a_remote`.
- **Semantic assessment:** the test compiled and ran. Its control passed first: gix `remote_names()` lists the keyless `[remote "empty"]` section. The test then failed on the planned assertion, `!remote_configured(b"empty")`, at `src/git/config.rs:565`, because the 04-01 body delegated to `remote_names()`.
- **Other tests in the same RED tree:** `global_remote_section_is_not_a_remote` also failed as expected (the child printed `configured=true` where `false` was expected). The other five Task 1 tests passed.

**Task 2 RED:** `target/gsd/04-06-red-evidence-2.json` gave `RED_EVIDENCE_OK` (`target_test_failed`, 1 test, 1 failure, exit 100) for `model::tests::fork_layout_resolves_each_remote`.
- **Semantic assessment:** `with_fork` returned, so its distinct-tips control passed. The test then failed on its first planned assertion: `base_remote` was `Some(b"origin")` where `Some(b"upstream")` was expected (`src/model/mod.rs:1282`), because `build` read no key.
- **Other tests in the same RED tree:** six more failed on their own assertions: `push_remote_can_differ_from_a_default_base`, `missing_base_remote_names_the_key_and_the_remote`, `missing_push_remote_names_the_key_and_the_remote`, `repeated_base_remote_is_an_error`, `empty_push_remote_is_an_error` and `remote_name_with_a_slash_resolves`. Six passed: the default, no-origin, upstream and pushDefault tests, which describe behaviour that already held, and `global_origin_section_does_not_hide_local_branches`, which Task 1's GREEN had already fixed.

Both RED records were re-classified at SUMMARY time and each gave `RED_EVIDENCE_OK`.

**Negative controls for the tests that passed in RED** (run after GREEN, each file restored from HEAD with `git checkout -- <file>`, tree clean afterwards):
- Mutant A, `remote_configured` restored to the `remote_names()` body. `model::tests::global_origin_section_does_not_hide_local_branches` and `git::config::tests::global_remote_section_is_not_a_remote` both failed (exit 100).
- Mutant B, an unset base key that prefers a defined `upstream`. `upstream_is_preferred_nowhere` and `upstream_is_not_auto_detected` both failed (exit 100).
- Mutant C, an unset push key that reads `remote.pushDefault`. `push_default_is_not_read` failed (exit 100).

## Files Created/Modified

- `src/git/config.rs`:
  - The new `remote_configured` body and its doc comment, which records the measurements.
  - Seven tests and the child half `remote_child`.
  - Helpers `assert_git_exits`, `git_remotes`, `configured`, `gix_lists_remote` and `child_configured`.
- `src/model/mod.rs`:
  - `BASE_REMOTE`, `PUSH_REMOTE` and `remote()`; `build` now reads both keys.
  - Thirteen tests.
  - Helpers `with_fork`, `assert_integration`, `refused_remote`, `state_child_output` and `remote_marker`.
  - `state_child` now prints `base_remote=` and `push_remote=`.

## Decisions Made

- **`remote_configured` does not filter the include level.** `local_config_values` does, for D-01. Here `git remote add` refuses a name that an `include.path` file defines, so an included section counts. The doc comment records this difference.
- **The base key is read before the push key**, both after the workspace-branch check. When both keys are bad, the error names `wsx.baseRemote`.

## Deviations from Plan

None in what the code does. Additions to the test shapes the plan named:
- `global_remote_section_is_not_a_remote` re-runs the child after `git remote add x /y` and expects `configured=true`. This shows the child can answer `true` for the same name.
- `keyless_local_remote_section_is_not_a_remote` asserts that gix `remote_names()` lists the section. This shows the test can tell the old answer from the new one.
- `included_remote_is_a_remote` and `worktree_config_remote_is_a_remote` assert that `git config --local --get` does not see the url. This shows the section really comes from the include file and from `config.worktree`.
- `upstream_is_not_auto_detected` asserts that `tried` is non-empty, so "every entry starts with `refs/remotes/origin/`" cannot pass on an empty list. It deletes `refs/remotes/origin/HEAD` with `--no-deref`, so `origin/main` survives, and checks that it did.
- The tests that resolve a base, beyond the four the plan named, also use `assert_integration` and check the tip.

**Total deviations:** 0 auto-fixed. **Impact on plan:** none.

## Issues Encountered

None.

## Limits of the Verification

- `remote_configured` was measured against one git version, 2.56.0, with gix 0.90.0 and gix-config 0.62.2. The differential is `git remote add`'s exit-3 refusal.
- A remote's URL is never checked. Resolution makes no network call (D-09), so a defined remote with a dead URL still counts.
- The legacy `.git/remotes/<name>` file is not counted and not tested. Planning measured that neither gix nor `git remote` lists it.
- A legacy `[remote.x]` header and subsection-case variants were not tested.
- The mutants above show that these tests catch three specific regressions. They are not a full mutation pass; plan 04-10 runs that.
- CONF-04 edge coverage ("review manually" in the planning probe) stays unresolved, as the plan requires. Slashed names, includes and `config.worktree` are now pinned by tests.

## Gate Runs

- After Task 1 GREEN: `scripts/check.sh all > target/gsd/04-06-all.log 2>&1` exited 0; the last line was `==> check.sh: all OK`; 424 tests ran, 424 passed, 61 skipped.
- After Task 2 GREEN: the same command exited 0 with the same last line; 437 tests ran, 437 passed, 61 skipped.
- Each test in both `<verify>` loops ran one per command, with `--no-tests=fail -E 'test(=...)'`. All 20 passed after GREEN.

## User Setup Required

None.

## Next Phase Readiness

- 04-07 can add the `next` and `<remote>/HEAD` rungs inside `integration(repo, base)`. `upstream_is_not_auto_detected` already requires every `tried` entry to start with `refs/remotes/origin/`, so it constrains the new rungs too.
- CONF-04 is not marked complete in REQUIREMENTS.md: `requirements.ready-ids` reports it blocked because 04-10, which also lists it, has no SUMMARY yet.

## Self-Check: PASSED

- Files FOUND: `src/git/config.rs`, `src/model/mod.rs`, `target/gsd/04-06-red-evidence.json`, `target/gsd/04-06-red-evidence-2.json`, `target/gsd/04-06-all.log`.
- Commits FOUND on HEAD: `c51e050`, `f5e3ee7`, `f7c1dda`, `c5b3536`. `git rev-list --count 9ca7711..HEAD` gives 4. `evaluation-scope --plan 04-06` resolved these commits from their subjects.
- Acceptance: `rg -q 'sections_by_name' src/git/config.rs` exited 0. `rg -q 'const BASE_REMOTE' src/model/mod.rs && rg -q 'const PUSH_REMOTE' src/model/mod.rs` exited 0.
- Stub scan over the plan's diff (TODO/FIXME/placeholder): none.

---
*Phase: 04-repo-model*
*Completed: 2026-11-08*
