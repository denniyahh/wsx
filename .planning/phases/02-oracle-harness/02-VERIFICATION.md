---
phase: 02-oracle-harness
verified: 2026-10-26T18:05:00Z
status: passed
score: 5/5 must-haves verified
covered_files:
  - ".config/nextest.toml"
  - ".planning/phases/02-oracle-harness/02-01-PLAN.md"
  - ".planning/phases/02-oracle-harness/02-01-SUMMARY.md"
  - ".planning/phases/02-oracle-harness/02-02-PLAN.md"
  - ".planning/phases/02-oracle-harness/02-02-SUMMARY.md"
  - ".planning/phases/02-oracle-harness/02-03-PLAN.md"
  - ".planning/phases/02-oracle-harness/02-03-SUMMARY.md"
  - ".planning/phases/02-oracle-harness/02-04-PLAN.md"
  - ".planning/phases/02-oracle-harness/02-04-SUMMARY.md"
  - ".planning/phases/02-oracle-harness/02-05-PLAN.md"
  - ".planning/phases/02-oracle-harness/02-05-SUMMARY.md"
  - ".planning/phases/02-oracle-harness/02-06-PLAN.md"
  - ".planning/phases/02-oracle-harness/02-06-SUMMARY.md"
  - ".planning/phases/02-oracle-harness/02-07-PLAN.md"
  - ".planning/phases/02-oracle-harness/02-07-SUMMARY.md"
  - ".planning/phases/02-oracle-harness/02-08-PLAN.md"
  - ".planning/phases/02-oracle-harness/02-08-SUMMARY.md"
  - ".planning/phases/02-oracle-harness/02-09-PLAN.md"
  - ".planning/phases/02-oracle-harness/02-09-SUMMARY.md"
  - ".planning/phases/02-oracle-harness/02-10-PLAN.md"
  - ".planning/phases/02-oracle-harness/02-10-SUMMARY.md"
  - ".planning/phases/02-oracle-harness/02-11-PLAN.md"
  - ".planning/phases/02-oracle-harness/02-11-SUMMARY.md"
  - ".planning/phases/02-oracle-harness/02-12-PLAN.md"
  - ".planning/phases/02-oracle-harness/02-12-SUMMARY.md"
  - ".planning/phases/02-oracle-harness/02-13-PLAN.md"
  - ".planning/phases/02-oracle-harness/02-13-SUMMARY.md"
  - ".planning/phases/02-oracle-harness/02-14-PLAN.md"
  - ".planning/phases/02-oracle-harness/02-14-SUMMARY.md"
  - ".planning/phases/02-oracle-harness/02-15-PLAN.md"
  - ".planning/phases/02-oracle-harness/02-15-SUMMARY.md"
  - "CLAUDE.md"
  - "CONTRIBUTING.md"
  - "Cargo.lock"
  - "Cargo.toml"
  - "deny.toml"
  - "rust-toolchain.toml"
  - "scripts/check.sh"
  - "scripts/install-tools.sh"
  - "src/main.rs"
  - "tests/gate_controls.rs"
  - "tests/harness/action.rs"
  - "tests/harness/check.rs"
  - "tests/harness/coverage.rs"
  - "tests/harness/cut.rs"
  - "tests/harness/env.rs"
  - "tests/harness/fixture.rs"
  - "tests/harness/guard.rs"
  - "tests/harness/hermeticity.rs"
  - "tests/harness/main.rs"
  - "tests/harness/oracle.rs"
  - "tests/harness/outcome.rs"
  - "tests/harness/provenance.rs"
  - "tests/harness/registry.rs"
  - "tests/harness/rust.rs"
  - "tests/harness/scenario.rs"
  - "tests/harness/signing.rs"
  - "tests/harness/sync.rs"
  - "tests/harness/worktree.rs"
  - "tests/oracle/LICENSE"
  - "tests/oracle/README.md"
  - "tests/oracle/check-workspace-divergence.sh"
  - "tests/oracle/cut-pr-branch.sh"
  - "tests/oracle/hooks/pre-push"
  - "tests/oracle/oracle.patch"
  - "tests/oracle/phase-worktree.sh"
  - "tests/oracle/sync-workspace.sh"
covered_digest: "v3:sha256:ee9b639a9fd4354af813b4d83e0e6191b6eb70c57abb26875763928a6230a4b7"
behavior_unverified: 0
overrides_applied: 0
prohibitions_resolved:
  - statement: "MUST NOT change a vendored oracle file, or the patch beyond path adaptation, to make a scenario pass (02-04)"
    verdict: held
    decided_by: "gsd-verifier, on evidence (non-authoritative judgment tier; owner may overrule)"
    basis: "All six vendored files hash to the blob and mode devflow ee6ec792 holds (independent git rev-parse against the local devflow clone). The patch changes three path references and replaces pre-push's last 18 lines (the CI-mirror tail that runs scripts/check-in-container.sh); that tail runs after the personal-path policy and divergence check and holds no policy logic. Documented in tests/oracle/README.md (patch table, Rules and re-vendoring step 2 all name the tail replacement since 811227a)."
  - statement: "MUST NOT let a scenario pass on agreement with the oracle alone (02-05)"
    verdict: held
    decided_by: "gsd-verifier, on evidence"
    basis: "validate rejects a case with no expected outcome; judge compares the oracle run with the explicit expected or recorded outcome and the Rust run with the expected outcome only; no code path compares a Rust run with a live oracle run."
  - statement: "MUST NOT report a scenario whose Rust side is not enabled as a pass (02-05)"
    verdict: held
    decided_by: "gsd-verifier, on evidence"
    basis: "All 41 Rust sides are #[ignore] and show as skipped (186 passed, 41 skipped); registry::rust_sides_not_enabled lists all 41 by name in the full run."
  - statement: "MUST NOT let any test process read the operator's real global/system git config, hooks or SSH agent, contact a network remote, or modify a repository outside its TempDir (02-09)"
    verdict: held
    decided_by: "gsd-verifier, on evidence"
    basis: "env_clear plus harness HOME/XDG/GIT_CONFIG_*; SSH_AUTH_SOCK dropped; GIT_ALLOW_PROTOCOL=file; ceiling at scenario root's parent. The planted-host proof and its wired control both pass. The wsx worktree's git status was unchanged after two host runs, two coverage runs, one container run and one failing Rust-side run, and no harness-* scratch tree was left under target/tmp."
---

# Phase 2: Oracle harness Verification Report

**Phase Goal:** Any wsx behaviour can be pinned as a hermetic scenario that runs against the vendored shell oracle or the Rust binary, with explicit expected outcomes that outlive the oracle
**Verified:** 2026-10-26T18:05:00Z
**Status:** passed
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `tests/oracle/` holds the four scripts and the pre-push hook from a recorded devflow commit, MIT-attributed, adapted by one documented patch. Each script's self-test passes against a fixture, and one scenario definition runs against the oracle, the Rust binary or both, from `cargo nextest` | ✓ VERIFIED | I checked independently against the local devflow clone: all 6 files match commit `ee6ec792` (on devflow `develop`) by blob and mode. `oracle.patch` applies cleanly to a scratch copy, the post-patch blobs match the README, and a second apply fails (control). Five `self_test::oracle` tests pass (sync, worktree, check, cut, guard/pre-push). The `scenarios!` macro selects the side per scenario (`oracle`, `rust_only`, `rust_enabled`). Negative control: I ran the ignored `cut::self_test::rust`. It spawned `wsx` (stderr `usage: wsx --version`), was judged against the expected outcome, and failed with 3 named differences. So the Rust side is reachable and judged. |
| 2 | Hermetic: a planted global hook and config change nothing and the hook never fires, while a control wired to the plant sees it fire. Signing is pinned locally. `cargo llvm-cov` covers code that only the spawned binary runs | ✓ VERIFIED | `hermeticity::planted_host_config_never_reaches_scenarios` passes: 3 real scenarios plus controls, no marker, decoy unchanged. Its control `wired_to_planted_config_fires_hook` passes: `hookspath:pre-commit` and `gpg:sign` fire. The fixture sets `commit.gpgsign=false` locally, and the signing scenarios use a per-run SSH key (`gpg.format=ssh`, `user.signingkey`). Coverage, run by me: under `cargo llvm-cov nextest` with `CARGO_LLVM_COV_SETUP=no`, `src/main.rs:17` (the `--version` println, reachable only in the spawned binary) counts **1** for `spawned_wsx_reports_version` and **0** for the no-`LLVM_*` control. |
| 3 | Every scenario states exit code, ref names, commits (message + author + author date + committer name/email + tree), signature presence and files. The oracle outcome is recorded in the scenario. Changing any one field fails, and "same as the oracle" alone is rejected | ✓ VERIFIED | The `Outcome`/`CommitExpect`/`FilesExpect` types carry every field (plus parents, ref set, config, HEAD/index/pending state). `outcome::changing_any_expected_field_fails` applies 24 single-field mutations to a real oracle run, each reported as its own field. Its unmutated control is clean. `validate` rejects `expected: None` (`rejects_scenario_without_explicit_expectation`). `fixed:` cases carry `OracleSide::Recorded(..)` in code. The committer date is omitted by owner decision; the ROADMAP criterion does not list it either. |
| 4 | Runner enforces the difference classes and guard controls | ✓ VERIFIED | `judge` returns `FixedPassesOnOracle` when a `fixed:` oracle run meets its expected outcome; real oracle runs prove it (`fixed_record_reordering_expected_refs_fails_on_oracle`, `fixed_record_using_unchanged_for_listed_refs_fails_on_oracle`), with a passing counterpart (`fixed_case_differing_on_oracle_passes`). On Rust, a `fixed:` case is judged against its expected outcome: `refs_exactly_fails_a_cut_that_writes_an_upstream` uses `judge(Side::Rust, …)`, and my manual `cut::self_test::rust` run failed. `RegressionNotARefusal`, `GuardWithoutKnownPositive` and `GuardWithoutCleanControl` each have a passing rejection test, plus direction checks (`KnownPositiveNotRefused`, `CleanControlRefused`). `every_scenario_is_well_formed` validates all registered scenarios. |
| 5 | Each HARN-06 case has a scenario with a control, oracle-recorded where the oracle implements it and Rust-only where it does not. Disabled Rust sides are reported by name | ✓ VERIFIED | `every_harn06_case_has_a_scenario` passes and prints the map: merges 5, squash 3, fork 5, scoped+unscoped 1, no-GSD 8, path names 5 (non-ASCII `\xC3\xA9`, newline, non-UTF-8 `\xFF`), existing PR branch 1. Rust-only cases are `worktree::rm_after_squash_merge` and the 3 `no_gsd_custom_personal_path` scenarios. `harn06_gaps` controls name a missing kind and a missing action. `validate` requires a control on every HARN-06 scenario. `rust_sides_not_enabled` lists all 41 by `module::name (reason)` in the full run. |

**Score:** 5/5 truths verified (0 present, behavior-unverified)

### Required Artifacts

`verify.artifacts` reports 15/15 plans passing (37/37 artifacts). `verify.key-links` reports 29/30. The one miss, 02-02 `outcome.rs -> git for-each-ref` (pattern `%(symref)`), is a tool false negative: `tests/harness/outcome.rs` `read_refs` runs `for-each-ref --format=%(refname)%00%(symref)%00%(objectname)`, and `outcome::symbolic_refs_are_recorded_by_target` passes.

| Artifact | Status | Details |
|---|---|---|
| `tests/oracle/*` (5 scripts, LICENSE, README, oracle.patch) | ✓ VERIFIED | Byte-identical to devflow; patch documented |
| `tests/harness/scenario.rs` (rules, judge, runner, macro) | ✓ VERIFIED | Wired by every scenario module via `scenarios!` |
| `tests/harness/outcome.rs` (observe/compare) | ✓ VERIFIED | Used by `judge`; mutation-tested |
| `tests/harness/env.rs`, `fixture.rs`, `oracle.rs`, `rust.rs`, `action.rs` | ✓ VERIFIED | Used by `run_case` |
| `tests/harness/{sync,worktree,check,cut,guard,signing}.rs` | ✓ VERIFIED | 41 scenarios registered through `harness_modules!` |
| `tests/harness/{registry,provenance,hermeticity,coverage}.rs` | ✓ VERIFIED | Rule, provenance, hermeticity and coverage proofs, all passing |
| `rust-toolchain.toml` llvm-tools, `scripts/install-tools.sh` cargo-llvm-cov 0.9.1 | ✓ VERIFIED | `cargo llvm-cov --version` = 0.9.1; coverage runs with SETUP=no |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full gate | `scripts/check.sh all` | fmt, clippy, test OK; 186 passed, 41 skipped; exit 0 | ✓ PASS |
| Container gate | `scripts/check-in-container.sh test` (devcontainers/rust:2.1.3-1-trixie) | 186 passed, 41 skipped; exit 0 (at HEAD ad5d5a5; 119fe46..HEAD touches only `.planning/`) | ✓ PASS |
| Supply chain | `scripts/check.sh deps` | advisories, bans, licenses, sources ok | ✓ PASS |
| Vendored provenance (independent) | `git rev-parse ee6ec792:<path>` vs `git hash-object --no-filters` | 6/6 blob+mode match | ✓ PASS |
| Patch applies (independent) | `git apply oracle.patch` on scratch copy, with ceiling | 3 files patched, blobs match README; re-apply refused | ✓ PASS |
| Spawned-binary coverage | `cargo llvm-cov nextest --text` subject vs control | `main.rs:17` count 1 vs 0 | ✓ PASS |
| Rust side reachable and judged | `cargo nextest run --run-ignored only -E 'test(=cut::self_test::rust)'` | spawns wsx, fails with 3 named differences (expected: wsx has no `cut` yet) | ✓ PASS |
| Filter that matches nothing fails | same, misspelled test name | exit 4 | ✓ PASS |

### Probe Execution

Step 7c: SKIPPED. The phase declares no `probe-*.sh`.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|---|---|---|---|
| HARN-01 | 02-01, 02-02, 02-03, 02-04, 02-07, 02-15 | ✓ SATISFIED | Truth 1 |
| HARN-02 | 02-02, 02-06, 02-08, 02-09 | ✓ SATISFIED | Truth 2. `env_passes_through_only_llvm_keys` passes, and `LLVM_*` passthrough is proven by coverage counts |
| HARN-03 | 02-02, 02-05, 02-08, + case plans | ✓ SATISFIED | Truths 3, 4. Committer date omitted by owner decision (IN-04) |
| HARN-05 | 02-05, 02-10, 02-13, + | ✓ SATISFIED | Truth 4. Guard scenarios validated for both control roles and directions |
| HARN-06 | 02-10 … 02-15 | ✓ SATISFIED | Truth 5 |

No orphans: REQUIREMENTS.md maps exactly HARN-01, -02, -03, -05 and -06 to Phase 2, and every plan requirement ID is in that set. HARN-04 is Phase 1.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| (phase files) | – | TBD/FIXME/XXX | none | `rg -q` exit 1 over harness, oracle README/patch, check.sh and install-tools.sh. A positive control matched |
| `tests/harness/scenario.rs` | – | No `regression:` scenario is registered; the rule is exercised by synthetic tests only | ℹ️ Info | Expected for phase 2; the rule is enforced |
| `tests/harness/rust.rs` | header | Rust argv is "best guess" until each enabling phase | ℹ️ Info | By design; every Rust side is ignored |

The README wording item raised in the first pass (the Rules section said "adapts paths only") was fixed in 811227a; `provenance::readme_records_the_pinned_commit_and_blobs` still passes.

Deferred review info findings (IN-01..IN-06, IN-08..IN-10) stand as recorded in 02-REVIEW-DISPOSITION.md. None affects a success criterion.

### Owner decisions checked against code

- **The commit expectation has no committer date (HARN-03):** `CommitExpect` has no committer-date field, and the ROADMAP criterion 3 list matches.
- **AR-02-02, exit 128 in the fork-layout counterparts:** the 128 is used in `sync.rs:403` and `cut.rs:1179`. Each carries an "oracle-observed, revised under IF-01" doc comment (`sync.rs:368-373`, `cut.rs:1190`), and a pending todo exists. Both Rust sides are ignored.
- **PR branches are cut with no upstream (PRCUT-06):** cut expectations hold an unchanged config. The oracle's upstream write is recorded as `fixed:` via `upstream_fixed`. `refs_exactly_fails_a_cut_that_writes_an_upstream` proves the config alone fails a Rust run.

### Human Verification Required

None. Each judgment-tier prohibition (in plans 02-04, 02-05 and 02-09) is a sign-off the tool requires by policy, not a found defect. Evidence settled all four, and they are recorded under `prohibitions_resolved` with their basis.

### Limits of this verification

- A Rust side runs only through the ignored tests. No enabled Rust side exists, so success criterion 4's "fails on Rust once enabled" is shown by `judge(Side::Rust, …)` on a real oracle run and by one manual ignored run. No real wsx command has been exercised.
- The mutation test proves the shared comparator reports each field on one real outcome (`sync::self_test`). It does not prove every scenario's expected values are right; the passing oracle sides cover that, only for the oracle.
- The coverage proof is a manual run (no CI job, by decision IN-06).
- In the hermeticity proof, the "host" is planted via environment variables pointing at scratch files. The operator's real `~/.gitconfig` is never touched, by design.

### Gaps Summary

None. All five success criteria are met in code and in fresh runs (host and container), each with a negative control.

---

_Verified: 2026-10-26T18:05:00Z_
_Verifier: Claude (gsd-verifier)_
