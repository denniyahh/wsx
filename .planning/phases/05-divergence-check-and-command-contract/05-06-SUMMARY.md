---
phase: 05-divergence-check-and-command-contract
plan: 06
subsystem: check
tags: [rust, git, hooks, symlinks, divergence-check, json-contract]

requires:
  - phase: 05-03
    provides: Repo::hooks_path, Repo::list_tree, Repo::read_blobs, ListedEntry and the MODE_ constants
  - phase: 05-04
    provides: check::run order (resolve, tree_diff, lookup_paths) and the D-05 problem groups
provides:
  - "src/check/hooks.rs: HookStatus, SYMLINK_HOPS = 40, normalise, at_tip, evaluate_hooks and the in-tree symlink resolver"
  - "ProblemKind::HookNotExecutable (hook_not_executable), D-05's third group"
  - "Report.hooks and the JSON hooks object before problems"
  - "the stderr notice 'wsx check: core.hooksPath <value> is not checked: <reason>'"
  - "scenarios hook_mode_from_the_commit, hook_symlink_to_an_executable, scripts_sh_not_checked, hooks_path_not_in_the_tree"
affects: [05-07, 05-08, 05-09, check, json-contract]

actuals:
  tokens: 13132
  tasks: 2
  commits: 4
plan_head_before: 480302257f3afdb3a5f729be1f3a3944dcb4442d
plan_head_after: 62556d398cb4db2e6c14f1992cee98d5f98f2871

tech-stack:
  added: []
  patterns:
    - "Kernel-style path resolution over a tree listing: components walked from the link's parent, links spliced in place, one hop budget"
    - "Fixture steps assert git's own view (ls-tree modes, config --get) before the action runs"

key-files:
  created:
    - src/check/hooks.rs
  modified:
    - src/check/mod.rs
    - src/check/report.rs
    - src/git/mod.rs
    - tests/harness/check.rs
    - tests/harness/contract.rs

key-decisions:
  - "The symlink blob read runs only when the hooks directory is Checked: an unset, refused or absent hooks path reads no listing or no blobs beyond what it needs"
  - "A link whose text is missing from the targets map fails with its own reason, never passes; run always reads every symlink of the listing, so this cannot occur from run"
  - "A resolution that ends on the tree root names it as '.' in the is-a-directory reason"

patterns-established:
  - "A harness fixture that needs a committed symlink removes it from the working tree after the commit, because the outcome observer records regular files only"

requirements-completed: [DIV-03]

coverage:
  - id: D1
    description: "Hook rule over the tip's tree listing: 100755 passes, other modes fail naming the mode, symlinks resolve in-tree through directory links under a 40-link budget"
    requirement: DIV-03
    verification:
      - kind: unit
        ref: "src/check/hooks.rs#check::hooks::tests (24 tests)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A hook committed 100644 and executable on disk fails on both sides (Parity); clean control passes"
    requirement: DIV-03
    verification:
      - kind: integration
        ref: "tests/harness/check.rs#check::hook_mode_from_the_commit::rust"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::hook_mode_from_the_commit::oracle"
        status: pass
    human_judgment: false
  - id: D3
    description: "A symlinked hook to a tracked executable passes (Fixed vs the oracle); a link to a 100644 file fails on both sides"
    requirement: DIV-03
    verification:
      - kind: integration
        ref: "tests/harness/check.rs#check::hook_symlink_to_an_executable::rust"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::hook_symlink_to_an_executable::oracle"
        status: pass
    human_judgment: false
  - id: D4
    description: "scripts/lib.sh committed 100644 outside the hooks directory is not judged (Fixed vs the oracle's scripts/*.sh rule)"
    requirement: DIV-03
    verification:
      - kind: integration
        ref: "tests/harness/check.rs#check::scripts_sh_not_checked::rust"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::scripts_sh_not_checked::oracle"
        status: pass
    human_judgment: false
  - id: D5
    description: "A core.hooksPath wsx does not check is said on stderr and in JSON; the three hooks object forms"
    requirement: DIV-03
    verification:
      - kind: integration
        ref: "tests/harness/check.rs#check::hooks_path_not_in_the_tree::rust"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::hooks_path_not_in_the_tree::oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/contract.rs#contract::check_json_hooks_not_set"
        status: pass
      - kind: integration
        ref: "tests/harness/contract.rs#contract::check_json_hooks_checked"
        status: pass
      - kind: integration
        ref: "tests/harness/contract.rs#contract::check_json_hooks_not_in_the_tree"
        status: pass
    human_judgment: false

duration: 17min
completed: 2026-11-18
status: complete
---

# Phase 5 Plan 06: Hook modes from the committed tree Summary

**`wsx check` judges every file under git's effective `core.hooksPath` by its mode in the workspace tip's tree, resolving symlinks inside the tree as the kernel does (directory links followed, 40-link budget), and reports the hooks path status in JSON and on stderr.**

## Performance

- **Duration:** 17 min
- **Started:** 2026-11-18T21:09:06.253Z
- **Completed:** 2026-11-18T21:26:00Z
- **Tasks:** 2
- **Files modified:** 6

## Accomplishments
- A hook committed 100644 now fails the check even when it is executable on disk, so a hook that would never run on a fresh clone is caught (success criterion 3).
- A symlinked hook passes when its target resolves in the committed tree to a 100755 blob, through directory links too; loops, escapes, absolute and untracked targets fail naming the hook.
- `scripts/*.sh` outside the hooks directory is no longer judged, a recorded fix over the oracle.
- A hooks path wsx cannot check (unset, absolute, not a tree path, or no directory at the tip) is always reported with its reason.

## Task Commits

1. **Task 1: hook rule and symlink resolution, pure** - `1f4499a` (test, RED), `8562e01` (feat, GREEN)
2. **Task 2: wiring, JSON, four scenarios** - `0c80367` (test, RED), `62556d3` (feat, GREEN)

## TDD Gate Compliance

- **Task 1 RED:** `check::hooks::tests::plain_hook_fails_naming_its_mode` failed on its planned `assert_eq!` (left `[]` from the stub, right the one `hook_not_executable` problem). Classifier: `RED_EVIDENCE_OK` (target/gsd/05-06-red-evidence.json, JUnit report). Semantic assessment: the target compiled and ran; the failure is the planned assertion, not a build, load or fixture fault.
- **Task 2 RED:** `check::hook_mode_from_the_commit::rust` failed on the scenario judge's exit-code and stderr comparison (expected exit 1 and the hook line, got exit 0 and empty stderr) after its fixture built. Classifier: `RED_EVIDENCE_OK` (target/gsd/05-06-red-evidence-2.json). Semantic assessment: the planned failure, with `Report.hooks` still `NotSet`.
- RED precedes GREEN by ancestry in both tasks (1f4499a then 8562e01; 0c80367 then 62556d3).

## Files Created/Modified
- `src/check/hooks.rs` - HookStatus, SYMLINK_HOPS, normalise, at_tip, evaluate_hooks, the resolver, 24 unit tests
- `src/check/mod.rs` - HookNotExecutable (group 2), Report.hooks, check_hooks wired into run after resolve
- `src/check/report.rs` - the hook problem line, the not-checked notice, the hooks JSON object
- `src/git/mod.rs` - re-exports ListedEntry, MODE_EXECUTABLE, MODE_SYMLINK, MODE_TREE
- `tests/harness/check.rs` - four scenarios and their fixture steps (17 registered)
- `tests/harness/contract.rs` - check_json_hooks_not_set, check_json_hooks_checked, check_json_hooks_not_in_the_tree

## Decisions Made
- Symlink blobs are read only once the hooks directory is `Checked` (fewer children than reading whenever the listing holds a link).
- A resolution ending at the tree root names it `.`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Test correctness] symlink_leaving_the_tree_fails uses ../../../x**
- **Found during:** Task 1 (RED)
- **Issue:** The plan's example `../../x` from `scripts/hooks` lands at the root's `x`, inside the tree, so it cannot prove the leave-the-tree case.
- **Fix:** `../../../x` fails with "leaves the tree"; `../../x` with a root `x` 100755 is kept as the passing control, pinning the exact boundary.
- **Files modified:** src/check/hooks.rs
- **Committed in:** 1f4499a

**2. [Rule 2 - Missing control] Boundary and negative controls in the resolver tests**
- **Found during:** Task 1 (RED)
- **Issue:** Several behaviours had only one direction.
- **Fix:** 41 links fails beside the 40-link pass; the dot-dot test also swaps modes so it must fail naming scripts/vendor/run; the beside-the-dir and scripts.sh tests add a positive that must still trip.
- **Files modified:** src/check/hooks.rs
- **Committed in:** 1f4499a

**3. [Rule 3 - Blocking] Interim dead-code expectations in Task 1**
- **Found during:** Task 1 (GREEN), `scripts/check.sh all`
- **Issue:** With no caller until Task 2, the hooks module and the new variant were dead code under `-D warnings`.
- **Fix:** `expect(dead_code)` on the module and `cfg_attr(not(test), expect(dead_code))` on the variant, both removed in Task 2 GREEN.
- **Committed in:** 8562e01 (added), 62556d3 (removed)

**4. [Rule 3 - Blocking] Committed symlink removed from the fixture's working tree**
- **Found during:** Task 2 (GREEN)
- **Issue:** The harness observer refuses any non-regular file in a checkout ("the observer records regular files only"), so the symlink scenario could not run.
- **Fix:** `hook_symlink` removes the link from the working tree after committing it and asserting 120000; the commit and the index keep it, which is all either side reads.
- **Files modified:** tests/harness/check.rs
- **Committed in:** 62556d3

---

**Total deviations:** 4 auto-fixed (1 test correctness, 1 missing control, 2 blocking). **Impact:** no scope change.

## Issues Encountered
None.

## Verification
- All 24 `check::hooks::tests` pass, each run as its own `-E 'test(=...)'` command.
- The eight scenario sides and the four contract tests pass, one command each.
- `scripts/check.sh all` exits 0 with last line `==> check.sh: all OK` (target/gsd/05-06-all.log), after each GREEN commit.
- `rg 'std::fs|index_path|symlink_metadata' src/check/hooks.rs` finds nothing; `rg -c 'rust_enabled;' tests/harness/check.rs` prints 17; in `run`, `check_hooks` (which makes the `hooks_path` call) sits after `model::resolve`.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
Ready for 05-07.

---
*Phase: 05-divergence-check-and-command-contract*
*Completed: 2026-11-18*

## Self-Check: PASSED
