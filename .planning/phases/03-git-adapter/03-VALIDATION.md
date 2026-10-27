---
phase: "3"
slug: "git-adapter"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-26"
---

# Phase 3 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.
> Source: `03-RESEARCH.md` § Validation Architecture. Test names are proposals until the plans fix them.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | libtest unit tests inside the binary crate (`src/git/`), run by cargo-nextest 0.9.150 |
| **Config file** | `.config/nextest.toml` (exists); `--no-tests=fail` is fixed by `scripts/check.sh` |
| **Quick run command** | `scripts/check.sh test -E 'test(=git::<module>::tests::<name>)'`, one test per command, chained with `&&` |
| **Full suite command** | `scripts/check.sh all` |
| **Estimated runtime** | Not measured yet for phase 3; phase 2's nextest run took about 27 seconds |

---

## Sampling Rate

- **After every task commit:** the task's named tests, through the quick run command
- **After every plan wave:** `scripts/check.sh all`
- **Before `/gsd-verify-work`:** `scripts/check.sh all`, `scripts/check.sh deps` and `scripts/check-in-container.sh all` (the container run exercises the root foreign-owner path) must all be green
- **Max feedback latency:** one named-test run

---

## Per-Task Verification Map

Filled by the planner as task IDs are fixed. Requirement → test map from research:

| Requirement | Behavior | Test Type | Proposed test |
|-------------|----------|-----------|---------------|
| GIT-01 | A commit and a merge made through the helper run the hooks and are signed (`G`). Control: explicit `--no-verify`/`gpgsign=false` gives `N` and no hook runs | unit (spawns git) | `git::spawn::tests::commit_and_merge_run_hooks_and_sign` |
| GIT-01 | merge-base, ahead/behind, tree diff and range walk equal git on a criss-cross fixture with a skewed commit date; each has a control that changes the answer | unit, differential | one test per query, e.g. `git::repo::tests::merge_base_matches_git` |
| GIT-02 | Inside wt1's pre-commit, wsx resolves wt1, the common dir and the hook's index | unit, re-exec child | `git::repo::tests::hook_resolves_linked_worktree` |
| GIT-02 | A child aimed at a sibling sees the sibling's git dir. Control: with removal disabled it sees wt1's (leak detected) | unit, re-exec child | `git::spawn::tests::sibling_child_env_scrubbed` |
| GIT-02 | A child aimed at wsx's own worktree keeps the hook's `GIT_INDEX_FILE` | unit, re-exec child | `git::spawn::tests::own_worktree_child_keeps_hook_index` |
| GIT-02 | Null stdin: `hash-object --stdin` gives the empty blob. Control: inherited stdin gives the marker blob | unit, re-exec child | `git::spawn::tests::children_get_null_stdin` |
| GIT-02 / CONTEXT | git < 2.54 is refused, naming the version found and the version required | unit (fake `git` on the child PATH) | `git::spawn::tests::old_git_refused` |
| GIT-03 | Reftable is refused, naming reftable; a files-backend repo with the same history opens | unit | `git::repo::tests::reftable_refused` |
| GIT-03 | Harness `regression:` scenario registered, oracle side recorded, Rust side disabled | harness | exact generated name set by the plan |
| GIT-04 | A repo really owned by another uid is refused by wsx and by git; a `safe.directory` control opens both | unit, re-exec child | `git::repo::tests::foreign_owner_refused` |
| CONTEXT | A SHA-256 repo opens | unit | `git::repo::tests::sha256_repo_opens` |
| CONTEXT | `GIT_CEILING_DIRECTORIES` matches git (refuses at the ceiling dir; control opens below it) | unit, differential | proposed by the plan |
| CONTEXT | The gix open/discover ban list is pinned (each banned entry point called under `expect`) | compile-time (clippy) | `scripts/check.sh clippy` |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `src/git/` test support: scratch-dir resolver (`current_exe()` → `deps/../../tmp`), hermetic env builder, fixture builder, re-exec helper with marker-line parsing, foreign-owner guard that restores ownership on drop
- [ ] `Cargo.toml` gix line, the `deny.toml` license additions (`Apache-2.0`, `Unicode-3.0`, `Zlib`), and the `clippy.toml` ban entries
- [ ] `tests/harness/check.rs`: the reftable scenario and its fixture step

---

## Manual-Only Verifications

All phase behaviors have automated verification. The foreign-owner test needs either an
unprivileged user namespace (host) or root (container); where neither is available it fails by
design rather than skipping.

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < one named-test run
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
