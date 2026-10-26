---
phase: "2"
slug: "oracle-harness"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-26"
---

# Phase 2 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source: `02-RESEARCH.md` § Validation Architecture. Test names are proposals until the plans fix them.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | libtest `#[test]` run by cargo-nextest 0.9.148 |
| **Config file** | `.config/nextest.toml` (exists; gains the per-test override that names disabled Rust sides) |
| **Quick run command** | `scripts/check.sh test -E 'test(=<module>::<name>)'`, one test per command, chained with `&&` |
| **Full suite command** | `scripts/check.sh all` |
| **Estimated runtime** | ~27 seconds for the nextest run (measured 2026-10-26: 153 passed, 41 skipped); `check.sh all` adds fmt and clippy |

---

## Sampling Rate

- **After every task commit:** that task's named tests, one `scripts/check.sh test -E 'test(=...)'` per command.
- **After every plan wave:** `scripts/check.sh all`.
- **Before `/gsd-verify-work`:** `scripts/check.sh all`, `scripts/check.sh deps` and
  `scripts/check-in-container.sh test`, all green.
- **Max feedback latency:** 120 seconds.

---

## Per-Task Verification Map

Each row's negative control must produce the opposite result. If both cases agree, the
measurement is broken, not the subject.

| Requirement / criterion | Behavior | Test Type | Automated Command (proposed) | Negative control | File Exists | Status |
|---|---|---|---|---|---|---|
| HARN-01 / SC1 | Vendored blobs equal the recorded devflow blobs | integration | `test(=provenance::vendored_files_match_devflow_blobs)` | Scratch copy with one flipped byte is reported | ✅ | ✅ green |
| HARN-01 / SC1 | `oracle.patch` applies cleanly and yields the recorded patched blobs | integration | `test(=provenance::oracle_patch_applies_cleanly)` | Apply without `GIT_CEILING_DIRECTORIES` in a nested repo leaves the files unpatched | ✅ | ✅ green |
| HARN-01 / SC1 | Self-test per script (5) | scenario | `test(=<module>::self_test::oracle)` ×5 | phase-worktree self-test expects the sync merge (catches the silent skip) | ✅ | ✅ green |
| HARN-01 / SC1 | One definition runs on oracle or Rust | scenario | `<name>::oracle` runs; `<name>::rust` is skipped | Selecting only `::rust` gives "no tests to run" | ✅ | ✅ green |
| HARN-02 / SC2 | Planted host config and hook never reach scenarios | integration | `test(=hermeticity::planted_host_config_never_reaches_scenarios)` | `hermeticity::wired_to_planted_config_fires_hook` sees the marker | ✅ | ✅ green |
| HARN-02 / SC2 | Oracle pre-push `unset GIT_CONFIG_*` leaves config sources unchanged (`GIT_CONFIG_NOSYSTEM` survives; global stays `$HOME/.gitconfig`) | integration | `test(=hermeticity::pre_push_unset_leaves_config_sources_unchanged)` | `test(=hermeticity::pre_push_unset_switches_source_when_global_is_elsewhere)`: with `GIT_CONFIG_GLOBAL` pointed away from `$HOME/.gitconfig`, the unset does switch source (a planted `/etc/gitconfig` is not plantable, so this replaces it) | ✅ | ✅ green |
| HARN-02 / SC2 | cargo-llvm-cov pinned; `install` and `verify` ask the binary through its `llvm-cov` subcommand | shell control, recorded in SUMMARY | `scripts/install-tools.sh verify cargo-llvm-cov` | A stub answering only `llvm-cov --version` with 0.9.0 is refused naming `expected 0.9.1`; a missing binary is refused; a `cargo-deny` stub accepting only a bare `--version` still passes | ✅ | ✅ green |
| HARN-02 / SC2 | Coverage sees code only the spawned binary runs | one-time, recorded in SUMMARY | `CARGO_LLVM_COV_SETUP=no cargo llvm-cov nextest --no-tests=fail -E 'test(=coverage::spawned_wsx_reports_version)' --text` | (A) the same line is 0 when nothing spawns `wsx`; (B) without the `LLVM_*` passthrough it is 0 and a stray profraw appears | ✅ | ✅ green |
| HARN-03 / SC3 | Every expected field is compared | integration | `test(=outcome::changing_any_expected_field_fails)` | The unmutated outcome gives zero mismatches | ✅ | ✅ green |
| HARN-03 / SC3 | Symbolic refs are observed by target name, so a fetch does not make `origin/HEAD` look changed | integration | `test(=outcome::symbolic_refs_are_recorded_by_target)` | In the same run, `Tip::Unchanged` on the fetched `origin/develop` is reported as a mismatch | ✅ | ✅ green |
| HARN-03 | Scenario scratch lives under `CARGO_TARGET_TMPDIR`, named `harness-<module>-<name>-`, and is removed on drop | integration | `test(=env::scenario_dirs_carry_the_harness_prefix)` | The phase-gate audit must find a planted `harness-audit-control-*` directory before it may report none left | ✅ | ✅ green |
| HARN-03 / SC3 | Signature presence asserted separately; signing is declared in the case's repo config (`RepoConfig::with_signing()`) | scenario | `test(=signing::cut_replay_signed::oracle)`, `test(=signing::sync_merge_signed::oracle)` | Unsigned control expects `signed: false` | ✅ | ✅ green |
| HARN-03 / SC3 | Runner rejects a scenario whose only expectation is "same as oracle" | integration | `test(=scenario::rejects_scenario_without_explicit_expectation)` | A well-formed scenario validates | ✅ | ✅ green |
| HARN-03 / SC4 | A `fixed:` scenario fails if it passes on the oracle: `validate` rejects a record literally equal to the expectation, and at run time the oracle side fails whenever the run meets the expected outcome, whatever the record's encoding | integration | `test(=scenario::rejects_fixed_matching_expected_on_oracle)`, `test(=scenario::fixed_record_reordering_expected_refs_fails_on_oracle)`, `test(=scenario::fixed_record_using_unchanged_for_listed_refs_fails_on_oracle)` | `test(=scenario::fixed_case_differing_on_oracle_passes)`; `test(=scenario::fixed_record_drift_fails_on_oracle)` must report the record's exit mismatch (a `judge` that skips the record returns Ok): a record that really differs from the expectation, on an oracle run that does not meet it, passes as `fixed:` | ✅ | ✅ green |
| HARN-05 / SC4 | `regression:` must be a refusal; guard needs known-positive and clean controls | integration | `rejects_regression_that_is_not_a_refusal`, `rejects_guard_without_known_positive`, `rejects_guard_without_clean_control` | Valid counterparts accepted | ✅ | ✅ green |
| HARN-06 / SC4 | A scenario tagged with any HARN-06 case kind needs a control | integration | `test(=scenario::rejects_harn06_case_without_control)`, `test(=scenario::accepts_every_harn06_kind_with_a_counterpart)` | Each of the seven kinds is rejected without its counterpart control | ✅ | ✅ green |
| HARN-06 / SC5 | Each case has a scenario and a control, oracle-recorded where the oracle implements it, else Rust-only | scenario | one `<module>::<case>::oracle` per case | Inline controls; an oracle record (`fixed:`) only where the observed oracle outcome differs from the expectation, otherwise Parity | ✅ | ✅ green |
| HARN-06 / SC5 | Every case kind, and every action the oracle runs for it, has an oracle-recorded scenario | integration | `test(=registry::every_harn06_case_has_a_scenario)` | `registry::harn06_gaps_names_a_missing_case_kind` and `registry::harn06_gaps_names_a_missing_oracle_action` name exactly the planted gap | ✅ | ✅ green |
| SC5 | Disabled Rust sides reported by name | integration | `test(=registry::rust_sides_not_enabled)` (output shown through the nextest override) | `registry::every_scenario_is_well_formed` | ✅ | ✅ green |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `tests/harness/` integration-test binary (exists: `wsx::harness`)
- [x] `tests/oracle/`: vendored scripts, LICENSE, README with the devflow SHA, `oracle.patch`
- [x] `tempfile` dev-dependency, with a written justification and a clean `scripts/check.sh deps`
- [x] `.config/nextest.toml` override for the disabled-sides report test
- [x] `scripts/install-tools.sh`: `cargo-llvm-cov` pin
- [x] `rust-toolchain.toml`: `llvm-tools` component (02-CONTEXT, Coverage tooling)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Coverage of the spawned binary | HARN-02 / SC2 | `cargo llvm-cov` is not part of the gate (no CI coverage job, per 02-CONTEXT) | Run the command in the map above. Record covered and uncovered counts for the `--version` line under both negative controls in the plan SUMMARY |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-10-26

## Validation Audit 2026-10-26

| Metric | Count |
|---|---|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |
