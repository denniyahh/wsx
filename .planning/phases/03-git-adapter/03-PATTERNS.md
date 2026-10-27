# Phase 3: Git adapter - Pattern Map

**Mapped:** 2026-10-26
**Files analyzed:** 11 (7 new or modified source/config files, 2 test-support units, 1 harness file, 1 planning file)
**Analogs found:** 10 / 11 (one partial: the gix reader has no in-repo analog; RESEARCH.md Patterns 1-3 are its source)

Every analog path below was checked with `git ls-files` (all tracked). No production Rust exists
besides `src/main.rs`, so most analogs are harness test code under `tests/`. Unit tests in
`src/git/` cannot import from `tests/harness/` (a separate crate): they **reproduce** these
patterns, they do not reuse them.

## Corrections to upstream inputs (read before planning)

1. **`tests/harness/registry.rs` does not change.** CONTEXT.md says "`tests/harness/registry.rs`
   gets the reftable `regression:` scenario". Registration actually happens in the
   `scenarios! { ... }` block of `tests/harness/check.rs` (lines 296-302). `tests/harness/main.rs`
   lines 36-55 (`harness_modules!`) collects every module's `registry()` into `all_scenarios()`
   automatically, and `registry::rust_sides_not_enabled` (registry.rs 141-166) then lists the new
   disabled side with no edit. Only `check.rs` changes.
2. **RESEARCH Assumption A3 is settled by reading (not yet by running).** `outcome::observe`
   reads refs through `git for-each-ref` (`tests/harness/outcome.rs` 314-321), and its file walk
   skips `.git` (outcome.rs 709-720). Nothing in the snapshot reads `.git/refs` from disk, so the
   oracle-side recording should work on reftable. The harness run during execution is still the
   proof.
3. **No `#[expect(...)]` attribute exists in any Rust file yet.** The convention is written down
   (clippy.toml 15-18, CLAUDE.md) but phase 3 is its first use. There is no code excerpt to copy;
   use RESEARCH Pitfall 1's `#![cfg_attr(not(test), expect(dead_code, reason = "..."))]` form.
4. **Harness signing proof checks presence, not validity.** `outcome.rs` 118 and 490 set
   `signed = true` when a `gpgsig`/`gpgsig-sha256` header exists. Criterion 1's unit test per
   RESEARCH asserts `%G?` = `G`, which also needs `gpg.ssh.allowedSignersFile`. Copy that config
   key from `enable_signing` (fixture.rs 347-352); the harness's presence-only check is not the
   model for the assertion.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/main.rs` (modify: `mod git;`) | entry point | request-response | itself (lines 1-25) | exact |
| `src/git/mod.rs` (new) | module root, error type, wsx-owned types | transform | `tests/harness/oracle.rs` 49-65 (`AdapterError` + `Display`) | role-match |
| `src/git/repo.rs` (new) | service: gix reader (discover, refusals, 4 reads) | file-I/O read, transform | none in repo; RESEARCH Patterns 1-3 | no analog (partial: error-string shape from `fixture.rs` 575-586) |
| `src/git/spawn.rs` (new) | service: `git` spawn helper | request-response (child process) | `tests/harness/env.rs` 133-181 (`hermetic_env`, `command`) + `tests/harness/fixture.rs` 427-471, 575-586 (`git`, `git_ok`, `checked`) | exact for the shape |
| `src/git/` test support (`#[cfg(test)]`; e.g. `src/git/test_support.rs` or `tests` submodule) | test utility: scratch dir, hermetic env, fixture builder, signing, re-exec, foreign-owner guard | file-I/O, batch | `tests/harness/env.rs` 27-125, `tests/harness/fixture.rs` 186-193, 297-365, 474-508; `tests/setup_repo.rs` 77-82 | role-match (must be reproduced, cannot be imported) |
| `#[cfg(test)] mod tests` in `repo.rs` and `spawn.rs` | test | differential, event-driven (hooks) | `tests/harness/env.rs` 284-347 (git-side ceiling test + control); `tests/harness/provenance.rs` 555-600 (foreign-owner via git switch) | role-match |
| `Cargo.toml` (+ `Cargo.lock`) | config | n/a | `Cargo.toml` 11-17 (justified `tempfile` dev-dep) | exact |
| `deny.toml` | config | n/a | `deny.toml` 13-22 | exact |
| `clippy.toml` | config | n/a | `clippy.toml` 15-22 | exact |
| `tests/harness/check.rs` (reftable scenario) | test scenario | request-response | same file: `self_test` 55-79, `no_gsd_repo` 232-258, helpers 15-47, block 296-302; `scenario.rs` 1494-1512 for `Class::Regression` | exact |
| `.planning/STATE.md` Blockers/Concerns (line 140) | planning doc | n/a | the existing section itself | exact |

Not changed (checked): `.config/nextest.toml`, `scripts/check.sh`, `tests/harness/registry.rs`,
`tests/harness/main.rs`.

## Pattern Assignments

### `src/main.rs` (entry point, modify)

**Analog:** itself, lines 1-25. Only add `mod git;` after the `use` block (line 8). Keep the crate
doc (lines 1-6), which already says exit codes become a contract in phase 5; do not add exit codes
for refusals (deferred to phase 5).

```rust
use std::process::ExitCode;

/// Exit code for input the CLI does not accept.
const EXIT_USAGE: u8 = 2;
```
Style to copy: named constants with `///` docs for magic numbers (the version gate's `(2, 54)`
should be a named constant the same way).

---

### `src/git/mod.rs` (module root, error enum, wsx types)

**Analog:** `tests/harness/oracle.rs` lines 49-65: a hand-written error enum with doc comments per
variant and a manual `Display`. No error crate is used anywhere in the repo.

```rust
/// Why an adapter produced no run.
#[derive(Clone, Debug)]
pub enum AdapterError {
    NotImplemented(NotImplemented),
    /// The process could not be started or fed its input.
    Spawn(String),
}

impl fmt::Display for AdapterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AdapterError::NotImplemented(NotImplemented { action, why }) => {
                write!(f, "not implemented: {action}: {why}")
            }
            AdapterError::Spawn(reason) => write!(f, "{reason}"),
        }
    }
}
```
Apply: variants `Reftable`, `ForeignOwner { path }`, `GitTooOld { found, required }`,
`NotARepo { .. }`, `GitFailed { argv, status, stderr }` (CONTEXT "Code shape"). Each `Display`
arm names its cause (CONTEXT discretion). `ForeignOwner` prints git's own fix:
`git config --global --add safe.directory <path>`.

**Module-level lint (first use in the repo, no excerpt exists):** per RESEARCH Pitfall 1,
`#![cfg_attr(not(test), expect(dead_code, reason = "no caller until phase 5's wsx check"))]`.
Consequence the planner must carry: under `cfg(test)` every item must be used by some test, or
`dead_code` fires under `clippy --all-targets -D warnings` (scripts/check.sh 154-161).

**Doc-comment style** (`tests/harness/env.rs` 1-17): a `//!` module header that states the
invariant and why (for example "`command` is the only way the harness builds a child: it clears
the environment first, so nothing from the host ... reaches git"). Copy that style for
`src/git/mod.rs`, stating the single-entry-point and the "gix types never leave the module" rules.

---

### `src/git/repo.rs` (gix reader: single entry point, refusals, four reads)

**Analog:** none in the repository (no gix code yet). Use RESEARCH.md Pattern 1 (lines 257-277),
Pattern 2 (281-290), Pattern 3 (295-301). In-repo conventions to apply:

- **Canonicalise once, at construction** (`tests/harness/env.rs` 89-96):
  ```rust
  let root = temp
      .path()
      .canonicalize()
      .map_err(|err| format!("canonicalize {}: {err}", temp.path().display()))?;
  let ceiling = root
      .parent()
      .ok_or_else(|| format!("{} has no parent", root.display()))?
      .to_path_buf();
  ```
  Apply to worktree, git dir and common dir (`common_dir()` is `<git-dir>/../..`, RESEARCH
  Pitfall 8). Map the io error into the wsx `Error`, not a `String`.
- **Raw-byte paths** (`tests/harness/fixture.rs` 474-481 uses `OsStr::from_bytes(path)` with
  `std::os::unix::ffi::OsStrExt`): paths leave the module as `Vec<u8>`/`BString`-free wsx types.
- **The sanctioned open site** carries
  `#[expect(clippy::disallowed_methods, reason = "the single sanctioned way wsx opens a repository")]`
  on the smallest enclosing item (clippy.toml 15-18 wording: "on its smallest enclosing ... item").
- **Ban-list pin** (RESEARCH Pitfall 2): a `#[cfg(test)]` function calling each of the 13 banned
  entry points under one reasoned `expect`, so a rename fails to compile and a mis-spelled path
  leaves the expect unfulfilled.
- **Ceiling refusal after discovery** (CONTEXT "Decided after research"): refuse as not-a-repo
  where git would. The git-side test shape and its control already exist; see the test section
  below (`env.rs` 307-347).

---

### `src/git/spawn.rs` (git spawn helper)

**Analog:** `tests/harness/env.rs` 133-181 and `tests/harness/fixture.rs` 427-471, 575-586.

**Env as an explicit snapshot, built by a pure function** (env.rs 53-54, 127-131, 136-167):
```rust
/// An environment for one child, as ordered key/value pairs.
pub type Env = Vec<(OsString, OsString)>;

/// The environment of the test process itself. Read-only: nothing here mutates
/// the process environment.
pub fn inherited_env() -> Env {
    std::env::vars_os().collect()
}
```
Apply (RESEARCH Pattern 4): the helper owns a `Vec<(OsString, OsString)>`; production captures it
with `std::env::vars_os()` once, tests pass a hermetic one. Removal of the `--local-env-vars` list
is a filter over that snapshot, applied only when `canonical(target) != own_worktree`.

**The single child constructor** (env.rs 169-181):
```rust
pub fn command(program: impl AsRef<OsStr>, cwd: &Path, env: &Env) -> Command {
    let mut command = Command::new(program);
    command
        .env_clear()
        .envs(env.iter().map(|(key, value)| (key, value)))
        .current_dir(cwd)
        .stdin(Stdio::null());
    command
}
```
This is exactly GIT-02's per-child shape: `env_clear` + filtered `envs`, explicit `current_dir`,
null stdin. Note from env.rs 170-172: because `PATH` is set on the child, a bare `"git"` resolves
through the child's PATH. That is what lets `old_git_refused` put a fake `git` first on the
snapshot's PATH.

**Argv collection and the failure carrying argv, exit status and stderr** (fixture.rs 447-454,
575-586):
```rust
let args: Vec<OsString> = args
    .into_iter()
    .map(|arg| arg.as_ref().to_os_string())
    .collect();
let output = command("git", &self.checkout(checkout), env)
    .args(&args)
    .output()
    .map_err(|err| format!("spawn git {} in {checkout}: {err}", render(&args)))?;
```
```rust
fn checked(checkout: &str, args: &[OsString], output: Output) -> Result<Vec<u8>, String> {
    if output.status.code() == Some(0) {
        Ok(output.stdout)
    } else {
        Err(format!(
            "git {} in {checkout} exited {:?}; stderr: {}",
            render(args),
            output.status.code(),
            String::from_utf8_lossy(&output.stderr).trim_end()
        ))
    }
}
```
Apply: the same fields go into `Error::GitFailed { argv, status, stderr }` (typed, not a
`String`). `generic I: IntoIterator<Item = S>, S: AsRef<OsStr>` signature (fixture.rs 428-432)
is the house style for argv parameters. Args via `Command::args`, never `sh -c` (RESEARCH threat
table).

**Once-per-process state** (version gate, `--local-env-vars` list): no analog
(`OnceLock`/`LazyLock` is used nowhere in the repo). Use `std::sync::OnceLock` per RESEARCH
diagram.

**The local-env-vars precedent** (the same list, shell form): `tests/oracle/hooks/pre-push`
line 34, `unset $(git rev-parse --local-env-vars)`, mirrored in
`tests/harness/hermeticity.rs` 427-430.

---

### `src/git/` test support (`#[cfg(test)]`)

The unit tests cannot import `tests/harness/*`. Reproduce these pieces, adapted:

**Scratch dir: TempDir with an attributable prefix, canonical root, ceiling = parent**
(env.rs 27-28, 82-96):
```rust
/// Where every scenario TempDir is created: on disk, inside `target/`.
const TMP_ROOT: &str = env!("CARGO_TARGET_TMPDIR");
...
let temp = TempDir::with_prefix_in(prefix, parent).map_err(|err| {
    format!(
        "create a TempDir {prefix}* under {}: {err}",
        parent.display()
    )
})?;
```
Difference: `CARGO_TARGET_TMPDIR` does not exist for a binary crate's unit tests. Replace the
`env!` with RESEARCH Pitfall 9's resolver: `current_exe()` → parent named `deps` (assert, fail
otherwise) → `.parent().parent().join("tmp")` → `create_dir_all` → `TempDir::with_prefix_in`.
Keep the prefix idea (e.g. `git-unit-<name>-`) so a leftover is attributable.

**Hermetic env** (env.rs 30-51, 136-167): copy the key list (HOME, `GIT_CONFIG_GLOBAL`,
`GIT_CONFIG_SYSTEM`, `GIT_CONFIG_NOSYSTEM=1`, XDG dirs, `GIT_CEILING_DIRECTORIES` = root's parent,
`GIT_TERMINAL_PROMPT=0`, `GIT_EDITOR=false`, `LC_ALL=C`, `TZ=UTC`,
`PATH=/usr/local/bin:/usr/bin:/bin`) and the global config text (user, `init.defaultBranch`,
`commit.gpgsign = false`). Two lines that are easy to drop and must not be:
```rust
env.extend(
    inherited
        .iter()
        .filter(|(key, _)| key.as_bytes().starts_with(b"LLVM_"))
        .cloned(),
);
```
Without the `LLVM_` pass-through, `scripts/check.sh coverage` loses every re-exec child. Prefer
this module over `tests/setup_repo.rs` 28-46 `hermetic()`, which prepends the user's PATH (host
linuxbrew git) and is less strict.

**Fixed dates** (fixture.rs 186-193), needed for the clock-skewed criss-cross fixture:
```rust
pub fn dated(env: &Env, epoch: i64) -> Env {
    let date = OsString::from(format!("@{epoch} +0000"));
    let mut env = env.clone();
    env.push(("GIT_AUTHOR_DATE".into(), date.clone()));
    env.push(("GIT_COMMITTER_DATE".into(), date));
    env
}
```
Skew = pass an epoch older than the parent's for one commit.

**Write + commit helpers** (fixture.rs 474-508): `write` creates parent dirs and takes raw bytes;
`commit` stages `--all` and commits with `--cleanup=verbatim --file <msg>` at the next tick.

**Throwaway SSH signing key** (fixture.rs 297-365, `enable_signing`): `ssh-keygen -q -t ed25519
-N "" -C harness -f <root>/signkey` through the hermetic command; parse `signkey.pub` into
`<email> namespaces="git" <kind> <base64>` for `allowed_signers`; set local config:
```rust
let signing_config = [
    ("gpg.format", OsStr::new("ssh")),
    ("user.signingkey", private_key.as_os_str()),
    ("commit.gpgsign", OsStr::new("true")),
    ("gpg.ssh.allowedSignersFile", allowed_path.as_os_str()),
];
```

**Executable stub/hook files** (tests/setup_repo.rs 77-82; gate_controls.rs 276-278):
```rust
let stub = self.dir.join("bin/gitleaks");
fs::write(&stub, "#!/bin/sh\nexit 0\n").expect("write gitleaks stub");
fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).expect("chmod stub");
```
Apply to: the fake old `git` (`echo "git version 2.53.0"`), the hook scripts that `exec` the test
binary, and the hook-log hooks for the signing test.

**Re-exec child and foreign-owner `Drop` guard:** no analog. No test in the repo re-executes
`current_exe()` (integration tests use `env!("CARGO_BIN_EXE_wsx")`, `tests/cli.rs` 8-10), and no
`impl Drop` exists. Use RESEARCH Pattern 5 and the foreign-owner mechanism (lines 389-396). The
`#[ignore = "<reason>"]` form for the child half has an in-repo precedent in the `scenarios!`
macro (`tests/harness/scenario.rs` 672-677):
```rust
(@rust rust $why:literal) => {
    #[test]
    #[ignore = $why]
    fn rust() { ... }
};
```

---

### Tests in `src/git/repo.rs` and `src/git/spawn.rs`

**Subject plus a control that must flip the answer, as two named tests**
(`tests/harness/env.rs` 307-347):
```rust
/// git run in the scenario root does not find a repository that encloses the
/// root: the ceiling stops discovery before it leaves the root.
#[test]
fn git_in_the_root_finds_no_enclosing_repository() {
    let (_host, nested) = nested_in_a_repository("ceiling_at_parent");
    let output = toplevel_from_root(&nested, &hermetic_env(&[], &nested));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(128), "stderr: {stderr}");
    ...
}

/// The counterpart: with the ceiling at the root itself, git run in the root
/// finds the enclosing repository, so the test above can fail.
#[test]
fn a_ceiling_at_the_root_lets_git_in_the_root_find_the_enclosing_repository() {
    let (_host, nested) = nested_in_a_repository("ceiling_at_root");
    let env: Env = hermetic_env(&[], &nested)
        .into_iter()
        .map(|(key, value)| match key.to_str() {
            Some("GIT_CEILING_DIRECTORIES") => (key, nested.root.clone().into_os_string()),
            _ => (key, value),
        })
        .collect();
```
Apply: the ceiling-match test (wsx vs git, with the control opening from below the ceiling), and
the "override one key of the hermetic env" idiom for every control (removal disabled, inherited
stdin, `safe.directory` present). Test names describe behaviour, with no review-round wording.
One test per behaviour; gate each with `scripts/check.sh test -E 'test(=git::<mod>::tests::<name>)'`.

**Foreign owner, git side only** (`tests/harness/provenance.rs` 559-580, 586-600):
```rust
let output = command("git", Path::new(root), &env)
    .envs(extra_env.iter().copied())
    .arg("-c")
    .arg(format!("safe.directory={root}"))
    .args(["ls-files", "-s", "--", "tests/oracle"])
```
```rust
let listing = oracle_index_listing_with(&[("GIT_TEST_ASSUME_DIFFERENT_OWNER", "1")]);
```
Caution for the planner: this is the git-only switch. Per CONTEXT it may be used only on the git
side; and `-c safe.directory` is ignored by gix (RESEARCH Pitfall 5), so wsx's control must put
`safe.directory` in the child's `GIT_CONFIG_GLOBAL` file instead.

**Assertion message style** (`tests/cli.rs` 22-29): every `assert_eq!`/`assert!` carries a
message naming what was checked and printing stdout/stderr.

---

### `Cargo.toml` (add gix)

**Analog:** `Cargo.toml` lines 11-17, the justified dev-dependency:
```toml
[dev-dependencies]
# Scenario tests build scratch git trees. tempfile's TempDir removes them even
# when a test panics, and `TempDir::new_in` keeps them under
# CARGO_TARGET_TMPDIR rather than the RAM-backed /tmp. Dev-only, so nothing
# ships in the binary; MIT OR Apache-2.0, and every crate in its graph offers
# MIT. Legitimacy checked in phase 2 research (Package Legitimacy Audit: OK).
tempfile = "3.27.0"
```
Apply under the empty `[dependencies]` (line 9): a comment stating why (reads without C deps), the
exact pin and feature set, why `blob-diff`, `interrupt` and network features are off, the license
consequence, and the legitimacy reference (03-RESEARCH Package Legitimacy Audit). Value:
`gix = { version = "=0.90.0", default-features = false, features = ["sha1", "sha256", "revision"] }`.
`Cargo.lock` changes in the same commit: every gate runs with `--locked` (scripts/check.sh 25-32),
so a stale lock fails the gate.

---

### `deny.toml` (licenses)

**Analog:** `deny.toml` lines 13-22:
```toml
[licenses]
# Only the licenses the crate graph actually uses: unused allowances only warn
# (license-not-encountered), so later phases extend this list as each crate
# arrives, not ahead of it.
allow = ["MIT"]
```
Apply: `allow = ["Apache-2.0", "MIT", "Unicode-3.0", "Zlib"]`, sorted, with a comment naming which
crates need each (RESEARCH: `unicode-bom` Apache-2.0; `unicode-ident`/`unicode-normalization`
Unicode-3.0; `foldhash`/`zlib-rs` Zlib). `include-dev = true` (line 22) stays, so `tempfile` is
still checked. `[graph] all-features = true` (line 6) applies to wsx's own features, of which there
are none; it does not switch on gix's optional features. Verify with `scripts/check.sh deps`.

---

### `clippy.toml` (gix open/discover ban)

**Analog:** `clippy.toml` lines 15-22:
```toml
# A justified exception carries `#[expect(clippy::disallowed_methods, reason =
# "...")]` on its smallest enclosing test-only item. `expect` rather than
# `allow`: an expectation that stops being needed becomes a warning, so the
# exception list cannot silently outlive its reason.
disallowed-methods = [
  { path = "std::env::set_var", reason = "process-global and racy ...; scope it to the child with Command::env instead. ..." },
  { path = "std::env::remove_var", reason = "..." },
]
```
Apply: append the 13 gix entry points from RESEARCH Pitfall 2, each with a `reason` naming the
sanctioned alternative (`ThreadSafeRepository::discover_with_environment_overrides_opts` in
`src/git/repo.rs`). Add a header comment for the new group (why: hook env must be honoured once,
siblings only via `main_repo()`/`worktrees()`). The existing comment says exceptions are
"test-only"; the gix sanctioned site is production code, so the comment needs one sentence
covering it.

---

### `tests/harness/check.rs` (reftable `regression:` scenario)

**Analog:** the same file. Helpers to reuse (lines 15-47):
```rust
/// No fixture steps beyond the preset.
fn no_steps(_: &mut Fixture) -> Result<(), String> {
    Ok(())
}

/// The check in `work`: it reads and writes nothing in the repository.
fn check_in_work(config: RepoConfig, fixture: FixtureFn) -> Case { ... }

/// The check's verdict with the repository left as it was.
fn verdict(exit: i32) -> Outcome { ... }
```
**Fixture-step shape** (lines 15-20, `add_note`): a `fn(&mut Fixture) -> Result<(), String>` with a
`///` doc. The new `to_reftable` step calls
`fixture.git_ok(WORK, ["refs", "migrate", "--ref-format=reftable"]).map(|_| ())`.
(The devflow preset's `work` has no linked worktrees, which `refs migrate` would refuse.)

**Scenario with a recorded oracle side and both guard controls** (lines 232-258, `no_gsd_repo`):
```rust
Scenario::new(
    "no_gsd_repo",
    check_in_work(RepoConfig::no_gsd(), no_steps)
        .expect(verdict(0))
        .class(Class::Fixed {
            defect: NO_GSD_DEFECT,
        })
        .oracle(OracleSide::Recorded(verdict(1)))
        ...
)
.control(
    Role::KnownPositive,
    check_in_work(RepoConfig::no_gsd(), add_scratch)
        .expect(verdict(1))
        .oracle_stderr(b"undeclared workspace-only path: scratch/y.md (declare: only scratch/y.md)\n"),
)
.control(
    Role::Clean,
    check_in_work(RepoConfig::devflow(), no_steps).expect(verdict(0)),
)
```
Apply: subject `check_in_work(RepoConfig::devflow(), to_reftable).expect(verdict(1))
.class(Class::Regression { reason: "..." }).oracle(OracleSide::Recorded(verdict(<recorded>)))`;
`Role::KnownPositive` = `self_test`'s (lines 69-74: `RepoConfig::devflow()`, `add_note`,
`verdict(1)`, `.oracle_stderr(UNDECLARED_NOTE)`); `Role::Clean` = devflow, `no_steps`,
`verdict(0)` (the files-backend control). The `Class::Regression` usage form is in
`tests/harness/scenario.rs` 1494-1512 (`.class(Class::Regression { reason: REGRESSION })`).

**Rules this must satisfy** (`tests/harness/scenario.rs`):
- 36-44: `Regression { reason: &'static str }`.
- 362-365: `Action::Check` is a guard, so a `KnownPositive` and a `Clean` control are required.
- 386-389 + 405-409: a `Regression` expectation must be a refusal (`exit != 0`, refs unchanged,
  no new commit); `verdict(1)` qualifies (exit 1 is a placeholder until IF-01, RESEARCH Open
  Question 3).
- 538: the oracle side of a `Regression` case is judged against its `Recorded` outcome.
- The `rust_stderr` requirement fires only once the Rust side is enabled (phase 5).

**Registration** (lines 296-302): append one line in the same form:
```rust
scenarios! {
    self_test: oracle, rust = "Rust side not enabled until phase 5";
    ...
    no_gsd_custom_personal_path: rust_only, rust = "Rust side not enabled until phase 5";
}
```
`reftable_refused: oracle, rust = "Rust side not enabled until phase 5";`. Generated test names:
`check::reftable_refused::oracle` and the ignored `check::reftable_refused::rust`
(scenario.rs 660-697). Update the module doc (check.rs line 1, "plans 02-07 and 02-13") to name
the phase 3 plan.

---

### `.planning/STATE.md` (Blockers/Concerns, line 140)

**Analog:** the section itself. Add the phase 5 obligation: enable
`check::reftable_refused`'s Rust side and add a `rust_stderr` naming reftable, and replace the
placeholder exit 1 with the IF-01 code. Writing this is authored content, not a GSD-artifact fix.

## Shared Patterns

### Hermetic child process (every git spawn in tests and in production)
**Source:** `tests/harness/env.rs` 169-181 (`command`), 136-167 (`hermetic_env`).
**Apply to:** `spawn.rs` (production shape: `env_clear` + snapshot + null stdin + `current_dir`),
every unit-test git invocation, every re-exec child.

### Errors carry the command, exit code and stderr
**Source:** `tests/harness/fixture.rs` 575-586 (`checked`); `map_err(|err| format!("<verb> {}:
{err}", path.display()))` everywhere in env.rs/fixture.rs.
**Apply to:** `Error::GitFailed`, `NotARepo` (include the gix message), and every test-support
helper (they return `Result<_, String>` and tests `.expect("...")` them).

### No process-global env mutation
**Source:** `clippy.toml` 1-22; CLAUDE.md. **Apply to:** all tests; env-dependent subjects (hook,
trust, ceiling, safe.directory, null stdin) go through the re-exec child, never `set_var`.

### Subject + negative control as separate named tests
**Source:** `tests/harness/env.rs` 307-347; check.rs controls. **Apply to:** each read query
(answer-changing control), leak (removal disabled), null stdin (inherited), trust
(`safe.directory`), reftable (files backend), version gate (2.54 passes, 2.53 refused), signing
(explicit `--no-verify -c commit.gpgsign=false` gives `N`, no hook lines).

### Scratch on disk, never `/tmp`
**Source:** `tests/harness/env.rs` 1-11, 27-28. **Apply to:** all unit-test fixtures through the
`deps/../../tmp` resolver, with `GIT_CEILING_DIRECTORIES` at the scratch root's parent because
`target/` sits inside the wsx checkout (env.rs 8-11).

## No Analog Found

| File / piece | Role | Data Flow | Reason |
|---|---|---|---|
| `src/git/repo.rs` gix calls | service | file-I/O read | no gix code in the repo; use RESEARCH Patterns 1-3 (executed on gix 0.90.0) |
| Re-exec of the unit-test binary (`current_exe()` + `--exact ... --include-ignored --nocapture`, marker line) | test utility | event-driven | integration tests use `CARGO_BIN_EXE_wsx`; nothing re-executes a test binary. Use RESEARCH Pattern 5, including its trap (wrong name exits 0) |
| Foreign-owner `Drop` guard (`chown` as root, `/usr/bin/unshare ... chown -R 1:1` otherwise, restore `0:0` on drop) | test utility | file-I/O | no `impl Drop` in the repo; use RESEARCH lines 389-396; fail, never skip |
| `OnceLock` for the version gate and the removal list | service state | n/a | no once-per-process state exists yet |
| Module-level `#![cfg_attr(not(test), expect(dead_code, ...))]` | lint | n/a | first `#[expect]` in any Rust file; RESEARCH Pitfall 1 |

## Metadata

**Analog search scope:** `src/`, `tests/*.rs`, `tests/harness/*.rs`, `tests/oracle/hooks/`,
`Cargo.toml`, `deny.toml`, `clippy.toml`, `.config/nextest.toml`, `scripts/check.sh`,
`.planning/STATE.md`.
**Files scanned:** 22 (18 analogs confirmed tracked with `git ls-files`).
**Pattern extraction date:** 2026-10-26
