---
last_mapped_commit: a686ea156dac3e4c93d9f9d180ee4a35e340c4f5
last_mapped_at: 2026-11-18
---
# Codebase Concerns

**Analysis Date:** 2026-11-18

Scope: `src/`, `tests/`, `Cargo.toml`, `clippy.toml`, `deny.toml`, `rust-toolchain.toml`, `.devcontainer/`, repo docs. `bin/wsx` and `tests/sandbox.sh` are the frozen Python 0.1.0 oracle, deleted at cutover; they are read-only reference.

Evidence: every item below comes from reading the files named (CODE-READ). Nothing here was executed for this audit.

## Tech Debt

**The Rust binary has no command surface yet:**
- Issue: `src/main.rs` handles only `--version`; any other argument prints `usage: wsx --version` and exits 2 (`EXIT_USAGE`). The git and model layers exist but nothing calls them.
- Files: `src/main.rs`, `src/git/mod.rs:23`, `src/model/mod.rs:15` (both `#![cfg_attr(not(test), expect(dead_code, reason = "no caller until phase 5"))]`)
- Impact: the whole library is exercised only by unit tests; integration seams (argument parsing, exit-code contract IF-01, error rendering to the user) are untested because they do not exist.
- Fix approach: phase 5 wires the CLI. Remove both crate-level `expect(dead_code)` attributes in the same change: `expect` turns into a warning once callers exist, so leaving them would fail clippy `-D warnings` rather than rot silently.

**Docs describe the Python tool, not the Rust tree:**
- Issue: `README.md` (Quick start, Commands, Automation, Layout sections) documents the Python 0.1.0 behaviour (`personal-workspace` branch, sync/pull/lww, hooks). The Rust binary implements none of it yet. `CONTRIBUTING.md:3-7` states the rewrite status; `README.md` does not.
- Files: `README.md`, `CONTRIBUTING.md`
- Impact: a reader building from `Cargo.toml` gets a binary that does nothing the README promises.
- Fix approach: rewrite README commands per phase as each command lands, or add a status note pointing at `CONTRIBUTING.md`.

**Very large modules, mostly inline tests:**
- Issue: `src/model/mod.rs` is 7086 lines with tests from line 751; `src/git/repo.rs` 3516 (tests from 660); `src/git/spawn.rs` 1637 (tests from 552); `src/git/refs.rs` 1390; `src/git/config.rs` 1361; `src/git/reads.rs` 1339. `src/git/testing.rs` (1708) is a `pub(crate)` test-support module (`src/git/mod.rs:33`).
- Impact: slow navigation and review; tool reads of these files exceed context budgets. Production logic is small (roughly 200-750 lines per file).
- Fix approach: move `#[cfg(test)] mod tests` blocks into sibling files (`src/model/tests.rs` via `#[cfg(test)] mod tests;`) without changing test names, so nextest filters `test(=module::name)` keep working.

**Workspace divergence check not ported:**
- Issue: `.workspace-divergence` header says "The divergence check hook is not ported yet." The file is consumed only by `scripts/workspace/cut-pr-branch.sh`.
- Files: `.workspace-divergence`
- Impact: a workspace-only path added without an `only` line can leak into a PR cut unnoticed.
- Fix approach: port the hook, or add a test in `tests/` that diffs the branch against develop and fails on undeclared paths.

## Known Bugs

**Shipped Python 0.1.0 has unresolved data-loss defects:**
- Symptoms: the previous audit reproduced, against `bin/wsx`: an unreadable local file deletes the source copy on push; `lww` deletes local-only files on pull with no backup; a both-sides delete leaves a poisoned base entry. The `pre-push` guard is skipped on a branch's first push.
- Files: `bin/wsx` (frozen; do not fix there)
- Trigger: see the Python oracle; anyone still running 0.1.0 is exposed until cutover.
- Workaround: none in-tree by policy. The Rust rewrite must not reproduce these: when `tests/harness/oracle.rs` compares against the oracle, each of these cases needs an explicit expected divergence, not oracle parity.

No defects detected in the Rust `src/` during this read.

## Security Considerations

**Repository ownership trust:**
- Risk: gix trusts a repo owned by `SUDO_UID` for any effective uid; git does so only as root.
- Files: `Cargo.toml` (`rustix` comment), `src/git/mod.rs` (`Error::ForeignOwner`)
- Current mitigation: wsx re-checks ownership itself with `rustix` effective uid.
- Recommendations: keep a test that runs with `SUDO_UID` set on the child (`Command::env`) and expects `ForeignOwner`.

**Hook environment and sibling worktrees:**
- Risk: inside a git hook, opening a repo any other way either ignores `GIT_DIR`/`GIT_INDEX_FILE` or applies them to a sibling worktree.
- Files: `clippy.toml` (gix open/discover ban list), `src/git/repo.rs` (`open_gix`, the sole `#[expect(clippy::disallowed_methods)]` production exception at ~L94, ban-list pins from ~L971)
- Current mitigation: clippy bans every public gix 0.90.0 open/discover entry point; a test calls each banned path under its own `expect` so a renamed path fails.
- Recommendations: on every gix bump, re-enumerate the public open/discover/revspec entry points; a new entry point is not banned automatically.

**Revspec resolution order differs from git:**
- Risk: gix resolves a name as hex prefix / describe output before refs; a ref named like `rel-1-g<hex>` resolves to a different object than git gives.
- Files: `clippy.toml` (revspec ban), `src/git/repo.rs` (`banned_gix_revspec_parsers_read_a_describe_shaped_ref_as_another_object`), `deny.toml` (`mailmap` feature banned because it parses revspecs internally)
- Current mitigation: no production revspec parsing; read refs by exact name and peel with `Repo::peel_commit`.
- Recommendations: same re-audit on each gix bump, including newly enabled features.

**Replace refs and object-store environment:**
- Risk: `refs/replace/`, `GIT_REPLACE_REF_BASE`, `GIT_OBJECT_DIRECTORY`, `GIT_NAMESPACE` can make gix and git see different objects.
- Files: `src/git/mod.rs` (`ReplaceRefsPresent`, `ReplaceBaseSet`, `ReplaceRefsUnreadable`, `ObjectStoreEnv`, `RefsNamespace`, `RelativeRepoEnv`)
- Current mitigation: wsx refuses these states with named errors.
- Recommendations: none beyond keeping the refusal tests.

## Performance Bottlenecks

**Git subprocess deadline is fixed:**
- Problem: every `git` child gets `CHILD_DEADLINE = 10s`; exit is polled every 50 µs (`EXIT_POLL`).
- Files: `src/git/spawn.rs:36`, `src/git/spawn.rs:41`, `src/git/spawn.rs:355` (`with_deadline`)
- Cause: a single constant; large repos or cold caches on slow disks may exceed 10 s and surface as `GitTimedOut`. The 50 µs poll spins a core while waiting.
- Improvement path: measure on a large repo before changing; if timeouts appear, scale per call via `with_deadline`. Consider a longer poll interval with backoff.

## Fragile Areas

**gix exact pin:**
- Files: `Cargo.toml` (`gix = "=0.90.0"`, features `sha1`, `sha256`, `revision`), `clippy.toml`, `deny.toml`
- Why fragile: gix ships breaking releases about monthly; the clippy ban lists name gix 0.90.0 paths verbatim; `blob-diff`, `interrupt`, `mailmap` are deliberately off for behavioural reasons recorded in `Cargo.toml`.
- Safe modification: one commit per bump, run `scripts/check.sh all` and `scripts/check.sh deps`, re-enumerate ban lists, check `gix::diff::tree` still matches `git diff-tree --no-renames`.
- Test coverage: ban-list pin tests in `src/git/repo.rs`.

**Test harness and gate controls:**
- Files: `tests/harness/` (`outcome.rs` 2326, `scenario.rs` 1608, `cut.rs` 1513, `hermeticity.rs`, `guard.rs`, `env.rs`), `tests/gate_controls.rs`, `tests/image_parity.rs`, `tests/oracle/cut-pr-branch.sh`
- Why fragile: tests stub `cargo`, `cargo-nextest`, `docker` on `PATH` and copy scripts into scratch checkouts; they depend on script paths and output text of `scripts/check.sh` and `scripts/check-in-container.sh`.
- Safe modification: change a gate script and its control test together; keep env scoped to children (`Command::env`), never `std::env::set_var` (banned in `clippy.toml`). Container checkout is read-only, so write to scratch dirs.
- Test coverage: the controls themselves are the coverage.

**Toolchain and image pins:**
- Files: `rust-toolchain.toml`, `.devcontainer/Dockerfile`, `.devcontainer/devcontainer.json`, `tests/image_parity.rs`
- Why fragile: compiler and image must move together; drift gives host/CI disagreement.
- Safe modification: bump, then `scripts/check-in-container.sh` and `scripts/assert-image-parity.sh`.

## Scaling Limits

**Single-threaded subprocess model:**
- Current capacity: one git child at a time per call path, each capped at 10 s.
- Limit: not measured.
- Scaling path: measure first; not a current concern for the tool's workload.

## Dependencies at Risk

**gix 0.90.0:**
- Risk: rapid breaking releases; behavioural divergences from git (revspec order, ownership trust) are managed by policy, not by upstream.
- Impact: security bans can go stale on upgrade.
- Migration plan: none planned; keep pinned and audit per bump.

`deny.toml` denies yanked crates, unknown registries and git sources, and `unmaintained = "all"`; `multiple-versions` is only `warn`. `scripts/check.sh deps` is not part of `all`, so a dependency advisory does not fail the default gate.

## Missing Critical Features

**Everything user-facing in the Rust binary:**
- Problem: no manifest command, sync, guard, hooks or worktree commands in `src/main.rs`.
- Blocks: cutover and deletion of `bin/wsx`; until then users run the Python tool with its known defects.

## Test Coverage Gaps

**CLI and exit codes:**
- What's not tested: argument parsing beyond `--version`, error rendering, exit-code contract.
- Files: `src/main.rs`
- Risk: library errors in `src/git/mod.rs` may render poorly or map to wrong codes once wired.
- Priority: High (lands with phase 5).

**Coverage not gated:**
- What's not tested: no coverage threshold; `scripts/check.sh coverage` has no CI job.
- Files: `scripts/check.sh` (out of scope paths; referenced by `CLAUDE.md`)
- Risk: untested branches in `src/git/` and `src/model/` go unnoticed.
- Priority: Low.

---

*Concerns audit: 2026-11-18*
