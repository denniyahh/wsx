---
last_mapped_commit: a686ea156dac3e4c93d9f9d180ee4a35e340c4f5
last_mapped_at: 2026-11-18
---
# Technology Stack

**Analysis Date:** 2026-11-18

## Languages

**Primary:**
- Rust, edition 2024 - the `wsx` binary crate (`src/main.rs`, `src/git/`, `src/model/`); no library crate
- Rust - integration tests and the scenario harness (`tests/*.rs`, `tests/harness/`)

**Secondary:**
- Python 3 - frozen 0.1.0 reference implementation `bin/wsx` (shebang `#!/usr/bin/env python3`); read-only, deleted at cutover
- Bash - `tests/sandbox.sh` (frozen, deleted at cutover) and the vendored devflow oracle scripts in `tests/oracle/` (`sync-workspace.sh`, `cut-pr-branch.sh`, `phase-worktree.sh`, `check-workspace-divergence.sh`, `hooks/`), byte-for-byte copies of devflow commit `ee6ec792`, deleted at cutover (REL-02)

## Runtime

**Environment:**
- Native Linux/Unix binary (uses `std::os::unix` APIs throughout `src/git/`)
- Requires an external `git` CLI, version 2.54 or newer, checked once on first spawn (`src/git/spawn.rs`)
- Toolchain pinned exactly: rustc `1.99.1`, components `clippy`, `rustfmt`, `llvm-tools`, `rust-analyzer`, profile `minimal` (`rust-toolchain.toml`)

**Package Manager:**
- Cargo (version follows the pinned toolchain)
- Lockfile: `Cargo.lock` present and committed

## Frameworks

**Core:**
- None (no CLI framework); argument dispatch is a hand-written match on `std::env::args_os()` in `src/main.rs`

**Testing:**
- cargo-nextest - runner driven by `scripts/check.sh test` with `--no-tests=fail` always set
- proptest `1.12.0` (dev, `default-features = false`, `std`) - manifest round-trip property test in `src/model/manifest.rs` (`PROPTEST_CASES` read there)
- tempfile `3.27.0` (dev) - scratch git trees via `TempDir::new_in` under `CARGO_TARGET_TMPDIR`
- Custom harness `tests/harness/main.rs` - runs each scenario against the oracle scripts and the `wsx` binary

**Build/Dev:**
- clippy with `clippy.toml` `disallowed-methods` bans (env mutation, gix open/discover, gix revspec parsing)
- rustfmt
- cargo-deny (`deny.toml`) via `scripts/check.sh deps`
- cargo-llvm-cov via `scripts/check.sh coverage` (`CARGO_LLVM_COV_SETUP=no`)
- Tool versions live only in `scripts/install-tools.sh`

## Key Dependencies

**Critical:**
- `gix` `=0.90.0` (exact pin; `default-features = false`, features `sha1`, `sha256`, `revision`) - pure-Rust repository reads: discovery, merge-base, walks, tree diff (`src/git/repo.rs`, `src/git/reads.rs`). `blob-diff`, `interrupt`, network, `worktree-mutation` features off; `mailmap` banned in `deny.toml`. Upgrade in its own commit with the full harness.
- `rustix` `1.1.5` (`std`, `process`) - effective uid for wsx's own repository ownership check (`src/git/repo.rs`, `SUDO_UID` handling)

**Infrastructure:**
- External `git` binary - all mutations and ref/config/object operations not done by gix go through `Git` in `src/git/spawn.rs` (empty env plus explicit snapshot, stdin `/dev/null`, `CHILD_DEADLINE` kill, args never through a shell)

## Configuration

**Environment:**
- No config file or env vars specific to wsx; it honours git's own environment (`GIT_DIR`, `GIT_INDEX_FILE`, `GIT_CEILING_DIRECTORIES`, read in `src/git/repo.rs`) the way git does
- Per-repo manifest `.wsx.conf` parsed in `src/model/manifest.rs`
- Tracked repo git policy `.gitconfig`, copied by allowlist (`POLICY_KEYS`) via `scripts/setup-repo.sh`; never `include.path`
- `std::env::set_var`/`remove_var` banned (`clippy.toml`); scope env with `Command::env`

**Build:**
- `Cargo.toml`, `rust-toolchain.toml`, `clippy.toml`, `deny.toml` (licenses allowed: Apache-2.0, MIT, Unicode-3.0, Zlib; `include-dev = true`; crates.io only)

## Platform Requirements

**Development:**
- Pinned toolchain via rustup; gate is `scripts/check.sh all` (fmt, clippy, test)
- Dev container `.devcontainer/Dockerfile` (`FROM mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`) and `.devcontainer/devcontainer.json` (named volumes for cargo registry, git and `target/`; no Features); `scripts/check-in-container.sh` runs the gate in it

**Production:**
- Unpublished (`publish = false`, version `0.0.0`, MIT); a local CLI binary on Unix hosts with git >= 2.54

---

*Stack analysis: 2026-11-18*
