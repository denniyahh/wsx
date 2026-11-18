---
phase: 04-repo-model
verified: 2026-11-18T05:53:56Z
status: gaps_found
score: 48/50 must-haves verified (4/5 roadmap success criteria; 44/45 applicable plan truths of 04-12 to 04-18; 04-01 to 04-11 regression-checked by the full suite)
covered_files:
  - ".planning/phases/04-repo-model/04-01-PLAN.md"
  - ".planning/phases/04-repo-model/04-01-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-02-PLAN.md"
  - ".planning/phases/04-repo-model/04-02-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-03-PLAN.md"
  - ".planning/phases/04-repo-model/04-03-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-04-PLAN.md"
  - ".planning/phases/04-repo-model/04-04-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-05-PLAN.md"
  - ".planning/phases/04-repo-model/04-05-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-06-PLAN.md"
  - ".planning/phases/04-repo-model/04-06-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-07-PLAN.md"
  - ".planning/phases/04-repo-model/04-07-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-08-PLAN.md"
  - ".planning/phases/04-repo-model/04-08-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-09-PLAN.md"
  - ".planning/phases/04-repo-model/04-09-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-10-PLAN.md"
  - ".planning/phases/04-repo-model/04-10-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-11-PLAN.md"
  - ".planning/phases/04-repo-model/04-11-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-12-PLAN.md"
  - ".planning/phases/04-repo-model/04-12-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-13-PLAN.md"
  - ".planning/phases/04-repo-model/04-13-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-14-PLAN.md"
  - ".planning/phases/04-repo-model/04-14-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-15-PLAN.md"
  - ".planning/phases/04-repo-model/04-15-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-16-PLAN.md"
  - ".planning/phases/04-repo-model/04-16-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-17-PLAN.md"
  - ".planning/phases/04-repo-model/04-17-SUMMARY.md"
  - ".planning/phases/04-repo-model/04-18-PLAN.md"
  - ".planning/phases/04-repo-model/04-18-SUMMARY.md"
  - "Cargo.lock"
  - "Cargo.toml"
  - "src/git/config.rs"
  - "src/git/mod.rs"
  - "src/git/objects.rs"
  - "src/git/quote.rs"
  - "src/git/refs.rs"
  - "src/git/repo.rs"
  - "src/git/spawn.rs"
  - "src/git/testing.rs"
  - "src/main.rs"
  - "src/model/manifest.rs"
  - "src/model/mod.rs"
  - "tests/harness/fixture.rs"
covered_digest: "v4:sha256:5512018c33b67ee61b90331492b8ba5c9d2301b6f603c5cfdcc3dbd001bb433a"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 100/101
  gaps_closed:
    - "04-11 (CONF-07): a replacement gix and git follow differently is refused, rather than read (nested replace ref refs/replace/<dir>/<hex>)"
  gaps_remaining: []
  regressions: []
gaps:
  - truth: "Roadmap SC 5 (CONF-07, D-16): a repo without wsx.sourceBranch is reported as not configured"
    status: partial
    reason: "In a repository whose .git/config file is missing (git still operates it: git status exits 0), any global remote.* or wsx.* record makes resolve fail with 'canonicalize <work>/.git/config: No such file or directory' instead of reporting not configured. Code review WR-03, reproduced by the verifier at 7a4201e in a scratch copy through model::tests' state_child: own config present -> state=not-configured (control); removed -> state=failed. The regex ^(wsx|remote)\\. also matches a global remote.pushDefault, so no wsx key is needed. Phase 6's guard would then refuse pushes in an unconfigured repo, where GUARD-03 says it exits 0."
    artifacts:
      - path: "src/git/config.rs"
        issue: "read_config canonicalizes <common dir>/config whenever git printed any record (lines 86-91), whether or not a local wsx.* record needs it; NotFound is not mapped to 'no own records'"
    missing:
      - "Treat a missing own config file as one that matches no record (own_config as Option, is_own_file Some(false) when None), or canonicalize lazily on the first local wsx.* record"
      - "A model test: repo_with_remote, a global [remote \"up\"] url, .git/config removed; assert git status exits 0 first, then child_state == not-configured; control with the file present"
  - truth: "04-18: a fresh full cargo-mutants pass, run after the last commit touching src/, leaves no missed or timed-out mutant outside target/gsd/04-18-equivalent-mutants.txt"
    status: failed
    reason: "The recorded pass ran at 128f281 (target/gsd/04-18-pass-head.txt is 32ab917); six src commits followed it (7c2e83c, f1b7ee7, 35cc024, 1961ac7, 3d44c24, d8cae67: the C3, C2 and CR-01 fixes). The verifier ran 04-18's own invocation (nextest, the same -E filter, -j 2, --timeout 300) with --in-diff over 5765862..HEAD -- src: 43 mutants, 37 caught, 5 unviable, 1 missed. The equivalent-mutant allowlist is empty."
    artifacts:
      - path: "src/git/refs.rs"
        issue: "MISSED src/git/refs.rs:307:21 'replace match guard err.kind() == std::io::ErrorKind::NotFound with true in packed_refs_name_the_base': no test makes metadata() of packed-refs fail with anything but NotFound, so the CR-01 fix's refusal of an unreadable packed-refs could become 'no replace refs' (fail open) unnoticed"
    missing:
      - "A git::refs unit test where metadata(packed-refs) fails with a non-NotFound error (for example a self-referencing symlink, ELOOP) and refuse_replace_refs returns ReplaceRefsUnreadable naming it"
      - "A fresh mutation pass on the new tip over 04-18-files.txt plus src/git/repo.rs, without --iterate, tested set equal to the listed set, 0 missed"
deferred:
  - truth: "Every command resolves the model (no production caller of model::resolve yet; src/model/mod.rs:15 carries expect(dead_code, reason = \"no caller until phase 5\"))"
    addressed_in: "Phase 5"
    evidence: "Phase 5 goal: 'it sets the versioned JSON and exit-code contract every later command must join'; STATE.md obligation: every wsx command gets its model from model::resolve or require before any other Repo read"
---

# Phase 4: Repo model Verification Report

**Phase Goal:** Every command resolves one repo model: the integration branch, the workspace branch, the base and push remotes, and a single personal-path list read from the workspace branch tip's `.wsx.conf`
**Verified:** 2026-11-18T05:53:56Z
**Status:** gaps_found
**Re-verification:** Yes, after gap closure (04-12 to 04-18, the git-binary pivot D-23 to D-36, and the post-04-18 fixes for diff review C2, C3, C4 and code review CR-01)

## Verdict

The gap from the last verification is closed: a nested replace ref (and every other replace-ref shape) now stops
`resolve` before any object read, and the model's objects are read by git itself (`cat-file --batch`), so the
manifest bytes are git's. The model is real and layered: config, refs and objects each come from one git child,
four children per resolve, p95 23 to 34 ms on the four R3 repos at HEAD. Two gaps remain, both small:

1. **SC 5 fails in one layout (reproduced).** An unconfigured repository with no `.git/config` file fails
   `resolve` as soon as any global `remote.*` or `wsx.*` key exists, instead of reading as not configured.
   This is code review WR-03, still `open` in 04-REVIEW-DISPOSITION.md. It fails loud, not silent, and the layout
   is rare (git init and clone always write the file); if the operator accepts it, the override below records that.
2. **The mutation pass is not fresh at HEAD.** The fixes after 04-18's pass added code a mutant survives in: the
   CR-01 fix's non-NotFound `metadata` branch for `packed-refs` has no test, so turning that refusal into
   "no replace refs" would pass every gate.

## Goal Achievement

### Observable Truths (roadmap success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Proptest round trip of any git path C-quoted; `.planning` covers `.planning/x` not `.planningX`; unknown line kind is an error naming the line | VERIFIED | `any_git_path_round_trips`, `property_runs_its_configured_cases`, `unknown_kind_names_the_line`, `covers_matches_a_literal_pathspec` PASS in the verifier's `scripts/check.sh all` at 7a4201e (707 run, 707 passed) |
| 2 | Chain develop, next, remote default; `wsx.baseRef` overrides; fork remotes base=upstream push=origin; both default origin | VERIFIED | `develop_beats_the_remote_default`, `next_follows_develop`, `remote_default_follows_next`, `base_ref_overrides_the_chain`, `fork_layout_resolves_each_remote`, `remotes_default_to_origin`, plus the D-34 `%(symref)` tests (`remote_head_chain_inside_the_remote_resolves`, `..._into_a_local_branch_is_not_a_default`, `remote_head_cycle_is_not_a_default`) PASS |
| 3 | `wsx.sourceBranch` names the workspace; else the only `workspace/*`/`personal/*`; zero or several is an error listing them | VERIFIED | `source_branch_names_the_workspace`, `only_candidate_is_inferred`, `several_candidates_are_listed`, `zero_candidates_are_an_error`, `private_branch_in_a_linked_worktree_is_not_a_candidate`, `corrupt_candidate_ref_is_skipped_as_git_skips_it` PASS |
| 4 | List from `<tip>:.wsx.conf` in the object database; working-tree edit changes nothing; clean checkout same list; only the two built-ins | VERIFIED | `working_tree_edit_changes_nothing`, `clean_checkout_resolves_the_same_list`, `undeclared_path_is_not_personal`, `declared_path_is_personal`, `partial_clone_manifest_is_unreadable_without_a_fetch` PASS; `build` (mod.rs:523-566) reads the manifest only from `read_tips` over the listed tip id, after `refuse_replace_refs` (mod.rs:533) |
| 5 | `wsx.sourceBranch` set + missing/invalid manifest fails closed with the reason; no key is reported not configured | PARTIAL (gap) | The named cases hold: `missing_manifest_fails_closed`, `invalid_manifest_fails_closed`, `no_source_branch_is_not_configured`, `global_source_branch_does_not_configure` PASS. Counterexample reproduced: own `.git/config` removed plus a global `[remote "up"]`, `git status` exits 0 and `state=failed: canonicalize .../.git/config: No such file or directory`; the same layout with the file present gives `state=not-configured` |

### Previous gap (re-verified in full)

| Truth | Status | Evidence |
|-------|--------|----------|
| 04-11: a replacement gix and git follow differently is refused, and wsx never reads manifest bytes other than git's | VERIFIED (closed) | `nested_replacement_fails_closed`, `packed_nested_replacement_fails_closed`, `single_replacement_fails_closed`, `private_replace_ref_in_a_linked_worktree_fails_closed` PASS. `refuse_replace_refs` (refs.rs:227-262) refuses on presence: any entry in `refs/replace` (common dir and a linked worktree's own) or the text `refs/replace/` anywhere in `packed-refs`, so nesting depth no longer matters. It runs in `build` before `read_tips`, the one object read, and the read itself is git's `cat-file`, so the bytes are git's even without the guard |

### Plan truths, 04-12 to 04-18

44 of 45 applicable truths VERIFIED (04-17's "under refuse" and "under accept" truths are not applicable: D-38 chose
drop). Method: every backticked identifier longer than 12 characters in all 18 plans' frontmatter and in the previous
report (266 names) was checked against the 707 PASS lines of the verifier's full run. The 52 not found are all
function or helper names, tests the plans delete (`root_entry_*`, `refs_directly_under_matches_for_each_ref`,
`find_ref_*`, 04-11's replacement-read tests), the four D-34 renames' old names, or `git_config_variable_is_refused`
(the refuse option D-38 rejected). None is a test a current truth needs.

| Plan | Truth | Status | Evidence |
|------|-------|--------|----------|
| 04-15, 04-16, 04-18 | D-25 inventory: deleted items absent, present at 4234f31 | VERIFIED | `ref_commit`, `refs_directly_under`, `SYMREF_MAX_DEPTH`, `RefTarget`, `root_entry`, `TreeNotSorted`, `local_config_values`, `remote_configured`: rg exit 1 at HEAD, git grep exit 0 at 4234f31. The one `fn find_ref` hit at HEAD is a gix trait impl inside a test double (repo.rs:1099), not the deleted `Repo::find_ref` |
| 04-14 | Config through one `git config -z --show-scope --show-origin` child; model reads config once | VERIFIED | `resolve` calls `repo.read_config(git)?` once (mod.rs:373); no `gix::` in `src/model` (rg exit 1; control `src/git` exit 0) |
| 04-16 | Four children per resolve | VERIFIED | `resolve_spawns_four_git_children_in_its_own_worktree`, `..._aimed_at_another_worktree_adds_the_env_var_query`, `resolve_without_one_candidate_skips_the_object_read` PASS |
| 04-17 | Operator answered before code; drop implemented | VERIFIED | STATE decision committed 383c74b at 17:14, RED 14745d8 at 18:34; D-38 in CONTEXT; `read_config` is the only non-test caller of `run_without_var` (config.rs:73); `git_config_variable_does_not_unconfigure_a_repo`, `run_without_var_hides_only_that_variable` PASS |
| 04-18 | D-26 latency, four children, p95 < 100 ms | VERIFIED (re-measured at HEAD) | 04-18's probe run by the verifier on a `git archive` of 7a4201e, release build: devflow p95 31.4 ms, wsx 23.3, gsd-core 29.8, gsd-pi 34.1, all `children=4`, `over_budget=0`. One run; p50 is about twice 04-18's figures (taken before the deadline poll loop landed and on a less loaded host), still well under budget |
| 04-18 | STATE obligations (read order, exit codes, grafts and shallow, pivot decision) | VERIFIED | STATE.md:258 holds the `info/grafts` obligation; `refuse_replace_refs` and `model::resolve` lines present |
| 04-18 | Fresh mutation pass after the last src commit, 0 missed | FAILED (gap) | See gaps: 1 missed of 43 in the post-pass fixes |
| 04-18 | `all`, `deps`, `coverage`, container `all` exit 0 on the tip | VERIFIED (container executor-reported) | Verifier at 7a4201e: `check.sh all` exit 0 (`==> check.sh: all OK`), `deps` exit 0, `coverage` exit 0 (93.41% lines). Container: recorded as passing after the CR-01 fix (04-DIFF-REVIEW-R1.md); `git diff d8cae67 HEAD` touches only four `.planning` files. Not re-run, per the orchestrator |
| post-04-18 | D-39: every git child stopped past 10 s; gix's open bounded; special `packed-refs` refused unopened | VERIFIED | `run_stops_a_child_past_its_deadline`, `run_with_input_stops_a_child_past_its_deadline`, `run_without_var_stops_a_child_past_its_deadline`, `run_keeps_a_child_within_its_deadline`, `config_read_stops_on_a_fifo_branch_under_an_onbranch_include`, `discover_stops_on_a_fifo_config`, `discover_stops_on_a_fifo_packed_refs`, `packed_refs_that_are_a_fifo_are_refused_unopened` PASS |

### Rulings on the executors' human_judgment items

| Item | Ruling | Basis |
|------|--------|-------|
| 04-15 D5: `require_passes_a_git_failure_through` re-pointed to a new fixture, against the "no D-01 to D-21 test changes beyond four" prohibition | Within the prohibition's intent; no change needed (decided by verifier) | The old trigger (a `garbage` loose ref under `refs/heads/workspace/`) can no longer produce a git failure, because D-35 makes the listing skip a corrupt ref as git does (pinned by `corrupt_candidate_ref_is_skipped_as_git_skips_it`). The asserted behaviour (D-16: `require` passes a non-replacement `Error::Git` through rather than wrapping it as not configured) is unchanged, and the new fixture asserts git's refusal first (`git_cannot_verify` on a blob rung) |
| 04-16 deviations: the same test's and `require_wraps_an_unreadable_manifest_as_not_configured`'s inner error changed; `git::objects::tests::` added to `DISCOVERING_SUITES` in testing.rs, outside Task 2's list | Acceptable (decided by verifier) | Both tests keep their outer assertion (passed through unwrapped; wrapped as not configured, with the restore control); only the inner cause moved from a deleted gix error to git's `ObjectMissing`. D-37 names only `unsorted_tree_manifest_fails_closed`, so these two are an extension of its rule by the executor, not covered by it literally; the operator may want D-37 widened to say so. The `DISCOVERING_SUITES` line is test registration an existing meta-test requires, not a behaviour change |

### Prohibitions

| Plan | Prohibition | Ruling | Basis |
|------|-------------|--------|-------|
| 04-12 | Must not read manifest bytes other than the ones git reads | Upheld | Presence guard before the only object read; the read is git's own `cat-file` |
| 04-13 | Must not read any object through gix while a replace-ref store holds an entry (test tier) | Upheld | The model calls no gix object read (04-16); guard tests PASS |
| 04-13 | Must not blame `.wsx.conf` for the guard's own read failures (test tier) | Upheld | `unreadable_replace_dir_blames_the_replace_refs`, `unreadable_private_replace_dir_blames_the_replace_refs` PASS. The missed mutant above is the one hole: an unreadable `packed-refs` other than a fifo or directory is untested |
| 04-14 | Global, system, include, worktree or command values must not configure a repo (test tier) | Upheld | `parse_config_z_keeps_only_the_own_local_file`, `global_source_branch_does_not_configure`, `included_source_branch_does_not_configure` PASS |
| 04-14 | No config read through gix; no backstop (test tier) | Upheld | Inventory above |
| 04-14 | Malformed config output is not fewer keys (test tier) | Upheld | `parse_config_z_refuses_a_truncated_record`, `parse_config_z_refuses_a_scope_git_does_not_print`, `parse_config_z_refuses_a_local_key_whose_origin_cannot_be_resolved` PASS |
| 04-15 | No by-name ref read; exact-name filter (test tier) | Upheld | `fifo_ref_is_skipped_without_hanging`, `nested_ref_under_a_rung_name_does_not_satisfy_the_rung` PASS |
| 04-15 | No D-01 to D-21 test change beyond four (judgment) | Upheld | Ruling above |
| 04-16 | Only `run_with_input` gets stdin; no network; no symlink text as manifest; at most four children (test tier) | Upheld | `run_with_input_feeds_exactly_the_buffer_then_eof`, `children_get_null_stdin`, `partial_clone_manifest_is_unreadable_without_a_fetch`, `symlink_manifest_fails_closed`, the child-count tests PASS |
| 04-17 | No option before the operator's answer (judgment) | Upheld | Commit order above |
| 04-17 | Only `GIT_CONFIG` on the config child loses a variable (test tier) | Upheld | Single non-test caller; `run_without_var_hides_only_that_variable` PASS |
| 04-18 | Must not weaken a production check to make a mutant equivalent; no timeout as a pass (judgment) | Upheld | Equivalent list is empty; 04-18's pass recorded 0 timeouts |
| 04-18 | Must not edit STATE.md by hand (test tier) | Unverified, flagged | No enforcement is wired and a verb-written line cannot be told from a hand-written one after the fact; the evidence is the SUMMARY's claim only. Human review recommended |

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | "Every command resolves" the model: no production caller yet | Phase 5 | Phase 5 goal sets the contract "every later command must join"; STATE.md obligation that every command takes its model from `model::resolve`/`require` first |

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| CONF-01 | 04-03, 04-04, 04-18 | SATISFIED | SC 1 |
| CONF-02 | 04-09, 04-16, 04-18 | SATISFIED | SC 4; `personal_paths` hard-codes only `.wsx.conf` and the worktree dir |
| CONF-03 | 04-07, 04-15, 04-18 | SATISFIED | SC 2 |
| CONF-04 | 04-06, 04-14, 04-17, 04-18 | SATISFIED | SC 2; D-38 |
| CONF-05 | 04-08, 04-14, 04-15, 04-18 | SATISFIED | SC 3 |
| CONF-07 | 04-01, 04-09, 04-11 to 04-18 | PARTIAL | SC 4 holds; SC 5 fails in the missing-`.git/config` layout (gap 1); REQUIREMENTS.md already marks it Complete |

No orphaned requirement: REQUIREMENTS.md maps CONF-01 to CONF-05 and CONF-07 to Phase 4 and every one is claimed by a plan;
CONF-06 maps to Phase 8.

### Anti-Patterns and open review findings

No `TBD`, `FIXME`, `XXX`, `TODO` or `HACK` in `src` or `tests/harness` (rg exit 1).

04-REVIEW-DISPOSITION.md leaves 8 of 9 findings `open` (recorded, not triaged). Besides WR-03 (gap 1):

| Finding | Severity | Effect on this phase |
|---------|----------|----------------------|
| WR-01: `collect` waits for pipe EOF and kills only the direct child | Warning | The reviewer says no phase-4 child starts a descendant; it bites later phases that run `git push`, `commit` or hooks through `Git`. Needs a disposition before those phases |
| WR-02: the `RefsNamespace` refusal gives a cause that is no longer true | Warning | Wrong error text, behaviour unchanged |
| WR-04: a symbolic workspace branch is followed, a symbolic rung refused | Warning | An inconsistency; SC 3's named cases hold |
| IN-01 to IN-04 | Info | Early error returns in `collect` leave a child running without its deadline (IN-01) is the one that weakens D-39 |

### Human Verification Required

None that blocks the status. One flagged prohibition (04-18, STATE.md edited only through the verbs) has no
evidence beyond the SUMMARY.

### Gaps Summary

Both gaps come from code added after the last full gate: gap 1 from 04-14's config read, gap 2 from the
post-04-18 fixes. Each closes with one code change and one test, then a fresh mutation pass on the new tip.

If the operator accepts gap 1 as a rare layout rather than fixing it, add to this frontmatter:

```yaml
overrides:
  - must_have: "Roadmap SC 5 (CONF-07, D-16): a repo without wsx.sourceBranch is reported as not configured"
    reason: "Only a repository with no .git/config file is affected; it fails loud, never open"
    accepted_by: "{name}"
    accepted_at: "{ISO timestamp}"
```

---

_Verified: 2026-11-18T05:53:56Z_
_Verifier: Claude (gsd-verifier)_
