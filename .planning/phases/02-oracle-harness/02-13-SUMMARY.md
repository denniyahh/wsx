---
phase: 02-oracle-harness
plan: 13
subsystem: testing
tags: [rust, nextest, oracle, harness, git, divergence-check]

requires:
  - phase: 02-oracle-harness (02-05)
    provides: Class, OracleSide, Role, CaseKind, validate and judge
  - phase: 02-oracle-harness (02-07)
    provides: check::self_test, its verdict/check_in_work helpers and controls, the recording rule
  - phase: 02-oracle-harness (02-08)
    provides: byte-exact comparator, raw-byte path names in fixtures
provides:
  - "check::quoted_personal_names (fixed: oracle exit 1), with Parity known-positive and clean controls"
  - "check::fork_layout (Parity on all three cases), checked against upstream/develop"
  - "check::no_gsd_repo (fixed: oracle exit 1), IF-02's negative control for the check, with Parity controls"
  - "check::no_gsd_custom_personal_path, Rust-only, with known-positive and clean controls"
affects: [02-15, phase-5]

actuals:
  tokens: 2200
  tasks: 2
  commits: 2
plan_head_before: be05d431797d0ab164483c43a853e162fbddc0c6
plan_head_after: 5bf63912566e9215c99cfbc4afbabbac5ee26af4

tech-stack:
  added: []
  patterns:
    - "A Rust-only guard scenario wraps every case in one helper that sets OracleSide::NotImplemented, so no case can claim an oracle side"

key-files:
  created: []
  modified:
    - tests/harness/check.rs

key-decisions:
  - "Every oracle case was classed by its first run as Parity: 7 Parity (fork_layout's three cases and both controls of the other two), 2 fixed: (quoted_personal_names and no_gsd_repo subjects, both exit 1 as the plan predicted)"
  - "The quoted-name files live in check.rs rather than reusing cut.rs's private QUOTED_PERSONAL, to keep the change inside the plan's one file; the bytes are the same three names"
  - "Defect texts name the exit code and the refused paths; the mechanism (C-quoted diff output, the missing .workspace-divergence) goes in the doc comments"

patterns-established:
  - "A divergence-check refusal is recorded as verdict(<exit>): refs, commits and files unchanged"

requirements-completed: [HARN-05, HARN-06]

coverage:
  - id: D1
    description: "Divergence check on personal files named with 0xC3 0xA9, a newline and 0xFF under .planning: wsx expects exit 0 (DIV-01); the oracle exits 1, recorded as fixed:; known-positive refused and clean passing, both Parity"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/check.rs#check::quoted_personal_names::oracle"
        status: pass
    human_judgment: false
  - id: D2
    description: "Divergence check in a fork layout against upstream/develop: subject passes, undeclared note refused, declared note passes, all Parity"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/check.rs#check::fork_layout::oracle"
        status: pass
    human_judgment: false
  - id: D3
    description: "Divergence check in a no-GSD repo: wsx expects exit 0 (DIV-01, CONF-02, IF-02); the oracle exits 1, recorded as fixed:; known-positive refused and devflow clean control passing, both Parity"
    requirement: HARN-06
    verification:
      - kind: integration
        ref: "tests/harness/check.rs#check::no_gsd_repo::oracle"
        status: pass
    human_judgment: false
  - id: D4
    description: "check::no_gsd_custom_personal_path registered Rust-only, generated as an ignored test with no oracle test, validated by the registry rule"
    requirement: HARN-05
    verification:
      - kind: integration
        ref: "tests/harness/registry.rs#registry::every_scenario_is_well_formed"
        status: pass
      - kind: other
        ref: "cargo nextest list --locked --run-ignored only -E 'test(=check::no_gsd_custom_personal_path::rust)'"
        status: pass
    human_judgment: false

duration: 4min
completed: 2026-10-26
status: complete
---

# Phase 2 Plan 13: Divergence-check HARN-06 cases Summary

**Four divergence-check scenarios were added, each with a refused known-positive and a passing clean control. The oracle refuses two cases that wsx is expected to pass, and both are recorded as `fixed:` with exit 1: personal files with quoted names under `.planning`, and a repo with no GSD files. The fork-layout check against `upstream/develop` matches the oracle (Parity). The custom personal path case is Rust-only.**

## Performance

- **Duration:** about 4 min (2026-10-26T19:32:02Z to 19:35:44Z)
- **Tasks:** 2 of 2
- **Files modified:** 1

## Accomplishments

- `check::quoted_personal_names`: the oracle's verdict on the quoted names is now recorded from a real run. Research had only predicted it from reading the code.
- `check::fork_layout`: the oracle checks against `upstream/develop`, and all three cases are Parity.
- `check::no_gsd_repo`: the oracle's refusal in a repo with no GSD files is recorded. This scenario is IF-02's negative control for the check.
- `check::no_gsd_custom_personal_path`: a Rust-only scenario. Its three fixtures were built and run once against the stub `wsx`.

## Classification of each case (first run as Parity)

| Scenario | Case | Rust expects (requirement) | Oracle observed | Class |
|---|---|---|---|---|
| `check::quoted_personal_names` | subject: `.planning/\xC3\xA9.md`, `.planning/a\nb.md`, `.planning/\xFF.md` added | exit 0, unchanged (DIV-01: `.planning` declared `only`) | **exit 1**, unchanged | `fixed:` |
| | known-positive: undeclared `notes/todo.md` | exit 1 | exit 1 | Parity |
| | clean: devflow preset | exit 0 | exit 0 | Parity |
| `check::fork_layout` | subject: fork preset, base `upstream/develop` | exit 0 (DIV-01, CONF-04) | exit 0 | Parity |
| | known-positive: undeclared `notes/todo.md` | exit 1 | exit 1 | Parity |
| | clean: `notes/todo.md` declared `only notes` | exit 0 | exit 0 | Parity |
| `check::no_gsd_repo` | subject: no-GSD preset (`notes/todo.md`, `.wsx.conf` with `only notes`) | exit 0, unchanged (DIV-01, CONF-02, IF-02) | **exit 1**, unchanged | `fixed:` |
| | known-positive: plus undeclared `scratch/y.md` | exit 1 | exit 1 | Parity |
| | clean: devflow preset | exit 0 | exit 0 | Parity |
| `check::no_gsd_custom_personal_path` | subject: `notes/x.md` under declared `notes` | exit 0 (DIV-01, CONF-02) | NotImplemented | Rust-only |
| | known-positive: undeclared `scratch/y.md` | exit 1 | NotImplemented | Rust-only |
| | clean: no-GSD preset | exit 0 | NotImplemented | Rust-only |

Every class and exit code matches the plan's prediction. Nothing differed.

### Recorded oracle outcomes

**`check::quoted_personal_names` subject.** Exit 1, with refs, commits and files unchanged. The only difference from the Rust expectation was the exit code. The stderr read:
```
check-workspace-divergence: this branch diverges from origin/develop in ways .workspace-divergence does not declare:
  undeclared workspace-only path: ".planning/a\nb.md" (declare: only ".planning/a\nb.md")
  undeclared workspace-only path: ".planning/\303\251.md" (declare: only ".planning/\303\251.md")
  undeclared workspace-only path: ".planning/\377.md" (declare: only ".planning/\377.md")
```
Defect text: "the oracle exits 1, refusing the personal files .planning/\xC3\xA9.md, .planning/a\nb.md and .planning/\xFF.md as undeclared".

**`check::no_gsd_repo` subject.** Exit 1, with refs, commits and files unchanged. The only difference was the exit code. The stderr read:
```
check-workspace-divergence: this branch diverges from origin/develop in ways .workspace-divergence does not declare:
  undeclared workspace-only path: .wsx.conf (declare: only .wsx.conf)
  undeclared workspace-only path: notes/todo.md (declare: only notes/todo.md)
```
Defect text: "the oracle exits 1, refusing .wsx.conf and notes/todo.md as undeclared".

**`check::no_gsd_repo` known-positive control.** Its stderr was read from a run with the expected exit perturbed to 3. It lists the same two paths plus `scratch/y.md`. The oracle would therefore refuse this control even without `scratch/y.md` (see the limits below).

**`check::fork_layout` known-positive control.** Its stderr was read from a run with the expected exit perturbed to 3. It reads `this branch diverges from upstream/develop`, so the oracle really compared against the base remote.

## Negative controls (all run, all failed as required, file restored byte-identical by sha256, none committed)

- **Recorded exits are compared.** Changing either `fixed:` record from `verdict(1)` to `verdict(2)` failed its subject on the exit code alone (expected 2, got 1).
- **Each control turns on the variable it names:**
  - Declaring `notes` in `quoted_personal_names`'s known-positive made it fail (expected 1, got 0).
  - Giving its clean control the undeclared note made it fail (expected 0, got 1).
  - In `fork_layout`, the undeclared note failed the subject and the clean control (expected 0, got 1 each).
  - Giving `no_gsd_repo`'s devflow clean control the undeclared note made it fail (expected 0, got 1).
- **Every control runs.** Each perturbation above failed on the control it targeted, so the runner reaches controls 1 and 2 after the subject.
- **Verify commands:**
  - Listing check: the plan's `cargo nextest list --run-ignored only` check finds `check::no_gsd_custom_personal_path::rust` (exit 0). The same check for `::nonexistent` exits 1.
  - Exit-4 check: selecting `check::no_gsd_custom_personal_path::oracle` exits 4 (`error: no tests to run`). The same `test $? -eq 4` wrapper around the existing `check::no_gsd_repo::oracle` exits 1.
- **The Rust-only fixtures build.** `check::no_gsd_custom_personal_path::rust` was force-run once with `cargo nextest run --run-ignored only`.
  - All three fixtures were built, with no `a fixture step failed`.
  - The stub `wsx` exits 2 (`usage: wsx --version`). On every case the only difference was the exit code (got 2), so the fixtures agree with the unchanged-state expectation.

## Task Commits

1. **Task 1:** `fad4694`, `test(02-13): record the divergence check on special names and in a fork layout`
2. **Task 2:** `5bf6391`, `test(02-13): record the divergence check in a no-GSD repo, add its custom-path case`

## Files Created/Modified

- `tests/harness/check.rs`: four scenarios, the fixture steps `add_quoted_personal`, `add_scratch` and `add_custom_note`, the `fork()` config, the defect constants `QUOTED_NAMES_DEFECT` and `NO_GSD_DEFECT`, and `rust_only_no_gsd_check`.

## Verification

- **Task 1:**
  - Both oracle tests (one command each, chained with `&&`) exited 0.
  - `registry::every_scenario_is_well_formed` exited 0.
  - `scripts/check.sh all` exited 0: 140 passed, 34 skipped. That is 2 run and 2 skipped more than after 02-12 (138/32).
- **Task 2:**
  - The `no_gsd_repo::oracle` test exited 0 with `1 test run: 1 passed`.
  - The listing check exited 0, and the exit-4 check succeeded.
  - The registry test followed by `scripts/check.sh all` exited 0: 141 passed, 36 skipped, ending `==> check.sh: all OK`.
- **Disabled-side report:** the full run lists `check::no_gsd_repo (Rust side not enabled until phase 5)` and `check::no_gsd_custom_personal_path (Rust side not enabled until phase 5)` under `Rust side not enabled (36):`.
- **Acceptance greps:** `PathNames`, `ForkLayout` and `NoGsd` in `check.rs` each exit 0. Each exited 1 at the pre-plan tip `be05d43`.
- **T-02-SC:** `Cargo.toml` and `Cargo.lock` are unchanged, and every cargo call used `--locked`.
- **Not run:** `scripts/check-in-container.sh`. These fixtures' behaviour in the pinned image is unverified.

## What these checks do not establish

- **Each scenario records the oracle on one fixture.** The Rust expectations are readings of DIV-01, CONF-02, CONF-04 and IF-02 that have never run against wsx. Phase 5 revises them when it enables the Rust sides.
- **`no_gsd_repo`'s known-positive does not show the oracle reacting to `scratch/y.md`.** On the oracle, that control is refused for the same reason as the subject: `.wsx.conf` and `notes/todo.md` are already undeclared to it. Its exit 1 matches the Rust expectation, so it is Parity by the classification rule, but on the oracle side it does not isolate the variable.
  - The control that does show the oracle can both pass and refuse in this scenario is the devflow clean control (exit 0). The undeclared-note perturbation of that clean control turned it into a refusal.
  - Phase 5's Rust side is where this known-positive becomes discriminating.
- **The `fixed:` records pin the exit code and the unchanged state, not the messages.** The refused paths quoted above come from one run's stderr each, and nothing asserts them. An oracle that exited 1 for another reason, leaving the same unchanged state, would still match the record.
- **The quoted-name case covers added files only,** and only under a built-in (`.planning`). It does not cover a quoted name under an `only` entry from `.workspace-divergence`, a modified or deleted quoted-name file, or a rename. The oracle's mechanism is the same C-quoting, but those shapes were not run.
- **The fork case covers a synced workspace only.** It does not show how the check behaves once `upstream/develop` has moved ahead of the workspace (Pitfall 6).
- **The Rust-only scenario has been checked against the stub `wsx` only.** That shows its fixtures are consistent with the unchanged-state expectation, not that a real `wsx check` reads a custom path.

## Decisions Made

- Classes were decided by the first Parity run, as listed in the table above.
- The quoted-name constant is a copy local to `check.rs` (see key-decisions). `cut.rs`'s `QUOTED_PERSONAL` is private and pairs each name with content, so reusing it would have meant editing a second file.

## Deviations from Plan

None. The plan was executed as written.

## Issues Encountered

None.

## Known Stubs

None.

## User Setup Required

None.

## Next Phase Readiness

- Phase 5 enables the check's Rust sides. Its negative controls are `quoted_personal_names` and `no_gsd_repo` (both `fixed:`). Its first custom-path scenario is `no_gsd_custom_personal_path`.
- 02-15 can count `check` among the modules that carry `PathNames`, `ForkLayout` and `NoGsd` cases.
- Cleanup: no `target/tmp/harness-*` tree remains. `rg '^harness-'` over `target/tmp` exits 1, while the same listing shows `cargo-resolution` and other entries, so it is not empty. The scratch directory `/tmp/claude-1000/-var-home-denniyahh-Github-wsx/97fae308-abb4-46c4-af94-89ae86614ec1/scratchpad/exec-02-13/` (run logs, file backups, the perturbation script) is removed after the final commit.

---
*Phase: 02-oracle-harness*
*Completed: 2026-10-26*

## Self-Check: PASSED

- FOUND: `tests/harness/check.rs`
- FOUND: commits `fad4694` and `5bf6391`. Both are ancestors of HEAD, and `check evaluation-scope --plan 02-13 --commits-only` resolves both.
