---
phase: 04-repo-model
plan: 10
subsystem: testing
tags: [cargo-mutants, mutation-testing, llvm-cov, container-gate, repo-model]

requires:
  - phase: 04-repo-model
    provides: "the parser (04-03, 04-04), the resolver (04-01, 04-05 to 04-09) and the replacement refusal (04-11) this plan mutates and gates"
provides:
  - "A fresh full cargo-mutants pass over the phase's six core files: 177 mutants, 154 caught, 0 missed, 23 unviable, 0 timeout"
  - "Two survivor-killing tests in model::tests and a model layer with no gix:: reference"
  - "The phase gate green on the host and in the pinned container, on the phase tip"
  - "Eight STATE.md Blockers/Concerns lines carrying what phase 4 leaves for phases 5, 6, 8 and 12"
affects: [phase-5, phase-6, phase-8, phase-12]

actuals:
  tokens: 2462
  tasks: 2
  commits: 3
plan_head_before: d51fdbabf139dab138ec73fbb1f4c19f83699375
plan_head_after: b505f2d

tech-stack:
  added: []
  patterns:
    - "Mutation evidence is one fresh full pass of the final source, checked against the --list total, never an --iterate run"

key-files:
  created: []
  modified:
    - src/model/mod.rs
    - src/model/manifest.rs
    - .planning/STATE.md

key-decisions:
  - "04-10: the model's test modules render bytes through crate::git::c_quote, not gix::bstr, so rg -qF 'gix::' src/model exits 1 and every gix use stays in src/git"
  - "04-10: the local integration-rung order has its own test (local_develop_beats_local_next) instead of resting on the shared CHAIN loop and the remote-side test"

patterns-established:
  - "A mutant-killing test is shown to fail with its mutant applied in place, on the planned assertion, before it is committed"

requirements-completed: [CONF-01, CONF-02, CONF-03, CONF-04, CONF-05, CONF-07]

coverage:
  - id: D1
    description: "Every cargo-mutants mutant of the six core files is caught or unviable in one fresh full pass of the final source"
    requirement: CONF-07
    verification:
      - kind: other
        ref: "04-10-PLAN Task 1 survivor check and fresh-pass check (target/gsd/mutants/mutants.out, pass head 09eca8c)"
        status: pass
    human_judgment: false
  - id: D2
    description: "require passes a git failure that is not a replacement refusal through unwrapped"
    requirement: CONF-05
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::require_passes_a_git_failure_through"
        status: pass
    human_judgment: false
  - id: D3
    description: "With no remote, a local develop beats a local next"
    requirement: CONF-03
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::local_develop_beats_local_next"
        status: pass
    human_judgment: false
  - id: D4
    description: "Phase gate passes on the host and in the pinned container"
    verification:
      - kind: other
        ref: "scripts/check.sh all; scripts/check.sh deps; scripts/check-in-container.sh all"
        status: pass
    human_judgment: false
  - id: D5
    description: "STATE.md records the obligations phase 4 leaves for phases 5, 6, 8 and 12"
    verification:
      - kind: other
        ref: "04-10-PLAN Task 2 obligations verify commands (both)"
        status: pass
    human_judgment: false

duration: 64min
completed: 2026-11-08
status: complete
---

# Phase 4 Plan 10: Mutation pass, phase gate and obligations Summary

**A fresh full cargo-mutants pass over the parser and resolver leaves no survivor (154 caught, 23 unviable of 177) after two new tests; the gate is green on the host and in the pinned image, and STATE.md carries the eight obligations phase 4 leaves for phases 5, 6, 8 and 12.**

## Performance

- **Duration:** 64 min
- **Started:** 2026-11-08T17:39:23Z
- **Completed:** 2026-11-08T18:43:39Z
- **Tasks:** 2
- **Files modified:** 3

## Accomplishments

- First pass found one survivor; a new test kills it, and the final fresh pass misses nothing.
- The model layer now holds no `gix::` reference at all. The one it had was a test import, and it was the only thing failing the plan's boundary check.
- The host gate, `deps` and the pinned container gate all exit 0 on the phase tip.
- The eight obligations are in `### Blockers/Concerns`, written through `state.add-blocker`.

## Task Commits

1. **Task 1: Mutation pass, killing every real survivor**: `28491fa` (test), `09eca8c` (refactor, see Deviations)
2. **Task 2: Phase gate, source boundary, obligations**: `b505f2d` (docs)

## Mutation pass (Task 1)

cargo-mutants **27.1.0** (`~/.cargo/bin`, not pinned by the repo), `--test-tool nextest` with `.tools/bin` first on `PATH`, `-j 2 --timeout 300`, the plan's six files and filter.

| Pass | Source | Caught | Missed | Unviable | Timeout | Total | `--list` |
|------|--------|--------|--------|----------|---------|-------|----------|
| First | `d51fdba` | 153 | 1 | 23 | 0 | 177 | 177 |
| Intermediate fresh | `28491fa` | 154 | 0 | 23 | 0 | 177 | 177 |
| **Final fresh** | `09eca8c` | **154** | **0** | **23** | **0** | **177** | **177** |

The intermediate pass is void as evidence: `09eca8c` touched `src/` after it. The final pass ran after that commit. No `--iterate` run was used at any point. `previously_caught.txt` is absent. `b505f2d` changes only STATE.md, so the fresh-pass check holds on the phase tip.

**The first pass's one survivor** (`target/gsd/04-10-first-missed.txt`):
`src/model/mod.rs:124:38: replace match guard inner.is_replacement_refusal() with true in RepoModel::require`. This was a real gap. No test gave `require` a `Git` error on the inferred path that was not a replacement refusal. The new test `model::tests::require_passes_a_git_failure_through` corrupts a candidate ref and asserts that `require` returns the listing's `Error::Git`. RED evidence: with the mutant applied in place, the test failed on its assertion (`got Err(NotConfigured { reason: Git(Gix { what: "list the refs directly under refs/heads/workspace/", .. }) })`). On the real code it passes. The final pass lists that mutant in `caught.txt`.

**Equivalent mutants:** none. `target/gsd/04-10-equivalent-mutants.txt` is empty, so there are no reasons to give.

**Survivor-check negative control:** I ran the plan's survivor check against `04-10-first-missed.txt` with the same empty allowlist. It exited 1 with `unexplained survivors above`, and against the final `missed.txt` it exits 0.

**Survivors reported by earlier plans:**
- *04-07, swapping `CHAIN`'s order.* cargo-mutants generates no array-reorder mutant, so this pass neither confirms nor kills it. `CHAIN` drives one loop for both remote and local rungs, so the swap was already caught on the remote side. The new `model::tests::local_develop_beats_local_next` pins the local order on its own: with `CHAIN` swapped in place it failed (`integration branch` was `next`), and on the real code it passes. **Killed.**
- *04-08, removing `refs_directly_under`'s own sort.* cargo-mutants generates no statement-deletion mutant, so this pass did not test it. On gix 0.90.0 it is **equivalent**: gix already returns refs in name order, so no test can tell the two apart. I did not re-run 04-08's measurement here.
- *04-01, `no_source_branch_and_no_candidate_is_not_configured` passing with `GIT_DIR` aimed at a decoy.* This is not a mutant. It is a gap in how the test tells which repository it read, and fixing it is outside this plan's mutation and gate scope, so it stays as 04-08 recorded it.

## Phase gate (Task 2)

| Command | Exit | nextest summary |
|---------|------|-----------------|
| `scripts/check.sh all` (phase tip source) | 0, last line `==> check.sh: all OK` | 639 tests run: 639 passed, 61 skipped |
| `scripts/check.sh deps` | 0, `advisories ok, bans ok, licenses ok, sources ok` | n/a |
| `scripts/check-in-container.sh all` (on `b505f2d`) | 0, last line `==> check.sh: all OK` | 639 tests run: 639 passed, 61 skipped |
| `scripts/check.sh coverage` | 0, `TOTAL` row present | 639 passed (1 slow), 61 skipped |

- The image is `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`. `docker run --rm --entrypoint bash <image> -c 'PATH=/usr/local/bin:/usr/bin:/bin git --version'` printed **`git version 2.56.0`**. Control: with `PATH=/usr/bin:/bin` it printed `git version 2.47.3`. The host's git is also 2.56.0.
- In the image, `readlink -f /bin/sh` printed `/usr/bin/dash`; on the host it is `/usr/bin/bash`. So the container gate ran 04-01's paste tests, `git::quote::tests::pasted_command_removes_a_ref_named_with_shell_syntax` and its control `display_quoting_leaves_a_command_substitution_live`, under dash (both show `PASS` in `04-10-container.log`), and the host gate ran them under bash in POSIX mode. zsh and fish are covered only by planning measurements.
- Line coverage, with no threshold set. Branch coverage is waived, as the plan says.

| File | Lines | Missed | Line cover |
|------|-------|--------|------------|
| `src/model/manifest.rs` | 705 | 25 | 96.45% |
| `src/model/mod.rs` | 3452 | 109 | 96.84% |
| `src/git/config.rs` | 500 | 35 | 93.00% |
| `src/git/refs.rs` | 1332 | 92 | 93.09% |
| `src/git/quote.rs` | 232 | 23 | 90.09% |
| `src/git/mod.rs` | 303 | 0 | 100.00% |
| TOTAL (whole crate) | 11390 | 805 | 92.93% |

Source boundary on the phase tip:
- `rg -qF 'gix::' src/model` exits 1. Controls: the same search over `src/git` exits 0, and the search exited 0 on `src/model` before `09eca8c`.
- The `src/git/refs.rs` revspec check exits 0.
- The phase-scoped subject check over `6946ae9..` for `src/model/mod.rs` exits 0, over 21 commits.

Obligations: both of Task 2's obligation verify commands exit 0 on STATE.md. The first one exits 1 on the pre-edit copy, which is the baseline control. Apart from the eight added lines, `git diff` showed only GSD's `last_updated` and `state_head` frontmatter refresh.

## Success criteria and what each does not establish

| # | Success criterion | Tests that prove it | What they do not establish |
|---|-------------------|---------------------|----------------------------|
| 1 | Any git path round-trips C-quoted; `.planning` covers `.planning/x` and not `.planningX`; an unknown line kind names the line | `model::manifest::tests::any_git_path_round_trips` (proptest), `property_runs_its_configured_cases`, `git_quoted_output_parses_to_git_raw_names` (all-bytes differential), `path_covers_what_is_under_it`, `name_extension_is_not_covered`, `covers_matches_a_literal_pathspec`, `unknown_kind_names_the_line` | The proptest is a search of 2048 cases or more, not a proof. A fixed `PROPTEST_RNG_SEED` would go unseen. Parity was measured on git 2.56.0 only. The pathspec differential covers five paths. |
| 2 | Chain `develop`, then `next`, then the remote default; `wsx.baseRef` overrides it; fork remotes are `upstream` and `origin`, defaulting to `origin` | `model::tests::develop_beats_the_remote_default`, `next_follows_develop`, `remote_default_follows_next`, `local_develop_beats_local_next`, `local_next_follows_local_develop`, `base_ref_overrides_the_chain`, `main_is_never_guessed`, `fork_layout_resolves_each_remote`, `remotes_default_to_origin`, `push_remote_can_differ_from_a_default_base` | Each rung is tested once, on one fixture, on git 2.56.0. "No network call" is shown only by deleting the remote's directory. A remote's URL is never checked. CONF-04's edge-probe item is unresolved. |
| 3 | `wsx.sourceBranch` names the workspace branch; otherwise the only `workspace/*` or `personal/*` branch; zero or several candidates is an error listing them | `model::tests::source_branch_names_the_workspace`, `only_candidate_is_inferred`, `personal_branch_is_a_candidate`, `several_candidates_are_listed` (`personal/workspace`, `personal/workspace-backup`), `zero_candidates_are_an_error`, `recorded_branches_are_not_candidates`, `nested_branch_is_not_a_candidate`, `require_lists_the_candidates` | CONF-05's "ignoring branches made by `wsx new`" is proven at `resolve`'s `recorded` input only. Production passes an empty set until phase 8 (D-17, now in STATE.md). `no_source_branch_and_no_candidate_is_not_configured` cannot tell which repository it resolved under a decoy `GIT_DIR`. |
| 4 | The list comes from `<tip>:.wsx.conf`; a working-tree edit changes nothing; a clean checkout resolves the same list; only `.wsx.conf` and the worktree dir are built in | `model::tests::configured_repo_resolves_from_the_object_database`, `working_tree_edit_changes_nothing`, `clean_checkout_resolves_the_same_list`, `linked_worktree_resolves_the_same_list`, `hook_environment_resolves_the_same_list`, `undeclared_path_is_not_personal` (`.planning/x`, `CLAUDE.md`), `declared_path_is_personal`, `built_ins_are_personal_with_an_empty_manifest`, `custom_worktree_dir_replaces_the_default` | No `wsx` command reads the model until phase 5, so the guard, the cut and `status` reading this one list is not exercised. The harness scenarios enable no Rust side. CONF-07's edge-probe item (a missing tree object or a malformed entry) is unresolved. |
| 5 | With `wsx.sourceBranch` set, a missing or invalid manifest fails closed with the reason; without it, the repo is not configured | `model::tests::missing_manifest_fails_closed`, `invalid_manifest_fails_closed`, `missing_blob_object_fails_closed`, `missing_tip_commit_fails_closed`, `symlink_manifest_fails_closed`, `unsorted_tree_manifest_fails_closed`, `replacement_chain_fails_closed`, `no_source_branch_is_not_configured`, `invalid_inferred_manifest_is_not_configured`, `require_wraps_a_manifest_failure_as_not_configured`, `require_passes_a_git_failure_through` | Exit codes are phase 5's job. The guard's exit 0 on not configured is phase 6's. An out-of-order root tree is always refused, which is stricter than git. |

**Still open to review:**
- Assumptions A1 to A6 (RESEARCH Assumptions Log). Each has been applied and measured where git has an opinion, but each is still a reading of a decision, not a decision.
- The edge-probe items flagged unresolved: CONF-03, CONF-04 and CONF-07. The backstop item: CONF-01 concurrency.
- The per-task map in `.planning/phases/04-repo-model/04-VALIDATION.md` (22 rows, filled at the revision after 03.1) still shows every row pending. That is for `/gsd-validate-phase`.

## Limits

- `src/git/repo.rs` is not in the mutation pass (phase 3's file). 04-05's `refuse_refs_namespace` rests on its refusal tests and their controls.
- Mutation testing shows the tests catch the mutations cargo-mutants generates, not every defect. It generates no semantic mutants (a changed filter value), no reorders and no statement deletions. That is why the two earlier hand-mutants needed separate handling.
- The 23 unviable mutants did not compile, so they say nothing about the tests.
- The final counts come from one fresh full pass, so no arithmetic across runs was needed. The fresh-pass check shows the pass tested as many mutants as `--list` names and that no source commit followed it. It does not show the two sets are the same mutants. Its conditions were validated on a scratch crate at planning, not on this repository.
- Line coverage shows a line ran, not that a test would notice it changing.
- The container run shows the gate passes in a second environment on the same git version, 2.56.0. It proves no second git version, and no real git at the 2.54 floor runs in this phase.
- No test exercises the model through a `wsx` command, since none exists until phase 5.
- I did not observe the new `shown()` helper's rendering in an actual failure message. It changes only what failing manifest tests print.

## Deviations from Plan

**1. [Rule 1 - Bug] The model held a `gix::` import, which failed a must-have**
- **Found during:** Task 2 (source boundary check)
- **Issue:** `rg -qF 'gix::' src/model` exited 0. The hit was `use gix::bstr::ByteSlice;` in `model::manifest::tests`, added by 04-03 (`92cf8f7`), used only for `.as_bstr()` in nine failure messages. Production code had no gix reference.
- **Fix:** a test-local `shown()` renders bytes through `crate::git::c_quote`, and the import is gone. No dependency was added.
- **Files modified:** `src/model/manifest.rs`
- **Verification:** `scripts/check.sh all` exits 0, and `rg -qF 'gix::' src/model` exits 1. The commit touched `src/`, so I re-ran a fresh full mutation pass after it: 0 missed.
- **Committed in:** `09eca8c`

**2. [Rule 2 - Missing test] Local integration-rung order had no test of its own**
- **Found during:** Task 1 (classifying 04-07's reported survivor)
- **Fix:** `model::tests::local_develop_beats_local_next`, with the `CHAIN` swap as its failing control.
- **Committed in:** `28491fa`

**3. A container run was voided and repeated**
- I made the Deviation 1 edit while a container run was in progress. Because the container mounts the live checkout, that run compiled a half-edited tree and exited 101. I discarded it, and the container gate reported above ran on the committed tip `b505f2d` with a clean tree.
- I wrapped the container command in `timeout 3000`, to bound a long run. It is otherwise the plan's command.

**Total deviations:** 2 auto-fixed (1 bug, 1 missing test), plus one discarded gate run. **Impact:** one more fresh mutation pass (19 min). There was no change to production code.

## TDD Gate Compliance

This is a `type: execute` plan, and no task carries `tdd="true"`. No production behaviour changed, so it has no RED/GREEN pair. Each new test's RED evidence is its failure with the mutant applied in place, recorded above, and it passes on the unmutated code.

## Kept on purpose (gitignored under `target/`)

- `target/gsd/mutants/mutants.out/`: the final fresh pass's lists, read by the survivor and fresh-pass checks.
- `target/gsd/04-10-pass-head.txt`: `09eca8c`, read by the fresh-pass check.
- `target/gsd/04-10-equivalent-mutants.txt`: empty, cited above.
- `target/gsd/04-10-first-missed.txt`: the first pass's survivor.
- `target/gsd/04-10-{all,deps,container,coverage,pass1,pass-final}.log`: the gate and pass output these figures come from.

Removed: `target/gsd/mutants-tmp` (removed by the pass on every exit, and absent now), `target/llvm-cov-target` (absent), `target/gsd/mutants/mutants.out.old` and my single-test logs. No process I started is still running, and no container is running.

## Issues Encountered

None beyond the deviations.

## User Setup Required

None.

## Next Phase Readiness

Phase 4 is complete, so the next step is `/gsd-verify-work 4` and `/gsd-validate-phase 4`. Phases 5, 6, 8 and 12 inherit the STATE.md obligations.

## Self-Check: PASSED

- The commits `28491fa`, `09eca8c` and `b505f2d` are all ancestors of HEAD.
- `src/model/mod.rs`, `src/model/manifest.rs`, `.planning/STATE.md` and the four kept `target/gsd` evidence paths all exist.
- Task 1's acceptance check, its survivor check and its fresh-pass check all exit 0.
- Both obligations checks exit 0.

---
*Phase: 04-repo-model*
*Completed: 2026-11-08*
