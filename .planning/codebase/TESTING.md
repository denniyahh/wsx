---
last_mapped_commit: a686ea156dac3e4c93d9f9d180ee4a35e340c4f5
last_mapped_at: 2026-11-18
---
# Testing Patterns

**Analysis Date:** 2026-11-18

## Test Framework

**Runner:**
- cargo-nextest (version pinned in `scripts/install-tools.sh`), driven by `scripts/check.sh`; `--no-tests=fail` is always set.
- Rust toolchain `1.99.1` (`rust-toolchain.toml`). The same gate runs in the pinned image via `scripts/check-in-container.sh`.

**Assertion Library:**
- std `assert!`/`assert_eq!` with a message naming the case; `proptest` (`prop_assert_eq!`) for properties. Dev-deps: `tempfile`, `proptest` (`default-features = false`).

**Run Commands:**
```bash
scripts/check.sh all                               # fmt, clippy, test: the gate for every change
scripts/check.sh test                              # suite only
scripts/check.sh test -E 'test(=module::name)'     # one named test; chain several with &&, never '|'
scripts/check.sh coverage                          # cargo llvm-cov, CARGO_LLVM_COV_SETUP=no
scripts/check-in-container.sh all                  # in the CI image
```

## Test File Organization

**Location:**
- Unit tests in-file: `#[cfg(test)] mod tests` at the bottom of each `src/` module (`src/git/mod.rs`, `src/model/manifest.rs`, ...).
- Shared unit helpers: `src/git/testing.rs` (`Scratch`, `hermetic_env`, `git_ok`, `commit_all`, `install_hook`, `child_print`, `all_bytes_repo`).
- Integration targets in `tests/`: `cli.rs`, `gate.rs`, `gate_controls.rs`, `image_parity.rs`, `repo_policy.rs`, `setup_repo.rs` (tooling and scripts contracts).
- Black-box scenario harness: one target `tests/harness/main.rs` with modules.
- Oracle: vendored devflow shell scripts `tests/oracle/*.sh` plus `oracle.patch`; fixture crate `tests/fixtures/env-ban/`.

**Naming:**
- Harness tests: `<module>::<scenario>::<side>`, e.g. `sync::self_test::oracle`, `sync::self_test::rust`.

**Structure:**
```
tests/harness/
├── main.rs        # harness_modules!(sync, worktree, check, cut, guard, signing)
├── scenario.rs    # Scenario/Case/Class, scenarios! macro, runner
├── fixture.rs     # Fixture, Ident, RepoConfig, ACTION_TIME, T0, TICK
├── outcome.rs     # Outcome/Snapshot, observe, compare, *Expect types
├── env.rs         # hermetic_env, ScenarioDirs
├── oracle.rs      # runs patched copy of tests/oracle scripts
├── rust.rs        # runs the wsx binary
├── action.rs, registry.rs, coverage.rs, provenance.rs, hermeticity.rs
└── sync.rs, worktree.rs, check.rs, cut.rs, guard.rs, signing.rs  # scenarios
```

## Test Structure

**Suite Organization (integration contract test, `tests/gate.rs`):**
```rust
fn assert_refused(args: &[&str], expected_stderr: &str) {
    let output = check_sh_test(args);
    assert_eq!(output.status.code(), Some(USAGE_EXIT), "check.sh test {args:?} exit code; stderr: {stderr:?}");
    assert!(stderr.contains(expected_stderr), "...");
    assert!(!stdout.contains("OK"), "...");
}

#[test]
fn help_flag_is_refused() {
    assert_refused(&["--help"], "not '--help'");
}
```

**Patterns:**
- Setup: `Scratch::new(name)` / `tempfile::TempDir::new_in` under `CARGO_TARGET_TMPDIR` (never RAM-backed `/tmp`); auto-removed on panic.
- Assertions check exit code, the intended stderr text, and absence of a false success, separately. No regex alternation where both halves are required.
- Messages name the input (`"only {}", shown(value)`) so a failure is readable.
- One scenario per test.

## Mocking

**Framework:** None. Tests run the real `git` binary and real gix against scratch repositories.

**Patterns:**
```rust
// Re-executed child: the parent spawns the test binary with a scoped env,
// the ignored child prints a key=value the parent asserts on (src/git/config.rs)
#[test]
#[ignore = "child half of the config scope tests; runs only when re-executed"]
fn count_child() {
    let start = std::env::var_os(START_VAR).expect("the parent set the start directory");
    match Repo::discover(Path::new(&start)).and_then(|found| found.read_config(&Git::from_process(&found))) {
        Ok(read) => testing::child_print("count", read.values(KEY).len()),
        Err(err) => testing::child_print("error", err.to_string().replace('\n', "\\n")),
    }
}
```

**What to Mock:**
- Nothing. Isolate instead: every child starts from `env_clear` plus harness keys (scenario `HOME`, `XDG_*`, git config, fixed `PATH`, `GIT_CEILING_DIRECTORIES` at the scratch parent) plus `LLVM_*` (`tests/harness/env.rs`, `src/git/testing.rs::hermetic_env`). Hooks are real scripts via `install_hook`.
- Process env: never `std::env::set_var` (banned by `clippy.toml`); to vary env, re-execute a `_child` test with `Command::env`.

**What NOT to Mock:**
- git, gix, the filesystem, the `wsx` binary, `scripts/check.sh`.

## Fixtures and Factories

**Test Data:**
```rust
// tests/harness/sync.rs
pub const SEED_FILES: [File; 2] = [(b"README.md", b"# Fixture\n"), (b"src/a.txt", b"a\n")];
pub const WORKSPACE_REF: &str = "refs/heads/workspace/dev";
// fixture steps: pub type FixtureFn = fn(&mut Fixture) -> Result<(), String>;
```
- Commits use fixed dates (`T0`, `TICK`, `ACTION_TIME`, `dated(env, epoch)`, `commit_all(scratch, dir, msg, epoch)`) for deterministic ids.
- Scenario = subject `Case` (repo config, fixture steps, `Action`, explicit expected `Outcome`) plus control cases. `Class::Parity`, `Class::Fixed { defect }`, `Class::Regression { reason }`; `OracleSide::SameAsExpected | Recorded(Outcome) | NotImplemented { why }`.
- The Rust side of a scenario is generated `#[ignore = "<reason>"]` until its phase enables it, so a filter selecting only it fails with "no tests to run".

**Location:**
- `tests/harness/fixture.rs`, per-scenario constants in each scenario module, `src/git/testing.rs` for unit tests, `tests/fixtures/` for fixture crates.

## Coverage

**Requirements:** None enforced; no CI job. `scripts/check.sh coverage` needs the pinned cargo-llvm-cov and the `llvm-tools` component.

**View Coverage:**
```bash
scripts/check.sh coverage
```

## Test Types

**Unit Tests:** in-file, against scratch repos; error-message contracts asserted verbatim (`error_messages_name_their_cause` in `src/git/mod.rs`).

**Property Tests:** `proptest!` in `src/model/manifest.rs` with `property_config()`: `PROPERTY_CASES = 2048`, `failure_persistence: None` (checkout is read-only in the container). A found failure becomes a named regression test, not a persisted seed. `property_runs_its_configured_cases` is the control that the count actually ran.

**Integration Tests:** `tests/*.rs` spawn scripts and the binary (`env!("CARGO_MANIFEST_DIR")`, `env!("CARGO_BIN_EXE_wsx")`). `tests/gate_controls.rs` holds negative controls proving the gate fails when it should.

**E2E Tests:** the scenario harness (oracle vs. Rust sides, judged against the same explicit outcome).

## Common Patterns

**Acceptance bar (`CONTRIBUTING.md` "AI change acceptance"):** a regression test that fails before the change, an assertion at a stable boundary, evidence it fails for the intended reason, `check.sh all` clean, independent review. Rejected: tests asserting only constants, reproducing the production algorithm, comparing a call with itself, or grepping source instead of asserting runtime behaviour.

**Error Testing:**
```rust
assert_eq!(
    parse(&only_line(value)),
    Err(ParseError { line: 1, text, problem }),
    "only {}", shown(value)
);
```

**Writes:** the container mounts the checkout read-only; any test that writes uses a scratch directory.

---

*Testing analysis: 2026-11-18*
