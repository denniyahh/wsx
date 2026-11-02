---
phase: "4"
slug: "repo-model"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-02"
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | libtest unit tests in the binary crate, proptest 1.12.0 (added in this phase), run by cargo-nextest 0.9.148 |
| **Config file** | `.config/nextest.toml` (exists); `--no-tests=fail` is fixed by `scripts/check.sh` |
| **Quick run command** | `scripts/check.sh test -E 'test(=<module::tests::name>)'`, one test per command, chained with `&&` |
| **Full suite command** | `scripts/check.sh all`, plus `scripts/check.sh deps` |
| **Estimated runtime** | Not timed for the whole suite. A research prototype's unit tests alone took 4.4 s; the harness was not included in that figure |

---

## Sampling Rate

- **After every task commit:** Run the task's named tests through the quick run command
- **After every plan wave:** Run `scripts/check.sh all`
- **Before `/gsd-verify-work`:** `scripts/check.sh all`, `scripts/check.sh deps` and `scripts/check-in-container.sh all` must be green
- **Max feedback latency:** Not measured; see Estimated runtime

---

## Per-Task Verification Map

To be filled from the phase plans. The proposed test for each requirement, with its control, is in `04-RESEARCH.md` § Validation Architecture.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `Cargo.toml`, `Cargo.lock`: add proptest with a justification comment; `scripts/check.sh deps` clean
- [ ] `src/git/mod.rs`: `#[cfg(test)] pub(crate) mod testing;`, the new `mod` lines, `pub use repo::Repo;`
- [ ] `src/main.rs`: `mod model;`
- [ ] `src/model/mod.rs`, `src/model/manifest.rs`: signatures with wrong-answer stubs, so RED comes from an assertion; `src/model/mod.rs` carries the `dead_code` expectation
- [ ] Shared model-test fixtures: a helper building a repo with a remote, `develop`, `next`, and a `workspace/dev` holding a `.wsx.conf`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency measured and recorded
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
