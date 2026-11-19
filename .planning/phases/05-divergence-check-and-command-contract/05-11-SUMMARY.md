---
phase: 05-divergence-check-and-command-contract
plan: 11
subsystem: testing
tags: [if02, sc5, runtime-comparison, gap-closure, lenient-reader]

requires:
  - phase: 05-divergence-check-and-command-contract
    provides: "05-10's two-state IF-02 runtime comparison (tests/harness/if02_runtime.rs) and decoded token scan (tests/if02_scan.rs)"
provides:
  - "tests/harness/if02_runtime.rs: three agent-file states (A and C valid with real, differing settings; B malformed), four raw-read and two lenient-reader controls"
  - "tests/if02_scan.rs: CRLF line continuation decoded like LF, with its test"
affects: [phase-05 re-verification, IF-02]

actuals:
  tokens: 5459
  tasks: 2
  commits: 4
plan_head_before: ff03eaa6a36b875f2291dd4083791227b2ecbd29
plan_head_after: f67d3cfbe512865845067ec2b6fa2bd75fdc1d0a

tech-stack:
  added: []
  patterns:
    - "Third valid state with different values, so a lenient reader whose fallback equals one state's value is still caught"

key-files:
  created: []
  modified:
    - tests/harness/if02_runtime.rs
    - tests/if02_scan.rs

key-decisions:
  - "Observation carries its State and the state passes into observe explicitly, so the difference text names both states without Runtime tracking a current state"

requirements-completed: [IF-02]

coverage:
  - id: D1
    description: "check and check --json give the same exit code, stdout and stderr in states A, B and C; state A passes; each of three JSON runs masks its tip once"
    requirement: IF-02
    verification:
      - kind: integration
        ref: "scripts/check.sh test -E 'test(=if02_runtime::check_output_does_not_depend_on_agent_files)'"
        status: pass
    human_judgment: false
  - id: D2
    description: "A lenient read of .planning/config.json through a variable-built path makes the states differ, for fallback none (A/B/C texts) and for fallback equal to A's value (A against C only)"
    requirement: IF-02
    verification:
      - kind: integration
        ref: "if02_runtime::lenient_config_read_makes_the_states_differ, if02_runtime::lenient_read_with_state_a_default_differs_in_state_c"
        status: pass
    human_judgment: false
  - id: D3
    description: "A CRLF line continuation gives one STATE.md finding instead of a decoder panic"
    requirement: IF-02
    verification:
      - kind: unit
        ref: "scripts/check.sh test -E 'test(=crlf_line_continuation_trips_the_scan)'"
        status: pass
    human_judgment: false

duration: 6min
completed: 2026-11-19
status: complete
---

# Phase 5 Plan 11: IF-02 runtime states with real settings and lenient-reader controls Summary

**The IF-02 runtime comparison now runs one devflow fixture through three states (A and C valid with real, differing GSD, devflow, Claude and Codex settings; B malformed), and two lenient readers of `.planning/config.json` that fall back to defaults are shown to be caught; the scan decodes a CRLF line continuation instead of panicking.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-11-19T02:28:52Z
- **Completed:** 2026-11-19T02:34:52.014Z
- **Tasks:** 2
- **Files modified:** 2

## Accomplishments

- `Read::Lenient` builds its path from fields (checkout joined with `dir`, then `file`), parses JSON and takes `/git/branching_strategy`, falling back to its default on any failure. Its RED run against the old note-only sentinels failed on the planned non-empty assertion: the reader gave `none` in both states, the gap 05-VERIFICATION.md found by reading.
- After GREEN, the fallback-`none` control's difference text carries `=phase`, `=none` and `=milestone`; the fallback-`phase` control differs between A and C and not between A and B, so state C is what catches it.
- `check` and `check --json` give identical exit code, stdout and stderr across all three states; the four raw-read controls pass with their sentinel texts unchanged.
- `unescape` accepts CR after a backslash; its RED run panicked with `cannot decode the escape \\Some('\\r')` (the lexer accepted the file), confirming WR-01's premise.

## Task Commits

1. **Task 1 (tracer): lenient config read controls** - `ae69933` (test, RED), `ca95de2` (test, GREEN)
2. **Task 2: CRLF line continuation** - `e4884f5` (test, RED), `f67d3cf` (test, GREEN)

## TDD Gate Compliance

- Task 1 RED: `scripts/check.sh test -E 'test(=if02_runtime::lenient_config_read_makes_the_states_differ)'` exit 100, failing at the non-empty assertion naming the read; target/gsd/05-11-red-evidence.json prints RED_EVIDENCE_OK. Semantic assessment: the target compiled and ran, `enter`'s git status assertion passed in both states, so the failure is the planned assertion, not a build or fixture fault.
- Task 2 RED: `scripts/check.sh test -E 'test(=crlf_line_continuation_trips_the_scan)'` exit 100, unescape's panic naming the CR at tests/if02_scan.rs:397; target/gsd/05-11-red-evidence-2.json prints RED_EVIDENCE_OK. Semantic assessment: no "cannot tokenize" finding, so the lexer accepted the continuation and the decoder is what failed.
- GREEN: every named test passed alone (seven if02_runtime tests, four if02_scan tests), and `scripts/check.sh all` exited 0 before each GREEN commit; target/gsd/05-11-all.log ends with `==> check.sh: all OK`.
- Base guard: `git diff --quiet <base> HEAD -- Cargo.toml Cargo.lock bin/wsx tests/sandbox.sh src` exits 0; the same diff over `tests` exits 1 (negative control).

## Gap items

| VERIFICATION missing item | Closed by |
|---|---|
| State A: real keys at non-default values | `gsd_config`, `claude_settings`, `devflow_config`, `codex_config`, `gsd_state`, `gsd_roadmap` in if02_runtime.rs; held by check_output_does_not_depend_on_agent_files |
| A third state with well-formed, different values | `State::C`; lenient_read_with_state_a_default_differs_in_state_c |
| A lenient reader-variant control | lenient_config_read_makes_the_states_differ and lenient_read_with_state_a_default_differs_in_state_c |
| Narrow the //! sentence at line 8 | //! header rewritten: "so a parse that fails on it also differs" removed, state bullets and Reach paragraph name what the three states prove |

- **WR-01:** premise confirmed by the RED run and fixed (one match arm); no scan rule added or widened.
- **Backlog (not planned, outside the operator's scope for this gap):** REVIEW-GAP IN-01 (the scan header's "consts and statics" wording); WR-01's optional `.gitattributes` forcing LF for `*.rs`.

## Deviations from Plan

- The non-empty assertion's message reads "...through a variable-built path made no difference between the states", without the comma the plan's text shows after "path": it reuses `assert_read_differs`'s existing "a read of {what} made no difference" format. No test asserts this message.

## Issues Encountered

None.

## Next Phase Readiness

Phase 5 is ready for re-verification of SC5 (gap WR-02) and WR-01.

## Self-Check: PASSED
