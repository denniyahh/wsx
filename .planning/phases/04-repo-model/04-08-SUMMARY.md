---
phase: 04-repo-model
plan: 08
subsystem: repo-model
tags: [rust, gix, git-refs, workspace-branch, config, tdd]

requires:
  - phase: 04-repo-model
    provides: "04-01: the model contract, `RepoModel`, `Error::WorkspaceCandidates`/`ManifestUnreadable`/`NotConfigured` and their messages, `repo_with_remote`"
  - phase: 04-repo-model
    provides: "04-05: `single` with the D-18 refusals, `resolved`, `state_child`"
  - phase: 04-repo-model
    provides: "04-07: `find_ref`, `ref_commit`, `is_branch_name`, the integration chain, `refusal`, `no_integration`, `delete_refs`, `exit_code`"
provides:
  - "`git::Repo::refs_directly_under(prefix)`: full names exactly one level under a `/`-terminated prefix, loose and packed, byte order; a ref that fails to read is `Error::Gix` naming the prefix"
  - "`model::candidates` (private): local `workspace/*` and `personal/*` short names in byte order, less the `recorded` set"
  - "`model::resolve` completed: with `wsx.sourceBranch` unset, the only candidate is inferred and every failure stays in `NotConfigured { inferred }`"
  - "`model::build` maps a failed tip-commit or root-tree read to `Error::ManifestUnreadable` (its first production use)"
  - "`RepoModel::require`: the configured or inferred model; a manifest failure wrapped as `Error::NotConfigured`; every other error unchanged"
affects: [04-09, 04-10, 04-11, phase-5-check, phase-6-guard, phase-8-new]

actuals:
  tokens: 14204
  tasks: 2
  commits: 5
plan_head_before: 3b10eb46956725b390e60daf650835f3cdbb8dac
plan_head_after: 1b6641ed24407d8d9e7fb51406df079009ec27b7

tech-stack:
  added: []
  patterns:
    - "A listing that feeds a choice fails whole on one unreadable entry, never returns a shorter list"
    - "Candidate tests assert the inferred tip against `git show-ref --verify`, so a test reading another repository fails"

key-files:
  created: []
  modified:
    - src/git/refs.rs
    - src/model/mod.rs

key-decisions:
  - "A3 adopted as planned: a candidate is one level under refs/heads/workspace/ or refs/heads/personal/, as git for-each-ref's glob reads it; workspace/a/b alone gives zero candidates"
  - "build's ManifestUnreadable mapping applies on the configured path too: a configured repo whose tip commit or root tree cannot be read still fails closed, now as ManifestUnreadable instead of Error::Git"
  - "A corrupt candidate ref surfaces as NotConfigured { inferred: Err(Git(Gix)) } and require passes it through unwrapped, since it is not a manifest read"

patterns-established:
  - "Model test helpers: inferred / inferred_with / not_configured_in / inferred_model / candidates_of / listed_candidates / required / wrapped_reason / commit_manifest / delete_manifest_object / two_personal_candidates / for_each_ref"
  - "refs test helpers: listing_repo, listed (union over CANDIDATE_PREFIXES), git_listed (for-each-ref with %(refname)%00, raw bytes)"

requirements-completed: []

coverage:
  - id: D1
    description: "refs_directly_under equals git for-each-ref's one-level globs over packed and loose refs, a non-UTF-8 name and a linked worktree, changes with a new branch, and fails on a corrupt ref naming it"
    requirement: CONF-05
    verification:
      - kind: unit
        ref: "src/git/refs.rs#git::refs::tests::{refs_directly_under_matches_for_each_ref,listing_changes_with_a_new_branch,listing_survives_packing,listing_is_sorted,non_utf8_branch_is_listed_by_its_bytes,corrupt_ref_under_the_prefix_is_an_error,linked_worktree_lists_the_same_branches}"
        status: pass
    human_judgment: false
  - id: D2
    description: "Success Criterion 3: the key names the workspace branch with another candidate present; the only candidate is inferred; several and zero candidates are listed errors; byte order across both prefixes; nested, remote-only and name-extension branches are not candidates; the checked-out branch gets no preference"
    requirement: CONF-05
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::{source_branch_names_the_workspace,source_branch_may_be_outside_the_candidate_prefixes,only_candidate_is_inferred,personal_branch_is_a_candidate,several_candidates_are_listed,candidates_sort_across_both_prefixes,zero_candidates_are_an_error,nested_branch_is_not_a_candidate,remote_only_branch_is_not_a_candidate,name_extension_is_not_a_candidate,checked_out_branch_gets_no_preference}"
        status: pass
    human_judgment: false
  - id: D3
    description: "D-17: the recorded set excludes a branch from the candidates, proven at the resolve input only"
    requirement: CONF-05
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::recorded_branches_are_not_candidates"
        status: pass
    human_judgment: false
  - id: D4
    description: "D-15, D-16, D-21: not configured is a value; every failure with the key unset stays in inferred; the key set fails closed"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::{no_source_branch_is_not_configured,several_candidates_do_not_fail_resolve,corrupt_candidate_ref_stays_in_inferred,invalid_inferred_manifest_is_not_configured,missing_inferred_manifest_is_not_configured,unreadable_inferred_manifest_is_not_configured,invalid_manifest_with_the_key_set_fails_closed,missing_source_branch_fails_closed,full_ref_as_source_branch_is_not_found,invalid_source_branch_is_an_error}"
        status: pass
    human_judgment: false
  - id: D5
    description: "RepoModel::require returns the configured or inferred model, wraps ManifestMissing/ManifestUnreadable/ManifestInvalid as NotConfigured saying run wsx init, and passes candidates and other failures through"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::{require_returns_the_configured_model,require_returns_the_inferred_model,require_wraps_a_manifest_failure_as_not_configured,require_wraps_a_missing_manifest_as_not_configured,require_wraps_an_unreadable_manifest_as_not_configured,require_lists_the_candidates,require_passes_other_failures_through}"
        status: pass
    human_judgment: false

duration: 14min
completed: 2026-11-08
status: complete
---

# Phase 4 Plan 08: Workspace Fallback and Configured State Summary

**`wsx.sourceBranch` names the workspace branch; without it the only local `workspace/*` or `personal/*` branch (one level, from gix's prefixed ref listing) is inferred and the repo stays `NotConfigured`; zero or several candidates are a listed error inside `inferred`, and `RepoModel::require` turns a manifest failure into "not configured, run `wsx init`".**

## Performance

- **Duration:** 14 min
- **Started:** 2026-11-08T16:43:30Z
- **Completed:** 2026-11-08T16:57:51Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `Repo::refs_directly_under` lists refs one level under a `/`-terminated prefix and equals `git for-each-ref 'refs/heads/workspace/*' 'refs/heads/personal/*'` on the measured shapes. A corrupt loose ref fails the listing, and the error names `refs/heads/workspace/bad` (git warns and skips it).
- `resolve` infers the only candidate with the key unset. Zero, several, a corrupt candidate ref, and a missing, invalid or unreadable inferred manifest all give `Ok(NotConfigured { inferred: Err(_) })`, so the phase 6 guard can read the state without classifying errors. With the key set, `resolve` still fails closed.
- `RepoModel::require` wraps `ManifestMissing`, `ManifestNotABlob`, `ManifestUnreadable` and `ManifestInvalid` as `Error::NotConfigured`. The rendering keeps the inner error (for an unreadable manifest, gix's message with the missing object's id) and ends with `Run wsx init.`. Every other error passes through unchanged.

## Task Commits

1. **Task 1: Listing branches exactly one level under a prefix.** RED `488a884` (test), GREEN `808172a` (feat), then `a53810f` (test: tighten the corrupt-ref assertion to the measured message; see Deviations)
2. **Task 2: Fallback, the configured state, and `require`.** RED `fb06479` (test), GREEN `1b6641e` (feat)

There are no REFACTOR commits.

## TDD Gate Compliance

Each task has a `test(04-08)` commit followed by a `feat(04-08)` commit that descends from it: `488a884` then `808172a`, and `fb06479` then `1b6641e`. No violations.

**Task 1 RED:** `gsd_run check tdd-red-evidence target/gsd/04-08-red-evidence.json --raw` printed `RED_EVIDENCE_OK` (`target_test_failed`, JUnit, 1 test, 1 failure, nextest exit 100) for `git::refs::tests::refs_directly_under_matches_for_each_ref`.
- **Semantic assessment:** the target compiled and ran. The fixture built, and its first assertion passed: git lists exactly `personal/workspace`, `personal/workspace-backup` and `workspace/a`. It then failed on the planned assertion: the stub returned `{}` against git's three names. This was not a build, fixture or load failure.
- **Other tests in the same RED tree:** all 7 Task 1 tests failed.

**Task 2 RED:** `target/gsd/04-08-red-evidence-2.json` gave `RED_EVIDENCE_OK` (`target_test_failed`, 1 test, 1 failure, exit 100) for `model::tests::only_candidate_is_inferred`.
- **Semantic assessment:** `resolve` returned `NotConfigured`. The test then failed on the planned assertion, that `inferred` be `Ok(model)`, because 04-01's `resolve` always answered `WorkspaceCandidates { candidates: [] }`.
- **Other tests in the same RED tree:** 20 of the 29 Task 2 tests failed. Nine passed:
  - six because the behaviour already held: `invalid_source_branch_is_an_error`, `missing_source_branch_fails_closed`, `full_ref_as_source_branch_is_not_found`, `source_branch_may_be_outside_the_candidate_prefixes`, `invalid_manifest_with_the_key_set_fails_closed` and `require_returns_the_configured_model`;
  - three only because the stub always answered "zero candidates": `nested_branch_is_not_a_candidate`, `remote_only_branch_is_not_a_candidate` and `name_extension_is_not_a_candidate`. The mutants below show that each of the three now catches its own fault.

Both RED records were classified again at SUMMARY time, and each gave `RED_EVIDENCE_OK`.

**Negative controls (run after GREEN):** each mutant was applied to the committed file, the named tests were run through `scripts/check.sh test -E 'test(=...)'`, and the file was restored with `git checkout -- <file>`. `git status --short` was clean afterwards.

| Mutant | Test(s) | Result |
|---|---|---|
| No one-level filter in `refs_directly_under` | `refs_directly_under_matches_for_each_ref`, `linked_worktree_lists_the_same_branches`, `nested_branch_is_not_a_candidate` | each failed, exit 100 |
| A failed ref read is skipped instead of returned | `corrupt_ref_under_the_prefix_is_an_error` | failed |
| No sort in `refs_directly_under` | `listing_is_sorted` | **survived**: gix already iterates in name order, so this test pins the contract and cannot catch the sort being removed |
| `candidates` ignores `recorded` | `recorded_branches_are_not_candidates` | failed |
| No sort in `candidates` | `candidates_sort_across_both_prefixes` | failed |
| Prefixes without the trailing slash | `name_extension_is_not_a_candidate` | failed |
| `refs/remotes/origin/workspace/` also counted | `remote_only_branch_is_not_a_candidate` | failed |
| Several candidates prefer the checked-out branch | `checked_out_branch_gets_no_preference` | failed |
| `WorkspaceCandidates` returned as `Err` from `resolve` | `several_candidates_do_not_fail_resolve` | failed |
| A configured repo returned as `NotConfigured` | `no_source_branch_is_not_configured`, `invalid_manifest_with_the_key_set_fails_closed` | each failed |
| `require` wraps every error | `require_lists_the_candidates`, `require_passes_other_failures_through` | each failed |
| `require` drops the `ManifestMissing` arm | `require_wraps_a_missing_manifest_as_not_configured` | failed |
| `require` drops the `ManifestUnreadable` arm | `require_wraps_an_unreadable_manifest_as_not_configured` | failed |
| `build` without the `ManifestUnreadable` mapping on `root_entry` | `unreadable_inferred_manifest_is_not_configured` | failed |

## Files Created/Modified

- `src/git/refs.rs`: `Repo::refs_directly_under` and its 7 tests, with the helpers `listing_repo`, `listed` and `git_listed`.
- `src/model/mod.rs`: `CANDIDATE_PREFIXES`, `BRANCH_PREFIX`, `candidates`, the `NotConfigured` branch of `resolve`, `build`'s `ManifestUnreadable` mapping, `RepoModel::require`, and 29 tests (the 28 the plan names and `corrupt_candidate_ref_stays_in_inferred`).

## Decisions Made

- `for-each-ref` has no `-z` on git 2.56.0 (checked with `git for-each-ref -h`). The differential uses `--format=%(refname)%00` and requires each line to end in NUL. A ref name holds neither NUL nor LF, so this is a raw-byte split.
- `build`'s `ManifestUnreadable` mapping is not limited to inferred branches. On the configured path a failed tip or root-tree read still returns `Err` from `resolve`, but now as `ManifestUnreadable` (rendered `cannot be read: <gix error>`) instead of `Error::Git`. The plan specifies the mapping in `build`, which both paths share.
- A single dangling symbolic ref under a prefix is listed as a candidate, and `ref_commit` on it fails. It therefore becomes `inferred: Err(ManifestUnreadable)`, which `require` wraps as "not configured, run `wsx init`". This follows from the plan's mapping. No test pins it.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] Test for threat T-04-20 at the `resolve` level**
- **Found during:** Task 2
- **Issue:** the threat register mitigates T-04-20, "a corrupt ref shortening the candidate list to one", with "`refs_directly_under` returns `Err` on a corrupt ref, which stays in `inferred`". Task 1 tests the `Err`. No named test showed that it stays in `inferred` and does not leave `workspace/dev` as the only candidate.
- **Fix:** added `model::tests::corrupt_candidate_ref_stays_in_inferred`. A garbage loose ref beside `workspace/dev` gives `NotConfigured { inferred: Err(Git(Gix { what: "list the refs directly under refs/heads/workspace/" })) }`. Its control shows that `git for-each-ref` skips that ref and lists only `workspace/dev`.
- **Files modified:** `src/model/mod.rs`
- **Committed in:** `fb06479` (RED, failed there) and `1b6641e` (GREEN)

**2. [Rule 1 - Test precision] The corrupt-ref listing test now requires the ref's name**
- **Found during:** Task 1, after GREEN
- **Issue:** the plan's behaviour says the listing fails "naming the failure". As first written, `corrupt_ref_under_the_prefix_is_an_error` accepted any non-empty gix message after the prefix.
- **Fix:** a probe run showed that gix's message is `The reference at "refs/heads/workspace/bad" could not be decoded`. The test now requires `refs/heads/workspace/bad` in the message. The "skip the failed read" mutant still fails the test.
- **Files modified:** `src/git/refs.rs`
- **Committed in:** `a53810f`

---

**Total deviations:** 2 auto-fixed (1 missing test for a threat mitigation, 1 test precision fix)
**Impact on plan:** both add tests only. Production code is as the plan specifies.

## Limits (what this plan does not establish)

- **CONF-05's exclusion is proven at the input only.** "Ignoring branches recorded as made by `wsx new`" is shown at `resolve`'s `recorded` parameter (`recorded_branches_are_not_candidates`), not end to end. Production passes `BTreeSet::new()` until phase 8 supplies the real set (D-17). No `refs/wsx/` format is defined: `rg -qF 'refs/wsx'` exits 1 on both `src/model/mod.rs` and `src/git/refs.rs`.
- The guard's exit 0 on `NotConfigured` is phase 6's behaviour. This plan only delivers the value it reads.
- `ManifestUnreadable` covers the tip commit and the root-tree read. A failure of `find_ref`, of the candidate listing or of the integration chain is not a manifest read, so `require` passes it through unchanged.
- `require`'s `ManifestNotABlob` arm has no test here, because `resolve` cannot produce that error until 04-09 adds the mode check. 04-09's `require_wraps_a_non_blob_manifest_as_not_configured` pins it.
- `listing_is_sorted` cannot catch the removal of `refs_directly_under`'s own sort (see the mutant table). `candidates_sort_across_both_prefixes` does catch the removal of `candidates`' sort.
- The differential compares against git on the shapes in the fixtures only: one-level, nested, name-extension, packed, loose, non-UTF-8, a corrupt loose ref and a linked worktree. A dangling symref under the prefix is not tested here. RESEARCH measured it as listed by gix and omitted by git.
- Everything was measured on git 2.56.0 and gix 0.90.0 only.

## Known gap carried from 04-01

The gap still stands. `model::tests::no_source_branch_and_no_candidate_is_not_configured` was not changed. With `GIT_DIR` aimed at a decoy repository that also has no candidate, it still gets the same answer. The new tests close the gap for the cases they cover, not for that test:
- `only_candidate_is_inferred`, `personal_branch_is_a_candidate` and `require_returns_the_inferred_model` each assert the inferred tip against `git show-ref --verify` in the fixture.
- `zero_candidates_are_an_error` first infers `workspace/dev` from the same repository, then deletes it and asserts zero candidates. Its zero-candidate claim therefore has a same-repo control.

## Issues Encountered

- The first anchor for the "no sort in `candidates`" mutant did not match the file, so the script reported it NOT APPLIED rather than passing. It was rerun with a line-number edit and failed as expected.

## User Setup Required

None. No external service configuration is required.

## Next Phase Readiness

- 04-09 can add `RootEntry::NotABlob` and the tree-order check. `build` already maps every `root_entry` error to `ManifestUnreadable`, and `require` already has the `ManifestNotABlob` arm.
- 04-11 can list `refs/replace/` with `refs_directly_under` and add its replacement arm in `build` and `require`.
- CONF-05 and CONF-07 stay open in REQUIREMENTS.md. `requirements.ready-ids` reports both as blocked, because 04-09, 04-10 and 04-11 also declare them and have no SUMMARY yet.

## Gate

- `scripts/check.sh all > target/gsd/04-08-all.log 2>&1` exited 0, and the last line of the log was `==> check.sh: all OK`. The final run was on `1b6641e`, after the mutants were restored: 567 tests run, 567 passed, 61 skipped. The same gate also exited 0 at `808172a` (538 run) and `a53810f`.
- Both `<verify>` loops ran after GREEN: the 7 Task 1 tests, and the 28 Task 2 tests plus `corrupt_candidate_ref_stays_in_inferred`, one `scripts/check.sh test -E 'test(=...)'` per test. Each exited 0.
- `scripts/check.sh deps` was not run. No dependency changed.

## Self-Check: PASSED

- Files FOUND: `src/git/refs.rs`, `src/model/mod.rs`, `target/gsd/04-08-red-evidence.json`, `target/gsd/04-08-red-evidence-2.json`, `target/gsd/04-08-all.log`.
- Commits FOUND as ancestors of HEAD: `488a884`, `808172a`, `a53810f`, `fb06479`, `1b6641e`.
- Acceptance criteria: `pub fn refs_directly_under` and `for-each-ref` are in `src/git/refs.rs` (exit 0). `pub fn require` and `fn candidates` are in `src/model/mod.rs` (exit 0). `require_wraps_a_missing_manifest_as_not_configured` and `require_wraps_an_unreadable_manifest_as_not_configured` are present (exit 0). `refs/wsx` is absent from both files (exit 1 each).

---
*Phase: 04-repo-model*
*Completed: 2026-11-08*
