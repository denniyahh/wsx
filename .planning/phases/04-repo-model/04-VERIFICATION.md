---
phase: 04-repo-model
verified: 2026-11-08T20:30:00Z
status: gaps_found
score: 100/101 must-haves verified (5/5 roadmap success criteria; 95/96 plan truths; 10/11 prohibitions upheld)
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
  - "Cargo.lock"
  - "Cargo.toml"
  - "src/git/config.rs"
  - "src/git/mod.rs"
  - "src/git/quote.rs"
  - "src/git/refs.rs"
  - "src/git/repo.rs"
  - "src/git/testing.rs"
  - "src/main.rs"
  - "src/model/manifest.rs"
  - "src/model/mod.rs"
  - "tests/harness/fixture.rs"
covered_digest: "v4:sha256:af7725fc67804c277afe323e505b49e70dcd322bfcd363b6b1e5f23f879c38f2"
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "04-11 (CONF-07): a replacement gix and git follow differently is refused, rather than read; and its prohibition MUST NOT read manifest bytes other than the ones git reads"
    status: failed
    reason: "A nested replace ref (refs/replace/<dir>/<hex>) is applied by git and ignored by gix, and refuse_unfollowable_replacements lists only refs directly under the replace base, so it returns Ok and resolve yields a Configured model whose personal-path list comes from bytes git does not show. Reproduced end to end in wsx's own code (verifier probe, loose and packed refs, with a direct-ref control in the same test)."
    artifacts:
      - path: "src/git/refs.rs"
        issue: "refuse_unfollowable_replacements (equal-base branch, line 349) uses refs_directly_under, whose one-level filter (lines 283-287) drops every nested replace ref; the doc comment at lines 294-296 ('returns the bytes git reads or nothing at all') is false under a nested ref"
      - path: "src/model/mod.rs"
        issue: "no test plants a nested replace ref; the 04-11 suite covers only direct refs, so the gap passed every gate including the mutation pass"
    missing:
      - "List the replace base at any depth; refuse a nested ref whose last path component is a full-length hex object id (git keys on the last component, gix parses the whole remainder and ignores it)"
      - "Key the duplicate and chain checks on the last component across all depths, so a nested ref cannot hide a chain or a duplicate"
      - "A model test: commit a manifest, plant refs/replace/<dir>/<manifest blob> -> another blob, assert git cat-file prints the replacement (control) and resolve fails closed naming the ref; plus a packed-refs variant"
deferred:
  - truth: "Every command resolves the model (no production entry point calls model::resolve yet; src/model/mod.rs carries expect(dead_code, reason = \"no caller until phase 5\"))"
    addressed_in: "Phase 5"
    evidence: "Phase 5 goal: 'it sets the versioned JSON and exit-code contract every later command must join'; STATE.md records 'Phase 5: every wsx command gets its model from model::resolve or require before any other Repo read'"
---

# Phase 4: Repo model Verification Report

**Phase Goal:** Every command resolves one repo model: the integration branch, the workspace branch, the base and push remotes, and a single personal-path list read from the workspace branch tip's `.wsx.conf`
**Verified:** 2026-11-08T20:30:00Z
**Status:** gaps_found
**Re-verification:** No, initial verification

## Verdict in one paragraph

All five roadmap success criteria hold in the scenarios they name, and the model is real, substantive
and layered (no gix type in `src/model`). One phase must-have fails: plan 04-11's claim, made for
CONF-07, that wsx never reads `.wsx.conf` bytes other than the ones git reads. A nested replace ref
(`refs/replace/<dir>/<hex>`) makes git and wsx read different manifests with no error. This was the
reviewer's CR-01; the verifier reproduced it end to end in wsx's own code, so it is not an inference
from a gix probe. It is a gap against the phase goal's "single personal-path list" and against CONF-07
as phase 4 defined it (D-22: "a silent mismatch with git is the worse failure"), so the status is
`gaps_found`. The fix is narrow (one function in `src/git/refs.rs` plus a test).

### Why CR-01 is a gap, not an accepted risk or an advisory

- **It breaks a stated must-have.** 04-11's first truth says "A replacement gix and git follow
  differently is refused ... rather than read", and its prohibition says "MUST NOT read manifest
  bytes other than the ones git reads". A nested ref is exactly such a replacement, and it is read.
- **It breaks the goal's "single list".** Later phases run git children (cherry-pick, `git show`) that
  see the replaced bytes, while the model holds the original ones. That makes two personal-path lists
  in one run, which is the failure CONF-02 and CONF-07 exist to prevent.
- **No acceptance covers it.** No VERIFICATION override exists. AR-04-03 accepts only a replace ref that
  changes during one run. T-04-30 is reopened in 04-SECURITY.md (rated medium, below the security
  gate's blocking threshold). That threshold governs the security gate. It does not turn a failed
  must-have into a pass here.
- **What argues against blocking.** The trigger needs a nested replace ref whose last component names
  an object on the manifest's read path: the tip commit, its root tree or the `.wsx.conf` blob. A
  fetched `+refs/replace/*:refs/replace/*` refspec can bring one in, but it is rare. Severity is the
  operator's call. If they accept it, the override below records that and the status can be recomputed.
  The verifier does not recommend that, because the fix is small and the failure is silent.

## Goal Achievement

### Observable Truths (roadmap success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Proptest: any git path round-trips C-quoted; `.planning` covers `.planning/x` not `.planningX`; unknown line kind is an error naming the line | VERIFIED | `model::manifest::tests::any_git_path_round_trips` passed (strategy: bytes 1-255 except `/`, so newline, quote, backslash, non-UTF-8, spaces; 2048 cases); `property_runs_its_configured_cases` passed (guards against `PROPTEST_CASES=0`); `unknown_kind_names_the_line`, `covers_matches_a_literal_pathspec` passed; `covers` at manifest.rs:315 is a prefix-plus-`/` test |
| 2 | Integration chain develop -> next -> remote default, `wsx.baseRef` overrides; fork remotes base=upstream push=origin, both default origin | VERIFIED | `develop_beats_the_remote_default` (asserts tip against `git show-ref` and unequal to skipped rungs), `base_ref_overrides_the_chain`, `fork_layout_resolves_each_remote`, `remotes_default_to_origin`, `push_default_is_not_read`, `upstream_is_not_auto_detected`, `main_is_never_guessed` all passed; `integration` (mod.rs:556) reads each rung once via `find_ref` and refuses symbolic rungs |
| 3 | `wsx.sourceBranch` names the workspace; else the only `workspace/*`/`personal/*`; zero or several is an error listing them | VERIFIED | `source_branch_names_the_workspace`, `only_candidate_is_inferred`, `several_candidates_are_listed` (asserts `personal/workspace, personal/workspace-backup`), `zero_candidates_are_an_error`, `checked_out_branch_gets_no_preference`, `several_candidates_do_not_fail_resolve` passed |
| 4 | Personal list from `<tip>:.wsx.conf` in the ODB; working-tree edit changes nothing; clean checkout same list; only built-ins `.wsx.conf` + worktree dir; undeclared `.planning`/`CLAUDE.md` not personal, declared is | VERIFIED (scenarios as written; see gap for the replace-ref case) | `working_tree_edit_changes_nothing` (asserts `git status` shows ` M .wsx.conf`), `clean_checkout_resolves_the_same_list`, `undeclared_path_is_not_personal`, `declared_path_is_personal`, `modified_path_is_not_personal`, `hook_environment_resolves_the_same_list`, `configured_repo_resolves_from_the_object_database`, `model_moves_with_the_workspace_tip` (control against a constant) passed; `personal_paths` (mod.rs:614) hard-codes only `MANIFEST_NAME` and `worktree_dir` |
| 5 | `wsx.sourceBranch` set + missing/invalid manifest fails closed with the reason; no key is reported not configured | VERIFIED | `missing_manifest_fails_closed`, `invalid_manifest_fails_closed`, `symlink_manifest_fails_closed`, `no_source_branch_is_not_configured`, `require_wraps_a_non_blob_manifest_as_not_configured` passed; `resolve` returns `Err` only on the configured path (mod.rs:357-382) |

### Plan truths

95 of 96 VERIFIED. Every test that a plan truth names exists in `cargo nextest list`: the names were
extracted from all 11 plans' `must_haves.truths` and diffed against the list, and the only names
missing from it are function identifiers, not tests. 38 were run one at a time and each passed. A
nonexistent test name exits 4, which is the negative control. The orchestrator ran the rest in the
full suite at 74ba890 (639 passed), and nothing outside `.planning/` changed after that commit.

| Plan | Truth | Status | Evidence |
|------|-------|--------|----------|
| 04-11 | "A replacement gix and git follow differently is refused with `Error::Git` wrapping `ReplacementRef`, rather than read" | FAILED (BLOCKER) | Verifier probe in a scratch copy: `refs/replace/team/<manifest blob>` -> blob `only b`. git prints `only b\n`, while `resolve` returns `Ok(Configured)` with entries `[only .planning]`. The same holds after `git pack-refs --all`. Control in the same test: the direct `refs/replace/<blob>` gives `[.wsx.conf, .worktrees, b]` from wsx, which agrees with git. Cause: refs.rs:349 `refs_directly_under(&git_base)` with the one-level filter at refs.rs:283-287. gix side by source: gix-0.90.0 `src/open/repository.rs` parses `ObjectId::from_hex(name.strip_prefix(prefix))`, which fails on `team/<hex>`, so gix ignores the ref |
| 04-04 | `parse` and `covers` are pure functions (`verification: backstop`) | VERIFIED (decided by verifier) | Basis: the signatures take `&[u8]` and return owned values or a bool. The production half of manifest.rs (lines 1-319) has no `static`, `thread_local`, `Cell`, `Mutex`, `Atomic`, `unsafe`, `std::fs`, `std::env`, `std::io` or `Command` (rg exit 1). Negative control: the test half matches `Atomic` twice. Safe Rust cannot share mutable state without one of those, so the compiler enforces the property. A runtime test would add nothing |
| 04-01, 04-07 | Read-once tip races (T-04-37, T-04-38) | VERIFIED as stated | These truths claim the race is accepted, not mitigated. The acceptance is recorded in the doc comments at refs.rs:157-161 and in 04-SECURITY.md. Nothing more is claimed |
| 04-10 | Mutation pass: 0 missed, from a fresh run after the last code commit | VERIFIED from artifacts, not from the SUMMARY | `target/gsd/mutants/mutants.out/outcomes.json`: 177 mutants = 154 caught + 23 unviable, 0 missed, 0 timeout. The run started at 2026-11-08T18:18:38Z, after 09eca8c (13:18:29 -0500), the last commit that touches `src`, `Cargo.toml` or `Cargo.lock`. `missed.txt` is 0 lines, and there is no `previously_caught.txt` |
| 04-10 | Phase gate: `all`, `deps`, container `all` exit 0 | VERIFIED (container run is executor-reported) | `scripts/check.sh deps` re-run by the verifier: exit 0. `all` was measured by the orchestrator at 74ba890. The container run was at b505f2d, and `git diff b505f2d HEAD -- src Cargo.toml Cargo.lock tests scripts` is empty, so it covers the same code. The verifier did not re-run the container |
| 04-10 | No gix type in the model | VERIFIED | `rg -qF 'gix::' src/model` exit 1. Control: the same search over `src/git` exits 0 |
| 04-10 | STATE.md obligations for phases 5, 6, 8, 12 | VERIFIED | Each key-link pattern is present in `.planning/STATE.md`: `model::resolve` (3), `recorded set`, `before any other Repo read`, `ReplacementRef has no model::Error variant` and `Repo::discover refusal` (1 each) |
| 04-02 | Harness fixture writes bare `wsx.baseRef`, explicit remotes | VERIFIED | fixture.rs:671-673 and 831-833 hold the values. `git diff --name-only 6946ae9 -- tests/harness` prints only `fixture.rs` |

### Prohibitions (judgment tier)

These are rules the plans set: things the code must not do. The verifier checked each one against the
code and its tests. Ten hold. One is broken, the reproduced defect above, not a sign-off asked for by policy.

| Plan | Prohibition | Ruling | Basis |
|------|-------------|--------|-------|
| 04-01 | Must not read the manifest from the working tree | Upheld | `root_entry` reads the tip commit's tree (refs.rs:216-253); `working_tree_edit_changes_nothing` passed |
| 04-03 | Must not truncate or reinterpret a declared path | Upheld | proptest round trip plus `c_unquote` refusal tests; `path_value` / `normalize_path` reject rather than shorten |
| 04-04 | Must not silently drop or truncate a manifest line | Upheld | `parse` returns on the first refused line, filters nothing (manifest.rs:156-189) |
| 04-05 | Must not let config outside this repo's file configure it | Upheld | `global_source_branch_does_not_configure`, `included_source_branch_does_not_configure`, `global_key_is_not_read`, `refs_namespace_in_config_is_refused` passed |
| 04-06 | Must not guess the push or base remote | Upheld | `remote` (mod.rs:451) reads only the key or a locally defined `origin`; `push_default_is_not_read`, `upstream_is_not_auto_detected` passed |
| 04-07 | Must not guess the integration branch | Upheld | `CHAIN` is `develop`, `next` then `<remote>/HEAD` only; `main_is_never_guessed` passed |
| 04-08 | Must not pick among several candidates | Upheld | `several_candidates_are_listed`, `checked_out_branch_gets_no_preference` passed |
| 04-08 | Must not block an unconfigured repo | Upheld | `several_candidates_do_not_fail_resolve` passed; every no-key failure is a value in `NotConfigured` |
| 04-09 | Must not hard-code personal paths beyond the two built-ins | Upheld | `personal_paths` (mod.rs:614-624) |
| 04-09 | Must not fail open once configured | Upheld | missing/symlink/invalid manifest tests return `Err` from `resolve` |
| 04-11 | Must not read manifest bytes other than the ones git reads | **Broken** | Same reproduction as the failed 04-11 truth |

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | "Every command resolves" the model: no production caller exists yet | Phase 5 | Phase 5 goal sets the command contract "every later command must join"; STATE.md obligation that every command takes its model from `model::resolve`/`require` before any other `Repo` read |

**How "every command resolves one repo model" was judged in this phase:** at the model API only. There is
one entry point, `model::resolve(&Repo, &recorded) -> Result<RepoModel, Error>` (mod.rs:357). It returns
one `Model` carrying the workspace branch and tip, the `Integration` (branch, full ref, tip), the base and
push remotes, the worktree dir and the parsed manifest. Every consumer query goes through
`personal_paths()` or `is_personal()` on that value. `mod model;` is declared in main.rs:11. The
`expect(dead_code)` at mod.rs:15 is what proves nothing calls it outside tests: if a caller existed, the
expectation would be unfulfilled and the build would fail. Whether each command really uses this single
model is a phase 5+ property, and it is recorded there.

### Required Artifacts

`gsd-tools verify.artifacts` passed all 29 artifacts across the 11 plans: each exists and contains its
declared pattern. Spot reads confirmed the code is substantive, not stubs: `resolve`, `build`,
`integration` and `remote` in `src/model/mod.rs`, `parse`, `normalize_path`, `encode_path` and `covers`
in `src/model/manifest.rs`, and `find_ref`, `ref_commit`, `root_entry`, `refs_directly_under` and
`refuse_unfollowable_replacements` in `src/git/refs.rs`.

### Key Link Verification

`verify.key-links` passed the 10 links whose `from` is a bare path. It cannot parse the 18 whose `from`
also names a symbol ("Source file not found"). That is a limit of the tool, not a missing link.
The verifier checked each of those 18 by hand:

| From | To | Status | Where |
|------|----|--------|-------|
| `root_entry` | `peeled_commit` | WIRED | refs.rs:222 |
| `root_entry` | `refuse_unfollowable_replacements` | WIRED | refs.rs:217 (the guard itself is the CR-01 defect) |
| `root_entry` | tree entry mode `is_blob` | WIRED | refs.rs:238 |
| `build` | `root_entry` with the `ref_commit` id | WIRED | mod.rs:485-487 |
| `build` | `normalize_path` (via `worktree_dir`) | WIRED | mod.rs:480, 520 |
| `build` / `require` | `is_replacement_refusal` | WIRED | mod.rs:489, 124 |
| `single` | `local_config_values` | WIRED | mod.rs:415 |
| `remote` | `remote_configured` | WIRED | mod.rs:453 |
| `integration` | `find_ref` / `peel_commit` / base prefix | WIRED | mod.rs:558, 564, 571, 596 |
| `candidates` | `refs_directly_under` | WIRED | mod.rs:396 |
| `parse` | `normalize_path` / `covers` | WIRED | manifest.rs:220, 203 |
| `Repo::discover` | `refuse_refs_namespace` | WIRED | repo.rs:38 |
| model tests | `crate::git::testing` | WIRED | mod.rs:642 |
| `RepoModel::NotConfigured` | phase 6 guard | Not yet consumable | phase 6 obligation in STATE.md |

### Data-Flow Trace (Level 4)

| Artifact | Data | Source | Real data | Status |
|----------|------|--------|-----------|--------|
| `Model.manifest` | `.wsx.conf` bytes | `root_entry(workspace_tip)` -> gix ODB `find_blob` | Yes; under a nested replace ref they are the wrong bytes (gap) | FLOWING, with the gap |
| `Model.integration.tip` | commit id | `find_ref` -> `peel_commit` of that read's id | Yes, asserted against `git show-ref` | FLOWING |
| `Model.workspace_tip` | commit id | `ref_commit(refs/heads/<branch>)` | Yes | FLOWING |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 38 named tests that carry the success criteria and prohibitions | `scripts/check.sh test -E 'test(=<path>)'`, one test per command | each exit 0, "1 test run: 1 passed" | PASS |
| A filter that matches no test fails | `scripts/check.sh test -E 'test(=model::tests::no_such_test_name)'` | exit 4 | PASS (negative control) |
| Supply-chain gate | `scripts/check.sh deps` | exit 0, "advisories ok, bans ok, licenses ok, sources ok" | PASS |
| Nested replace ref (CR-01) | verifier probe test in a scratch copy, `cargo nextest run -E 'test(=model::tests::verifier_probe_nested_replacement)'` | git `only b`, wsx `[.planning]`, both loose and packed; direct-ref control agrees | FAIL (gap reproduced) |
| Stray candidate without an integration branch (WR-03) | scratch probe `verifier_probe_stray_candidate` | `require()` gives `NoIntegrationBranch`; with `develop` added it gives `NotConfigured { ManifestMissing }` | Reproduced (warning) |
| ESC in a remote name (WR-02) | scratch probe `verifier_probe_escape_in_remote` | rendered error holds a raw 0x1b: `read refs/remotes/a\u{1b}[2J/develop: ...` | Reproduced (warning) |

### Probe Execution

There are no `scripts/*/tests/probe-*.sh` files, and the phase documents declare none (`rg -q` exit 1).
This step does not apply.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| CONF-01 | 04-03, 04-04, 04-10 | SATISFIED | SC1 tests; grammar, overlap and 64 KiB limit in manifest.rs |
| CONF-02 | 04-01, 04-09, 04-10 | SATISFIED at the model API | built-ins only; one list from `personal_paths`. The guard, cut, check and status consumers come in phases 5-8 |
| CONF-03 | 04-02, 04-07, 04-10 | SATISFIED | SC2 chain tests; no `origin/HEAD`-first and no `main` guess |
| CONF-04 | 04-02, 04-06, 04-10 | SATISFIED | fork and default remote tests; `NoSuchRemote` with no fallback |
| CONF-05 | 04-08, 04-10 | SATISFIED (recorded-set exclusion proven at the input only, which the plan states; phase 8 obligation) | SC3 tests; `recorded_branches_are_not_candidates` |
| CONF-07 | 04-01, 04-05, 04-08, 04-09, 04-11, 04-10 | **BLOCKED (partial)** | ODB read, clean checkout, fail-closed and configured state all hold. The "same bytes git reads" half fails under a nested replace ref. REQUIREMENTS.md already marks CONF-07 `[x]` Complete, so that tick is premature until the gap closes |

No orphaned requirements. REQUIREMENTS.md maps CONF-01, -02, -03, -04, -05 and -07 to Phase 4, and
every one is claimed by a plan. CONF-06 maps to Phase 8.

### Anti-Patterns Found

The debt-marker scan found nothing: `rg` for TBD, FIXME, XXX, TODO, HACK and PLACEHOLDER over the 11
changed files exits 1. The code-review findings are classified below against the phase's must-haves.

| File | Line | Finding | Severity | Impact |
|------|------|---------|----------|--------|
| src/git/refs.rs | 283-287, 349 | CR-01: nested replace refs bypass the guard | BLOCKER | the gap above |
| src/git/refs.rs | 260-261, 333-349 | WR-01: a `GIT_REPLACE_REF_BASE` with no trailing `/` makes the guard check nothing | Warning | git itself aborts (`BUG: refs.c`) in that environment, so no silent mismatch; not re-run by the verifier |
| src/git/refs.rs | 79-82 | WR-02: `lossy` lets a raw control byte from config reach a rendered message through `Error::Git` | Warning | Reproduced by the verifier. It contradicts the comment at mod.rs:182-183. The 04-01 truth is scoped to the model's own variants, so no must-have fails |
| src/model/mod.rs | 477-501 | WR-03: `build` reads the integration branch before the manifest, so with no key, a stray candidate and no integration branch, `require()` says "no integration branch" rather than D-16's "not configured, run wsx init" | Warning | Reproduced by the verifier, with the control. `resolve` still returns `NotConfigured`, so SC5 and the guard state hold |
| src/model/mod.rs | 474-485 | WR-04: a symbolic workspace branch is followed (`ref_commit`), unlike a symbolic integration rung | Warning | Consistent with 04-01's `ref_commit` truth. The harm would land in later phases that write the ref; no test pins either choice |
| src/model/mod.rs | 292-296, 275-280 | IN-01 duplicate "run wsx init"; IN-02 C-quotes in the middle of a ref name | Info | message text only |

### Gaps Summary

There is one gap, and it has a single root cause. The D-22 replacement guard
(`refuse_unfollowable_replacements` in `src/git/refs.rs`) lists only the refs one level under the
replace base. git applies a replace ref by its last path component at any depth. gix applies only the
refs directly under the base. A nested ref therefore makes gix, and so wsx, read different objects than
git, and the guard never sees the ref. The verifier reproduced this in wsx's own `resolve`, loose and
packed, with a direct-ref control in the same test. The fix the reviewer gave (04-REVIEW.md CR-01) needs
two things: list the base at any depth and refuse a nested ref whose last component is a full hex id,
and key the duplicate and chain checks on the last component. Add a model test with a git control.
WR-01 lives in the same function, so it is cheap to fix in the same plan.

If the operator decides to accept the gap instead, record it like this, and the status can be recomputed:

```yaml
overrides:
  - must_have: "04-11: a replacement gix and git follow differently is refused, rather than read"
    reason: "{why a nested replace ref is acceptable}"
    accepted_by: "{name}"
    accepted_at: "{ISO timestamp}"
```

### What this verification does not establish

- The verifier did not re-run the full suite or the container gate. They rest on the orchestrator's
  run (74ba890) and the executor's run (b505f2d), both at code identical to HEAD.
- Several plan truths carry one-shot controls made in scratch copies (a guard stripped, an encoder
  broken, `unarray` banned). The verifier did not re-run them; they show the tests have teeth, not
  that the behavior holds.
- The nested-ref chain and duplicate variants (a direct ref plus a nested ref) were traced from code,
  not run. Only the single nested ref on the manifest blob was reproduced.
- Behavior was measured on git 2.56.0 only. The 2.54 floor is unexercised.

---

_Verified: 2026-11-08T20:30:00Z_
_Verifier: Claude (gsd-verifier)_
