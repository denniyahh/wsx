---
phase: 05-divergence-check-and-command-contract
verified: 2026-11-19T01:14:57Z
status: gaps_found
score: 4/5 roadmap success criteria verified (plan truths: all named tests pass)
covered_files: [".planning/phases/05-divergence-check-and-command-contract/05-01-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-01-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-02-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-02-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-03-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-03-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-04-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-04-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-05-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-05-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-06-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-06-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-07-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-07-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-08-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-08-SUMMARY.md",".planning/phases/05-divergence-check-and-command-contract/05-09-PLAN.md",".planning/phases/05-divergence-check-and-command-contract/05-09-SUMMARY.md","Cargo.lock","Cargo.toml","deny.toml","docs/exit-codes.md","docs/json/README.md","docs/json/check.v1.schema.json","src/check/hooks.rs","src/check/mod.rs","src/check/report.rs","src/check/staleness.rs","src/cli/commands.rs","src/cli/mod.rs","src/exit.rs","src/git/config.rs","src/git/mod.rs","src/git/objects.rs","src/git/reads.rs","src/git/refs.rs","src/git/repo.rs","src/git/spawn.rs","src/git/staleness.rs","src/git/testing.rs","src/git/trees.rs","src/json.rs","src/main.rs","src/model/mod.rs","tests/cli.rs","tests/harness/check.rs","tests/harness/contract.rs","tests/harness/main.rs","tests/harness/provenance.rs","tests/harness/registry.rs","tests/harness/rust.rs","tests/if02_scan.rs"]
covered_digest: "v4:sha256:680406f18fc5af07b80d21087196cb51ca5589d101a5e43a35432a2d22e2f262"
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "SC5: A source-scan test fails on any read of ROADMAP.md/STATE.md contents, .planning/config.json, devflow.toml or .claude/.codex settings (control: a planted read trips it)"
    status: partial
    reason: "tests/if02_scan.rs matches .planning/config.json only as one literal (NAMED, substring) and treats only .claude/.codex as components (COMPONENTS). The idiomatic Path::new(\".planning\").join(\"config.json\") yields two literals, neither matches, so a real read of the GSD config passes the gate. Escaped spellings (\"STATE\\x2emd\") and concat! also pass, because literal.to_string() keeps escapes. The planted control only uses the single-literal spelling. (Review WR-05, open.)"
    artifacts:
      - path: "tests/if02_scan.rs"
        issue: "COMPONENTS lacks .planning; literals are matched unescaped; no planted control for the split-literal spelling"
    missing:
      - "Add .planning to COMPONENTS (whole-component match)"
      - "Unescape each literal before matching"
      - "Planted control using .join(\".planning\").join(\"config.json\") that must give one finding"
---

# Phase 5: Divergence check and command contract Verification Report

**Phase Goal:** `wsx check` reports, offline, every way the workspace branch differs from the integration branch that the manifest doesn't declare, and it sets the versioned JSON and exit-code contract every later command must join
**Verified:** 2026-11-19T01:14:57Z
**Status:** gaps_found
**Re-verification:** No — initial verification

Evidence base: `scripts/check.sh test` run once by the verifier on HEAD e2a7568: 863 passed, 0 failed, 57 skipped. Every test named in the nine plans' must_haves appears as PASS in that run (checked by name, none missing). `scripts/check.sh fmt` and `clippy` exit 0.

## Goal Achievement

### Observable Truths (roadmap success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Undeclared added / modified / deleted path fails naming it; fully declared passes; oracle outcome recorded per case, Rust never passes what the oracle catches; unreachable remote runs on local ref with fetch age | ✓ VERIFIED | PASS: check::{self_test,quoted_personal_names,fork_layout,no_gsd_repo,undeclared_modified,undeclared_deleted,remote_unreachable}::{rust,oracle}. Rust-passes-oracle-fails cases are forced through Class::Fixed with a recorded oracle outcome (tests/harness/scenario.rs FixedWithoutRecordedOracle / FixedPassesOnOracle); the three Fixed cases (feature_worktree D-02, hook symlink, scripts/*.sh) are declared differences, not undeclared ones. check::run (src/check/mod.rs:116) calls model::resolve first, then tree_diff of integration tip vs workspace tip; no HEAD/index read in src/check. No remote subcommand in production src (all fetch/clone/remote literals sit inside #[cfg(test)] modules). |
| 2 | Stale declarations fail, each named: carried `only`, unchanged `modified`, built-in-covered | ✓ VERIFIED | PASS: check::stale_only, check::stale_modified, check::redundant_builtin (::rust and ::oracle), check::tests::declaration_under_a_builtin_is_redundant_and_nothing_else, problems_are_grouped_then_sorted |
| 3 | Hook committed 100644 fails though executable on disk; 120000 symlink to tracked executable passes; non-hook scripts/*.sh 100644 does not fail (recorded oracle difference) | ✓ VERIFIED | PASS: check::hook_mode_from_the_commit, check::hook_symlink_to_an_executable, check::scripts_sh_not_checked, check::hooks_path_not_in_the_tree (both sides), check::hooks::tests::* |
| 4 | `--json` matches a versioned documented schema; exit codes documented; contract test enumerates the command tree and fails on missing pin or drift (control: field change without bump fails) | ✓ VERIFIED | docs/json/check.v1.schema.json, docs/json/README.md, docs/exit-codes.md exist. PASS: contract::every_command_output_matches_its_schema, schema_rejects_a_renamed_field, schema_files_match_their_pins, a_schema_edit_without_a_version_bump_fails, every_command_has_a_pinned_schema, a_command_without_a_pinned_schema_is_named, exit::tests::exit_codes_doc_lists_every_class_and_kind. Warning: see WR-01 below (exit 101 outside the table in human mode). |
| 5 | Source-scan test fails on any read of the named agent/GSD files (planted control trips it); every command runs in a repo with none of them | ✗ PARTIAL | Scan and controls pass (real_tree_reads_no_agent_file, planted_read_trips_the_scan, comments_do_not_trip_the_scan, unlexable_file_fails_closed, component_match_is_exact; registry::every_command_runs_in_a_repo_without_agent_files). But by code reading of tests/if02_scan.rs (NAMED substring match, COMPONENTS = [.claude, .codex], literal text kept escaped), `.join(".planning").join("config.json")`, `concat!` and escaped spellings pass the gate. The second half (no-agent-files run) holds. Today's src/ reads none of the files (scan clean). |

**Score:** 4/5 truths verified (0 present, behavior-unverified)

### Plan must-haves (05-01..05-09)

All truths naming tests: every named test PASS in the verifier's run. Spot checks beyond tests:

- D-07 no wildcard / bare-binding arm in exit::classify / of_git: none found in src/exit.rs.
- 05-09 / STATE obligation, module-level dead_code expectations removed: no `#![...dead_code]` in src/git/mod.rs or src/model/mod.rs; each remaining expectation is item-level `cfg_attr(not(test), expect(dead_code, reason = "first caller in phase N"))`. STATE.md line 279 still lists this obligation as open although the code has closed it (stale state entry).
- 05-09 mutation pass: target/gsd/05-09-pass-head.txt = 69bc5d2, the last commit touching src/, Cargo.toml or Cargo.lock; mutants.out: 194 caught, 33 unviable, 0 timeout, 3 missed, and the 3 missed equal target/gsd/05-09-equivalent-mutants.txt line for line. Not re-run by the verifier. The allowlist lines carry no reason text in the file itself; the reasons are in 05-09-SUMMARY.md.
- Dependencies: serde_json, proc-macro2, jsonschema are under [dev-dependencies] only; deny.toml allows MIT-0 for borrow-or-share (operator decision 2026-11-18).
- No stdin read in src/.

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| src/cli/commands.rs (COMMANDS) | ✓ VERIFIED | used by parser and by tests/harness via #[path] |
| src/cli/mod.rs, src/exit.rs, src/json.rs | ✓ VERIFIED | wired from main.rs through cli::run |
| src/check/{mod,report,hooks,staleness}.rs | ✓ VERIFIED | wired from check::run |
| src/git/{trees,staleness}.rs, Repo::hooks_path | ✓ VERIFIED | called by check::run |
| docs/exit-codes.md, docs/json/check.v1.schema.json, docs/json/README.md | ✓ VERIFIED | pinned and doc-tested |
| tests/if02_scan.rs | ⚠️ PARTIAL | see gap (SC5) |
| tests/harness/{contract,registry}.rs | ✓ VERIFIED | enumerate COMMANDS |

### Key Link Verification

| From | To | Status |
|------|----|--------|
| main.rs | cli::run | WIRED |
| cli | COMMANDS / exit::classify / json | WIRED |
| check::run | model::resolve (first read) | WIRED (src/check/mod.rs:118) |
| check::run | tree_diff, lookup_paths, hooks_path/list_tree/read_blobs, staleness::read | WIRED (src/check/mod.rs:122-129) |
| contract.rs | schema file, provenance hash_blob | WIRED (tests pass) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full suite | scripts/check.sh test | 863 passed, 57 skipped | ✓ PASS |
| Lints | scripts/check.sh fmt && clippy | exit 0 | ✓ PASS |
| --json to a broken pipe | `{ sleep 0.3; wsx check --json; } \| true` | exit 6, "cannot write the JSON object to stdout" | ✓ PASS (documented class) |
| human output to a broken pipe | `{ sleep 0.3; wsx --version; } \| true` | panic, exit 101 | ⚠️ outside the exit-code table (WR-01) |

### Probe Execution

None declared for this phase.

### TDD order (05-05, 05-08)

05-05: 5dadbeb RED (stub scan) → c01704f GREEN; e358bc7 RED → 730079f GREEN. 05-08: 1625073 RED (schema is {} until GREEN) → 79a090f GREEN; 3922c17 RED (zero pin, stub checks) → 9f02fd6 GREEN. Order is RED before GREEN in both plans; the gate's "missing GREEN" is the commit-type false positive already logged. RED commits were not re-run red by the verifier.

### Requirements Coverage

| Requirement | Plans | Status | Evidence |
|-------------|-------|--------|----------|
| DIV-01 | 01,02,04,09 | ✓ SATISFIED | SC1 |
| DIV-02 | 03,04,09 | ✓ SATISFIED | SC2 |
| DIV-03 | 03,06,09 | ✓ SATISFIED | SC3 |
| DIV-04 | 03,07,09 | ✓ SATISFIED | SC1 staleness, remote_unreachable, contract staleness tests |
| IF-01 | 01,02,08,09 | ✓ SATISFIED | SC4 (warning WR-01) |
| IF-02 | 05,09 | ⚠️ PARTIAL | code reads none of the files today; enforcement scan has the SC5 hole |

No orphaned requirement: REQUIREMENTS.md maps exactly these six IDs to Phase 5.

### Prohibitions (judgment tier, non-authoritative verifier verdict)

All 05-01..05-09 prohibitions hold on the evidence above (no HEAD/index read in check, no wildcard arm, resolve first, no stdin, no lossy byte conversion asserted by json round-trip tests, dev-only deps, no allowlist in the IF-02 scan, planted file under CARGO_TARGET_TMPDIR, no remote subcommand in production code, item-level dead_code only). Not mechanically checked: "no test weakened to kill a mutant" and "paths only on stdin, never a pathspec"; human review recommended for those two.

### Anti-Patterns / Review findings (05-REVIEW.md, all open)

| Finding | Severity | Impact |
|---------|----------|--------|
| WR-05 IF-02 scan misses split / escaped literals | 🛑 Blocker (SC5) | the gate does not cover the idiomatic spelling |
| WR-01 println! to broken stdout panics, exit 101 | ⚠️ Warning | reproduced on --version; --json path returns 6, so the orchestrator contract holds |
| WR-02 one missing symlink blob (partial clone) fails check with exit 6 | ⚠️ Warning | not reproduced |
| WR-03 replace-ref guard comment overclaims for new children | ⚠️ Warning | not reproduced |
| WR-04 impossible-state arm in exit classification | ⚠️ Warning | necessity |
| IN-01..IN-05 | ℹ️ Info | docs wording, message quality, bare-repo hooksPath guess |

No TBD/FIXME/XXX debt markers found by the review; not re-scanned separately.

### Gaps Summary

One gap: success criterion 5's source scan does not fail on a read of `.planning/config.json` written as two path components (`.join(".planning").join("config.json")`), nor on escaped or concat! spellings of any named file. Fix is local to tests/if02_scan.rs: add `.planning` as a component, unescape literals, and add a planted split-literal control. If the operator judges single-literal coverage sufficient, accept with:

```yaml
overrides:
  - must_have: "A source-scan test fails on any read of ROADMAP.md/STATE.md contents, .planning/config.json, devflow.toml or .claude/.codex settings"
    reason: "<why single-literal coverage is enough>"
    accepted_by: "<name>"
    accepted_at: "<ISO timestamp>"
```

---

_Verified: 2026-11-19T01:14:57Z_
_Verifier: Claude (gsd-verifier)_
