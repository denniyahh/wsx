---
last_mapped_commit: a686ea156dac3e4c93d9f9d180ee4a35e340c4f5
last_mapped_at: 2026-11-18
---
# Codebase Structure

**Analysis Date:** 2026-11-18

## Directory Layout

```
wsx/
├── src/
│   ├── main.rs            # CLI entry
│   ├── git/               # git adapter (gix reads + git CLI)
│   └── model/             # repository model + .wsx.conf manifest
├── tests/
│   ├── harness/           # scenario harness test target (main.rs)
│   ├── oracle/            # vendored devflow shell scripts (deleted at cutover)
│   ├── fixtures/env-ban/  # mini crate proving the clippy env ban bites
│   ├── *.rs               # standalone integration test targets
│   └── sandbox.sh         # frozen Python-era sandbox (reference only)
├── bin/wsx                # frozen Python 0.1.0 (reference only)
├── .devcontainer/         # pinned image (Dockerfile, devcontainer.json)
├── Cargo.toml / Cargo.lock
├── rust-toolchain.toml    # rustc 1.99.1 pin
├── clippy.toml            # disallowed-methods (env, gix open, revspec)
├── deny.toml              # cargo-deny licenses/bans
├── .gitconfig             # tracked repo policy, copied by scripts/setup-repo.sh
├── .workspace-divergence  # workspace-branch-only paths for PR cut
├── CLAUDE.md, CONTRIBUTING.md, README.md, CHANGELOG.md
└── AGENTS.md, CLAUDE.local.md  # workspace-branch only, stripped from PRs
```

## Directory Purposes

**`src/git/`:**
- Purpose: all repository access; the only module that touches gix
- Key files: `mod.rs` (re-exports, `ObjectId`, `Error`), `repo.rs` (`Repo`, discovery, refusals; 3.5k lines incl. tests), `spawn.rs` (`Git`), `reads.rs`, `objects.rs`, `refs.rs`, `config.rs`, `quote.rs`, `testing.rs` (cfg(test) builders)

**`src/model/`:**
- Purpose: resolved repository view
- Key files: `mod.rs` (`resolve`, `Model`, `Error`; ~7k lines, mostly unit tests), `manifest.rs` (`parse`, `normalize_path`, `encode_path`, `covers`)

**`tests/harness/`:**
- Purpose: black-box oracle-vs-Rust scenarios
- Infrastructure: `scenario.rs` (macros, `Case`, `Scenario`), `fixture.rs`, `env.rs` (hermetic env), `oracle.rs`, `rust.rs`, `outcome.rs`, `action.rs`, `registry.rs`, `coverage.rs`, `hermeticity.rs`, `provenance.rs` (oracle blob pins)
- Scenario modules: `sync.rs`, `worktree.rs`, `check.rs`, `cut.rs`, `guard.rs`, `signing.rs`

**`tests/oracle/`:**
- Purpose: byte-identical devflow scripts (`sync-workspace.sh`, `phase-worktree.sh`, `cut-pr-branch.sh`, `check-workspace-divergence.sh`, `hooks/pre-push`) plus `oracle.patch`; provenance in `README.md`
- Generated: No. Never edit the scripts (blob-pinned by `provenance::vendored_files_match_devflow_blobs`).

**Top-level test targets:** `tests/cli.rs` (binary output contract), `tests/gate.rs` + `tests/gate_controls.rs` (`scripts/check.sh` behaviour and negative controls), `tests/image_parity.rs`, `tests/repo_policy.rs`, `tests/setup_repo.rs`.

## Key File Locations

**Entry Points:** `src/main.rs`; `tests/harness/main.rs`
**Configuration:** `Cargo.toml`, `rust-toolchain.toml`, `clippy.toml`, `deny.toml`, `.devcontainer/Dockerfile`, `.gitconfig`
**Core Logic:** `src/model/mod.rs`, `src/git/repo.rs`, `src/git/spawn.rs`
**Testing:** inline `#[cfg(test)] mod tests` in each `src/` file; `tests/`

## Naming Conventions

**Files:** snake_case module files; one concern per file (`refs.rs`, `reads.rs`).
**Directories:** module dirs with `mod.rs` (`src/git/mod.rs`, `src/model/mod.rs`).
**Tests:** behaviour sentences in snake_case (`missing_clippy_fails_clippy_by_name`); harness tests named `<module>::<scenario>::<side>` (`sync::self_test::oracle`).
**Commits (workspace branch):** `type(NN-MM): ...`.

## Where to Add New Code

**New command (subcommand):**
- Dispatch: `src/main.rs`; command logic in a new module beside `src/model/` (e.g. `src/check/`) consuming `model::resolve` -> `RepoModel::require`
- Tests: CLI contract in `tests/cli.rs`; behaviour as a scenario in the matching `tests/harness/<module>.rs` (un-ignore its Rust side)

**New git read:** `impl Repo` in `src/git/reads.rs`/`objects.rs`/`refs.rs`; re-export from `src/git/mod.rs`; return wsx types.
**New history change:** via `Git::run*` in `src/git/spawn.rs` callers.
**New config key:** `pub const` in `src/model/mod.rs` next to `SOURCE_BRANCH`.
**New harness scenario module:** add file under `tests/harness/` and its name to `harness_modules!` in `tests/harness/main.rs`.
**Utilities:** quoting in `src/git/quote.rs`; test repo builders in `src/git/testing.rs`.
**New ignore lines:** top of `.gitignore`.

## Special Directories

**`target/`:** build output. Generated: Yes. Committed: No.
**`.tools/`:** pinned tool installs from `scripts/install-tools.sh`. Generated: Yes. Committed: No.
**`tests/oracle/`:** vendored, temporary until cutover. Committed: Yes.
**`bin/`, `tests/sandbox.sh`:** frozen Python 0.1.0; do not change; deleted at cutover. Committed: Yes.
**`.planning/`, `graphify-out/`:** workspace-branch only. Committed: Yes (workspace branch).

---

*Structure analysis: 2026-11-18*
