---
phase: "3"
slug: "git-adapter"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: validated
nyquist_compliant: true
wave_0_complete: true
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

### As built (audited 2026-10-27 at `1a34613`)

Every test below exists and passed in `scripts/check.sh all` at `1a34613` (259 run, 259 passed,
53 skipped; the orchestrator re-ran it independently). Names were cross-checked against
`cargo nextest list`: all 76 test references in the ten plans resolve. The four `*_child` halves
(`null_stdin_child`, `linked_hook_spawn_child`, `foreign_owner_child`, `ceiling_child`) are
`#[ignore]`d by design and run only when their parent re-executes them; `check::reftable_refused::rust`
is disabled until phase 5 (STATE.md blocker).

| Requirement | Plan(s) | Tests (module `git::`) | Status |
|-------------|---------|------------------------|--------|
| GIT-01 reads equal git | 03-01, 03-03 | `reads::tests::{merge_base_matches_git, merge_base_moves_with_the_pair, ahead_behind_matches_git, ahead_behind_changes_with_the_pair, range_walk_matches_git, range_walk_changes_with_the_hidden_tip, tree_diff_matches_git, tree_diff_changes_with_the_pair}` + edge cases | ✅ green |
| GIT-01 hooks and signing | 03-07 | `spawn::tests::{commit_and_merge_run_hooks_and_sign, explicit_bypass_skips_hooks_and_signing}` | ✅ green |
| GIT-02 resolution in a hook | 03-08 | `repo::tests::{hook_resolves_linked_worktree, hook_resolves_main_worktree_with_relative_index}` | ✅ green |
| GIT-02 sibling scrub + leak control | 03-05, 03-08 | `spawn::tests::{sibling_target_removes_local_env_vars, sibling_child_env_scrubbed, own_worktree_child_keeps_hook_index}` | ✅ green |
| GIT-02 null stdin | 03-07 | `spawn::tests::children_get_null_stdin` | ✅ green |
| GIT-02 git ≥ 2.54 | 03-05 | `spawn::tests::old_git_refused` | ✅ green |
| GIT-02 ceiling parity | 03-10 | `repo::tests::ceiling_*` (4 tests) + `repo::tests::starting_in_a_ceiling_that_holds_the_repository_opens_it` | ✅ green |
| GIT-03 reftable refused | 03-06, 03-10 | `repo::tests::{reftable_repo_is_refused, empty_reftable_repo_is_refused, explicit_files_ref_storage_opens, miscased_ref_storage_is_refused_as_git_refuses_it, ref_storage_in_{global,worktree,environment}_config_does_not_refuse_a_files_repo}` | ✅ green |
| GIT-03 harness `regression:` scenario | 03-02 | `check::reftable_refused::oracle` (Rust side disabled until phase 5) | ✅ green (oracle side) |
| GIT-04 foreign owner | 03-09 | `repo::tests::{foreign_owner_refused, safe_directory_in_global_config_opens_a_foreign_repo, command_line_safe_directory_does_not_trust_a_foreign_repo}` | ✅ green (user-namespace path on host; root path in container) |
| CONTEXT SHA-256 | 03-04 | `repo::tests::sha256_repo_opens` | ✅ green |
| CONTEXT open ban pinned | 03-04 | `repo::tests::banned_gix_entry_points_refuse_a_missing_path` + `scripts/check.sh clippy` | ✅ green |
| Test hermeticity | 03-10 | `testing::{discovering_tests_pass_with_a_runner_git_dir, a_runner_git_dir_redirects_unscrubbed_discovery, rerun_scrubbed_discovers_the_fixture_under_a_runner_git_dir}` | ✅ green |

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

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < one named-test run
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** approved 2026-10-27

## Validation Audit 2026-10-27

| Metric | Count |
|---|---|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |
