---
phase: 01-development-foundation
plan: 01
subsystem: infra
tags: [rust, cargo, nextest, clippy, rustfmt, rust-toolchain, bash, check.sh]

requires: []
provides:
  - "Binary crate `wsx` 0.0.0 (edition 2024, MIT, publish = false, zero dependencies) with committed Cargo.lock"
  - "`wsx --version` prints `wsx 0.0.0` (exit 0); any other input prints `usage: wsx --version` on stderr (exit 2)"
  - "tests/cli.rs: two public-boundary tests spawning CARGO_BIN_EXE_wsx"
  - "scripts/check.sh targets all (default), fmt, clippy, test [nextest args]; require_tool, show_toolchain"
  - "rust-toolchain.toml pinning rustc 1.99.1 (clippy, rustfmt, profile minimal)"
  - "clippy.toml disallowing std::env::set_var and std::env::remove_var"
  - ".config/nextest.toml writing target/nextest/default/junit.xml on every run"
affects: [01-02 deps target and install-tools.sh, 01-03 container runner, 01-04 CI jobs, 01-06 CONTRIBUTING/CLAUDE docs, phase 2 oracle harness, phase 5 CLI contract]

actuals:
  tokens: 2922
  tasks: 2
  commits: 3
plan_head_before: 69a0bc365812dc9b526398b6fb2818398d9c58cf
plan_head_after: d3b1721b8d2767e928fcb2ec62a32d0516fdb129

tech-stack:
  added: [rustc 1.99.1 via rust-toolchain.toml, cargo-nextest 0.9.148 (host, brew)]
  patterns:
    - "scripts/check.sh is the single gate entry point; every target names a missing tool and exits 1"
    - "--no-tests=fail is fixed on the nextest command line before caller args; caller --no-tests is refused (exit 2)"
    - "CLI tests spawn the built binary through env!(\"CARGO_BIN_EXE_wsx\") and assert stdout, stderr and exit code"
    - "Rust block sits at the top of .gitignore so PR-cut cherry-picks onto develop do not conflict"

key-files:
  created:
    - Cargo.toml
    - Cargo.lock
    - .config/nextest.toml
    - src/main.rs
    - tests/cli.rs
    - scripts/check.sh
    - rust-toolchain.toml
    - clippy.toml
  modified:
    - .gitignore

key-decisions:
  - "RED evidence record kept at target/gsd/01-01-red-evidence.json (inside the project, gitignored, not committed); verdict and fields are quoted in this SUMMARY"
  - "show_toolchain captures rustc --version through a plain assignment, not echo \"$(...)\", so a failing rustc stops the gate under set -e"
  - "check.sh refuses --no-tests in any caller argument position, including after `--`; stricter than needed, never weaker"

patterns-established:
  - "Discriminating controls: each failing control is paired with a counterpart that must pass (C1 full PATH, C2 bare nextest, C3 without clippy.toml)"
  - "Commit gating: `scripts/check.sh all && git commit` for every non-RED commit; RED commit gated on RED_EVIDENCE_OK"

requirements-completed: [DEV-01, DEV-02, HARN-04]

coverage:
  - id: D1
    description: "wsx --version prints `wsx 0.0.0` and exits 0; no argument exits 2 with `usage: wsx --version` on stderr"
    requirement: DEV-01
    verification:
      - kind: integration
        ref: "tests/cli.rs#version_flag_prints_package_version"
        status: pass
      - kind: integration
        ref: "tests/cli.rs#missing_argument_exits_with_usage"
        status: pass
      - kind: other
        ref: "test \"$(cargo run --quiet --locked -- --version)\" = \"wsx 0.0.0\""
        status: pass
    human_judgment: false
  - id: D2
    description: "scripts/check.sh all runs fmt, clippy, test in order and ends `==> check.sh: all OK`; no-arg and empty-arg run the same; unknown target or extra arg exits 2; caller --no-tests exits 2"
    requirement: DEV-01
    verification:
      - kind: other
        ref: "scripts/check.sh all"
        status: pass
      - kind: other
        ref: "scripts/check.sh test --no-tests=pass -E 'test(=no_such_test)' (exit 2)"
        status: pass
      - kind: other
        ref: "scripts/check.sh bogus; scripts/check.sh fmt extra (both exit 2)"
        status: pass
    human_judgment: false
  - id: D3
    description: "No-match guard holds against NEXTEST_NO_TESTS=pass (control C2)"
    requirement: DEV-01
    verification:
      - kind: other
        ref: "NEXTEST_NO_TESTS=pass scripts/check.sh test -E 'test(=no_such_test)' (exit 4) vs bare cargo nextest (exit 0)"
        status: pass
    human_judgment: false
  - id: D4
    description: "A missing tool is named, not skipped (control C1)"
    requirement: DEV-01
    verification:
      - kind: other
        ref: "env PATH=$HOME/.cargo/bin:/usr/bin:/bin scripts/check.sh test (exit 1, names cargo-nextest); env PATH=/usr/bin:/bin scripts/check.sh fmt (exit 1, names cargo)"
        status: pass
    human_judgment: false
  - id: D5
    description: "rustc 1.99.1 resolved through rust-toolchain.toml and printed once per check.sh run"
    requirement: DEV-02
    verification:
      - kind: other
        ref: "rustup show active-toolchain (overridden by rust-toolchain.toml); scripts/check.sh all prints `==> rustc 1.99.1` exactly once"
        status: pass
    human_judgment: false
  - id: D6
    description: "Planted unsafe std::env::set_var in a #[cfg(test)] module fails clippy for the disallowed-method lint and passes without clippy.toml (control C3)"
    requirement: HARN-04
    verification:
      - kind: other
        ref: "C3 scratch copies: check.sh all exit 101 with disallowed-method message and no nextest line; counterpart check.sh clippy exit 0"
        status: pass
    human_judgment: false

duration: 4min
completed: 2026-10-22
status: complete
---

# Phase 1 Plan 01: Host gate tracer (wsx binary, check.sh, toolchain pin, env-mutation ban) Summary

**A `wsx` binary that reports `wsx 0.0.0`, two CLI tests that spawn it, and `scripts/check.sh` running fmt, clippy (with the set_var/remove_var ban) and nextest (with a no-match guard nothing can override) on pinned rustc 1.99.1, each silent-pass route closed by a discriminating control.**

## Performance

- **Duration:** about 4 min of execution (19:37:20Z to 19:41:01Z; excludes reading the plan and references)
- **Started:** 2026-10-22T19:37:20Z
- **Completed:** 2026-10-22T19:41:01Z
- **Tasks:** 2 of 2
- **Files modified:** 9 (8 created, 1 modified)

## What these results do and do not establish

- Every control below ran on **this host only**. Container parity is 01-03; CI is 01-04/01-07. Nothing here shows the gate behaves the same inside the image.
- The two CLI tests prove the two cases they exercise (`--version`, no argument). They do not cover the CLI contract Phase 5 defines (IF-01). `--version extra` and `--help` both exit 2 with usage (checked by hand at HEAD), but no test asserts them.
- Each control ran once. That shows each guard can fire. It does not show the guards are reliable across environments.
- The estimate was 48000 tokens; the realized diff is about 2922 (chars/4). The estimate was roughly 16 times too high.

## Accomplishments

- `wsx` binary: `--version` prints `wsx <CARGO_PKG_VERSION>` to stdout and exits 0. Any other input prints `usage: wsx --version` to stderr and exits 2. Standard library only, no `unwrap`.
- `scripts/check.sh` (mode 100755, passes shellcheck and `bash -n`):
  - `all` (the default, also used for an empty first argument) runs fmt, then clippy, then test.
  - `require_tool` names any missing binary and exits 1.
  - `--no-tests=fail` comes before the caller's arguments, and a caller `--no-tests` is refused with exit 2.
  - An unknown target, or an extra argument to any target other than `test`, exits 2 with usage.
- `rust-toolchain.toml` pins rustc 1.99.1. `check.sh` prints the `rustc` version once per run.
- `clippy.toml` bans `std::env::set_var` and `std::env::remove_var`. clippy runs with `--all-targets`, so test code is covered.
- `.config/nextest.toml` writes a JUnit report to `target/nextest/default/junit.xml`, which is the format the TDD RED classifier reads.
- `.gitignore` gains a `# Rust` / `/target/` block at the top, above `# Python`.

## Task Commits

1. **Task 1 RED: failing CLI tests + gate script + crate skeleton** - `9c01ad8` (test)
2. **Task 1 GREEN: report the wsx version on --version** - `2667415` (feat)
3. **Task 2: pin rustc 1.99.1 and ban env mutation in tests** - `d3b1721` (feat)

**Plan metadata:** recorded in the docs commit that adds this SUMMARY.

## TDD record (Task 1)

- **RED:** `scripts/check.sh test` exited **100** with both tests failing on their assertions. There was no compile error.
  - Target `version_flag_prints_package_version` failed with `assertion left == right failed: wsx --version stdout`, left `""`, right `"wsx 0.0.0\n"` (tests/cli.rs:27:5).
  - Its sibling `missing_argument_exits_with_usage` failed at tests/cli.rs:39:5, on the usage-on-stderr assertion.
  - Report `target/nextest/default/junit.xml`: run started 2026-10-22T19:37:56.142Z, report modified 19:37:56.810Z (newer than run start).
  - Classifier: `check tdd-red-evidence` returned `RED_EVIDENCE_OK`, reason `target_test_failed`, format `junit`, matched `wsx::cli#version_flag_prints_package_version`.
  - Negative controls on the classifier using the same record: a changed target name gave `INVALID_RED / no_target_test_failure`, and exit code 0 gave `INVALID_RED / unexpected_green`.
  - **Semantic assessment:** the target ran and failed on the planned stdout assertion, because `main` was empty. Two tests were discovered. There were no setup or compile faults and no unrelated failures.
- **GREEN:** `src/main.rs` matches on `args_os().skip(1)`. `scripts/check.sh all` exits 0 with both tests passing.
- **REFACTOR:** none needed.
- **Gate log:** `test(01-01)` 9c01ad8 comes before `feat(01-01)` 2667415. Both gate commits are present.

## Control evidence (quoted from the runs)

**C2, no-match guard against the env override (DEV-01, T-01-03)**
```
$ NEXTEST_NO_TESTS=pass scripts/check.sh test -E 'test(=no_such_test)'   -> exit 4
==> cargo nextest run --locked --workspace --no-fail-fast --no-tests=fail -E test(=no_such_test)
    Starting 0 tests across 2 binaries (2 tests skipped)
     Summary [   0.000s] 0 tests run: 0 passed, 2 skipped
error: no tests to run
$ NEXTEST_NO_TESTS=pass cargo nextest run --locked -E 'test(=no_such_test)'   -> exit 0
    Starting 0 tests across 2 binaries (2 tests skipped)
     Summary [   0.000s] 0 tests run: 0 passed, 2 skipped
```
For reference, bare `cargo nextest run` with that filter and no env var exits 4. This is nextest's own default, which is why the env-var counterpart is the control that discriminates.

**Adjacency edge (caller `--no-tests`, T-01-01):** `scripts/check.sh test --no-tests=pass -E 'test(=no_such_test)'` exits **2** with `error: --no-tests is fixed to fail by check.sh and cannot be overridden`. The split form `--no-tests pass` is refused the same way. The abbreviation `--no-test=pass` gets past check.sh's check, but nextest rejects it itself with exit 2.

**Empty and usage edges:** `scripts/check.sh` and `scripts/check.sh ""` both run fmt, clippy and test, then print `==> check.sh: all OK`. `scripts/check.sh bogus` exits 2 (`error: unknown target 'bogus'` plus usage). `scripts/check.sh fmt extra` exits 2 (`error: check.sh fmt takes no arguments` plus usage).

**C1, missing tool is named (T-01-02)**
```
$ env PATH="$HOME/.cargo/bin:/usr/bin:/bin" scripts/check.sh test   -> exit 1
==> rustc 1.99.1 (58cf8f9c3 2026-10-13)
error: 'cargo-nextest' is required for 'check.sh test' but is not on PATH; run scripts/install-tools.sh install cargo-nextest
$ env PATH=/usr/bin:/bin scripts/check.sh fmt   -> exit 1
error: 'cargo' is required for 'check.sh fmt' but is not on PATH; install rustup from https://rustup.rs
$ env PATH=/usr/bin:/bin scripts/check.sh clippy   -> exit 1
error: 'cargo' is required for 'check.sh clippy' but is not on PATH; install rustup from https://rustup.rs
```
Counterparts: `scripts/check.sh test` with the full PATH ends `==> check.sh: test OK` (exit 0), and `scripts/check.sh fmt` ends `==> check.sh: fmt OK` (exit 0). Both minimal PATHs were explicit; neither was built by filtering the real PATH (Pitfall 3).

**DEV-02, toolchain pin:** `rustup show active-toolchain` gives `1.99.1-x86_64-unknown-linux-gnu (overridden by '.../rust-toolchain.toml')`. Before the file existed, it gave `stable-x86_64-unknown-linux-gnu (default)`. `scripts/check.sh all` prints `==> rustc 1.99.1 (58cf8f9c3 2026-10-13)` exactly once.

**C3, env-mutation ban (HARN-04, T-01-04)**
```
$ <scratch a>/scripts/check.sh all   -> exit 101
==> rustc 1.99.1 (58cf8f9c3 2026-10-13)
==> cargo fmt --check
==> cargo clippy --locked --workspace --all-targets -- -D warnings
error: use of a disallowed method `std::env::set_var`
  --> src/main.rs:31:18
31 |         unsafe { std::env::set_var("WSX_PLANTED", "1") };
   = note: process-global and racy across concurrent tests and the children they spawn; scope it to the child with Command::env instead. ...
error: could not compile `wsx` (bin "wsx" test) due to 1 previous error
$ <scratch b, no clippy.toml>/scripts/check.sh clippy   -> exit 0
==> check.sh: clippy OK
```
The planted run has no `==> cargo nextest` line, which confirms the **ordering edge**: `all` stops at the first failing target. I ran the counterpart a second time to confirm the plant really compiled there. That run printed `Checking wsx v0.0.0`, and nextest listed 3 tests (2 plus the plant). The scratch copies under `exec-01-01/wsx-scratch/` were removed by their own `trap`, and none of the `c3.*` names remain.

## Files Created/Modified

- `Cargo.toml`: single package `wsx`, edition 2024, MIT, publish false, empty `[dependencies]`, no authors.
- `Cargo.lock`: zero-dependency lockfile, so `--locked` works.
- `.gitignore`: `# Rust` / `/target/` block added at the top.
- `.config/nextest.toml`: `[profile.default.junit] path = "junit.xml"`.
- `src/main.rs`: version reporting and usage refusal.
- `tests/cli.rs`: two CLI boundary tests.
- `scripts/check.sh`: the gate.
- `rust-toolchain.toml`: the rustc 1.99.1 pin.
- `clippy.toml`: the disallowed-methods ban.

## Decisions Made

- The RED evidence record is at `target/gsd/01-01-red-evidence.json`. It sits inside the project, so the classifier accepts it, and `target/` is gitignored, so it is not committed. The plan did not name a location.
- `show_toolchain` assigns `rustc --version` to a local first and echoes it afterwards (see Deviations).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] rustc failure could pass silently in show_toolchain**
- **Found during:** Task 2, step 3
- **Issue:** The plan says to echo `==> ` followed by `rustc --version`. Written as `echo "==> $(rustc --version)"`, a failing rustc only prints an empty version line, because `set -e` ignores a failed command substitution inside an argument. The plan's own rule bans exactly this kind of masked call.
- **Fix:** `local version; version="$(rustc --version)"; echo "==> $version"`. The assignment passes rustc's exit status to `set -e`.
- **Files modified:** scripts/check.sh
- **Verification:** shellcheck and `bash -n` pass, and `scripts/check.sh all` prints the version line once. The failing-rustc path itself was not exercised, because no control broke rustc.
- **Committed in:** d3b1721

**2. [Rule 3 - Blocking] rustfmt layout of tests/cli.rs**
- **Found during:** Task 1, GREEN step 9
- **Issue:** The RED commit's tests/cli.rs was not rustfmt-clean, so `check.sh all` failed at fmt.
- **Fix:** Ran `cargo fmt`. It changed only the line wrapping of two `assert_eq!` calls; no assertion changed.
- **Committed in:** 2667415

---

**Total deviations:** 2 auto-fixed (1 missing critical, 1 blocking)
**Impact on plan:** Both are small and stay inside the plan's own rules. No scope creep.

## Issues Encountered

- `println!` in `src/main.rs` panics (exit 101) if stdout is closed, for example `wsx --version` piped into a command that exits first. This is acceptable for a placeholder, but Phase 5's exit-code contract (IF-01) should decide how a broken pipe is handled.
- The RED commit (9c01ad8) does not pass `check.sh fmt`. This is by design: only the RED commit is exempt from the all-green gate. It matters only if someone bisects with `check.sh all`.

## User Setup Required

None. No external service configuration is required.

## Next Phase Readiness

- 01-02 can add the `deps` target and `scripts/install-tools.sh`. `check.sh` already points at `scripts/install-tools.sh install cargo-nextest`, and that script does not exist until 01-02.
- The `rust-toolchain.toml` comment refers to `.devcontainer/Dockerfile` and `scripts/check-in-container.sh`, which arrive in 01-03.
- Requirements: HARN-04 is marked complete. DEV-01 and DEV-02 are also declared by sibling plans, so the shared-ID gate keeps them open until those plans finish.

## Self-Check: PASSED

- All 9 key files exist on disk.
- Commits 9c01ad8, 2667415 and d3b1721 are ancestors of HEAD. `check evaluation-scope --plan 01-01 --commits-only` resolves 3 commits.
- Every Task 1 and Task 2 acceptance criterion was re-run at HEAD d3b1721 and passed:
  - `.gitignore` head and `git check-ignore`
  - `cargo metadata --locked`
  - Cargo.lock tracked
  - `scripts/check.sh` at mode 100755, shellcheck, `bash -n`
  - test-before-feat commit order
  - no diff to bin/wsx or tests/sandbox.sh
  - TOML contents
  - no `c3.*` scratch left
- Plan-level verification re-run at HEAD: `check.sh all` exit 0, a single `==> rustc` line, and `==> check.sh: all OK` as the last line. C2 gave 4 and 0, the adjacency edge 2, the bogus target 2, and `fmt extra` 2.

---
*Phase: 01-development-foundation*
*Completed: 2026-10-22*
