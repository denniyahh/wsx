---
phase: 03-git-adapter
verified: 2026-10-28T00:57:06Z
status: passed
score: "4/4 roadmap success criteria verified; 58/58 plan truths verified (1 by override, 1 backstop verified on code-enforced ordering for the present code)"
covered_files: [".gitignore", ".planning/phases/03-git-adapter/03-01-PLAN.md", ".planning/phases/03-git-adapter/03-01-SUMMARY.md", ".planning/phases/03-git-adapter/03-02-PLAN.md", ".planning/phases/03-git-adapter/03-02-SUMMARY.md", ".planning/phases/03-git-adapter/03-03-PLAN.md", ".planning/phases/03-git-adapter/03-03-SUMMARY.md", ".planning/phases/03-git-adapter/03-04-PLAN.md", ".planning/phases/03-git-adapter/03-04-SUMMARY.md", ".planning/phases/03-git-adapter/03-05-PLAN.md", ".planning/phases/03-git-adapter/03-05-SUMMARY.md", ".planning/phases/03-git-adapter/03-06-PLAN.md", ".planning/phases/03-git-adapter/03-06-SUMMARY.md", ".planning/phases/03-git-adapter/03-07-PLAN.md", ".planning/phases/03-git-adapter/03-07-SUMMARY.md", ".planning/phases/03-git-adapter/03-08-PLAN.md", ".planning/phases/03-git-adapter/03-08-SUMMARY.md", ".planning/phases/03-git-adapter/03-09-PLAN.md", ".planning/phases/03-git-adapter/03-09-SUMMARY.md", ".planning/phases/03-git-adapter/03-10-PLAN.md", ".planning/phases/03-git-adapter/03-10-SUMMARY.md", "CONTRIBUTING.md", "Cargo.lock", "Cargo.toml", "clippy.toml", "deny.toml", "scripts/check-in-container.sh", "scripts/check.sh", "scripts/install-tools.sh", "src/git/mod.rs", "src/git/reads.rs", "src/git/repo.rs", "src/git/spawn.rs", "src/git/testing.rs", "src/main.rs", "tests/gate_controls.rs", "tests/harness/check.rs"]
covered_digest: "v4:sha256:d33062a7e5d1a2c163e0d1191d35a64f158955211f1573e067ceb6990744430a"
behavior_unverified: 0
overrides_applied: 1
overrides:
  - must_have: "the normal dependency tree contains gix v0.90.0 and no signal-hook, gix-transport, gix-protocol, reqwest or curl crate"
    reason: "The plan's wording overstated the CONTEXT decision. gix 0.90.0 depends on gix-protocol unconditionally, and gix-protocol depends on gix-transport. The decision itself, no network client, holds: gix-transport has only its empty default feature, and no reqwest, curl, gix-credentials or signal-hook crate is in the tree (verified at e61833a; rustix added in 72a4bd4 brings no new crate)"
    accepted_by: "orchestrator (on evidence)"
    accepted_at: "2026-10-27"
re_verification:
  previous_status: gaps_found
  previous_score: "4/4 roadmap; 55/58 plan truths (1 failed as worded, 1 backstop insufficient_spec, 1 uncertain)"
  gaps_closed:
    - "03-01 must_have 3 (gix-transport/gix-protocol in the tree): PASSED (override), accepted by the orchestrator on evidence"
    - "03-09 prohibition, SUDO_UID case: fixed in 72a4bd4. The verifier re-ran the new test and removed the fix in a scratch copy, where the test fails (wsx opened, git refused)"
    - "03-10 container-gate clause: the orchestrator reports scripts/check-in-container.sh all exit 0 (316 passed) at 72a4bd4; cd7d3d3 after it is planning-only"
  gaps_remaining: []
  regressions: []
coincidental_reliance_items:
  - truth: "Roadmap SC1 (reads part): merge-base answers from gix with the result git gives on the same fixture"
    reason: incidental-ordering
    harden: "On a criss-cross history with several best bases, gix returns the first base in its own order. It equals git's single answer on the one fixture tested (code doc; RESEARCH line 372); nothing makes the tie-break match git elsewhere. A caller that needs git's exact pick, or every best base, should compare merge_bases_many with `git merge-base --all` as sets."
---

# Phase 3: Git adapter Verification Report

**Phase Goal:** wsx opens the right repository from any worktree or hook, reads it with gix, and makes every history change through the `git` binary so the user's hooks, signing, merge drivers and credentials apply
**Verified:** 2026-10-28T00:57:06Z (first pass 2026-10-28T00:46:56Z at e61833a; this pass at cd7d3d3)
**Status:** passed
**Re-verification:** Yes, after the orchestrator's override and the SUDO_UID fix (72a4bd4), with the planning updates in cd7d3d3

## What this verification does and does not establish

- **Gates run by the verifier at cd7d3d3** (code identical to 72a4bd4), on this host: Fedora
  Kinoite, uid 1000, git 2.56.0.
  - `scripts/check.sh all` exit 0: `316 tests run: 316 passed, 58 skipped`.
  - `scripts/check.sh deps` exit 0: `advisories ok, bans ok, licenses ok, sources ok`.
  - The first pass at e61833a gave 315 passed; the one new test is the SUDO_UID test.
- **The container gate was not run by the verifier** (instructed not to). The orchestrator
  reports exit 0 (316 passed) at 72a4bd4. Only `.planning/` changed after that commit.
- **The root `chown` path of the foreign-owner fixture has run only in the rootless container,
  never on a GitHub runner.** On this host the tests use the user-namespace path. CI's
  root-with-CHOWN behaviour is still RESEARCH's `[ASSUMED]`.
- **Each test proves the case it builds, not the class.**
  - Hooks live in `.git/hooks` and signing in the repository's local config. Global config and
    `core.hooksPath` are not tested.
  - Merge drivers and credentials have no test. They are reached only because children run the
    real `git` with the snapshot environment unchanged, which
    `child_gets_exact_argv_cwd_and_no_bypass` pins.
  - Read parity holds on one criss-cross, clock-skewed fixture.
- **SUDO_UID test coverage is split across environments.** On a non-root host the test proves the
  refusal; the opening direction is proven only in a root run, which here means the container.
  Neither environment exercises both directions.
- **Test-first.** The `tdd.review-checkpoint` gate passed vacuously (no `type: tdd` plans).
  - History shows `test(...): add failing ...` commits before each `feat(...)` commit for 03-01,
    03-03, 03-05, 03-06, 03-08, 03-09 and 03-10. RED records exist under `target/gsd/`.
  - 72a4bd4 committed its test together with the fix. The verifier checked its failing direction
    directly (see Behavioral Spot-Checks).

## Goal Achievement

### Roadmap success criteria

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | A commit and a merge through wsx's git layer run the user's hooks and are signed; merge-base, ahead/behind, tree diff and range walk answer from gix as git does | ✓ VERIFIED (merge-base: coincidental-reliance advisory) | **Hooks and signing:** `commit_and_merge_run_hooks_and_sign` PASS: `%G?` = G,G; pre-commit 1, commit-msg 2, pre-merge-commit 1. Control `explicit_bypass_skips_hooks_and_signing` PASS: N, no hooks. **Reads:** `merge_base_matches_git`, `ahead_behind_matches_git`, `range_walk_matches_git`, `tree_diff_matches_git` PASS, each with an answer-changing control PASS. The git side of each comparison runs through wsx's own `Git::run`. |
| 2 | Inside a hook exporting `GIT_DIR` and `GIT_INDEX_FILE` from a linked worktree, wsx resolves worktree and common dir; a sibling child loses the `--local-env-vars` list and gets null stdin; the control detects the leak | ✓ VERIFIED | **Resolution:** `hook_resolves_linked_worktree` PASS. It runs a real pre-commit of a partial commit that re-executes the test binary. **Fixture is real:** a verifier probe with git 2.56.0 shows a linked-worktree pre-commit gets an absolute `GIT_DIR` and a `next-index-*.lock` index, while the main worktree gets `GIT_DIR` unset and `.git/index`. **Variable removal:** `sibling_child_env_scrubbed` PASS. Control `sibling_child_leaks_without_removal` PASS. `own_worktree_child_keeps_hook_index` PASS. **Null stdin:** `children_get_null_stdin` PASS, with control `piped_stdin_reaches_an_unmanaged_child` PASS. This runs outside a hook, but `Stdio::null` is set in `child()`, the only production constructor of git children. |
| 3 | Reftable refused with a message naming reftable (a `regression:` scenario); the same history in files backend opens | ✓ VERIFIED (the scenario's wsx side is deferred by CONTEXT decision) | **Unit tests:** `reftable_repo_is_refused`, `empty_reftable_repo_is_refused`, `files_backend_twin_opens` and `reftable_ref_decode_maps_to_the_refusal` PASS. **Harness:** `check::reftable_refused::oracle` PASS. `::rust` selected alone exits 4 with 0 tests run, and it is listed in the disabled-side report. The phase 5 obligation is in STATE.md Blockers; phase 14 SC3 is the backstop. |
| 4 | A repository owned by another user is refused, matching git | ✓ VERIFIED (host user-namespace path; root path container only) | **Refusal:** `foreign_owner_refused` PASS, with git's own `safe.directory` fix line. **Control:** `safe_directory_in_global_config_opens_a_foreign_repo` PASS. **Recorded divergence:** `command_line_safe_directory_does_not_trust_a_foreign_repo` PASS. **SUDO_UID:** `sudo_uid_naming_the_owner_trusts_a_foreign_repo_only_as_git_does` PASS. It asks git first and requires wsx to agree, and it fails when the fix is removed (see Behavioral Spot-Checks). |

### Plan truths

All 39 tests named by full path in plan frontmatter PASS in the full run. The exception is
`check::reftable_refused::rust`, which is ignored by design. Non-test truths:

| Plan | Truth | Status | Evidence |
|------|-------|--------|----------|
| 03-01 | gix `=0.90.0`, `default-features = false`, `sha1`/`sha256`/`revision` | ✓ | Cargo.toml |
| 03-01 | no gix-transport / gix-protocol in normal tree | ✓ PASSED (override) | Override: the plan's wording overstated the CONTEXT decision; the no-network-client decision holds. Accepted by the orchestrator (on evidence) on 2026-10-27. |
| 03-01 | deny allow list `[Apache-2.0, MIT, Unicode-3.0, Zlib]`, deps passes | ✓ | `scripts/check.sh deps` exit 0 at cd7d3d3. The `["MIT"]` scratch control was not re-run; it rests on the SUMMARY. |
| 03-01 | `cfg_attr(not(test), expect(dead_code, ...))`; the gate passes | ✓ | mod.rs:23; clippy passes in `all` |
| 03-02 | `to_reftable` proves the subject is reftable; both guard controls; STATE obligation | ✓ | check.rs:300-345; STATE.md Blockers |
| 03-04 | 14 banned entry points, one production expect | ✓ | 6+7+1 = 14 in clippy.toml. The only non-test `#[expect(clippy::disallowed_methods)]` is on `open_gix`. The scratch-copy clippy controls were not re-run. |
| 03-05 | prohibition: no bypass, no history change through gix | ✓ resolved | `child()` adds nothing to argv. Production code has no gix ref, transaction, commit or object write. Its only process spawn is spawn.rs:97. |
| 03-05 | backstop: concurrent first calls | ✓ VERIFIED for the present code; forward clause belongs to phase 5 | **Ordering:** `Git::run` calls `ensure_version` before `command`. **Caches:** `version_checked` and `local_env_vars` are `OnceLock`s written only after a passing check or a non-empty list, so no thread can see a cached pass that never happened. **Docs:** the `Git` doc states that the cache is per helper. `version_is_checked_once_per_helper` pins the sequential behaviour. **Not established:** "once per process" in production, which has no caller until phase 5. |
| 03-06 | prohibition: no reftable repo opened, backstop never swallowed | ✓ resolved | `refuse_ref_storage` runs before `from_gix`. `resolve` maps `is_unsupported()` to the reftable refusal. `tree_diff_keeps_the_reftable_refusal` PASS. |
| 03-07 | prohibition: every child gets null stdin | ✓ resolved | As SC2 |
| 03-08 | prohibition: hook variables never reach a child or handle aimed at a sibling | ✓ resolved, with a future-caller caveat | `siblings()` uses only `main_repo()`/`worktrees()`. Caveat: `Repo::discover(<sibling path>)` inside a hook follows the hook's variables (pinned by `discovering_a_sibling_inside_a_hook_follows_the_hook_env`). Only documentation stops a later caller from doing it. |
| 03-09 | prohibition: never open a repo git refuses as foreign-owned | ✓ resolved | `refuse_untrusted` now also refuses when a non-root process has `SUDO_UID` set and does not own the git dir or worktree (`trusted_only_through_sudo_uid`, repo.rs). It reads paths that cannot be stat'ed as not owned, so it fails closed. The accepted divergences (`-c`/`GIT_CONFIG_COUNT` `safe.directory`; a `safe.directory`-listed foreign repo while non-root with `SUDO_UID` set) are both stricter than git. |
| 03-10 | gate: check.sh all, deps, container all exit 0 on the tip | ✓ | Host `all` and `deps` were run by the verifier at cd7d3d3. The container run is the orchestrator's report at 72a4bd4; only planning files changed after it. |
| 03-10 | no `git-unit-*` and no foreign-owned file under target; 0.89 concern gone | ✓ | After both full runs and the scratch control, `find target -name 'git-unit-*'` and `find target ! -uid 1000` return nothing. |

**Score:** 4/4 roadmap criteria; 58/58 plan truths (1 by override).

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/git/mod.rs` | ✓ VERIFIED | `Error` enum, 12 variants. The review-added refusals are recorded in CONTEXT, and their phase 5 exit codes in STATE.md Blockers (cd7d3d3). |
| `src/git/repo.rs` | ✓ VERIFIED | **`discover` order:** ceiling, object-store env, trust (with the SUDO_UID re-check), ref storage, relative env, implicit worktree, then `from_gix`. |
| `src/git/reads.rs` | ✓ VERIFIED | **Reads:** four queries on `resolve`/`resolve_commit`; tree diff through `gix::diff::tree` + `Recorder`. **Tree-to-blob:** gix-diff 0.70.0 reports it as delete plus add. A directory replaced by a file has no test. |
| `src/git/spawn.rs` | ✓ VERIFIED | **Single `child()` constructor:** `env_clear`, explicit cwd, `Stdio::null`, arguments only. |
| `src/git/testing.rs` | ✓ VERIFIED | cfg(test) only |
| `Cargo.toml` (rustix) | ✓ VERIFIED | Direct dependency added in 72a4bd4, with a written justification. **No new crate:** rustix 1.1.5 was already in the graph through gix-index and tempfile; the only Cargo.lock change is the new edge from `wsx`. `deps` is clean. |
| `clippy.toml`, `deny.toml`, `tests/harness/check.rs` | ✓ VERIFIED | As in the first pass |

**gsd-tools checks.** `verify.artifacts`: all plans pass. `verify.key-links`: 12 links could not
be evaluated by the tool (11 descriptive `from` values, 1 non-RE2 pattern), so all 12 were
checked by hand and are wired. The list is in the first-pass notes: `self.resolve(`, the
`refuse_*` calls in `discover`, `Error::Reftable` from `resolve`, `current_exe`,
`with_own_worktree`, `main_repo()`, `--include-ignored`, and `git_dir_trust`/`Trust::Full`.

### Data-Flow Trace (Level 4)

Not applicable: nothing renders. Reads return gix query results converted to wsx types, with no
static fallbacks.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full gate at cd7d3d3 | `scripts/check.sh all` | exit 0; 316 passed, 58 skipped | ✓ PASS |
| Supply chain | `scripts/check.sh deps` | exit 0 | ✓ PASS |
| SUDO_UID parity | `scripts/check.sh test -E 'test(=git::repo::tests::sudo_uid_naming_the_owner_trusts_a_foreign_repo_only_as_git_does)'` | 1 passed | ✓ PASS |
| SUDO_UID negative control | `git archive 72a4bd4` into `target/`; the `&& !trusted_only_through_sudo_uid(repo)` condition replaced by `true`; the same single test run with `--no-tests=fail` | exit 100; `left: "opened"`, `right: "foreign_owner"` (git refused, wsx opened); scratch removed, no foreign-owned file left | ✓ the test detects the hole |
| Reftable oracle side | `scripts/check.sh test -E 'test(=check::reftable_refused::oracle)'` | 1 passed (first pass) | ✓ PASS |
| Reftable Rust side disabled | `scripts/check.sh test -E 'test(=check::reftable_refused::rust)'` | exit 4, 0 tests run (first pass) | ✓ PASS (expected) |
| Hook env shape | scratch probe under target/ (removed) | linked: absolute GIT_DIR + next-index lock; main: GIT_DIR unset, `.git/index` | ✓ PASS |

### Probe Execution

None declared. `scripts/*/tests/probe-*.sh` does not exist.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| GIT-01 | 03-01, 03-03, 03-05, 03-07, 03-10 | ✓ SATISFIED (hooks and signing proven; merge drivers and credentials inferred, not tested) | SC1. No history change goes through gix. |
| GIT-02 | 03-01, 03-04, 03-05, 03-07, 03-08, 03-10 | ✓ SATISFIED | SC2 |
| GIT-03 | 03-02, 03-06, 03-10 | ✓ SATISFIED (wsx side of the scenario deferred to phase 5 by CONTEXT) | SC3. The requirement text now says gix 0.90 (cd7d3d3). |
| GIT-04 | 03-09, 03-10 | ✓ SATISFIED | SC4, including SUDO_UID |

No orphaned requirements.

### Anti-Patterns Found

No TBD/FIXME/XXX/TODO/HACK markers in phase-modified files (`rg` exit 1; the control string
matched). No `#[allow` in src.

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `03-REVIEW-DISPOSITION.md` | frontmatter | IN-01..IN-03 still `open` | ℹ️ Info, resolved in substance | e61833a fixed IN-01 (test no longer detaches maintenance), deferred IN-02 to `deferred-items.md` with its effect, and recorded IN-03 in CONTEXT and, since cd7d3d3, in STATE.md Blockers. Only the disposition table lags; re-running the disposition gate would update it. |
| `.planning/STATE.md` | 157 | the first ceiling decision line describes the old rule | ℹ️ Info, settled | Superseded by the decision line cd7d3d3 added (line 160) in the append-only log |
| `deferred-items.md` | item 2 | 26 in-process discovery tests still read the runner's global/system git config | ℹ️ Info | Already recorded as the owner's call; effect not measured |

The first pass's HANDOFF.json warning ("SUDO_UID only stricter") is closed: cd7d3d3 corrects it,
along with `.continue-here.md` and CONTEXT.

### Human Verification Required

None. Every item the first pass raised was settled on evidence:
- The override was accepted by the orchestrator.
- SUDO_UID was fixed and checked by the verifier with a negative control.
- The container gate was re-run by the orchestrator.
- The backstop truth was verified on code-enforced ordering.

### Gaps Summary

No gaps. The phase goal is achieved in the codebase:
- **Opening:** wsx resolves its repository through one gix entry point that honours hook
  variables, ceilings, ownership trust (now including git's root-only `SUDO_UID` rule) and ref
  storage as git does. Each remaining divergence is refused or documented in the stricter
  direction.
- **Reading:** reads come from gix and match git on the fixture.
- **Writing:** every git child comes from one constructor that adds nothing, runs with an
  explicit cwd and null stdin, and removes git's `--local-env-vars` list when aimed at another
  worktree. That is proven inside a real hook, with a leak control.

Residual limits, none blocking:
- The root `chown` path has never run on a GitHub runner.
- Merge drivers and credentials are inferred, not tested.
- Merge-base tie-breaking matches git on one fixture only (advisory).
- `Repo::discover` on a sibling path inside a hook would follow the hook's variables if a later
  phase called it that way.

---

_Verified: 2026-10-28T00:57:06Z_
_Verifier: Claude (gsd-verifier)_
