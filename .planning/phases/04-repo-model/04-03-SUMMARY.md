---
phase: 04-repo-model
plan: 03
subsystem: repo-model
tags: [rust, gix, quoting, manifest, proptest, tdd]
status: complete

requires:
  - phase: 04-01
    provides: "c_quote in src/git/quote.rs, the model::manifest parse/Entry/Problem/ParseError contract, testing::git_ok and write"
  - phase: 04-05
    provides: "DISCOVERING_SUITES at five suites in src/git/testing.rs (this plan adds no entry)"
provides:
  - "git::c_unquote (re-exported beside c_quote): gix's undo plus a refusal of text after the closing quote"
  - "model::manifest::encode_path: c_quote plus double quotes around a bare path whose edge, one trailing / set aside, is A6 whitespace"
  - "parse's path step (private path_value): C-quoted values through c_unquote, bare values refused on edge whitespace, one trailing / stripped"
  - "Problem::EdgeWhitespace and Problem::BadQuoting { reason } with Display arms"
  - "testing::all_bytes_repo (270 names, self-checked against git ls-files -z) and testing::records"
  - "property_config() and the weighted git_path() strategy in the manifest test module, for 04-04's property"
affects: [04-04, 04-10, phase-12-init-migrate]

actuals:
  tokens: 7307
  tasks: 2
  commits: 4
plan_head_before: 1d899803b0a03575ae66589ae2d19e7a418a4e45
plan_head_after: cac87db1d21a7b8ff37bd5b474af6af5d175373f

tech-stack:
  added:
    - "proptest 1.12.0 (dev-only, default-features = false, features = [std]); lock adds chacha20, num-traits, proptest, rand, rand_core, rand_xorshift, regex-syntax, unarray"
  patterns:
    - "all-bytes differential: the fixture's names from git ls-files -z paired by index with git ls-files lines"
    - "every proptest! block takes property_config(); property_runs_its_configured_cases is the evidence the cases ran"

key-files:
  created: []
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/git/quote.rs
    - src/git/mod.rs
    - src/git/testing.rs
    - src/model/manifest.rs

key-decisions:
  - "Added text_after_a_quoted_path_is_bad_quoting (not in the plan's sixteen): no planned test constructed Problem::BadQuoting, and only \"a\"b is threat T-04-06 at the manifest level"
  - "all_bytes_repo fails unless git ls-files -z lists exactly the 270 names written, so a name the filesystem or git changed cannot pass unnoticed"
  - "testing::records (split on a separator, drop the trailing empty record) is shared by the quote and manifest differentials rather than written twice"

patterns-established:
  - "a differential test reports the count of mismatches and the first five, not the whole list"

requirements-completed: []

coverage:
  - id: D1
    description: "c_quote and c_unquote equal git 2.56.0 on every byte value 1-255 but / plus 16 edge names; the comparison is not vacuous"
    requirement: CONF-01
    verification:
      - kind: integration
        ref: "src/git/quote.rs#git::quote::tests::c_quote_matches_git_on_every_byte"
        status: pass
      - kind: integration
        ref: "src/git/quote.rs#git::quote::tests::a_changed_byte_is_detected"
        status: pass
      - kind: integration
        ref: "src/git/quote.rs#git::quote::tests::c_unquote_reverses_git_output_on_every_byte"
        status: pass
    human_judgment: false
  - id: D2
    description: "Malformed C-quoted tokens are refused: octal past 377, text after the closing quote, unterminated, unknown escape; 377 gives 0xFF"
    requirement: CONF-01
    verification:
      - kind: unit
        ref: "src/git/quote.rs#git::quote::tests::octal_escape_at_the_byte_limit_parses"
        status: pass
      - kind: unit
        ref: "src/git/quote.rs#git::quote::tests::octal_escape_past_the_byte_limit_is_refused"
        status: pass
      - kind: unit
        ref: "src/git/quote.rs#git::quote::tests::c_unquote_refuses_text_after_the_closing_quote"
        status: pass
      - kind: unit
        ref: "src/git/quote.rs#git::quote::tests::c_unquote_refuses_an_unterminated_token"
        status: pass
      - kind: unit
        ref: "src/git/quote.rs#git::quote::tests::c_unquote_refuses_an_unknown_escape"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::text_after_a_quoted_path_is_bad_quoting"
        status: pass
    human_judgment: false
  - id: D3
    description: "Any git path round-trips through encode_path and parse (Success Criterion 1), with a control showing the property catches a bare-edge encoder, and a run with no cases is a named failure"
    requirement: CONF-01
    verification:
      - kind: property
        ref: "src/model/manifest.rs#model::manifest::tests::any_git_path_round_trips"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::property_runs_its_configured_cases"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::encoded_edge_spaces_are_quoted"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::encoded_space_before_a_trailing_slash_round_trips"
        status: pass
      - kind: integration
        ref: "src/model/manifest.rs#model::manifest::tests::git_quoted_output_parses_to_git_raw_names"
        status: pass
    human_judgment: false
  - id: D4
    description: "D-19 with A6: everything after the keyword and one space is the path; space, tab, CR, VT and FF bare on an edge are refused; a control byte on an edge is not; a quoted path's trailing / is stripped"
    requirement: CONF-01
    verification:
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::space_after_the_keyword_starts_the_path"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::trailing_slash_is_stripped_from_a_quoted_path"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::leading_tab_is_edge_whitespace"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::trailing_tab_is_edge_whitespace"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::leading_vertical_tab_is_edge_whitespace"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::trailing_vertical_tab_is_edge_whitespace"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::leading_form_feed_is_edge_whitespace"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::trailing_form_feed_is_edge_whitespace"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::leading_carriage_return_is_edge_whitespace"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::carriage_return_before_the_trailing_slash_is_edge_whitespace"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::control_byte_on_the_edge_parses_bare"
        status: pass
    human_judgment: false

metrics:
  duration: "11 min"
  completed: 2026-11-08
---

# Phase 4 Plan 03: C-quoted manifest paths Summary

Manifest paths now round-trip on every byte: `c_unquote` is gix's `undo` plus a refusal of text
after the closing quote, proven equal to git 2.56.0 on all 255 byte values; `encode_path` adds
the quotes git leaves off edge spaces; and a 2048-case proptest, with a control and a case-count
guard, shows any git path written through `encode_path` parses back to the same bytes.

## What was built

- `src/git/quote.rs`: `c_unquote(token) -> Result<Vec<u8>, String>`. A token not starting with
  `"` comes back unchanged; otherwise it must be exactly one C-quoted string. `"a"b`, which git
  accepts as `a`, is `Err("text after the closing quote")`.
- `src/git/mod.rs`: `pub use quote::{c_quote, c_unquote};`, landed in the GREEN commit beside
  `parse`, its first caller.
- `src/model/manifest.rs`: `encode_path`, the private `path_value` / `has_edge_whitespace` /
  `is_edge_whitespace` (A6's five bytes, one definition shared by both directions),
  `Problem::EdgeWhitespace` and `Problem::BadQuoting { reason }` with the planned Display text.
- `src/git/testing.rs`: `all_bytes_repo` (270 staged names, self-checked against
  `git ls-files -z`) and `records`.
- `Cargo.toml` / `Cargo.lock`: proptest 1.12.0, dev-only, with its justification comment.

## TDD record

### Task 1 RED: `git::quote::tests::c_unquote_reverses_git_output_on_every_byte`

`gsd-tools check tdd-red-evidence target/gsd/04-03-red-evidence.json --raw` printed
`RED_EVIDENCE_OK` (`target_test_failed`, exit 100, JUnit report, 1 test, 1 fail).
Semantic assessment: the target compiled and ran, and the fixture's own `ls-files -z` check
passed. It failed on its planned comparison: 171 of 270 names differ (the ones git quotes), first
at index 0, `"\tleadtab"` kept quoted by the stub. At RED the suite showed six of the eight new
tests failing and the two `c_quote` tests passing, as the plan expected (`c_quote` came from 04-01;
`a_changed_byte_is_detected` shows the comparison bites).

### Task 1 GREEN

`c_unquote` per RESEARCH Pattern 5. Gate: `scripts/check.sh all` exit 0, last line
`==> check.sh: all OK`, 400 tests run and passed, 60 skipped. Commit `a24de6c`. I also checked
the doc comment's git claims against git 2.56.0 here with `git check-ignore --stdin`: `"a"b`
matched pattern `a` (exit 0), `"\400"` exited 128, and `"abc` and `"a\q"` exited 128 with
`line is badly quoted`.

### Task 2 RED: `model::manifest::tests::any_git_path_round_trips`

`gsd-tools check tdd-red-evidence target/gsd/04-03-red-evidence-2.json --raw` printed
`RED_EVIDENCE_OK` (`target_test_failed`, exit 100).
Semantic assessment: the target compiled and ran (15 successes before the failure). It failed on
the round trip. While shrinking, it panicked on the planned `.expect` (a raw newline split the
line, `line 2 UnknownKind`). The minimal input `path = [10], trailing_slash = false` failed the path
equality (`left: []`, `right: [10]`). Both come from the stub `encode_path`, which wrote the
newline raw. At RED, 14 of the 17 new tests failed. `property_runs_its_configured_cases`,
`space_after_the_keyword_starts_the_path` and `control_byte_on_the_edge_parses_bare` passed, as
pins of behaviour that already held.

### Task 2 GREEN

`encode_path`, `path_value`, the re-export. Gate: `scripts/check.sh all` exit 0, last line
`==> check.sh: all OK`, 417 tests run and passed, 60 skipped; `scripts/check.sh deps` exit 0 with
`advisories ok, bans ok, licenses ok, sources ok`. Commit `cac87db`.

No REFACTOR commit.

## Controls (Task 2)

| Control | Command shape | Result |
|---|---|---|
| Bare-edge encoder (scratch copy, `encode_path` returns `c_quote(path)`) | plan's second `<automated>`, pinned nextest 0.9.148 from `.tools/bin` | nextest status **100**; output has `any_git_path_round_trips`, **`minimal failing input`** (`path = [32], trailing_slash = false`) and **`EdgeWhitespace`** (`Test failed: an encoded path parses: ParseError { line: 1, text: [111, 110, 108, 121, 32, 32], problem: EdgeWhitespace }`); predicate exit 0. The real `parse` produced the planned text, so the predicate was not changed |
| Correct encoder (opposite case) | the per-test loop | `any_git_path_round_trips` passed |
| `PROPTEST_CASES=0` | `scripts/check.sh test -E 'test(=model::manifest::tests::property_runs_its_configured_cases)'` | status **100**, message `a property under property_config() ran 0 cases, below 2048; PROPTEST_CASES lowers the count, and is Some("0") here` |
| `PROPTEST_CASES=2047` (extra) | same | status 100, `ran 2047 cases, below 2048` |
| `PROPTEST_CASES=4096` | same | status 0 |
| cargo-deny ban on `unarray` (scratch copy) | plan's `deny.XXXXXX` `<automated>`, pinned cargo-deny 0.20.2 | exit **2**, `error[banned]: crate 'unarray = 0.1.4' is explicitly banned`; predicate exit 0 |

Acceptance commands: all seven of Task 2's `rg`/lock checks exited 0. That includes the lock diff
against `6946ae9`, which only adds lines, and the eight new crate names. Task 1's
`pub fn c_unquote` / `consumed` check also exited 0.

## Limits (what these checks do not establish)

- 2048 random cases per property search well for encoding defects but do not prove the encoding
  correct. The generator draws from all of bytes 1-255 except `/`, weighted toward printable bytes
  and spaces, but a given run is not guaranteed to hit every byte. The all-bytes differential is
  what covers each value. The weighting's 0 misses in 200 runs was measured at planning on a stub
  parser. In this session the control ran once against the real `parse`, and one run is a weak
  bound.
- `property_runs_its_configured_cases` counts cases and cannot show they were useful. It does not
  see a fixed `PROPTEST_RNG_SEED`, which keeps 2048 cases but repeats them.
- Each A6 test pins one line per byte and edge, not every path that holds such a byte there. How
  often the property now generates edge whitespace before a trailing `/` was not measured.
  `encoded_space_before_a_trailing_slash_round_trips` pins that shape.
- Parity was measured on git 2.56.0 only (the host's). The pinned image's git was the same version
  at planning; that was not re-checked here, and the container gate was not run in this plan.
- cargo-deny trusts each crate's declared license.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing test] `Problem::BadQuoting` had no test**
- **Found during:** Task 2 RED
- **Issue:** None of the sixteen planned tests builds `BadQuoting`, and 04-04's list does not
  either (`rg BadQuoting` over the later plans is empty). Threat T-04-06 (`only "a"b` read as `a`)
  was tested only inside `c_unquote`.
- **Fix:** `text_after_a_quoted_path_is_bad_quoting` asserts that `only "a"b` gives the whole
  `ParseError` at line 1 with `BadQuoting { reason: "text after the closing quote" }`. It failed
  at RED and passes at GREEN.
- **Files modified:** src/model/manifest.rs
- **Commits:** 92cf8f7 (test), cac87db (passes)

**2. [Rule 3 - Shared helper] `testing::records`**
- **Found during:** Task 1
- **Issue:** The quote and manifest differentials both split `git ls-files` output, so each would
  have needed its own copy of the split.
- **Fix:** One `pub fn records(output, separator)` in `src/git/testing.rs`, the file the plan
  already modifies for `all_bytes_repo`. `all_bytes_repo` also returns `Err` unless
  `git ls-files -z` lists exactly the names it wrote.
- **Commit:** 1752d23

Otherwise the plan ran as written. 17 tests were added in Task 2, not 16; the extra one is item 1.

## TDD Gate Compliance

`test(04-03)` 1752d23, then `feat(04-03)` a24de6c; `test(04-03)` 92cf8f7, then `feat(04-03)`
cac87db. Both RED records returned `RED_EVIDENCE_OK`. No violations.

## Requirements

CONF-01 is also listed by 04-04 and 04-10, which have no SUMMARY yet, so REQUIREMENTS.md was left
unchanged. This plan delivers CONF-01's quoting and round-trip half.

## Known Stubs

None. The RED stubs (`c_unquote` and `encode_path` returning their input) were replaced in the
GREEN commits.

## Self-Check: PASSED

- Files exist: src/git/quote.rs, src/model/manifest.rs, src/git/testing.rs, Cargo.toml, and
  RED evidence target/gsd/04-03-red-evidence.json and target/gsd/04-03-red-evidence-2.json
- Commits on HEAD: 1752d23, a24de6c, 92cf8f7, cac87db
