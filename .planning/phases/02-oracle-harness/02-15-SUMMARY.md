---
phase: 02-oracle-harness
plan: 15
subsystem: testing
tags: [rust, nextest, harness, registry, harn-06, provenance]

requires:
  - phase: 02-oracle-harness
    provides: "scenario registry and rules (02-03, 02-05), vendored oracle and provenance (02-04), HARN-06 case scenarios (02-10 to 02-14)"
provides:
  - "registry::harn06_gaps and ORACLE_ACTIONS: HARN-06 completeness over the registry, per case kind and per oracle action"
  - "registry::every_scenario_module_registers_a_scenario (orchestrator-added)"
  - "provenance::vendored_files_have_their_recorded_modes (orchestrator-added)"
  - "Phase-gate evidence: host all and deps, container test, scratch audit"
affects: [phase-2-verification, phase-5, phase-6, phase-7, phase-8, phase-9, phase-10]

actuals:
  tokens: 5259
  tasks: 2
  commits: 5
plan_head_before: 7031dc91ebf6db7d1b3df86eab797908a7f2c65a
plan_head_after: 69c52c496f4fbd7badc40d1bee55a9ac956cb4f7

tech-stack:
  added: []
  patterns:
    - "Synthetic registry entries through a `tagged!` macro that defines one `fn` builder per use, because `Entry::build` is a plain fn pointer"
    - "A completeness gap list that names a missing kind once instead of each of its actions"

key-files:
  created: []
  modified:
    - tests/harness/registry.rs
    - tests/harness/provenance.rs
    - tests/harness/scenario.rs
    - .planning/phases/02-oracle-harness/02-VALIDATION.md

key-decisions:
  - "harn06_gaps reports an untagged case kind as one gap and skips its per-action gaps, so the missing-kind control gets exactly one gap"
  - "The vendored-file mode check reads the git index with git ls-files -s run in the checkout under the hermetic env; it lives in provenance.rs beside the blob check and now uses the mode column of VENDORED_BLOBS"
  - "The module-registration rule lives in registry.rs, and the scenario.rs doc comment that said a module may register nothing now points at it"

patterns-established:
  - "Each completeness or provenance check has an in-memory control that must report exactly the planted defect, plus a hand mutation of the real data shown to fail before commit"

requirements-completed: [HARN-06]

coverage:
  - id: D1
    description: "Every HARN-06 case kind is tagged on a registered scenario, and every action ORACLE_ACTIONS lists for it has an oracle-recorded scenario"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/registry.rs#every_harn06_case_has_a_scenario"
        status: pass
      - kind: integration
        ref: "tests/harness/registry.rs#harn06_gaps_names_a_missing_case_kind"
        status: pass
      - kind: integration
        ref: "tests/harness/registry.rs#harn06_gaps_names_a_missing_oracle_action"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every scenario module registers at least one scenario (orchestrator-added)"
    verification:
      - kind: integration
        ref: "tests/harness/registry.rs#every_scenario_module_registers_a_scenario"
        status: pass
      - kind: integration
        ref: "tests/harness/registry.rs#a_module_without_scenarios_is_named"
        status: pass
    human_judgment: false
  - id: D3
    description: "The index records the vendored scripts at 100755 and LICENSE, README.md and oracle.patch at 100644 (orchestrator-added)"
    verification:
      - kind: integration
        ref: "tests/harness/provenance.rs#vendored_files_have_their_recorded_modes"
        status: pass
      - kind: integration
        ref: "tests/harness/provenance.rs#mode_check_reports_a_flipped_mode"
        status: pass
      - kind: integration
        ref: "tests/harness/provenance.rs#mode_check_reports_an_empty_listing"
        status: pass
    human_judgment: false
  - id: D4
    description: "The phase gate passes on the host and in the pinned image, and no harness scratch is left behind"
    verification:
      - kind: other
        ref: "scripts/check.sh all"
        status: pass
      - kind: other
        ref: "scripts/check.sh deps"
        status: pass
      - kind: other
        ref: "scripts/check-in-container.sh test"
        status: pass
      - kind: other
        ref: "planted harness-audit-control scratch audit over target/tmp"
        status: pass
    human_judgment: false
  - id: D5
    description: "Each HARN-06 tag is apt: the scenario tagged with a kind really exercises that situation"
    verification: []
    human_judgment: true
    rationale: "The completeness test proves tags exist with the right registration and action; whether each tagged scenario's fixture really is that case is a reading of the scenario, reviewed at verification"

duration: 7min
completed: 2026-10-26
status: complete
---

# Phase 2 Plan 15: HARN-06 completeness and phase gate Summary

**The registry now fails the gate unless all seven HARN-06 case kinds are tagged, and unless each of the 19 (kind, action) pairs the oracle runs has an oracle-recorded scenario. Two orchestrator-added rules are in place (every scenario module registers a scenario; vendored oracle file modes), each with a control. The phase gate is green on the host and in the pinned image.**

## Performance

- **Duration:** 7 min
- **Started:** 2026-10-26T19:54:21Z
- **Completed:** 2026-10-26T20:01:45Z
- **Tasks:** 2 (plus the two orchestrator-added checks)
- **Files modified:** 4

## Accomplishments

- `harn06_gaps` with `ORACLE_ACTIONS`. A kind no scenario is tagged with is one gap, `<kind>: no scenario is tagged with this case`. A listed action with no `Kind::Oracle` scenario of that kind whose subject runs it is `<kind>: <action>`, for example `no-GSD repo: cut`.
- `every_harn06_case_has_a_scenario` runs on the real registry in every gate and prints the per-kind listing below.
- Orchestrator-added: `every_scenario_module_registers_a_scenario` restores the rule 02-03 dropped while the modules were empty.
- Orchestrator-added: `vendored_files_have_their_recorded_modes` checks the index modes of `tests/oracle`.
- The phase gate passed: `check.sh all`, `check.sh deps` and `check-in-container.sh test` all exit 0. The 41 skipped tests equal the 41 lines of the disabled-side report, and no `harness-*` scratch is left on the host or in the container's target volume.

## Per-kind listing (printed by `every_harn06_case_has_a_scenario`)

```
merge commits:
  cut::topic_merge_dropped (cut, oracle-recorded)
  cut::integration_merged_into_feature (cut, oracle-recorded)
  cut::merge_resolution_adds_personal_file (cut, oracle-recorded)
  cut::merge_resolution_only_personal_file (cut, oracle-recorded)
  guard::merge_resolution_adds_personal_file (pre-push, oracle-recorded)
squash-merged PRs:
  sync::after_squash_merged_pr (sync, oracle-recorded)
  worktree::rm_after_squash_merge (remove worktree, Rust-only)
  cut::recut_after_squash_merge (cut, oracle-recorded)
fork layout:
  sync::fork_layout (sync, oracle-recorded)
  worktree::fork_layout (new worktree, oracle-recorded)
  check::fork_layout (check, oracle-recorded)
  cut::fork_layout (cut, oracle-recorded)
  guard::fork_layout (pre-push, oracle-recorded)
scoped and unscoped commits in one cut:
  cut::scoped_and_unscoped_commits (cut, oracle-recorded)
no-GSD repo:
  sync::no_gsd_repo (sync, oracle-recorded)
  worktree::no_gsd_repo (new worktree, oracle-recorded)
  check::no_gsd_repo (check, oracle-recorded)
  check::no_gsd_custom_personal_path (check, Rust-only)
  cut::no_gsd_repo (cut, oracle-recorded)
  cut::no_gsd_custom_personal_path (cut, Rust-only)
  guard::no_gsd_repo (pre-push, oracle-recorded)
  guard::no_gsd_custom_personal_path (pre-push, Rust-only)
non-ASCII, newline and non-UTF-8 path names:
  check::quoted_personal_names (check, oracle-recorded)
  cut::quoted_personal_names (cut, oracle-recorded)
  guard::quoted_non_ascii_name (pre-push, oracle-recorded)
  guard::quoted_newline_name (pre-push, oracle-recorded)
  guard::quoted_non_utf8_name (pre-push, oracle-recorded)
existing PR branch with extra commits:
  cut::existing_pr_branch_with_extra_commits (cut, oracle-recorded)
```

## TDD Gate Compliance (Task 1)

- **RED** `836417f` `test(02-15): add failing HARN-06 completeness control`. Command `scripts/check.sh test -E 'test(=registry::harn06_gaps_names_a_missing_case_kind)'` exited 100. The JUnit report `target/nextest/default/junit.xml` was written at 19:55:27.201Z; the run started at 19:55:25.617Z. The record is `target/gsd/02-15-red-evidence.json` (gitignored). Classifier verdict: **`RED_EVIDENCE_OK`** (`target_test_failed`, matched `wsx::harness#registry::harn06_gaps_names_a_missing_case_kind`, 1 test, 1 fail).
- **Semantic assessment:** the target compiled clean (`check.sh clippy` OK) and ran. It failed on the planned `assert_eq` at `registry.rs:226`: `left: []`, `right: ["fork layout: no scenario is tagged with this case"]`. The stub returned no gaps, so a missing kind read as covered. That is the defect this check exists to catch, and there was no build, fixture or load fault.
- **GREEN** `dec51a7` `feat(02-15): check that every HARN-06 case has a scenario`. All three named tests pass, and `check.sh all` passed before the commit.
- **REFACTOR:** none.

## Negative controls (each must produce the opposite result)

| Check | In-memory control (committed) | Real-data mutation (run by hand, reverted) |
|---|---|---|
| HARN-06 kinds | `harn06_gaps_names_a_missing_case_kind`: exactly `["fork layout: no scenario is tagged with this case"]` | Retagged `sync::after_squash_merged_pr` as `Merges`; the real-registry test failed with `HARN-06 gaps: squash-merged PRs: sync`; `git checkout -- tests/harness/sync.rs` |
| HARN-06 oracle actions | `harn06_gaps_names_a_missing_oracle_action`: exactly `["no-GSD repo: cut"]` | (same run as above: one action gap named) |
| Module registers a scenario | `a_module_without_scenarios_is_named`: exactly `["empty"]` | Emptied `signing`'s `scenarios!`; the test failed with `scenario modules registering no scenario: ["signing"]`; reverted |
| Vendored modes | `mode_check_reports_a_flipped_mode`: exactly `["hooks/pre-push: mode 100644, must be 100755"]`; `mode_check_reports_an_empty_listing`: all 8 files `not in the index` | `git update-index --chmod=-x tests/oracle/hooks/pre-push`; the test failed with that same line; restored with `--chmod=+x` and confirmed `100755` |
| Scratch audit | The planted `target/tmp/harness-audit-control-$$` was found before the leftover check ran | n/a |

## Phase gate (run on code tip `6c98d78`)

`scripts/check.sh all` (host) exited 0:
```
    Rust side not enabled (41):
      check::fork_layout (Rust side not enabled until phase 5)
      check::no_gsd_custom_personal_path (Rust side not enabled until phase 5)
      check::no_gsd_repo (Rust side not enabled until phase 5)
      check::quoted_personal_names (Rust side not enabled until phase 5)
      check::self_test (Rust side not enabled until phase 5)
      cut::conflict_leaves_partial_pr_branch (Rust side not enabled until phase 9)
      cut::existing_pr_branch_with_extra_commits (Rust side not enabled until phase 10)
      cut::fork_layout (Rust side not enabled until phase 9)
      cut::integration_merged_into_feature (Rust side not enabled until phase 9)
      cut::merge_resolution_adds_personal_file (Rust side not enabled until phase 9)
      cut::merge_resolution_only_personal_file (Rust side not enabled until phase 9)
      cut::no_gsd_custom_personal_path (Rust side not enabled until phase 9)
      cut::no_gsd_repo (Rust side not enabled until phase 9)
      cut::quoted_personal_names (Rust side not enabled until phase 9)
      cut::recut_after_squash_merge (Rust side not enabled until phase 10)
      cut::scoped_and_unscoped_commits (Rust side not enabled until phase 10)
      cut::self_test (Rust side not enabled until phase 9)
      cut::topic_merge_dropped (Rust side not enabled until phase 9)
      guard::fork_layout (Rust side not enabled until phase 6)
      guard::merge_resolution_adds_personal_file (Rust side not enabled until phase 6)
      guard::name_prefix_is_not_personal (Rust side not enabled until phase 6)
      guard::new_branch_without_own_commits (Rust side not enabled until phase 6)
      guard::no_gsd_custom_personal_path (Rust side not enabled until phase 6)
      guard::no_gsd_repo (Rust side not enabled until phase 6)
      guard::quoted_newline_name (Rust side not enabled until phase 6)
      guard::quoted_non_ascii_name (Rust side not enabled until phase 6)
      guard::quoted_non_utf8_name (Rust side not enabled until phase 6)
      guard::self_test (Rust side not enabled until phase 6)
      guard::unlisted_branch_pattern (Rust side not enabled until phase 6)
      signing::cut_replay_signed (Rust side not enabled until phase 9)
      signing::sync_merge_signed (Rust side not enabled until phase 7)
      sync::after_squash_merged_pr (Rust side not enabled until phase 7)
      sync::already_up_to_date (Rust side not enabled until phase 7)
      sync::fork_layout (Rust side not enabled until phase 7)
      sync::no_gsd_repo (Rust side not enabled until phase 7)
      sync::self_test (Rust side not enabled until phase 7)
      worktree::fork_layout (Rust side not enabled until phase 8)
      worktree::no_gsd_repo (Rust side not enabled until phase 8)
      worktree::rerun_with_own_commits (Rust side not enabled until phase 8)
      worktree::rm_after_squash_merge (Rust side not enabled until phase 8)
      worktree::self_test (Rust side not enabled until phase 8)
     Summary [  15.756s] 153 tests run: 153 passed, 41 skipped
==> check.sh: all OK
```
The skipped count (41) equals the report's 41 lines, and the plan's Task 2 verify chain printed `VERIFY-OK`.

`scripts/check.sh deps` exited 0:
```
==> cargo deny --locked check
advisories ok, bans ok, licenses ok, sources ok
==> check.sh: deps OK
```

`scripts/check-in-container.sh test` exited 0. Image `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`:
```
    Starting 153 tests across 8 binaries (41 tests skipped)
     Summary [  35.830s] 153 tests run: 153 passed, 41 skipped
==> check.sh: test OK
```
Every HARN-06 scenario's oracle side passed in the CI image. No test failed in the container that passes on the host.

**Scratch audit:** the plan's audit command exited 0. It found the planted `harness-audit-control-*` directory, removed it, and then found no `harness-*` directory in host `target/tmp`. `env::scenario_dirs_carry_the_harness_prefix` passed. The container's target volume `wsx-ci-target-420d8c328b7e` was checked read-only from the same image: `harness count: 0`. Neither audit found anything to remove. The non-harness directories in `target/tmp` (`setup-repo-*`, `missing-tool-*` and others) belong to the script tests. They are outside this audit and were left alone. `WSX_SCRATCH` is unset; this plan's scratch was its session scratchpad, removed after this SUMMARY was committed.

## What these checks do not establish

- The completeness test proves that each kind is tagged, and that each `ORACLE_ACTIONS` pair is covered by a `Kind::Oracle` scenario whose subject runs that action. It does not prove that each tag is apt, meaning that the fixture really is that case. That is a reading of the scenario, left to verification (coverage D5).
- `ORACLE_ACTIONS` is a hand table of 7 kinds. A new `CaseKind` variant would not be flagged until someone adds it to the table: there is no compile-time exhaustiveness link between the enum and the table.
- An oracle-recorded scenario counts whatever its class (Parity, `fixed:` or `regression:`). The plan's definition requires exactly that. Whether the oracle run matches its target is proven by that scenario's own `::oracle` test, not here.
- The skipped count equals the disabled-side report only while no Rust side is enabled and no other test is ignored.
- The mode check reads the index of the checkout it runs in. On the host and in the container's read-only gitdir mount, `git ls-files` worked. In GitHub Actions CI it has not run yet (unverified): a checkout without a `.git` directory, or one git calls dubious ownership, would fail the test loudly rather than skip it.
- The container-volume leftover audit had no planted control. The listing of the volume shows it was read; the host audit is the one with a control.

## Decisions Made

- `harn06_gaps` names a missing kind once, not once per action, so the missing-kind control has exactly one gap.
- The mode check sits in `provenance.rs` beside the content (blob) check and reuses the mode column of `VENDORED_BLOBS`, which no code read before. `README.md` and `oracle.patch` are in a separate `LOCAL_MODES` table. A file listed in the index with no recorded mode is reported, as is a line that does not parse.

## Deviations from Plan

### Orchestrator-added checks

**1. [Orchestrator-added] Every scenario module registers at least one scenario**
- **Found during:** orchestrator carry-forward note (02-03 removed the rule while the modules were empty)
- **Fix:** `modules_without_scenarios` with `every_scenario_module_registers_a_scenario` over `crate::registries()`, and the control `a_module_without_scenarios_is_named`. The `scenario.rs` doc comment on `registry_entries_match_their_scenarios`, which said a module may register nothing, now points at the rule.
- **Files modified:** tests/harness/registry.rs, tests/harness/scenario.rs
- **Verification:** both tests pass; the real-data mutation (emptied `signing`) failed as shown above
- **Committed in:** `110d966`

**2. [Orchestrator-added] Vendored-file mode check**
- **Found during:** orchestrator carry-forward note (02-04's blob check covers content only)
- **Fix:** `mode_mismatches` and `oracle_index_listing` (`git ls-files -s -- tests/oracle`, hermetic env, checkout as cwd), the test `vendored_files_have_their_recorded_modes`, and the controls `mode_check_reports_a_flipped_mode` and `mode_check_reports_an_empty_listing`
- **Files modified:** tests/harness/provenance.rs
- **Verification:** passes on the host and in the container; the real index mutation failed as shown above
- **Committed in:** `6c98d78`

### Other

- `02-VALIDATION.md`: the rows for `every_harn06_case_has_a_scenario` and for the scratch audit (`env::scenario_dirs_carry_the_harness_prefix`) are now marked `✅ | ✅ green` (commit `69c52c4`). `nyquist_compliant` was not touched. No rows were added for the two orchestrator-added checks; validate-phase owns that.

**Total deviations:** 2 orchestrator-added checks, 1 documentation update. **Impact:** additional tests only, no change to the behaviour of existing code.

## Issues Encountered

- `scripts/check.sh test` rejects `--no-capture`, so the per-kind listing was captured with `cargo nextest run --locked --no-tests=fail --no-capture -E 'test(=registry::every_harn06_case_has_a_scenario)'`. In a normal run, nextest hides this listing because the test passes.

## User Setup Required

None.

## Next Phase Readiness

- Phase 2's last plan is done, and the phase is ready for `/gsd-verify-work 2`. The judgment item for the verifier is coverage D5: whether each HARN-06 tag is apt.

## Self-Check: PASSED

- Files: tests/harness/registry.rs, tests/harness/provenance.rs, tests/harness/scenario.rs and 02-VALIDATION.md are all present.
- Commits on HEAD: `836417f`, `dec51a7`, `110d966`, `6c98d78` and `69c52c4` (5, measured by `git rev-list --count 7031dc9..HEAD`).
- `rg -q 'fn harn06_gaps' tests/harness/registry.rs` exits 0. The `test(02-15)` commit precedes the `feat(02-15)` commit.

---
*Phase: 02-oracle-harness*
*Completed: 2026-10-26*
