---
phase: "1"
slug: "development-foundation"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-22"
---

# Phase 1 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo-nextest 0.9.148 (Rust); negative-control recipes C1-C7 in 01-RESEARCH.md (shell, scratch space); `shellcheck` + `bash -n` for scripts |
| **Config file** | none — `check.sh` carries all nextest flags; Wave 0 creates the crate and `check.sh` |
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
| DEV-01 | each target runs; a missing tool is named and fails | smoke + control | `scripts/check.sh all`; recipe C1 (explicit minimal `PATH`) | ❌ W0 | ⬜ pending |
| DEV-02 | pinned rustc 1.99.1 used | smoke | `rustc --version` at repo root; container log shows 1.99.1 installed | ❌ W0 | ⬜ pending |
| DEV-03/04 | docs carry traps, acceptance bar, four rejected shapes | manual review | reviewer reads (no grep-as-test) | ❌ W0 | ⬜ pending |
| DEV-05 | signing + git-flow names via include | scripted control | recipe C6 with `GIT_CONFIG_GLOBAL=/dev/null`, with and without the include | ❌ W0 | ⬜ pending |
| DEV-06 | container parity from a linked worktree; tag drift fails | smoke + control | `scripts/check-in-container.sh all` from `.worktrees/phase-1`; recipe C7 | ❌ W0 | ⬜ pending |
| SUP-01 | deny job exists, advisory | smoke + owner CI run | `scripts/check.sh deps` | ❌ W0 | ⬜ pending |
| SUP-02 | dependabot config (actions + docker), weekly, grouped, develop | review (effective only on `main`) | YAML review | ❌ W0 | ⬜ pending |
| SUP-03/05 | hook refuses a secret; deny fails a vulnerable crate | scripted control | recipes C4, C5 | ❌ W0 | ⬜ pending |
| SUP-04 | lock committed, SHAs pinned, versions in one script | smoke | `cargo metadata --locked`; every `uses:` carries a 40-hex SHA | ❌ W0 | ⬜ pending |
| HARN-04 | no-match filter fails; set_var banned | scripted control | recipe C2 (with `NEXTEST_NO_TESTS=pass`), recipe C3 | ❌ W0 | ⬜ pending |
| REL-04 | required checks on both rulesets | owner-gated | prepared command; re-read rulesets after the owner runs it | n/a | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `Cargo.toml`, `src/main.rs`, `tests/cli.rs`, `Cargo.lock`, `rust-toolchain.toml` — nothing exists yet
- [ ] `scripts/check.sh`, `scripts/install-tools.sh` — the gate itself
- [ ] No framework install on the host (all tools present, verified 2026-10-22)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| CI runs Test/Clippy/Format in the pinned image on a PR to `develop` | REL-04, DEV-06 | Opening a PR is outward-facing; owner-gated | Owner approves the PR cut and push; check `gh pr checks` on the PR's head SHA |
| Rulesets list Test, Clippy, Format as required | REL-04 | Changes GitHub settings; owner-gated | Owner runs the prepared command; re-read `gh api repos/denniyahh/wsx/rulesets/<id>` |
| Docs content | DEV-03, DEV-04 | Prose quality | Read CLAUDE.md and CONTRIBUTING.md against the requirement text |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
