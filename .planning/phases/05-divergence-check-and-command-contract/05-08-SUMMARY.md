---
phase: 05-divergence-check-and-command-contract
plan: 08
subsystem: testing
tags: [json-schema, jsonschema, contract, cargo-deny, nextest]

requires:
  - phase: 05-05
    provides: COMMANDS table, --json envelope and failure object
  - phase: 05-07
    provides: staleness object and the local-branch fixture
provides:
  - docs/json/check.v1.schema.json, strict draft 2020-12 schema for check --json
  - docs/json/README.md, the JSON contract and its versioning rule
  - SCHEMA_PINS, pin_mismatches, coverage_gaps and six D-10 contract tests
  - provenance::hash_blob public, shared by vendored-file and schema pins
affects: [05-09, every later command that joins COMMANDS]

actuals:
  tokens: 8300
  tasks: 2
  commits: 4
plan_head_before: 1d4fa44e820912ef0d0faa64a392bdfc771e8725
plan_head_after: 9f02fd6df936d7e76bd798c8fb44ad97f32530f9

tech-stack:
  added: [jsonschema 0.61.2 (dev-only, default-features off)]
  patterns:
    - "Schema per (command, version) in docs/json, pinned by git hash-object --no-filters blob"
    - "Coverage derived from COMMANDS and the directory listing, never a hand list"

key-files:
  created:
    - docs/json/check.v1.schema.json
    - docs/json/README.md
  modified:
    - tests/harness/contract.rs
    - tests/harness/provenance.rs
    - Cargo.toml
    - Cargo.lock
    - deny.toml

key-decisions:
  - "jsonschema 0.61.2 is the validator; deny.toml allows MIT-0 for borrow-or-share 0.2.4 (jsonschema -> fluent-uri -> borrow-or-share), by operator decision 2026-11-18"
  - "A schema file never changes once pinned: any change is a new <name>.v<N+1>.schema.json with its own pin, the old file and pin kept"

patterns-established:
  - "Contract case table: SchemaCase per exit class, matched exhaustively on CommandId"

requirements-completed: [IF-01]

coverage:
  - id: D1
    description: "check --json output validates against check.v1.schema.json for exit 0, exit 1 with all seven problem kinds, exit 2, 3, 4 and the local-branch staleness form"
    requirement: IF-01
    verification:
      - kind: integration
        ref: "tests/harness/contract.rs#contract::every_command_output_matches_its_schema"
        status: pass
    human_judgment: false
  - id: D2
    description: "The schema is strict: a renamed field, an unknown kind and an odd-length hex path each fail; the unmutated object passes"
    requirement: IF-01
    verification:
      - kind: integration
        ref: "tests/harness/contract.rs#contract::schema_rejects_a_renamed_field"
        status: pass
    human_judgment: false
  - id: D3
    description: "Each schema file matches its pinned blob; an edit without a version bump is exactly one mismatch saying a change needs a new version"
    requirement: IF-01
    verification:
      - kind: unit
        ref: "tests/harness/contract.rs#contract::schema_files_match_their_pins"
        status: pass
      - kind: unit
        ref: "tests/harness/contract.rs#contract::a_schema_edit_without_a_version_bump_fails"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every COMMANDS entry has a schema file and a pin; an extra table entry and a stray schema file are each named"
    requirement: IF-01
    verification:
      - kind: unit
        ref: "tests/harness/contract.rs#contract::every_command_has_a_pinned_schema"
        status: pass
      - kind: unit
        ref: "tests/harness/contract.rs#contract::a_command_without_a_pinned_schema_is_named"
        status: pass
    human_judgment: false
  - id: D5
    description: "docs/json/README.md documents the envelope, stdout/stderr rule, byte encoding, no-JSON usage errors, key order and versioning"
    requirement: IF-01
    verification: []
    human_judgment: true
    rationale: "Prose accuracy and readability of a contract document; no test asserts it"
  - id: D6
    description: "The validator is a dev-dependency only and scripts/check.sh deps passes with it"
    verification:
      - kind: other
        ref: "scripts/check.sh deps"
        status: pass
    human_judgment: false

duration: 15min
completed: 2026-11-18
status: complete
---

# Phase 5 Plan 08: JSON schema contract Summary

**Strict draft 2020-12 schema for `check --json`, validated with jsonschema across every exit class, pinned by git blob id, with table-driven coverage that every later command must join.**

## Performance

- **Duration:** 15 min
- **Started:** 2026-11-18T22:00:23Z
- **Completed:** 2026-11-18T22:15:33Z
- **Tasks:** 2
- **Files modified:** 7

## Accomplishments

- `check --json` output in six cases (exit 0, exit 1 carrying all seven problem kinds with `hooks.checked` true, exit 2 usage, exit 3 two candidates, exit 4 `wsx.baseRemote nowhere`, local-branch staleness) validates against `check.v1.schema.json`; each case also asserts its exit code.
- The schema rejects drift: `problems` renamed, an unknown kind, and `{"hex": "abc"}` each fail, and the unmutated object passes.
- `check.v1.schema.json` is pinned at blob `4dddf8bbb55fbcde054f2946b76105761202bfc1`; editing one field name in a scratch copy gives exactly one mismatch saying a change needs a new version. A missing or stray schema file, or a missing pin, is named.

## Task Commits

1. **Task 1 RED: validate --json output against its schema** - `1625073` (test)
2. **Task 1 GREEN: add the check v1 JSON schema and its contract doc** - `79a090f` (docs)
3. **Task 2 RED: pin each schema file and require one per command** - `3922c17` (test)
4. **Task 2 GREEN: check schema pins and command coverage** - `9f02fd6` (test)

## TDD Gate Compliance

- **Task 1 RED:** `contract::schema_rejects_a_renamed_field` failed on its planned `assert!` ("problems renamed problem: the schema accepts {...}") with the placeholder schema `{}`, after the unmutated-object control passed. Classifier: `RED_EVIDENCE_OK` (target/gsd/05-08-red-evidence.json, JUnit report). Semantic assessment: the target compiled and ran, wsx exited 1 on the fixture, and the failure is the planned assertion, not a build, load or fixture fault.
- **Task 2 RED:** `contract::a_schema_edit_without_a_version_bump_fails` failed on its one-mismatch `assert_eq!` (left 0, right 1, "mismatches: []") with `pin_mismatches` stubbed. Classifier: `RED_EVIDENCE_OK` (target/gsd/05-08-red-evidence-2.json). Semantic assessment: the scratch copy was made and the edit changed the file (`assert_ne!` passed); the failure is the planned assertion.
- **GREEN:** both RED targets and the other four D-10 tests pass after `79a090f` and `9f02fd6`. Task 1's GREEN commit is typed `docs` and Task 2's `test`, as the plan specifies, not `feat`: the plan adds only tests, a schema and a doc, no product code. A `^feat\(05-08\)` grep therefore finds nothing, by design.

## Files Created/Modified

- `docs/json/check.v1.schema.json` - check's v1 schema: `oneOf` success or failure, `additionalProperties: false` and every field required throughout
- `docs/json/README.md` - the envelope, stdout and stderr, D-09 bytes, no JSON before a command is recognised, key order, versioning
- `tests/harness/contract.rs` - `contract_with`, `all_kinds`, `bad_base_remote`, `local_branch`, `SchemaCase`, `SCHEMA_PINS`, `pin_mismatches`, `coverage_gaps` and the six tests
- `tests/harness/provenance.rs` - `hash_blob` is now `pub`
- `Cargo.toml`, `Cargo.lock` - jsonschema dev-dependency with its justification comment
- `deny.toml` - MIT-0 in the license allow list, comment naming borrow-or-share 0.2.4 and the chain

## Decisions Made

- Validator: jsonschema 0.61.2, `default-features = false`. `scripts/check.sh deps` failed only on MIT-0 (borrow-or-share 0.2.4); the operator chose to allow MIT-0 rather than fall back to boon. A negative control (MIT-0 removed) made deps fail with `licenses FAILED` naming borrow-or-share, so the allowance is needed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Strictness test mutates the one-problem object, not the clean one**
- **Found during:** Task 1
- **Issue:** The plan says to take the clean case's real object, but that case has an empty `problems` array, so there is no kind or path to mutate.
- **Fix:** `schema_rejects_a_renamed_field` takes the real object of `with_note` (exit 1, one `undeclared_added` problem). The unmutated-object control is kept.
- **Files modified:** tests/harness/contract.rs
- **Committed in:** 1625073

**2. [Rule 3 - Blocking] RED commits gated on fmt and clippy, not on all**
- **Found during:** Tasks 1 and 2
- **Issue:** The project gate (`scripts/check.sh all`) cannot pass on a RED commit, whose target test fails by design.
- **Fix:** `fmt` and `clippy` gated both RED commits; `check.sh all` gated both GREEN commits and ended `==> check.sh: all OK`. The `hash_blob` import moved to the GREEN commit so the RED tree was warning-free.
- **Committed in:** 1625073, 3922c17

**Plan-level notes (no behaviour change):** `all_kinds` adds `src/b.txt` and `shared/x.md` to `origin/develop` in one `advance_remote` rather than two. The local-branch case is a `local_branch` builder pulled out of the 05-07 test, which now calls it. MIT-0 is outside the plan's listed licenses; the operator's checkpoint answer covers it.

---

**Total deviations:** 2 auto-fixed (2 blocking).
**Impact on plan:** None on scope; every truth in the plan's must_haves is covered by the named tests.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Ready for 05-09. A new command must add a `CommandId` arm to `every_command_output_matches_its_schema` (it does not compile otherwise), a schema file and a pin.

## Self-Check: PASSED

- FOUND: docs/json/check.v1.schema.json, docs/json/README.md, tests/harness/contract.rs
- FOUND commits: 1625073, 79a090f, 3922c17, 9f02fd6
- `git status --porcelain -- docs` empty after the tests ran; no all-zero pin left in contract.rs; the pinned blob equals `git hash-object --no-filters docs/json/check.v1.schema.json`
- `scripts/check.sh all` and `scripts/check.sh deps` exit 0 on the final tree

---
*Phase: 05-divergence-check-and-command-contract*
*Completed: 2026-11-18*
