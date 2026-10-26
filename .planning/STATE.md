---
gsd_state_version: "1.0"
milestone: v0.1.0
current_phase: 2
current_phase_name: Oracle harness
status: executing
stopped_at: Completed 02-05-PLAN.md
last_updated: "2026-10-26T17:37:36.795Z"
last_activity: 2026-10-26
last_activity_desc: Phase 2 execution started
state_head: db12777e4a9f568b5e7a6f18271c31ade9d85816
progress:
  total_phases: 15
  completed_phases: 1
  total_plans: 22
  completed_plans: 12
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-10-25)

**Core value:** wsx must never let a personal file reach a shared branch or PR, and must never lose work while moving it between the workspace branch, feature worktrees and PR branches.
**Current focus:** Phase 2 — Oracle harness

## Current Position

Phase: 2 (Oracle harness) — EXECUTING
Plan: 6 of 15
Status: Ready to execute
Last activity: 2026-10-26 — Phase 2 execution started

Progress: [█░░░░░░░░░] 7%

## Performance Metrics

**Velocity:**
- Total plans completed: 7
- Average duration: -
- Total execution time: 0.0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1 | 7 | - | - |

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

### Pending Todos

- [2026-10-25] [tooling] Re-check Dependabot after first merge to main — [todo file](.planning/todos/pending/2026-10-25-re-check-dependabot-after-first-merge-to-main.md)

### Blockers/Concerns

- Phase 12: MIG-03 covers the 0.1.0 projects on the owner's work PC, which are not on this machine (checked 2026-10-20); their expected results must be recorded and checked there
- Phase 3: gix 0.89 method names are unverified against a stale crate-status item (research/SUMMARY.md, Gaps); spike at phase start
- Until Phase 15, this repo's own PR cuts still use `scripts/workspace/cut-pr-branch.sh`, which is untested here (research/DEVFLOW-SETUP.md section 5)

## Deferred Items

Items acknowledged and deferred at milestone close, most recent first:

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| *(none)* | | | | |

## Session Continuity

Last session: 2026-10-26T17:37:36.680Z
Stopped at: Completed 02-05-PLAN.md
Resume file: None
