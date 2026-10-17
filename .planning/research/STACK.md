# Stack Research

**Domain:** Rust CLI that syncs personal, untracked workspace files across git worktrees (rewrite of wsx 0.1.0, Python)
**Researched:** 2026-10-17
**Confidence:** HIGH for the gix capability table and crate versions (checked against the released crate source and by running code); MEDIUM for build and release advice (reasoned from the dependency tree, not built for musl)

## How this was verified (read first)

- **Versions** were read from the crates.io API on 2026-10-17, not from memory.
- **gix capabilities** were read from the **released `gix-0.89.0.crate` tarball** (VCS sha `0115e902fd0334b8f370766993eb17babed3157f`), not from docs. A probe binary then ran every wsx git operation through gix 0.89.0 against a fixture repo, and each result was compared with the git 2.56.0 CLI on the same repo. Each check had a case that had to come out the other way:
  - an unstaged edit had to stay out of the staged set;
  - the main worktree had to *not* see a linked worktree's per-worktree excludes;
  - removing a worktree with untracked, non-ignored files had to fail.
- **gitoxide's own `crate-status.md` is stale.** It still lists "create linked worktrees" as new and "write configuration back" / "remove linked worktrees" as missing. The 0.89.0 source has all three. Do not plan from `crate-status.md` or from Context7, which only indexes that file for this topic.
- **Confidence labels.** The GSD confidence seam has no tier for "ran it locally against the reference implementation". Labels below say what each one rests on: **HIGH (executed)** = probe output matched git CLI; **HIGH (source)** = read in the 0.89.0 crate source; **MEDIUM** = inferred; **UNVERIFIED** = not checked.
- **Limits of the probe.**
  - It ran on one fixture. That fixture uses the files ref backend, SHA-1, absolute worktree links, no submodules, no sparse checkout and no split index.
  - It did not exercise unborn HEAD, detached HEAD on the probed side, locked worktrees, `worktree.useRelativePaths`, or non-UTF-8 paths.
  - Matching git on these cases does not show that gix matches git on all inputs.

## The gix capability table (most important output)

gix version: **0.89.0** (released 2026-10-08; MSRV 1.88). The worktree add and remove APIs were **added in 0.89.0**: they are 9 days old. `config_file_mut()` was added in 0.88.0 (2026-09-25).

| wsx operation (0.1.0 git call) | gix 0.89.0 API | Verdict | Confidence | Notes / traps |
|---|---|---|---|---|
| Open / discover from a linked worktree or its subdirectory (`rev-parse --git-dir`) | `gix::discover(path)`; at the entry point prefer `ThreadSafeRepository::discover_with_environment_overrides(cwd)` | **gix** | HIGH (executed) | `gix::discover` ignores `GIT_DIR`. git honours it (`GIT_DIR=main git -C wt1 …` resolves to main). Use the env-overrides variant at the entry point so hook context (`GIT_DIR`, `GIT_INDEX_FILE`) behaves like git. |
| git-dir / common-dir / toplevel (`--absolute-git-dir`, `--git-common-dir`, `--show-toplevel`) | `git_dir()`, `common_dir()`, `workdir()` | **gix** | HIGH (executed) | `common_dir()` comes back as `<git-dir>/../..`, not canonical. Canonicalise before comparing or deriving names. |
| Current branch (`rev-parse --abbrev-ref HEAD`) | `head_name()?.map(\|n\| n.shorten())` | **gix** | HIGH (executed, attached). MEDIUM for detached | Detached HEAD gives `None`. Python got the string `"HEAD"`. Map it explicitly. |
| List worktrees (`worktree list --porcelain`: path + branch) | `main_repo()` + `worktrees()` → `Proxy::{base(), id(), is_locked(), is_prunable(), into_repo()}` → `head_name()` | **gix** | HIGH (executed) | `worktrees()` excludes the main worktree; add it from `main_repo()`. The order is directory-read order, so sort explicitly. Whether git's own porcelain order is sorted is UNVERIFIED; the harness should normalise order. |
| Add worktree (`worktree add <path> -b <branch> <base>`) | `reference(name, id, PreviousValue::MustNotExist, msg)` then `add_worktree(dest, Head::Attached(..), progress, &interrupt)` | **git CLI (recommended)** | HIGH (executed) | gix only checks out an **existing** local branch or a detached commit; it has no `-b`. Measured differences from `git worktree add -b feat4 … origin/main`: **(1)** git set `branch.feat4.remote=origin` / `merge=refs/heads/main` (autoSetupMerge) and gix set no upstream, which changes which range the pre-push guard checks; **(2)** gix runs no `post-checkout` hook (gix has no hook execution at all); **(3)** the HEAD reflog message is empty. Both git and gix copy the source worktree's `config.worktree` into the new worktree, so the new worktree briefly carries the *creator's* `core.excludesFile` until wsx rewrites it. |
| Remove worktree (`worktree remove [--force]`) | `remove_worktree(path, Force::{Never, DiscardChanges, OverrideLock}, progress)` | **gix** | HIGH (executed) | `Never` refused untracked non-ignored files and allowed a worktree whose only extra files were per-worktree-excluded (same as git). `DiscardChanges` matches `--force`; `OverrideLock` matches `-f -f`. Refuses the main worktree. Keeps the branch. Brand new in 0.89.0. |
| Read config: repo, global, system (`config --get wsx.sourceBranch`, `core.bare`, `wsx.baseRef`) | `config_snapshot().{string, boolean, trusted_path}` | **gix** | HIGH (executed) | `trusted_path` expands `~/`. The snapshot is frozen when the repo is opened (see the write row). |
| Read **per-worktree** config (`extensions.worktreeConfig` + `config.worktree`) | same snapshot | **gix** | HIGH (executed) | wt1 saw its own `core.excludesFile`; main saw the global one; both matched git. This works at `repositoryformatversion 0`, as git does. |
| Read the **global/system** `core.excludesFile` underneath a per-worktree override (the "inherit custom global excludes" fix) | `config_snapshot().plumbing().string_filter("core.excludesFile", &mut \|m\| matches!(m.source, Source::User \| Source::Git \| Source::System))` | **gix** | HIGH (executed) | The filtered raw value is **not interpolated**: `~/custom-global-ignore` comes back literally. Interpolate it, or use the path accessor with interpolation. Confirmed: inside a worktree with a per-worktree `core.excludesFile`, both gix and git stop applying the global file. That is the 0.1.0 defect. |
| Write config: `extensions.worktreeConfig=true` (shared) and `config --worktree core.excludesFile <file>` | `config_path(Source::Local \| Source::Worktree)` → `config_file_mut(path)` → `set_raw_value(..)` → `commit()` | **gix** | HIGH (executed: git CLI read back both values, and main did not see wt2's value) | Takes `config.lock` with git-compatible `core.configLockTimeout` and writes atomically, keeping formatting. **It does not refresh the open `Repository`**: call `reload()` or reopen before reading back. Added in 0.88.0. |
| Staged paths for the commit guard (`diff --cached --name-only`) | `open_index()` + `head_tree_id()` + `tree_index_status(.., TrackRenames::Disabled, cb)` | **gix** | HIGH (executed inside a real `pre-commit` hook) | Honours `GIT_INDEX_FILE`. Matched git for a plain commit, `commit -a` (`index.lock`) and a partial commit (`next-index-*.lock`). On an unborn HEAD (first commit), `head_tree_id()` fails; use the empty tree (MEDIUM, not executed). |
| `<upstream>..HEAD` changed paths (`diff --name-only A..B`, pre-push guard) | `rev_parse_single("@{upstream}")` + `diff_tree_to_tree(old, new, opts)` | **gix** | HIGH (executed) | Matches git byte-for-byte on the fixture. **But see the guard gap below.** |
| Commits in range, and the first push with no upstream (`HEAD --not --remotes`) | `rev_walk([tip]).with_hidden(remote_tips).all()` + per-commit `diff_tree_to_tree` vs each parent | **gix** | HIGH (executed: matched `git log --name-only nopush --not --remotes`) | `references()?.remote_branches()` supplies the tips to hide. |
| Upstream lookup / no upstream (`rev-parse --abbrev-ref @{upstream}`) | `rev_parse_single("@{upstream}")`, `branch_remote_tracking_ref_name(name, Direction::Fetch)` → `None` | **gix** | HIGH (executed) | |
| Fetch from a remote (`git fetch <remote>` in `wsx new --fetch`) | `blocking-network-client` + a transport | **git CLI** | HIGH (source) for the transport facts; MEDIUM for the recommendation | gix's file:// transport spawns `git-upload-pack`, and its ssh:// transport spawns `ssh`. https needs a reqwest or curl + TLS stack. gix writes no `FETCH_HEAD`. So "pure-Rust fetch" still execs git or ssh while adding a network stack (crate count not measured). `git fetch` matches the user's credential helpers and ssh config exactly. |
| Is a path ignored (doctor / tests) | `excludes(&index, None, Source::WorktreeThenIdMappingIfNotSkipped)` → `at_entry(p, Some(Mode::FILE)).is_excluded()` | **gix** | HIGH (executed: matched `git check-ignore`, including the per-worktree override) | |
| Bare repo refusal (`config --get core.bare`) | `is_bare()` | **gix** | HIGH (executed) | |
| Branch-exists check before `new` (`rev-parse --verify refs/heads/<b>`) | `try_find_reference("refs/heads/<b>")` | **gix** | MEDIUM | **Trap (executed and source-confirmed):** `PreviousValue::MustNotExist` *succeeds silently* if the ref already exists with the same target. Check existence explicitly; do not rely on the constraint. |
| Reftable repositories (`extensions.refStorage=reftable`, Git 3.0's planned default) | — | **not supported** | HIGH (executed) | gix failed with "This reference uses an unsupported storage backend, such as reftable". The Python oracle works on these repos, so this is a parity regression. |
| SHA-256 repositories | `sha256` feature | **gix with feature** | HIGH (executed: open + `head_id` matched git) | Without the feature, opening fails ("Object format is not enabled in this build"). Adds about 77 KB. |
| Cleanup of gix lock files on Ctrl-C (`config.lock`, `index.lock`) | `interrupt` feature + `unsafe { gix::interrupt::init_handler(1, ..) }` | **gix** | MEDIUM (source, not executed) | Registers SIGINT and SIGTERM handlers that delete gix tempfiles and locks. Without it, a Ctrl-C during `config_file_mut` can leave a stale `config.lock` that blocks the user's next `git config`. |

**Repo-local env leak rule (HIGH, executed):**
- Plain `gix::discover` and `gix::open` of *another* path still apply `GIT_INDEX_FILE` from the environment; the probe opened main with a bogus index path.
- `main_repo()` and `Proxy::into_repo()` drop repository-local overrides.
- Rule: open the current worktree once, and reach every other worktree through `main_repo()` / `worktrees()`. When spawning `git` for a different worktree, `env_remove` `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_COMMON_DIR`, `GIT_PREFIX`.

**Guard gap the probe found (real defect, present in 0.1.0):**
- `git diff --name-only upstream..HEAD` compares only the two end trees.
- A managed file added in one commit and deleted in a later one never shows up in that diff, yet the push still sends it in history. Both git and gix showed this (`leak.md`).
- The commit walk in the table catches it. Use the walk for the pre-push guard and the first push alike. This is a deliberate divergence from the oracle.

### Recommended gix dependency line

```toml
gix = { version = "0.89", default-features = false, features = [
  "sha1",               # required object format
  "sha256",             # open SHA-256 repos (Git 3.0); +77 KB
  "worktree-mutation",  # remove_worktree (+ add_worktree); implies status, dirwalk, excludes, index, blob-diff
  "revision",           # rev_parse_single("@{upstream}"), rev_walk; without it rev_parse_single does not exist (compile-verified)
  "interrupt",          # lock/tempfile cleanup on SIGINT/SIGTERM
] }
```

Measured (probe binary, release, thin LTO, stripped, 4 cores, clean build):

| Feature set | Crates in tree | Binary | Clean release build |
|---|---|---|---|
| default features | 199 | 7.35 MB | 160 s |
| `sha1 + worktree-mutation + revision` | 171 | 6.19 MB | 111 s |
| + `sha256` | — | 6.26 MB | — |

`excludes` and `index` are implied, and adding them changed nothing. Leave out `blocking-network-client`, `blocking-http-transport-*`, `credentials`, `blame`, `merge`, `worktree-archive`, `worktree-stream`, `mailmap`, `revparse-regex`, `parallel`/`max-performance` (wsx touches few objects), and `serde`.

gix ships a breaking release roughly monthly (0.83 Apr → 0.89 Oct 2026). 0.89 changed every API to `gix::Result`. Keep gix behind one internal `git` module so each upgrade touches one file, commit `Cargo.lock`, and upgrade deliberately.

## Recommended Stack

### Core Technologies

| Technology | Version | Purpose | Why Recommended |
|---|---|---|---|
| Rust (edition 2024) | toolchain pinned to **1.99.0** (current stable on host); `rust-version = "1.89"` | Language | The floor is set by `std::fs::File::lock` (stable 1.89.0), gix 0.89 (1.88) and edition 2024 (1.85). Pin the toolchain with `rust-toolchain.toml` so clippy lints do not drift between runs. HIGH |
| gix | 0.89.0 | All git reads, config writes, worktree removal | See the table. Pure Rust, no C, no subprocess parsing. HIGH (executed) |
| `git` binary | ≥ 2.31 (host 2.56.0) | Fallback for `worktree add -b`, `fetch` | These are the only gaps where gix is missing behaviour wsx depends on (branch creation with tracking, hooks, transports). git is already a hard runtime requirement. HIGH |
| clap (derive) | 4.6.7 | CLI parsing (13 subcommands) | The standard choice. Derive keeps the 13-subcommand surface declarative. Usage errors exit 2, as argparse does. `clap_complete` 4.6.11 can later replace the hand-written fish completions. HIGH (version) |
| serde + serde_json | 1.0.229 / 1.0.154 | `base.json` v1 read, v2 read/write | Standard. Typed v1/v2 structs make the one-time upgrade explicit. HIGH |
| sha2 | 0.11.1 | sha256 of file contents | RustCrypto. Hex output matched Python `hashlib.sha256().hexdigest()` on three inputs (executed). **The 0.11 digest output does not implement `LowerHex`** (compile-verified; 0.10 did), so `format!("{:x}")` fails. Use a 6-line local hex encoder rather than a dependency. HIGH |
| thiserror | 2.0.21 | One typed `Error` enum | wsx's correctness depends on telling error kinds apart: "unreadable" vs "absent", guard-block vs fail-open, conflict vs problem. A typed enum forces the planner to handle each one. HIGH |

### Supporting Libraries

| Library | Version | Purpose | When to Use |
|---|---|---|---|
| tempfile | 3.27.0 | Unique same-directory temp files + atomic `persist()` (rename) | Every file write (`base.json`, pulled files, exclude files). Gives unique names like `.base.json.bIog0R.wsx-tmp` (executed), which fixes the fixed `*.wsx-tmp` name collision. `persist_noclobber` refuses existing targets (executed). HIGH |
| walkdir | 2.5.0 | Walking managed directories | Already in gix's tree, so it adds nothing. Use `follow_links(false)` and `sort_by_file_name()` for deterministic plans. **Treat every walk error as "state unknown"**, never as "file absent". Python's `os.walk` silently drops unreadable directories, which is the same data-loss path as the unreadable-file defect. HIGH |
| jiff | 0.2.38 | Local timestamps (`updated`, backup stamps) | Already in gix's tree, so it adds nothing. `strftime("%Y-%m-%dT%H:%M:%S%z")` matched Python's output exactly (executed). Add sub-second precision to backup stamps to fix the stamp-collision defect. HIGH |
| std only | — | File locking, mtimes, symlinks, colour, signals | `File::lock`/`try_lock` (flock; exclusivity executed), `File::set_times` (copy mtime like `shutil.copy2`), `symlink_metadata` / `os::unix::fs::symlink`, `io::IsTerminal` for Python-parity ANSI colour. No crates needed. HIGH |

### Development Tools

| Tool | Version | Purpose | Notes |
|---|---|---|---|
| cargo-nextest | 0.9.148 | Test runner | Process-per-test isolation suits fixtures that spawn git and wsx. `cargo nextest run`. HIGH |
| assert_cmd | 2.2.2 | Black-box CLI assertions | Wrap an explicit binary path (`Command::new(bin)`), not `cargo_bin`, so the same test runs the Python oracle. HIGH (version) |
| predicates | 3.1.4 | Output and file predicates for assert_cmd | HIGH |
| snapbox | 1.2.2 | Normalised output comparison with redactions (`[..]`, temp-path substitution) | Use it to compare oracle stdout with Rust stdout, and to **freeze oracle outputs as golden files before Python is deleted**. Once the oracle binary is gone, those files carry the spec. `SNAPSHOTS=overwrite` regenerates them. MEDIUM (approach) |
| proptest | 1.12.0 | Property tests for the planner (3-way classification) and the `.wsx.conf` parser | Required by PROJECT.md. HIGH |
| cargo-mutants | 27.1.0 | Mutation testing on plan/apply | `cargo mutants --test-tool nextest --file src/plan.rs` (flag confirmed locally). HIGH |
| cargo-llvm-cov | 0.9.1 | Coverage | `cargo llvm-cov nextest`. `--branch` is still unstable (nightly). Use line coverage on stable. HIGH |
| cargo-deny | 0.20.2 | Licences, advisories, bans, sources | The full candidate set below passed `cargo deny check`. A negative control (removing `Zlib` from the allow list) failed with exit 2, so the check is real. HIGH (executed) |
| cargo-machete | 0.9.2 | Unused dependencies | HIGH |

### Test harness shape (how the tools fit the oracle)

- Integration tests in `tests/` resolve the binary as `std::env::var("WSX_BIN")`, falling back to `env!("CARGO_BIN_EXE_wsx")`:
  - `WSX_BIN=$PWD/bin/wsx cargo nextest run` runs the suite against the Python oracle;
  - a plain `cargo nextest run` runs it against Rust.
- Each case builds its fixture in a `tempfile::TempDir` and starts every child process (git and wsx) from `env_clear()`, setting only:
  - `PATH`;
  - `HOME`, `XDG_CONFIG_HOME` (both point into the tempdir);
  - `GIT_CONFIG_NOSYSTEM=1`, `GIT_CONFIG_GLOBAL=<tmp>/gitconfig`;
  - `TZ=UTC`, `LANG=C.UTF-8`.
- No process-global `set_var`.
- Compare outcomes, not bytes:
  - exit code;
  - the resulting file tree;
  - **parsed** `base.json`. Python's `json.dumps` escapes non-ASCII (`"café"`) and serde_json writes raw UTF-8 (`"café"`), both executed, so byte comparison would fail spuriously.
- Compare stdout through snapbox redactions. Do not compare `--help` text: argparse and clap differ by design.
- Skip `trycmd`: its file-per-case format cannot express per-case git fixtures. Skip `assert_fs`: `tempfile` plus std covers it, and every dependency has to earn its place.

## Installation

```toml
[package]
edition = "2024"
rust-version = "1.89"

[dependencies]
gix = { version = "0.89", default-features = false, features = ["sha1", "sha256", "worktree-mutation", "revision", "interrupt"] }
clap = { version = "4.6", features = ["derive"] }
thiserror = "2"
sha2 = "0.11"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tempfile = "3"
walkdir = "2"
jiff = "0.2"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
snapbox = "1"
proptest = "1"

[profile.release]
strip = true
lto = "thin"
codegen-units = 1
# Do NOT set panic = "abort": `wsx hook` must catch panics (catch_unwind) and exit 0.
```

This exact set (plus `anyhow`, `assert_fs`; both since dropped) passed `cargo check --all-targets` and `cargo deny check` on 2026-10-17. The tree has 210 crates and **no C build dependencies**: no `cc`, `openssl` or `libz-sys`. zlib is `zlib-rs` and SHA-1 is `sha1dc`, both pure Rust.

`deny.toml` licence allow list needed: `MIT`, `Apache-2.0`, `Apache-2.0 WITH LLVM-exception`, `Unlicense`, `Zlib`, `Unicode-3.0`, `BSD-3-Clause`. Set `multiple-versions = "warn"`: hashbrown ×3 and syn ×2 come from gix and are not fixable downstream.

```bash
cargo install --locked cargo-nextest cargo-mutants cargo-llvm-cov cargo-deny cargo-machete   # all already present on this host
```

## Alternatives Considered

| Recommended | Alternative | When to Use Alternative |
|---|---|---|
| gix | git2 0.21.0 (libgit2) | Never for this project. It brings in a C library (`libgit2-sys`), which breaks the pure-Rust and musl story. Its worktree-config coverage was not evaluated here. |
| gix `remove_worktree` | `git worktree remove` | If the operator is uneasy relying on an API 9 days old. Semantics are equivalent, so swapping later is cheap behind the `git` module. |
| `git worktree add` CLI | gix `reference()` + config write + `add_worktree()` | Only if hook-free creation is wanted. You must then write `branch.<b>.remote/merge` yourself to keep the upstream and guard behaviour. |
| thiserror only | anyhow 1.0.104 | If error *classification* stops mattering. It does matter here: guard vs fail-open, unreadable vs absent. |
| std `File::lock` | fs4 1.1.0, fd-lock 4.0.4 | Only if the MSRV had to drop below 1.89. |
| clap derive | lexopt 0.3.2 / bpaf 0.9.28 | lexopt if binary size became critical; it is hand-rolled and gives no help or completions. Not worth it at about 6 MB with gix. |
| snapbox | insta 1.49.0 | insta's review workflow (`cargo insta review`) is nicer, but snapbox is the same family as assert_cmd and covers redacted differential comparison. Pick one. |
| glibc dynamic build (milestone 1) | static musl (`x86_64/aarch64-unknown-linux-musl`) | Milestone 2 distribution. The tree has no C dependencies, so musl should be a plain `rustup target add` + `--target` (UNVERIFIED: not built here). musl's slow allocator does not matter for an I/O-bound tool. For aarch64 cross builds, cargo-zigbuild 0.23.4. |

## What NOT to Use

| Avoid | Why | Use Instead |
|---|---|---|
| `ignore` crate for walking managed paths | It applies `.gitignore` and excludes. Managed files are git-ignored *by design*, so the walk would skip exactly the files wsx syncs. | walkdir |
| gix `blocking-network-client` / http transports | Adds a TLS stack and many crates (count UNVERIFIED, not built), and still spawns `git-upload-pack` / `ssh` for file and ssh remotes. | `git fetch <remote>` subprocess |
| `PreviousValue::MustNotExist` as the "branch already exists" check | Silently succeeds when the existing ref has the same target (executed + source). | An explicit `try_find_reference` before creating |
| `gix::open` / `discover` on a second worktree from inside a hook | `GIT_INDEX_FILE` leaks in, so it reads the hook's temp index for the wrong worktree (executed). | `main_repo()` / `worktrees()` proxies |
| `println!` in hook paths | Rust ignores SIGPIPE, so a closed stdout makes `println!` **panic** (seen in the probe under `\| head`). That breaks "hooks never raise". | `writeln!` with the error ignored, inside `catch_unwind` |
| `panic = "abort"` | Makes `catch_unwind` useless, so `wsx hook` could no longer fail open. | Default unwind |
| `camino` (UTF-8-only paths) | It would reject non-UTF-8 file names that a sync tool must at least report instead of crashing on. | `std::path::Path` / `OsStr` with an explicit policy for JSON keys |
| miette, color-eyre | Fancy diagnostics with heavy dependency trees, of no use to a hook tool whose output is often discarded. | thiserror + plain stderr |
| `time` crate for local offsets | Its local-offset lookup is restricted when multithreaded (UNVERIFIED here, from crate docs as remembered). It is also redundant, since jiff is already in the tree. | jiff (already in tree) |
| `std::fs::copy` alone | It copies permissions but not mtime. `lww` and the stat cache depend on mtime parity with `shutil.copy2`. | `fs::copy` + `File::set_times`. Note that copy2 also copies xattrs; dropping that is acceptable but is a divergence. |
| gitoxide `crate-status.md` / Context7 for gix capability | Stale: it marks the worktree and config-write work as missing or new. | The released crate source / docs.rs for the pinned version |

## Stack Patterns by Variant

**If running as `wsx hook <name>` (git or agent hook):**
- Open with `discover_with_environment_overrides`, so `GIT_INDEX_FILE` and `GIT_DIR` from git apply.
- Install `gix::interrupt::init_handler`.
- Wrap the body in `catch_unwind` and return 0, except for a guard block.
- Write output with `writeln!`, ignoring errors.

**If running an interactive command (`new`, `rm`, `list`):**
- The same entry point.
- Spawn `git` (worktree add, fetch) with the repo-local env vars removed when the target is a different worktree.

**If the repo uses reftable:**
- gix cannot open it. The roadmap must pick a behaviour (see below).

## Version Compatibility

| Package A | Compatible With | Notes |
|---|---|---|
| gix 0.89.0 | rustc ≥ 1.88 | Pulls gix-ref 0.69, gix-worktree 0.58, gix-status 0.36. A stale local index once failed to resolve gix-archive 0.38 under default features; `cargo update` fixed it. |
| sha2 0.11.1 | digest 0.11.3, hybrid-array 0.4.15 | No `LowerHex` on the output. |
| `std::fs::File::lock` | rustc ≥ 1.89.0 | flock(2) on Linux; the lock is released when the fd closes. Rust opens files `O_CLOEXEC`, so spawned git and npm do not inherit it. |
| cargo-nextest 0.9.148 | toolchain ≥ 1.91 to build the tool | Irrelevant to the crate MSRV. |

## Open decisions for the roadmap (operator)

1. **Worktree creation path.** gix can do it, but without `-b`, without upstream tracking and without hooks. Recommendation: use the `git worktree add` CLI. This is the operator's own stated fallback criterion (gix lacks the needed behaviour), not a preference.
2. **Reftable repos.** Python 0.1.0 works on them; gix 0.89 cannot open them. Options: (a) detect `extensions.refStorage=reftable` and refuse with a clear message (recommended for milestone 1, since the author's repos use the files backend), or (b) fall back to git CLI for everything (doubles the code paths).
3. **Pre-push guard semantics.** Parity (end-tree diff) vs hardening (commit walk, which catches add-then-delete leaks). Recommendation: the commit walk, logged as a deliberate divergence from the oracle.

## Sources

- crates.io API (`/api/v1/crates/<name>`), 2026-10-17: every version above.
- `gix-0.89.0.crate` and `gix-0.88.0.crate` from static.crates.io (0.88.0 has no `worktree/add.rs` or `remove.rs`, so the APIs are new in 0.89.0):
  - `src/worktree/add.rs`, `src/worktree/remove.rs`, `src/repository/config/mod.rs`, `src/config/file_mut.rs`, `src/discover.rs`, `src/interrupt.rs`, `Cargo.toml` `[features]`.
  - Permalinks at VCS sha 0115e902: https://github.com/GitoxideLabs/gitoxide/blob/0115e902fd0334b8f370766993eb17babed3157f/gix/src/worktree/add.rs and …/gix/src/worktree/remove.rs
- docs.rs: https://docs.rs/gix/0.89.0/gix/struct.Repository.html (methods `add_worktree`, `remove_worktree`, `config_file_mut`, `tree_index_status`, `rev_walk`).
- gitoxide `gix/CHANGELOG.md` (main @ ad0202e3, 2026-10-17): the worktree add/remove and `config_file_mut` entries. `gix-transport/src/client/blocking_io/file.rs`: spawning `git-upload-pack`. `SHORTCOMINGS.md`, `crate-status.md` (stale).
- https://doc.rust-lang.org/std/fs/struct.File.html: `lock`/`try_lock` stable since 1.89.0, flock on Unix.
- https://docs.rs/assert_cmd/2.2.2: `cargo_bin` macro and function, `Command` wrapper.
- Context7 `/gitoxidelabs/gitoxide`: checked; it only surfaced the stale `crate-status.md` for worktree mutation.
- Local execution (git 2.56.0, rustc 1.99.0): the gix probe across the read/mutate/remove/hook steps; reftable and sha256 repo opens; the sha2, `File::lock`, tempfile and jiff probe; `cargo deny check` with a negative control; `cargo check --all-targets` of the full candidate set.

---
*Stack research for: wsx Rust rewrite (milestone 1)*
*Researched: 2026-10-17*
