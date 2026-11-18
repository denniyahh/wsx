---
phase: 04-repo-model
plan: 18
subsystem: git
tags: [rust, git-binary, cat-file, cargo-mutants, llvm-cov, latency]

requires:
  - phase: 04-repo-model
    provides: "04-13 to 04-17: the model's inputs read through the git binary (config, for-each-ref, cat-file --batch)"
provides:
  - "D-25 inventory checked against the tree (deleted items gone, kept items defined, control at 4234f31)"
  - "STATE.md read-order, exit-code and grafts-and-shallow obligation lines and the pivot's decision line"
  - "D-26 latency measured on the shipped resolve: p95 10.8 to 14.0 ms on the four R3 repos, four children each"
  - "parse_batch reads git's '<oid> submodule' answer, so a .wsx.conf gitlink whose commit is absent reads as a submodule"
  - "fresh full mutation pass: 265 listed = 237 caught + 28 unviable, 0 missed, 0 timeout"
affects: [phase-5, phase-6, merge_base, ahead_behind, range_walk, tree_diff]

actuals:
  tokens: 4727
  tasks: 3
  commits: 4
plan_head_before: c55138471a0dbe1d9f459615276bd9c3fa8ce0cd
plan_head_after: 32ab91782ab45cd685e44aba1410b193e48fa846

tech-stack:
  added: []
  patterns:
    - "A batch answer git prints without bytes (missing, submodule) is its own BatchAnswer variant"

key-files:
  created: []
  modified:
    - .planning/STATE.md
    - src/git/objects.rs
    - src/git/config.rs
    - src/git/spawn.rs

key-decisions:
  - "04-18: git cat-file --batch answers a <tree-ish>:<path> gitlink whose commit is absent with '<oid> submodule' (git 2.56.1 host, 2.56.0 image); parse_batch reads it as BatchAnswer::Submodule and manifest_entry as RootEntry::NotABlob submodule"
  - "04-18: D-26 holds on one host: resolve p95 10.8 to 14.0 ms over 200 calls in each R3 repo, four git children each"

patterns-established:
  - "Mutation survivors are closed at the parser's own surface, one scenario per test, asserting the refusal text"

requirements-completed: [CONF-01, CONF-02, CONF-03, CONF-04, CONF-05, CONF-07]

coverage:
  - id: D1
    description: "The tree matches the D-25 inventory; 4234f31 is the control that each deleted item existed"
    verification:
      - kind: other
        ref: "Task 1 first <automated> block (exit 0); same block on 4234f31's src exits 1 naming all nine deleted items"
        status: pass
    human_judgment: false
  - id: D2
    description: "STATE.md carries the rewritten read-order line, the exit-code line, the grafts-and-shallow obligation and the pivot decision, all through state verbs"
    verification:
      - kind: other
        ref: "Task 1 second <automated> block (exit 0); same block on HEAD~ STATE.md exits 1 on all six conditions"
        status: pass
    human_judgment: false
  - id: D3
    description: "D-26 latency of the shipped resolve on the four R3 repos, four-child path, p95 under 100 ms"
    verification:
      - kind: other
        ref: "Task 2 <automated> block (exit 0); target/gsd/04-18-latency.txt"
        status: pass
    human_judgment: false
  - id: D4
    description: "A .wsx.conf gitlink whose commit is absent reads as a submodule instead of failing the model with GitBadOutput"
    requirement: CONF-04
    verification:
      - kind: unit
        ref: "src/git/objects.rs#git::objects::tests::gitlink_whose_commit_is_absent_is_not_a_blob"
        status: pass
      - kind: unit
        ref: "src/git/objects.rs#git::objects::tests::parse_batch_reads_a_submodule_answer"
        status: pass
    human_judgment: false
  - id: D5
    description: "Fresh full mutation pass over the six changed files with no unexplained survivor, tested = listed"
    verification:
      - kind: other
        ref: "Task 3 blocks 1 to 3 on 32ab917 (exit 0); doctored-copy controls exit 1"
        status: pass
    human_judgment: false
  - id: D6
    description: "Coverage, deps and container gates green on the final tip"
    verification:
      - kind: other
        ref: "scripts/check.sh coverage, scripts/check.sh deps, scripts/check-in-container.sh all (each exit 0)"
        status: pass
    human_judgment: false

duration: 1h 34m
completed: 2026-11-18
status: complete
---

# Phase 4 Plan 18: Replan close-out Summary

**D-25 inventory and STATE obligations brought in line with the git-binary reads, D-26 measured at p95 10.8 to 14.0 ms, and a fresh mutation pass that found and fixed git's `<oid> submodule` batch answer breaking a submodule manifest**

## Performance

- **Duration:** 1h 34m (dominated by two 19 and 34 min mutation passes and the gate runs)
- **Started:** 2026-11-17T23:37:52Z
- **Completed:** 2026-11-18T01:12Z
- **Tasks:** 3
- **Files modified:** 4

## Accomplishments

- Task 1: the inventory block exited 0; on 4234f31's `src` it exits 1 naming all nine deleted items, so it bites. `rg -qw ConfigFileEnv src` exited 1 (04-17 chose drop), so no `ConfigFileEnv` clause was inserted, and `src/git/repo.rs` stayed off the files list. STATE.md changed only through `state.resolve-blocker` (both `resolved: true`), `state.add-blocker` (three) and `state.add-decision`; the diff held those lines plus GSD's `last_updated` and `state_head`. The STATE block exited 0 after the commit and exits 1 on the pre-change STATE.md on all six conditions.
- Task 2: release build of the committed tip in a scratch copy, 200 calls each, one fresh `Git::from_process` per call, load average 4.20 at start and 6.22 at end:

| Repo | children | p50 | p95 | spike 001 batched (bash) p50 / p95 |
|------|----------|-----|-----|------|
| devflow | 4 | 10.9 ms | 13.0 ms | 20.0 / 40.7 ms |
| wsx | 4 | 10.0 ms | 10.8 ms | 15.2 / 26.7 ms |
| gsd-core (`personal/workspace-backup` recorded) | 4 | 12.5 ms | 14.0 ms | 22.0 / 38.8 ms |
| gsd-pi | 4 | 10.4 ms | 13.0 ms | 15.4 / 23.0 ms |

  Every line had `children=4` on the first run, so no recorded set changed. The gate's tail exits 1 on a sample with one `children=3` line. No `latency.*` directory was left.
- Task 3: final pass on `32ab917` (`target/gsd/04-18-pass-head.txt`): 237 caught, 0 missed, 28 unviable, 0 timeout, sum 265, equal to the `cargo mutants --list` total of 265. The allowlist `target/gsd/04-18-equivalent-mutants.txt` is empty: no mutant was called equivalent.

## Task Commits

1. **Task 1: inventory and STATE obligations** - `e0fb840` (docs)
2. **Task 2: latency** - no commit (probe and record under `target/gsd/`, gitignored)
3. **Task 3: survivors** - `e0442a2` (test), `7708ebf` (test, RED), `32ab917` (fix, GREEN)

## Mutation pass

First pass (`target/gsd/04-18-first-missed.txt`, 19 min): 265 mutants, 228 caught, 9 missed, 28 unviable, 0 timeout. Each survivor and its test:

| Survivor | Test |
|----------|------|
| `src/git/mod.rs:70:22` `&&` to `\|\|` in `ObjectId::from_hex` | `parse_batch_refuses_an_abbreviated_id` |
| `src/git/config.rs:79:19` guard `stderr.is_empty()` to true in `read_config` | `config_read_exit_1_with_an_error_is_a_failure` |
| `src/git/objects.rs:90:45` `&&` to `\|\|` in `parse_batch` (size) | `parse_batch_refuses_a_signed_size` |
| `src/git/objects.rs:93:30` `+` to `*` in `parse_batch` | `parse_batch_refuses_data_without_its_lf` |
| `src/git/objects.rs:154:48` guard `kind == b"commit"` to true in `peeled_commit` | `commit_peel_answered_with_another_type_is_bad_output` |
| `src/git/objects.rs:183:38` guard `kind == b"blob"` to true in `manifest_entry` | `file_mode_on_an_object_that_is_not_a_blob_is_bad_output` |
| `src/git/objects.rs:198:58` guard `kind == b"tree"` to true in `manifest_entry` | `missing_entry_beside_a_root_that_is_not_a_tree_is_bad_output` |
| `src/git/objects.rs:201:35` guard `mode == GITLINK_MODE` to false in `manifest_entry` | `gitlink_found_only_in_the_root_tree_is_not_a_blob` (unit), and the real-git `gitlink_whose_commit_is_absent_is_not_a_blob`, which exposed the defect below |
| `src/git/spawn.rs:40:24` `\|\|` to `&&` in `version_part` | `git_version_with_a_signed_part_is_refused` |

Final pass (34 min, under load average about 17 from its own two jobs): 0 missed, 0 timeout, so block 2 exited 0 with an empty allowlist, and block 3 exited 0 (tested = listed, no `previously_caught.txt`, source unchanged since the pass head).

Doctored copies of `mutants.out`: last `caught.txt` line deleted, block 3 exit 1, `tested mutants differ from the listed ones`; last line replaced with `src/git/refs.rs:1:1: replace nothing with nothing`, block 3 exit 1, same message; that line appended to `timeout.txt`, block 2 exit 1, `unexplained survivors above`. `target/gsd/04-18-checks-control` removed.

## TDD Evidence (the defect fix)

- **RED** (`7708ebf`): `git::objects::tests::gitlink_whose_commit_is_absent_is_not_a_blob`, run alone, exit 100. `target/gsd/04-18-red-evidence.json` (JUnit from `target/nextest/default/junit.xml`); `gsd_run check tdd-red-evidence ... --raw --pick verdict` printed `RED_EVIDENCE_OK`; the same record with the failure removed and exit 0 printed `INVALID_RED`. Semantic assessment: the target ran after both git-side premise checks passed (HEAD holds a 160000 entry; the commit is absent) and failed because `read_tips` returned `GitBadOutput` (`answer 2: a header that is not four fields`): git answered `<oid> submodule`. Not a build, load or fixture failure.
- **GREEN** (`32ab917`): the RED test and three unit tests for the new variant pass alone; `scripts/check.sh all` 696 passed, 63 skipped, `==> check.sh: all OK`.
- No REFACTOR commit.

## Coverage and gates (on `32ab917`)

Line coverage (`target/gsd/04-18-coverage.log`; branch coverage waived as in 04-10): `git/config.rs` 91.20%, `git/mod.rs` 99.04%, `git/objects.rs` 94.08%, `git/refs.rs` 93.51%, `git/spawn.rs` 93.07%, `model/mod.rs` 96.62%; TOTAL 93.18%. Line coverage shows a line ran, not that a test checks it.

- `scripts/check.sh coverage`: exit 0
- `scripts/check.sh deps`: exit 0 (`advisories ok, bans ok, licenses ok, sources ok`)
- `scripts/check-in-container.sh all`: exit 0, `Summary [  45.896s] 696 tests run: 696 passed, 63 skipped`, last line `==> check.sh: all OK` (`target/gsd/04-18-container.log`); image `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie` git 2.56.0, so the submodule test passes on the image's git too.

Final tip for the orchestrator's Tier A diff review (D-28): `32ab91782ab45cd685e44aba1410b193e48fa846` (code); this SUMMARY's docs commit follows it.

## Files Created/Modified

- `.planning/STATE.md` - two stale obligation lines replaced, the grafts-and-shallow obligation and the pivot decision added
- `src/git/objects.rs` - `BatchAnswer::Submodule` and its handling; survivor and fix tests
- `src/git/config.rs` - test: exit 1 with stderr fails the config read
- `src/git/spawn.rs` - test: a signed version part is refused

Kept under `target/gsd/` (gitignored): `04-18-latency-probe.rs`, `04-18-latency.txt`, `04-18-files.txt`, `04-18-mutants/mutants.out` (final) and `04-18-mutants/mutants.out.old` (first pass), `04-18-pass-head.txt`, `04-18-equivalent-mutants.txt`, `04-18-first-missed.txt`, `04-18-first-timeout.txt`, `04-18-listed.txt`, `04-18-tested.txt`, `04-18-coverage.log`, `04-18-container.log`, `04-18-red.log`, `04-18-red-evidence.json`, `04-18-m1.log`, `04-18-m2.log`. `04-18-mutants-tmp`, `04-18-checks-control` and `target/llvm-cov-target` are gone.

## Decisions Made

- The submodule answer gets its own `BatchAnswer` variant rather than being folded into `Missing`, so `manifest_entry` reads it directly and a `^{commit}` or `^{tree}` request answered so stays `GitBadOutput`. The tree-scan fallback for a missing entry beside a gitlink is kept for a git that answers `missing`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A `.wsx.conf` that is a real submodule failed the model**
- **Found during:** Task 3 (closing survivor `objects.rs:201:35` with a real-git test)
- **Issue:** git 2.56 answers `<tree-ish>:<path>` for a gitlink whose commit the repository lacks (the usual case for a submodule) with `<oid> submodule` and no bytes. `parse_batch` refused that header, so `resolve` failed with `ManifestUnreadable(GitBadOutput)` instead of reading the manifest as a submodule. The 04-16 fallback assumed git answers `missing`.
- **Fix:** `BatchAnswer::Submodule` for `<full hex id> submodule`; `manifest_entry` maps it to `RootEntry::NotABlob { kind: "submodule" }`; `peeled_commit` and the tree answer treat it as `GitBadOutput`.
- **Files modified:** src/git/objects.rs
- **Verification:** RED evidence above; GREEN on host and container; the final mutation pass has no survivor.
- **Committed in:** `7708ebf` (RED), `32ab917` (GREEN)

**2. [Rule 3 - Blocking] `WSX_SCRATCH` was not exported**
- **Found during:** Task 2 precondition
- **Issue:** the precondition needs `WSX_SCRATCH`; the shell had none.
- **Fix:** exported it in the same call to a fresh directory inside the session scratchpad, the convention every earlier phase-3 and phase-4 plan states.
- **Committed in:** none (environment only)

---

**Total deviations:** 2 auto-fixed (1 bug, 1 blocking). **Impact on plan:** the fix changes production behaviour for a submodule manifest only (an error becomes the submodule answer the model already handles); no scope beyond that.

## Issues Encountered

None beyond the deviations.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 4 plans are complete. The orchestrator's Tier A external diff review (D-28) of the final tip comes next, before phase verification.

## Self-Check: PASSED

- All 17 kept `target/gsd/` artifacts listed above checked present with `test -f`; the three removed directories checked absent.
- Commits `e0fb840`, `e0442a2`, `7708ebf`, `32ab917` are ancestors of HEAD; `git rev-list --count c551384..HEAD` = 4.

---
*Phase: 04-repo-model*
*Completed: 2026-11-18*
