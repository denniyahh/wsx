---
phase: 05-divergence-check-and-command-contract
verified: 2026-11-19T02:40:00Z
status: passed
score: 5/5 roadmap success criteria verified (05-11 plan truths 6/6 pass)
covered_files: [".planning/phases/05-divergence-check-and-command-contract/05-01-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-01-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-02-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-02-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-03-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-03-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-04-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-04-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-05-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-05-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-06-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-06-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-07-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-07-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-08-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-08-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-09-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-09-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-10-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-10-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-11-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-11-SUMMARY.md","Cargo.lock","Cargo.toml","deny.toml","docs/exit-codes.md","docs/json/README.md","docs/json/check.v1.schema.json","src/check/hooks.rs","src/check/mod.rs","src/check/report.rs","src/check/staleness.rs","src/cli/commands.rs","src/cli/mod.rs","src/exit.rs","src/git/config.rs","src/git/mod.rs","src/git/objects.rs","src/git/reads.rs","src/git/refs.rs","src/git/repo.rs","src/git/spawn.rs","src/git/staleness.rs","src/git/testing.rs","src/git/trees.rs","src/json.rs","src/main.rs","src/model/mod.rs","tests/cli.rs","tests/harness/check.rs","tests/harness/contract.rs","tests/harness/if02_runtime.rs","tests/harness/main.rs","tests/harness/provenance.rs","tests/harness/registry.rs","tests/harness/rust.rs","tests/if02_scan.rs"]
covered_digest: "v4:sha256:915845d95c689379903524a57bcfbddd067f953fc51977a23dfadb9b4effd65c"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 4/5
  previous_verified: 2026-11-19T02:21:00Z
  history:
    - round: "2026-11-19T01:14:57Z (after 05-09)"
      status: gaps_found
      gap: "SC5: tests/if02_scan.rs missed Path::new(\".planning\").join(\"config.json\"), escaped spellings and concat!"
      closed_by: "05-10 (decoded literals, concat!, join chains, two-state runtime comparison)"
    - round: "2026-11-19T02:21:00Z (after 05-10)"
      status: gaps_found
      gap: "SC5, narrowed: a lenient (default-on-failure) read of .planning/config.json through a non-literal .planning base escaped both the scan and the runtime states, because state A held no key a reader acts on (05-REVIEW-GAP.md WR-02)"
      closed_by: "05-11 (real, differing settings in states A and C, malformed B, two lenient-reader controls)"
  gaps_closed:
    - "State A holds real GSD/devflow/Claude/Codex keys at non-default values (gsd_config, claude_settings, devflow_config, codex_config, gsd_state, gsd_roadmap in if02_runtime.rs)"
    - "A third well-formed state C with different values; lenient_read_with_state_a_default_differs_in_state_c PASS (differs A vs C only)"
    - "Lenient reader control through a variable-built path: lenient_config_read_makes_the_states_differ PASS; its RED run against the old note-only sentinels failed on the non-empty assertion (05-11-red-evidence.json RED_EVIDENCE_OK)"
    - "//! header narrowed: the 'a parse that fails on it also differs' sentence is gone; Reach paragraph names the six reader variants"
    - "05-REVIEW-GAP WR-01 (CRLF line continuation panic): fixed, crlf_line_continuation_trips_the_scan PASS"
  gaps_remaining: []
  regressions: []
advisory:
  - finding: "The runtime comparison sees a lenient read only of a setting the fixture states hold (git.branching_strategy and the branch templates, base_branch, model, permissions, env, model_reasoning_effort, the STATE.md frontmatter keys). A lenient read of a key no state holds (e.g. a workflow.* key of .planning/config.json) gets its default in A, B and C, and its spelling planning_dir.join(\"config.json\") is clean in the scan."
    category: other
    reason: "Inherent to operator option 1 (runtime comparison, no new scan rule); 05-11 delivered exactly the four missing items the prior gap listed. Established by reading lenient_branching_strategy and the sentinel builders, not by a planted reader. Would be narrowed by adding keys to the states, or closed by a scan rule the operator ruled out."
    evidence_status: "none provided"
---

# Phase 5: Divergence check and command contract Verification Report

**Phase Goal:** `wsx check` reports, offline, every way the workspace branch differs from the integration branch that the manifest doesn't declare, and it sets the versioned JSON and exit-code contract every later command must join
**Verified:** 2026-11-19T02:40:00Z
**Status:** passed
**Re-verification:** Yes, second, after gap-closure plan 05-11

Evidence base for this round, run by the verifier on HEAD bb49437: 14 named tests chained with `&&`, each through `scripts/check.sh test -E 'test(=NAME)'` (`--no-tests=fail` always set): exit 0, 14 PASS lines, one per name. They are the seven `if02_runtime::*` tests (the main comparison, four raw-read controls, two lenient-reader controls), `crlf_line_continuation_trips_the_scan`, `escaped_spellings_trip_the_scan`, `malformed_escapes_fail_closed_at_the_lexer`, `real_tree_reads_no_agent_file`, `planted_join_read_trips_the_scan`, `path_data_does_not_trip_the_scan` and `registry::every_command_runs_in_a_repo_without_agent_files`. Negative control: the same gate with `if02_runtime::no_such_test` exits 4 ("no tests to run"). `scripts/check.sh all` was not re-run by the verifier; target/gsd/05-11-all.log ends with `==> check.sh: all OK` and the orchestrator reports exit 0 on HEAD. No planted reader in src/: the hole's shape is planted on the test side by the two lenient controls (path built from fields, `serde_json` parse, default on any failure), and their RED run is the negative control showing the old states missed it.

## Goal Achievement

### Observable Truths (roadmap success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Undeclared added/modified/deleted fails naming the path; declared passes; oracle recorded; offline with fetch age | ✓ VERIFIED (regression check) | `git diff ff03eaa HEAD -- src Cargo.toml Cargo.lock` is empty, as is the same diff over `bin/wsx tests/sandbox.sh docs deny.toml`; production code and contract docs unchanged since the prior verdict |
| 2 | Stale declarations fail, each named | ✓ VERIFIED (regression check) | as SC1 |
| 3 | Hook mode from the commit; symlink passes; scripts/*.sh not checked | ✓ VERIFIED (regression check) | as SC1 |
| 4 | --json versioned schema, documented exit codes, contract test with control | ✓ VERIFIED (regression check) | as SC1; 05-REVIEW WR-01 (exit 101 on broken stdout in human mode) still open, warning |
| 5 | Source-scan test fails on any read of the named files (planted control trips it); every command runs in a repo with none of them | ✓ VERIFIED | Under operator option 1 (scan for spellings, runtime comparison for reads that reach output). The prior hole, a lenient read of `.planning/config.json` through a variable-built path, now makes the runtime comparison fail: `lenient_config_read_makes_the_states_differ` (fallback "none"; texts =phase, =none, =milestone) and `lenient_read_with_state_a_default_differs_in_state_c` (fallback equal to A's value; differs A vs C, not A vs B) PASS, and the same reader made no difference against the old sentinels (RED record, RED_EVIDENCE_OK). `check`/`check --json` stay identical across A, B and C. Registry no-agent-files test PASS. Residual reach in `advisory` |

**Score:** 5/5 truths verified (0 present, behavior-unverified)

### 05-11 plan must-haves

| # | Truth (short) | Status | Evidence |
|---|---------------|--------|----------|
| 1 | check / check --json identical across A, B, C; A exits 0 passing; three JSON runs mask one tip each | ✓ VERIFIED | check_output_does_not_depend_on_agent_files PASS; asserts by_command[1].len() == 3 and masked == 1 each |
| 2 | Lenient reader (fallback none) differs, texts phase/none/milestone; RED showed no difference | ✓ VERIFIED | lenient_config_read_makes_the_states_differ PASS; 05-11-red-evidence.json RED_EVIDENCE_OK; RED ae69933 is an ancestor of GREEN ca95de2 and touches only tests/harness/if02_runtime.rs |
| 3 | Same reader with A's value as fallback differs A vs C only | ✓ VERIFIED | lenient_read_with_state_a_default_differs_in_state_c PASS |
| 4 | Four raw-read controls pass, sentinel texts unchanged | ✓ VERIFIED | four PASS; expected texts are still sentinel-A with sentinel-B or absent |
| 5 | CRLF continuation gives one STATE.md finding; LF case, malformed-escape control, real-tree scan pass | ✓ VERIFIED | crlf_line_continuation_trips_the_scan, escaped_spellings_trip_the_scan, malformed_escapes_fail_closed_at_the_lexer, real_tree_reads_no_agent_file PASS; 05-11-red-evidence-2.json RED_EVIDENCE_OK; RED e4884f5 is an ancestor of GREEN f67d3cf |
| 6 | scripts/check.sh all exits 0, last line "==> check.sh: all OK" | ✓ VERIFIED | log last line; orchestrator exit 0; not re-run by the verifier |

### Prohibitions (judgment tier, decided by the verifier from the evidence)

| Prohibition | Verdict | Evidence |
|-------------|---------|----------|
| No new or widened scan rule | holds | `git diff ff03eaa HEAD -- tests/if02_scan.rs`: one match arm `Some('\n' \| '\r')` in unescape (decoding) plus one test; COMPONENTS and matching code untouched |
| Nothing under src/ changed | holds | diff empty |
| No dependency, Cargo.toml/Cargo.lock unchanged | holds | diff empty; the reader uses the existing serde_json dev-dependency |
| No fixture under the checkout | holds | runtime fixtures via ScenarioDirs; `git status --porcelain` empty after the 14-test run |
| bin/wsx, tests/sandbox.sh unchanged | holds | diff empty |

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| tests/harness/if02_runtime.rs | ✓ VERIFIED | contains fn lenient_config_read_makes_the_states_differ; per-kind sentinel builders with real keys; State::C; wired via `mod if02_runtime;` in tests/harness/main.rs |
| tests/if02_scan.rs | ✓ VERIFIED | contains fn crlf_line_continuation_trips_the_scan; CR continuation arm |

### Key Link Verification

| From | To | Status |
|------|----|--------|
| if02_runtime.rs | built wsx (check, check --json) | WIRED (WSX = env!("CARGO_BIN_EXE_wsx"), spawned in each of the three states) |
| if02_runtime.rs | serde_json (existing dev-dependency) | WIRED (from_slice, pointer, json!, to_vec_pretty) |
| if02_scan.rs | real src/ | WIRED (real_tree_reads_no_agent_file PASS) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| 14 IF-02 / registry tests, one name per gate | `scripts/check.sh test -E 'test(=NAME)' && ...` | exit 0, 14 PASS | ✓ PASS |
| gate rejects an unknown name (negative control) | `scripts/check.sh test -E 'test(=if02_runtime::no_such_test)'` | exit 4 | ✓ PASS |
| RED records | `gsd-tools check tdd-red-evidence target/gsd/05-11-red-evidence{,-2}.json --raw --pick verdict` | RED_EVIDENCE_OK twice | ✓ PASS |

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
| IF-02 | 05,09,10,11 | ✓ SATISFIED | SC5. REQUIREMENTS.md marks it [x] / Complete (lines 174, 288); set by close-out ahead of this verdict, now consistent with it |

No orphaned requirement: REQUIREMENTS.md maps exactly these six IDs to Phase 5.

### Anti-Patterns / Review findings

| Finding | Severity | Impact |
|---------|----------|--------|
| 05-REVIEW-GAP WR-02 lenient reader undetected | closed | 05-11 Task 1 |
| 05-REVIEW-GAP WR-01 decoder panic on CRLF continuation | closed | 05-11 Task 2, RED confirmed the premise |
| 05-REVIEW-GAP IN-01 header wording on consts | ℹ️ Info | backlog per 05-11 SUMMARY |
| 05-REVIEW WR-01..WR-04, IN-01..IN-05 | ⚠️/ℹ️ | carried, out of gap scope by operator decision |
| Lenient read of a key no fixture state holds | 📋 Advisory | see frontmatter `advisory` |

No TBD/FIXME/XXX in tests/if02_scan.rs or tests/harness/if02_runtime.rs (rg, no match).

### Human Verification Required

None. The five judgment-tier prohibitions are settled by the diffs and status checks above (decided by the verifier on that evidence).

### Gaps Summary

None remaining. 05-10 closed the spellings; 05-11 closed the lenient-reader hole by giving the runtime states real, differing settings and a malformed state, with two lenient-reader controls whose RED run shows the old states missed the shape.

---

_Verified: 2026-11-19T02:40:00Z_
_Verifier: Claude (gsd-verifier)_
