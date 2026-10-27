---
gsd_state_version: "1.0"
milestone: v0.1.0
current_phase: 3
current_phase_name: Git adapter
status: executing
stopped_at: Completed 03-02-PLAN.md
last_updated: "2026-10-27T01:52:49.148Z"
last_activity: 2026-10-26
last_activity_desc: Phase 3 execution started
state_head: 3daf0b1bb99d40dab3edcf295adabdc576b972bd
progress:
  total_phases: 15
  completed_phases: 2
  total_plans: 32
  completed_plans: 24
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-10-26)

**Core value:** wsx must never let a personal file reach a shared branch or PR, and must never lose work while moving it between the workspace branch, feature worktrees and PR branches.
**Current focus:** Phase 3 — Git adapter

## Current Position

Phase: 3 (Git adapter) — EXECUTING
Plan: 3 of 10
Status: Ready to execute
Last activity: 2026-10-26 — Phase 3 execution started

Progress: [█░░░░░░░░░] 13%

## Performance Metrics

**Velocity:**
- Total plans completed: 22
- Average duration: -
- Total execution time: 0.0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1 | 7 | - | - |
| 2 | 15 | - | - |

**Recent Trend:**
- Last 5 plans: -
- Trend: -

*Updated after each plan completion*
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 1 P01 | 4 min | 2 tasks | 9 files |
| Phase 01 P02 | 6min | 2 tasks | 3 files |
| Phase 01 P05 | 5min | 2 tasks | 2 files |
| Phase 01 P03 | 9min | 2 tasks | 3 files |
| Phase 01 P04 | 4min | 2 tasks | 4 files |
| Phase 01 P06 | 6min | 2 tasks | 2 files |
| Phase 02 P01 | 3min | 3 tasks | 10 files |
| Phase 02 P02 | 18min | 2 tasks | 9 files |
| Phase 02 P03 | 7min | 3 tasks | 12 files |
| Phase 02 P04 | 7 min | 2 tasks | 2 files |
| Phase 2 P05 | 11 min | 2 tasks | 3 files |
| Phase 02 P06 | 6 min | 2 tasks | 4 files |
| Phase 02 P07 | 37 min | 2 tasks | 6 files |
| Phase 02 P08 | 8min | 2 tasks | 3 files |
| Phase 02 P09 | 9 min | 2 tasks | 1 files |
| Phase 2 P10 | 9 min | 3 tasks | 1 files |
| Phase 02 P11 | 8 min | 2 tasks | 1 files |
| Phase 02 P12 | 9 min | 2 tasks | 2 files |
| Phase 02 P13 | 4min | 2 tasks | 1 files |
| Phase 02 P14 | 10min | 2 tasks | 1 files |
| Phase 02 P15 | 7 min | 2 tasks | 4 files |
| Phase 03 P01 | 13 min | 2 tasks | 9 files |
| Phase 03 P02 | 2 min | 2 tasks | 2 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Roadmap: horizontal layers (owner). Phases run strictly in order; each oracle-backed layer (Phases 5-10) enables and passes the Rust side of its scenarios before the next starts
- Roadmap: every HARN requirement lands by Phase 2 (HARN-04, the nextest gate and clippy env-mutation ban, in Phase 1); later layers enable the Rust side of the scenarios Phase 2 records against the oracle
- Roadmap: the `--json`/exit-code contract (IF-01) and the no-GSD rule (IF-02) start with the first command (Phase 5), so every later command joins an enforced contract
- Roadmap: a separate release gate (Phase 14) precedes cutover; cutover order is the wsx repo, then devflow's repo tooling (Phase 15)
- [Phase 1]: tracked `.gitconfig` is copied into local config by `setup-repo.sh` (allowlisted keys; `all|policy|check`), never included (owner, 2026-10-24)
- [Phase 1]: tool pins are verified through `cargo` itself (`install-tools.sh verify-cargo`), not by a PATH or CARGO_HOME search (code review CR-02)
- [Phase 1]: the PR cut aborts on a failed cherry-pick, refuses protected branches, and re-cuts only with `--force` (CR-03, WR-04)
- [Phase 1]: phase-1 fixes made after PR #2 merged (f1f1303) reach develop only through a follow-up PR; a phase-N cut replays only phase-N commits
- [Phase 2]: the vendored oracle stays byte-identical to devflow ee6ec792; oracle.patch is its only adaptation and is applied with git apply under GIT_CEILING_DIRECTORIES (without it, git apply in a nested repo skips and exits 0)
- [Phase 2]: deny.toml license-checks dev-dependencies (include-dev = true); cargo-deny 0.20.2 skips them by default
- [Phase 2]: Harness fixture steps are fn(&mut Fixture) -> Result<(), String> and run_case(side, &Entry, &Case, make_env) returns fixture and adapter errors instead of panicking (02-02)
- [Phase 2]: Adapters return Result<ActionRun, AdapterError { NotImplemented | Spawn }>; an oracle-side NotImplemented panics naming the scenario and action (02-02)
- [Phase 2]: CommitExpect has no committer date per the 02-02 contract; only the author date is pinned to the action instant (resolved 2026-10-26 by the orchestrator: no field — HARN-03 lists commits as message + author + author date + committer name/email + tree and deliberately leaves out the committer date, which a replay or merge sets at run time and so cannot be pinned the same way for oracle and Rust)
- [Phase 2]: Harness registry test allows a scenario module with no scenarios yet (scenarios! {}); a scenario fn left out of scenarios! is rejected by clippy as unused (02-03)
- [Phase 2]: Fixture::commit and Fixture::git_tick take dates from one private Fixture::tick; a git_tick command must create exactly one commit, which is documented but not enforced (02-03)
- [Phase 02]: 02-04: run_script always sets WSX_ORACLE_DIR, so the absent-variable oracle test spawns the same bash command line itself; the with-variable test also requires a stdout marker because phase-worktree.sh exits 0 when the oracle dir is wrong
- [Phase 2]: 02-05: a fixed: case is judged at run time against its expected outcome before its record (judge returns FixedPassesOnOracle when the oracle run meets the expectation); FixedRecordMatchesExpected stays as the early literal-copy check
- [Phase 2]: 02-05: a rust_only scenario's cases must all use OracleSide::NotImplemented; SameAsExpected counts as claiming an oracle side
- [Phase 2]: 02-05: guard control direction is judged by expected exit only (known-positive non-zero, clean 0); a regression: refusal is exit != 0, RefsExpect::Unchanged and no new commits
- [Phase 2]: 02-05: validate runs inside try_run_with_env, so every generated test validates before any fixture is built; reports are '<module>::<name>, <case><class>: <failure>'
- [Phase 02]: 02-06: cargo-llvm-cov is pinned in install-tools.sh with direct_sub=llvm-cov (the binary rejects a bare --version when run directly); install and verify pass that word, verify-cargo does not; its release binary prints exactly 'cargo-llvm-cov 0.9.1', so the pattern keeps the $ anchor
- [Phase 02]: 02-06: llvm-tools comes only from rust-toolchain.toml (rustup toolchain install sufficed on the host; the pinned image installs it too); every cargo llvm-cov run sets CARGO_LLVM_COV_SETUP=no
- [Phase 02]: 02-06: one-time coverage proof recorded: println! count 1 for the spawned subject, 0 for no spawn and 0 for spawn without LLVM_*; no gate or CI job runs coverage
- [Phase 2]: 02-07: worktree::self_test is fixed: by observation (oracle dir .worktrees/phase-1 vs wsx .worktrees/feature/phase-1); cut::self_test is Parity by observation (script returns to the source branch)
- [Phase 02]: 02-08: names_are_observed_byte_exact declares i18n.commitEncoding=ISO-8859-1, because git itself re-encodes a non-UTF-8 message byte under the default UTF-8 encoding
- [Phase 02]: 02-08: comparator needed no fix; all 25 single-field mutations of sync::self_test are reported as their own field
- [Phase 02]: 02-08: signing is declared with RepoConfig::with_signing(); all three signing cases are Parity by observation
- [Phase 02]: 02-09: the planted host config sets gpg.program to a recording script that fails, because a real gpg started a keyboxd daemon under the scenario HOME that outlived the test
- [Phase 02]: 02-09: the planted-host proof's env-value check is the only check that catches a leaked HOME; the behavioural checks (marker, decoy, outcomes) do not in the sync, cut and guard self-tests
- [Phase 2]: 02-10: guard scenario classes come from the first Parity run; P1-P5 reproduced as researched. In the fork fixture --remotes=origin did not over-scan; the hook's tail refused the workspace push on a hard-coded origin/develop instead
- [Phase 2]: cargo fmt does not reach scenario modules declared inside harness_modules!; guard.rs is formatted with rustfmt directly and the gap is logged in deferred-items.md and WINDOWS.md
- [Phase 02]: 02-11: every cut case was classed by its first Parity run; 6 subjects fixed:, integration_merged_into_feature and all 7 counterparts Parity, all as predicted
- [Phase 02]: 02-11: merge_resolution_adds_personal_file is fixed: only for the stray feature/topic-pr ref; the oracle refuses on an earlier replay conflict and never reaches the merge
- [Phase 02]: 02-11: a pre-existing PR branch in a cut fixture is never left checked out in another worktree (git would refuse the oracle's branch -f and checkout)
- [Phase 02]: 02-12: every oracle case classed by its first Parity run: 7 Parity (all 6 sync cases, the worktree re-run counterpart), 5 fixed: (the 3 worktree subjects and 2 directory-name counterparts)
- [Phase 02]: 02-12: worktree::no_gsd_repo's oracle refusal exits 2, not RESEARCH's exit 1 (the error-path grep fails on the missing ROADMAP.md; pipefail and set -e pass 2 through); recorded as observed
- [Phase 02]: 02-12: sync::fork_layout's counterpart (upstream removed) expects exit 128, taken from the oracle run as the plan directs; no requirement fixes that exit code, phase 7 may choose another
- [Phase 02]: 02-13: check::quoted_personal_names and check::no_gsd_repo are fixed: (oracle exit 1, refusing C-quoted .planning names and, without .workspace-divergence, .wsx.conf and notes/todo.md); check::fork_layout is Parity against upstream/develop; no_gsd_custom_personal_path is Rust-only
- [Phase 02]: 02-14: fork_layout advances upstream/develop so the expected PR parent (feat: add d) tells upstream/develop apart from local develop
- [Phase 02]: 02-14: merge_resolution_only_personal_file reads PRCUT-01 own resolution changes as changes against every parent (combined diff); a conflict resolved to develop's side is not one. Phase 9 should confirm
- [Phase 02]: 02-14: all four planned cut oracle scenarios observed Parity as predicted; fork counterpart exit 128 recorded; the orchestrator-added merge counterpart is fixed:
- [Phase 02]: 02-15: harn06_gaps names an untagged HARN-06 kind once (not per action); ORACLE_ACTIONS lists 19 (kind, action) pairs that need an oracle-recorded scenario
- [Phase 02]: 02-15: vendored oracle file modes are checked from the git index (git ls-files -s) in provenance.rs; every scenario module must register a scenario
- [Phase 03]: Owner 2026-10-26: planning granularity stays `fine` for phase 3 onward (question carried from the phase 2 pause; config unchanged)
- [Phase 03]: 03-01: merge-base control pairs main/feat~3 (F1) with main~2/feat (C1); git's single base for main/feat on the criss-cross fixture is F1, so comparing against it cannot catch a constant
- [Phase 03]: 03-01: gix-protocol/gix-transport are unconditional gix 0.90.0 deps; the no-network check asserts gix-transport has only its empty default feature and no client crate (gix-credentials, curl, reqwest) is in the tree
- [Phase 03]: 03-02: the oracle's record on reftable is verdict(0), observed through the harness (exit 0, refs unchanged); it matches RESEARCH's scratch prediction

### Pending Todos

- [2026-10-25] [tooling] Re-check Dependabot after first merge to main — [todo file](.planning/todos/pending/2026-10-25-re-check-dependabot-after-first-merge-to-main.md)
- [2026-10-26] [tests] Replace oracle-observed exit 128 under IF-01 (phases 7/9, accepted risk AR-02-02) — [todo file](.planning/todos/pending/2026-10-26-replace-oracle-observed-exit-128-under-if-01.md)

### Blockers/Concerns

- Phase 12: MIG-03 covers the 0.1.0 projects on the owner's work PC, which are not on this machine (checked 2026-10-20); their expected results must be recorded and checked there
- Phase 3: gix 0.89 method names are unverified against a stale crate-status item (research/SUMMARY.md, Gaps); spike at phase start
- Until Phase 15, this repo's own PR cuts still use `scripts/workspace/cut-pr-branch.sh`, which is untested here (research/DEVFLOW-SETUP.md section 5)
- Phases 5-10: each phase that enables a Rust side must pin wsx's own refusal stderr (`rust_stderr`; the registry rule enforces it) and keep the comparator's checkout/config checks green; no wsx side has run as an enabled scenario yet (Phase 2 verification)
- Host linuxbrew cargo-nextest is 0.9.150 (pin 0.9.148) and shadows ~/.cargo/bin; phase-3 gates run with PATH=$PWD/target/gsd-tools:$PATH (pinned tools installed there by scripts/install-tools.sh) until the pin or host tool is reconciled
- Phase 5: enable check::reftable_refused's Rust side with wsx check, give it a rust_stderr naming reftable, and replace its placeholder refusal exit 1 with the IF-01 exit code (scenario added in 03-02; the unit-level refusal is proven by git::repo::tests::reftable_repo_is_refused in 03-06)

## Deferred Items

Items acknowledged and deferred at milestone close, most recent first:

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| *(none)* | | | | |

## Session Continuity

Last session: 2026-10-27T01:52:48.952Z
Stopped at: Completed 03-02-PLAN.md
Resume file: None
