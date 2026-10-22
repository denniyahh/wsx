---
phase: 01-development-foundation
plan: 06
subsystem: docs
tags: [claude-md, contributing, ai-change-acceptance, repo-traps, onboarding]

requires:
  - phase: 01-development-foundation
    provides: "scripts/check.sh targets (01-01), scripts/install-tools.sh (01-02), scripts/check-in-container.sh with /workspace/target volume layout (01-03), CI jobs, parity check, deny workflow, Dependabot (01-04), .gitconfig and scripts/setup-repo.sh (01-05)"
provides:
  - "CLAUDE.md: 55-line agent guide with sections Shape of the code, Canonical commands, Repo traps (10 traps), Do not"
  - "CONTRIBUTING.md: Rust contributor guide (Status, Getting started, Repository git policy, Development commands, CI and required checks, Design constraints, AI change acceptance, Supply-chain scanning, Style, Commits)"
  - "devflow's AI change acceptance bar (5 requirements, 4 rejected test shapes) with only item 4 adapted to scripts/check.sh"
affects: [01-07 phase verification of DEV-03/DEV-04, PR cut onto develop]

actuals:
  tokens: 2994
  tasks: 2
  commits: 3
plan_head_before: ec77dd49fe1bf6ad8a7eb13c3622c37a04a404c4
plan_head_after: ed34b0ad1ade11770de0485c57d03545529d7e9f

tech-stack:
  added: []
  patterns:
    - "A file whose branch base came from an unscoped commit is first restored to develop's content in its own phase-scoped commit, so the rewrite commit's diff starts from develop and replays cleanly"

key-files:
  created: []
  modified:
    - CLAUDE.md
    - CONTRIBUTING.md

key-decisions:
  - "Getting started orders tool install before scripts/setup-repo.sh, because setup-repo refuses to run without gitleaks on PATH; signing key comes before the first commit because the included policy refuses unsigned commits"
  - "Supply-chain section claims a planted-problem proof only for the gitleaks hook and cargo-deny; push protection and Dependabot were enabled but never shown to refuse a planted problem, so the plan's 'every scanner' wording was narrowed"
  - "The stale wsx-ci-target volume warning from 01-03 is in CONTRIBUTING's Development commands: delete the volume after changing where it is mounted"

patterns-established:
  - "Public docs name no tool versions; scripts/install-tools.sh stays the single pin site"

requirements-completed: [DEV-03, DEV-04]

coverage:
  - id: D1
    description: "CLAUDE.md gives the Rust workflow and repo traps in at most 80 lines, public-safe, every named script exists"
    requirement: DEV-03
    verification:
      - kind: other
        ref: "Task 1 verify block 1 (trap phrases, four headings, <= 80 lines) plus personal-data gate with positive control and scripts/*.sh existence check"
        status: pass
      - kind: other
        ref: "Task 1 verify block 2: git merge-tree of the rewrite commit onto origin/develop, parent CLAUDE.md == develop's"
        status: pass
    human_judgment: true
    rationale: "Plan human-check: whether each trap is clear and the right one is a judgment over prose; string checks prove only that the phrases and sections exist"
  - id: D2
    description: "CONTRIBUTING.md documents the Rust workflow and carries devflow's acceptance bar and four rejected shapes"
    requirement: DEV-04
    verification:
      - kind: other
        ref: "Task 2 verify blocks 1-2 (ten headings, eight verbatim openings, exactly nine numbered items, item 4 names scripts/check.sh all, no .claude/)"
        status: pass
      - kind: other
        ref: "diff of the acceptance body against devflow CONTRIBUTING.md lines 300-325: only item 4 and the closing sentence differ"
        status: pass
      - kind: other
        ref: "Task 2 verify block 3: git merge-tree of the rewrite commit onto origin/develop"
        status: pass
    human_judgment: true
    rationale: "Plan human-check: whether Getting started, followed in order on a fresh clone, leaves a clone that can commit and run every gate is a judgment over prose and was not exercised on a fresh clone"

duration: 6min
completed: 2026-10-22
status: complete
---

# Phase 1 Plan 06: CLAUDE.md and CONTRIBUTING.md rewritten for Rust Summary

**CLAUDE.md is now a 55-line agent guide: the `scripts/check.sh` targets, one-named-test gating, the container and setup scripts, and ten repo traps (nextest no-match, `rg -c` on zero, zsh `PIPESTATUS`, `.gitignore` placement, phase-scoped subjects, `scripts/workspace/`, the common-dir include, the read-only container). CONTRIBUTING.md is a Rust guide whose Getting started runs `scripts/install-tools.sh install cargo-nextest cargo-deny gitleaks`, then `scripts/setup-repo.sh`, then a signing key, and which carries devflow's AI change acceptance bar word for word except item 4. Both rewrite commits replay onto `origin/develop` without conflict.**

## What these results do and do not establish

- **String checks prove presence, not quality.** The verify blocks show the required phrases, headings and numbered items exist and the files are short and free of emails and home paths. They do not show the prose is clear or that each trap is the right one. The two human-checks in the plan carry that judgment to the end-of-phase review.
- **Getting started was not run on a fresh clone.** Its order is derived from what the scripts enforce (setup-repo refuses without gitleaks on PATH; the included policy refuses an unsigned commit), as recorded in the 01-02 and 01-05 summaries. Nobody followed it end to end here.
- **The replay check is `git merge-tree`, not the real PR cut.** It shows each rewrite commit applies onto `origin/develop` with no conflict, and that the replayed file equals the branch file. It does not run `scripts/workspace/cut-pr-branch.sh`. A negative control showed the same check fails (exit 1) when the rewrite is replayed from the interim CLAUDE.md base, so the check can fail.
- **`origin/develop` was the local ref, not fetched.** If develop has moved on GitHub since the last fetch, the replay result may differ.

## Performance

- **Duration:** about 6 min (20:18:58Z to 20:24:23Z)
- **Started:** 2026-10-22T20:18:58Z
- **Completed:** 2026-10-22T20:24:23Z
- **Tasks:** 2 of 2
- **Files modified:** 2

## Accomplishments

- `CLAUDE.md` (55 lines): purpose and frozen-Python note; Shape of the code; Canonical commands; Repo traps (a) to (j) from the plan; Do not. No tool versions, no personal data, no negative-control literals.
- `CONTRIBUTING.md` (197 lines): Status; Getting started (rustup, pinned tools, setup-repo, signing key, `scripts/check.sh all`); Repository git policy (devflow's text, with the common-git-dir note); Development commands table, one-test usage, container volumes `wsx-ci-target-*` / `wsx-ci-registry-*` with the stale-volume warning, parity check; CI and required checks; six design constraints from PROJECT.md; AI change acceptance; Supply-chain scanning; Style (verbatim); Commits (plus the phase-scope rule).

## Task Commits

1. **Task 1: Rewrite CLAUDE.md** - `594da30` (docs: restore develop's CLAUDE.md as the base) and `db523e5` (docs: rewrite)
2. **Task 2: Rewrite CONTRIBUTING.md** - `ed34b0a` (docs)

## Verification evidence

| Check | Result |
|---|---|
| Task 1 verify 1 (trap phrases, headings, <= 80 lines) | exit 0; same phrase check on develop's CLAUDE.md exits 1 |
| Task 1 verify 2 (replay; parent == develop) | exit 0; replay from the interim base exits 1 |
| Task 1 personal-data gate (positive control matches) | exit 0 |
| Task 1 scripts/*.sh existence | exit 0 (5 paths) |
| Task 2 verify 1 (headings, eight verbatim openings) | exit 0; same check on the old CONTRIBUTING exits 1 |
| Task 2 verify 2 (nine numbered items, item 4, no `.claude/`) | exit 0; same check on the old CONTRIBUTING exits 1 |
| Task 2 verify 3 (replay) | exit 0; replayed CONTRIBUTING.md equals the branch file |
| Acceptance body vs devflow lines 300-325 | `diff` shows only item 4 and the added closing sentence |
| Style section vs the old file | `diff` shows only the added phase-scope line in Commits |
| Personal-data gate, scripts existence, control literals (Task 2) | exit 0 each |
| Single pin site for 0.9.148, 0.20.2, 8.30.1 | exit 0 each (run with stdin closed; see Issues) |

## Decisions Made

- Tool install comes before `scripts/setup-repo.sh` in Getting started, and the signing key before the first commit, because each later step refuses without the earlier one.
- The supply-chain section claims a planted-problem proof only for the gitleaks hook and cargo-deny (see Deviations).
- The 01-03 stale-volume warning is documented next to the container command.

## Deviations from Plan

**1. [Rule 1 - Accuracy] Narrowed the "every scanner was proven" claim**
- **Found during:** Task 2
- **Issue:** The plan asked CONTRIBUTING to say every scanner was proven to refuse a planted problem. Only the gitleaks hook (01-05) and cargo-deny (01-02) were. Push protection was checked as enabled, not shown to refuse anything, and Dependabot is not a refusing scanner.
- **Fix:** The text says the gitleaks hook and cargo-deny were each shown to refuse a planted problem, described generically.
- **Files modified:** CONTRIBUTING.md
- **Committed in:** `ed34b0a`

**2. [Rule 1 - Accuracy] cargo-deny workflow trigger wording**
- **Found during:** Task 2
- **Issue:** A first draft said the workflow runs on every pull request; `deny.yml` limits `pull_request` to `develop` and `main`.
- **Fix:** Corrected before the commit.
- **Committed in:** `ed34b0a`

**Total deviations:** 2 auto-fixed (both accuracy corrections to public docs). **Impact on plan:** the docs claim no more than the earlier plans proved. No scope added.

## Issues Encountered

- **The plan's single-pin-site criterion hangs as written when stdin is a pipe.** `rg -l ... -- VERSION` with no path argument searches stdin instead of the working tree when stdin is not a terminal. In this agent shell the three runs hung until I killed each `rg` by its PID (782981, 791817, 792243). Re-run with `</dev/null`, all three pass. The check is unchanged; it only needs stdin closed (or a terminal) to be meaningful. Worth fixing in the plan template: any `rg` without a path in an `<automated>` block should carry `</dev/null`.

## User Setup Required

None.

## Next Phase Readiness

- 01-07 can verify DEV-03 and DEV-04 from these two files; the two human-checks (CLAUDE.md clarity, CONTRIBUTING Getting started order) remain for the end-of-phase review.

## Self-Check: PASSED

- FOUND: CLAUDE.md, CONTRIBUTING.md
- FOUND commits on this branch: 594da30, db523e5, ed34b0a
- Measured commits since plan start: 3

---
*Phase: 01-development-foundation*
*Completed: 2026-10-22*
