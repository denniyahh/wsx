---
phase: 04-repo-model
plan: 11
subsystem: git-layer
tags: [gix, refs-replace, replace-refs, fail-closed, manifest]
status: complete

requires:
  - phase: 04-repo-model
    provides: "04-01 shell_word and c_quote; 04-08 refs_directly_under, require and build's ManifestUnreadable mapping; 04-09 root_entry's mode and tree-order checks; 04-05 state_child and the namespace refusal"
provides:
  - "git::Error::ReplacementRef, ReplacementRefDuplicate, ReplaceBaseDiffers with messages whose removal commands quote the ref name through shell_word"
  - "git::Error::is_replacement_refusal, the one list build and require read"
  - "Repo::refuse_unfollowable_replacements, called first in root_entry, repo-wide"
  - "build keeps a replacement refusal as Error::Git (D-22); require wraps it as NotConfigured with no key (D-16)"
affects: [phase-05-check, phase-06-guard, phase-09-cut]

actuals:
  tokens: 12308
  tasks: 1
  commits: 2
plan_head_before: 71e2265d5ec6c40811f4b5936b75c9d1a7aeba8b
plan_head_after: 49939a09b6b483f93fbe6d36dc8f5923a2901102

tech-stack:
  added: []
  patterns:
    - "Each replacement test asserts git's own answer (git cat-file blob refs/heads/workspace/dev:.wsx.conf) before asserting the model's"
    - "Read every object id a test needs before adding any replacement, since a replacement changes git rev-parse's answer"

key-files:
  created: []
  modified:
    - src/git/mod.rs
    - src/git/refs.rs
    - src/model/mod.rs

key-decisions:
  - "The base check when git's and gix's replace bases differ reads only the first ref gix's prefixed listing returns per base: one ref is enough to refuse, and a ref that fails to read is Error::Gix naming the base"
  - "gix's base is read with config_snapshot().string, which takes the last value even from a reduced-trust file, while gix's open uses a trust-filtered read; where they differ the guard over-refuses, the safe side"

patterns-established:
  - "side_manifest: a real side commit from workspace/dev, giving a blob, a tree and a commit to replace with no stdin or git mktree"

requirements-completed: []

coverage:
  - id: D1
    description: "One replacement of the manifest blob, its tree or its commit reads the bytes git prints, and the workspace tip stays the id git show-ref prints"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::single_replacement_reads_as_git_does"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::single_tree_replacement_reads_as_git_does"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::single_commit_replacement_reads_as_git_does"
        status: pass
    human_judgment: false
  - id: D2
    description: "A chain (blob, tree, commit, uppercase link, packed, under GIT_REPLACE_REF_BASE, among unrelated objects), a cycle and a symbolic replace ref are refused as ReplacementRef before any manifest read"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replacement_chain_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replacement_chain_on_the_tip_tree_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replacement_chain_on_the_tip_commit_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replacement_cycle_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::symbolic_replacement_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replacement_chain_through_an_uppercase_name_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::packed_replacement_chain_fails_closed"
        status: pass
      - kind: integration
        ref: "src/model/mod.rs#model::tests::replacement_chain_under_the_env_base_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replacement_chain_on_an_unrelated_object_fails_closed"
        status: pass
    human_judgment: false
  - id: D3
    description: "Two replace refs naming one object in different hex case are ReplacementRefDuplicate; gitoxide.objects.replaceRefBase differing from git's base is ReplaceBaseDiffers only while a ref sits under either base"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replace_refs_differing_only_in_case_fail_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replace_base_key_git_ignores_fails_closed"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replace_base_key_without_a_replace_ref_resolves"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replace_ref_under_git_base_with_the_key_set_fails_closed"
        status: pass
    human_judgment: false
  - id: D4
    description: "D-22 and D-16: the refusal names the ref, never says .wsx.conf cannot be read, and in a repo with no key stays in inferred and reads as run wsx init through require; a missing blob still renders cannot be read"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::replacement_chain_leaves_a_not_configured_repo_not_configured"
        status: pass
      - kind: unit
        ref: "src/model/mod.rs#model::tests::missing_blob_still_renders_cannot_be_read"
        status: pass
      - kind: unit
        ref: "src/git/mod.rs#git::tests::replacement_refusals_are_recognised"
        status: pass
    human_judgment: false
  - id: D5
    description: "The three messages render whole as planned, and the removal commands quote a ref name for the shell through shell_word, or print <ref>"
    verification:
      - kind: unit
        ref: "src/git/mod.rs#git::tests::replacement_ref_message_names_the_ref_and_the_fix"
        status: pass
      - kind: unit
        ref: "src/git/mod.rs#git::tests::duplicate_replacement_refs_message_names_both_and_the_fix"
        status: pass
      - kind: unit
        ref: "src/git/mod.rs#git::tests::replace_base_differs_message_names_the_key_and_the_fix"
        status: pass
      - kind: unit
        ref: "src/git/mod.rs#git::tests::replacement_fix_commands_quote_a_ref_name_for_the_shell"
        status: pass
    human_judgment: false

duration: 12min
completed: 2026-11-08
---

# Phase 4 Plan 11: Replacement refusal Summary

**`root_entry` now refuses, repo-wide and before any manifest read, the replace refs gix and git follow differently: chains, cycles, symbolic replace refs, case-only duplicate names, and a `gitoxide.objects.replaceRefBase` that differs from git's base while a ref sits under either base. One replacement still reads the bytes git prints.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-11-08T17:22:18Z
- **Completed:** 2026-11-08T17:34:44Z
- **Tasks:** 1 (RED and GREEN commits; no refactor commit, nothing to clean up)
- **Files modified:** 3

## Accomplishments

- `Repo::refuse_unfollowable_replacements` (private, `src/git/refs.rs`), called first in `root_entry`. git's base is `GIT_REPLACE_REF_BASE` from the process environment, else `refs/replace/`. gix's base is `gitoxide.objects.replaceRefBase` from the config snapshot, else `refs/replace/`. When they differ, a ref under either base, at any depth, is `ReplaceBaseDiffers`, and with no such ref the check passes. When they are equal, the refs directly under the base are keyed by lower-cased name in a `BTreeMap`: a second ref on one key is `ReplacementRefDuplicate`, and a symbolic ref, or one whose target hex is a key, is `ReplacementRef`.
- `git::Error::is_replacement_refusal` is one `matches!` over the three variants. `build` keeps a refusal as `Error::Git`, so it never reads "cannot be read" (D-22). `require` wraps it as `NotConfigured` when there is no key (D-16).
- The messages are as the plan specifies. Prose goes through `c_quote`. Each `git update-ref --no-deref -d` argument goes through `shell_word`, or prints `<ref>` when no quoting can carry the name.

## TDD Gate Compliance

- **RED** `ad245a2` `test(04-11): add failing replacement tests`. Target `model::tests::replacement_chain_fails_closed`, run with `scripts/check.sh test -E 'test(=model::tests::replacement_chain_fails_closed)'`, exit 100. The record is `target/gsd/04-11-red-evidence.json`, from JUnit `target/nextest/default/junit.xml` (run started 17:27:49.077Z, report written 17:27:55.986Z). `gsd-tools check tdd-red-evidence ... --raw` printed **`RED_EVIDENCE_OK`** (reason `target_test_failed`, 1 test, 1 fail).
  - Semantic assessment: the target ran and failed on the planned assertion. The test first asserted that git prints C's bytes (`only c`). `resolve` then answered `Configured` with the manifest `only b`, which is B's bytes, because gix followed one hop. There was no build, fixture or load failure.
  - An informational run of all 23 new tests in the RED state (not a gate) showed 13 model tests and `replacement_refusals_are_recognised` failing on their refusal assertions, each after git's own answer had passed. The controls passed, as expected: the single replacements, `missing_blob_still_renders_cannot_be_read`, `replace_base_key_without_a_replace_ref_resolves` and the message tests.
- **GREEN** `49939a0` `feat(04-11): refuse replacements gix and git follow differently`. It descends from the RED commit (`git merge-base --is-ancestor`).
- **REFACTOR:** none.

## Task Commits

1. **Task 1: refusal of replacements gix and git follow differently.** RED `ad245a2` (test), then GREEN `49939a0` (feat).

## Files Created/Modified

- `src/git/mod.rs`: the three variants, their `Display` arms, `is_replacement_refusal`, the private helpers `shown` and `ref_word`, and five tests.
- `src/git/refs.rs`: the constants `REPLACE_BASE_KEY`, `REPLACE_BASE_VAR` and `DEFAULT_REPLACE_BASE`, plus `refuse_unfollowable_replacements` with the plan's doc comment and the private `any_ref_starts_with`. `root_entry` calls the check first.
- `src/model/mod.rs`: the refusal arm in `build`, the refusal arm in `require` and updated doc comments. Test-side: `side_manifest` and its helpers (`rev_id`, `objects`, `sides`, `git_manifest_in`, `git_prints`, `git_refuses`, `assert_lists`, `replacement_ref`, `assert_blames_the_ref`, `assert_base_differs`) and 18 tests. `state_child_output` and `child_state` now delegate to new `_with` variants that take extra environment.

## Verification

- `<verify>` loop: each of the 18 `model::tests` names and the 5 `git::tests` names ran in its own `scripts/check.sh test -E 'test(=...)'` command. The loop exited 0. The log shows 23 single-test summaries, each `1 test run: 1 passed`.
- `scripts/check.sh all > target/gsd/04-11-all.log 2>&1` exited 0, and the log's last line is `==> check.sh: all OK`: 637 tests run, 637 passed, 61 skipped.
- Acceptance greps: both chains exited 0. As a control, the first chain exits 1 against the RED commit's `refs.rs`. `replacement_fix_commands_quote_a_ref_name_for_the_shell` passes.

### What this does not establish

- The refusal is checked only when the model resolves. `Repo`'s phase 3 reads (`merge_base` and the rest) do not check it themselves; they are covered only because every command resolves the model first, an obligation on phase 5 that 04-10 records in STATE.md. The check does not run in a repo with zero or several candidates, since that repo never reaches `root_entry`.
- Because the check is repo-wide, a chain or symbolic replace ref among objects the manifest read never touches stops the model where git reads `.wsx.conf` correctly. In a repo with `wsx.sourceBranch` set, every wsx command fails and, from phase 6, every push through the guard, until the owner removes the ref the message names. Removing that link also changes what git reads for that object. The owner accepted this cost in D-22.
- A corrupt or unreadable ref under the replace base keeps 04-08's mapping. `refs_directly_under`'s `Err`, or the `prefixed` listing's when the bases differ, becomes `ManifestUnreadable`, which says `cannot be read` and so blames `.wsx.conf` for a ref-store fault. No test pins this.
- Over-refusal happens in the shapes the doc comment names: a non-hex name, two non-hex names differing in case, `core.useReplaceRefs=false`, `GIT_NO_REPLACE_OBJECTS`, and a differing base whose only refs are nested or non-hex names. None of these is tested.
- `GIT_REPLACE_REF_BASE` with no trailing slash, where git itself dies, is not covered, and the variable set to an empty string was not measured. A key value with no trailing `/` is covered only through gix's `prefixed` matching, read from source and not tested.
- A replace ref changed between `Repo::discover` and the check, during one run, is not caught: gix's map is loaded at open. The owner accepted this on 2026-11-03 (T-04-32), and no test pins it.
- Each agreeing shape is pinned on one fixture per kind, and each second-review shape on one fixture. Everything was measured on git 2.56.0 and gix 0.90.0 only.
- `actuals.tokens` (12,308) is chars/4 over the 49,232-byte realized diff. The estimate of 74,000 also counted reading the plan, the workflow and the source, so the two numbers are not on one scale.

## Decisions Made

- When the bases differ, the check reads only the first ref per base from gix's `prefixed` listing, because one ref is enough to refuse. A first item that fails to read is `Error::Gix` naming the base.
- gix's base comes from `config_snapshot().string`, which takes the last value even from a reduced-trust file. gix's own open uses a trust-filtered read, so in that case the guard over-refuses, which is the safe side.
- `REPLACE_BASE_VAR` is read with `std::env::var_os` and converted with `OsStringExt::into_vec`; `src/git/spawn.rs` already relies on unix-only `OsStrExt`.

## Deviations from Plan

None in the code. Three process notes:

- **Leftover process from the executor's own mistake.** Trying to load the GSD resolver, I ran `source` on `~/.claude/gsd-core/references/gsd-run-resolver.md`, a Markdown file. Its "```bash" fence line started an interactive `bash`, which is waiting on stdin. It ran as background task `boc35pd4e`, with PIDs 1580080, 1580129 and 1580130 (the waiting `bash`). My `kill` by those recorded PIDs was denied by the permission classifier, so they are still running. The rest of that file is one resolver line and prose. If the waiting `bash` gets EOF, the resolver may append a `PATH` export to `$CLAUDE_ENV_FILE`; the prose lines would only fail as unknown commands. Every GSD call after that used `node ~/.claude/gsd-core/bin/gsd-tools.cjs` directly.
- The plan names `replace -d` and `update-ref --no-deref -d` for its controls; the tests use exactly those.
- `REPLACE_BASE_KEY` and `OTHER_BASE` are also test-side constants in `model::tests`, separate from the private `refs.rs` constants.

**Total deviations:** 0 auto-fixed.

## Issues Encountered

None in the code. The leftover process is described under Deviations.

## Requirements

`CONF-07` is not marked complete. `requirements.ready-ids` reports it as blocked, because `04-10-PLAN.md` also lists it and has no SUMMARY yet. REQUIREMENTS.md is unchanged.

## User Setup Required

None.

## Next Phase Readiness

04-10 is the last plan of phase 4 without a SUMMARY. Its Task 2 records in STATE.md the phase 5 obligation that every command resolves the model before any other `Repo` read.

## Self-Check: PASSED

- FOUND: src/git/mod.rs, src/git/refs.rs, src/model/mod.rs, target/gsd/04-11-red-evidence.json, target/gsd/04-11-all.log
- FOUND: ad245a2, 49939a0 (both ancestors of HEAD; `check evaluation-scope --plan 04-11 --commits-only` lists both)

---
*Phase: 04-repo-model*
*Completed: 2026-11-08*
