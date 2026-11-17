---
phase: 04-repo-model
plan: 12
subsystem: git-layer
tags: [gix, refs-replace, replace-refs, fail-closed, manifest, mutation-testing]
status: complete

requires:
  - phase: 04-repo-model
    provides: "04-11 is_replacement_refusal and its readers in build and require; 04-08 build and require; 04-05 state_child"
provides:
  - "git::Error::ReplaceRefsPresent { path, packed }, ReplaceBaseSet { name, value }, ReplaceRefsUnreadable { path, source }, their messages, and is_replacement_refusal listing exactly these three"
  - "Repo::refuse_replace_refs (variable, key, loose, packed, in that order) with private holds_an_entry and packed_refs_name_the_base"
  - "refuse_replace_base_env, the first statement of Repo::discover, before gix opens the repository"
  - "model::build runs the guard right after the WorkspaceBranchMissing check, before any object read; root_entry keeps its call"
affects: [phase-05-check, phase-06-guard, phase-09-cut]

actuals:
  tokens: 18331
  tasks: 3
  commits: 4
plan_head_before: 463ab95961cc867979515bbf7453b436ebaff891
plan_head_after: 1ab5bb14177be1366d45fef2d01dfcaa17f3086a

tech-stack:
  added: []
  patterns:
    - "Refuse replace refs by presence: one directory entry or one substring of packed-refs, no ref name parsed and no record decoded"

key-files:
  created:
    - .planning/phases/04-repo-model/deferred-items.md
  modified:
    - src/git/mod.rs
    - src/git/refs.rs
    - src/git/repo.rs
    - src/model/mod.rs
    - .planning/STATE.md

key-decisions:
  - "A GIT_REPLACE_REF_BASE of exactly refs/replace/ passes, pinned by replace_base_variable_set_to_the_default_resolves (the one survivor of the first mutation pass)"
  - "The RUSTSEC-2026-0486 advisory on rustix 1.1.5 is left for the operator: a lockfile update is a package-manager change the executor does not make on its own"

patterns-established:
  - "assert_refused_as_loose / assert_refused_as_packed / assert_refused_as_base: one refusal assertion per store, each also checking the message blames the replace refs and never says cannot be read"

requirements-completed: [CONF-07]

coverage:
  - id: D1
    description: "Any replace ref, of any shape, loose or packed, stops resolve in a configured repo before any object read, with git's own answer asserted first"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::nested_replacement_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::packed_nested_replacement_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::symlinked_replace_ref_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::crlf_packed_replace_record_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::unsorted_packed_replace_record_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::single_replacement_fails_closed"
        status: pass
    human_judgment: false
  - id: D2
    description: "Another replace base is refused, GIT_REPLACE_REF_BASE before gix opens the repository, the key by the guard; the default value passes"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replace_base_variable_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replace_base_variable_gix_cannot_open_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replace_base_key_alone_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replace_base_variable_set_to_the_default_resolves"
        status: pass
    human_judgment: false
  - id: D3
    description: "The guard runs in build before the tip is peeled, shown by a test that fails with build's call deleted"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replaced_workspace_commit_is_refused_before_it_is_peeled"
        status: pass
      - kind: command
        ref: "Task 1 placement block -> target/gsd/04-12-placement.log"
        status: pass
    human_judgment: false
  - id: D4
    description: "Failed reads of the stores blame the replace refs; empty refs/replace/, packed-refs without a replace ref and an unconfigured repo resolve as before"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::unreadable_replace_dir_blames_the_replace_refs"
        status: pass
      - kind: unit
        ref: "src/git/refs.rs#git::refs::tests::packed_refs_that_fail_to_read_are_unreadable"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replace_dir_left_empty_resolves"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::packed_refs_without_a_replace_ref_resolve"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replace_ref_leaves_a_not_configured_repo_not_configured"
        status: pass
    human_judgment: false

duration: 32 min
completed: 2026-11-17
---

# Phase 4 Plan 12: Refuse any replace ref by presence Summary

Any entry under `refs/replace/`, any `packed-refs` text naming it, or a replace base other than
`refs/replace/` now stops the model in a configured repo before any object is read, with errors that
blame the replace refs and name the remedy; 04-11's per-shape guard is gone (CONF-07, D-22 as amended
2026-11-16).

Started 2026-11-17T02:13:44Z, ended 2026-11-17T02:46:15Z; 3 tasks, 4 code and state commits, 5 source
and planning files changed.

## Accomplishments

- `Repo::refuse_replace_refs` in `src/git/refs.rs` applies the rule in order: `refuse_replace_base_env`,
  `gitoxide.objects.replaceRefBase`, `holds_an_entry(<common dir>/refs/replace)`,
  `packed_refs_name_the_base(<common dir>/packed-refs)`. `Repo::discover` calls
  `refuse_replace_base_env` first; `build` calls the guard right after the branch-exists check.
- Three `git::Error` variants replace 04-11's three; `is_replacement_refusal` lists exactly them, so
  `build`'s and `require`'s existing arms carry the new refusals unchanged (D-16, D-22).
- 04-11's replacement tests adapted, one deleted, eleven new model tests, four message tests, one
  refs unit test, plus one survivor test from Task 3.
- STATE.md's two phase-5 lines and its phase-6 discovery line rewritten through GSD.

## TDD Evidence

**RED** (`c008f37`): `target/gsd/04-12-red-evidence.json` from
`scripts/check.sh test -E 'test(=model::tests::single_replacement_fails_closed)'`, exit 100;
`gsd_run check tdd-red-evidence` printed `RED_EVIDENCE_OK` (`target_test_failed`). Semantic assessment:
the target ran (1 run, 713 skipped) and failed on the planned assertion: git printed `only b`, then
`resolve: expected an error, got Configured(...)` with the manifest `only b`.

Every other RED test, run alone at the RED commit, log `target/gsd/04-12-red/<test>.log`, last line
`exit 100` for each:

| Tests | Error met at RED |
|-------|------------------|
| single_, single_tree_, single_commit_replacement, nested_, packed_nested_, symlinked_, crlf_, unsorted_, replace_base_key_alone_, unreadable_replace_dir_ | `resolve: expected an error, got Configured` |
| replacement_chain_, _on_the_tip_tree_, _on_the_tip_commit_, cycle_, symbolic_, uppercase_, packed_replacement_chain_, unrelated_object_, differing_only_in_case_ | `expected Err(Git(ReplaceRefsPresent)), got Git(...)` (04-11's variants) |
| replaced_workspace_commit_is_refused_before_it_is_peeled | `expected Err(Git(ReplaceRefsPresent)), got ManifestUnreadable` |
| replace_base_key_git_ignores_fails_closed | `expected Err(Git(ReplaceBaseSet)), got Git(...)` |
| replace_base_variable_fails_closed | child `answered "failed: refusing refs/other/<a>: ...` |
| replace_base_variable_gix_cannot_open_fails_closed | child `answered "failed: not a git repository: ...` |
| replace_ref_with_replace_objects_off_fails_closed | child `answered "configured", not ...` |
| replace_ref_leaves_a_not_configured_repo_not_configured | `inferred: expected an error, got` |

Each test's assertion of git's own answer held at RED (CRLF `bad file`, unsorted `only .planning`,
`GIT_NO_REPLACE_OBJECTS=1` `only .planning`, absolute base `only .planning`, tip commit replaced by a
blob `does not exist in`), so no premise in the plan differed from git 2.56.1. The two controls
`replace_dir_left_empty_resolves` and `packed_refs_without_a_replace_ref_resolve`, and
`missing_blob_still_renders_cannot_be_read`, passed at RED (`exit 0`). Task 1's first block exited 0.

**GREEN** (`9fe2955`): Task 1's second block (34 tests, one per command), third block (absence gate,
positive control held) and fourth block (`scripts/check.sh all`, `650 tests run: 650 passed, 61
skipped`, last line `==> check.sh: all OK`) exited 0 before the commit. Placement block after it exited
0; `target/gsd/04-12-placement.log` holds
`expected Err(Git(ReplaceRefsPresent)), got ManifestUnreadable { branch: [...] ...` and
`src/model/mod.rs` matched HEAD again. Tracer gate (interactive, end-of-phase, automated-only verify):
all five blocks re-run on the committed source and passed, so expansion went ahead.

No REFACTOR commit: nothing to change after GREEN.

## Task 2: controls' failing direction and STATE

- `target/gsd/04-12-negative/0/listed.txt`: `src/git/refs.rs:383:5: replace holds_an_entry -> Result<bool, Error> with Ok(true)`, caught by `replace_dir_left_empty_resolves`; its log holds `resolve: expected RepoModel::Configured, got Err(Git(ReplaceRefsPresent { path: ".../work/.git/refs/replace", packed: false`.
- `target/gsd/04-12-negative/1/listed.txt`: `src/git/refs.rs:401:5: replace packed_refs_name_the_base -> Result<bool, Error> with Ok(true)`, caught by `packed_refs_without_a_replace_ref_resolve`; its log holds `resolve: expected RepoModel::Configured, got Err(Git(ReplaceRefsPresent { path: ".../work/.git/packed-refs", packed: true`.
- Both `caught.txt` equal their `listed.txt`; each run `1 mutant tested ...: 1 caught`; the block exited 0; both `tmp` directories are gone.
- GSD results: the three `state.resolve-blocker` calls each returned `"resolved": true`; the three `state.add-blocker --text-file` calls each returned `"added": true`. The diff showed only those three lines under Blockers/Concerns plus frontmatter and the orchestrator's uncommitted begin-phase position edit, which went into the same commit (`c140780`). Scratch text files deleted. Task 2's STATE block exited 0.

## Task 3: mutation, coverage and gates

- First pass on `c140780`: 107 mutants, 90 caught, 1 missed, 16 unviable, 0 timeout. Survivor
  (`target/gsd/04-12-first-missed.txt`): `src/git/refs.rs:371:24: replace match guard value != DEFAULT_REPLACE_BASE with true in refuse_replace_base_env`,
  a real gap: no test showed `GIT_REPLACE_REF_BASE=refs/replace/` passes. Fixed by
  `model::tests::replace_base_variable_set_to_the_default_resolves` (`1ab5bb1`).
- Final pass on `1ab5bb1` (`target/gsd/04-12-pass-head.txt`), no `--iterate`: caught 91, missed 0,
  unviable 16, timeout 0 (`wc -l`), sum 107, equal to the `--list` total of 107. Blocks 1-3 exited 0.
  `target/gsd/04-12-equivalent-mutants.txt` is empty: no equivalent mutant.
- Doctored copies: last `caught.txt` line deleted -> exit 1 `tested mutants differ from the listed ones`;
  that line replaced by `src/git/refs.rs:1:1: replace nothing with nothing` -> exit 1 `tested mutants differ from the listed ones`;
  that line appended to `timeout.txt` -> exit 1 `unexplained survivors above`. Copies removed.
- Line coverage (`scripts/check.sh coverage`, rc 0, no threshold): `git/mod.rs` 99.28%, `git/refs.rs`
  93.44%, `model/mod.rs` 96.72%, TOTAL 92.97%. Branch coverage waived as in 04-10.
- `scripts/check.sh all` on `1ab5bb1`: exit 0, `651 tests run: 651 passed, 61 skipped`, `==> check.sh: all OK`.
- `scripts/check-in-container.sh all` on `1ab5bb1`: exit 0, `651 tests run: 651 passed, 61 skipped`,
  `==> check.sh: all OK`; the image's git is 2.56.0 (host 2.56.1).
- `scripts/check.sh deps`: **exit 1**, `advisories FAILED, bans ok, licenses ok, sources ok`. See
  Deferred Issues.

Scope of these checks: each shape is pinned on one fixture, on host git 2.56.1 and image git 2.56.0;
the 2.54 floor is not exercised; a device file or socket under the base is refused by the any-entry
rule and not tested; mutation testing shows the tests catch the mutants cargo-mutants generates, and
line coverage shows a line ran.

## Deviations from Plan

**1. [Rule 1 - Bug] `io::Error::other` in place of `io::Error::new(io::ErrorKind::Other, ...)`**
- Found during: Task 1 RED. The two build the same error; the latter form is the one clippy's
  `io_other_error` lint rejects under `-D warnings`. Files: `src/git/mod.rs`. Commit: `c008f37`.

**2. [Rule 3 - Blocking] The `ReplaceRefsPresent` arm writes its first line, then the remedy starting
with `\n`**, because clippy rejects a `write!` whose format string ends in one newline. The rendered
text is the plan's exactly, pinned by the two message tests. Commit: `c008f37`.

**3. Some new tests skip `sides`** (`unreadable_replace_dir_...`, `replace_base_variable_gix_cannot_open_...`,
`packed_refs_without_a_replace_ref_resolve`, `replace_base_key_alone_...` as before): they use no side
object, so the fixture cost buys nothing.

**4. `assert_refused_as_loose` returns the rendered refusal**, so the unrelated-object test checks
`.wsx.conf` is not named without resolving twice.

**Total deviations:** 4 minor, none changing behaviour or a test's premise.

## Deferred Issues

- **`scripts/check.sh deps` fails on this tip** with RUSTSEC-2026-0486 (rustix 1.1.5 unsound xattr
  functions, via gix-index 0.58.0 and tempfile 3.27.0). Not caused by this plan: `Cargo.lock` is
  unchanged since before the phase; the advisory is new. Fix is rustix `>=1.1.6`
  (`cargo update -p rustix`), not applied: a lockfile update is a package-manager change the executor
  does not make without the operator. Bumping `Cargo.lock` after this pass makes Task 3's pass-head
  check fail until the mutation pass is re-run. Logged in `deferred-items.md` and STATE.md's blockers.

## Kept on purpose (gitignored, under `target/gsd/`)

`04-12-red-evidence.json`, `04-12-red/<test>.log`, `04-12-all.log`, `04-12-placement.log`,
`04-12-negative/{0,1}/listed.txt` and `mutants.out`, `04-12-mutants/mutants.out`, `04-12-pass-head.txt`,
`04-12-equivalent-mutants.txt`, `04-12-first-missed.txt`, `04-12-first-timeout.txt`, `04-12-listed.txt`,
`04-12-tested.txt`, `04-12-coverage.log`, `04-12-container.log`, `04-12-deps.log`.
`04-12-mutants-tmp`, `04-12-checks-control` and `target/llvm-cov-target` are gone.

## Next

Phase 4's plans all have summaries; ready for phase verification, the review and security gates
(CR-01, WR-01, T-04-30), after the deps advisory is settled.

## Self-Check: PASSED
