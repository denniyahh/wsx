---
phase: 04-repo-model
plan: 04
subsystem: repo-model
tags: [rust, manifest, parser, grammar, pathspec, proptest, tdd]

requires:
  - phase: 04-repo-model
    provides: "04-03: `path_value` C-quote reading, `encode_path`, `has_edge_whitespace`, `property_config()` and `git_path()` for proptests, `all_bytes_repo`"
  - phase: 04-repo-model
    provides: "04-01: the model contract: `parse`, `covers`, `Entry`, `Manifest`, `ParseError`, `Problem::UnknownKind`"
provides:
  - "`pub fn normalize_path(path) -> Result<Vec<u8>, Problem>`: one trailing `/` stripped, then NUL, empty path, leading `/`, empty component and `.`/`..` component refused (D-14)"
  - "`parse` check order: size limit, then per line CR, comment/empty, keyword, quoting, `normalize_path`, cross-kind overlap"
  - "`Problem` variants `LineEndsInCr`, `EmptyPath`, `AbsolutePath`, `EmptyComponent`, `DotComponent`, `Nul`, `Overlap { other_line, other_text }`, `TooLarge { size }`, each with a Display arm"
  - "`const MAX_MANIFEST_BYTES: usize = 65_536` (private), checked first in `parse`"
  - "`ParseError` Display omits `: {text}` when `text` is empty (only `TooLarge`)"
affects: [04-07, 04-08, 04-09, 04-10, phase-5-check, cut, push-guard]

actuals:
  tokens: 7773
  tasks: 2
  commits: 4
plan_head_before: 4620ddb610f9ca5d5b02667b505f86c77459174b
plan_head_after: 01f6b7dc0a774b9f4bdca1e80be9a8eb26218d72

tech-stack:
  added: []
  patterns:
    - "Per-line reading split into small helpers (`read_line`, `keyword`, `path_value`, `normalize_path`), each under 40 lines, on `&[u8]` only"
    - "Refusal tests assert through `refusal(bytes) -> (line, problem)`; parse tests through `paths(bytes)`; overlap tests assert the full `ParseError` and its exact rendering"

key-files:
  created: []
  modified:
    - src/model/manifest.rs

key-decisions:
  - "`path_value` no longer strips the trailing `/`; `normalize_path` strips exactly one, so `only a//` is EmptyComponent rather than being stripped twice"
  - "The overlap check runs `covers` both ways against every earlier entry of the other kind and reports the first such earlier line; same-kind entries are never compared"
  - "TooLarge's line is computed as 1 + the newlines before byte index 65536, so a newline belongs to the line it ends, as `parse` counts lines"

patterns-established:
  - "Mutation controls run before a GREEN commit: copy the file to scratch, mutate one condition, run the module's tests, restore and `cmp`"

requirements-completed: []

coverage:
  - id: D1
    description: "Every line rule from D-11, D-14 and D-19 has its own named test with the stated result, and a refusal names the line"
    requirement: CONF-01
    verification:
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests:: the 40 tests in Task 1's verify loop (each run alone with --no-tests=fail)"
        status: pass
    human_judgment: false
  - id: D2
    description: "A path declared both only and modified is refused naming both lines, whichever kind covers the other and in either line order; same-kind duplicates, nesting and name extensions parse"
    requirement: CONF-01
    verification:
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::{only_with_modified_of_the_same_path_overlaps,only_with_modified_under_it_overlaps,modified_under_a_later_only_overlaps,only_with_a_trailing_slash_overlaps_modified,modified_with_only_under_it_overlaps,only_under_a_later_modified_overlaps,exact_duplicates_of_one_kind_parse,nested_lines_of_one_kind_parse,name_extension_across_kinds_parses}"
        status: pass
    human_judgment: false
  - id: D3
    description: "A manifest over 64 KiB is refused as TooLarge before any line is read; exactly 64 KiB parses"
    requirement: CONF-01
    verification:
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::{manifest_of_exactly_64_kib_parses,manifest_over_64_kib_is_refused,size_limit_is_checked_before_any_line}"
        status: pass
    human_judgment: false
  - id: D4
    description: "Built-in declarations parse and stay in entries with their line numbers (D-13); the real manifest's shape parses to its two entries"
    requirement: CONF-01
    verification:
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::{declaration_covered_by_a_built_in_parses,parses_the_shape_of_a_real_manifest}"
        status: pass
    human_judgment: false
  - id: D5
    description: "covers matches git's literal pathspec and the path boundary: .planning covers itself and .planning/x, not .planningX or .plannin; a*b covers only itself; NFC and NFD spellings differ"
    requirement: CONF-01
    verification:
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::{path_covers_itself,path_covers_what_is_under_it,shorter_name_is_not_covered,name_extension_is_not_covered,byte_equality_separates_unicode_normal_forms}"
        status: pass
      - kind: integration
        ref: "src/model/manifest.rs#model::manifest::tests::covers_matches_a_literal_pathspec"
        status: pass
    human_judgment: false
  - id: D6
    description: "Prohibition: a manifest line is never silently dropped, truncated or reinterpreted"
    verification:
      - kind: unit
        ref: "Task 1 refusal tests (trailing_text_is_part_of_the_path, inner_spaces_stay_in_the_path, tab_after_the_keyword_is_an_unknown_line, line_ending_in_cr_is_refused)"
        status: pass
    human_judgment: true
    rationale: "The plan marks this prohibition `verification: judgment`. The tests pin each shape they parse; that no other shape is read as less than it says is a review judgment over the grammar, not something a finite set of tests proves."

duration: 8min
completed: 2026-11-08
status: complete
---

# Phase 4 Plan 04: Manifest grammar and matcher Summary

**The complete `.wsx.conf` line grammar with `normalize_path` (D-14), CR and keyword rules (D-11, D-19), a two-way cross-kind overlap check (D-12), a 64 KiB size limit checked before any line, and `covers` shown equal to `git ls-files -- ':(literal)<path>'`, with 60 new named tests.**

## Performance

- **Duration:** about 8 min (16:20Z to 16:28Z)
- **Started:** 2026-11-08T16:20:03Z
- **Completed:** 2026-11-08T16:27:46Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- `parse` now reads each line through `read_line`: a line ending in CR is refused first (comments included), then comments and empty lines are skipped, then `keyword` splits `only`/`modified` (exactly the keyword is `EmptyPath`, anything but one space after it is `UnknownKind`), then `path_value` reads the quoting, then `normalize_path` checks the path.
- `normalize_path` strips one trailing `/` and refuses NUL, an empty path, a leading `/`, an empty component and a `.` or `..` component.
- A manifest over 65,536 bytes is `TooLarge { size }` at the line holding its 65,537th byte, with empty text, before any line is split; `ParseError` renders `line N: <problem>` with no text in that case.
- Cross-kind overlap: each new entry is compared with every earlier entry of the other kind using `covers` in both directions; the first match is `Overlap` naming the earlier line's number and text.

## Task Commits

1. **Task 1 RED: line-rule tests** - `ac73b10` (test)
2. **Task 1 GREEN: enforce the line grammar** - `217c550` (feat)
3. **Task 2 RED: overlap, size and matcher tests** - `8ffe634` (test)
4. **Task 2 GREEN: refuse a path both only and modified, and the size limit** - `01f6b7d` (feat)

No REFACTOR commit: nothing needed cleaning after GREEN.

## TDD Gate Compliance

Both RED commits precede their GREEN commits (`git log --grep='^(test|feat|refactor)\(0*4-0*4\):'` lists ac73b10, 217c550, 8ffe634, 01f6b7d in that order).

- **Task 1 RED:** `scripts/check.sh test -E 'test(=model::manifest::tests::dot_dot_component_is_refused)'` exited 100; `gsd-tools check tdd-red-evidence target/gsd/04-04-red-evidence.json --raw` printed `RED_EVIDENCE_OK` (reason `target_test_failed`). Semantic assessment: the target ran (1 run, 537 skipped) and failed in its own `refusal` helper because `parse` accepted `only a/../b` as the path `a/../b`; the only compiler warning was the expected dead-code warning for the stub `normalize_path`. In the same RED state, all 60 manifest tests ran: 16 failed (every Task 1 refusal rule not yet enforced) and 44 passed (the parse shapes 04-03 already handled, plus 04-03's own tests).
- **Task 2 RED:** `scripts/check.sh test -E 'test(=model::manifest::tests::only_with_modified_under_it_overlaps)'` exited 100; `target/gsd/04-04-red-evidence-2.json` classified `RED_EVIDENCE_OK`. The target failed on its first assertion because `parse` returned `Ok` with both entries. In the same RED state, all 80 manifest tests ran: exactly 8 failed, the six overlap tests, `manifest_over_64_kib_is_refused` (on `Ok`) and `size_limit_is_checked_before_any_line` (got `(1, UnknownKind)`, wanted `(4096, TooLarge { size: 65537 })`), while `manifest_of_exactly_64_kib_parses` passed, as the plan requires.

## Verification run in this session

- Task 1 verify loop (40 named tests, one `scripts/check.sh test -E 'test(=model::manifest::tests::<name>)'` each): all exited 0.
- Task 2 verify loop (20 named tests, same form): all exited 0.
- `scripts/check.sh all` after Task 1 GREEN: exit 0, last line `==> check.sh: all OK`, 477 tests run, 477 passed, 61 skipped.
- `scripts/check.sh all` after Task 2 GREEN: exit 0, last line `==> check.sh: all OK`, 497 tests run, 497 passed, 61 skipped.
- Mutation controls on Task 2's GREEN, each run over all 80 manifest tests and then reverted (`cmp` confirmed the file was restored):
  - overlap checking only "earlier covers later": `modified_under_a_later_only_overlaps` and `only_under_a_later_modified_overlaps` failed;
  - overlap checking only "later covers earlier": `only_with_modified_under_it_overlaps` and `modified_with_only_under_it_overlaps` failed;
  - the limit as `>=` instead of `>`: `manifest_of_exactly_64_kib_parses` failed.
- Acceptance greps: Task 1's `normalize_path`/`LineEndsInCr`/`DotComponent` exit 0; Task 2's `Overlap` and `:(literal)` exit 0, both overlap test names present exit 0, `include_bytes!` absent (exit 1).

## What these checks do not establish

- A parser test proves the shape it parses, not every shape. The prohibition that no line is read as less than it says is a review judgment (D6 above).
- The pathspec differential covers five tracked paths and three entries on git 2.56.0. The proptest is a search over 2,048 or more cases, not a proof.
- A manifest with two bad lines reports only the first, which meets D-11 and D-12; phase 5 can revisit this if `wsx check` wants a full list (RESEARCH Open Question 4).
- The overlap check is quadratic in the entry count; the 64 KiB limit bounds it. The bound's cost (60 ms in release for the slowest of six shapes) is the planner's measurement on a scratch reimplementation, not on this code, and not a proven worst case; no test times it.
- The limit does not stop the whole-blob read in `root_entry`, which is linear in the blob's size (T-04-10's accepted residual).
- The mutation controls cover three mutations of the overlap and limit conditions, not a full `cargo mutants` pass; 04-10 owns the phase's mutation pass.

## Files Created/Modified

- `src/model/manifest.rs`: `normalize_path`, `read_line`, `keyword`, `line_holding`, `overlap`, `MAX_MANIFEST_BYTES`, eight new `Problem` variants, the `ParseError` Display change, and 60 new tests.

## Decisions Made

- `path_value` passes the value on whole, trailing `/` included, and `normalize_path` strips one `/`. The edge-whitespace check still looks past one trailing `/`, so 04-03's `encode_path` round-trip and `carriage_return_before_the_trailing_slash_is_edge_whitespace` are unchanged.
- `TooLarge`'s line is 1 plus the newlines before byte index 65536, so a newline counts toward the line it ends.

## Deviations from Plan

None in behavior. Four small choices inside the plan's remit:

1. **`covers_matches_a_literal_pathspec` pins git's answers as literals, and `.plan` selects `.plan`.** RESEARCH recorded "`.plan` nothing", measured on a repository that did not track `.plan`. This plan tracks `.plan` itself, and git 2.56.0's `:(literal).plan` lists `.plan` (probed in a scratch repository, removed afterwards). So the test expects `[.plan]`, and asserts each entry's expected selection literally as well as equal to `covers`, so agreement on a wrong or empty repository cannot pass. A plain (non-literal) `a*b` pathspec also lists only `a*b/c`, so it was not usable as a negative control and is not in the test.
2. **`quoted_nul_is_refused` uses `only "a\000b"`** (the escape between other bytes) rather than RESEARCH's `only "\000"`. Both give `Nul`, since `normalize_path` checks NUL first.
3. **Some tests assert a little more than the behavior line names:** `shorter_name_is_not_covered` also checks `.planningX`, `exact_duplicates_of_one_kind_parse` and `nested_lines_of_one_kind_parse` cover both kinds, and `only_under_a_later_modified_overlaps` puts a comment and a blank line between its two lines, so the earlier line number reported is 2 and the later is 4.
4. **`encode_path`'s doc comment** now says `parse` "sets aside" the trailing `/` before its edge check rather than "strips it", because stripping moved to `normalize_path`.

## Issues Encountered

None.

## Requirements

CONF-01 is also listed by 04-10, which has not run, so REQUIREMENTS.md is unchanged and `requirements-completed` is empty, as 04-03 did.

## Next Phase Readiness

The manifest grammar is complete for 04-07 (built-ins and `is_personal`) and 04-09 (`ManifestInvalid` through `root_entry`). `TooLarge` reaches the user through `model::Error::ManifestInvalid` with no new `model::Error` variant.

## Self-Check: PASSED

- FOUND: src/model/manifest.rs
- FOUND: ac73b10, 217c550, 8ffe634, 01f6b7d (each an ancestor of HEAD)
- FOUND: target/gsd/04-04-red-evidence.json and target/gsd/04-04-red-evidence-2.json, both re-classified `RED_EVIDENCE_OK`
- Measured commit count from the ledger: 4 (`git rev-list --count 4620ddb..HEAD`)
