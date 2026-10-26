---
phase: 02-oracle-harness
reviewed: 2026-10-26T21:40:00Z
depth: deep
iteration: 3
files_reviewed: 20
files_reviewed_list:
  - CLAUDE.md
  - CONTRIBUTING.md
  - rust-toolchain.toml
  - scripts/check.sh
  - scripts/install-tools.sh
  - tests/gate_controls.rs
  - tests/harness/check.rs
  - tests/harness/coverage.rs
  - tests/harness/cut.rs
  - tests/harness/env.rs
  - tests/harness/guard.rs
  - tests/harness/main.rs
  - tests/harness/oracle.rs
  - tests/harness/outcome.rs
  - tests/harness/registry.rs
  - tests/harness/scenario.rs
  - tests/harness/signing.rs
  - tests/harness/sync.rs
  - tests/harness/worktree.rs
  - tests/oracle/README.md
findings:
  critical: 0
  warning: 0
  info: 9
  total: 9
status: issues_found
---

# Phase 2: Code Review Report (re-review, iteration 3, final)

**Reviewed:** 2026-10-26
**Depth:** deep
**Files Reviewed:** 20
**Status:** issues_found (Info only; no Critical or Warning remains)

## Narrative Findings (AI reviewer)

## Summary

This pass re-checked the commits after the iteration-2 review (`5926275`..`119fe46`): `a0956e6`,
`6bd5229`, `07c67ee`, the owner-decision commits `17f6abc` and `e245be0`, and `119fe46`. Since
`5926275`, these source files changed: `cut.rs`, `env.rs`, `outcome.rs`, `registry.rs`,
`scenario.rs`, `signing.rs`, `sync.rs`, `worktree.rs` and `tests/oracle/README.md`.

Both iteration-2 Warnings are fixed. The no-upstream reclassification is consistent. No new
Critical or Warning defect was found. Two new Info items are recorded (IN-09 and IN-10). IN-07 is
resolved. The other Info items carry forward because they are still true.

**Verification run in this pass:** `scripts/check.sh test` at HEAD `119fe46`, on the host,
exited 0: 186 tests run, 186 passed, 41 skipped (the disabled Rust sides). The IN-10 probe was
run against a scratch repository, with a negative control: `git ls-tree -r -z --full-tree HEAD`
exits 128 on an unborn branch and 0 once a commit exists, while `ls-files -s -z` and
`worktree list --porcelain -z` both succeed on the unborn branch.

**What this does not establish:**
- Every Rust side is still disabled. The new comparisons and the Rust reason rule have only run
  against oracle runs and synthetic cases.
- `scripts/check-in-container.sh` was not run, so the cut records were not checked under the CI
  image's git. `119fe46` reduces that risk: it pins `branch.autoSetupMerge = true` in the
  harness's global config (`env.rs:36-46`). The system config is an empty file, so only that
  global config and git's built-in default could set the key.
- `fmt`, `clippy`, `coverage` and `deps` were not rerun.
- `119fe46` names a control in its message ("with the pin set to false, cut::self_test::oracle
  fails"), but no test for it was committed. Removing the pin changes nothing on any git whose
  built-in default is `true`, so no test can tell the pin is there. The pin is harmless but not
  tested, so it is not raised as a finding.

### Iteration-2 findings: status

| ID | Status | Evidence |
|----|--------|----------|
| WR-01 (`Exactly` forms blind to checkout state, remotes and config) | Fixed | `outcome.rs:984-1046` (`compare_listed_checkouts`): the checkout set, each listed checkout's HEAD branch, index equal to HEAD's tree (`read_head_tree`, `outcome.rs:650-674`) and no pending operation, for every checkout. `outcome.rs:897-916`: remote refs are compared under both `RefsExpect` forms, and the config is compared against `before + config_added` (`compare_config`, `outcome.rs:949-980`). Every sync and new-worktree success expectation now lists its checkouts (`sync.rs:41-43`, `123-129`, `244-250`; `worktree.rs:97-104`, `164-183`, `396-399`, `524-527`). There are 8 negative controls (`outcome.rs` `files_exactly_*` and `refs_exactly_*`), and each asserts one exact field list. |
| WR-02 (Rust-side refusals pin only the exit code) | Fixed | `scenario.rs:74-77` (`rust_stderr`), `scenario.rs:457-461` (`rust_reason_rule`), `scenario.rs:516-530` (`validate_entry`, applied when `rust_disabled.is_none()`, which is exactly the `rust_enabled` arm of `scenarios!`, `scenario.rs:669-671`), and `scenario.rs:607-619` (`judge` checks `rust_stderr` on `Side::Rust`). The runner (`scenario.rs:773`) and `registry::every_scenario_is_well_formed` (`registry.rs:177`) both validate with it. The controls `rejects_enabled_rust_refusal_without_reason` (no pin, empty pin, and rejection by the runner) and `rust_side_is_judged_on_its_own_stderr` discriminate. |
| IN-07 (stale ceiling and `new_commits` docs) | Resolved | `tests/oracle/README.md:82`, `outcome.rs:48` |

### No-upstream reclassification (PRCUT-06): consistent

- **Expectations.** `cut_outcome` (`cut.rs:244-269`), `self_test` (`cut.rs:71-121`) and
  `fork_layout` (`cut.rs:1197-1240`) use `RefsExpect::exactly`, which leaves the config unchanged.
- **Call-site count, checked two ways.** There are 14 `*upstream_fixed` call sites in `cut.rs`
  (lines 107, 452, 550, 619, 623, 736, 843, 937, 1014, 1113, 1218, 1299, 1312, 1392). With the 2
  inherited `signing::cut_replay_signed` cases, that is 16 newly `fixed:` cases, and the 7
  `tracked(` records make 23. This matches the fixer's count of 23 single-field (config) failures.
- **Defect texts of the upstream-only cases.** Each `upstream_fixed` record is
  `tracked(expected)`, the expectation plus exactly the two `branch.<pr>.*` entries
  (`cut.rs:173-182`). Each one's defect text (`TOPIC_PR_TRACKS`, `PHASE_PR_TRACKS`,
  `FORK_PR_TRACKS`, `cut.rs:157-166`) names that upstream and nothing else. The branch and remote
  in each text match the `pr_ref` and `remote` passed. For example, `fork_layout` passes
  `upstream`, and its text says `upstream/develop`.
- **The two refused counterparts.** `fork_layout` and `no_gsd_repo` create no PR branch and stay
  Parity.
- **Fixtures.** `phase_with_existing_pr` creates the pre-existing PR branch with `--no-track`
  (`cut.rs:874-882`), so the "before" config holds no tracking entries. The `+2` count in
  `compare_config` is therefore correct.
- **Signing.** `signing::with_signatures` now signs the recorded outcome too, and the recorded
  commit carries the signed message.
- **The other direction is not uniform** (a text that omits part of its record). See IN-09.

## Info

### IN-01: The mode check reads the index, not the files the oracle copy executes (carried forward, still true)

**File:** `tests/harness/oracle.rs:77` (`OracleCopy::prepare`), `tests/harness/provenance.rs:551-567`

**Issue:** `rg '0o111|mode\(\)|EXECUTABLE' tests/harness/oracle.rs` still finds nothing. Take a
checkout with `core.fileMode=false`: it passes the mode test, which reads the index, and the
oracle then skips sync or fails with exit 126.

**Fix:** In `prepare`, assert `mode & 0o111 != 0` on each copied script.

### IN-02: `is_refusal` ignores files (carried forward, still true)

**File:** `tests/harness/scenario.rs:387-389`

**Fix:** Require `outcome.files == FilesExpect::Unchanged`, or document why it is not required.

### IN-03: Pre-push input takes the first `ls-remote` line (carried forward, still true; file outside this iteration's scope list)

**File:** `tests/harness/action.rs:99-104`

**Fix:** Pass `--refs`, and select the line whose second field equals `remote_ref` exactly.

### IN-04: `CommitExpect` omits the committer date (carried forward, still true)

**File:** `tests/harness/outcome.rs:107-118` (struct), `tests/harness/outcome.rs:489` (the
committer is parsed, and its date is discarded)

**Fix:** Add `committer_date` to `CommitExpect`, and compare it.

### IN-05: The nextest override can silently match nothing, and host runs have no timeout (carried forward, still true; file outside this iteration's scope list)

**File:** `.config/nextest.toml:17-18`

**Fix:** Add a test that the override's test name exists, and add
`slow-timeout = { period = "60s", terminate-after = 5 }`.

### IN-06: The coverage tests are vacuous outside a coverage run, and control B is not the decided control (carried forward, still true)

**File:** `tests/harness/coverage.rs:72-117`. The decision text is at `02-CONTEXT.md:121-122`.

**Fix:** Record the substitution as a deviation, or add the decided control (the same line is
uncovered when the binary is not spawned) to the proof procedure.

### IN-08: The index record has no entry flags, and the config record has no per-worktree config (carried forward, still true)

**File:** `tests/harness/outcome.rs:624` (`ls-files -s -z`), `tests/harness/outcome.rs:414`
(`config --local`)

**Issue:**
- **Skip-worktree.** A cut or sync that sets `skip-worktree` or `assume-unchanged` on the user's
  files still passes both `FilesExpect` forms. That includes the new index-equals-HEAD-tree check,
  because `ls-files -s` carries no flags.
- **Per-worktree config.** `config.worktree` entries are not observed.

**Fix:** Record the tag from `ls-files -t` (or `--debug`). When `extensions.worktreeConfig` is
set, read `config --worktree --list -z` per checkout.

### IN-09: Six of the seven older `fixed:` defect texts omit the upstream their records now carry

**File:** `tests/harness/cut.rs:500-501` (`TOPIC_MERGE_DEFECT`), `cut.rs:730`
(`merge_resolution_adds_personal_file`), `cut.rs:805-806` (`CODE_RESOLUTION_DEFECT`),
`cut.rs:931-932` (`existing_pr_branch_with_extra_commits`), `cut.rs:1007-1008`
(`conflict_leaves_partial_pr_branch`), `cut.rs:1107-1108` (`scoped_and_unscoped_commits`)

**Issue:** Each of these records is wrapped in `tracked(...)`, so each also differs from its
expectation in the two `branch.<pr>.*` entries. Their doc comments say so. The `defect` strings
do not:
- `QUOTED_NAMES_DEFECT` (`cut.rs:396-398`) was updated to add "and the cut sets
  feature/topic-pr to track origin/develop".
- The other six still describe only their original difference.

No text names something its record lacks, so nothing misleads the judge, which compares records,
not texts. The texts are inconsistent, though, and a reader of a `fixed:` report sees an
incomplete list of the oracle's divergences.

**Fix:** Append the same clause to each of the six. A minimal option is a helper that builds the
text, for example a `concat!` with `TOPIC_PR_TRACKS` or `PHASE_PR_TRACKS`.

### IN-10: The observer now fails on any checkout whose HEAD is unborn

**File:** `tests/harness/outcome.rs:611` (`head_tree: read_head_tree(...)` is called for every
checkout, under both `FilesExpect` forms), `tests/harness/outcome.rs:650-654`

**Issue:** `read_head_tree` runs `git ls-tree -r -z --full-tree HEAD` through `git_ok`. On a
branch with no commit, git exits 128 ("Not a valid object name HEAD"); this was reproduced in
scratch. `read_checkouts` and `read_index` both handle that state, so before `a0956e6` the
observer could snapshot it. Now `observe` returns an error for it.

No registered fixture has an unborn checkout today (the 186-test run passes), and the failure is
loud rather than a false green. A future scenario would hit this error, though, for example an
action that creates an orphan or empty checkout, or a fixture that starts from a bare `init`.

**Fix:** When `rev-parse --verify --quiet HEAD` fails, treat `head_tree` as empty, the index of a
fresh repository. Add a control that observes an unborn checkout.

---

_Reviewed: 2026-10-26T21:40:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
_Iteration: 3_
