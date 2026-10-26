---
phase: 02-oracle-harness
plan: 10
subsystem: testing
tags: [rust, nextest, scenario-harness, oracle, pre-push, guard]

requires:
  - phase: 02-oracle-harness
    provides: "02-05 scenario rules and runner, 02-07 PrePush oracle mapping and guard::self_test, 02-08 byte-exact observer"
provides:
  - "Ten guard scenarios recording devflow's pre-push hook against HARN-06 cases and the reproduced bypasses P1-P5"
  - "Shared known-positive (feature/leak) and clean (feature/clean) control builders in guard.rs"
  - "Rust expectations for GUARD-01, GUARD-02, CONF-01, CONF-02 and IF-02, ready for phase 6 to enable"
affects: [phase-6-guard, 02-15]

actuals:
  tokens: 6238
  tasks: 3
  commits: 3
plan_head_before: 61407ecae4645ad284aa96fd1acba9934dee72f4
plan_head_after: 6abca82e12f4a41bf24b63d4ce348dc0f1cfbe99

tech-stack:
  added: []
  patterns:
    - "Guard scenario = subject + shared known-positive + shared clean control (with_controls)"
    - "fixed(case, expected_exit, oracle_exit, DEFECT) for a guard verdict with the repo untouched"
    - "Non-capturing closures as FixtureFn for parameterised fixture steps"

key-files:
  created:
    - .planning/phases/02-oracle-harness/deferred-items.md
    - .planning/WINDOWS.md
  modified:
    - tests/harness/guard.rs

key-decisions:
  - "Every case was first run as Parity; each class follows from that run. All observed outcomes match RESEARCH P1-P5; the fork case differed from research's code-reading prediction (see below)."
  - "fork_layout keeps the plan's own controls; its clean control is recorded refused on the oracle, so on the oracle side the case that shows the hook passing in that scenario is the subject."
  - "guard.rs is formatted with rustfmt directly, because cargo fmt does not reach modules declared inside harness_modules!; the other affected modules are logged, not reformatted."

patterns-established:
  - "Defect texts as named constants beside the scenarios that cite them"

requirements-completed: []

coverage:
  - id: D1
    description: "Quoted-name bypass recorded for non-ASCII, newline and non-UTF-8 personal names (HARN-06 path names, GUARD-02)"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/guard.rs#guard::quoted_non_ascii_name::oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/guard.rs#guard::quoted_newline_name::oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/guard.rs#guard::quoted_non_utf8_name::oracle"
        status: pass
    human_judgment: false
  - id: D2
    description: "Range, ref-pattern, merge-resolution and name-prefix defects recorded (GUARD-01, CONF-01, HARN-06 merges)"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/guard.rs#guard::unlisted_branch_pattern::oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/guard.rs#guard::new_branch_without_own_commits::oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/guard.rs#guard::merge_resolution_adds_personal_file::oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/guard.rs#guard::name_prefix_is_not_personal::oracle"
        status: pass
    human_judgment: false
  - id: D3
    description: "Fork-layout and no-GSD guard behaviour recorded from real oracle runs; no-GSD custom personal path registered Rust-only and listed by name"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/guard.rs#guard::fork_layout::oracle"
        status: pass
      - kind: integration
        ref: "tests/harness/guard.rs#guard::no_gsd_repo::oracle"
        status: pass
      - kind: other
        ref: "cargo nextest list --locked --run-ignored only -E 'test(=guard::no_gsd_custom_personal_path::rust)'"
        status: pass
      - kind: other
        ref: "scripts/check.sh test -E 'test(=guard::no_gsd_custom_personal_path::oracle)' exits 4"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every guard scenario carries a known-positive and a clean control (HARN-05), validated across the registry"
    requirement: HARN-05
    verification:
      - kind: unit
        ref: "tests/harness/registry.rs#registry::every_scenario_is_well_formed"
        status: pass
    human_judgment: false

duration: 9min
completed: 2026-10-26
status: complete
---

# Phase 2 Plan 10: Guard HARN-06 and Bypass Scenarios Summary

**Ten pre-push guard scenarios record devflow's hook allowing quoted personal names, unlisted refs, branches without own commits and personal files added in a merge resolution, and refusing `.planningX`, each beside a known-positive the hook refuses and a clean push it passes in the same run.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-10-26T18:51:08Z
- **Completed:** 2026-10-26T19:00:06Z
- **Tasks:** 3
- **Files modified:** 1 source file (`tests/harness/guard.rs`), plus 2 planning files

## Accomplishments

- Shared controls in `guard.rs`: `known_positive()` pushes `feature/leak` (adds `.planning/x.md`, exit 1) and `clean_control()` pushes `feature/clean` (adds `src/clean.txt`, exit 0). Both are made from `develop` and pushed as new refs from `work/.worktrees/develop`. `with_controls` attaches them; `guard::self_test` now uses `known_positive()`.
- Nine oracle-recorded scenarios and one Rust-only scenario, all registered with `rust = "Rust side not enabled until phase 6"`.

## Recorded outcomes (oracle exit next to Rust expected exit)

Every case was first written as Parity with its Rust expectation and run on the oracle. A case whose run matched is Parity. A case whose run differed is `fixed:`, with the observed outcome as its record. In every differing case the exit code was the only difference: refs, commits and files were unchanged.

| Scenario | Case | Rust expects (requirement) | Oracle recorded | Class |
|---|---|---|---|---|
| `quoted_non_ascii_name` | subject `.planning/é.md` (0xC3 0xA9) | exit 1 (GUARD-02) | exit 0 | `fixed:` |
| `quoted_newline_name` | subject `.planning/a\nb.md` | exit 1 (GUARD-02) | exit 0 | `fixed:` |
| `quoted_non_utf8_name` | subject `.planning/\xFF.md` | exit 1 (GUARD-02) | exit 0 | `fixed:` |
| `unlisted_branch_pattern` | `.planning/y.md` to `refs/heads/wip/y` | exit 1 (GUARD-01) | exit 0 | `fixed:` |
| `new_branch_without_own_commits` | new `feature/noown` at the `workspace/dev` tip | exit 1 (GUARD-01 a) | exit 0 | `fixed:` |
| `merge_resolution_adds_personal_file` | merge resolution adds `.planning/STATE.md` | exit 1 (GUARD-01 a, b) | exit 0 | `fixed:` |
| `name_prefix_is_not_personal` | `.planningX/z.md` | exit 0 (CONF-01) | exit 1, `REFUSING ... .planningX/z.md` | `fixed:` |
| `fork_layout` | subject `feature/pr` (`src/pr.txt`) to `origin` | exit 0 (GUARD-01) | exit 0 | Parity |
| `fork_layout` | known-positive `feature/leak` to `origin` | exit 1 | exit 1, `REFUSING ... .planning/x.md` | Parity |
| `fork_layout` | clean `workspace/dev` from `work` | exit 0 (GUARD-01 skips the workspace branch) | exit 1, `check-workspace-divergence: base ref 'origin/develop' not found.` | `fixed:` |
| `no_gsd_repo` | subject `workspace/dev` from `work` | exit 0 (GUARD-01, IF-02) | exit 0 | Parity |
| `no_gsd_repo` | counterpart `feature/leak` (`.planning/x.md`) | exit 0 (CONF-02, undeclared) | exit 1, `REFUSING ... feature/leak` | `fixed:` |
| all nine above | shared known-positive / clean | exit 1 / exit 0 | exit 1 / exit 0 | Parity |
| `no_gsd_custom_personal_path` | `notes/x.md` / `.wsx.conf` / `src/clean.txt` | exit 1 / 1 / 0 (GUARD-01, CONF-02, IF-02) | NotImplemented (hook hard-codes its paths) | Rust-only |

**Against RESEARCH's predictions:** P1-P5 reproduced exactly as research recorded them. Research did not execute the fork or no-GSD cases. What the oracle actually did there:

- **Fork:** research predicted, from reading the code, that `--remotes=origin` would over-scan when pushing to a remote without `origin/*` refs. That did not happen in this fixture. `origin` is the push remote, and its `workspace/dev` already contains `develop`'s commit, so the range was bounded correctly and the subject passed. What failed was the hook's tail: in `work`, which carries `.workspace-divergence`, it runs the divergence check against a hard-coded `origin/develop`, which a fork lacks.
- **No-GSD:** the oracle matched the plan's code-reading prediction. The subject passed (Parity) because `work` has no `.workspace-divergence`, so the tail is skipped. The counterpart was refused by the hook's fixed list.

## Negative controls (all run, all reverted byte-identical, none committed)

- **Quoted-name mechanism:** the three subjects run with `core.quotePath=false` in the fixture's local config. The non-ASCII and non-UTF-8 subjects were then refused by the hook, so the `fixed:` case failed with `FixedPassesOnOracle`. The newline subject was still allowed, because git always quotes a newline. This confirms the defect text: the quoting is what hides the name.
- **Merge fixture shape:** a probe fixture step printed the merge commit's details: two parents, `.planning/STATE.md` in the tip tree, `git log --name-only` printing only the subject line and no file, and the first-parent diff listing `.planning/STATE.md` and `src/a.txt`. The fixture step also fails unless the merge conflicts (exit 1).
- **Fork known-positive is a policy refusal:** a counterpart copy of the fork leak push, expecting exit 2, printed `pre-push: REFUSING to push personal/agent artifacts to 'refs/heads/feature/leak': .planning/x.md`, and not a "could not inspect the commit range" failure. The fork subject's stderr was empty.
- **Scenario rule on the guard:** removing the fork scenario's known-positive role made the registry reject the scenario before anything ran.
- **Rust-only registration:** registering `no_gsd_custom_personal_path` as `oracle` made `registry::every_scenario_is_well_formed` fail, with one rejection for each of the three cases.
- **Ignored-test listing:** the same listing command for `::nonexistent` exits 1. The `::oracle` exit-4 check exits non-4 for `no_gsd_repo::oracle`, which exists.
- **Rust-only fixtures build:** running the ignored `guard::no_gsd_custom_personal_path::rust` once built all three fixtures. Each case failed only because the binary is a stub (`usage: wsx --version`, exit 2).

## Task Commits

1. **Task 1: Quoted non-ASCII, newline and non-UTF-8 personal names:** `7c44da6` (test)
2. **Task 2: Range, ref-pattern, merge-resolution and name-prefix defects:** `4ec4e2b` (test)
3. **Task 3: Fork layout, no-GSD repo, Rust-only custom personal path:** `6abca82` (test)

## Files Created/Modified

- `tests/harness/guard.rs`: the shared control builders, fixture helpers (`develop_checkout`, `branch_from_develop`, `push`, `fixed`, `rust_only`), defect constants, and ten new scenarios.
- `.planning/phases/02-oracle-harness/deferred-items.md`: the rustfmt gap (below).
- `.planning/WINDOWS.md`: created by the ledger append for the same gap.

## Verification

- Every task's `<verify>` chain exited 0. Each oracle test was run as its own `--no-tests=fail -E 'test(=...)'` command, the commands chained with `&&`.
- `scripts/check.sh all` exited 0 after each commit, ending `==> check.sh: all OK`. Final run: 125 passed, 18 skipped. The disabled-side report lists `guard::no_gsd_custom_personal_path (Rust side not enabled until phase 6)` and `guard::no_gsd_repo (Rust side not enabled until phase 6)`.
- Acceptance greps: `PathNames`, `Merges`, `ForkLayout` and `NoGsd` are each found in `guard.rs` (exit 0), and each is absent at the pre-plan tip (exit 1).
- `Cargo.toml` and `Cargo.lock` are unchanged (T-02-SC). The plan's commits touch only `tests/harness/guard.rs`.
- Not run: `scripts/check-in-container.sh`. This plan added no new tool or script path, but the container image's behaviour on these fixtures is unverified.

## What these checks do not establish

- Each scenario records the oracle's behaviour on **one fixture**. For example, the fork result says nothing about a fork whose push remote holds no workspace branch. In that layout `--remotes=origin` may still over-scan, as research predicted, and that case is untested.
- The Rust expectations are this plan's reading of GUARD-01, GUARD-02, CONF-01, CONF-02 and IF-02. They have never run against wsx. Phase 6 revises them in the commit that enables them.
- The `fixed:` records pin the exit code with the repo untouched. They do not pin the hook's message. The quoted-name and merge mechanisms are evidenced by the negative controls above, not by the recorded outcome.
- On the oracle side, `fork_layout`'s clean control is refused. In that scenario, the evidence that the hook can pass a push is the subject (Parity, exit 0), not the clean control.
- `no_gsd_custom_personal_path`'s Rust expectations are unexercised beyond fixture construction.

## Decisions Made

- Classes come from the first Parity run, never from the plan's prediction. Every class matched the plan's prediction except where the plan left it open (the fork case).
- `fork_layout` keeps exactly the plan's three cases. I did not add a second clean control: it would duplicate the subject, which already passes on the oracle.
- The `no_gsd_repo` counterpart is a `Role::Counterpart` control, which runs in the no-GSD layout. The shared controls run in devflow's layout.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The conflicting merge cannot run through `git_tick`**
- **Found during:** Task 2 (`merge_resolution_adds_personal_file`)
- **Issue:** The plan says "merges `origin/develop` into `feature/merge` with `git_tick`". `git_tick` requires exit 0, and a conflicting merge exits 1.
- **Fix:** The fixture runs `git merge --no-commit origin/develop` through `Fixture::git` and requires exit 1 (a conflict). It then resolves the conflict, adds `.planning/STATE.md`, and creates the merge commit with `git_tick(["commit", "--quiet", "--no-edit"])`, so the commit is still dated on the fixture clock.
- **Verification:** the merge-shape probe above.
- **Committed in:** `4ec4e2b`

**2. [Rule 3 - Blocking] `cargo fmt` does not format or check `guard.rs`**
- **Found during:** Task 2
- **Issue:** rustfmt left over-100-column lines in place, and `cargo fmt --check` still passed. `main.rs` declares the scenario modules inside `harness_modules!`, and rustfmt does not follow `mod` items inside a macro.
- **Fix:** `guard.rs` is formatted with `rustfmt --edition 2024` directly. Long defect texts became named constants with `\` continuations. Standalone rustfmt agrees with `cargo fmt` on every file `cargo fmt` reaches, so the result is the project's own style. The other affected modules (`sync.rs`, `worktree.rs`, `cut.rs`) belong to other plans and were not touched. The gap is logged in `deferred-items.md` and the windows ledger.
- **Committed in:** `4ec4e2b`. That commit also reformats the helpers from Task 1.

---

**Total deviations:** 2 auto-fixed (2 blocking).
**Impact on plan:** None on scope. Both deviations are mechanical.

## Issues Encountered

- A first probe that removed the fork known-positive's role was rejected by the scenario rules, as designed. It was repeated with an added counterpart copy instead.

## Deferred Issues

- **`check.sh fmt` gives a false green on the scenario modules** (`deferred-items.md`, windows ledger entry 1, kind `lint-warning`). This is pre-existing and was not caused by this plan. While this entry stays open, `/gsd-ship` blocks.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 6 has ten guard scenarios to enable, all listed in the disabled-side report.
- Ready for 02-11.

## Self-Check: PASSED

- `tests/harness/guard.rs` and `deferred-items.md` exist.
- `7c44da6`, `4ec4e2b` and `6abca82` are ancestors of HEAD. `check evaluation-scope --plan 02-10 --commits-only` resolves all three.
- HARN-05 and HARN-06 are not marked complete: `requirements.ready-ids` reports both as blocked by sibling plans that also declare them.

---
*Phase: 02-oracle-harness*
*Completed: 2026-10-26*
