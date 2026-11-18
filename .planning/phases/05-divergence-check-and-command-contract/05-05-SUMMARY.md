---
phase: 05-divergence-check-and-command-contract
plan: 05
subsystem: testing
tags: [rust, proc-macro2, nextest, if-02, harness, registry]

requires:
  - phase: 05-divergence-check-and-command-contract
    provides: "05-02: the #[path] commands module in tests/harness/main.rs, so the harness reads the binary's COMMANDS table"
provides:
  - "tests/if02_scan.rs: scan(root) tokenises every src/**/*.rs and fails on a literal naming ROADMAP.md, STATE.md, .planning/config.json, devflow.toml, or .claude/.codex as a path component; six tests including a planted-read control"
  - "rust::command_word: the first wsx argument for an action; wsx_argv takes its first word from it"
  - "registry::no_agent_file_gaps: every COMMANDS entry needs an enabled no-GSD run with a devflow parity control expecting the same exit (D-15)"
affects: [05-06, 05-07, 05-08, 05-09, phase 6-10 command plans]

actuals:
  tokens: 5700
  tasks: 2
  commits: 4

plan_head_before: 282807c58384e7fa2003af46cda775bf563a1aed
plan_head_after: 730079f19ae2c150fad572bda5ec974545b090cf

tech-stack:
  added: ["proc-macro2 1.0.107 as a direct dev-dependency (already in Cargo.lock; no crate added)"]
  patterns:
    - "Source rules over src/ are token scans with proc-macro2, never text greps: comments drop out and doc comments arrive as #[doc] attributes that are skipped whole"
    - "A command added to COMMANDS cannot pass the suite until a no-GSD scenario with a devflow parity control runs it"

key-files:
  created:
    - tests/if02_scan.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - tests/harness/registry.rs
    - tests/harness/rust.rs

key-decisions:
  - "A read error on a source file panics the test (expect) rather than becoming a finding; non-UTF-8 and lexer refusals are findings as the plan specifies"
  - "D-15's subject need not be Parity (check::no_gsd_repo's subject is Fixed against devflow's defect); the parity requirement sits on the devflow-layout control, as the plan states"

patterns-established:
  - "Planted controls for source rules live in CARGO_TARGET_TMPDIR/if02-scan/<test>, never in the checkout"

requirements-completed: [IF-02]

coverage:
  - id: D1
    description: "IF-02 source scan: the real src/ passes and the scan counts as many files as an independent walk; a planted read of .planning/config.json in a scratch copy is exactly one finding naming the file, the literal and IF-02"
    requirement: IF-02
    verification:
      - kind: unit
        ref: "tests/if02_scan.rs#real_tree_reads_no_agent_file"
        status: pass
      - kind: unit
        ref: "tests/if02_scan.rs#planted_read_trips_the_scan"
        status: pass
      - kind: unit
        ref: "tests/if02_scan.rs#each_forbidden_name_trips_the_scan"
        status: pass
    human_judgment: false
  - id: D2
    description: "Comments and doc comments do not trip the scan while a same-file literal does; .claude/.codex match only as a whole path component; an unlexable file is a finding"
    requirement: IF-02
    verification:
      - kind: unit
        ref: "tests/if02_scan.rs#comments_do_not_trip_the_scan"
        status: pass
      - kind: unit
        ref: "tests/if02_scan.rs#component_match_is_exact"
        status: pass
      - kind: unit
        ref: "tests/if02_scan.rs#unlexable_file_fails_closed"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every command in COMMANDS has an enabled Rust run in a repository without GSD, devflow or agent files whose expected exit matches the configured fixture's parity case; a missing, disabled or mismatched run is named"
    requirement: IF-02
    verification:
      - kind: unit
        ref: "tests/harness/registry.rs#registry::every_command_runs_in_a_repo_without_agent_files"
        status: pass
      - kind: unit
        ref: "tests/harness/registry.rs#registry::a_command_without_a_no_agent_files_run_is_named"
        status: pass
      - kind: unit
        ref: "tests/harness/registry.rs#registry::a_disabled_no_agent_files_run_is_a_gap"
        status: pass
      - kind: unit
        ref: "tests/harness/registry.rs#registry::a_no_agent_files_run_that_differs_from_the_configured_fixture_is_a_gap"
        status: pass
      - kind: integration
        ref: "tests/harness/check.rs#check::no_gsd_repo::rust"
        status: pass
      - kind: unit
        ref: "tests/harness/rust.rs#rust::every_action_maps_to_a_wsx_command"
        status: pass
    human_judgment: false

duration: 12min
completed: 2026-11-18
status: complete
---

# Phase 5 Plan 05: IF-02 enforcement Summary

**A proc-macro2 token scan fails the suite on any literal in src/ naming a GSD, devflow or agent file, and a registry rule over the binary's COMMANDS table requires every command to have an enabled no-agent-files run matching the configured fixture.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-11-18T20:53:08Z
- **Completed:** 2026-11-18T21:04:58Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments
- tests/if02_scan.rs: `scan` walks src/ sorted, tokenises each file, skips `#[doc]`/`#![doc]` attributes, and reports `<path>: literal <lit> names <name>, a GSD, devflow or agent file wsx must not read (IF-02)`; non-UTF-8 or unlexable files report `cannot tokenize <path>: <error>`. No allowlist.
- The real src/ (20 files, matched by an independent iterative walk) passes; the planted copy under CARGO_TARGET_TMPDIR trips it once; the checkout's src/ is untouched (`git status --porcelain -- src` empty).
- `rust::command_word` names each action's wsx command; `wsx_argv` pushes it first.
- `registry::no_agent_file_gaps` iterates COMMANDS (through the 05-02 `#[path]` module): `check` is covered by `check::no_gsd_repo` (no-GSD subject exit 0, devflow Clean parity control exit 0).

## Task Commits

1. **Task 1: IF-02 source scan** - `5dadbeb` (test, RED), `c01704f` (test, GREEN)
2. **Task 2: no-agent-files run for every command** - `e358bc7` (test, RED), `730079f` (test, GREEN)

## TDD Gate Compliance

- **Task 1 RED:** `planted_read_trips_the_scan` failed on its planned `assert_eq!` of the finding count (left 0 from the stub, right 1). Classifier: `RED_EVIDENCE_OK` (target/gsd/05-05-red-evidence.json). Semantic assessment: the target compiled and ran after the scratch copy and planted.rs were written; the failure is the planned assertion, not a build, load or fixture fault.
- **Task 2 RED:** `registry::a_command_without_a_no_agent_files_run_is_named` failed on its planned `assert_eq!` of the gap count (left 0 from the stub, right 1). Classifier: `RED_EVIDENCE_OK` (target/gsd/05-05-red-evidence-2.json). Semantic assessment: the target compiled and ran over the real registry; the failure is the planned assertion.
- **GREEN commit type:** both GREEN commits are `test(05-05)`, as the plan names them, because every change is test code (precedent: 04-18). A gate scan for `feat(05-05)` finds none; the RED to GREEN order holds by ancestry (5dadbeb before c01704f, e358bc7 before 730079f).

## Files Created/Modified
- `tests/if02_scan.rs` - the IF-02 scan and its six tests
- `Cargo.toml`, `Cargo.lock` - proc-macro2 direct dev-dependency (lock gains one dependency line; no new package)
- `tests/harness/rust.rs` - `command_word`; `wsx_argv` uses it
- `tests/harness/registry.rs` - `no_agent_file_gaps`, its run model and four tests

## Decisions Made
- A source file that cannot be read at all panics the test rather than being a finding: the plan's fail-closed rule covers non-UTF-8 and lexer refusals, and a panic also fails the suite.
- D-15's parity requirement is placed on the devflow-layout control, not the subject: `check::no_gsd_repo`'s subject is `Fixed` because devflow itself refuses the no-GSD repo.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing control] Test for the exit-mismatch gap text**
- **Found during:** Task 2 (GREEN)
- **Issue:** The plan's three registry tests exercise the "no enabled Rust run" gap only; the second gap text (expected exits differ) had no test that could fail.
- **Fix:** Added `registry::a_no_agent_files_run_that_differs_from_the_configured_fixture_is_a_gap`: `check::no_gsd_repo` rebuilt with its devflow controls expecting exit 1 yields exactly that gap with both exits.
- **Files modified:** tests/harness/registry.rs
- **Verification:** the test passes; `scripts/check.sh all` OK
- **Committed in:** 730079f

**2. [Rule 3 - Blocking] clippy collapsible_if in doc_attribute_end**
- **Found during:** Task 1 (GREEN), `scripts/check.sh all`
- **Fix:** collapsed into a let-chain; committed in c01704f.

---

**Total deviations:** 2 auto-fixed (1 missing control, 1 blocking lint). **Impact:** no scope change.

## Issues Encountered
None.

## Verification
- All six if02_scan tests, the four registry tests, `registry::every_scenario_is_well_formed`, `rust::every_action_maps_to_a_wsx_command` and `check::no_gsd_repo::rust` pass, each as its own `--no-tests=fail -E 'test(=...)'` run.
- `scripts/check.sh all`: last line `==> check.sh: all OK` (799 tests run, 799 passed, 57 skipped; target/gsd/05-05-all.log).
- `scripts/check.sh deps`: `advisories ok, bans ok, licenses ok, sources ok` (target/gsd/05-05-deps.log).
- No `allowlist|allow_list|exception` in non-comment lines of tests/if02_scan.rs; `rg -q COMMANDS tests/harness/registry.rs` exits 0.
- Scope of the evidence: literal reads in src/ and the registry rule; a path built at run time (`concat!`, `format!`, escapes) is accepted as T-05-18.

## User Setup Required
None - no external service configuration required.

## Next Phase Readiness
Ready for 05-06. Any command added to COMMANDS later needs a no-GSD scenario with a devflow parity control before the suite passes.

---
*Phase: 05-divergence-check-and-command-contract*
*Completed: 2026-11-18*

## Self-Check: PASSED
