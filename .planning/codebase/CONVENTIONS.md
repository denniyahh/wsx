---
last_mapped_commit: a686ea156dac3e4c93d9f9d180ee4a35e340c4f5
last_mapped_at: 2026-11-18
---
# Coding Conventions

**Analysis Date:** 2026-11-18

Scope: the Rust rewrite (`src/`, `tests/`, `Cargo.toml`, `clippy.toml`, `deny.toml`, `rust-toolchain.toml`). `bin/wsx` and `tests/sandbox.sh` are the frozen Python 0.1.0 oracle: read for reference only, never edit, deleted at cutover. Workflow rules live in `CONTRIBUTING.md`; agent traps in `CLAUDE.md`.

## Naming Patterns

**Files:**
- snake_case module files, one concern each: `src/git/refs.rs`, `src/git/spawn.rs`, `src/model/manifest.rs`.
- Directory modules use `mod.rs` (`src/git/mod.rs`, `src/model/mod.rs`), not `git.rs` beside `git/`.
- Test-only helper modules are named `testing.rs` and declared `#[cfg(test)] pub(crate) mod testing;` (`src/git/testing.rs`).
- Harness scenario modules are named by the action they cover: `tests/harness/sync.rs`, `worktree.rs`, `check.rs`, `cut.rs`, `guard.rs`, `signing.rs`.

**Functions:**
- snake_case, read as what they return or do: `check_full_ref_name`, `is_branch_name`, `render_argv`, `c_quote`/`c_unquote`.
- Constructors/conversions: `from_gix`, `from_hex`, `to_gix`, `from_process` (`src/git/mod.rs`, `src/git/spawn.rs`).
- Test names state the behaviour as a sentence: `error_messages_name_their_cause`, `any_git_path_round_trips`, `help_flag_is_refused`, `property_runs_its_configured_cases`. Never name tests after review rounds or finding numbers.
- Re-executed child halves end in `_child` (`remote_child`, `count_child` in `src/git/config.rs`).

**Variables:**
- Full words, no single letters except closures over a single obvious item (`|byte|`, `|arg|`, `|err|`).
- Constants SCREAMING_SNAKE with a `///` doc naming why: `NO_LAZY_FETCH` (`src/git/mod.rs`), `EXIT_USAGE` (`src/main.rs`), `USAGE_EXIT` (`tests/gate.rs`), `PROPERTY_CASES` (`src/model/manifest.rs`). No magic numbers.

**Types:**
- UpperCamelCase newtypes over primitives for domain values: `ObjectId(String)` (`src/git/mod.rs`).
- Error enums named `Error` per layer, variants named for the cause with struct fields (`Error::NotARepo { start, reason }`, `Error::GitFailed { argv, status, stderr }`).
- gix types never leave `src/git/`: callers get wsx's own types (`ObjectId`, raw-byte paths).

## Code Style

**Formatting:**
- `rustfmt` defaults (no `rustfmt.toml`), edition 2024, toolchain pinned to `1.99.1` in `rust-toolchain.toml` (components clippy, rustfmt, llvm-tools, rust-analyzer).
- Run via `scripts/check.sh fmt`.

**Linting:**
- `cargo clippy --all-targets -- -D warnings` via `scripts/check.sh clippy`; any warning fails.
- `clippy.toml` `disallowed-methods`:
  - `std::env::set_var` / `std::env::remove_var`: banned; scope env to the child with `Command::env`, `env_remove`, `env_clear`.
  - Every gix open/discover entry point (`gix::open`, `ThreadSafeRepository::discover*`, `open_opts`, ...): banned except `open_gix` in `src/git/repo.rs`. Open repositories only through `Repo::discover`.
  - gix revspec parsers (`Repository::rev_parse`, `rev_parse_single`, `revision::Spec::from_bstr`, `revision::plumbing::spec::parse`): banned; take an `ObjectId` and peel with `Repo::peel_commit`, read refs by exact name.
- A justified exception uses `#[expect(clippy::disallowed_methods, reason = "...")]` on the smallest test-only item. Use `expect`, never `allow`, so a stale exception warns.
- Same rule for other lints: `#![cfg_attr(not(test), expect(dead_code, reason = "no caller until phase 5"))]` in `src/git/mod.rs`.
- `scripts/check.sh deps` (cargo-deny, `deny.toml`) whenever dependencies change.

## Import Organization

**Order:**
1. `std` imports, grouped by path (`use std::ffi::OsString; use std::fmt; use std::path::PathBuf;`).
2. External crates (`gix`, `proptest`, `tempfile`).
3. `crate::` / `super::` imports.

Separate groups with a blank line; rustfmt sorts within a group. Module declarations (`mod x;`) and `pub use` re-exports come before `use` lines (see `src/git/mod.rs`).

**Path Aliases:**
- None. Re-export the public surface of a layer from its `mod.rs` (`pub use repo::Repo; pub use spawn::Git;`) and keep submodules private (`mod config;`).

## Error Handling

**Patterns:**
- Each layer has one hand-written `enum Error` with `impl fmt::Display` and `impl std::error::Error` (`src/git/mod.rs`). No `thiserror`/`anyhow`.
- Every refusal names its cause and, where possible, the fix. Example from `Error::ForeignOwner`: `refusing /r: it is owned by another user, ...\nTo trust it, run:\n\tgit config --global --add safe.directory /r`.
- Foreign errors cross the boundary as a message only: `Error::Gix { what, message }` built with `.map_err(|err| failed(err.to_string()))?`.
- Fail rather than read partial data: `GitBadOutput`, `GitEmptyOutput` reject unparseable or empty child output; children have timeouts (`GitTimedOut`, `OpenTimedOut`).
- Propagate with `?`. No `.unwrap()` in non-test code; `expect("reason")` in tests states the invariant (`.expect("spawn scripts/check.sh")`). In tests, `unwrap_or_else(|err| panic!("{err}"))` surfaces the error text.
- Parse errors carry location and the offending text: `ParseError { line, text, problem }` (`src/model/manifest.rs`).
- The tool never prompts and never resolves a conflict for the user.

## Logging

**Framework:** None. `println!`/`eprintln!` in `src/main.rs` only.

**Patterns:**
- Diagnostics go to stderr; usage errors exit `2` (`EXIT_USAGE`). Exit codes become a versioned contract in phase 5 (IF-01).
- User-facing output is aligned, coloured only on a TTY, and says what to do next when it reports a problem (`CONTRIBUTING.md` Style).

## Comments

**When to Comment:**
- Explain *why*, never *what*. A non-obvious constraint names the failure it prevents (`NO_LAZY_FETCH` doc: "so a read never reaches the network").
- `Cargo.toml` comments justify every dependency and feature choice (licence, legitimacy audit, why a feature is off). Add the same for any new dependency.
- `clippy.toml` entries carry a `reason` that tells the caller the sanctioned alternative.

**Doc comments:**
- `//!` module docs on every module, describing its contract (`src/git/mod.rs`, `tests/harness/main.rs`).
- `///` on every item, including private helpers and tests (child tests document what they print).

## Function Design

**Size:** about 40 lines, one job each. Large files (`src/model/mod.rs` 7086 lines, `src/git/repo.rs` 3516) are dominated by in-file `#[cfg(test)] mod tests`.

**Parameters:** borrow (`&[u8]`, `&Path`, `&[OsString]`); raw bytes for git paths and output, not `String`. Generic `I: IntoIterator<Item = S>, S: AsRef<OsStr>` for argv builders (`src/git/testing.rs`).

**Return Values:** `Result<T, Error>` in production; `Result<T, String>` in test helpers and the harness; `Option` for pure validation (`ObjectId::from_hex`). Use `bool::then` for conditional construction.

## Module Design

**Exports:** binary crate only, no lib. Items are `pub` within private modules, re-exported from `mod.rs`; `pub(crate)` for test helpers. Harness modules are plain private `mod` so `-D warnings` rejects unused helpers.

**Barrel Files:** `src/git/mod.rs` acts as the layer's facade; `tests/harness/main.rs` registers scenario modules through `harness_modules!` so a module cannot be declared without being registered.

---

*Convention analysis: 2026-11-18*
