---
phase: 05-divergence-check-and-command-contract
plan: 02
subsystem: cli
tags: [rust, cli, json, serde_json, proptest, exit-codes]

requires:
  - phase: 05-divergence-check-and-command-contract
    provides: "05-01: COMMANDS table, exit::classify, check::Report, report::human"
provides:
  - "src/json.rs: json::Value, json::bytes (D-09), Value::render (compact, RFC 8259 escaping, keys in build order)"
  - "wsx <name> [--json]: one object on stdout for exit 0 to 6; human text on stderr; usage object after a command word"
  - "report::json: configured, workspace {branch, tip}, integration {branch, ref, tip}, problems [{kind, path, detail}]"
  - "D-04 notice on stderr when the workspace branch was inferred"
  - "tests/harness/contract.rs: the --json contract over real fixtures, iterating the binary's own COMMANDS"
affects: [05-04, 05-06, 05-07, 05-08, 05-09]

actuals:
  tokens: 11066
  tasks: 2
  commits: 4

plan_head_before: 2435032dd9031b730d91356f110f8e3731429196
plan_head_after: 143d781379c3d0d01ee08aa5a32ad8347a7e0d9d

tech-stack:
  added: ["serde_json 1.0.154 (dev-dependency only; brings zmij 1.0.23)"]
  patterns:
    - "One Sinks value decides each line's stream: diagnostics stderr, verdict stdout or stderr with --json, the object stdout once, last, in one write"
    - "Byte fields reach JSON only through json::bytes: UTF-8 as a string, else {\"hex\": ...}"
    - "The harness includes src/cli/commands.rs with #[path], so contract tests iterate the binary's table, never a copy"

key-files:
  created:
    - src/json.rs
    - tests/harness/contract.rs
  modified:
    - src/main.rs
    - src/cli/mod.rs
    - src/check/report.rs
    - src/check/mod.rs
    - src/exit.rs
    - Cargo.toml
    - Cargo.lock
    - tests/cli.rs
    - tests/harness/main.rs

key-decisions:
  - "A failed write of the JSON object to stdout is reported on stderr and exits 6 (operating-system failure) instead of panicking"
  - "ExitClass::name keeps its cfg_attr(not(test)) dead_code expectation: the failure object carries kind and exit_code, not the class name, so --json does not read it"
  - "Usage errors parse argv per argument as OsStr, so a non-UTF-8 argument after a command word with --json still yields the usage object"

patterns-established:
  - "Every --json run: envelope(spec, payload) = command, schema_version, then the payload's keys; failure(spec, kind, message, class) adds error {kind, message, exit_code}"

requirements-completed: [IF-01, DIV-01]

coverage:
  - id: D1
    description: "JSON emitter whose output serde_json reads back exactly, for any string and any bytes, with the D-09 hex form"
    requirement: IF-01
    verification:
      - kind: unit
        ref: "src/json.rs#json::tests::strings_escape_as_rfc8259"
        status: pass
      - kind: unit
        ref: "src/json.rs#json::tests::non_utf8_bytes_render_as_hex"
        status: pass
      - kind: unit
        ref: "src/json.rs#json::tests::integers_at_the_i64_limits_round_trip"
        status: pass
      - kind: unit
        ref: "src/json.rs#json::tests::object_keys_keep_their_order"
        status: pass
      - kind: unit
        ref: "src/json.rs#json::tests::any_string_round_trips_through_serde_json"
        status: pass
      - kind: unit
        ref: "src/json.rs#json::tests::any_bytes_round_trip_through_serde_json"
        status: pass
    human_judgment: false
  - id: D2
    description: "wsx check --json prints exactly one object on stdout for exit 0, 1 and 3, with the D-11 fields, hex for a non-UTF-8 path, and error {kind, message, exit_code} on failure"
    requirement: IF-01
    verification:
      - kind: e2e
        ref: "tests/harness/contract.rs#contract::check_json_reports_problems_as_one_object"
        status: pass
      - kind: e2e
        ref: "tests/harness/contract.rs#contract::check_json_passes_as_one_object"
        status: pass
      - kind: e2e
        ref: "tests/harness/contract.rs#contract::check_json_encodes_a_non_utf8_path_as_hex"
        status: pass
      - kind: e2e
        ref: "tests/harness/contract.rs#contract::check_json_failure_is_one_object"
        status: pass
      - kind: e2e
        ref: "tests/harness/contract.rs#contract::every_command_prints_one_json_object"
        status: pass
    human_judgment: false
  - id: D3
    description: "Usage errors: after a command word with --json, check's usage object (exit 2); before any command, nothing on stdout"
    requirement: IF-01
    verification:
      - kind: integration
        ref: "tests/cli.rs#check_json_usage_error_is_one_object"
        status: pass
      - kind: integration
        ref: "tests/cli.rs#unknown_command_with_json_prints_no_json"
        status: pass
      - kind: integration
        ref: "tests/cli.rs#duplicate_json_flag_is_a_usage_error"
        status: pass
      - kind: integration
        ref: "tests/cli.rs#missing_argument_exits_with_usage"
        status: pass
      - kind: integration
        ref: "tests/cli.rs#check_with_an_extra_argument_exits_with_usage"
        status: pass
    human_judgment: false
  - id: D4
    description: "D-04: an unconfigured repo with one candidate is checked against it with configured false and a notice; none or two candidates exit 3 with kind workspace_candidates"
    requirement: DIV-01
    verification:
      - kind: e2e
        ref: "tests/harness/contract.rs#contract::check_in_an_unconfigured_repo_checks_the_one_candidate"
        status: pass
      - kind: e2e
        ref: "tests/harness/contract.rs#contract::check_with_no_workspace_branch_is_not_configured"
        status: pass
      - kind: e2e
        ref: "tests/harness/check.rs#check::self_test::rust"
        status: pass
    human_judgment: false

duration: 17min
completed: 2026-11-18
status: complete
---

# Phase 5 Plan 02: wsx check --json and the D-09 byte encoding Summary

**`wsx check --json` prints one versioned object on stdout for every outcome, from a dependency-free emitter that serde_json reads back exactly; non-UTF-8 paths are `{"hex": ...}`, and an unconfigured repo with one workspace branch is checked against it with a notice.**

## Performance

- **Duration:** 17 min
- **Started:** 2026-11-18T20:02:30Z
- **Completed:** 2026-11-18T20:19:36Z
- **Tasks:** 2
- **Files modified:** 11

## Accomplishments

- `src/json.rs`: `Value`, `bytes` and `render`; RFC 8259 escaping (short escapes for the seven, `\u00xx` lowercase for other controls, everything else raw UTF-8), keys in build order, compact.
- `wsx <name> [--json]`: stdout carries exactly one object and a newline on exit 0, 1, 2 and 3; the verdict and diagnostics go to stderr with `--json`; a usage error after a command word with `--json` prints that command's failure object; before any command, stdout stays empty.
- `report::json` carries `configured`, `workspace`, `integration` and `problems` (empty array when none); every byte field goes through `json::bytes`.
- D-04: with `wsx.sourceBranch` unset and one candidate the check runs and stderr says the key is not set, names the branch and says to run wsx init.

## Task Commits

1. **Task 1: JSON emitter (RED)** - `c1420b0` (test)
2. **Task 1: JSON emitter (GREEN)** - `739d87a` (feat)
3. **Task 2: --json contract (RED)** - `6000f14` (test)
4. **Task 2: --json contract (GREEN)** - `143d781` (feat)

## TDD Gate Compliance

- **Task 1 RED:** `json::tests::strings_escape_as_rfc8259` failed on its `assert_eq!`: the stub `render` returned `""`. Classifier: `RED_EVIDENCE_OK` (target/gsd/05-02-red-evidence.json). Semantic assessment: the crate compiled and the target ran; the failure is the planned escaping assertion, not a build or load fault.
- **Task 2 RED:** `contract::check_json_reports_problems_as_one_object` failed on its exit-code assertion: wsx exited 2 (`check takes no argument, got --json`) where 1 was expected. Classifier: `RED_EVIDENCE_OK` (target/gsd/05-02-red-evidence-2.json). Semantic assessment: the fixture built and wsx ran in `work`; the failure is the planned assertion.
- GREEN commits follow each RED; no REFACTOR commit was needed.
- Negative controls run once each and reverted: dropping the quote escape made `any_string_round_trips_through_serde_json` fail with "serde_json rejects"; sending the verdict to stdout under `--json` made `check_json_passes_as_one_object` fail with "stdout is not one line".

## Files Created/Modified

- `src/json.rs` - the emitter and the D-09 byte encoding, with its unit and property tests
- `src/cli/mod.rs` - the `[--json]` grammar, `Sinks`, the envelope and the failure object
- `src/check/report.rs` - `report::json` and the D-04 notice in `human`
- `src/check/mod.rs` - `ProblemKind::name` loses its dead_code expectation (now read by `--json`)
- `src/exit.rs` - `ExitClass::name`'s expectation reason updated (still read only by tests)
- `src/main.rs` - `mod json;`
- `Cargo.toml`, `Cargo.lock` - serde_json 1.0.154 dev-dependency; the lock gains only serde_json and zmij
- `tests/harness/contract.rs`, `tests/harness/main.rs` - the contract tests and the `#[path]` include of the command table
- `tests/cli.rs` - three usage-with-`--json` tests

## Decisions Made

- A failed write of the JSON object exits 6 with a stderr line rather than panicking, since `println!` would abort on a closed pipe.
- `ExitClass::name` stays test-read only: the contract's failure object has `exit_code`, not a class name, and no later phase-5 plan reads it.
- argv after the command word is compared as `OsStr`, so `--json` is honoured beside a non-UTF-8 argument.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] ExitClass::name's dead_code expectation kept, with a corrected reason**
- **Found during:** Task 2
- **Issue:** 05-01 left the expectation saying "read by --json, 05-02", but the planned failure object carries no class name, so removing it would fail clippy (dead code) and keeping the old reason would misstate it.
- **Fix:** kept the `cfg_attr(not(test), expect(dead_code))`, reworded the reason and the comment.
- **Files modified:** src/exit.rs
- **Verification:** `scripts/check.sh all` OK
- **Committed in:** 143d781

**2. [Rule 2 - Missing critical] stdout write failure handled**
- **Found during:** Task 2
- **Issue:** the plan's single write of the object needs an outcome when stdout is closed; `println!` would panic.
- **Fix:** `Sinks::finish` writes with `write_all` and `flush`, and on error prints to stderr and exits 6.
- **Files modified:** src/cli/mod.rs
- **Verification:** contract tests pass; the write path is not separately tested.
- **Committed in:** 143d781

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 missing critical). **Impact:** no scope change.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Verification

- `scripts/check.sh all`: last line `==> check.sh: all OK` (target/gsd/05-02-all.log, after both GREEN commits).
- `scripts/check.sh deps`: `advisories ok, bans ok, licenses ok, sources ok`.
- Each of the six json tests, the seven contract tests, the five cli tests and `check::self_test::rust` passed as its own `--no-tests=fail` command.
- Acceptance: no `from_utf8_lossy` in src/json.rs; no `expect(dead_code` in src/json.rs; one `src/cli/commands.rs` line in tests/harness/main.rs; no `println!` under src/check; serde_json only under `[dev-dependencies]`.

## Next Phase Readiness

Ready for 05-03. The staleness (05-07) and hooks (05-06) payload fields and the schema file (05-08) slot into `report::json` between `integration` and `problems`.

---
*Phase: 05-divergence-check-and-command-contract*
*Completed: 2026-11-18*

## Self-Check: PASSED
