---
phase: 05-divergence-check-and-command-contract
plan: 09
subsystem: testing
tags: [cargo-mutants, dead-code, llvm-cov, nextest, container-gate]

requires:
  - phase: 05-08
    provides: the finished phase-5 source (check, hooks, staleness, report, exit, json, cli, trees, git staleness)
provides:
  - item-level dead_code expectations in place of the module-level ones in src/git/mod.rs and src/model/mod.rs
  - a fresh mutation pass over the nine phase-5 files at 69bc5d2, tested = listed, no unexplained survivor
  - eight survivor tests and a deadline on the two symlink-loop tests
  - coverage, deps and container gates on the final tip
affects: [06-pre-push-guard, 07-workspace-sync, 11-commit-guard, 13-status-and-doctor]

actuals:
  tokens: 4968
  tasks: 2
  commits: 4
plan_head_before: 68cd18dcc565efa3034b0ff47641fe4ea4b094a9
plan_head_after: 69bc5d287e16a12d2b90cf0a2e6aa75721be6641

tech-stack:
  added: []
  patterns:
    - "A test that could hang on a mutant judges on its own thread and must answer within a deadline, so the mutant fails as an assertion"

key-files:
  created: []
  modified:
    - src/git/mod.rs
    - src/git/reads.rs
    - src/git/refs.rs
    - src/git/repo.rs
    - src/git/trees.rs
    - src/git/staleness.rs
    - src/model/mod.rs
    - src/check/hooks.rs
    - src/json.rs
    - tests/cli.rs

key-decisions:
  - "Each item no command reaches yet carries cfg_attr(not(test), expect(dead_code, reason = \"first caller in phase N\")), N from ROADMAP; MODE_BLOB has no ROADMAP caller and says \"not called by any command yet\""
  - "Three mutants are allowlisted as equivalent (distinct_entries key negation, below's > vs >=, resolve's last-guard fallback); every other survivor got a behaviour test"
  - "The symlink-loop tests run under a ten-second deadline so a hop count that never grows fails as an assertion instead of a cargo-mutants timeout"

patterns-established:
  - "judge_within_deadline in src/check/hooks.rs tests: run a possibly endless walk on a thread with recv_timeout"

requirements-completed: [DIV-01, DIV-02, DIV-03, DIV-04, IF-01, IF-02]

coverage:
  - id: D1
    description: "No module-level dead_code expectation in src/git/mod.rs or src/model/mod.rs; each unreached item has its own, and check.sh all passes"
    verification:
      - kind: other
        ref: "! rg -q 'no caller until phase 5' src && ! rg -q '^#!\\[cfg_attr\\(not\\(test\\), expect\\(dead_code' src/git/mod.rs src/model/mod.rs && scripts/check.sh all"
        status: pass
    human_judgment: false
  - id: D2
    description: "Fresh mutation pass at 69bc5d2 over the nine phase-5 files: 230 listed = 194 caught + 3 missed (all allowlisted) + 33 unviable + 0 timeout; doctored copies trip both checks"
    verification:
      - kind: other
        ref: "05-09-PLAN.md Task 2 verify blocks 1-3 (cargo mutants exit 2, survivor block exit 0, identity block exit 0)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Survivor tests: dot component in a symlink hook, loop judged within a deadline, raw space in JSON, C-quoted unknown command, worktrees not a directory, negative epoch seconds, signed reflog time, signed tree mode, tree asked for as a blob"
    verification:
      - kind: unit
        ref: "src/check/hooks.rs#check::hooks::tests::symlink_with_a_dot_component_passes"
        status: pass
      - kind: unit
        ref: "src/check/hooks.rs#check::hooks::tests::symlink_loop_stops_at_the_hop_limit"
        status: pass
      - kind: unit
        ref: "src/json.rs#json::tests::a_space_is_written_raw"
        status: pass
      - kind: integration
        ref: "tests/cli.rs#unknown_command_is_named_c_quoted_on_stderr"
        status: pass
      - kind: unit
        ref: "src/git/staleness.rs#git::staleness::tests::fetch_head_paths_refuse_a_worktrees_entry_that_is_not_a_directory"
        status: pass
      - kind: unit
        ref: "src/git/trees.rs#git::trees::tests::read_blobs_refuses_an_object_that_is_not_a_blob"
        status: pass
    human_judgment: false
  - id: D4
    description: "scripts/check.sh all, deps and coverage, and scripts/check-in-container.sh all exit 0 on 69bc5d2"
    verification:
      - kind: other
        ref: "scripts/check.sh coverage; scripts/check.sh deps; scripts/check-in-container.sh all"
        status: pass
    human_judgment: false

duration: 2h 40m
completed: 2026-11-19
status: complete
---

# Phase 5 Plan 09: Dead-code accounting and the phase-closing mutation pass Summary

**Dead code is now expected item by item, each naming the phase of its first caller, and the nine phase-5 files pass a fresh no-iterate mutation pass at 69bc5d2 with only three justified equivalent mutants, plus host, deps, coverage and container gates on that tip.**

## Performance

- **Duration:** 2h 40m (two full mutation passes of 62 and 43 minutes, one stopped pass)
- **Started:** 2026-11-18T22:17:20Z
- **Completed:** 2026-11-19T00:57:49Z
- **Tasks:** 2
- **Files modified:** 10

## Accomplishments

- The module-level `dead_code` expectations are gone from `src/git/mod.rs` and `src/model/mod.rs` (the STATE obligation and D-07); 20 items carry their own.
- The final mutation pass leaves no unexplained survivor, and it tested exactly the mutants `cargo mutants --list` lists.
- The first pass's nine real gaps each got a named behaviour test, every one shown failing on its mutant.

## Task Commits

1. **Task 1: item-level dead-code expectations** - `0f2c0b3` (chore)
2. **Task 2: survivor tests** - `7cf29c1`, `663819a`, `69bc5d2` (test)

## Task 1: the expectations

The module attributes came off both files; neither had been removed by 05-01. `cargo clippy` then named these items, each now under `#[cfg_attr(not(test), expect(dead_code, reason = ...))]`, with the moved `cfg_attr(not(test))` comment beside the first one in each file:

| Item | File | Reason |
|------|------|--------|
| `ObjectId::from_gix` | src/git/mod.rs | first caller in phase 6 |
| `Error::StaleWorktree` | src/git/mod.rs | first caller in phase 7 |
| `AheadBehind` | src/git/reads.rs | first caller in phase 13 |
| `Repo::merge_base` | src/git/reads.rs | first caller in phase 7 |
| `Repo::ahead_behind` | src/git/reads.rs | first caller in phase 13 |
| `Repo::range_walk` | src/git/reads.rs | first caller in phase 6 |
| `Repo::commit_ids` | src/git/reads.rs | first caller in phase 6 |
| `Repo::commit_id` | src/git/reads.rs | first caller in phase 6 |
| `render_ids` | src/git/reads.rs | first caller in phase 6 |
| `Repo::peel_commit` | src/git/refs.rs | first caller in phase 6 |
| `Repo::peeled_commit` | src/git/refs.rs | first caller in phase 6 |
| `Repo.index_path` (field) | src/git/repo.rs | first caller in phase 11 |
| `Repo::index_path` | src/git/repo.rs | first caller in phase 11 |
| `Repo::siblings` | src/git/repo.rs | first caller in phase 7 |
| `Repo::list_failed` | src/git/repo.rs | first caller in phase 7 |
| `refuse_missing_worktree` | src/git/repo.rs | first caller in phase 7 |
| `refuse_untrusted_sibling` | src/git/repo.rs | first caller in phase 7 |
| `MODE_BLOB` | src/git/trees.rs | not called by any command yet |
| `Model::personal_paths` | src/model/mod.rs | first caller in phase 6 |
| `Model::is_personal` | src/model/mod.rs | first caller in phase 6 |

How each phase was chosen from ROADMAP: phase 6's guard walks a pushed range and tests personal paths (`range_walk` and its helpers, the peel, `personal_paths`/`is_personal`); phase 7's sync runs from any worktree and refuses a rewritten integration branch (`siblings` and its refusals, `merge_base`); phase 11's commit guard reads a hook's index (`index_path`); phase 13's status shows ahead/behind. `src/git/spawn.rs` and `src/model/manifest.rs` had no unused item (research's `encode_path` is reached now). The verify block exited 0 (`target/gsd/05-09-all.log` ends `==> check.sh: all OK`); at the plan's base the same block failed on the two `no caller until phase 5` attributes.

## Task 2: mutation, coverage and gates

**First pass** on 0f2c0b3 (62 min): 230 mutants, 184 caught, 12 missed, 33 unviable, 1 timeout (`target/gsd/05-09-first-missed.txt`, `05-09-first-timeout.txt`). Each survivor:

| Survivor | Outcome |
|----------|---------|
| `src/cli/mod.rs:73:5` `shown` -> `String::new()` and -> `"xyzzy".into()` | test `unknown_command_is_named_c_quoted_on_stderr` (tests/cli.rs) |
| `src/json.rs:113:32` `<` -> `<=` in `write_string` | test `json::tests::a_space_is_written_raw` |
| `src/check/hooks.rs:157:13` delete arm `b"" \| b"."` | test `check::hooks::tests::symlink_with_a_dot_component_passes` |
| `src/check/hooks.rs:211:15` `+=` -> `*=` in `splice` (timeout) | the two loop tests now judge within a 10 s deadline (`judge_within_deadline`) |
| `src/git/staleness.rs:95:25` NotFound guard -> `true` | test `fetch_head_paths_refuse_a_worktrees_entry_that_is_not_a_directory` |
| `src/git/staleness.rs:122:91` delete `-` in `epoch_seconds` | test `epoch_seconds_is_negative_before_the_epoch` |
| `src/git/staleness.rs:145:45` `&&` -> `\|\|` in `parse_reflog_selector` | test `parse_reflog_selector_refuses_a_signed_time` |
| `src/git/trees.rs:149:58` blob guard -> `true` in `read_blobs` | test `read_blobs_refuses_an_object_that_is_not_a_blob` |
| `src/git/trees.rs:168:24` `\|\|` -> `&&` in `octal_mode` | test `parse_lookup_refuses_a_signed_mode` |
| `src/check/mod.rs:262:49` `==` -> `!=` in `distinct_entries` | allowlisted (below) |
| `src/check/hooks.rs:122:40` `>` -> `>=` in `below` | allowlisted (below) |
| `src/check/hooks.rs:171:34` guard `last` -> `false` in `resolve` | allowlisted (below) |

Each new test was run against its mutant applied by hand and failed with an assertion (for the loop tests: `judging did not answer within 10s` at 10 s, and the whole filtered set finished in 25 s under that mutant instead of hanging). No production defect was found, so there is no `fix(05-09)` commit.

**Allowlist** `target/gsd/05-09-equivalent-mutants.txt`, one line each:

- `src/check/mod.rs:262:49: replace == with != in distinct_entries`: negating the bool half of the dedup key is a bijection on keys, so the same entries are kept.
- `src/check/hooks.rs:122:40: replace > with >= in below`: `rest.len() == 1` means a path of the hooks dir plus a lone `/`, which git's tree listing never yields.
- `src/check/hooks.rs:171:34: replace match guard last with false in resolve`: the directory then falls through to the end of the walk, which returns the same "is a directory" message for the same path.

**Final pass** on `69bc5d2` (`target/gsd/05-09-pass-head.txt`), without `--iterate`, 43 min: 194 caught, 3 missed, 33 unviable, 0 timeout (`wc -l`), sum 230 = the `cargo mutants --list` total of 230. cargo-mutants exited 2; the survivor block exited 0 (all three missed lines are allowlisted); the identity block exited 0 (tested = listed, no `previously_caught.txt`, source unchanged since the pass head). A second pass on 663819a was stopped part way (it showed the `splice` mutant still timing out, which led to 69bc5d2) and is not evidence.

**Doctored copies** under `target/gsd/05-09-checks-control/`: last `caught.txt` line deleted -> identity block exit 1, `tested mutants differ from the listed ones`; last line replaced with `src/exit.rs:1:1: replace nothing with nothing` -> exit 1, same message; that line appended to `timeout.txt` -> survivor block exit 1, printing the line then `unexplained survivors above`. The copies were removed.

**Coverage** (`target/gsd/05-09-coverage.log`, exit 0, nextest `863 tests run: 863 passed (1 slow), 57 skipped`), line coverage:

| File | Lines | Cover |
|------|-------|-------|
| check/mod.rs | 431 | 99.54% |
| check/hooks.rs | 492 | 98.37% |
| check/staleness.rs | 95 | 100.00% |
| check/report.rs | 278 | 99.64% |
| exit.rs | 481 | 100.00% |
| json.rs | 128 | 96.88% |
| cli/mod.rs | 139 | 95.68% |
| git/trees.rs | 384 | 89.84% |
| git/staleness.rs | 459 | 89.76% |
| TOTAL | 16548 | 93.52% |

**Gates on 69bc5d2:** `scripts/check.sh all` exit 0 (863 passed, 57 skipped; run on this tree before its commit); `scripts/check.sh deps` exit 0 (advisories, bans, licenses, sources ok); `scripts/check-in-container.sh all` exit 0, last line `==> check.sh: all OK`, nextest `863 tests run: 863 passed (1 slow), 57 skipped` (`target/gsd/05-09-container.log`). No source was edited while the container run was in progress.

Kept under `target/gsd/` (gitignored): `05-09-mutants/mutants.out`, `05-09-pass-head.txt`, `05-09-files.txt`, `05-09-equivalent-mutants.txt`, `05-09-first-missed.txt`, `05-09-first-timeout.txt`, `05-09-listed.txt`, `05-09-tested.txt`, `05-09-coverage.log`, `05-09-container.log`, `05-09-all.log`. `target/gsd/05-09-mutants-tmp`, `target/gsd/05-09-checks-control` and `target/llvm-cov-target` are gone.

## Files Created/Modified

- `src/git/mod.rs`, `src/model/mod.rs` - module expectation removed; item expectations
- `src/git/reads.rs`, `src/git/refs.rs`, `src/git/repo.rs`, `src/git/trees.rs` - item expectations; trees.rs also two survivor tests
- `src/git/staleness.rs` - three survivor tests
- `src/check/hooks.rs` - dot-component test, `judge_within_deadline` on the two loop tests
- `src/json.rs` - raw-space test
- `tests/cli.rs` - C-quoted unknown-command test

## Decisions Made

See key-decisions. The phase numbers are the earliest ROADMAP phase whose requirement needs the item.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `MODE_BLOB` in src/git/trees.rs, outside Task 1's file list**
- **Found during:** Task 1
- **Issue:** with the module expectation gone, clippy named it unused.
- **Fix:** the same item-level expectation, reason "not called by any command yet".
- **Committed in:** 0f2c0b3

**2. [Rule 1 - Bug] The `splice` mutant read as a timeout although a test failed**
- **Found during:** Task 2 (stopped second pass)
- **Issue:** a standalone deadline test failed at 10 s, but nextest waited for the two existing loop tests, which spun until cargo-mutants' 300 s timeout.
- **Fix:** both loop tests judge through `judge_within_deadline`; the standalone test (added in 7cf29c1) is folded in. Their assertions are unchanged.
- **Committed in:** 69bc5d2

**3. Survivor tests in `tests/cli.rs`, outside Task 2's file list**
- `shown` is reached only through the binary's stderr, so its test lives with the other CLI boundary tests; its name starts `unknown_command`, which the pass's filter selects.

---

**Total deviations:** 3 (1 blocking, 1 bug in the evidence, 1 file placement). **Impact on plan:** none on production behaviour; every change is a test or an attribute.

## Issues Encountered

None beyond the deviations.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

Phase 5's plans are complete. Each later phase's first caller removes the expectation on the items it reaches; rustc reports an expectation left on a used item as unfulfilled.

---
*Phase: 05-divergence-check-and-command-contract*
*Completed: 2026-11-19*

## Self-Check: PASSED

All four task commits are ancestors of HEAD; the SUMMARY exists; 4 commits measured from the plan ledger.
