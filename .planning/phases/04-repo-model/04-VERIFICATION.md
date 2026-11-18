---
phase: 04-repo-model
verified: 2026-11-18T06:21:12Z
status: passed
score: 50/50 must-haves verified (5/5 roadmap success criteria; 45/45 applicable plan truths of 04-12 to 04-18; 04-01 to 04-11 regression-checked by the full suite)
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
covered_digest: "v4:sha256:9c405c82cc427266b0ae461a73fa4e3e0cb929b539eed743fa319c4907b185de"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 48/50
  gaps_closed:
    - "Roadmap SC 5 (CONF-07, D-16): a repo without wsx.sourceBranch is reported as not configured, now also with no .git/config file and a global remote.* or wsx.* record"
    - "04-18: a fresh full cargo-mutants pass, run after the last commit touching src/, leaves no missed or timed-out mutant"
  gaps_remaining: []
  regressions: []
deferred:
  - truth: "Every command resolves the model (no production caller of model::resolve yet; src/model/mod.rs carries expect(dead_code, reason = \"no caller until phase 5\"))"
    addressed_in: "Phase 5"
    evidence: "Phase 5 goal: 'it sets the versioned JSON and exit-code contract every later command must join'; STATE.md obligation: every wsx command gets its model from model::resolve or require before any other Repo read"
human_verification:
  - test: "04-18 prohibition 'MUST NOT edit STATE.md by hand; only the gsd-tools state verbs' (test tier, no wired enforcement): confirm from commit 962cd70"
    expected: "Recommended resolution (verifier, from the diff): upheld. 962cd70's STATE.md diff is exactly the verbs' shape: two blocker lines removed in place (state.resolve-blocker x2), three appended at the list's end (state.add-blocker x3), one decision appended at the end of Decisions (state.add-decision), plus last_updated and state_head bumped; +6/-4 lines, nothing rewritten in place"
    why_human: "A test-tier prohibition with no enforcement is flagged by policy, never green; a hand edit producing the same lines cannot be excluded after the fact"
---

# Phase 4: Repo model Verification Report

**Phase Goal:** Every command resolves one repo model: the integration branch, the workspace branch, the base and push remotes, and a single personal-path list read from the workspace branch tip's `.wsx.conf`
**Verified:** 2026-11-18T06:21:12Z
**Status:** human_needed (one policy-flagged prohibition, answerable from the evidence below; no gap)
**Re-verification:** Yes, after closure of the two gaps in the report committed at 8191252

## Verdict

Both gaps are closed at aca0250, checked by the verifier, not taken from the commits:

1. **SC 5 holds in the missing-`.git/config` layout.** The verifier's own end-to-end reproduction (a scratch test
   through `model::tests`' `state_child`: `repo_with_remote`, a global `[remote "up"]` url, `.git/config` removed,
   `git status` exit 0 asserted first) gives `not-configured` at HEAD. Negative control: the same test on the
   8191252 tree fails with `left: "failed: canonicalize .../work/.git/config: No such file or directory (os error 2)"`.
2. **The mutation pass is fresh at HEAD.** `target/gsd/04-review-mutants/mutants.out`: 46 listed, outcomes 41 caught,
   5 unviable, 0 missed, 0 timeout; written 01:15, after d61cf32 (01:11), and `git diff d61cf32 aca0250 -- src tests
   Cargo.*` is empty. The previously missed `refs.rs:307:21` guard mutants (`true` and `false`) and the new
   `config.rs:92:25` NotFound guard mutants (`true` and `false`) are all in `caught.txt`. Not re-run, per the orchestrator.

The status is `human_needed` only because one test-tier prohibition (04-18, STATE.md through the verbs) has no wired
enforcement; the evidence settles it (see Human Verification).

## Goal Achievement

### Observable Truths (roadmap success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Proptest round trip of any git path C-quoted; `.planning` covers `.planning/x` not `.planningX`; unknown line kind is an error naming the line | VERIFIED (regression) | Verifier's `scripts/check.sh all` at aca0250: exit 0, `==> check.sh: all OK`, 710 run, 710 passed, 63 skipped |
| 2 | Chain develop, next, remote default; `wsx.baseRef` overrides; fork remotes base=upstream push=origin; both default origin | VERIFIED (regression) | Same run; no src change touches refs listing or chain code (diff since 8191252: `config.rs` read path, one new test in `refs.rs`) |
| 3 | `wsx.sourceBranch` names the workspace; else the only `workspace/*`/`personal/*`; zero or several is an error listing them | VERIFIED (regression) | Same run |
| 4 | List from `<tip>:.wsx.conf` in the object database; working-tree edit changes nothing; clean checkout same list; only the two built-ins | VERIFIED (regression) | Same run |
| 5 | `wsx.sourceBranch` set + missing/invalid manifest fails closed with the reason; no key is reported not configured | VERIFIED (gap closed) | `read_config` maps a NotFound canonicalize of the own config to `None` (config.rs:90-99) and `is_own_file` then answers `Some(false)` for every record (config.rs:205-209); any other canonicalize error still fails naming the file. Tests `config_read_without_an_own_config_file_keeps_no_keys` (with a present-file control) and `config_read_with_an_unresolvable_own_config_file_is_an_error` (self-symlink, `Error::Io` naming `.git/config`) PASS. End-to-end through the model: verifier's scratch reproduction PASS at HEAD, FAIL with the canonicalize error at 8191252 |

**Score:** 50/50 truths verified (0 present, behavior-unverified)

### Previous gaps (re-verified in full)

| Truth | Status | Evidence |
|-------|--------|----------|
| SC 5 in the missing-`.git/config` layout (WR-03) | VERIFIED (closed) | Above. The suite pins it at the `read_config` level (no own keys); the model step from "no own keys" to `NotConfigured` is the existing `no_source_branch_is_not_configured` path, and the verifier's reproduction joins the two |
| 04-18: fresh mutation pass after the last src commit, 0 missed | VERIFIED (closed) | Above. New test `packed_refs_whose_metadata_fails_are_unreadable` (self-symlink `packed-refs`, control asserts `metadata` fails with a non-NotFound error, then `ReplaceRefsUnreadable` naming the path) PASS and kills both `refs.rs:307:21` mutants |

### Plan truths, 04-12 to 04-18

Unchanged from the previous report (44 VERIFIED there) plus the closed mutation-pass truth: 45/45 applicable VERIFIED.
04-17's "under refuse" and "under accept" truths remain not applicable (D-38 chose drop). Regression evidence: the
verifier's full run at aca0250 (710 passed).

### Rulings on the executors' human_judgment items

| Item | Ruling | Basis |
|------|--------|-------|
| 04-15 D5: `require_passes_a_git_failure_through` re-pointed to a new fixture | Within the prohibition's intent (decided by verifier) | The old trigger can no longer produce a git failure (D-35 skips a corrupt ref as git does); the asserted D-16 behaviour is unchanged and the new fixture asserts git's refusal first |
| 04-16: the inner error of `require_passes_a_git_failure_through` and `require_wraps_an_unreadable_manifest_as_not_configured` changed; `git::objects::tests::` added to `DISCOVERING_SUITES` | Acceptable, now covered by the decision record (decided by verifier) | D-37 widened at aca0250 (04-CONTEXT.md) names both tests and their new inner causes; outer behaviour unchanged. The `DISCOVERING_SUITES` line is test registration an existing meta-test requires |

### Prohibitions

All rulings of the previous report stand (no code they rest on changed, except the 04-13 "must not blame
`.wsx.conf` for the guard's own read failures" hole, now closed by `packed_refs_whose_metadata_fails_are_unreadable`).
The one open item:

| Plan | Prohibition | Ruling | Basis |
|------|-------------|--------|-------|
| 04-18 | Must not edit STATE.md by hand (test tier) | Unverified by policy, flagged; evidence says upheld | No enforcement wired. 962cd70's diff has the verbs' shape exactly (removals in place, appends at list ends, `last_updated` and `state_head` bumped, +6/-4); the other 04-18-era STATE commits (5765862, 3bc206a) are plan-completion and decision records |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/git/config.rs` | Own config optional in the config read | VERIFIED | `Option<&Path>` threaded through `parse_config_z` and `is_own_file`; error path kept for non-NotFound |
| `src/git/refs.rs` | Test for non-NotFound `packed-refs` metadata | VERIFIED | Test present and passing; production code unchanged since 8191252 |
| `04-CONTEXT.md` D-37 | Widened to the two 04-16 tests | VERIFIED | Lines 205-207 |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Project gate | `scripts/check.sh all` | exit 0, 710 passed | PASS |
| SC 5, no own config, through the model | scratch test `zz_verifier_no_own_config_is_not_configured` on a `git archive` of HEAD, `--no-tests=fail -E 'test(=...)'` | PASS | PASS |
| Negative control for the above | same test on a `git archive` of 8191252 (sources touched to force a rebuild; the first attempt reused HEAD's binary and was discarded) | FAIL, `failed: canonicalize .../.git/config: No such file or directory` | PASS (control bites) |

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| CONF-01 | 04-03, 04-04, 04-18 | SATISFIED | SC 1 |
| CONF-02 | 04-09, 04-16, 04-18 | SATISFIED | SC 4 |
| CONF-03 | 04-07, 04-15, 04-18 | SATISFIED | SC 2 |
| CONF-04 | 04-06, 04-14, 04-17, 04-18 | SATISFIED | SC 2; D-38 |
| CONF-05 | 04-08, 04-14, 04-15, 04-18 | SATISFIED | SC 3 |
| CONF-07 | 04-01, 04-09, 04-11 to 04-18 | SATISFIED | SC 4 and SC 5 (gap closed) |

No orphaned requirement: REQUIREMENTS.md maps CONF-01 to CONF-05 and CONF-07 to Phase 4, each claimed by a plan;
CONF-06 maps to Phase 8.

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | "Every command resolves" the model: no production caller yet | Phase 5 | Phase 5 goal sets the contract "every later command must join"; STATE.md obligation that every command takes its model from `model::resolve`/`require` first |

### Anti-Patterns and open review findings

No debt markers added (the diff since 8191252 adds none). `04-REVIEW-DISPOSITION.md` still lists WR-03 as `open`
(line 54) although 9b996b1 fixes it; the record is stale, not the code. The other open findings (WR-01, WR-02,
WR-04, IN-01 to IN-04) are unchanged from the previous report and none blocks this phase's criteria.

### Human Verification Required

### 1. 04-18 STATE.md edited only through the verbs

**Test:** Compare 962cd70's STATE.md diff with the output shape of `state.resolve-blocker`, `state.add-blocker` and `state.add-decision`.
**Expected:** Upheld: the diff is removals in place, appends at the ends of Blockers and Decisions, and the verbs' `last_updated`/`state_head` bump; no in-place rewrite.
**Why human:** Policy: a test-tier prohibition without wired enforcement is never auto-green. The verifier's recommended resolution is "upheld, decided on the diff shape".

### Gaps Summary

None. Both previous gaps are closed with code, tests and a fresh mutation pass at HEAD.

---

_Verified: 2026-11-18T06:21:12Z_
_Verifier: Claude (gsd-verifier)_
