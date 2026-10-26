---
phase: 02-oracle-harness
plan: 06
subsystem: testing
tags: [rust, coverage, cargo-llvm-cov, llvm-tools, rustup, install-tools, harness]

requires:
  - phase: 02-oracle-harness
    provides: "02-02 tests/harness/env.rs (ScenarioDirs, hermetic_env with LLVM_* passthrough, command), 02-03 coverage.rs module header"
provides:
  - "llvm-tools component on the pinned 1.99.1 toolchain (rust-toolchain.toml), installed on the host and in the pinned image"
  - "cargo-llvm-cov 0.9.1 pinned by version and sha256 in scripts/install-tools.sh, with a per-tool direct_sub word for install and verify"
  - "coverage::spawned_wsx_reports_version (subject, no-profraw invariant) and coverage::spawned_wsx_without_llvm_passthrough (control B)"
  - "One-time coverage proof with both negative controls, recorded below"
affects: [05-cli-contract, ci-coverage-job]

actuals:
  tokens: 2978
  tasks: 2
  commits: 2
plan_head_before: 5315f4fcc18c7eea98025d4c1085dd483679b17d
plan_head_after: 4b31c5544d44ea6df3532d1207f984f5196aa23e

tech-stack:
  added: ["cargo-llvm-cov 0.9.1 (developer tool, pinned; not a crate dependency)", "rustup component llvm-tools"]
  patterns:
    - "A tool that must be run with a subcommand word when invoked directly declares it as direct_sub in tool_info; install and verify pass it as ${direct_sub:+\"$direct_sub\"}, verify-cargo never does"
    - "Every cargo llvm-cov command runs with CARGO_LLVM_COV_SETUP=no"
    - "Spawned-binary tests assert no *.profraw under the scenario root (catches a lost LLVM_* passthrough under coverage)"

key-files:
  created: []
  modified:
    - rust-toolchain.toml
    - scripts/install-tools.sh
    - CONTRIBUTING.md
    - tests/harness/coverage.rs

key-decisions:
  - "02-06: the cargo-llvm-cov release binary prints exactly `cargo-llvm-cov 0.9.1`, so its version pattern keeps cargo-deny's `$` anchor, not nextest's `( |$)` tail (RESEARCH A5 settled by the scratch install)"
  - "02-06: `rustup toolchain install` at the repository root installed llvm-tools from the toolchain file; no `rustup component add` was needed"
  - "02-06: control B's coverage branch is proven to run under cargo llvm-cov by a mutation (flipping its assertion makes the coverage run fail with the stray-profile message), since cargo-llvm-cov 0.9.1 cannot report coverage of tests/ files"

patterns-established:
  - "direct_sub in tool_info: the subcommand word a cargo-* binary needs when run directly"

requirements-completed: [HARN-02]

coverage:
  - id: D1
    description: "The pinned 1.99.1 toolchain carries llvm-tools on the host and in the pinned image, from rust-toolchain.toml alone"
    requirement: HARN-02
    verification:
      - kind: other
        ref: "rustup component list --installed --toolchain 1.99.1-x86_64-unknown-linux-gnu lists llvm-tools-x86_64-unknown-linux-gnu (control: 1.97.1 does not, rc 1)"
        status: pass
      - kind: other
        ref: "docker run of the pinned image with the new rust-toolchain.toml lists llvm-tools; with HEAD's file it does not"
        status: pass
      - kind: integration
        ref: "scripts/check-in-container.sh test (==> check.sh: test OK)"
        status: pass
    human_judgment: false
  - id: D2
    description: "cargo-llvm-cov is pinned by version and sha256; install, verify (missing, wrong version) and verify-cargo bite, and the other tools are checked as before"
    requirement: HARN-02
    verification:
      - kind: other
        ref: "Task 1 verify block controls C1-C7 and mutations M1-M3 (quoted below)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Spawned wsx --version through the hermetic env reports its version with no profile file; the no-passthrough control asserts the stray profile under coverage and none outside"
    requirement: HARN-02
    verification:
      - kind: integration
        ref: "tests/harness/coverage.rs#coverage::spawned_wsx_reports_version"
        status: pass
      - kind: integration
        ref: "tests/harness/coverage.rs#coverage::spawned_wsx_without_llvm_passthrough"
        status: pass
    human_judgment: false
  - id: D4
    description: "cargo llvm-cov counts the println! line only the spawned binary runs, and both negative controls (no spawn; spawn without LLVM_*) count 0"
    requirement: HARN-02
    verification:
      - kind: other
        ref: "CARGO_LLVM_COV_SETUP=no cargo llvm-cov nextest --locked --no-tests=fail -E 'test(=...)' --text, three runs quoted below"
        status: pass
    human_judgment: false
  - id: D5
    description: "CONTRIBUTING setup step 2 says cargo-llvm-cov is pinned but optional; the required install line is unchanged"
    verification:
      - kind: other
        ref: "rg -q 'cargo-llvm-cov' CONTRIBUTING.md (HEAD baseline rc 1); rg -q 'install cargo-nextest cargo-deny gitleaks$' CONTRIBUTING.md rc 0"
        status: pass
    human_judgment: false

duration: 6min
completed: 2026-10-26
status: complete
---

# Phase 2 Plan 06: Coverage tooling and spawned-binary coverage proof Summary

**llvm-tools on the pinned 1.99.1 toolchain, cargo-llvm-cov 0.9.1 pinned by sha256 with a `direct_sub` subcommand word, and a one-time proof that `cargo llvm-cov` counts `src/main.rs`'s `println!` only when the spawned `wsx` receives `LLVM_*` (subject 1, both controls 0).**

## Performance

- **Duration:** 6 min (measured from the recorded start; reading time before it is not counted)
- **Started:** 2026-10-26T17:39:04Z
- **Completed:** 2026-10-26T17:45:22Z
- **Tasks:** 2
- **Files modified:** 4

## Accomplishments

- `rust-toolchain.toml` lists `llvm-tools`; host and pinned image both install it from that file.
- `scripts/install-tools.sh` pins cargo-llvm-cov 0.9.1 (sha256 `3fca9503...a522`); `install` and `verify` pass the binary's `llvm-cov` word, `verify-cargo` does not.
- Two harness tests: the coverage subject with the no-profraw invariant, and control B.
- The coverage proof ran once with both negative controls; `target/llvm-cov-target` (181M) was removed afterwards.

## Task Commits

1. **Task 1: llvm-tools on the pinned toolchain and the cargo-llvm-cov pin** - `9e80f26` (chore)
2. **Task 2: spawned-binary coverage subject, control B, one-time proof** - `4b31c55` (test)

## Files Created/Modified

- `rust-toolchain.toml` - `components = ["clippy", "rustfmt", "llvm-tools"]`, with the reason in the opening comment
- `scripts/install-tools.sh` - cargo-llvm-cov pin, `tool_info` case, `direct_sub` for every tool, usage and header text
- `CONTRIBUTING.md` - setup step 2: cargo-llvm-cov pinned but optional, no version number
- `tests/harness/coverage.rs` - `spawned_wsx_reports_version`, `spawned_wsx_without_llvm_passthrough`

## Task 1 evidence (each failing line next to its passing counterpart)

- Component, host: `rustup toolchain install` alone installed it (`info: downloading component llvm-tools`); `rustup component add` was not needed. Check rc 0 on 1.99.1; control on 1.97.1 rc 1.
- Component, pinned image `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`: with the new file the installed list includes `llvm-tools-x86_64-unknown-linux-gnu`; with HEAD's file it does not (rustc, cargo, rust-std, clippy, rustfmt only). `scripts/check-in-container.sh test` exited 0, last line `==> check.sh: test OK`, after `info: downloading 6 components`.
- Bare flag measurement: `cargo-llvm-cov --version` exits 1 with `error: expected subcommand 'llvm-cov', found argument '--version'`; `cargo-llvm-cov llvm-cov --version` prints `cargo-llvm-cov 0.9.1`.
- `verify-cargo cargo-llvm-cov`: `==> cargo llvm-cov: cargo-llvm-cov 0.9.1 OK`.
- Scratch install: `==> installed cargo-llvm-cov 0.9.1 to .../exec-02-06/llvmcov.999RbH`, and the installed binary printed exactly `cargo-llvm-cov 0.9.1` (so the `$`-anchored pattern holds; RESEARCH A5 settled).
- Wrong-version stub (answers only `llvm-cov`): exit 1, `error: cargo-llvm-cov at .../stub-llvmcov/cargo-llvm-cov reports 'cargo-llvm-cov 0.9.0', expected 0.9.1; ...`; the real binary then `==> cargo-llvm-cov 0.9.1 OK`.
- Unchanged-tool stub (cargo-deny accepting only a bare `--version`): `==> cargo-deny 0.20.2 OK`, then `==> cargo nextest: cargo-nextest 0.9.148 OK` and `==> cargo deny: cargo-deny 0.20.2 OK`.
- Minimal PATH: exit 1, `error: cargo-llvm-cov is not on PATH; run scripts/install-tools.sh install cargo-llvm-cov`.
- Mutations of a scratch copy of the script, each of which must fail and did:
  - M1, passthrough removed from `cmd_verify`: the wrong-version stub fails with `failed to report its version`, not with the version mismatch, and the real binary fails with `expected subcommand 'llvm-cov'`.
  - M2, `"$direct_sub"` passed unconditionally: the cargo-deny stub sees `unexpected arguments:  --version` and fails. This shows the `:+` form matters.
  - M3, one sha256 digit changed: `error: checksum mismatch for cargo-llvm-cov 0.9.1: refusing to install`, rc 1, nothing installed.
- `bash -n` and `shellcheck` clean; `scripts/check.sh all` exit 0 (`==> check.sh: all OK`).
- Acceptance: the version `0.9.1` appears only at `scripts/install-tools.sh:43` (`git grep -nwF`; HEAD baseline had no match). `git diff --name-only <merge-base workspace/denniyahh> HEAD -- scripts/check.sh .github` is empty, and the same command for `scripts/install-tools.sh` lists the file.

## Task 2 evidence: the one-time coverage proof

All runs used `CARGO_LLVM_COV_SETUP=no cargo llvm-cov nextest --locked --no-tests=fail -E 'test(=<name>)' --text` and exited 0. They shared one instrumented build in `target/llvm-cov-target`.

| Run | Selected test | `src/main.rs` line | Test result |
|-----|---------------|--------------------|-------------|
| Subject | `coverage::spawned_wsx_reports_version` | `17\|      1\|            println!("wsx {}", env!("CARGO_PKG_VERSION"));` | `PASS [   0.007s] (1/1) wsx::harness coverage::spawned_wsx_reports_version` |
| Control A (never spawns wsx) | `fixture::fixture_commits_have_distinct_dates` | `17\|      0\|            println!("wsx {}", env!("CARGO_PKG_VERSION"));` | `PASS [   0.172s] (1/1) wsx::harness fixture::fixture_commits_have_distinct_dates` |
| Control B (spawn without `LLVM_*`) | `coverage::spawned_wsx_without_llvm_passthrough` | `17\|      0\|            println!("wsx {}", env!("CARGO_PKG_VERSION"));` | `PASS [   0.012s] (1/1) wsx::harness coverage::spawned_wsx_without_llvm_passthrough` |

**Proof that control B's coverage branch ran.** Run B passing does not by itself show which branch executed. To check, the coverage-branch assertion was flipped temporarily to `stray.is_empty()` and run B repeated. It failed (rc 100) with `under coverage, a child without LLVM_* must drop a profile file in its working directory .../target/llvm-cov-target/tmp/harness-coverage-without_llvm-7udvQj`. The file was then restored, and `git status` showed it unchanged. So under `cargo llvm-cov` the test process does have `LLVM_PROFILE_FILE`, and the child without passthrough leaves a stray profile file.

Outside coverage, `scripts/check.sh test` runs both tests and they pass, which exercises the no-profile branch. A misnamed filter control exits 4.

**Cleanup:** `target/llvm-cov-target` (181M) was removed, and `test -d target/llvm-cov-target` now exits 1. No other `target/llvm-cov*` path, `*.profraw` file or `harness-coverage-*` directory remains. The scratch directory `exec-02-06` is empty.

## What these checks do not establish

- The proof is one run of three commands on this host. It shows that the passthrough mechanism carries coverage for one line of one spawned binary under one test. It does not show that every future scenario is covered.
- No coverage threshold is set or checked, and no gate or CI job runs coverage (CONTEXT defers that to phase 5 or later). Nothing will catch a later regression of the passthrough, except the no-profraw invariant when someone happens to run coverage.
- The container check proves the pinned image installs llvm-tools from the toolchain file and that the test gate passes there. Coverage was not run in the container.
- The scratch install proves the 0.9.1 release tarball matches the pinned sha256 today. That digest was taken from RESEARCH (GitHub API plus a local `sha256sum`) and was not re-derived independently in this run.

## Decisions Made

See key-decisions in the frontmatter: the `$`-anchored pattern is kept (A5 settled), the toolchain install alone was sufficient, and the mutation was used as branch evidence for control B.

## Deviations from Plan

None to plan intent. Execution differences:

- **Verify-command shape:** the plan's stub and install controls create scratch dirs with `mktemp -d` plus `trap 'rm -rf "$S"'` inside `bash -c`. The Claude Code safety check refused that form for the stub controls. The removal targets a variable it cannot resolve. The same assertions ran with fixed literal scratch paths, which were removed afterwards by literal paths. The assertions themselves are unchanged.
- **CONTRIBUTING wording:** the plan says to add "one line". The addition is one sentence, wrapped over three lines at the file's width.
- **Extra controls beyond the plan:** mutations M1-M3, the direct container component listing with the HEAD control, the 1.97.1 component control, and the control-B branch mutation. They add evidence and change no code.

**Total deviations:** 0 auto-fixed. **Impact:** none.

## Issues Encountered

- `rustup toolchain install` also self-updated rustup itself from 1.29.1 to 1.29.2. This is rustup's default auto-self-update, not something this plan asked for. The host now reports `rustup 1.29.2 (d8c92c676 2026-10-21)`. The pinned toolchain and rustc 1.99.1 are unchanged.

## User Setup Required

None. cargo-llvm-cov is optional for contributors.

## Next Phase Readiness

HARN-02's coverage half is shown. A future CI coverage job can rely on `rust-toolchain.toml` for the component and on `scripts/install-tools.sh install cargo-llvm-cov` for the tool.

---
*Phase: 02-oracle-harness*
*Completed: 2026-10-26*

## Self-Check: PASSED

Commits 9e80f26 and 4b31c55 are ancestors of HEAD; all four modified files exist; target/llvm-cov-target is absent.
