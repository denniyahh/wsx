---
phase: "1"
slug: "development-foundation"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-22"
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo-nextest 0.9.148 (Rust); negative-control recipes C1-C7 in 01-RESEARCH.md (shell, scratch space); `shellcheck` + `bash -n` for scripts |
| **Config file** | `.config/nextest.toml` (JUnit report only, for TDD red-evidence); `check.sh` carries the gate flags. Wave 0 creates the crate and `check.sh` |
| **Quick run command** | `scripts/check.sh test` |
| **Full suite command** | `scripts/check.sh all && scripts/check-in-container.sh all && scripts/assert-image-parity.sh && shellcheck scripts/*.sh` |
| **Estimated runtime** | ~60 seconds on the host for the placeholder crate; container run adds a one-time toolchain install (~17 s, network) |

---

## Sampling Rate

- **After every task commit:** Run `scripts/check.sh all` (once it exists)
- **After every plan wave:** Run the full suite command
- **Before `/gsd-verify-work`:** Full suite green, and every control C1-C7 run with its failing output quoted
- **Max feedback latency:** 120 seconds

---

## Per-Task Verification Map

Filled in by the planner per task. Requirement-level map (from 01-RESEARCH.md § Validation Architecture):

| Requirement | Behavior | Test Type | Automated Command | File Exists | Status |
|-------------|----------|-----------|-------------------|-------------|--------|
| DEV-01 | each target runs; a missing tool is named and fails | integration | `tests/gate_controls.rs` `missing_*_fails_*` (6 tests); CI runs each target | ✅ | ✅ green |
| DEV-02 | pinned rustc 1.99.1 used | integration | `tests/repo_policy.rs` `rustc_at_repo_root_is_the_pinned_channel`, `rust_toolchain_toml_is_what_selects_the_compiler` | ✅ | ✅ green |
| DEV-03/04 | docs carry traps, acceptance bar, four rejected shapes | manual review | see Manual-Only | n/a | manual |
| DEV-05 | signing + git-flow names via include | integration | `tests/repo_policy.rs` `tracked_gitconfig_sets_exactly_the_shared_policy` + checker fixture; full signed-commit control C6 is evidence (01-05-SUMMARY) | ✅ | ✅ green |
| DEV-06 | container parity from a linked worktree; tag drift fails | integration + manual | `tests/image_parity.rs` (fixtures a-e + real repo); linked-worktree container run is manual (docker) | ✅ | ✅ green |
| SUP-01 | deny job exists, advisory | CI | `.github/workflows/deny.yml` (push, PR, weekly) runs `scripts/check.sh deps` | ✅ | ✅ green |
| SUP-02 | dependabot config (actions + docker), weekly, grouped, develop | manual review | see Manual-Only | n/a | manual |
| SUP-03/05 | hook refuses a secret; deny fails a vulnerable crate | evidence | see Manual-Only (C4, C5 need gitleaks install / network) | n/a | manual |
| SUP-04 | lock committed, SHAs pinned, versions in one script | integration + gate | `tests/repo_policy.rs` `every_workflow_action_is_pinned_to_a_full_commit_sha` + checker fixture; `--locked` and `install-tools.sh verify` in `check.sh` | ✅ | ✅ green |
| HARN-04 | no-match filter fails; set_var banned | integration | `tests/gate_controls.rs` `no_match_filter_fails_even_when_env_allows_no_tests`, `repo_clippy_toml_bans_env_mutation` (+ counterpart), fixture `tests/fixtures/env-ban/`; `tests/gate.rs` option refusals | ✅ | ✅ green |
| REL-04 | required checks on both rulesets | owner-gated | done 2026-10-23: PR #2 checks green, rulesets 25445011/25445012 read back (01-07-SUMMARY) | n/a | ✅ done |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `Cargo.toml`, `src/main.rs`, `tests/cli.rs`, `Cargo.lock`, `rust-toolchain.toml` — nothing exists yet
- [x] `scripts/check.sh`, `scripts/install-tools.sh` — the gate itself
- [x] No framework install on the host (all tools present, verified 2026-10-22)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| CI runs Test/Clippy/Format in the pinned image on a PR to `develop` | REL-04, DEV-06 | Opening a PR is outward-facing; owner-gated | Owner approves the PR cut and push; check `gh pr checks` on the PR's head SHA |
| Rulesets list Test, Clippy, Format as required | REL-04 | Changes GitHub settings; owner-gated | Owner runs the prepared command; re-read `gh api repos/denniyahh/wsx/rulesets/<id>` |
| Docs content | DEV-03, DEV-04 | Prose quality | Read CLAUDE.md and CONTRIBUTING.md against the requirement text |
| Dependabot config | SUP-02 | Takes effect only on `main`; a shape test would need a YAML dev-dependency | Read `.github/dependabot.yml`: github-actions + docker, weekly, grouped, `target-branch: develop` |
| Hook refuses a planted secret (C4) | SUP-03, SUP-05 | Needs gitleaks, which the CI image lacks (install is a network step) | Recipe C4 (01-RESEARCH.md); evidence in 01-05-SUMMARY. Owner removed the repo-local hook from this clone 2026-10-22; the global hook's refusal was re-proven that day |
| cargo deny fails a vulnerable crate (C5) | SUP-05 | Needs the advisory DB and crates.io (network) | Recipe C5; evidence in 01-02-SUMMARY |
| Container run from a linked worktree | DEV-06 | Needs a Docker daemon | `scripts/check-in-container.sh all` from `.worktrees/phase-N`; last green 2026-10-23 (28/28) |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 120s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-10-23 (validate-phase; orchestrator decided the manual-only set from the evidence)

## Validation Audit 2026-10-23

| Metric | Count |
|---|---|
| Gaps found | 7 |
| Resolved | 7 |
| Escalated | 0 |
