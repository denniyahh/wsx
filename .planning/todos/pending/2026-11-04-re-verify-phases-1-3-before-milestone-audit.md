---
created: 2026-11-04T15:43:41.566Z
title: Re-verify Phases 1-3 and 03.1 before milestone audit
area: planning
severity: minor
files:
  - .planning/phases/01-development-foundation/01-VERIFICATION.md
  - .planning/phases/02-oracle-harness/02-VERIFICATION.md
  - .planning/phases/03-git-adapter/03-VERIFICATION.md
  - .planning/phases/03.1-resolve-ref-names-as-git-does/03.1-VERIFICATION.md
---

## Problem

The VERIFICATION.md reports for Phases 1, 2 and 3 read `stale` (checked 2026-11-04): later phases edited files they cover (Cargo.toml, deny.toml, scripts/check.sh, src/git/*, among others). It will recur as Phases 4-15 edit shared files again, so a refresh now would go stale again. `init.progress` also reports Phase 01 as current while any of these is non-passed; STATE.md and ROADMAP.md are authoritative.

Owner decision 2026-11-04: deferred, not done before Phase 4 planning.

Added 2026-11-18: Phase 03.1's report now also reads `stale` (Phase 4 edited Cargo.toml, src/git/mod.rs, refs.rs, repo.rs, testing.rs). Same deferral, owner's word 2026-11-18.

## Solution

Once, after the last phase that touches shared files and before the milestone audit, run `/gsd-execute-phase 01`, `02`, `03` and `03.1`. GSD's only refresh route resumes at the verification gates and re-runs the verifier.

Known side effect, measured on a scratch clone 2026-11-04: `state.begin-phase` and `phase.complete` overwrite STATE.md `Current focus`, both `Stopped at` lines (frontmatter and Session Continuity), `last_activity_desc`, and the Velocity row key (`1` becomes `01`). After each run, check STATE.md, restore those lines, and log the fix in `~/.config/agents/gsd-manual-fix-incidents.md`. `phase.complete` returns `next_phase: 04` (or the then-current phase) because it skips complete phases. Not measured: whether the transition workflow's `evolve_project` also edits PROJECT.md.
