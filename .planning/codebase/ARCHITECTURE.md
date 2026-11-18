---
last_mapped_commit: a686ea156dac3e4c93d9f9d180ee4a35e340c4f5
last_mapped_at: 2026-11-18
---
<!-- refreshed: 2026-11-18 -->
# Architecture

**Analysis Date:** 2026-11-18

## System Overview

```text
┌─────────────────────────────────────────────────────────────┐
│                 CLI entry  `src/main.rs`                     │
│   argv parse (today: only `--version`; exit 0 / 2 usage)     │
└────────────────────────────┬────────────────────────────────┘
                             │ (no caller yet; wired by phase 5 `wsx check`)
                             ▼
┌─────────────────────────────────────────────────────────────┐
│            Repository model  `src/model/`                    │
│  `resolve()` -> RepoModel{Configured | NotConfigured}        │
│  `manifest.rs`: `.wsx.conf` parse / normalise / covers()     │
└────────────────────────────┬────────────────────────────────┘
                             │ wsx-owned types only (ObjectId, raw bytes)
                             ▼
┌──────────────────────────────┬──────────────────────────────┐
│ Reads via gix                │ Writes/reads via git binary  │
│ `src/git/repo.rs` (Repo)     │ `src/git/spawn.rs` (Git)     │
│ `reads.rs` `objects.rs`      │ `config.rs` `refs.rs`        │
└──────────────────────────────┴──────────────────────────────┘
                             │
                             ▼
              Git repository on disk (object db, refs, config)

Test side (no runtime coupling):
  `tests/harness/`  scenario harness: oracle (`tests/oracle/*.sh`) vs `wsx` binary
  `bin/wsx` + `tests/sandbox.sh`  frozen Python 0.1.0, reference only
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| CLI | Parse argv, print version, map to `ExitCode` | `src/main.rs` |
| Model | Resolve workspace branch, tip, integration branch, remotes, worktree dir, manifest | `src/model/mod.rs` |
| Manifest | Parse `.wsx.conf`, path normalise/encode, `covers()` | `src/model/manifest.rs` |
| Git facade | Re-exports; `ObjectId`, `Error` with actionable `Display` | `src/git/mod.rs` |
| Repo discovery | The single gix open site (`open_gix`), ceiling/ownership/env refusals | `src/git/repo.rs` |
| Process runner | `Git` spawns the real `git` binary with scoped env, deadlines, stdin | `src/git/spawn.rs` |
| Reads | ahead/behind, tree diff (`TreeChange`) | `src/git/reads.rs` |
| Objects | Tip tree reads (`TipObjects`, `RootEntry`, `read_tips`) | `src/git/objects.rs` |
| Refs | `RefRecord`, `list_refs`, `peel_commit`, replace-ref refusals, ref-name checks | `src/git/refs.rs` |
| Config | `ConfigRead` (`read_config`, `values`, `defines_remote`) | `src/git/config.rs` |
| Quoting | C-quote/unquote, `shell_word` | `src/git/quote.rs` |
| Test repos | `#[cfg(test)]` builders for unit tests | `src/git/testing.rs` |

## Pattern Overview

**Overall:** Single binary crate, layered: CLI -> model -> git adapter (gix for reads, git CLI for history changes).

**Key Characteristics:**
- gix types never leave `src/git/`; callers see `ObjectId` (lowercase hex) and `Vec<u8>` paths/names.
- Every history change runs the real `git` binary so user hooks, signing, merge drivers and credentials apply.
- Every refusal is a typed `Error` variant whose message names cause and fix.
- Resolve once: `model::resolve` produces one `Model` every command reads.
- Non-interactive: the tool never prompts.

## Layers

**CLI:**
- Purpose: argument handling and exit codes
- Location: `src/main.rs`
- Contains: `main`, `EXIT_USAGE = 2`
- Depends on: declares `mod git; mod model;`
- Used by: users, `tests/cli.rs`, `tests/harness/rust.rs`

**Model:**
- Purpose: one resolved view of the repository
- Location: `src/model/`
- Contains: config key constants (`wsx.sourceBranch`, `wsx.baseRemote`, `wsx.pushRemote`, `wsx.baseRef`, `wsx.worktreeDir`), `Model`, `Integration`, `RepoModel`, `Error`, `resolve` (`src/model/mod.rs:372`), `personal_paths`/`is_personal`
- Depends on: `crate::git` only
- Used by: nothing in `main` yet (`expect(dead_code)` until phase 5)

**Git adapter:**
- Purpose: repository access
- Location: `src/git/`
- Contains: `Repo` (gix-backed), `Git` (process-backed), errors, quoting
- Depends on: `gix =0.90.0` (sha1, sha256, revision), `rustix` (euid)
- Used by: `src/model/`

## Data Flow

### Model resolution (the path future commands use)

1. `Repo::discover` opens the repo via `open_gix`, honouring `GIT_DIR`/`GIT_INDEX_FILE`, canonicalising worktree/git dir/common dir, refusing untrusted owners, ceilings, object-store env, ref namespaces and unsupported ref storage (`src/git/repo.rs:30-660`)
2. `Git::for_repo` / `Git::from_process` builds the spawner (`src/git/spawn.rs:297-355`)
3. `model::resolve(repo, git, recorded)` reads local config (`ConfigRead`), lists refs, refuses replace refs, picks configured or inferred workspace branch, resolves integration branch (`wsx.baseRef` or fallback chain) and reads `.wsx.conf` from the workspace tip's tree (`src/model/mod.rs:372`)
4. `RepoModel::require` converts manifest/replace-ref failures on an unconfigured repo into `Error::NotConfigured` ("run `wsx init`") (`src/model/mod.rs:114`)

### Scenario harness

1. `tests/harness/main.rs` `harness_modules!(sync, worktree, check, cut, guard, signing)` registers scenarios
2. `fixture.rs` builds a scratch repo under `CARGO_TARGET_TMPDIR` with hermetic env (`env.rs`)
3. `oracle.rs` runs patched `tests/oracle/*.sh`; `rust.rs` runs the `wsx` binary (ignored until the implementing phase)
4. `outcome.rs` compares against the explicit expected `Outcome`

**State Management:**
- No persistent wsx state beyond git: local config keys `wsx.*` and the `.wsx.conf` blob at the workspace tip.

## Key Abstractions

**`Repo`:** gix repository handle with canonical paths; the only open site. `src/git/repo.rs:15`
**`Git`:** builder-style spawner (`with_own_worktree`, `with_removal_disabled`, `with_deadline`; `run`, `run_without_var`, `run_with_input`). `src/git/spawn.rs:277`
**`ObjectId`:** hash-kind-independent hex id. `src/git/mod.rs:55`
**`Model` / `RepoModel`:** resolved repository. `src/model/mod.rs:66,87`
**`Manifest` / `Entry` / `EntryKind`:** personal-path list. `src/model/manifest.rs`
**Harness `Scenario`/`Case`/`Outcome`:** `tests/harness/scenario.rs`, `tests/harness/outcome.rs`

## Entry Points

**Binary:** `src/main.rs` — invoked as `wsx`; only `--version` today.
**Test targets:** `tests/cli.rs`, `tests/harness/main.rs`, `tests/gate.rs`, `tests/gate_controls.rs`, `tests/image_parity.rs`, `tests/repo_policy.rs`, `tests/setup_repo.rs`.
**Frozen oracle:** `bin/wsx` (Python 0.1.0) — reference only.

## Architectural Constraints

- **Threading:** single-threaded; child processes via `std::process`. gix `interrupt` feature off so wsx owns SIGINT.
- **Global state:** none; `std::env::set_var`/`remove_var` banned (`clippy.toml`); scope env with `Command::env`.
- **gix open ban:** all gix open/discover entry points are `disallowed-methods` except `open_gix` in `src/git/repo.rs`.
- **Revspec ban:** gix `rev_parse*`/spec parsers banned (ref-vs-hex ordering differs from git); take an `ObjectId` and `Repo::peel_commit`.
- **No network:** model children pass `--no-lazy-fetch` (`NO_LAZY_FETCH`, `src/git/mod.rs`); no gix network features; `mailmap` banned in `deny.toml`.
- **Circular imports:** none; `model` -> `git` one way.

## Anti-Patterns

### Opening a repo outside `Repo::discover`
**What happens:** calling `gix::open`/`discover` directly.
**Why it's wrong:** ignores hook env or pairs a sibling worktree with the hook's index.
**Do this instead:** `Repo::discover`, siblings via `main_repo()`/`worktrees()` (`src/git/repo.rs`).

### Leaking gix types upward
**What happens:** returning `gix::ObjectId` or `BString` from `src/git/`.
**Why it's wrong:** breaks the adapter boundary documented in `src/git/mod.rs`.
**Do this instead:** convert to `ObjectId` / `Vec<u8>`; carry gix errors as message only.

### Mutating history through gix
**Do this instead:** use `Git::run` so user hooks and signing apply.

## Error Handling

**Strategy:** typed per-layer `Error` enums (`src/git/mod.rs:103`, `src/model/mod.rs:135`) with `From<git::Error>` for model; `Display` renders an actionable message naming cause and fix; argv rendered via `render_argv`.

**Patterns:**
- `refuse_*` functions return `Result<(), Error>` before any work.
- `Result` + `?`; no `unwrap` outside tests.

## Cross-Cutting Concerns

**Logging:** none; messages to stderr from `main`.
**Validation:** ref names (`check_full_ref_name`, `is_branch_name`), manifest paths (`normalize_path`).
**Authentication:** delegated to the git binary (credentials, signing).

---

*Architecture analysis: 2026-11-18*
