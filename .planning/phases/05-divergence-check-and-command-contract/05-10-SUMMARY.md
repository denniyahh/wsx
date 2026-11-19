---
phase: 05-divergence-check-and-command-contract
plan: 10
subsystem: testing
tags: [if02, sc5, proc-macro2, runtime-comparison, gap-closure]

requires:
  - phase: 05-divergence-check-and-command-contract
    provides: "05-05's IF-02 token scan (tests/if02_scan.rs) and the harness fixture, env and contract runner"
provides:
  - "tests/harness/if02_runtime.rs: wsx check and check --json compared across two agent-file states, with four reader-variant controls"
  - "tests/if02_scan.rs: literals matched on decoded value, concat! values, chained .join paths, nine new tests"
affects: [phase-05 re-verification, IF-02]

actuals:
  tokens: 11362
  tasks: 3
  commits: 6
plan_head_before: 27bd3ba83d935df0f3e4f5a138458b39a714a3f8
plan_head_after: 1805a0bd1f2f4bf6e15c1cb2c97259e652b68f94

tech-stack:
  added: []
  patterns:
    - "Runtime independence test: one fixture in two states, outputs compared with only each run's own tip masked, and a reader variant per channel as the positive control"

key-files:
  created:
    - tests/harness/if02_runtime.rs
  modified:
    - tests/harness/main.rs
    - tests/if02_scan.rs

key-decisions:
  - "The scan's dot-component gap is written as .planning/./config.json in the //! header, the spelling a join argument ./config.json produces"

requirements-completed: [IF-02]

coverage:
  - id: D1
    description: "wsx check and check --json give the same exit code, stdout and stderr whether the SC5 files are well-formed or malformed and whether the HOME and ignored agent files exist; state A passes"
    requirement: IF-02
    verification:
      - kind: integration
        ref: "scripts/check.sh test -E 'test(=if02_runtime::check_output_does_not_depend_on_agent_files)'"
        status: pass
    human_judgment: false
  - id: D2
    description: "A read of a tracked worktree file, the ignored file, a committed blob or a HOME file makes the two states differ, carrying the file's content"
    requirement: IF-02
    verification:
      - kind: integration
        ref: "if02_runtime::tracked_file_read_makes_the_states_differ, ignored_file_read_makes_the_states_differ, object_read_makes_the_states_differ, home_read_makes_the_states_differ"
        status: pass
    human_judgment: false
  - id: D3
    description: "Escaped, concat! and chained-join spellings of a forbidden name each trip the scan once; path data, lookalikes and the real src/ stay clean; malformed escapes are lexer findings"
    requirement: IF-02
    verification:
      - kind: unit
        ref: "tests/if02_scan.rs, all fifteen tests, each run alone by name"
        status: pass
    human_judgment: false

duration: 13min
completed: 2026-11-19
status: complete
---

# Phase 5 Plan 10: IF-02 runtime comparison and decoded scan Summary

**wsx check's output is shown independent of the SC5 files by comparing two fixture states with four read controls, and the IF-02 token scan now matches decoded literals, concat! values and chained .join paths while .planning path data stays clean.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-11-19T02:00:36.668Z
- **Completed:** 2026-11-19T02:13:08.218Z
- **Tasks:** 3
- **Files modified:** 3

## Accomplishments

- A runtime test puts one devflow fixture in two states (six tracked SC5 files well-formed then malformed, HOME and git-ignored agent files present then absent) and requires identical exit code, stdout and stderr from `check` and `check --json`; four test-side reader variants each make the states differ, so the comparison is shown to detect a read through each channel.
- The scan decodes every literal (hex and unicode escapes of either case, underscores, line continuations; string, byte, C and char literals), so `"STATE\x2emd"` is a finding.
- `concat!` (nested, `std::`/`core::`, any delimiter) and `.join` chains of literal or `concat!` arguments are matched by their joined value, with each argument still checked alone; the verifier's planted `Path::new(".planning").join("config.json")` read gives exactly one finding.

## Task Commits

1. **Task 1: runtime test (tracer)** - `e2587a7` (test, RED), `ba3a038` (test, GREEN)
2. **Task 2: decoded literals and concat!** - `a97284d` (test, RED), `a9a6ba7` (test, GREEN)
3. **Task 3: join chains, stated reach, phase gate** - `06a37e2` (test, RED), `1805a0b` (test, GREEN)

## Files Created/Modified

- `tests/harness/if02_runtime.rs` - State, Runtime, Read, Observation; the five runtime tests
- `tests/harness/main.rs` - `mod if02_runtime;`
- `tests/if02_scan.rs` - Piece, Joined, JoinKind, Literals; literal_value, unescape, concat_end, concat_bytes, argument_value, join_chain_end, push_joined, file_findings, planted_copy; nine new tests and the rewritten //! header

## Gap items (for re-verification)

| VERIFICATION missing item | Closed by |
|---|---|
| Unescape each literal before matching | `literal_value` and `unescape`; `planted_escaped_read_trips_the_scan`, `escaped_spellings_trip_the_scan`, `escaped_lookalikes_do_not_trip_the_scan`, `malformed_escapes_fail_closed_at_the_lexer` |
| Planted control using `.join(".planning").join("config.json")` that must give one finding | `join_chain_end`; `planted_join_read_trips_the_scan`, `join_spellings_trip_the_scan`, `path_data_does_not_trip_the_scan` |
| Add .planning to COMPONENTS (whole-component match) | SUPERSEDED: a `.planning` component contradicts D-14 (.planning is path data) and would fail the real tree on 28 path-data lines in src/model, fixable only by the allowlist D-14 forbids; the split read it aimed at is caught by the join-chain rule and, for check in any spelling, by the runtime test |

Coverage claim: the scan catches the spellings its //! header lists (decoded literals, concat! values, chained .join of literal or concat! arguments) and no more; for `wsx check`, the only command in COMMANDS, `tests/harness/if02_runtime.rs` covers any read whose result reaches its exit code, stdout or stderr, whatever the spelling.

## TDD Gate Compliance

- **Task 1 RED:** `if02_runtime::tracked_file_read_makes_the_states_differ` failed on its non-empty assertion, "a read of the worktree's .planning/config.json made no difference between the states". Classifier: `RED_EVIDENCE_OK` (target/gsd/05-10-red-evidence.json). Semantic assessment: the fixture built both states (enter's clean-status assertion passed) and the failure is the planned assertion, because RED's `differences` compared exit codes only.
- **Task 2 RED:** `planted_escaped_read_trips_the_scan` failed on the count assertion (left 0, right 1). Classifier: `RED_EVIDENCE_OK` (target/gsd/05-10-red-evidence-2.json). Semantic assessment: the scratch src/ copy was scanned and the failure is the planned assertion, because the scan matched the escaped source text.
- **Task 3 RED:** `planted_join_read_trips_the_scan` failed on the count assertion (left 0, right 1). Classifier: `RED_EVIDENCE_OK` (target/gsd/05-10-red-evidence-3.json). Semantic assessment: neither literal is forbidden alone and no chain rule existed, so the failure is the planned assertion.
- **GREEN commit type:** all three GREEN commits are `test(05-10)`, as the plan names them, because every change is test code (05-05 precedent). Each RED commit precedes its GREEN by ancestry.

## Verification

- All five `if02_runtime` tests and all fifteen `if02_scan` tests exit 0, each run alone by name after the last commit.
- `scripts/check.sh all` exits 0; target/gsd/05-10-all.log ends with `==> check.sh: all OK`.
- `git diff --quiet <base> HEAD -- Cargo.toml Cargo.lock bin/wsx tests/sandbox.sh src` exits 0 (base in target/gsd/05-10-base.txt).

## Decisions Made

- The //! header names the dot-component gap as `.planning/./config.json` (what a `./config.json` join argument produces) rather than the bare `./config.json` the plan text lists.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] clippy sliced_string_as_bytes in literal_value**
- **Found during:** Task 2 GREEN
- **Issue:** `raw[open + 1..close].as_bytes()` failed `clippy -D warnings` in `scripts/check.sh all`
- **Fix:** `raw.as_bytes()[open + 1..close]`
- **Files modified:** tests/if02_scan.rs
- **Committed in:** a9a6ba7 (part of the GREEN commit)

## Issues Encountered

None.

## Next Phase Readiness

Phase 5 is ready for re-verification of SC5 against the gap items above.

## Self-Check: PASSED
