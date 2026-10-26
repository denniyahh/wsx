---
phase: 02-oracle-harness
plan: 03
subsystem: testing
tags: [harness, fixture, git, fork-layout, no-gsd, module-tree]

requires:
  - phase: 02-oracle-harness
    provides: "02-02: the wsx::harness target, Fixture with the devflow preset and clock, scenarios! with its empty form, harness_modules!"
provides:
  - "Layout::NoGsd and RepoConfig::no_gsd(): workspace commit 1 adds only notes/todo.md and a .wsx.conf of `only notes`"
  - "RepoConfig::{with_remotes, with_manifest_entry, with_local_config} and ManifestKind::Modified"
  - "Fixture::git_tick (a raw commit-creating git command dated at the next tick) and Fixture::remove"
  - "Four layout tests: devflow preset, no-GSD, fork, per-commit clock"
  - "Private header-only modules provenance, hermeticity, coverage, registry"
  - "harness_modules!(sync, worktree, check, cut, guard, signing), each new scenario module holding an empty scenarios! {}"
affects: [02-04, 02-05, 02-06, 02-07, 02-08, 02-09, 02-10, 02-11, 02-12, 02-13, 02-14, 02-15]

actuals:
  tokens: 7171   # chars/4 over `git diff d2f63cf fcd3b1d` (28685 chars)
  tasks: 3
  commits: 3
plan_head_before: d2f63cfacda5c6bd91238d774db4e26c0b58d2c8
plan_head_after: fcd3b1de7f861008ee89e9f6c80914469a32ce1d

tech-stack:
  added: []
  patterns:
    - "Every preset shares one build path; the layout only chooses which workspace files commit 1 adds"
    - "Every fixture commit, from commit or git_tick, takes the next tick through one private Fixture::tick"
    - "Fixture tests compare git output as strict UTF-8 text, so a failure prints readable text and a non-UTF-8 byte fails instead of being rendered lossily"

key-files:
  created:
    - tests/harness/provenance.rs
    - tests/harness/hermeticity.rs
    - tests/harness/coverage.rs
    - tests/harness/registry.rs
    - tests/harness/worktree.rs
    - tests/harness/check.rs
    - tests/harness/cut.rs
    - tests/harness/guard.rs
    - tests/harness/signing.rs
  modified:
    - tests/harness/fixture.rs
    - tests/harness/main.rs
    - tests/harness/scenario.rs

key-decisions:
  - "scenario::registry_entries_match_their_scenarios no longer asserts that every module registers a scenario: the plan requires five empty modules and that test passing. A scenario fn left out of scenarios! {} is already rejected by clippy as unused (measured by control)"
  - "The no-GSD check scans every path component of every ref's tree, in work and in each bare remote, plus the work checkout on disk, against six names: .planning, .workspace-divergence, CLAUDE.md, .claude, .codex, devflow.toml"
  - "Fixture::commit now takes its date from Fixture::tick, which advances the clock before the git commit instead of after it succeeds; dates on success are unchanged, and a failed commit aborts the fixture anyway"

patterns-established:
  - "A new scenario module is a //! contract header plus scenarios! {}; later plans add scenarios inside it without touching main.rs"
  - "Module headers describe purpose and isolation in plain words and name no harness type, test or constant a later plan's baseline check expects to be absent"

requirements-completed: [HARN-01, HARN-06]

coverage:
  - id: D1
    description: "The devflow layout carries both manifests with extra `only` and `modified` entries appended, the ROADMAP phase heading, workspace/dev checked out and on the push remote, and wsx.sourceBranch / wsx.baseRef in local config"
    requirement: HARN-01
    verification:
      - kind: integration
        ref: "tests/harness/fixture.rs#fixture::devflow_layout_matches_the_preset (scripts/check.sh test -E 'test(=fixture::devflow_layout_matches_the_preset)')"
        status: pass
      - kind: other
        ref: "control E: extra entries not appended to .workspace-divergence -> the test fails on `.workspace-divergence on workspace/dev`"
        status: pass
    human_judgment: false
  - id: D2
    description: "The no-GSD layout has no GSD, devflow or agent file on any ref or in the checkout, and has .wsx.conf = `only notes` on workspace/dev with exactly four files"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/fixture.rs#fixture::no_gsd_layout_has_no_gsd_devflow_or_agent_files"
        status: pass
      - kind: other
        ref: "controls A (devflow layout in its place: 9 offending ref paths), H (committed docs/CLAUDE.md: caught on 3 refs) and I (untracked sub/.claude in the checkout: caught by the disk scan)"
        status: pass
    human_judgment: false
  - id: D3
    description: "The fork layout (base upstream, push origin) has refs/remotes/upstream/develop and refs/remotes/origin/workspace/dev but no refs/remotes/origin/develop; the bare remotes split the same way; the with_local_config pair is set"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/fixture.rs#fixture::fork_layout_has_no_integration_branch_on_the_push_remote"
        status: pass
      - kind: other
        ref: "control B2: build also pushes develop to the push remote -> fails on `!has_ref(... refs/remotes/origin/develop)`"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every fixture commit (commit, a committed remove, a git_tick merge) gets author and committer date T0 + 60k in step order, all distinct"
    requirement: HARN-01
    verification:
      - kind: integration
        ref: "tests/harness/fixture.rs#fixture::fixture_commits_have_distinct_dates"
        status: pass
      - kind: other
        ref: "controls C (git_tick undated: merge at wall-clock time), D2 (git_tick does not advance the clock: merge and the next commit share 1767225840), G (remove is a no-op: the next commit has nothing to commit)"
        status: pass
    human_judgment: false
  - id: D5
    description: "Nine private header-only modules exist, main.rs declares them without pub, harness_modules! lists sync, worktree, check, cut, guard, signing, and the empty registrations expand without warnings"
    verification:
      - kind: other
        ref: "scripts/check.sh all at fcd3b1d: rc 0, 62 passed, 2 skipped, `==> check.sh: all OK`"
        status: pass
      - kind: other
        ref: "header loop and forbidden-name rg over the nine files: rc 0; controls: a file without //! is reported, a planted `Fixed` is found"
        status: pass
      - kind: unit
        ref: "tests/harness/scenario.rs#scenario::registry_entries_match_their_scenarios"
        status: pass
    human_judgment: false

duration: 7min
completed: 2026-10-26
status: complete
---

# Phase 2 Plan 03: Fixture layouts and module tree Summary

**The fixture now builds a no-GSD repo and a fork with separate base and push remotes, each pinned by a test with negative controls; every commit-creating fixture step takes its own 60-second tick; and all nine modules later plans own exist as private header-only files, five of them already registered with `harness_modules!`.**

## Performance

- **Duration:** about 7 min
- **Started:** 2026-10-26T17:03:30Z
- **Completed:** 2026-10-26T17:10:37Z
- **Tasks:** 3
- **Files modified:** 12 (9 created, 3 modified)

## Accomplishments

- `RepoConfig::no_gsd()`: `workspace/dev` holds exactly `.wsx.conf`, `README.md`, `notes/todo.md` and `src/a.txt`. No ref in `work` or on the remote, and nothing on disk in `work`, carries any of the six GSD, devflow or agent names.
- `RepoConfig::devflow().with_remotes("upstream", "origin")`: `work` has exactly the remotes `origin` and `upstream`. `upstream` alone holds `develop`, `origin` alone holds `workspace/dev`, and `wsx.baseRef` is `upstream/develop`.
- `Fixture::git_tick` and `Fixture::remove` share the clock with `commit`. In the date test, six commits (two presets, two `commit`s, a `git_tick` merge, a committed removal) carry `T0 + 60k` for k = 0 to 5, in step order, as both author and committer date.
- `main.rs` declares `provenance`, `hermeticity`, `coverage` and `registry`, and `harness_modules!(sync, worktree, check, cut, guard, signing)`. No `pub mod` anywhere, and `ls tests/harness` matches the plan's 18-file list exactly.

## Task Commits

1. **Task 1: No-GSD and fork fixture layouts, each proven by a test**: `38930d3` (feat)
2. **Task 2: Declare the harness support modules**: `3695d14` (chore)
3. **Task 3: Register the scenario modules**: `fcd3b1d` (chore)

## Verification evidence and negative controls

Every `<verify>` command was run before its commit:

| Check | Result |
|---|---|
| Each of the four `fixture::` tests, one `check.sh test -E 'test(=...)'` per test | rc 0, `1 passed` each |
| `scripts/check.sh all` after Task 1, Task 2 and Task 3 | rc 0, `==> check.sh: all OK`, 62 passed, 2 skipped (58 before this plan, plus the 4 new tests) |
| Header loop over the support modules, then over the scenario modules | rc 0 |
| Forbidden-name `rg` over all nine new files | no match (exit 1, negated to 0) |
| `scenario::registry_entries_match_their_scenarios` | rc 0 |
| Acceptance `rg -q` checks (`fn no_gsd`, `fn with_remotes`, `Modified`, `mod registry;`, the widened macro) | rc 0 each, against baselines of rc 1 at the previous commit; `rg -q 'pub mod' main.rs` rc 1 |

Controls. Each was a temporary mutation, checked to have applied, then reverted and confirmed with `cmp`:

| Control | Mutation | Observed |
|---|---|---|
| A | the no-GSD test builds the devflow layout | fails, listing 9 offending ref paths |
| B | `with_remotes("upstream", "upstream")` | fails, but on the remote list, so it did not exercise the integration-branch check; superseded by B2 |
| B2 | the build also pushes `develop` to the push remote | fails on `!has_ref(&fixture, WORK, "refs/remotes/origin/develop")` |
| C | `git_tick` runs with the undated env | fails: merge dated at wall-clock time 1793034470 |
| D2 | `git_tick` dates the current tick but does not advance it | fails: the merge and the following commit share 1767225840 |
| E | extra entries not appended to `.workspace-divergence` | fails: `.workspace-divergence on workspace/dev`, got `"only .wsx.conf\n"` |
| G | `remove` is a no-op | fails: the following `commit` has nothing to commit (exit 1) |
| H | no-GSD layout also commits `docs/CLAUDE.md` | fails on 3 refs (nested name caught by the component scan) |
| I | an untracked `sub/.claude/settings.json` in `work` | fails on the disk scan (the ref scan cannot see it) |
| Orphan | `fn forgotten() -> Scenario` in `worktree.rs` with `scenarios! {}` | `clippy` rc 101, `function forgotten is never used` |
| Header and name scans | a scratch file without `//!`; a scratch header containing `Fixed` | reported and found, respectively |

Measured on the way: `git show-ref --verify --quiet` exits 1 for a missing ref on git 2.56.0. `has_ref` accepts only 0 and 1, and any other code panics.

## What these checks do not establish

- **Each layout's shape is proven once.** The oracle's behaviour on the no-GSD and fork layouts is proven by the scenarios in 02-10 to 02-14, not here.
- **The no-GSD check is a fixed list of six names, matched exactly per path component.** A near name (`.planning-old`, lower-case `claude.md`, `AGENTS.md`) is not looked for. The no-GSD layout itself adds only the files listed above, and its full tree is pinned exactly.
- **The fork test does not check the push remote's own HEAD.** `refs/remotes/origin/HEAD` is neither expected nor asserted in the fork layout. A remote added with `git remote add` and then pushed to gets none, but this is not pinned.
- **`git_tick` is documented for commands that create one commit.** A command creating several (a multi-commit rebase or cherry-pick) would give them all the same date. Nothing enforces "one commit".
- **`remove` on a directory is not exercised.** Only file removal is covered (control G).
- **A `with_local_config` pair that overrides a preset key** (for example `wsx.baseRef`) is applied after the preset keys, so it should win, but no test covers it.
- **The container gate was not run**, so host git 2.56.0 is the only git measured.

## Files Created/Modified

- `tests/harness/fixture.rs`: `Layout::NoGsd`, `ManifestKind::Modified`, the `RepoConfig` builders, a shared `build` with per-layout `write_devflow_files` / `write_no_gsd_files`, `Fixture::{git_tick, remove}` and the private `tick`, plus the four tests and their helpers
- `tests/harness/main.rs`: four support modules declared, macro list widened
- `tests/harness/scenario.rs`: registry test no longer requires a non-empty registry per module
- `tests/harness/{provenance,hermeticity,coverage,registry}.rs`: `//!` contract header only
- `tests/harness/{worktree,check,cut,guard,signing}.rs`: `//!` contract header and `scenarios! {}`

## Decisions Made

See `key-decisions` in the frontmatter.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The registry test rejected empty scenario modules**
- **Found during:** Task 3
- **Issue:** `scenario::registry_entries_match_their_scenarios` asserted `module {module} registers no scenario` for every module. With the five empty registrations Task 3 requires, it failed (`module worktree registers no scenario`). The plan requires both the empty registrations and that test passing.
- **Fix:** removed the per-module non-empty assertion and documented, in the test's doc comment, why it is redundant. The remaining checks (the entry's module and name match, `all_scenarios()` has the right count, `sync::self_test` is registered with its reason) are unchanged.
- **Files modified:** `tests/harness/scenario.rs` (outside the plan's `files_modified`)
- **Verification:** the orphan control above shows that clippy rejects a scenario function left out of `scenarios! {}`. That was the failure the assertion stood in for.
- **Committed in:** `fcd3b1d`

**2. [Scope - additive] Surface beyond the plan's list:** the private `Fixture::tick`, `build` (renamed from `build_devflow`), `write_devflow_files` and `write_no_gsd_files`. The test helpers `build_for_test`, `text`, `blob`, `has_ref`, `marked_paths_on_disk` and `NOT_IN_A_NO_GSD_REPO`. Each is used in this plan.

---

**Total deviations:** 1 auto-fixed (blocking) plus additive private surface.
**Impact on plan:** if 02-15 wants "no module stays empty" as a phase-end rule, it has to add that check itself. This plan removed the only place it was asserted.

## Issues Encountered

- My first fork control (B) failed for the wrong reason: with one remote, the remote-list assertion fired first. I replaced it with B2, which keeps both remotes and hits the integration-branch assertion.
- Byte-slice assertions printed failures as number arrays (seen in controls B and E). I switched them to strict UTF-8 text before committing.

## Known Stubs

None. The five scenario modules are empty on purpose, and 02-07 to 02-14 fill them. `.planning/WINDOWS.md` does not exist, so nothing was appended to the ledger.

## User Setup Required

None.

## Next Phase Readiness

- 02-04, 02-05, 02-06, 02-08, 02-09, 02-10 to 02-14 and 02-15 can each add tests only inside the module files they own; `main.rs` needs no further change.
- Scenarios can now state `RepoConfig::no_gsd()` and `RepoConfig::devflow().with_remotes("upstream", "origin")`.

## Self-Check: PASSED

- FOUND: all 12 touched files under `tests/harness/` (`[ -f ]`)
- FOUND: commits 38930d3, 3695d14, fcd3b1d (each an ancestor of HEAD), none with file deletions
- No `harness-*` directory left under `target/tmp`

---
*Phase: 02-oracle-harness*
*Completed: 2026-10-26*
