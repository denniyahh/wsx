# Phase 2: Oracle harness - Pattern Map

**Mapped:** 2026-10-26
**Files analyzed:** 30 new or modified: 15 `tests/harness/*.rs`, 8 `tests/oracle/*`, 7 config/script/doc edits
**Analogs found:** 16 / 30 with an in-repo analog. 6 vendored files are byte copies of an external source (devflow). 7 have no analog (5 scenario modules, `oracle.patch`, oracle `README.md`). `Cargo.lock` is generated

All analog paths below were checked with `git ls-files` on `feature/phase-2` at `8f77240`. Line
numbers were read this session, not taken from RESEARCH.md.

## Scope corrections since RESEARCH.md

RESEARCH.md's "Suggested plan decomposition" (lines 760-777) was written before CONTEXT.md was
corrected. Two CONTEXT decisions now override it:

1. **`rust-toolchain.toml` gains `llvm-tools`** (CONTEXT lines 113-118). RESEARCH Open Question 2
   recommended option B (a host-only step); CONTEXT locked option A. Plan 02-07 therefore also
   modifies `rust-toolchain.toml`.
2. **Fork layout and the no-GSD repo get oracle-recorded variants where the oracle runs them**
   (CONTEXT lines 52-59). RESEARCH Open Question 1 and its case map (lines 381, 383) still say
   "Rust-only (locked)". Plans 02-08/09/10 must add oracle variants: cut and sync with a non-`origin`
   base remote, and `phase-worktree.sh` refusing in a repo with no ROADMAP (a recorded refusal).

Still open, not settled by CONTEXT: RESEARCH Open Question 4, adding `include-dev = true` to
`deny.toml [licenses]`. Without it the license gate does not check `tempfile` at all (RESEARCH
line 230). That is a planner or owner choice, not a pattern question.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `Cargo.toml` | config | n/a | `Cargo.toml` (self) + `deny.toml` comment style | exact (edit) |
| `Cargo.lock` | config (generated) | n/a | `cargo` output | n/a |
| `deny.toml` (only if Open Q4 is answered yes) | config | n/a | `deny.toml:13-18` | exact (edit) |
| `rust-toolchain.toml` | config | n/a | `rust-toolchain.toml:11-14` | exact (edit) |
| `.config/nextest.toml` | config | n/a | `.config/nextest.toml:1-12` | exact (edit) |
| `scripts/install-tools.sh` | utility (tool pin) | file-I/O (download, verify) | `scripts/install-tools.sh:34-94` cargo-deny entry | exact (edit) |
| `CONTRIBUTING.md` install line | docs | n/a | `CONTRIBUTING.md:19,25` | exact (edit) |
| `tests/oracle/{sync-workspace,phase-worktree,cut-pr-branch,check-workspace-divergence}.sh`, `tests/oracle/hooks/pre-push`, `tests/oracle/LICENSE` | vendored fixture | n/a | devflow `ee6ec792` blobs (external) | byte copy |
| `tests/oracle/README.md` | docs (provenance) | n/a | none in repo | no analog |
| `tests/oracle/oracle.patch` | fixture (patch) | transform | none in repo | no analog |
| `tests/harness/main.rs` | test binary root, registry | batch | `tests/gate_controls.rs:1-34` (module doc, consts) | role-match |
| `tests/harness/env.rs` | test utility | transform (pure env builder) | `tests/setup_repo.rs:28-46` `hermetic()`; `tests/gate_controls.rs:349-361` `without_git_env()` | exact |
| `tests/harness/fixture.rs` | test utility (fixture builder) | file-I/O + process spawn | `tests/setup_repo.rs:48-117` `Fixture` | exact |
| `tests/harness/oracle.rs` | adapter | request-response (spawn bash) | `tests/setup_repo.rs:84-90` `setup()`; `tests/image_parity.rs:30-40` `run_parity()` | role-match |
| `tests/harness/rust.rs` | adapter | request-response (spawn wsx) | `tests/cli.rs:7-17` `wsx_bin()`, `run_wsx()` | exact |
| `tests/harness/outcome.rs` | model + comparator | transform | `tests/image_parity.rs:24-28` `Run`; `tests/repo_policy.rs:152-171` violations list | partial |
| `tests/harness/scenario.rs` | model + validator + macro | transform | `tests/repo_policy.rs:152-212` (checker plus rejection test) | partial |
| `tests/harness/provenance.rs` | test | file-I/O | `tests/repo_policy.rs:173-212` (policy check and its rejecting control) | role-match |
| `tests/harness/hermeticity.rs` | test (proof + control) | event-driven (hook markers) | `tests/setup_repo.rs:196-232` (marker-file refusal + control) | exact |
| `tests/harness/coverage.rs` | test | request-response | `tests/cli.rs:19-30` | role-match |
| `tests/harness/{sync,worktree,cut,check,guard}.rs` | scenario modules | batch | none (the scenario DSL is new) | no analog |

## Pattern Assignments

### `tests/harness/env.rs` (test utility, pure env builder)

**Analog:** `tests/setup_repo.rs:28-46`, the only `env_clear()` spawner in the repo.

```rust
/// A command that sees no inherited `GIT_*` variable and no global or system
/// config. `stub_bin` is put first on PATH.
fn hermetic(program: &str, dir: &Path, stub_bin: &Path) -> Command {
    let home = dir.join("home");
    fs::create_dir_all(&home).expect("create scratch HOME");
    let path = std::env::var_os("PATH").expect("PATH is set");
    let mut paths = vec![stub_bin.to_path_buf()];
    paths.extend(std::env::split_paths(&path));
    let mut command = Command::new(program);
    command
        .current_dir(dir.join("repo"))
        .env_clear()
        .env("PATH", std::env::join_paths(paths).expect("join PATH"))
        .env("HOME", &home)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("LC_ALL", "C");
    command
}
```

Copy: `env_clear()` first, then explicit `.env(...)` per key; `LC_ALL=C`; `GIT_CONFIG_NOSYSTEM=1`.
Deliberately diverge in three places (RESEARCH lines 413-415 and 387-411):
- `GIT_CONFIG_GLOBAL` is a real file at `<home>/.gitconfig`, not `/dev/null`, because the oracle
  pre-push hook unsets it and git then falls back to `$HOME/.gitconfig`.
- PATH is built from scratch (`/usr/local/bin:/usr/bin:/bin`), not prefixed to the parent's.
  The parent PATH on this host puts linuxbrew bash and ssh-keygen first.
- The builder is a pure function `hermetic_env(inherited, dirs) -> Vec<(OsString, OsString)>`, so
  the hermeticity proof can feed it a planted host environment without `set_var`.
- Add the keys RESEARCH found by experiment: `GIT_CEILING_DIRECTORIES`, `GIT_ALLOW_PROTOCOL=file`,
  `GIT_TERMINAL_PROMPT=0`, `GIT_EDITOR=false`, `GIT_MERGE_AUTOEDIT=no`, `TZ=UTC`, XDG dirs, and
  `LLVM_*` passthrough only.

Secondary analog, for the opposite approach the harness must NOT use: `tests/gate_controls.rs:349-361`
`without_git_env()` removes `GIT_*` keys one at a time and keeps the rest of the parent env. It
leaks HOME, XDG and SSH_AUTH_SOCK, which is fine for that test and wrong for the harness.

### `tests/harness/fixture.rs` (fixture builder, file-I/O and spawn)

**Analog:** `tests/setup_repo.rs:48-117` (`struct Fixture`, `new`, `git`, `git_ok`).

Spawn-and-assert helper to copy (lines 92-108); every fixture git call goes through one helper that
fails with the args and stderr:
```rust
fn git_ok(&self, args: &[&str]) -> String {
    let output = self.git(args);
    let stderr = text(output.stderr, "git stderr");
    assert_eq!(
        output.status.code(),
        Some(0),
        "git {args:?}; stderr: {stderr:?}"
    );
    text(output.stdout, "git stdout")
}
```

Diverge:
- Scratch root: the existing tests use a fixed name under `CARGO_TARGET_TMPDIR` plus
  `remove_dir_all` (`tests/setup_repo.rs:55-58`, `tests/gate_controls.rs:41-48`). CONTEXT locks
  `tempfile::TempDir` here instead: `TempDir::with_prefix_in("<scenario>-", env!("CARGO_TARGET_TMPDIR"))`.
  Keep the `CARGO_TARGET_TMPDIR` constant idiom: `const TMP_ROOT: &str = env!("CARGO_TARGET_TMPDIR");`
  (`tests/setup_repo.rs:15`, `tests/gate_controls.rs:27`, `tests/repo_policy.rs:12`).
- Paths and file names are bytes (`OsStrExt`), so do not reuse `text()` (`tests/setup_repo.rs:24-26`)
  for anything that can carry a fixture path. It panics on non-UTF-8, and HARN-06 has non-UTF-8 names.
- Args are `&[&OsStr]` or `impl AsRef<OsStr>`, not `&[&str]`, for the same reason.
- Each git invocation advances the deterministic clock (RESEARCH Pattern 4).

Executable stub pattern, if a fixture needs one (`tests/setup_repo.rs:77-82`):
```rust
fs::write(&stub, "#!/bin/sh\nexit 0\n").expect("write gitleaks stub");
fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod stub");
```

### `tests/harness/oracle.rs` (adapter, spawn bash on a patched copy)

**Analogs:** `tests/setup_repo.rs:84-90` (run a copied script under `bash` in the fixture) and
`tests/image_parity.rs:43-65` (copy the real script out of `REPO_ROOT` into a scratch tree, because
the script resolves paths from its own location).

```rust
fn setup(&self, args: &[&str]) -> Output {
    hermetic("bash", &self.dir, &self.dir.join("bin"))
        .arg("scripts/setup-repo.sh")
        .args(args)
        .output()
        .expect("spawn setup-repo.sh")
}
```
```rust
fs::copy(
    Path::new(REPO_ROOT).join("scripts/assert-image-parity.sh"),
    root.join("scripts/assert-image-parity.sh"),
)
.expect("copy assert-image-parity.sh");
```

Adapter-specific additions with no repo analog:
- Copy from `concat!(env!("CARGO_MANIFEST_DIR"), "/tests/oracle")` into the scenario TempDir, then
  `git apply oracle.patch` under `GIT_CEILING_DIRECTORIES=<TempDir>` (RESEARCH Pitfall 1: without the
  ceiling, `git apply` exits 0 and applies nothing).
- Set `WSX_ORACLE_DIR` on the child only (`Command::env`).
- Pre-push only: `Stdio::piped()` stdin with `<lref> <lsha> <rref> <rsha>\n` lines (RESEARCH lines
  732-740). Every other spawn uses `Stdio::null()`.
- Invoke as `bash <script>`, matching `hermetic("bash", ...)` above, not by executing the file
  directly. The copied mode is then irrelevant to whether it runs.

### `tests/harness/rust.rs` (adapter, spawn wsx)

**Analog:** `tests/cli.rs:7-17`.
```rust
/// Path to the `wsx` binary cargo built alongside these tests.
fn wsx_bin() -> &'static str {
    env!("CARGO_BIN_EXE_wsx")
}
```
Copy `env!("CARGO_BIN_EXE_wsx")`. Do not copy `run_wsx`'s plain `Command::new` (no `env_clear`).
Every spawn goes through `env.rs`. Under cargo-llvm-cov, `CARGO_BIN_EXE_wsx` is already the
instrumented binary (RESEARCH line 479).

### `tests/harness/outcome.rs` (model, comparator, plain-words rendering)

**Partial analogs:**
- `tests/image_parity.rs:24-40`: a typed struct holding one run's observed result (`Run { code, stdout,
  stderr }`). The harness `Observed` keeps `code: Option<i32>` the same way, so a signal death is
  not read as an exit code.
- `tests/repo_policy.rs:152-171` plus `:173-181`: a checker returns `Vec<String>` of human-readable
  violations, and the test asserts the list is empty and prints it with `{:#?}`:
```rust
let violations = git_policy_violations(&entries);
assert!(
    violations.is_empty(),
    "tracked .gitconfig violates the repository policy: {violations:#?}\nentries: {entries:#?}"
);
```
  The comparator `compare(expected, observed) -> Vec<Mismatch>` follows this shape. The difference
  is that `Mismatch` is typed (it names the field), so the per-field mutation meta-test can assert
  which field failed.
- Exhaustive destructuring without `..` (RESEARCH Pattern 1) has no analog in the repo.

### `tests/harness/scenario.rs` (model, `validate()`, `scenarios!` macro)

**Partial analog:** `tests/repo_policy.rs:183-212`, a checker paired with a test that feeds it
known-bad input and asserts each specific rejection by exact text:
```rust
for expected in [
    "personal key user.email must stay in local config",
    "unexpected commit.gpgsign=false",
    ...
] {
    assert!(
        violations.iter().any(|v| v == expected),
        "the policy check must report {expected:?}, got {violations:#?}"
    );
}
```
Each `validate()` rejection (missing expectation; `fixed:` without a differing recorded oracle
outcome; `regression:` that is not a refusal; a guard without both controls) gets one negative-control
test asserting its exact variant. Following the house rule (CLAUDE.md "Discriminating assertions"),
match the variant, not an alternation.

The macro and registry have no analog. Use RESEARCH lines 742-758 as the starting sketch; it has not
been compiled.

### `tests/harness/hermeticity.rs` (proof + control)

**Analog:** `tests/setup_repo.rs:196-232`. This is the closest match in the repo. A marker file
proves a planted command never ran, and a separate control test proves the same plant does run when
wired in:
```rust
// Run git the way a user would next: had the key been applied, this would
// run the fsmonitor command.
fixture.git_ok(&["status", "--porcelain"]);
assert!(!marker.exists(), "core.fsmonitor ran: {}", marker.display());
```
```rust
#[test]
fn refused_key_control_the_legacy_include_would_have_run_it() {
    // Counterpart to the refusal test: through the old include, the same
    // tracked file runs its command on `git status`. ...
    fixture.git(&["status", "--porcelain"]);
    assert!(marker.exists(), "the include did not apply core.fsmonitor");
}
```
Copy: one test per direction, not one test with both (CLAUDE.md "atomic test granularity"), and a
comment in the control naming what it rules out. The planted-host proof and
`wired_to_planted_config_fires_hook` map one-to-one onto this pair.

### `tests/harness/provenance.rs` (blob match, patch applies, fail-loud)

**Analog for "a check plus a control that must trip it":** `tests/gate_controls.rs:305-346`,
`repo_clippy_toml_bans_env_mutation` and `env_mutation_passes_clippy_without_the_ban`. The control's
comment states what the counterpart rules out:
```rust
// Counterpart: the same fixture under an empty clippy.toml passes, so the
// failure above comes from the ban and not from anything else in the
// fixture (a compile error, another lint).
```
Apply to: blob match vs. a scratch copy with one flipped byte; patch applies (with ceiling) vs. the
no-ceiling apply leaving files unchanged; `WSX_ORACLE_DIR` unset fails loud vs. set passes.

Recorded constants follow the named-`const` style with a doc comment (`tests/gate_controls.rs:28-34`,
`tests/image_parity.rs:16-22`). Blob SHAs come from RESEARCH lines 245-252 and were re-checked against
devflow `ee6ec792` this session with `git ls-tree` (all six match).

### `tests/harness/coverage.rs` (spawned-binary coverage proof)

**Analog:** `tests/cli.rs:19-30` `version_flag_prints_package_version`. Same assertion triple
(stdout, stderr empty, exit code), but the spawn goes through `env.rs`:
```rust
let expected = format!("wsx {}\n", env!("CARGO_PKG_VERSION"));
assert_eq!(stdout, expected, "`wsx --version` stdout");
assert_eq!(stderr, "", "`wsx --version` must not write to stderr");
assert_eq!(output.status.code(), Some(0), "`wsx --version` exit code");
```

### `tests/harness/main.rs` (binary root, registry meta-tests)

**Analog:** the module-doc header convention every test file uses (`tests/gate_controls.rs:1-18`,
`tests/setup_repo.rs:1-7`, `tests/image_parity.rs:1-7`). Each states what the file proves, how its
fixtures are isolated, and why. Example (`tests/gate_controls.rs:15-18`):
```rust
//! Unlike tests/gate.rs, the last two start nested cargo builds. Each gets its
//! own target directory under `CARGO_TARGET_TMPDIR`: the outer run's build
//! lock is never contended, and the container's read-only source mount is
//! never written.
```
nextest name shape: `tests/harness/main.rs` with `mod sync { mod x { fn oracle() } }` is gated as
`-E 'test(=sync::x::oracle)'`. Auto-discovery of `tests/harness/main.rs` as target `harness` is
RESEARCH assumption A2 and unverified. The first plan's verify step should confirm it with
`cargo nextest list`.

### `.config/nextest.toml` (add a per-test override)

**Analog:** the file itself (`.config/nextest.toml:1-12`). Each block is preceded by a comment
explaining why it exists:
```toml
# The JUnit report exists for one reason: the TDD gate's RED-evidence
# classifier reads JUnit XML, ...
[profile.default.junit]
path = "junit.xml"
```
Append the override (RESEARCH lines 527-533) with a matching "why" comment. Do not add any
no-tests setting: lines 8-10 say `--no-tests=fail` lives on the command line in `scripts/check.sh:151`.

### `scripts/install-tools.sh` (add the cargo-llvm-cov pin)

**Analog:** the cargo-deny entry. All touch points, in order:
1. Header comment, lines 2-4, lists the tools.
2. Pins, lines 34-39: add `CARGO_LLVM_COV_VERSION=0.9.1` and `CARGO_LLVM_COV_SHA256=3fca9503...a522`
   next to them.
3. Usage text, line 47 (`TOOL is one of: ...`) and line 55 (`cargo-nextest and cargo-deny only`).
4. `tool_info` case, lines 73-80. The cargo-deny shape to copy:
```bash
cargo-deny)
    version="$CARGO_DENY_VERSION"
    sha256="$CARGO_DENY_SHA256"
    url="https://github.com/EmbarkStudios/cargo-deny/releases/download/${version}/cargo-deny-${version}-x86_64-unknown-linux-musl.tar.gz"
    member="cargo-deny-${version}-x86_64-unknown-linux-musl/cargo-deny"
    version_arg="--version"
    pattern="^cargo-deny ${version//./\\.}\$"
    ;;
```
   For cargo-llvm-cov, `member="cargo-llvm-cov"` (archive root, RESEARCH line 235). The version
   pattern `^cargo-llvm-cov 0\.9\.1$` is RESEARCH assumption A5, so the downloaded binary's output
   has not been run. Copy nextest's `( |\$)` tail (line 71) instead of `\$` if the release build
   appends a hash.
5. Nothing else. `verify-cargo` (lines 125-136) already accepts any `cargo-*` name.

Existing pin tests to mirror: `tests/gate_controls.rs:144-210`, which checks that the pin is verified
on the binary cargo actually runs. The existing pins have no direct unit test for `tool_info`, so a
new test is optional.

Also update `CONTRIBUTING.md:19` and `:25`, the install and verify command lines, if cargo-llvm-cov is
meant to be installed by contributors. Leave `scripts/check-in-container.sh:146` alone: there is no
CI coverage job in phase 2 (CONTEXT line 123).

### `rust-toolchain.toml` (add llvm-tools)

**Analog:** the file itself, lines 11-14:
```toml
[toolchain]
channel = "1.99.1"
components = ["clippy", "rustfmt"]
profile = "minimal"
```
Add the component to `components`. The file opens with a "why" comment block (lines 1-10); add a
line explaining why llvm-tools is there. Component name: cargo-llvm-cov's own error asks for
`llvm-tools-preview` (RESEARCH line 482, assumption A6). Verify with `rustup component list --installed`
after the edit. The existing guard `tests/repo_policy.rs:58-100` parses `channel` only, so adding a
component does not affect it.

### `Cargo.toml` and `Cargo.lock` (add tempfile dev-dependency)

**Analog:** `Cargo.toml:9` has an empty `[dependencies]`; there is no `[dev-dependencies]` yet. The
justification-comment convention to copy is `deny.toml:13-16`:
```toml
[licenses]
# Only the licenses the crate graph actually uses: unused allowances only warn
# (license-not-encountered), so later phases extend this list as each crate
# arrives, not ahead of it.
allow = ["MIT"]
```
Add `[dev-dependencies]` with `tempfile = "3.27.0"` and a comment above it giving the justification
(CLAUDE.md "Do not add a dependency without a justification"). `scripts/check.sh:151` runs
`cargo nextest run --locked`, so `Cargo.lock` must be regenerated and committed in the same commit.

### `tests/oracle/*` vendored scripts and LICENSE

**Source:** devflow `ee6ec792d03f1495fc8c4d90d5894ff08b968d80`. Copy with
`git -C ~/Github/devflow show <sha>:<path>`, not from wsx's `scripts/workspace/` ports (CONTEXT
lines 66-74). Mode: devflow tracks all five scripts as `100755` and LICENSE as `100644`. Match that
with `git update-index --chmod=+x` or `chmod` before `git add`, following this repo's convention
(`git ls-files -s scripts/*.sh` shows `100755`). The blob test checks content only, so a wrong mode
would not be caught by it.

PR-cut safety, checked: `tests/oracle/` and `tests/harness/` match no `FORBIDDEN_PATHS` prefix
(`scripts/workspace/cut-pr-branch.sh:161`) and no `.workspace-divergence` `only` line, and
`git check-ignore` reports none of the planned paths as ignored.

## Shared Patterns

### Spawn-and-assert with the context in the message
**Source:** `tests/setup_repo.rs:128-137` `assert_exit`; `tests/gate.rs:18-36` `assert_refused`.
**Apply to:** every harness spawn helper, plus the provenance and hermeticity tests.
```rust
fn assert_exit(output: &Output, expected: i32, what: &str) -> (String, String) {
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(
        output.status.code(),
        Some(expected),
        "{what}: stdout {stdout:?}, stderr {stderr:?}"
    );
    (stdout, stderr)
}
```
Note that this uses `from_utf8_lossy`. Use it for rendering failure messages only, never for
observed outcome data (paths, messages), which stay raw bytes.

### Named exit-code constants
**Source:** `tests/gate.rs:7-8`, `tests/setup_repo.rs:17-20`, `tests/gate_controls.rs:28-34`.
```rust
/// Exit code for a refusal or, under `check`, drift.
const FAILURE_EXIT: i32 = 1;
```
**Apply to:** the expected exit codes in scenario outcomes, and nextest's `4` (no tests).

### Every check has a counterpart that must trip
**Source:** `tests/setup_repo.rs:220-232`, `tests/gate_controls.rs:330-346`,
`tests/repo_policy.rs:183-212`.
**Apply to:** provenance, hermeticity, coverage, the `validate()` rejections, and the guard controls.
Make the control a separate `#[test]`, with a comment that names what it rules out.

### Environment scoped to the child only
**Source:** `clippy.toml:19-22` bans `std::env::set_var` and `remove_var`;
`tests/repo_policy.rs:46-56` (`env_remove("RUSTUP_TOOLCHAIN")` on the child).
**Apply to:** all harness code. `WSX_ORACLE_DIR`, `LLVM_*` and the test-only "leaky" builder flag
all go through `Command::env`/`envs`. `std::env::vars_os()` is read-only and allowed.

### Scratch location
**Source:** `const TMP_ROOT: &str = env!("CARGO_TARGET_TMPDIR");` (`tests/setup_repo.rs:15`).
**Apply to:** the base directory for every `TempDir`. Never `/tmp` (CONTEXT lines 105-107), and never
under the checkout outside `target/`, because the container mounts the checkout read-only.

### Module doc header
**Source:** `tests/gate_controls.rs:1-18`, `tests/image_parity.rs:1-7`.
**Apply to:** every `tests/harness/*.rs` file. State what it proves, what isolates it, and why.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `tests/harness/{sync,worktree,cut,check,guard}.rs` | scenario modules | batch | The scenario DSL is new. Use RESEARCH Patterns 2-4 and the macro sketch at lines 742-758 |
| `scenarios!` / `harness_modules!` macros (in `scenario.rs`, `main.rs`) | macro registry | n/a | No `macro_rules!` exists in the repo. Use the RESEARCH sketch, which has not been compiled |
| `tests/oracle/oracle.patch` | patch fixture | transform | No patch files in the repo. Use RESEARCH lines 318-332 (4 substitutions, verified by research) |
| `tests/oracle/README.md` | provenance doc | n/a | No vendored code exists yet. Content: SHA, blob table (RESEARCH lines 245-252), MIT attribution, and what each patch hunk does |
| raw `git cat-file commit` parser (in `outcome.rs`) | parser | transform | Nothing in the repo parses git objects. Byte-level, `-z` everywhere (RESEARCH Pitfall 9) |
| SSH signing setup (in `fixture.rs`) | fixture step | process spawn | No signing in tests yet. Use RESEARCH lines 456-463 |

## Metadata

**Analog search scope:** `tests/*.rs` (all 7), `tests/fixtures/env-ban/`, `scripts/*.sh`,
`.config/nextest.toml`, `rust-toolchain.toml`, `Cargo.toml`, `deny.toml`, `clippy.toml`,
`.gitignore`, `.workspace-divergence`, `scripts/workspace/cut-pr-branch.sh:161`, `CONTRIBUTING.md:12-30`.
**Files scanned:** 18
**Verified this session:**
- every analog path is tracked;
- the devflow pin resolves, and its six blob SHAs equal RESEARCH's table;
- none of the planned paths is gitignored or matches a PR-cut forbidden prefix.

**Not verified:**
- that `tests/harness/main.rs` is auto-discovered (A2);
- the cargo-llvm-cov release binary's version string (A5);
- the llvm-tools component name for 1.99.1 (A6).

**Pattern extraction date:** 2026-10-26
