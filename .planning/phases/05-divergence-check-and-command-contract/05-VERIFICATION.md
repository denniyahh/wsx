---
phase: 05-divergence-check-and-command-contract
verified: 2026-11-19T02:21:00Z
status: gaps_found
score: 4/5 roadmap success criteria verified (05-10 plan truths 8/8 pass)
covered_files: [".planning/phases/05-divergence-check-and-command-contract/05-01-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-01-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-02-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-02-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-03-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-03-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-04-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-04-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-05-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-05-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-06-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-06-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-07-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-07-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-08-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-08-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-09-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-09-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-10-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-10-SUMMARY.md","Cargo.lock","Cargo.toml","deny.toml","docs/exit-codes.md","docs/json/README.md","docs/json/check.v1.schema.json","src/check/hooks.rs","src/check/mod.rs","src/check/report.rs","src/check/staleness.rs","src/cli/commands.rs","src/cli/mod.rs","src/exit.rs","src/git/config.rs","src/git/mod.rs","src/git/objects.rs","src/git/reads.rs","src/git/refs.rs","src/git/repo.rs","src/git/spawn.rs","src/git/staleness.rs","src/git/testing.rs","src/git/trees.rs","src/json.rs","src/main.rs","src/model/mod.rs","tests/cli.rs","tests/harness/check.rs","tests/harness/contract.rs","tests/harness/if02_runtime.rs","tests/harness/main.rs","tests/harness/provenance.rs","tests/harness/registry.rs","tests/harness/rust.rs","tests/if02_scan.rs"]
covered_digest: "v4:sha256:8dd15232f2837e0a4828f74e7f37f16796f7d1ec93ec27fc77b6be9764083ec3"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/5
  previous_verified: 2026-11-19T01:14:57Z
  previous_gap: "SC5: tests/if02_scan.rs missed Path::new(\".planning\").join(\"config.json\"), escaped spellings (\"STATE\\x2emd\") and concat!; missing items were: add .planning to COMPONENTS, unescape literals, planted split-literal control"
  gaps_closed:
    - "Escaped spellings: literals matched by decoded value (planted_escaped_read_trips_the_scan, escaped_spellings_trip_the_scan PASS)"
    - "concat! spellings matched by joined value (concat_spellings_trip_the_scan PASS)"
    - "Split spelling: planted std::fs::read(Path::new(\".planning\").join(\"config.json\")) gives one finding (planted_join_read_trips_the_scan, join_spellings_trip_the_scan PASS)"
    - "Add .planning to COMPONENTS: superseded, not done; it contradicts D-14 (.planning is path data) and the join-chain rule covers the split read it aimed at. Accepted as superseded."
  gaps_remaining:
    - "SC5, narrowed: a lenient read of .planning/config.json whose .planning part is not a literal in a .join chain (review 05-REVIEW-GAP.md WR-02)"
  regressions: []
gaps:
  - truth: "SC5: A source-scan test fails on any read of ROADMAP.md/STATE.md contents, .planning/config.json, devflow.toml or .claude/.codex settings (control: a planted read trips it)"
    status: partial
    reason: "Operator option 1 (2026-11-19) closes SC5 by the token scan for its listed spellings plus the runtime comparison for reads that change check's output. One plausible read escapes both: a lenient reader of .planning/config.json that falls back to defaults on a parse failure (e.g. unwrap_or_default) and builds the path from a non-literal base (a variable, a const binding, or the manifest's .planning path data, then .join(\"config.json\")). The scan does not flag it: \"config.json\" alone is clean (path_data_does_not_trip_the_scan) and consts and variables are outside the scan by its own header. The runtime test does not see it: state A's config.json is {\"note\": \"if02 sentinel-A\"}, carrying no key a reader acts on, so a lenient reader gets defaults in A and, after the parse of B's malformed text fails, defaults in B too, and the output is the same in both states. Established by reading tests/harness/if02_runtime.rs:72-86 and tests/if02_scan.rs; not run with a planted lenient reader. For ROADMAP.md, STATE.md, devflow.toml and .claude/.codex, any literal naming them trips the scan, so the hole is specific to .planning/config.json."
    artifacts:
      - path: "tests/harness/if02_runtime.rs"
        issue: "State-A sentinels hold no real GSD/devflow/agent keys; the //! line 'a parse that fails on it also differs' holds only for a parse failure that reaches the output"
    missing:
      - "State A: write .planning/config.json, devflow.toml and .claude/settings.json with real keys at non-default values (e.g. a non-default git.branching_strategy and branch template)"
      - "A third state with well-formed, different non-default values, so a reader that falls back to defaults on malformed input still makes the states differ"
      - "A reader-variant control that parses leniently (defaults on failure) and must make the states differ"
      - "Narrow the //! sentence at if02_runtime.rs line 8 to what the states prove"
---

# Phase 5: Divergence check and command contract Verification Report

**Phase Goal:** `wsx check` reports, offline, every way the workspace branch differs from the integration branch that the manifest doesn't declare, and it sets the versioned JSON and exit-code contract every later command must join
**Verified:** 2026-11-19T02:21:00Z
**Status:** gaps_found
**Re-verification:** Yes, after gap-closure plan 05-10

Evidence base for this round, run by the verifier on HEAD 39286a2: 13 named tests each run alone through `scripts/check.sh test -E 'test(=...)'` (chained with `&&`, exit 0, 13 PASS lines): the five `if02_runtime::*` tests, `real_tree_reads_no_agent_file`, `planted_read_trips_the_scan`, `planted_escaped_read_trips_the_scan`, `planted_join_read_trips_the_scan`, `join_spellings_trip_the_scan`, `path_data_does_not_trip_the_scan`, `escaped_spellings_trip_the_scan`, `concat_spellings_trip_the_scan`; plus `registry::every_command_runs_in_a_repo_without_agent_files` (exit 0). Negative control: the same gate with a name that matches nothing exits 4. `scripts/check.sh all` was not re-run by the verifier; target/gsd/05-10-all.log (written at 1805a0b) ends with `==> check.sh: all OK`, and 1805a0b..HEAD touches only .planning/ files.

## Goal Achievement

### Observable Truths (roadmap success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Undeclared added/modified/deleted fails naming the path; declared passes; oracle recorded; offline with fetch age | ✓ VERIFIED (regression check) | Prior verdict stands: `git diff e2a7568 HEAD -- src Cargo.toml Cargo.lock docs deny.toml` is empty, so production code and contract docs are unchanged since the prior run |
| 2 | Stale declarations fail, each named | ✓ VERIFIED (regression check) | as SC1 |
| 3 | Hook mode from the commit; symlink passes; scripts/*.sh not checked | ✓ VERIFIED (regression check) | as SC1 |
| 4 | --json versioned schema, documented exit codes, contract test with control | ✓ VERIFIED (regression check) | as SC1; WR-01 of 05-REVIEW.md (exit 101 on broken stdout in human mode) still open, warning |
| 5 | Source-scan test fails on any read of the named files (planted control trips it); every command runs in a repo with none of them | ✗ PARTIAL | Second half holds (registry test PASS). First half: the three prior missing spellings are closed (gaps_closed). Remaining hole: a lenient, default-on-failure read of `.planning/config.json` with a non-literal `.planning` base is caught by neither the scan nor the runtime test (see gap) |

**Score:** 4/5 truths verified (0 present, behavior-unverified)

### SC5 under the operator's decision, plainly

The operator closed SC5 on 2026-11-19 by two tests: the token scan for the spellings it lists, and the runtime comparison for any read whose result reaches `wsx check`'s exit code, stdout or stderr. Both pass and both have working positive controls (three planted reads; four reader variants). Review 05-REVIEW-GAP.md WR-02 is correct: the runtime states cannot detect a reader that falls back to defaults, because state A holds no key such a reader uses, so A and B both yield defaults. That alone would not matter where the scan catches the spelling. For `ROADMAP.md`, `STATE.md`, `devflow.toml`, `.claude` and `.codex`, the scan flags any literal that names them, so a read needs a contrived spelling (`format!`, fragments) to slip past it. For `.planning/config.json` it does not: `.planning` alone is path data by D-14 and `"config.json"` alone is clean, so `planning_dir.join("config.json")` with a lenient parse passes both gates. wsx already holds `.planning` as manifest path data, so this is a natural shape for a regression, not a contrived one. SC5 therefore does not hold for that shape. The fix is local to the runtime test (gap `missing`). If the operator judges it acceptable, record:

```yaml
overrides:
  - must_have: "A source-scan test fails on any read of ROADMAP.md/STATE.md contents, .planning/config.json, devflow.toml or .claude/.codex settings"
    reason: "<why a lenient read of .planning/config.json through a non-literal base is acceptable>"
    accepted_by: "<name>"
    accepted_at: "<ISO timestamp>"
```

### 05-10 plan must-haves

| # | Truth (short) | Status | Evidence |
|---|---------------|--------|----------|
| 1 | check / check --json identical across states A and B; A exits 0 passing | ✓ VERIFIED | if02_runtime::check_output_does_not_depend_on_agent_files PASS |
| 2 | Four reader variants make the states differ, carrying their sentinels | ✓ VERIFIED | tracked_/ignored_/object_/home_read_makes_the_states_differ PASS |
| 3 | Escaped spellings trip once; planted escaped read trips once; malformed escape is cannot-tokenize | ✓ VERIFIED | escaped_spellings_trip_the_scan, planted_escaped_read_trips_the_scan PASS; malformed_escapes_fail_closed_at_the_lexer covered by check.sh all log only |
| 4 | concat! spellings trip once | ✓ VERIFIED | concat_spellings_trip_the_scan PASS |
| 5 | .join chains trip once; planted join read trips once | ✓ VERIFIED | join_spellings_trip_the_scan (includes `root.join(".planning").join("config.json")`), planted_join_read_trips_the_scan PASS |
| 6 | Path data stays clean (D-14) | ✓ VERIFIED | path_data_does_not_trip_the_scan PASS; lookalike tests in the check.sh all log only |
| 7 | Real src/ scans clean; 05-05 tests unchanged and passing | ✓ VERIFIED | real_tree_reads_no_agent_file, planted_read_trips_the_scan PASS; the other four in the check.sh all log |
| 8 | scripts/check.sh all exits 0 with "all OK" | ✓ VERIFIED | log last line; orchestrator reports exit 0 on HEAD; only .planning changed since |

### Prohibitions (judgment tier, non-authoritative verifier verdict)

| Prohibition | Verdict | Evidence |
|-------------|---------|----------|
| No allowlist / exception / cfg(test) exclusion in the scan | holds | rg finds only the //! sentence stating there is none |
| .planning not a forbidden component | holds | COMPONENTS = [".claude", ".codex"] |
| No dependency, Cargo.toml/Cargo.lock unchanged | holds | `git diff 27bd3ba HEAD -- Cargo.toml Cargo.lock` empty |
| Nothing under src/ changed | holds | `git diff 27bd3ba HEAD -- src` empty |
| Single-literal check kept | holds | planted_read_trips_the_scan PASS with unchanged assertions |
| No fixture under the checkout | holds | TMP_ROOT = CARGO_TARGET_TMPDIR; `git status --porcelain` clean after runs |
| bin/wsx, tests/sandbox.sh unchanged | holds | diff empty |

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| tests/harness/if02_runtime.rs | ⚠️ substantive, wired, sentinel design incomplete | 422 lines, `mod if02_runtime;` in tests/harness/main.rs, spawns CARGO_BIN_EXE_wsx; see gap |
| tests/harness/main.rs | ✓ VERIFIED | `mod if02_runtime;` |
| tests/if02_scan.rs | ✓ VERIFIED | decoded literals, concat!, join chains; header states what it leaves |

### Key Link Verification

| From | To | Status |
|------|----|--------|
| if02_runtime.rs | built wsx (check, check --json) | WIRED (WSX = env!("CARGO_BIN_EXE_wsx"); state A asserts exit 0 and the passing verdict) |
| harness main.rs | if02_runtime.rs | WIRED (tests run under the harness binary) |
| if02_scan.rs | real src/ | WIRED (real_tree_reads_no_agent_file PASS) |

### TDD order (05-10)

e2587a7 RED → ba3a038 GREEN; a97284d RED → a9a6ba7 GREEN; 06a37e2 RED → 1805a0b GREEN. Each RED is an ancestor of its GREEN (`git merge-base --is-ancestor`), each RED commit touches test files only, and the three RED records print RED_EVIDENCE_OK. RED commits were not re-run red by the verifier. The tdd.review-checkpoint flags on 05-05 and 05-08 are the logged commit-type false positive (prior report shows RED before GREEN for both).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 13 IF-02 tests, one name per gate | `scripts/check.sh test -E 'test(=NAME)' && ...` | exit 0, 13 PASS | ✓ PASS |
| no-agent-files run | `scripts/check.sh test -E 'test(=registry::every_command_runs_in_a_repo_without_agent_files)'` | 1 passed | ✓ PASS |
| gate rejects an unknown name (negative control) | `scripts/check.sh test -E 'test(=if02_runtime::no_such_test)'` | exit 4 | ✓ PASS |

### Probe Execution

None declared for this phase.

### Requirements Coverage

| Requirement | Plans | Status | Evidence |
|-------------|-------|--------|----------|
| DIV-01 | 01,02,04,09 | ✓ SATISFIED | SC1 (unchanged code) |
| DIV-02 | 03,04,09 | ✓ SATISFIED | SC2 |
| DIV-03 | 03,06,09 | ✓ SATISFIED | SC3 |
| DIV-04 | 03,07,09 | ✓ SATISFIED | SC1 staleness |
| IF-01 | 01,02,08,09 | ✓ SATISFIED | SC4 (warning WR-01) |
| IF-02 | 05,09,10 | ⚠️ PARTIAL | src/ reads none of the files today (scan clean); enforcement has the narrowed SC5 hole. REQUIREMENTS.md already marks IF-02 `[x]` and "Complete" (line 174, 288), ahead of this verdict |

No orphaned requirement: REQUIREMENTS.md maps exactly these six IDs to Phase 5.

### Anti-Patterns / Review findings

| Finding | Severity | Impact |
|---------|----------|--------|
| 05-REVIEW-GAP WR-02 lenient reader undetected by the runtime states | 🛑 Blocker (SC5) | the gap above |
| 05-REVIEW-GAP WR-01 decoder panics on a CRLF line continuation | ⚠️ Warning | fails closed (panic, not a pass) with a misleading message on valid CRLF source; not reproduced |
| 05-REVIEW-GAP IN-01 header wording on consts | ℹ️ Info | wording |
| 05-REVIEW WR-01..WR-04, IN-01..IN-05 | ⚠️/ℹ️ | carried from the prior report, out of 05-10 scope by operator decision |

No TBD/FIXME/XXX in tests/if02_scan.rs or tests/harness/if02_runtime.rs (rg, no match).

### Human Verification Required

None. The remaining question is a gap or an override, not a test a human must perform.

### Gaps Summary

05-10 closes the three spellings the prior gap named and adds a runtime comparison that works for reads that change output. One hole remains in SC5: a lenient read of `.planning/config.json` built from a non-literal `.planning`. Close it by giving state A real, non-default keys plus a third well-formed state and a lenient reader control, or accept it with the override above.

---

_Verified: 2026-11-19T02:21:00Z_
_Verifier: Claude (gsd-verifier)_
