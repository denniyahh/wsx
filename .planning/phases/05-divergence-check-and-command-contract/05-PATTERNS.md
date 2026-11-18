# Phase 5: Divergence check and command contract - Pattern Map

**Mapped:** 2026-11-18
**Files analyzed:** 19 (new and modified)
**Analogs found:** 17 / 19 (every analog path below is git-tracked; `docs/` does not exist yet)

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `src/main.rs` (modify) | entry point | request-response | itself (`src/main.rs:1-28`) | exact |
| `src/cli/mod.rs` (new) | controller (argv parse, dispatch) | request-response | `src/main.rs:16-28` | role-match |
| `src/cli/commands.rs` (new, data-only) | config table | static data | `tests/harness/provenance.rs:28-60` (`VENDORED_BLOBS` const table) | partial |
| `src/exit.rs` (new) | utility (error classifier) | transform | `src/git/mod.rs:203-216` (`is_replacement_refusal`) + `src/model/mod.rs:114-130` (`require`) | role-match |
| `src/json.rs` (new) | utility (serializer, D-09 bytes) | transform | `src/model/mod.rs:182-203` (`shown`, `command_word`, `listed`) + `src/git/quote.rs:1-13` | partial |
| `src/check/mod.rs` (new) | service (pure evaluator) | transform | `src/model/mod.rs:727-745` (`personal_paths`, `is_personal`) + `src/model/manifest.rs:313-318` (`covers`) | role-match |
| `src/check/hooks.rs` (new) | service (hooks rule, symlink resolution) | transform | `src/model/manifest.rs:261-284` (`normalize_path`) | partial |
| `src/git/<new reads>` (tree lookup `-Z`, `ls-tree -r -z`, reflog time, `core.hooksPath`, FETCH_HEAD mtime) | service (git adapter read) | request-response (child process) | `src/git/objects.rs:252-318` (`read_tips`), `src/git/config.rs:44-116` (`read_config`) | exact |
| `src/git/mod.rs` (modify: drop module `dead_code`, re-exports) | module root | — | itself (`src/git/mod.rs:19-40`) | exact |
| `src/model/mod.rs` (modify: drop module `dead_code`) | module root | — | itself (`src/model/mod.rs:12-15`) | exact |
| `Cargo.toml` (modify: serde_json, lexopt; dev jsonschema, proc-macro2) | config | — | itself (`Cargo.toml` `[dependencies]` comment-per-dep blocks) | exact |
| `docs/exit-codes.md` (new) | doc | — | none | no analog |
| `docs/json/check.v1.schema.json` (+ `error.v1.schema.json`) (new) | schema | — | none | no analog |
| `tests/contract.rs` (new) | test (integration, binary + schema) | request-response | `tests/cli.rs:1-52` + `tests/harness/provenance.rs:103-143` (hash pin) | role-match |
| `tests/if02_scan.rs` (new) | test (source scan + planted control) | file-I/O | `tests/image_parity.rs:1-90` (scratch copy under `CARGO_TARGET_TMPDIR`) + `tests/harness/fixture.rs:720-740` (no-agent-files scan) | role-match |
| `tests/harness/check.rs` (modify: enable Rust sides, new DIV scenarios) | test (scenario) | event-driven (fixture -> action -> outcome) | itself (`tests/harness/check.rs:1-355`) | exact |
| `tests/harness/rust.rs` (modify only if argv changes) | test adapter | request-response | itself (`tests/harness/rust.rs:20-65`) | exact |
| `tests/cli.rs` (modify: usage text) | test (CLI boundary) | request-response | itself | exact |
| `tests/harness/registry.rs` / D-15 no-agent-files run | test | batch | `tests/harness/check.rs:265-294` (`rust_only_no_gsd_check`) + `RepoConfig::no_gsd()` (`tests/harness/fixture.rs:126`) | role-match |

## Pattern Assignments

### `src/main.rs` (entry point) and `src/cli/mod.rs` (controller)

**Analog:** `src/main.rs` (whole file, 28 lines)

Module-doc + `mod` list + named exit constant + `ExitCode` return (lines 1-28):
```rust
//! wsx: keeps personal files off shared branches while moving work between a
//! workspace branch and its worktrees.
//! ... Exit codes become a versioned contract in Phase 5 (IF-01) ...
use std::process::ExitCode;

mod git;
mod model;

/// Exit code for input the CLI does not accept.
const EXIT_USAGE: u8 = 2;

fn main() -> ExitCode {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    match args.as_slice() {
        [flag] if flag == "--version" => {
            println!("wsx {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("usage: wsx --version");
            ExitCode::from(EXIT_USAGE)
        }
    }
}
```
Keep: `args_os` (non-UTF-8 argv), `ExitCode::from(u8)`, `--version` outside the command table (RESEARCH Open Question 3). The module doc's "until then" sentence must be rewritten. `tests/cli.rs:36-52` pins `stderr.contains("usage: wsx --version")` and exit 2: either keep that substring in the new usage text or update the test in the same commit.

Repo/model entry the dispatcher calls (read order, STATE obligation): `git::Repo::discover` (`src/git/repo.rs:39`), then `model::resolve(repo, git, recorded)` (`src/model/mod.rs:372`) before any other read, then `RepoModel::require()` (`src/model/mod.rs:114-130`) unless `check` needs the `Configured`/`NotConfigured` distinction for `configured: false` (D-04): then match on `RepoModel` as the `require` doc says (lines 111-113: "a caller that must tell the states apart ... matches on `RepoModel`").

---

### `src/cli/commands.rs` (data-only table, `#[path]`-included by tests)

**Analog:** `tests/harness/provenance.rs:28-60` — a doc-commented `const` array of tuples consumed by several functions.
```rust
/// Every vendored file: its path under `tests/oracle`, its path in devflow,
/// its mode there and its blob at `DEVFLOW_COMMIT`, ...
const VENDORED_BLOBS: [(&str, &str, &str, &str); 6] = [
    (
        "sync-workspace.sh",
        ...
```
Shape for the new file: `pub struct CommandSpec { pub name: &'static str, pub schema_version: u32 }` and `pub const COMMANDS: &[CommandSpec]`, no `crate::` import, so `#[path = "../src/cli/commands.rs"] mod commands;` compiles in `tests/contract.rs` and the harness. Test-crate `mod` declarations live in `tests/harness/main.rs:21-32`; add the `#[path]` module there. Under the binary, unused fields trip `dead_code`: use item-level `expect` as in Shared Patterns.

---

### `src/exit.rs` (classifier, transform)

**Analog A:** `src/git/mod.rs:203-216` — the one-list helper every caller reads:
```rust
impl Error {
    /// Whether this is one of the refusals of the replace refs: ...
    /// Every caller that treats those apart from other failures reads this
    /// one list.
    pub fn is_replacement_refusal(&self) -> bool {
        matches!(
            self,
            Error::ReplaceRefsPresent { .. }
                | Error::ReplaceBaseSet { .. }
                | Error::ReplaceRefsUnreadable { .. }
        )
    }
}
```
**Analog B:** `src/model/mod.rs:114-130` — wrapping into `NotConfigured { reason: Box<Error> }`, which `exit.rs` must look through (a replacement refusal inside `reason`, `Error::Git(inner)`, or `ManifestUnreadable { source }` takes class 5):
```rust
RepoModel::NotConfigured { inferred } => inferred.map_err(|err| match err {
    err @ (Error::ManifestMissing { .. } | Error::ManifestNotABlob { .. }
    | Error::ManifestUnreadable { .. } | Error::ManifestInvalid { .. }) => Error::NotConfigured {
        reason: Box::new(err),
    },
    Error::Git(inner) if inner.is_replacement_refusal() => Error::NotConfigured {
        reason: Box::new(Error::Git(inner)),
    },
    other => other,
}),
```
Variant lists to match exhaustively (no `_` arm, D-07): `git::Error` at `src/git/mod.rs:103-201` (20 variants), `model::Error` at `src/model/mod.rs:135-174` (16 variants). Exhaustive `match self { Error::X { .. } => ... }` style is already used by both `impl fmt::Display` blocks (`src/git/mod.rs:218-...`, `src/model/mod.rs:205-...`); copy their arm style. The `error.kind` snake_case name comes from the same match (RESEARCH Pattern 1). Use `{ .. }` on every arm so field additions do not break the classifier, only new variants.

---

### `src/json.rs` (serializer, D-08/D-09)

**Analog:** `src/model/mod.rs:182-203` — small private per-purpose rendering helpers for raw bytes, each with a doc saying why:
```rust
/// `bytes` for prose in a message: git's C-style quoting, so a control byte
/// from a blob or a config value never reaches the terminal raw.
fn shown(bytes: &[u8]) -> String {
    String::from_utf8_lossy(&git::c_quote(bytes)).into_owned()
}
```
`shown` is for **stderr prose only** (lossy is fine after C-quoting); JSON byte fields must never go through it (D-09: never lossy). Use RESEARCH's `bytes_value` (UTF-8 string else `{"hex": ...}`). Human report paths reuse `git::c_quote` (`src/git/quote.rs:11-13`, re-exported `src/git/mod.rs:38`).

---

### `src/check/mod.rs` (pure evaluator) and `src/check/hooks.rs`

**Analog A:** `src/model/mod.rs:727-745` — built-ins first, then manifest entries filtered by kind:
```rust
pub fn personal_paths(&self) -> Vec<&[u8]> {
    let mut paths = vec![MANIFEST_NAME, self.worktree_dir.as_slice()];
    paths.extend(
        self.manifest.entries.iter()
            .filter(|entry| entry.kind == EntryKind::Only)
            .map(|entry| entry.path.as_slice()),
    );
    paths
}
```
The two built-ins are `[MANIFEST_NAME, model.worktree_dir]` (constants at `src/model/mod.rs:47-51`); do not hard-code `.wsx.conf`/`.worktrees`.

**Analog B:** `src/model/manifest.rs:313-318` — the only coverage rule (literal, no glob):
```rust
pub fn covers(entry: &[u8], path: &[u8]) -> bool {
    path.strip_prefix(entry)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with(b"/"))
}
```
Inputs: `Model` fields (`src/model/mod.rs:53-82`: `workspace_tip`, `integration.{branch, full_ref, tip}`, `base_remote`, `worktree_dir`, `manifest`), `Entry { kind, path, line, text }` (`src/model/manifest.rs:18-38`), and `ChangeKind::{Added, Deleted, Modified, TypeChanged}` from `Repo::tree_diff(&from, &to)` (`src/git/reads.rs:27-39`, `:118-123`; `from` = integration tip so `Added` = workspace-only). `tree_diff` output is already sorted by path bytes then kind.

**hooks.rs analog:** `src/model/manifest.rs:261-284` (`normalize_path`) — reuse it directly for the `core.hooksPath` value after stripping one `./`; copy its component loop shape (`split(|byte| *byte == b'/')`, match `b"" | b"." | b".."`) for symlink-target resolution within the tree.

Unit tests: `#[cfg(test)] mod tests` at the bottom of the file, using `crate::git::testing` helpers (`Scratch`, `git_ok`, `commit_all`, `object_id`, `T0`, `TICK`; `src/git/testing.rs:35-336`), as `src/git/objects.rs:320-325` imports them.

---

### `src/git/` new reads (tree lookup, hooks listing, reflog, hooksPath, FETCH_HEAD)

**Analog A (batch over stdin, by id, `-Z`):** `src/git/objects.rs:252-318` `Repo::read_tips`
```rust
let argv: Vec<OsString> = [NO_LAZY_FETCH, "cat-file", BATCH_FORMAT]
    .into_iter().map(OsString::from).collect();
let cwd = self.worktree().unwrap_or(self.git_dir());
let stdout = git.run_with_input(cwd, &argv, &input)?;
let answers = parse_batch(&stdout, 4).map_err(|problem| Error::GitBadOutput {
    argv: argv.clone(),
    problem,
})?;
```
Copy: `NO_LAZY_FETCH`, `cwd` choice, a separate pure `parse_*` fn returning `Result<_, String>` mapped to `GitBadOutput`, a count check on answers. New: `--batch-check=... -Z` (NUL framing; RESEARCH Pitfall 3) instead of the LF framing guarded by `debug_assert!(!name.contains(&b'\n'))` at line 280. Parser unit tests follow `src/git/objects.rs:342-404` (`parse_batch_refuses_a_short_read`, `..._an_abbreviated_id`, etc.).

**Analog B (git exit 1 = "unset", not failure):** `src/git/config.rs:71-104` `read_config`
```rust
let output = git.run_without_var(cwd, READ_ARGS, "GIT_CONFIG")?;
let nothing_matched =
    output.status.code() == Some(1) && output.stderr.is_empty() && output.stdout.is_empty();
if nothing_matched {
    return Ok(ConfigRead::default());
}
if !output.status.success() {
    return Err(Error::GitFailed {
        argv: READ_ARGS.iter().map(OsString::from).collect(),
        status: output.status.code(),
        stderr: output.stderr,
    });
}
```
Use for `git config --get core.hooksPath` (exit 1 + empty = unset). Note `run_without_var` exists for one child only (`src/git/spawn.rs:472-479` doc: "No other child loses a variable its target would keep"); a hooksPath read needs a whole-`Output` runner too, so either add a sibling to `run_without_var` or justify reusing it. Argv as a named `const [&str; N]` (`src/git/config.rs:107-116`).

**Analog C (plain read):** `Git::run(worktree, args) -> Result<Vec<u8>, Error>` (`src/git/spawn.rs:451-470`) for `ls-tree -r -z <tree-id>` and `reflog show -n1 --date=unix --format=%gd <full-ref>`.

**Analog D (fs read as `Error::Io`):** `src/git/config.rs:90-99` for FETCH_HEAD mtime (D-12 amended): `NotFound` → absent (null + reason), any other error → `Error::Io { what: format!(... path.display()), source }`. Path from `self.git_dir()` / `self.common_dir()` (`src/git/repo.rs:141-150`).

Module wiring: new file gets `mod x;` in `src/git/mod.rs:25-33` and a `pub use` in `:35-40`. Module doc starts with `//!` stating what child it runs (see `src/git/objects.rs:1-6`, `src/git/config.rs:1-4`).

---

### `src/git/mod.rs`, `src/model/mod.rs` (drop module `dead_code`)

Current (`src/git/mod.rs:19-23`, `src/model/mod.rs:12-15`):
```rust
#![cfg_attr(not(test), expect(dead_code, reason = "no caller until phase 5"))]
```
Replacement per RESEARCH Pitfall 2: item-level `#[cfg_attr(not(test), expect(dead_code, reason = "first caller in phase N"))]` on each item `check` still does not reach (measured list: `merge_base`, `ahead_behind`, `range_walk`, `commit_ids`, `commit_id` in `src/git/reads.rs`; `index_path`, `siblings`, `list_failed` in `src/git/repo.rs`; `encode_path` in `src/model/manifest.rs:292`; `StaleWorktree` variant). Keep the existing comment's reasoning (why `cfg_attr(not(test), ...)`, short reason for rustfmt).

---

### `Cargo.toml`

**Analog:** itself. Every dependency carries a comment block: purpose, why this crate/features, `default-features = false` reasoning, licenses added to `deny.toml`, "Legitimacy checked in phase N research (Package Legitimacy Audit: OK)". Dev-deps say "Dev-only, so nothing ships in the binary". Copy that for serde_json, lexopt (`[dependencies]`) and jsonschema, proc-macro2 (`[dev-dependencies]`); gate on `scripts/check.sh deps`.

---

### `tests/contract.rs` (integration: schema validation, hash pin, table coverage)

**Analog A:** `tests/cli.rs:1-18` — binary-boundary test file header and spawner:
```rust
//! CLI boundary tests: the binary's observable output (stdout, stderr and exit
//! code) is the contract under test, not any function inside it. ...
use std::process::{Command, Output};

fn wsx_bin() -> &'static str {
    env!("CARGO_BIN_EXE_wsx")
}
fn run_wsx(args: &[&str]) -> Output {
    Command::new(wsx_bin()).args(args).output().expect("spawn the wsx binary")
}
```
Assertion style (`tests/cli.rs:27-33`): `assert_eq!(output.status.code(), Some(N), "<what>")` with a message naming the command.

**Analog B (pinned hash by git):** `tests/harness/provenance.rs:125-143` `hash_blob`
```rust
let output = command("git", &dirs.home, env)
    .args([OsStr::new("hash-object"), OsStr::new("--no-filters")])
    .arg(file)
    .output()
    .map_err(|err| format!("spawn git hash-object: {err}"))?;
let stdout = String::from_utf8_lossy(&output.stdout);
let blob = stdout.trim_end();
if output.status.code() != Some(0) || blob.len() != 40 {
    return Err(format!("git hash-object exited {:?} with stdout {blob:?}; stderr: {}", ...));
}
```
and the mismatch collector `blob_mismatches` (`:103-123`, `"<path>: blob <actual>, recorded <recorded>"`). Pin table shape: `VENDORED_BLOBS` (`:28-60`). Run git in a hermetic env (no host config), as `command(..., &dirs.home, env)` does; outside the harness crate, set `HOME`/`GIT_CONFIG_NOSYSTEM` on the child only (no `set_var`, clippy.toml). Repo fixtures for success/failure `--json` runs: a tempfile `TempDir::new_in(env!("CARGO_TARGET_TMPDIR"))` repo, or reuse the harness `Fixture` (`RepoConfig::devflow()` / `no_gsd()`) by placing the contract test in `tests/harness/`.

Control (b): scratch-copy the schema with one field renamed, assert the failure text names the file and `(command, version)` (04-LEARNINGS: assert intended text, not only exit).

---

### `tests/if02_scan.rs` (source scan + planted-read control)

**Analog A (scratch copy under `CARGO_TARGET_TMPDIR`, real tree as the positive case):** `tests/image_parity.rs:1-65`
```rust
//! Each case copies the script into its own tree under CARGO_TARGET_TMPDIR
//! with a fixture ... The real tree is checked in place as the positive case.
const REPO_ROOT: &str = env!("CARGO_MANIFEST_DIR");
const TMP_ROOT: &str = env!("CARGO_TARGET_TMPDIR");

fn fixture_tree(name: &str, ...) -> PathBuf {
    let root = Path::new(TMP_ROOT).join("image-parity").join(name);
    if root.exists() {
        fs::remove_dir_all(&root).expect("remove stale fixture tree");
    }
    ...
}
```
`assert_refused` (`:72-85`) asserts exit and that success text is absent; for the scan, assert the finding text names the planted file and literal. Container note (CLAUDE.md): checkout is read-only, so the planted copy must live under `CARGO_TARGET_TMPDIR`, never under `src/`.

**Analog B (walking for forbidden files, collecting all findings):** `tests/harness/fixture.rs:720-740` `no_gsd_layout_has_no_gsd_devflow_or_agent_files` (collect into `found`, assert empty with the list). Second control: a comment-only mention must not trip (RESEARCH Pattern 7).

---

### `tests/harness/check.rs` (enable Rust sides, add DIV scenarios)

**Analog:** itself.
- Helpers to reuse: `check_in_work(config, fixture)` (`:28-37`), `verdict(exit)` (`:39-47`), fixture steps `add_note` / `no_steps` (`:16-26`), pinned oracle stderr consts `UNDECLARED_NOTE` (`:49-52`).
- Guard-with-both-controls shape (`self_test`, `:54-79`):
```rust
Scenario::new("self_test", check_in_work(
        RepoConfig::devflow().with_manifest_entry(ManifestKind::Only, b"notes"), add_note,
    ).expect(verdict(0)))
.control(Role::KnownPositive,
    check_in_work(RepoConfig::devflow(), add_note).expect(verdict(1)).oracle_stderr(UNDECLARED_NOTE))
.control(Role::Clean, check_in_work(RepoConfig::devflow(), no_steps).expect(verdict(0)))
```
- Class variants: `Class::Fixed { defect }` + `.oracle(OracleSide::Recorded(verdict(1)))` (`:128-137`); `Class::Regression { reason }` (`:326-335`); Rust-only via `.oracle(OracleSide::NotImplemented { why })` (`:265-270`).
- Registration (`:348-355`): switch each line from `rust = "Rust side not enabled until phase 5"` to `rust_enabled`; under `rust_enabled` every non-zero-exit case needs `.rust_stderr(b"...")` (`tests/harness/scenario.rs:642-650`, `Case::rust_stderr` at `:121`), and the Rust side is judged by `contains(stderr, rust_stderr)` (`:596-626`).
- `reftable_refused` (`:316-346`): subject `verdict(1)` → `verdict(5)` plus `.rust_stderr(b"reftable")`-style pin; update its doc's "placeholder" sentence.
- Fixture steps that fail loudly when a setup silently did nothing: `to_reftable` (`:296-311`) re-reads state and returns `Err(format!(...))`; copy for hook-mode and remote-unreachable fixtures.
- `RepoConfig::with_manifest_entry` writes both `.wsx.conf` and `.workspace-divergence` (`tests/harness/fixture.rs:368-387` shows `extra` appended to both).

### `tests/harness/rust.rs`
`Action::Check { .. } => push("check")` (`:46`) already matches; the module doc (`:1-5`) says the enabling phase revises the line in the same commit, so update the doc sentence when enabling. A `--json` variant, if the harness needs one, goes through `wsx_argv` (`:20-65`).

## Shared Patterns

### Error rendering (stderr prose)
**Source:** `src/model/mod.rs:182-203`, `src/git/mod.rs:218-232`
**Apply to:** `check` human report, `cli` error path, `exit.rs` message field
Every path or ref in prose goes through `git::c_quote` (via `shown`); a command the user should paste goes through `git::shell_word` (`command_word`). Messages state cause then fix, as `ForeignOwner` does (`"refusing {path}: ...\nTo trust it, run:\n\t..."`).

### Git child errors
**Source:** `src/git/config.rs:79-103`, `src/git/objects.rs:303-311`
**Apply to:** all new `src/git/` reads
Non-zero exit → `Error::GitFailed { argv, status, stderr }`; unparseable → `Error::GitBadOutput { argv, problem }`; missing object → `Error::ObjectMissing { request }`. Never return fewer records than git printed.

### Dead-code expectations
**Source:** `src/git/mod.rs:19-23`
**Apply to:** `src/cli/commands.rs` fields unused by the binary, remaining phase-6+ items
`#[cfg_attr(not(test), expect(dead_code, reason = "..."))]`, short reason, comment explaining the `cfg_attr`.

### Test isolation
**Source:** `tests/image_parity.rs:13-14,42-47`; `tests/harness/check.rs:7-9`
**Apply to:** `tests/contract.rs`, `tests/if02_scan.rs`, new harness scenarios
Scratch under `CARGO_TARGET_TMPDIR`; env on the child only (`std::env::set_var` banned by `clippy.toml`); one scenario per test; each guard keeps a known-positive and a clean control; run one test with `scripts/check.sh test -E 'test(=module::name)'`.

### Module docs
**Source:** `src/git/mod.rs:1-17`, `src/model/mod.rs:1-10`, `src/git/config.rs:1-4`
**Apply to:** every new `src/` and `tests/` file
`//!` header saying what the module reads/runs and its invariants; `///` on every item.

## No Analog Found

| File | Role | Data Flow | Reason |
|------|------|-----------|--------|
| `docs/exit-codes.md` | doc | — | `docs/` does not exist; content is the D-06 table |
| `docs/json/check.v1.schema.json`, `docs/json/error.v1.schema.json` | schema | — | no JSON or JSON Schema in the repo; use RESEARCH Pattern 5 (`oneOf` string / `{"hex"}` byte field) |

Partial-analog notes: no JSON serialization or argv crate exists in the codebase yet (`src/json.rs`, `src/cli/mod.rs` follow RESEARCH Standard Stack for the library calls and the analogs above only for structure). No existing `#[path]` include of a `src/` file from `tests/`; `tests/harness/main.rs:21-32` is where test-crate modules are declared.

## Metadata

**Analog search scope:** `src/` (all 12 files), `tests/*.rs`, `tests/harness/`, `Cargo.toml`; graphify used for `tree_diff`; `docs/` absent
**Files scanned:** 20 (bounded line ranges)
**Pattern extraction date:** 2026-11-18
