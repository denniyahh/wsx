---
phase: 04-repo-model
plan: 02
subsystem: testing
tags: [rust, harness, fixture, git-config, tdd]

requires:
  - phase: 02
    provides: "harness fixture presets (RepoConfig::devflow, with_remotes, local-config block) and the oracle scenarios"
provides:
  - "harness fixture local config in the repo model's form: bare wsx.baseRef (D-08), explicit wsx.baseRemote and wsx.pushRemote (D-02)"
  - "fork preset states wsx.baseRemote=upstream instead of relying on detection (D-05)"
  - "devflow and fork fixture tests pin all three keys"
affects: [phase-5-check, phase-6, phase-7, phase-8, phase-9, phase-10, phase-12-init-migrate]

actuals:
  tokens: 413
  tasks: 1
  commits: 2
plan_head_before: 02329c9c2d207c74f6a0e67d3d311790c3efe9d8
plan_head_after: c08a6951d60060918263bda1d3e55c76714b0e80

tech-stack:
  added: []
  patterns:
    - "fixture writes wsx.* keys straight from RepoConfig fields; no derived <remote>/<branch> strings"

key-files:
  created: []
  modified:
    - tests/harness/fixture.rs

key-decisions:
  - "Fixture writes wsx.baseRef as the bare integration branch and both remote keys from RepoConfig, so phase 5 Rust sides start from config the model accepts (D-02, D-05, D-08)"

patterns-established:
  - "Harness presets state every wsx remote key explicitly; none relies on the model's origin default"

requirements-completed: [CONF-03, CONF-04]

coverage:
  - id: D1
    description: "Devflow preset writes wsx.baseRef=develop, wsx.baseRemote=origin, wsx.pushRemote=origin"
    requirement: "CONF-03"
    verification:
      - kind: integration
        ref: "tests/harness/fixture.rs#fixture::devflow_layout_matches_the_preset"
        status: pass
    human_judgment: false
  - id: D2
    description: "Fork preset writes wsx.baseRef=develop, wsx.baseRemote=upstream, wsx.pushRemote=origin"
    requirement: "CONF-04"
    verification:
      - kind: integration
        ref: "tests/harness/fixture.rs#fixture::fork_layout_has_no_integration_branch_on_the_push_remote"
        status: pass
    human_judgment: false
  - id: D3
    description: "No oracle scenario depended on the old wsx.baseRef form: full suite passes with only fixture.rs changed under tests/harness"
    verification:
      - kind: other
        ref: "scripts/check.sh all (367 run, 367 passed, 58 skipped) and git diff --name-only 6946ae9 -- tests/harness"
        status: pass
    human_judgment: false

duration: 3min
completed: 2026-11-08
status: complete
---

# Phase 4 Plan 02: Harness fixture config in the repo model's form Summary

**The harness fixture now writes `wsx.baseRef` as a bare branch name plus explicit `wsx.baseRemote` and `wsx.pushRemote`, so phase 5's first Rust scenarios will not refuse on a branch literally named `origin/develop`.**

## Performance

- **Duration:** about 3 min
- **Started:** 2026-11-08T15:32:01Z
- **Completed:** 2026-11-08T15:35Z
- **Tasks:** 1 (TDD: RED + GREEN, no refactor)
- **Files modified:** 1

## Accomplishments
- Fixture local-config block writes `wsx.baseRef = config.integration`, `wsx.baseRemote = config.base_remote`, `wsx.pushRemote = config.push_remote`; `wsx.sourceBranch`, `commit.gpgsign` and the trailing `extra_config` are unchanged.
- `fixture::devflow_layout_matches_the_preset` pins `develop` / `origin` / `origin`; `fixture::fork_layout_has_no_integration_branch_on_the_push_remote` pins `develop` / `upstream` / `origin`.
- Full suite still green with no scenario file, `tests/harness/oracle.rs` or `tests/harness/check.rs` touched.

## Task Commits

1. **Task 1 RED: expect bare baseRef and explicit remotes** - `6f3af15` (test)
2. **Task 1 GREEN: write the repo model's config keys** - `c08a695` (fix)

## TDD record

- **RED:** both assertion tables changed first. `scripts/check.sh test -E 'test(=fixture::devflow_layout_matches_the_preset)'` exited 100; 1 test run, 1 failed, at `tests/harness/fixture.rs:674`, `local config wsx.baseRef`, left `"origin/develop\n"`, right `"develop\n"`. Evidence record `target/gsd/04-02-red-evidence.json` (JUnit `target/nextest/default/junit.xml`, run started 2026-11-08T15:32:12.195Z, report modified 15:32:14.659Z). `gsd_run check tdd-red-evidence target/gsd/04-02-red-evidence.json --raw` printed verdict `RED_EVIDENCE_OK`, reason `target_test_failed`, matched `wsx::harness#fixture::devflow_layout_matches_the_preset`.
- **Semantic assessment:** the target ran and failed on the planned assertion after the preset built (the earlier `.wsx.conf`, HEAD and push-remote assertions in the same test passed); no build, setup or unrelated failure. The fork test was also run in RED and failed the same way at line 835 (left `"upstream/develop\n"`).
- **Negative control on the classifier:** the same record fed the post-GREEN (passing) report returned `INVALID_RED` / `unexpected_green`, so the classifier does distinguish the two. The control file was deleted.
- **GREEN:** both target tests pass; `scripts/check.sh all` exit 0.
- **REFACTOR:** none needed.

## Files Created/Modified
- `tests/harness/fixture.rs` - preset local config keys and the two layout assertions

## Decisions Made
None beyond the plan; it implements D-02, D-05 and D-08 as written.

## Deviations from Plan

None - plan executed exactly as written.

## TDD Gate Compliance

The GREEN commit uses the subject the plan prescribes, `fix(04-02): ...`, not `feat(04-02): ...`. The tdd.md executor gate greps for `^feat\(04-02\):` and finds nothing, so an automated TDD review will report this plan's GREEN as missing. The change is a fix to test fixture data, not a new feature, so `fix` is the accurate type; the RED commit `6f3af15` precedes GREEN `c08a695` and GREEN descends from it.

## Verification

| Command | Exit / result |
|---|---|
| `scripts/check.sh test -E 'test(=fixture::devflow_layout_matches_the_preset)'` (RED) | 100, target failed on the planned assertion |
| `gsd_run check tdd-red-evidence target/gsd/04-02-red-evidence.json --raw` | `RED_EVIDENCE_OK` |
| `scripts/check.sh test -E 'test(=fixture::devflow_layout_matches_the_preset)' && scripts/check.sh test -E 'test(=fixture::fork_layout_has_no_integration_branch_on_the_push_remote)'` | 0 (pre- and post-commit) |
| `scripts/check.sh all > target/gsd/04-02-all.log 2>&1` + last-line check | 0, `==> check.sh: all OK`, 367 run / 367 passed / 58 skipped (run twice, pre- and post-commit) |
| AC1 `rg -qF` for both new `.to_owned(), config.*_remote` writes | 0 (baseline `6946ae9`: 1) |
| AC2 `rg -qF 'upstream/develop\n'` / `'origin/develop\n'` | 1 / 1 (baseline `6946ae9`: 0 / 0) |
| AC3 `git diff --name-only 6946ae9 -- tests/harness` | `tests/harness/fixture.rs` only |

**What this does not establish:** it proves the fixture writes the agreed values and that no existing scenario depended on the old form. It does not prove `model::resolve` accepts them end to end: the harness is a separate test crate and cannot call the binary crate's model, and no wsx command exists until phase 5. `model::tests::fork_layout_resolves_each_remote` (04-06) and `model::tests::base_ref_overrides_the_chain` (04-07) cover the remote keys and a bare `wsx.baseRef` on the model's own fixture; no model test sets the three keys together. The suite passing also shows only that the oracle scenarios that exist today do not read the key; it says nothing about scenarios phase 5 adds.

## Issues Encountered
None.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
Ready for 04-03. Phase 5's first enabled Rust scenarios will run against these keys.

## Self-Check: PASSED
- `tests/harness/fixture.rs` exists and carries the new writes (AC1).
- `6f3af15` and `c08a695` are ancestors of HEAD.
- `git rev-list --count 02329c9..HEAD` = 2 before this SUMMARY commit.

---
*Phase: 04-repo-model*
*Completed: 2026-11-08*
