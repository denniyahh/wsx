---
phase: 02-oracle-harness
plan: 04
subsystem: testing
tags: [oracle, provenance, git-apply, blob-hash, negative-controls]

requires:
  - phase: 02-oracle-harness
    provides: "02-01: tests/oracle (devflow ee6ec792 scripts, LICENSE, oracle.patch)"
  - phase: 02-oracle-harness
    provides: "02-02: ScenarioDirs, hermetic_env, env::command, OracleCopy::prepare, oracle::run_script, oracle::spawn_and_wait"
  - phase: 02-oracle-harness
    provides: "02-03: provenance.rs header, RepoConfig::devflow, Fixture"
provides:
  - "DEVFLOW_COMMIT, VENDORED_BLOBS (six files: path, devflow path, mode, blob) and PATCHED_BLOBS (three post-patch blobs) in tests/harness/provenance.rs"
  - "blob_mismatches: hashes each listed file with git hash-object --no-filters in the hermetic env and reports every mismatch or unhashable file by path"
  - "readme_missing_records: the recorded values tests/oracle/README.md lacks"
  - "tests/oracle/README.md: provenance, blob tables, MIT attribution, one row per patch hunk with its reason, rules, re-vendoring steps"
  - "Eight provenance:: tests, each check paired with a control that must trip"
affects: [02-15, phase-15-cutover]

actuals:
  tokens: 6575   # chars/4 over `git diff 64d7844 fd18911` (26298 chars)
  tasks: 2
  commits: 3
plan_head_before: 64d7844838aed7c8f43996dce9cbb66e0b887ba3
plan_head_after: fd18911b7308d64defac81b5472c5ae42f91a764

tech-stack:
  added: []
  patterns:
    - "Recorded hashes live as named consts in the test module; the README is tied to them by a drift test, not by convention"
    - "A checker returns every mismatch as `<path>: <reason>`, and an unhashable file counts as a mismatch, so a missing file never reads as a match"
    - "git subprocesses that need no repository run in the scenario HOME, a proper subdirectory of the ceiling, so discovery never reaches the wsx checkout"

key-files:
  created:
    - tests/oracle/README.md
  modified:
    - tests/harness/provenance.rs

key-decisions:
  - "patched_scripts_require_oracle_dir spawns `bash <copy>/<script>` itself, mirroring run_script's command line, because run_script always sets WSX_ORACLE_DIR; patched_scripts_run_with_oracle_dir goes through run_script"
  - "patched_scripts_run_with_oracle_dir also requires a stdout marker per script: with a wrong WSX_ORACLE_DIR, phase-worktree.sh prints `not found; skipping sync` and still exits 0, so exit 0 alone would not prove its hunk worked"
  - "blob_mismatches takes the ScenarioDirs as well as the env, and runs git hash-object in the scenario HOME rather than in the hashed directory"
  - "The no-ceiling control runs `git apply -v` and requires `Skipped patch '<file>'.` for each patched file, so it fails if the apply ever stops being a silent skip"

patterns-established:
  - "Each provenance check is one test and its control another, with a `Counterpart:` doc comment naming what the control rules out"

requirements-completed: [HARN-01]

coverage:
  - id: D1
    description: "The six files under tests/oracle hash to the blobs recorded for devflow ee6ec792, and a copy with one flipped byte is reported naming exactly that file"
    requirement: HARN-01
    verification:
      - kind: unit
        ref: "tests/harness/provenance.rs#provenance::vendored_files_match_devflow_blobs"
        status: pass
      - kind: unit
        ref: "tests/harness/provenance.rs#provenance::flipped_byte_copy_is_reported (RED_EVIDENCE_OK at 1ec67d0)"
        status: pass
      - kind: other
        ref: "control A: one recorded vendored blob altered -> vendored_files_match_devflow_blobs fails naming sync-workspace.sh"
        status: pass
    human_judgment: false
  - id: D2
    description: "oracle.patch applies cleanly through OracleCopy::prepare, the three patched scripts hash to PATCHED_BLOBS, the other three files keep their devflow blobs, and a second apply fails with `patch does not apply`"
    requirement: HARN-01
    verification:
      - kind: integration
        ref: "tests/harness/provenance.rs#provenance::oracle_patch_applies_cleanly"
        status: pass
      - kind: other
        ref: "control B: one PATCHED_BLOBS value altered -> fails naming hooks/pre-push"
        status: pass
    human_judgment: false
  - id: D3
    description: "Without GIT_CEILING_DIRECTORIES, inside a repository at the scenario root, git apply skips every file and exits 0, and OracleCopy::prepare refuses that copy"
    requirement: HARN-01
    verification:
      - kind: integration
        ref: "tests/harness/provenance.rs#provenance::patch_apply_without_ceiling_is_a_silent_no_op"
        status: pass
      - kind: other
        ref: "control C: ceiling kept -> the apply prints `Applied patch ... cleanly` and the test fails"
        status: pass
    human_judgment: false
  - id: D4
    description: "Each patched script fails with bash's `WSX_ORACLE_DIR: parameter null or not set` when the variable is absent, and runs to exit 0 (with its stdout marker) through run_script when it is set"
    requirement: HARN-01
    verification:
      - kind: integration
        ref: "tests/harness/provenance.rs#provenance::patched_scripts_require_oracle_dir"
        status: pass
      - kind: integration
        ref: "tests/harness/provenance.rs#provenance::patched_scripts_run_with_oracle_dir"
        status: pass
      - kind: other
        ref: "control D: variable set in the require test -> fails (divergence check exits 0); control E: WSX_ORACLE_DIR=/nonexistent -> fails for all three, phase-worktree.sh only on its marker"
        status: pass
    human_judgment: false
  - id: D5
    description: "tests/oracle/README.md records the commit, every source path, mode and blob, the post-patch blobs, MIT attribution and each patch hunk with its reason, and a test fails when README and constants drift"
    requirement: HARN-01
    verification:
      - kind: unit
        ref: "tests/harness/provenance.rs#provenance::readme_records_the_pinned_commit_and_blobs"
        status: pass
      - kind: unit
        ref: "tests/harness/provenance.rs#provenance::readme_check_reports_a_missing_blob"
        status: pass
      - kind: other
        ref: "control F: the tracked README with one patched blob, then the commit, removed -> fails naming that value"
        status: pass
    human_judgment: false
  - id: D6
    description: "The README is readable by someone who does not know git internals and describes each hunk accurately"
    requirement: HARN-01
    verification: []
    human_judgment: true
    rationale: "Prose quality and plain-words clarity are judgment; the drift test pins only the hashes and the commit, not the hunk descriptions"

duration: 7min
completed: 2026-10-26
status: complete
---

# Phase 2 Plan 04: Oracle provenance Summary

**Eight `provenance::` tests pin `tests/oracle` to devflow ee6ec792. They check each of the six vendored files against its recorded blob and check the patch against three recorded post-patch blobs. They show that a missing git ceiling makes the patch a silent no-op that `OracleCopy::prepare` refuses, and that `WSX_ORACLE_DIR` is both required and sufficient for each patched script. A new `tests/oracle/README.md` carries the provenance and attribution, and a drift test keeps it equal to the constants.**

## Performance

- **Duration:** about 7 min
- **Started:** 2026-10-26T17:14:22Z
- **Completed:** 2026-10-26T17:20:58Z
- **Tasks:** 2 (3 commits: RED, GREEN, docs)
- **Files modified:** 2 (1 created, 1 modified)

## Accomplishments

- `blob_mismatches` hashes each listed file with `git hash-object --no-filters` in the hermetic env. `tests/oracle` reports nothing. A copy with byte 0 of `cut-pr-branch.sh` flipped reports exactly `cut-pr-branch.sh`.
- `PATCHED_BLOBS` comes from a scratch `git apply -v` under the ceiling. Its three values (`21465e9…`, `5553649…`, `093b279…`) also match the post-image abbreviations on the patch's own `index` lines, so two independent sources agree.
- `oracle_patch_applies_cleanly` goes through the harness's own `OracleCopy::prepare`. It checks the patched blobs and that `sync-workspace.sh`, `cut-pr-branch.sh` and `LICENSE` are unchanged. It then requires a second apply to fail with `patch does not apply`.
- `patch_apply_without_ceiling_is_a_silent_no_op` sets up a repository at the scenario root and removes only `GIT_CEILING_DIRECTORIES`. The apply then exits 0 and prints `Skipped patch '<file>'.` for each of the three files, every blob stays at devflow's, and `prepare` fails with `lacks WSX_ORACLE_DIR`.
- Each patched script runs in a fresh devflow fixture from `work`: the divergence check with `origin/develop`, `phase-worktree.sh 1`, and `hooks/pre-push origin <url>` with the workspace branch's push line on stdin. Without the variable, each fails with `WSX_ORACLE_DIR: parameter null or not set`. With it, through `run_script`, each exits 0 and prints its marker (`every difference from origin/develop is declared.` or `==> syncing workspace/dev`).
- `tests/oracle/README.md` covers the source, a table for each blob set, attribution, one row per patch hunk with its reason, the rules and the re-vendoring steps. `readme_records_the_pinned_commit_and_blobs` and its in-memory counterpart tie it to the constants.

## Task Commits

1. **Task 1 RED: failing flipped-byte control**: `1ec67d0` (test)
2. **Task 1 GREEN: blob checker, PATCHED_BLOBS and the patch tests**: `5542d40` (feat)
3. **Task 2: provenance README and its drift test**: `fd18911` (docs)

## TDD Gate Compliance

- **RED** `1ec67d0`: `scripts/check.sh test -E 'test(=provenance::flipped_byte_copy_is_reported)'` exited 100. The JUnit report `target/nextest/default/junit.xml` was written at 2026-10-26T17:16:04.624Z, after the run started at 17:16:03.970Z. The record is at `target/gsd/02-04-red-evidence.json` (gitignored): target `provenance::flipped_byte_copy_is_reported`, file `tests/harness/provenance.rs`. Classifier verdict: **`RED_EVIDENCE_OK`** (`target_test_failed`, matched `wsx::harness#provenance::flipped_byte_copy_is_reported`, 1 test, 1 fail).
  - **Semantic assessment:** the test compiled without warnings, copied the six files and the patch, and flipped the byte. It then failed on its own assertion (`provenance.rs:122`, left `[]`, right `["cut-pr-branch.sh"]`), because the stub checker reported nothing. It was not a compile error, a copy panic or zero-test discovery: 1 test ran and 64 were skipped. The run was redone after `cargo fmt`, so the evidence matches the committed bytes.
- **GREEN** `5542d40`: all six Task 1 tests pass. `scripts/check.sh all` exited 0 (68 passed, 2 skipped) before the commit.
- **REFACTOR:** none.
- `git log --reverse "$(git merge-base HEAD workspace/denniyahh)..HEAD"` lists `test(02-04)` before `feat(02-04)`.

## Verification evidence and negative controls

All eight tests were run at final HEAD `fd18911`, one `scripts/check.sh test -E 'test(=provenance::<name>)'` per test, and each gave rc 0. A misnamed filter gives rc 4. `scripts/check.sh all` gives rc 0: 70 passed, 2 skipped, `==> check.sh: all OK`.

Each control below was a temporary mutation. I checked that it applied (the helper asserts that the target string occurs exactly once), then restored it and confirmed the restore with `cmp`.

| Control | Mutation | Observed |
|---|---|---|
| A | last hex digit of the `sync-workspace.sh` entry in `VENDORED_BLOBS` | `vendored_files_match_devflow_blobs` fails: `sync-workspace.sh: blob 82ed…85, recorded 82ed…86` |
| B | last hex digit of the `hooks/pre-push` entry in `PATCHED_BLOBS` | `oracle_patch_applies_cleanly` fails, naming `hooks/pre-push` |
| C | no-ceiling test keeps the ceiling | fails: stderr shows `Applied patch check-workspace-divergence.sh cleanly.` instead of `Skipped patch` |
| D | require test runs with the variable set | fails: `check-workspace-divergence.sh: exit Some(0)` |
| E | the with-variable test runs with `WSX_ORACLE_DIR=/nonexistent` | fails for all three: divergence exits 2 (`sed: can't read /nonexistent/cut-pr-branch.sh`), pre-push exits 127. **`phase-worktree.sh` exits 0** and fails only on its missing `==> syncing` marker |
| F | tracked README without the `phase-worktree.sh` patched blob, then without the commit | `readme_records_the_pinned_commit_and_blobs` fails, naming exactly that value each time |
| RED | stub `blob_mismatches` | flipped-byte test fails on its assertion (above) |

Acceptance criteria: the commit SHA is in `provenance.rs` (rc 0; rc 1 at `64d7844`). `PATCHED_BLOBS` holds the three paths. `git log 64d7844..HEAD -- tests/oracle` lists only `fd18911`, which adds `README.md`. No vendored file, `LICENSE` or `oracle.patch` changed. `git status --porcelain tests/oracle` is empty. The README contains the SHA and `MIT`, mentions `WSX_ORACLE_DIR` on 6 lines, and names all five scripts and `LICENSE`.

## What these checks do not establish

- **The blob check proves content, not mode.** Modes are pinned only by 02-01's `git ls-files -s` acceptance check. No provenance test checks modes directly. A lost executable bit *would* change oracle runs: `copy_tree` keeps permission bits, `phase-worktree.sh` tests `-x` on the copy's `sync-workspace.sh`, and pre-push executes the copy's divergence check directly. `patched_scripts_run_with_oracle_dir` would catch those two cases, through the `==> syncing` marker and pre-push's exit code. It would not catch a lost bit on `cut-pr-branch.sh`, which the adapter runs as `bash <script>`.
- **The oracle-dir tests prove the three patched hunks are reached on these fixtures,** with one argument set each. They say nothing about other paths through the scripts. `phase-worktree.sh`'s line-54 test and line-61 call are both exercised (the run syncs). The pre-push tail replacement is shown only to exit 0. No test asserts that the container gate is absent beyond that exit.
- **`readme_missing_records` checks that each recorded value appears somewhere in the README,** not that it sits in the right table row. Swapping two blobs between rows would pass. The hunk descriptions are prose that no test checks (coverage D6).
- **`PATCHED_BLOBS` checks the result of one patch application.** A change to `oracle.patch` that yields the same three files would pass. That is the intended contract.
- **The container gate was not run.** Host git 2.56.0 is the only git measured, including the `Skipped patch` wording and bash's `parameter null or not set` message, which the tests match literally.

## Files Created/Modified

- `tests/harness/provenance.rs`: constants, `vendored_pairs`, `unpatched_pairs`, `blob_mismatches`, `hash_blob`, `scratch_copy`, `reported_paths`, `reapply_fails`, `script_inputs`, `run_patched`, `describe`, `readme_missing_records`, `oracle_readme`, and the eight tests
- `tests/oracle/README.md`: provenance, attribution and patch documentation

## Decisions Made

See `key-decisions` in the frontmatter.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `run_script` cannot run a script without `WSX_ORACLE_DIR`**
- **Found during:** Task 1 GREEN
- **Issue:** the plan has both oracle-dir tests run through `oracle::run_script` and leave the variable out "by building the child's env without it". But the as-built `run_script` always adds `.env("WSX_ORACLE_DIR", &copy.dir)` after the env, so no env passed in can remove it. The plan also says not to change `oracle.rs`.
- **Fix:** `run_patched(.., false)` builds the same command line that `run_script` builds: `bash <copy>/<script> <args>`, the same cwd, env and piped stdin/stdout/stderr, through `env::command` and the public `oracle::spawn_and_wait`, minus the variable. `run_patched(.., true)` calls `run_script` itself. Both tests also assert that the hermetic env never carries `WSX_ORACLE_DIR`.
- **Files modified:** `tests/harness/provenance.rs`
- **Verification:** controls D and E.
- **Commit:** `5542d40`

**2. [Rule 2 - Correctness] Exit 0 alone does not prove `phase-worktree.sh`'s hunk worked**
- **Found during:** Task 1, control E
- **Issue:** with a wrong `WSX_ORACLE_DIR`, `phase-worktree.sh` prints `scripts/sync-workspace.sh not found; skipping sync`, creates the worktree and exits 0.
- **Fix:** the with-variable test also requires a stdout marker per script.
- **Commit:** `5542d40`

**3. [Scope - signature] `blob_mismatches(dir, table, dirs, env)`**: the plan's `(dir, table)` gained the hermetic env and the scenario dirs. `git hash-object` runs in the scenario HOME, a proper subdirectory of the ceiling. If it ran in `tests/oracle`, it could discover the wsx checkout.

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 correctness) and one signature widening.
**Impact on plan:** none on intent. Every planned test exists under its planned name.

## Issues Encountered

- None blocking. My first control harness had a shell syntax error and ran nothing. I rewrote it as a script and checked it with `bash -n` before running any control.

## Known Stubs

None. `.planning/WINDOWS.md` does not exist, so nothing was appended to the ledger.

## Threat Flags

None. The plan's threat register covers T-02-06, T-02-07 and T-02-08: the flipped-byte and blob controls, the post-patch blobs and no-ceiling control, and the README drift test. No package was installed and `Cargo.lock` is unchanged (T-02-SC).

## User Setup Required

None.

## Next Phase Readiness

- HARN-01's provenance claims (recorded commit, byte-identical files, one patch, attribution) now each have a test and a control.
- 02-15's scratch audit: these tests create `harness-provenance-*` dirs under `target/tmp`. After all runs and controls, none were left.

## Self-Check: PASSED

- FOUND: `tests/harness/provenance.rs`, `tests/oracle/README.md`
- FOUND: commits `1ec67d0`, `5542d40`, `fd18911` (each an ancestor of HEAD), none with file deletions
- `git rev-list --count 64d7844..HEAD` = 3
- No `harness-*` directory left under `target/tmp`

---
*Phase: 02-oracle-harness*
*Completed: 2026-10-26*
