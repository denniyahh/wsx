---
phase: 04-repo-model
plan: 01
subsystem: repo-model
tags: [rust, gix, model, manifest, refs, quoting, tdd]

requires:
  - phase: 03.1
    provides: "Repo::peel_commit and private peeled_commit (hex decode, no revspec parse), clippy.toml revspec ban, testing::describe_shaped_ref, DISCOVERING_SUITES with its scan test"
provides:
  - "model::resolve, RepoModel, Model, Integration, Model::personal_paths, Model::is_personal"
  - "the whole model::Error interface (16 variants) with pinned messages"
  - "manifest::parse, manifest::covers, Manifest, Entry, EntryKind, ParseError, Problem::UnknownKind"
  - "Repo::find_ref (exact full-name read), Repo::ref_commit (read-once, git's symref depth), Repo::root_entry (object database only), is_branch_name"
  - "Repo::local_config_values, Repo::remote_configured (narrow forms; 04-05 and 04-06 replace them)"
  - "git::c_quote (display) and git::shell_word (pasteable command argument)"
  - "shared model-test fixture repo_with_remote / set_config / tip"
  - "model::tests:: under the runner-GIT_DIR check; scan reads src/model"
affects: [04-03, 04-04, 04-05, 04-06, 04-07, 04-08, 04-09, 04-10, 04-11, phase-5-check]

actuals:
  tokens: 19167
  tasks: 2
  commits: 4
plan_head_before: decd979ae2fefd54a6aaee815135190d1ecc9c82
plan_head_after: 6b23bdfa7c123ed0e1efa1451771ef6d6f3f9917

tech-stack:
  added: []
  patterns:
    - "exact ref read: FullName::try_from + try_find_reference, then compare the returned name"
    - "a ref's commit is the id its one read returned, peeled; never the name resolved again"
    - "prose byte fields through c_quote; command-line byte fields through shell_word or a placeholder"

key-files:
  created:
    - src/model/mod.rs
    - src/model/manifest.rs
    - src/git/config.rs
    - src/git/quote.rs
  modified:
    - src/main.rs
    - src/git/mod.rs
    - src/git/refs.rs
    - src/git/testing.rs

key-decisions:
  - "find_ref treats a ref gix returns under another name as absent: gix 0.90.0 falls back to .git/heads/<full> (and tags/, remotes/), which git never reads; the test pins the raw lookup's different answer as its control"
  - "ref_commit follows symbolic refs through find_ref only, at most 5 reads (git 2.56.0's measured depth: 4 hops resolve, 5 refused), and peels the id each read returned"
  - "shell_word refuses (None) empty values, bytes outside 0x21-0x7e and backslash; callers print a placeholder (<remote>, <ref>) instead of a quoted form a shell might misread"
  - "tree.find_entry takes the &[u8] name directly: &BStr does not implement PartialEq<BStr> in gix 0.90.0, so RESEARCH Pattern 6's .as_bstr() form does not compile"

patterns-established:
  - "model tests start with rerun_scrubbed and build fixtures with testing::git_ok; config is set before Repo::discover"
  - "each model::Error message pinned by one named test asserting the whole string"

requirements-completed: [CONF-02, CONF-07]

coverage:
  - id: D1
    description: "A configured repo (local wsx.sourceBranch) resolves one model whose personal-path list comes from .wsx.conf at the workspace tip in the object database, and the model moves when that tip moves"
    requirement: CONF-07
    verification:
      - kind: integration
        ref: "src/model/mod.rs#model::tests::configured_repo_resolves_from_the_object_database"
        status: pass
      - kind: integration
        ref: "src/model/mod.rs#model::tests::model_moves_with_the_workspace_tip"
        status: pass
      - kind: integration
        ref: "src/model/mod.rs#model::tests::no_source_branch_and_no_candidate_is_not_configured"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::only_line_keeps_its_number_and_text"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::unknown_kind_names_the_line"
        status: pass
    human_judgment: false
  - id: D2
    description: "Exact ref reads: find_ref refuses a ref gix finds under another name, ref_commit reads the commit a describe-shaped ref holds and follows symbolic refs exactly as far as git does; base and push remote default to origin"
    requirement: CONF-02
    verification:
      - kind: integration
        ref: "src/git/refs.rs#git::refs::tests::find_ref_tells_direct_from_symbolic"
        status: pass
      - kind: integration
        ref: "src/git/refs.rs#git::refs::tests::find_ref_refuses_a_ref_read_under_another_name"
        status: pass
      - kind: integration
        ref: "src/git/refs.rs#git::refs::tests::ref_commit_reads_the_commit_a_describe_shaped_ref_holds"
        status: pass
      - kind: integration
        ref: "src/git/refs.rs#git::refs::tests::ref_commit_follows_symbolic_refs_as_far_as_git_does"
        status: pass
    human_judgment: false
  - id: D3
    description: "The whole model::Error interface renders a message naming cause and fix, with escape bytes C-quoted"
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests (18 message tests, escape_bytes_in_a_value_render_quoted)"
        status: pass
      - kind: unit
        ref: "src/model/manifest.rs#model::manifest::tests::escape_bytes_in_a_line_render_quoted"
        status: pass
    human_judgment: false
  - id: D4
    description: "Printed fix commands are safe to paste: byte arguments go through shell_word or a placeholder"
    verification:
      - kind: integration
        ref: "src/git/quote.rs#git::quote::tests::pasted_command_removes_a_ref_named_with_shell_syntax"
        status: pass
      - kind: integration
        ref: "src/git/quote.rs#git::quote::tests::display_quoting_leaves_a_command_substitution_live"
        status: pass
      - kind: unit
        ref: "src/git/quote.rs#git::quote::tests::shell_word_quotes_what_a_shell_would_read"
        status: pass
      - kind: unit
        ref: "src/git/quote.rs#git::quote::tests::shell_word_has_no_form_for_bytes_a_quote_cannot_carry"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::command_lines_quote_a_value_for_the_shell"
        status: pass
    human_judgment: false
  - id: D5
    description: "model::tests:: runs under a decoy runner GIT_DIR, and the scan keeps DISCOVERING_SUITES equal to the modules under src/git and src/model that call rerun_scrubbed"
    verification:
      - kind: integration
        ref: "src/git/testing.rs#git::testing::discovering_tests_pass_with_a_runner_git_dir"
        status: pass
      - kind: unit
        ref: "src/git/testing.rs#git::testing::discovering_suites_are_the_modules_that_rerun_scrubbed"
        status: pass
      - kind: other
        ref: "scratch-copy control: guard stripped from src/model/mod.rs, nextest exit 100"
        status: pass
    human_judgment: false

duration: 17min
completed: 2026-11-08
status: complete
---

# Phase 4 Plan 01: Repo model tracer slice Summary

**`model::resolve` reads local `wsx.sourceBranch`, the `origin` remote, an exact `refs/remotes/origin/develop` read, and `.wsx.conf` from the workspace tip's root tree in the object database, and returns a `Configured` model whose tips equal `git show-ref`; the full 16-variant `model::Error` renders escaped prose and paste-safe fix commands.**

## Performance

- **Duration:** about 17 min
- **Started:** 2026-11-08T15:11:34Z
- **Completed:** 2026-11-08T15:28Z
- **Tasks:** 2 (each RED then GREEN)
- **Files modified:** 8 (4 created, 4 modified)

## Accomplishments

- One end-to-end path, local config to personal-path list, agrees with git on a real clone where `main`, `develop` and `next` hold three distinct commits; the control commits a new manifest and the model follows it.
- `find_ref` reads a ref only under its exact name; `ref_commit` reads each ref once and peels the id it got, so a describe-shaped ref resolves to the commit it holds (gix's own revspec parser, run as the control, gives the other commit).
- Every `model::Error` variant later plans construct exists, with its message pinned by a named test; an ESC byte renders as `\033`.
- `shell_word` makes the model's printed commands safe to paste; the display-quoted form is shown to run a command substitution.
- `model::tests::` is held to the runner-`GIT_DIR` check, and the scan test now reads `src/model`.

## Task Commits

1. **Task 1 RED:** `bdcc0a8` test(04-01): add failing configured-repo model test
2. **Task 1 GREEN:** `5f65fc8` feat(04-01): resolve one model from the tip of the workspace branch
3. **Task 2 RED:** `87a0b6f` test(04-01): add failing model error message tests
4. **Task 2 GREEN:** `6b23bdf` feat(04-01): render every model refusal with its cause and fix

No REFACTOR commits.

## TDD Evidence

**Task 1 RED:** `gsd_run check tdd-red-evidence target/gsd/04-01-red-evidence.json --raw` printed `"verdict": "RED_EVIDENCE_OK"` (`reason: target_test_failed`) for `model::tests::configured_repo_resolves_from_the_object_database`, nextest exit 100. Semantic assessment: the test failed in its `configured()` helper with `resolve: expected RepoModel::Configured, got Ok(NotConfigured { inferred: Err(WorkspaceCandidates { candidates: [] }) })`, after the fixture's three distinct rung tips had been checked, so it was the planned assertion, not a fixture fault. The other seven new Task 1 tests also failed on their own assertions against the stubs; `no_source_branch_and_no_candidate_is_not_configured` passed on the stub, as expected, since its answer is the stub's answer.

**Task 2 RED:** `gsd_run check tdd-red-evidence target/gsd/04-01-red-evidence-2.json --raw` printed `"verdict": "RED_EVIDENCE_OK"` (`reason: target_test_failed`) for `model::tests::escape_bytes_in_a_value_render_quoted`, nextest exit 100, failing on `a raw ESC byte reached the message: "wsx.sourceBranch = a\u{1b}[31mb is not a valid branch name"`. Of the 26 Task 2 tests run against the stubs, 15 failed: the nine new-variant message tests, both escape tests, `command_lines_quote_a_value_for_the_shell`, both `shell_word` table tests and `pasted_command_removes_a_ref_named_with_shell_syntax`, the last on its exit-status assertion (`error: refusing to update ref with bad name 'refs/replace/'`, exit 1, because the bare line ran the substitution). The display-quoting control and Task 1's message tests passed, as planned.

**Gate order:** each `test(04-01)` commit precedes its `feat(04-01)` commit (`bdcc0a8` before `5f65fc8`, `87a0b6f` before `6b23bdf`).

## Measurements the plan asked for

- **Scan test on the array change alone** (`DISCOVERING_SUITES` at `[&str; 4]`, scan still reading only `src/git`): nextest exit 100, `assertion 'left == right' failed: modules calling rerun_scrubbed() against DISCOVERING_SUITES`, left `{"git::reads::tests::", "git::refs::tests::", "git::repo::tests::"}`, right the same plus `"model::tests::"`. After extending the scan to `src/model` it passed.
- **Scratch-copy control** (guard stripped from `src/model/mod.rs`, pinned cargo-nextest 0.9.148 verified via `install-tools.sh verify-cargo`): nextest exit 100; the output held `with GIT_DIR=<decoy>/.git the suite model::tests:: exited Some(101), counts (passed, failed) Some((21, 2))`, then `test model::tests::model_moves_with_the_workspace_tip ... FAILED` and `test model::tests::configured_repo_resolves_from_the_object_database ... FAILED`. The control's three conditions held (exit 0 from the check).
- **Paste tests:** `PASS [0.047s] (1/1) wsx::bin/wsx git::quote::tests::pasted_command_removes_a_ref_named_with_shell_syntax` and `PASS [0.026s] (1/1) wsx::bin/wsx git::quote::tests::display_quoting_leaves_a_command_substitution_live`. `/bin/sh` on this host is a symlink to bash (GNU bash 5.3.9), which runs in POSIX mode under that name. fish and bash were measured at planning, dash (the pinned image's `/bin/sh`) by the checker and on 2026-11-05; zsh was not run.
- **Source lint on `src/git/refs.rs`:** exit 0. Negative control: a scratch copy with a second `find_ref` call in `ref_commit` failed with `ref_commit has 2 find_ref call sites, not 1`.

## Gate runs

| Command | Exit |
|---|---|
| `scripts/check.sh all` at the baseline `decd979` | 0 (333 tests passed) |
| `scripts/check.sh all` after Task 1 GREEN, first run | 101 (clippy `write_with_newline` in my new code; fixed before commit) |
| `scripts/check.sh all` after Task 1 GREEN, rerun | 0, last line `==> check.sh: all OK` (342 passed) |
| the nine Task 1 `<verify>` tests, one `-E 'test(=...)'` per command | 0 each |
| `scripts/check.sh all` after Task 2 GREEN | 0, last line `==> check.sh: all OK` (367 passed) |
| Task 2 `<verify>` loops, 25 tests one per command | 0 |
| `discovering_tests_pass_with_a_runner_git_dir && discovering_suites_are_the_modules_that_rerun_scrubbed` | 0 |

## Files Created/Modified

- `src/model/mod.rs`: `RepoModel`, `Model`, `Integration`, `Error` (16 variants), `resolve`, `single`, `build`, `integration`, `shown`, `command_word`, the fixture, 23 tests.
- `src/model/manifest.rs`: the manifest types, `parse`, `covers`, 3 tests.
- `src/git/config.rs`: `local_config_values`, `remote_configured`.
- `src/git/quote.rs`: `c_quote`, `shell_word`, 4 tests.
- `src/git/refs.rs`: `RefTarget`, `RootEntry`, `SYMREF_MAX_DEPTH`, `is_branch_name`, `find_ref`, `ref_commit`, `root_entry`, extended module doc, 4 tests.
- `src/git/mod.rs`: `mod config; mod quote;`, `pub(crate) mod testing`, re-exports of `c_quote`, `shell_word`, `RootEntry`, `is_branch_name`, `Repo`.
- `src/git/testing.rs`: `DISCOVERING_SUITES` gains `model::tests::`; the scan reads `src/model`.
- `src/main.rs`: `mod model;`.

## Decisions Made

- `tree.find_entry(name)` with the raw `&[u8]`: RESEARCH Pattern 6's `.as_bstr()` argument does not compile on gix 0.90.0 (`&BStr` has no `PartialEq<BStr>`). Recorded as the plan asked.
- `ref_commit_follows_symbolic_refs_as_far_as_git_does` builds on the existing `tagged_repo` fixture plus `git branch workspace/dev`; the plan named no fixture for it.
- `worktree_dir_invalid_message_names_the_value` uses `Problem::UnknownKind`, the only `Problem` variant before 04-03 and 04-04 add the path problems.
- The new tests in `src/git/refs.rs` use a small `cloned_repo` fixture local to that test module; the model's `repo_with_remote` lives in `model::tests` as the plan specifies.

## Deviations from Plan

**1. [Rule 3 - Blocking] The scratch-copy control ran without its `trap 'rm -rf "$S" "$B"' EXIT`**
- **Found during:** Task 2 step 3
- **Issue:** this session's command-safety check refuses an `rm -rf` on variable paths inside a shell script.
- **Fix:** ran the control's commands unchanged otherwise, then removed the scratch copy and `target/gsd/discover-control` by their literal paths; both are gone.
- **Files modified:** none
- **Committed in:** n/a

**Total deviations:** 1 (procedure only). **Impact:** none on the code or on what the control shows.

## Known Stubs

| File | Line | Stub | Resolved by |
|---|---|---|---|
| `src/model/mod.rs` | 296 | `resolve` with no `wsx.sourceBranch` returns `NotConfigured { inferred: Err(WorkspaceCandidates { candidates: [] }) }` without looking for candidates | 04-08 (candidate selection, D-17, D-21); recorded in `.planning/WINDOWS.md` |

These reads are deliberately narrower than the phase's final behaviour, each replaced behind its own RED test by the named plan: `single` takes the last value (04-05, D-18), `local_config_values` has no level filter (04-05), `remote_configured` uses gix's `remote_names()` (04-06), `root_entry` has no mode check (04-09), `integration` tries only `develop` and follows a symbolic one (04-07).

## Limits of what this shows

- The tracer proves one path on one fixture. It does not prove the other chain rungs, config scope, the fallback or any refusal; those are 04-03 to 04-09 and 04-11. `model` has no production caller until phase 5.
- `ref_commit_reads_the_commit_a_describe_shaped_ref_holds` shows `ref_commit` does not resolve a name through gix's revspec parser, on one describe-shaped name, direct and symbolic. That it reads each ref once, so a ref changed between two reads cannot redirect it, has no runtime test and is accepted as T-04-37. The source check bounds call sites in `ref_commit`, not how often one runs; a read moved into a helper passes it, so it is a lint, not evidence.
- `find_ref`'s name comparison does not see a ref namespace. Until 04-05 Task 3 lands, a `Repo` opened with `GIT_NAMESPACE` or `gitoxide.core.refsNamespace` set reads refs inside the namespace.
- The paste test ran under bash as `/bin/sh` here; dash runs it in 04-10's container gate. A value `shell_word` cannot carry prints a placeholder, so that command must be edited before it runs.
- The scan test's extension shows a module under `src/git` or `src/model` whose tests call `rerun_scrubbed` is listed. It does not see a discovering test that omits the guard; only the runner test and its control catch that.
- In the control run, `no_source_branch_and_no_candidate_is_not_configured` passed even with its guard stripped and `GIT_DIR` aimed at the decoy: the decoy has no `wsx.sourceBranch` either, so it gets the same `NotConfigured` answer. That test cannot tell which repository it resolved.

## Issues Encountered

- clippy `write_with_newline` on the first Task 1 GREEN gate run; changed to `writeln!` before the commit.

## User Setup Required

None.

## Next Phase Readiness

Ready for 04-02 and the expansion plans: the `RepoModel`/`Model`/`Error` names, the fixture and the git-layer reads they build on are in place.

## Self-Check: PASSED

- Files: `src/model/mod.rs`, `src/model/manifest.rs`, `src/git/config.rs`, `src/git/quote.rs`, `src/git/refs.rs`, `src/git/testing.rs`, `src/git/mod.rs`, `src/main.rs` all FOUND.
- Commits: `bdcc0a8`, `5f65fc8`, `87a0b6f`, `6b23bdf` all FOUND as ancestors of HEAD; `check evaluation-scope --plan 04-01 --commits-only` resolved these four.
- Task 1 and Task 2 acceptance criteria re-run: all exit as required (`#[allow(` search exits 1).
