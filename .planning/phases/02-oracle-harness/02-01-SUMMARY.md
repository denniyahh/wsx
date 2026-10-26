---
phase: 02-oracle-harness
plan: 01
subsystem: testing
tags: [oracle, vendoring, git-apply, tempfile, cargo-deny]

requires: []
provides:
  - "tests/oracle/: devflow ee6ec792's five workspace scripts and LICENSE, byte-identical, devflow's modes"
  - "tests/oracle/oracle.patch: the single adaptation (four WSX_ORACLE_DIR redirects, pre-push CI-mirror tail replaced by exit 0)"
  - "tempfile 3.27.0 as a dev-dependency, locked"
  - "deny.toml [licenses] include-dev = true, so the license gate covers dev-dependencies"
affects: [02-02, 02-04, oracle-harness]

actuals:
  tokens: 12061   # chars/4 over `git diff c08a736 69b1c73` (48244 chars)
  tasks: 3
  commits: 3
plan_head_before: c08a736e5327072e9be4da210fec6ef66bd28161
plan_head_after: 69b1c7379009c130adcf90840ea0018c7b3ecbb9

tech-stack:
  added: [tempfile 3.27.0 (dev-only)]
  patterns:
    - "Vendored third-party files stay byte-identical; every adaptation lives in one patch applied to a scratch copy"
    - "git apply on a scratch copy runs under GIT_CEILING_DIRECTORIES=<scratch root>"

key-files:
  created:
    - tests/oracle/sync-workspace.sh
    - tests/oracle/phase-worktree.sh
    - tests/oracle/cut-pr-branch.sh
    - tests/oracle/check-workspace-divergence.sh
    - tests/oracle/hooks/pre-push
    - tests/oracle/LICENSE
    - tests/oracle/oracle.patch
  modified:
    - Cargo.toml
    - Cargo.lock
    - deny.toml

key-decisions:
  - "oracle.patch is a pure diff with no preamble; its documentation belongs to tests/oracle/README.md, which this plan does not create"
  - "Substitutions matched whole lines, not substrings: `scripts/sync-workspace.sh` occurs on four lines of phase-worktree.sh, and only lines 54 and 61 are paths the oracle executes"

patterns-established:
  - "Patch headers are relative to tests/oracle (a/hooks/pre-push), so git apply runs from the scratch copy's root"

requirements-completed: [HARN-01, HARN-02]

coverage:
  - id: D1
    description: "Five devflow scripts and LICENSE vendored byte-identically from ee6ec792, scripts 100755 and LICENSE 100644"
    requirement: HARN-01
    verification:
      - kind: other
        ref: "02-01-PLAN Task 1 verify 1: git hash-object --no-filters of each file against RESEARCH's blob table"
        status: pass
      - kind: other
        ref: "git ls-files -s tests/oracle (index blobs and modes)"
        status: pass
    human_judgment: false
  - id: D2
    description: "oracle.patch applies cleanly to a scratch copy, a second apply fails, and it touches only the three scripts that need it"
    requirement: HARN-01
    verification:
      - kind: other
        ref: "02-01-PLAN Task 1 verify 2: scratch git apply under GIT_CEILING_DIRECTORIES"
        status: pass
    human_judgment: false
  - id: D3
    description: "tempfile 3.27.0 is a justified dev-only dependency, Cargo.lock committed, deps gate clean"
    requirement: HARN-02
    verification:
      - kind: other
        ref: "scripts/check.sh deps"
        status: pass
      - kind: other
        ref: "scripts/check.sh all"
        status: pass
    human_judgment: false
  - id: D4
    description: "include-dev = true puts tempfile's graph under the cargo-deny license check"
    requirement: HARN-02
    verification:
      - kind: other
        ref: "02-01-PLAN Task 3 verify 2: ISC-only scratch copies with and without include-dev"
        status: pass
    human_judgment: false

duration: 3min
completed: 2026-10-26
status: complete
---

# Phase 2 Plan 01: Oracle inputs Summary

**devflow ee6ec792's five workspace scripts and LICENSE vendored byte-identically under `tests/oracle/`, adapted only by a 72-line `oracle.patch` (four `WSX_ORACLE_DIR` redirects, pre-push CI-mirror tail cut), plus tempfile 3.27.0 as a dev-dependency that the cargo-deny license gate now actually checks.**

## Performance

- **Duration:** about 3 min
- **Started:** 2026-10-26T16:36:58Z
- **Completed:** 2026-10-26T16:39:53Z
- **Tasks:** 3
- **Files modified:** 10 (7 created, 3 modified)

## Accomplishments

- `tests/oracle/` holds devflow's `sync-workspace.sh`, `phase-worktree.sh`, `cut-pr-branch.sh`, `check-workspace-divergence.sh`, `hooks/pre-push` and `LICENSE`. Each `git hash-object --no-filters` equals RESEARCH's blob table. Devflow's own `git ls-tree` at ee6ec792 gives the same six blobs, and so does wsx's index after staging, so three independent sources agree. Modes are 100755 for the scripts and 100644 for LICENSE.
- `tests/oracle/oracle.patch` (72 lines, the same length as RESEARCH's verified candidate) changes `check-workspace-divergence.sh` line 30, `phase-worktree.sh` lines 54 and 61, and `hooks/pre-push` line 209. It replaces pre-push lines 233-250 with a two-line comment and `exit 0`. It leaves `origin/develop`, `origin` and `cut-pr-branch.sh`'s `FORBIDDEN_PATHS` line alone.
- `Cargo.toml` has a `[dev-dependencies]` table with `tempfile = "3.27.0"` and its justification. `[dependencies]` is still empty (checked with `tomllib`). Cargo resolved exactly the 13 crates RESEARCH listed.
- `deny.toml` `[licenses]` gains `include-dev = true`, with a comment explaining why.

## Task Commits

1. **Task 1: Vendor devflow's scripts and LICENSE, build oracle.patch**: `2cfc2f2` (chore)
2. **Task 2: tempfile as a dev-dependency**: `c9b7edf` (chore)
3. **Task 3: License-check dev-dependencies**: `69b1c73` (chore)

## Verification evidence

**Task 1**
- Blob check (plan verify 1): rc 0. Negative control: the same check on a scratch copy with one byte appended to `hooks/pre-push` printed `blob mismatch: hooks/pre-push`, rc 1.
- Scratch apply (plan verify 2, run verbatim): rc 0. `git apply -v` printed `Applied patch ... cleanly` for all three files.
- Extra control: the patched scratch tree is byte-identical (`diff -r`) to the output of the exact-match substitution script. So the patch makes the intended edits and nothing else.
- Pitfall 1 control: the same apply inside a nested `git init` scratch repo, without `GIT_CEILING_DIRECTORIES`, printed `Skipped patch 'check-workspace-divergence.sh'.` (and the same for the other two), **exited 0**, and left `hooks/pre-push` unpatched. The ceiling is therefore load-bearing, and an exit code alone is no proof that the patch applied.
- Acceptance criteria: `rg -c WSX_ORACLE_DIR oracle.patch` gives 4. The frozen paths (`bin/wsx`, `tests/sandbox.sh`, `scripts/workspace`) have an empty diff against `merge-base HEAD workspace/denniyahh` (536fb37).

**Task 2**
- `scripts/check.sh deps` gave `advisories ok, bans ok, licenses ok, sources ok`, rc 0. `scripts/check.sh all` gave 49/49 tests passed and `==> check.sh: all OK`.
- Baseline: `rg -qF 'tempfile = "3.27.0"'` on `HEAD~` Cargo.toml exits 1. `Cargo.lock` appears in the diff from the merge-base.

**Task 3**: control output, quoted. Both copies use `allow = ["ISC"]`.
- Copy A (with `include-dev = true`) exited rc 4 with 14 `error[rejected]` diagnostics: wsx itself plus all 13 registry crates (bitflags, cfg-if, errno, fastrand, getrandom, libc, linux-raw-sys, once_cell, r-efi, rustix, tempfile, windows-link, windows-sys). The tempfile rejection:
  ```
  error[rejected]: failed to satisfy license requirements
     ┌─ /home/denniyahh/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/tempfile-3.27.0/Cargo.toml:46:12
     │
  46 │ license = "MIT OR Apache-2.0"
  ```
- Copy B (the same, without `include-dev`) exited rc 4 with exactly one `error[rejected]`. It was for wsx's own `license = "MIT"` (`Cargo.toml:5:12`). The output never mentions `tempfile`.
- Real tree with `allow = ["MIT"]` and `include-dev = true`: `scripts/check.sh deps` gave `licenses ok`, rc 0. `scripts/check.sh all` gave `==> check.sh: all OK`.
- `rg -qx 'include-dev = true' deny.toml` exits 0, and on `HEAD~` it exits 1.

## What these checks do not establish

- The scratch apply only shows that the patch applies to these exact bytes and makes the intended text edits. It does **not** show that the patched scripts run. Plan 02-02 proves that for sync, and 02-04 for the other three.
- The blob check is a one-time run in this session. No test enforces it yet: 02-04 adds the recorded-hash test and its flipped-byte control. Until then, an edit to a vendored file would go unnoticed by the gate.
- `include-dev` brings dev-dependencies under the **license** check, as the control shows. Nothing here tests whether advisories or bans cover dev-dependencies. The deps gate passed, but the control only exercised `check licenses`.
- tempfile is not used yet, so `check.sh all` passing says nothing about how it behaves under `CARGO_TARGET_TMPDIR`. That starts in 02-02.
- The cargo-deny version came from the host (`/home/linuxbrew/.linuxbrew/bin/cargo-deny`, `0.20.2`). The container gate was not run for this plan.

## Files Created/Modified

- `tests/oracle/{sync-workspace.sh, phase-worktree.sh, cut-pr-branch.sh, check-workspace-divergence.sh, hooks/pre-push}`: devflow's oracle scripts, byte-identical
- `tests/oracle/LICENSE`: devflow's MIT/Apache-2.0 license, byte-identical (attribution)
- `tests/oracle/oracle.patch`: the single adaptation of the vendored oracle
- `Cargo.toml`, `Cargo.lock`: tempfile dev-dependency and its locked graph
- `deny.toml`: `include-dev = true`

## Decisions Made

- The patch is a pure diff. Its explanation belongs in `tests/oracle/README.md`, which CONTEXT names but this plan's file list does not include.
- Substitutions were matched against whole lines, each asserted to occur exactly once. RESEARCH's table shows line 61 indented by 8 spaces, but the actual file has 12. Matching on the real bytes made that difference harmless.

## Deviations from Plan

None. The plan was executed as written.

## Issues Encountered

None.

## Known Stubs

None.

## User Setup Required

None.

## Next Phase Readiness

- 02-02 (the tracer) can apply `oracle.patch` to a scratch copy and run `sync-workspace.sh` under `WSX_ORACLE_DIR`. tempfile is available as a dev-dependency.
- `tests/oracle/README.md` (pin SHA, blob table, MIT attribution, patch rationale) is still to be written. CONTEXT requires it, and this plan's `files_modified` did not include it.

## Self-Check: PASSED

- FOUND: all 7 created files and 3 modified files (`[ -f ]`)
- FOUND: commits 2cfc2f2, c9b7edf, 69b1c73 (each `git merge-base --is-ancestor` of HEAD)
- No file deletions in any of the three commits
- The blob check, re-run at HEAD after all three commits, is still clean (`blobs_ok`)
- No `patch.*` or `deny.*` directory is left under the scratch directory

---
*Phase: 02-oracle-harness*
*Completed: 2026-10-26*
