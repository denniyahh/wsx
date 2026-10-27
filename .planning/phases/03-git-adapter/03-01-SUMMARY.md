---
phase: 03-git-adapter
plan: 01
subsystem: git
tags: [gix, git, merge-base, cargo-deny, error-enum, tdd, tracer]

requires:
  - phase: 02-oracle-harness
    provides: hermetic env and fixture shapes (tests/harness/env.rs, fixture.rs) reproduced in src/git/testing.rs
provides:
  - gix =0.90.0 runtime dependency (sha1, sha256, revision; no default features) with its license policy
  - src/git module skeleton with the cfg_attr(not(test)) dead-code expectation
  - Error enum, final seven variants, every message pinned word for word
  - Repo::discover through the single gix open site, canonical worktree/git dir/common dir
  - Repo::resolve and Repo::merge_base (gix), with gix-vs-git differential tests
  - Git spawn helper (env snapshot, env_clear, explicit cwd, null stdin, GitFailed)
  - unit-test support (scratch root under target/tmp, hermetic env, criss-cross fixture)
affects: [03-02, 03-03, 03-04, 03-05, 03-06, 03-07, 03-09, 03-10, phase-5]

actuals:
  tokens: 9300      # chars/4 over the realized diff excluding Cargo.lock (37184 chars); Cargo.lock adds 1350 generated lines
  tasks: 2
  commits: 4
plan_head_before: a49656f3dd4524423b9520faa440831b79413dad
plan_head_after: 25148619c063bd1894ac226f0f9879c884fc6a68

tech-stack:
  added: [gix 0.90.0]
  patterns:
    - "Module-level #![cfg_attr(not(test), expect(dead_code, reason = ...))] for a module with no caller yet"
    - "Differential read test: gix answer vs git's through wsx's own Git helper, plus a control pair that moves the answer"
    - "Unit-test scratch root = current_exe() deps/../../tmp (no CARGO_TARGET_TMPDIR for bin unit tests)"

key-files:
  created:
    - src/git/mod.rs
    - src/git/repo.rs
    - src/git/reads.rs
    - src/git/spawn.rs
    - src/git/testing.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - deny.toml
    - src/main.rs

key-decisions:
  - "The merge-base control pairs main/feat~3 (F1) with main~2/feat (C1): on the plan's fixture git's single base for main/feat is F1, the same as main/feat~3, so the plan's comparison against main/feat could not catch a constant answer"
  - "The dead-code expectation's reason is 'no caller until phase 5' (phase 5 removal noted in a comment above it): rustfmt wraps any longer reason, which broke the one-line acceptance grep"
  - "gix-protocol and gix-transport stay in the graph: they are unconditional gix 0.90.0 dependencies; the no-network check asserts gix-transport has only its empty default feature and no client crate is present"

patterns-established:
  - "Pattern: Display arm text pinned whole with assert_eq!, never a substring"
  - "Pattern: Git::new(env) takes an explicit environment snapshot; tests pass scratch.env"

requirements-completed: [GIT-01, GIT-02]

coverage:
  - id: D1
    description: "Repo::discover from a subdirectory resolves the canonical worktree, git dir and common dir, and Repo::merge_base equals git merge-base (run through wsx's Git helper) on a criss-cross, clock-skewed history"
    requirement: "GIT-01"
    verification:
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::merge_base_matches_git"
        status: pass
    human_judgment: false
  - id: D2
    description: "Answer-changing control: gix and git both answer F1 for main/feat~3 and C1 for main~2/feat, and the two differ; a constant-F1 mutation fails it"
    requirement: "GIT-01"
    verification:
      - kind: unit
        ref: "src/git/reads.rs#git::reads::tests::merge_base_moves_with_the_pair"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every Error variant renders a message naming its cause, pinned word for word"
    requirement: "GIT-02"
    verification:
      - kind: unit
        ref: "src/git/mod.rs#git::tests::error_messages_name_their_cause"
        status: pass
    human_judgment: false
  - id: D4
    description: "gix pinned =0.90.0 passes cargo-deny with the four-license list; an MIT-only copy fails naming zlib-rs"
    verification:
      - kind: other
        ref: "scripts/check.sh deps"
        status: pass
      - kind: other
        ref: "cargo deny --manifest-path <scratch copy, allow = [\"MIT\"]> --locked check licenses (rc 4, names zlib-rs)"
        status: pass
    human_judgment: false

duration: 13min
completed: 2026-10-27
status: complete
---

# Phase 3 Plan 01: Git adapter tracer slice Summary

**wsx opens a repository from a subdirectory through its single gix entry point, and its merge-base equals git's on a criss-cross, clock-skewed history. gix 0.90.0 is pinned under a four-license cargo-deny policy, and the seven-variant `Error` enum is pinned word for word.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-10-27T01:33:46Z
- **Completed:** 2026-10-27T01:46:29Z
- **Tasks:** 2 (both TDD, RED then GREEN)
- **Files modified:** 9

## Accomplishments

- `Repo::discover` opens only through `gix::ThreadSafeRepository::discover_with_environment_overrides_opts` (`match_ceiling_dir_or_error: false`, a trust map with no bail) and canonicalises the worktree, git dir and common dir once.
- `Repo::merge_base` takes revspec bytes, resolves them through `Repo::resolve`, and returns wsx's `ObjectId`. No gix type leaves the module.
- `Git::run` is the one constructor of git children. It starts from an empty environment plus an explicit snapshot, sets the working directory, uses a null stdin and passes argv through `Command::args`. A failure is `Error::GitFailed { argv, status, stderr }`.
- `Error` has its final variant set: `NotARepo`, `ForeignOwner`, `Reftable`, `GitTooOld`, `GitFailed`, `Io` and `Gix`. The foreign-owner message carries git's own `git config --global --add safe.directory <path>` fix line.
- Test support (`src/git/testing.rs`): scratch trees under `<target>/tmp` (TempDir, removed on drop), a hermetic env with the ceiling at the scratch root's parent, and the criss-cross fixture, which checks itself (exactly two `--all` bases C1 and F1, and F3's committer time below F2's).

## Task Commits

1. **Task 1 RED: add failing merge-base parity test**: `7df9b9e` (test)
2. **Task 1 GREEN: open the repository through gix and answer merge-base**: `8b7a486` (feat)
3. **Task 2 RED: add failing error message test**: `5816d93` (test)
4. **Task 2 GREEN: name each refusal's cause in its message**: `2514861` (feat)

## TDD Gate Compliance

- Task 1 RED: `RED_EVIDENCE_OK` for `git::reads::tests::merge_base_matches_git` (record `target/gsd/03-01-red-evidence.json`, JUnit, exit 100). Semantic assessment: the target ran and failed on the planned gix-vs-git assertion (`left: None`, `right: Some("15e2f5f...")` at `src/git/reads.rs:75`). The fixture had built, and discovery and the three path assertions before it had passed, so the failure was not a build, fixture or discovery fault.
- Task 2 RED: `RED_EVIDENCE_OK` for `git::tests::error_messages_name_their_cause` (record `target/gsd/03-01-red-evidence-2.json`, exit 100). Semantic assessment: the test failed on the ForeignOwner assertion (`left: "ForeignOwner"`) after the NotARepo assertion passed, as the plan requires.
- Classifier negative control: a record of the GREEN run of the same target was rejected as `INVALID_RED` / `unexpected_green`.
- Each `test(03-01)` commit precedes its `feat(03-01)` commit. No refactor commit was needed.

## Files Created/Modified

- `src/git/mod.rs`: invariants doc, dead-code expectation, `ObjectId`, `Error` with `Display`, the error message test
- `src/git/repo.rs`: `Repo`, `discover`, `open_gix` (the only gix open site), `from_gix`, accessors, `resolve`
- `src/git/reads.rs`: `merge_base` and its two differential tests
- `src/git/spawn.rs`: `Git::{new, command, run}`
- `src/git/testing.rs`: `Env`, `T0`, `TICK`, `scratch_root`, `Scratch`, `hermetic_env`, `dated`, `command`, `git`, `git_ok`, `write`, `commit_all`, `criss_cross`
- `Cargo.toml`, `Cargo.lock`: the gix pin and its graph (the existing 14 lock entries are unchanged)
- `deny.toml`: `allow = ["Apache-2.0", "MIT", "Unicode-3.0", "Zlib"]`, with one comment line per new license naming the crates that need it
- `src/main.rs`: `mod git;`

## Decisions Made

See `key-decisions` in the frontmatter. Everything else follows the plan's contract: names, signatures, file layout and the fixture recipe.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Test bug] The merge-base control compared against a pair with the same answer**
- **Found during:** Task 1 GREEN
- **Issue:** The plan's control asserted that `merge_base(main, feat~3)` differs from `merge_base(main, feat)`. On the plan's own fixture, git's single `merge-base main feat` is F1 (the newer of the two criss-cross bases, `15e2f5f...`), the same as `main`/`feat~3`. gix agreed with git on every pair, so the parity holds. But the control as written fails on a correct implementation, and would pass a constant-F1 implementation if inverted.
- **Fix:** The plan's `main`/`feat~3` assertions are kept (gix = git = F1 = `rev-parse feat~3`). The `assert_ne` now compares against `main~2`/`feat`, whose base is C1, and requires gix = git = `rev-parse main~2` there too.
- **Verification:** A mutation that makes `merge_base` always answer F1 still passes `merge_base_matches_git`, but fails the new control on the `main~2 feat` assertion. The mutation was reverted, and its absence was checked.
- **Files modified:** `src/git/reads.rs`
- **Commit:** `8b7a486`
- **Plan truth affected:** must_have 2 ("that answer differs from the main/feat answer") is false for this fixture. It holds for `main~2`/`feat` instead. No later phase-3 plan relies on git picking C1 for `main`/`feat` (checked by searching 03-02 to 03-10).

**2. [Rule 1 - Plan check bug] The "no gix-protocol / gix-transport" tree check cannot pass with gix 0.90.0**
- **Found during:** Task 1 (first `cargo check`)
- **Issue:** gix 0.90.0 depends on `gix-protocol` unconditionally, and `gix-protocol` depends on `gix-transport`, so both are in the normal tree with exactly the planned features. The plan's verify (`! rg -q '^(signal-hook|gix-transport|gix-protocol|reqwest|curl) '`) exits 1. The graph is the one research measured: 99 crates excluding wsx.
- **Fix:** The dependency was not changed. Its intent, no network client, was checked instead: `gix v0.90.0` is present; no `signal-hook`, `signal-hook-registry`, `reqwest`, `curl`, `curl-sys`, `gix-credentials`, `async-trait` or `futures-io` crate is present; and `cargo tree -e normal,features --prefix none -i gix-transport` shows only `gix-transport feature "default"` (empty in gix-transport 0.62.0), with no `blocking-client`, `async-client` or `http-client*` feature.
- **Verification:** The corrected check passes on the tree. Negative control: a scratch copy with gix's `blocking-network-client` feature fails it, naming `gix-credentials`. That copy was resolved with its own fresh lockfile and then removed.
- **Plan truth affected:** must_have 3 ("no ... gix-transport, gix-protocol ... crate") is false as worded. Its no-network and no-signal intent holds.

**3. [Rule 1 - Format] Dead-code expectation reason shortened**
- **Found during:** Task 1 acceptance check
- **Issue:** rustfmt wraps `#![cfg_attr(not(test), expect(dead_code, reason = "<plan's reason>"))]` over several lines, so the acceptance grep `#![cfg_attr(not(test), expect(dead_code` failed.
- **Fix:** The reason is now `"no caller until phase 5"`. A comment above the attribute says phase 5's `wsx check` removes it.
- **Verification:** The grep passes. Negative control: with the attribute deleted, clippy exits 101 with 8 dead-code errors. With it in place, clippy exits 0.
- **Commit:** `8b7a486`

**4. [Rule 3 - Blocking] Host cargo-nextest drifted from the pin**
- **Found during:** Task 1 RED run
- **Issue:** `scripts/check.sh test` refused: `cargo nextest` resolves to linuxbrew's 0.9.150, and the pin is 0.9.148. cargo-deny likewise resolved to linuxbrew first.
- **Fix:** The repo's own installer (sha256-checked) put the pinned cargo-nextest 0.9.148 and cargo-deny 0.20.2 into `target/gsd-tools` (gitignored), and that directory was prepended to PATH for gate commands only. No host config, PATH file or pin changed.
- **Verification:** `scripts/install-tools.sh verify-cargo cargo-nextest cargo-deny` reports both pins OK with that PATH.

**Total deviations:** 4 auto-fixed (2 plan-check bugs, 1 formatting, 1 blocking environment). **Impact:** the code contract is as planned, and two must_have truths are reworded as described above. No architectural change. gix's merge-base matched git, so the plan's stop condition (step 11) did not trigger.

## Verification Results (limits stated)

- `scripts/check.sh all`: OK (190 tests run, 190 passed, 41 skipped). `scripts/check.sh deps`: `advisories ok, bans ok, licenses ok, sources ok`, with one warning for duplicate `hashbrown`, which `multiple-versions = "warn"` allows.
- Named tests, each run on its own with `--no-tests=fail`: `merge_base_matches_git`, `merge_base_moves_with_the_pair` and `error_messages_name_their_cause` each passed (`1 test run: 1 passed`).
- MIT-only license control: rc 4, names `zlib-rs` (and `dunce`, `foldhash`, `unicode-bom`, `unicode-ident`).
- No `git-unit-*` directory remains under `target/tmp`. Frozen files (`bin/wsx`, `tests/sandbox.sh`, `scripts/workspace`) are untouched.
- **What this does not establish:** the parity proof covers one read on one fixture. gix choosing git's merge-base on this criss-cross shows that it can match git, not that it always does. No refusal check exists yet: until 03-06, 03-09 and 03-10 land, `discover` opens reftable, foreign-owned and ceiling-hidden repositories. Nothing in `main` calls it, so no user is exposed. The discovery tests run gix in-process with the developer's process environment. They assert the resolved paths, so a leaked `GIT_DIR` would fail them loudly, but they do not prove hermeticity of the gix side. RESEARCH assumption A2 (the `deps/../../tmp` layout under llvm-cov and `--target`) was not exercised: `scripts/check.sh coverage` was not run.

## Known Stubs

None. Backlog 999.4 (the nested nextest run in `tests/gate_controls.rs`) did not affect the RED evidence, because both RED runs were filtered to one unit test.

## Threat Flags

None. The new surface (the gix open site, git child spawns, test fixtures under `target/tmp`) is what T-03-01 to T-03-05 in the plan's threat model cover. T-03-02 (argv through `Command::args`) and T-03-03 (`Stdio::null()`) are implemented in `Git::command`.

## Cleanup

My scratchpad files and the scratch copies for the deny and tree controls are removed. I kept `target/gsd/03-01-red-evidence.json` and `-2.json` (the plan requires them) and `target/gsd-tools/` (the pinned nextest and deny, which later plans in this worktree need while the host's linuxbrew tools are off-pin). Both are gitignored. `target/tmp` holds fixed-name directories from the existing integration suite, not from this plan. No background process is running.

## Next Phase Readiness

Ready for 03-02. The `Error` interface is final, so 03-05, 03-06 and 03-09 construct `GitTooOld`, `Reftable` and `ForeignOwner` without editing `mod.rs`. The gate on this host needs `PATH="$PWD/target/gsd-tools:$PATH"` until the nextest pin and the host tool are reconciled.

## Self-Check: PASSED

- FOUND: src/git/mod.rs, src/git/repo.rs, src/git/reads.rs, src/git/spawn.rs, src/git/testing.rs, target/gsd/03-01-red-evidence.json, target/gsd/03-01-red-evidence-2.json
- FOUND commits on HEAD: 7df9b9e, 8b7a486, 5816d93, 2514861; `gsd_run check evaluation-scope --plan 03-01 --commits-only` resolved 4 plan commits
