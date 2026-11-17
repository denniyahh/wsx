---
phase: 02
phase_name: "Oracle harness"
project: "wsx"
generated: "2026-11-17"
counts:
  decisions: 8
  lessons: 6
  patterns: 5
  surprises: 6
missing_artifacts:
  - "02-UAT.md"
---

# Phase 02 Learnings: Oracle harness

## Decisions

### Fixture steps and adapters return values, never panic
`FixtureFn = fn(&mut Fixture) -> Result<(), String>`, `run_case(side, &Entry, &Case, make_env)`, and adapters return `Result<ActionRun, AdapterError>` with `NotImplemented` or `Spawn`. An oracle-side `NotImplemented` still panics, naming the scenario and action.

**Rationale:** A failing fixture step or a spawn failure must reach `run_case`'s `Err` so a planted-host control can assert on it; `run_case` takes the registry entry because a `Case` carries no name for the TempDir prefix.
**Source:** 02-02-SUMMARY.md

### The oracle is vendored byte-identical; `oracle.patch` is its only adaptation
The patch is a pure diff of three path references plus the last line of `pre-push`, with no preamble. Substitutions match whole lines, because `scripts/sync-workspace.sh` occurs on four lines of `phase-worktree.sh` and only two are executed paths. A test checks each file's blob and mode against devflow `ee6ec792`.

**Rationale:** A scenario must never be made to pass by editing the oracle.
**Source:** 02-01-SUMMARY.md, 02-VERIFICATION.md

### Every case is classed by its first oracle run, as `Parity` or `fixed:`
Classes come from observation, not prediction. A `fixed:` case records the defect text and the oracle's observed exit code (for example 128 for a sync with no base remote), which the Rust side revises when its phase enables it.

**Rationale:** The oracle is the ground truth; the plan's predictions held in 02-07, 02-08, 02-11, 02-13 and 02-14, and differed once (the fork case in 02-10).
**Source:** 02-10-SUMMARY.md, 02-11-SUMMARY.md, 02-12-SUMMARY.md, 02-14-SUMMARY.md

### CommitExpect has no committer date
Only the author date is pinned to the action instant (orchestrator decision, 2026-10-26).

**Rationale:** HARN-03 lists commits as message, author, author date, committer name/email and tree; a replay or merge sets the committer date at run time, so it cannot be pinned the same way for oracle and Rust.
**Source:** 02-REVIEW-DISPOSITION.md (IN-04)

### The phase-end rule that every scenario module registers a scenario lives in `registry.rs`
Plan 02-03 removed the per-module non-empty assertion because it required five empty modules at that point; plan 02-15 restored it as `every_scenario_module_registers_a_scenario` over `crate::registries()`, with a control naming an empty module.

**Rationale:** The plan 02-03 summary warned that 02-15 would have to add the rule itself.
**Source:** 02-03-SUMMARY.md, 02-15-SUMMARY.md

### The gpg-signing control plants a recording `gpg.program`, not a missing key
The plant sets `gpg.program` to a script that appends `gpg:sign` to a marker and exits 1. `plant_host` takes a name and creates its own `harness-hermeticity-<name>-` ScenarioDirs. `HARNESS_KEYS`, the hermetic PATH and `MIN_GIT` are restated as literals in `hermeticity.rs`.

**Rationale:** A real gpg started a keyboxd daemon that outlived its TempDir; an independent literal pins the value rather than echoing it.
**Source:** 02-09-SUMMARY.md

### Rust-only cases claim an oracle side unless it is `OracleSide::NotImplemented`; control direction is judged by expected exit
A known-positive control must expect a non-zero exit and a clean control exit 0. A regression is a refusal: exit != 0, `RefsExpect::Unchanged` and no new commits. `validate` runs inside `try_run_with_env`, so every generated test validates before it runs.

**Rationale:** Matches 02-10/12/13/14, which set `NotImplemented` on every case of a rust-only scenario; validating in the runner covers both sides.
**Source:** 02-05-SUMMARY.md

### Pinned the cargo-llvm-cov version pattern with a `$` anchor
The release binary prints exactly `cargo-llvm-cov 0.9.1`, so the pattern keeps cargo-deny's `$` anchor, not nextest's `( |$)` tail. `rustup toolchain install` at the repo root installed llvm-tools from the toolchain file.

**Rationale:** RESEARCH A5 was settled by the scratch install.
**Source:** 02-06-SUMMARY.md

---

## Lessons

### `git apply` inside a nested repo silently skips and exits 0 without a ceiling
The patch is applied under `GIT_CEILING_DIRECTORIES`. A no-ceiling control runs `git apply -v` and requires `Skipped patch '<file>'.` for each file, so it fails if the apply ever stops being a silent skip.

**Context:** 02-04 provenance tests; exit 0 alone proved nothing.
**Source:** 02-04-SUMMARY.md

### Exit 0 alone does not prove a patched script worked
With a wrong `WSX_ORACLE_DIR`, `phase-worktree.sh` prints `not found; skipping sync`, creates the worktree and exits 0. The with-variable test also requires a stdout marker per script.

**Context:** `run_script` always sets `WSX_ORACLE_DIR`, so the without-variable test spawns `bash <copy>/<script>` itself.
**Source:** 02-04-SUMMARY.md

### Git rewrites a non-UTF-8 commit byte unless the commit encoding is declared
Under the default UTF-8 commit encoding, `0xFF` is stored as `0xC3 0xBF`. The byte-exactness test sets `i18n.commitEncoding=ISO-8859-1` and checks raw `git cat-file commit` output, which separates "git rewrote it" from "the observer rewrote it".

**Context:** First run of `names_are_observed_byte_exact`; the observer and comparator needed no change.
**Source:** 02-08-SUMMARY.md

### The ceiling at the scenario root did not protect git run in the root itself
The first review found `GIT_CEILING_DIRECTORIES` did not cover commands run in the scenario root; it now sits at the root's parent, and a control shows the old ceiling leaked.

**Context:** Review WR-02 (fixed 4c1e4c0).
**Source:** 02-REVIEW-DISPOSITION.md

### Exact-exit pins are not enough for refusals and observation
The review found the harness never observed HEAD, the index or in-progress operations (CR-01), and pinned refusals by exit code only (WR-01); both were fixed, with remotes, other ref namespaces and local config added (WR-03). Fixes were verified by reverting each and seeing an assertion failure.

**Context:** Three review iterations; iteration 3 found 0 critical and 0 warning.
**Source:** 02-REVIEW-DISPOSITION.md, 02-REVIEW-FIX.md

### A control that does not apply its mutation measures the unmodified test
The first comparator-only attempt at control E did not apply because rustfmt had reflowed the target lines, and its rc 0 was discarded; the retry checked that the mutation printed `mutated` first. Later plans added a runtime test (`runner_runs_each_control_and_names_the_failing_one`) because the control loop had no coverage.

**Context:** 02-02 control E; 02-05 found its control loop untested while writing the summary.
**Source:** 02-02-SUMMARY.md, 02-05-SUMMARY.md

---

## Patterns

### Plan-intent deviations recorded as Rule 1/2/3 auto-fixes with commit ids
Each summary lists deviations by rule (blocking, missing critical, missing verification, scope), with the finding, the fix and the commit, and states the impact on the plan.

**When to use:** Any plan where execution meets a gap the plan did not foresee.
**Source:** 02-02-SUMMARY.md, 02-04-SUMMARY.md, 02-11-SUMMARY.md

### Negative control plus positive control per check
Each new check ships with a control that must produce the opposite result, and the failing direction prints a real assertion. Examples: a widened env filter fails the hermetic test (control H); the `modules_without_scenarios` control; flipped-mode and empty-listing controls for the vendored mode check.

**When to use:** Every harness rule or guard.
**Source:** 02-02-SUMMARY.md, 02-15-SUMMARY.md

### Share fixture items between scenario modules instead of duplicating
`sync.rs` made `SEED_FILES`, `WORKSPACE_FILES`, `B_FILE`, `advance_develop`, `tree` and `date` `pub` for worktree and cut, and extracted `sync_merge` and `synced_files`; `DIRECTORY_DEFECT` is one constant shared by three cases.

**When to use:** When several scenarios need the same seed, steps or defect text, so it cannot drift.
**Source:** 02-07-SUMMARY.md, 02-12-SUMMARY.md

### Build pre-existing branches in the source worktree, then switch away
A pre-existing PR branch is built in the phase worktree with `checkout --no-track -b` and the worktree returned to its branch; fetch `origin/develop` in the fixture when the case needs `RefsExpect::Unchanged`.

**When to use:** Fixtures where git would otherwise refuse `branch -f` or checkout of a branch checked out elsewhere, or where the cut's own fetch would move a ref.
**Source:** 02-11-SUMMARY.md

### Fixtures that make competing hypotheses distinguishable
`fork_layout` advances `upstream/develop` with `feat: add d` so the expected PR parent tells it apart from local `develop` and the initial commit; the conflicting merge case adds a personal file in the resolution.

**When to use:** When two implementations would otherwise produce the same observable result.
**Source:** 02-14-SUMMARY.md

---

## Surprises

### `cargo fmt` never read the scenario modules
`main.rs` declares them inside `harness_modules!`, and rustfmt does not follow `mod` items in a macro; `sync.rs`, `worktree.rs` and `cut.rs` failed standalone rustfmt while `cargo fmt --check` passed. Fixed 2026-11-04: `check.sh fmt` also runs `rustfmt --edition 2024 --check` on every git-visible `.rs` file.

**Impact:** The gate was green on unformatted files until the check was added, proved by `gate_controls::fmt_checks_an_untracked_rs_file`.
**Source:** deferred-items.md, 02-10-SUMMARY.md

### A planted gpg left a keyboxd daemon running
The plant's `commit.gpgsign=true` made the fixture commit run the host's gpg under the scenario HOME; keyboxd (PID 3552803) was still running 229 s later after its TempDir was removed. It was stopped by PID, and `ps` showed no keyboxd or gpg-agent after the fix.

**Impact:** Led to the recording `gpg.program` plant.
**Source:** 02-09-SUMMARY.md

### `worktree::no_gsd_repo` oracle exits 2, not 1
RESEARCH listed exit 1 for a missing ROADMAP heading; the error path's second grep fails on the missing file and pipefail plus `set -e` pass its 2 through. Recorded as observed.

**Impact:** The expected exit is pinned from the oracle, not from research.
**Source:** 02-12-SUMMARY.md

### An oracle replay conflict stops `merge_resolution_adds_personal_file` before its merge
The case is `fixed:` only for the leftover `feature/topic-pr`, and says nothing about how devflow treats a merge's resolution changes. The orchestrator added `merge_resolution_only_personal_file` in 02-14, reading PRCUT-01 as changes against every parent; Phase 9 should confirm that reading.

**Impact:** Without the added scenario the oracle never reached its merge.
**Source:** 02-11-SUMMARY.md, 02-14-SUMMARY.md

### A rustup self-update and a rate limit interrupted execution
`rustup toolchain install` self-updated rustup from 1.29.1 to 1.29.2 (pinned toolchain unchanged), and an API rate limit paused 02-07 for about 29 minutes after the pre-push negative controls, with no work lost.

**Impact:** Host rustup changed as a side effect of 02-06; no repo effect from the pause.
**Source:** 02-06-SUMMARY.md, 02-07-SUMMARY.md

### cargo-llvm-cov 0.9.1 cannot report coverage of `tests/` files
Control B's coverage branch is proven to run under it by a mutation that flips its assertion and fails with the stray-profile message.

**Impact:** Coverage of the harness itself cannot be measured by line coverage; the control proves the branch runs.
**Source:** 02-06-SUMMARY.md
