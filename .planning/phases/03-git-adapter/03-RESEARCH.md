# Phase 3: Git adapter - Research

**Researched:** 2026-10-26
**Domain:** gix 0.90.0 reads, `git` binary spawning, hook environments, repository trust and reftable refusal, all inside the binary crate's unit tests
**Confidence:** HIGH for the gix API, hook environment, trust, reftable and test mechanics (each executed this session with a control). MEDIUM for CI-runner capability claims (executed in the pinned image under rootless Docker, not on a GitHub runner).

## Summary

gix **0.90.0** (released 2026-10-22) is the newest release, not 0.89. Every API this phase needs was compiled and run against 0.90.0 this session, so the STATE.md concern about unverified 0.89 method names is settled by the newer version. With `default-features = false` and features `["sha1", "sha256", "revision"]`, the tree has 99 crates and no C build dependencies. All four read queries work with that set, and their results matched `git` on a criss-cross, clock-skewed fixture; each query had a control that changed the answer. Tree diff must use the low-level `gix::diff::tree` + `Recorder`: `Repository::diff_tree_to_tree` needs the `blob-diff` feature, which adds 14 crates (attributes, filter, pathspec, submodule). cargo-deny accepts the graph once `Apache-2.0`, `Unicode-3.0` and `Zlib` are added to `deny.toml`.

The hook and trust behaviour was observed directly. In a linked worktree, pre-commit, commit-msg and post-commit export an absolute `GIT_DIR` and `GIT_INDEX_FILE`. In the main worktree `GIT_DIR` is **unset** and `GIT_INDEX_FILE` may be **relative** (`.git/index`). `discover_with_environment_overrides_opts` honours both. `main_repo()`/`worktrees()` give siblings their own index. A git child aimed at a sibling with the hook env intact reports the hook's git dir and `index.lock`; with the `--local-env-vars` list removed it reports the sibling's. That is criterion 2's leak control, executed. gix 0.90 opens a reftable repo without error and fails only when it decodes a ref, so the refusal must come from `extensions.refStorage`, which gix reads cleanly. gix refuses foreign-owned repos and honours `safe.directory` in global config (exact path, `*`, `<dir>/*`). It **ignores** `-c safe.directory` and `GIT_CONFIG_COUNT`, which git honours.

A foreign-owned repo can be created for real on both targets. Under root (CI and the container), `chown -R 65534:65534` creates one; this was executed in the pinned image, and git and the gix probe both refused. As the unprivileged host user, `unshare --map-root-user --map-users=auto --map-groups=auto chown -R 1:1 <repo>` creates one owned by subuid 524288; this was executed, and the same command with `0:0` restores ownership. Tests whose subject depends on process environment (hook context, trust, safe.directory, ceiling) must run in a **re-executed child of the test binary**, because `set_var` is banned. That pattern works under nextest, but a wrong child name exits 0 with "running 0 tests", so the parent must assert a marker.

**Primary recommendation:** Pin `gix = { version = "=0.90.0", default-features = false, features = ["sha1", "sha256", "revision"] }`. Open only through `ThreadSafeRepository::discover_with_environment_overrides_opts` with `match_ceiling_dir_or_error = false`. Refuse on `git_dir_trust() != Trust::Full`, then on `extensions.refStorage` not `files`. Diff with `gix::diff::tree`. Spawn git from an explicit env snapshot with `env_clear()`. Use `#![cfg_attr(not(test), expect(dead_code, reason = ...))]`, not a bare `expect`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

### How phase 3 is tested
- **Git-layer tests are unit tests inside the binary crate** (`src/git/`), run against real
  temporary repos built with the `git` binary. `tempfile` is already a dev-dependency.
  - **Why:** there is no library crate (owner, 2026-10-20), so integration tests under `tests/`
    cannot reach the module, and no command exists yet for the harness to drive.
  - Nothing in `main` calls the module until phase 5, so it carries a module-level
    `#[expect(dead_code, reason = "...")]`. Phase 5 removes it when `check` becomes the first
    caller; `expect` turns into a warning once the reason is gone.
- **Reftable `regression:` scenario (criterion 3).** Phase 3 proves the refusal, with a
  files-backend control that opens, in unit tests. It also registers the harness `regression:`
  scenario with the oracle side recorded from a real run and the Rust side disabled. Research
    found that the Python 0.1.0 works on reftable repos; whether the shell-script oracle does is
    unverified, and the recorded run settles it.
  - Phase 5's `check` enables that scenario's Rust side. Record the obligation in STATE.md
    Blockers/Concerns so phase 5 cannot miss it.
- **gix-vs-git parity (criterion 1) is a differential test.** Each read query runs through gix and
  through `git` on the same fixture, and the results must be equal. Each query has a control that
  must change the answer (for example an extra commit that moves the merge-base, or a commit that
  changes ahead/behind).
- **Foreign-owner refusal (criterion 4) is proven against a repo really owned by another uid.**
  git's `GIT_TEST_ASSUME_DIFFERENT_OWNER` only affects git, not gix, so it cannot prove wsx's
  behaviour; use it only on the git side of the comparison. Research picks the mechanism (for
  example a user namespace).
  - A test that cannot create the condition **fails; it never skips**.
  - Control: the same repo listed in `safe.directory` opens.

### Running the `git` binary (GIT-01, GIT-02)
- **The variables to remove come from `git rev-parse --local-env-vars` at runtime, queried once per
  process.** That is exactly the list GIT-02 names, and it follows the user's git version. The
  query needs no repo, so it works inside hooks.
- **Removal applies to any child whose target worktree differs from wsx's own resolved worktree.**
  A child aimed at wsx's own worktree keeps the hook's environment, so a pre-commit's
  `GIT_INDEX_FILE` (partial commit) still applies.
  - **Every child gets a null stdin and an explicit working directory**, whichever worktree it
    targets.
  - Criterion 2's control: with removal disabled, the scenario detects the leak.
- **By default the spawn helper adds nothing that bypasses hooks or signing:** no `--no-verify`,
  no `-c commit.gpgsign=false`. A caller that needs one passes it explicitly for that call. Phase
  9's replay commits will need `--no-verify` (PRCUT-02).
- **git ≥ 2.54 is checked once, on the first `git` spawn.** An older git is refused with a message
  naming the version found and the version required.

### Opening the repository and refusals
- **One entry point that honours `GIT_DIR`/`GIT_INDEX_FILE` the way git does** (gix's
  environment-override discover; the research names
  `ThreadSafeRepository::discover_with_environment_overrides`).
  - Sibling worktrees are reached only through gix's main-repo and worktree-list calls, which
    switch the repository environment overrides off.
  - A plain `gix::open`/`gix::discover` is banned through `clippy.toml` `disallowed-methods`, with
    the single allowed call site marked `#[expect(clippy::disallowed_methods, reason = "...")]`.
- **Reftable is detected from `extensions.refStorage` in the repo config, before refs are
  touched.** gix's "unsupported storage backend" error maps to the same refusal as a backstop.
  The message names reftable and says this version of wsx cannot read it.
- **SHA-256 repositories are supported:** gix's `sha256` feature is on (research measured about
  77 KB), with one open test.
- **Paths are canonicalised once, when the repo is resolved:** the worktree, the git dir and the
  common dir. On this host `/home` is a symlink to `/var/home`, and gix returns `common_dir` as
  `<git-dir>/../..`. Code downstream compares only canonical paths.

### Code shape and dependencies
- **gix is pinned exactly (`=x.y.z`) with `default-features = false`.** Only the features used are
  enabled: `sha1`, `sha256` and revision walking.
  - No network, `worktree-mutation` or `interrupt` features. `interrupt` installs signal handlers
    that would compete with wsx's own SIGINT handling.
  - The version is the newest one the phase-start spike verifies; the carried concern says the
    0.89 method names are unverified.
  - Every later gix upgrade goes in its own commit, with the full harness.
  - The dependency needs its written justification and a clean `scripts/check.sh deps`.
- **`src/git/` holds one concrete gix reader and one concrete `git` spawn helper, with no trait.**
  gix types never leave the module: callers get wsx's own types (object IDs, raw-byte paths).
- **Errors are a hand-written enum, with no new error crate.** It has one variant per refusal:
  reftable, foreign owner, git too old, not a repo, and a git failure carrying its argv, exit status
  and stderr. Exit codes stay phase 5's contract.
- **Scope of reads:** repo discovery plus exactly the four queries criterion 1 names (merge-base,
  ahead/behind, tree diff, range walk). Phase 4 adds its own config and blob reads.

### Claude's Discretion
- File layout inside `src/git/` and the names of the wsx-owned types.
- The exact refusal wording, as long as each message names its cause (reftable; foreign owner,
  with the `safe.directory` fix git itself suggests; git version found against required).
- Fixture construction helpers for the unit tests.

### Deferred Ideas (OUT OF SCOPE)
- Config reads (`wsx.sourceBranch`, `wsx.baseRef`) and reading `<tip>:.wsx.conf` from the object
  database: phase 4.
- Exit codes and `--json` for refusals: phase 5 (IF-01).
- Enabling the reftable `regression:` scenario's Rust side: phase 5, with `wsx check`.
- Reftable support itself: before the public release (PROJECT.md key decision).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| GIT-01 | History-changing operations (merge, cherry-pick, branch and worktree creation, commit, fetch) go through the `git` binary, so the user's hooks, signing, merge drivers and credentials apply. Read-only analysis uses gix | Signing and hook proof executed: SSH signing with a throwaway key gives `%G?` = `G` on commits and a `--no-ff` merge. Hooks fire (`pre-commit`, `commit-msg`, `pre-merge-commit`); the `--no-verify -c commit.gpgsign=false` control gives `N` and no hook runs. The four reads were verified against git (see "Read queries") |
| GIT-02 | Child git processes get the repo-local variables `git rev-parse --local-env-vars` lists removed when they target another worktree, plus a null stdin. wsx resolves its own worktree and common dir correctly when a hook exports `GIT_DIR`/`GIT_INDEX_FILE` | Hook env, gix resolution, the leak and its scrub control all executed (see "Hook environment"); null-stdin control via `git hash-object --stdin` |
| GIT-03 | A reftable repo is refused with a clear message (gix 0.89 can't open it; a `regression:` scenario) | gix 0.90 **opens** reftable and fails only on ref decode; `extensions.refStorage` reads as `Some("reftable")`. The shell oracle's `check` works on reftable (executed); fixture via `git refs migrate --ref-format=reftable` |
| GIT-04 | A repo owned by another user is refused, as git does | Real foreign-uid fixture on both targets (executed); gix and git both refuse; `safe.directory` control opens both; divergences documented |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- One binary crate `wsx`, edition 2024, **no library crate**; never run `cargo init`/`cargo new`.
- Gate: `scripts/check.sh all` (fmt, clippy `-D warnings`, nextest with `--no-tests=fail`); `scripts/check.sh deps` (cargo-deny) is separate and must be clean for the new dependency.
- One named test per gate command: `scripts/check.sh test -E 'test(=module::name)'`; never `test(=a) | test(=b)`.
- `std::env::set_var`/`remove_var` are banned by `clippy.toml`; scope env with `Command::env`/`env_remove`/`env_clear`.
- `#[expect(..., reason = "...")]`, never `allow`.
- Add a dependency only with a written justification and a clean `scripts/check.sh deps`.
- The tool never prompts; never resolves conflicts on the user's behalf.
- Commit subjects `type(03-NN): ...`.
- Container mounts the checkout read-only; tests that write must use a scratch dir (target/ is a writable volume).
- New `.gitignore` lines at the top; scripts that reach develop never go under `scripts/workspace/`.
- The Python 0.1.0 (`bin/wsx`, `tests/sandbox.sh`) is frozen reference; do not change it.
- Agents do not push, open PRs or change repo settings.
- User global rules: negative control for every check; scratch on disk, never RAM-backed `/tmp`; never fail closed silently (every refusal names its cause).

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Repo resolution (worktree, git dir, common dir, canonical paths) | `src/git/` gix reader | — | Single entry point; reads process env (`GIT_DIR`, `GIT_INDEX_FILE`, ceilings) exactly once |
| Trust and reftable refusals | `src/git/` gix reader (right after open) | wsx error enum | Typed checks on the opened repo before any ref is touched |
| Read queries (merge-base, ahead/behind, tree diff, range walk) | `src/git/` gix reader | — | Pure object-database reads; return wsx types |
| History changes (commit, merge, branch, worktree add, fetch, cherry-pick) | `src/git/` git spawn helper → `git` binary | user's hooks/signing/credentials | GIT-01: the user's own git config must apply |
| Env scrubbing and null stdin for children | `src/git/` spawn helper | `git rev-parse --local-env-vars` | GIT-02 |
| git version gate | spawn helper, first spawn | — | Locked decision |
| Reftable `regression:` scenario | `tests/harness/check.rs` | oracle adapter | Oracle side recorded now, Rust side enabled in phase 5 |

## Standard Stack

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| gix | **=0.90.0** (2026-10-22; MSRV 1.88) [VERIFIED: crates.io API `crates/gix/versions`; built and run this session] | discovery, trust, refStorage read, merge-base, rev walk, tree diff | Pure Rust, no C deps; the project's chosen reader |
| `git` binary | ≥ 2.54 (host 2.56.0; CI image `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie` has 2.56.0) [VERIFIED: `git --version` on host and in the image] | every history change | GIT-01 |

Features: `default-features = false, features = ["sha1", "sha256", "revision"]`. This exact set compiled with every API used below [VERIFIED: probe build].

| Feature set (gix 0.90.0, `default-features = false`) | Unique crates in `cargo tree -e normal` (excl. root) |
|---|---|
| `sha1` | 88 |
| `sha1, revision` | 92 |
| **`sha1, sha256, revision`** (recommended) | **99** |
| `+ blob-diff` (needed only for `Repository::diff_tree_to_tree`) | 113 |
| `+ worktree-mutation, interrupt` | 126 |

Build impact (recommended set, a probe that exercises discovery, walk and tree diff): cold `cargo build --release` took **73.6 s** wall-clock on 4 cores; the binary is 4.5 MB unstripped and 3.3 MB stripped [VERIFIED: measured]. The default release profile was used, with no LTO. These numbers come from the probe binary, not from wsx.

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| tempfile (dev, existing) | 3.27.0 | scratch repos | Every unit test, via `TempDir::new_in(<target>/tmp)` |
| `ssh-keygen` (system) | present on host and image [VERIFIED: `command -v` in image] | throwaway signing key for criterion 1 | Signing test |
| `/usr/bin/unshare` (util-linux 2.41.5 host; present in image) + `/usr/bin/newuidmap` | [VERIFIED] | foreign-owner fixture on an unprivileged host | Only when euid ≠ 0 |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `gix::diff::tree` + `Recorder` | `Repository::diff_tree_to_tree` | Needs `blob-diff` (+14 crates, pulls attributes/filter/pathspec); applies config-driven rename tracking, which then diverges from `git diff-tree --no-renames` |
| Typed `git_dir_trust()` check | `bail_if_untrusted(true)` | The bail error is an untyped `gix::Error` message ("…considered unsafe as it's not owned by the current user"); mapping it to the `ForeignOwner` variant would need string matching |
| Two hidden walks for ahead/behind | a single painted walk | gix has no ahead/behind API; two `rev_walk().with_hidden()` counts matched `git rev-list --left-right --count` on every pair tried |

**Installation:**
```toml
[dependencies]
gix = { version = "=0.90.0", default-features = false, features = ["sha1", "sha256", "revision"] }
```

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| gix | crates.io | first published 2023-02-10 per seam (gix 0.x line); 0.90.0 published 2026-10-22 | ~1.0M/wk | github.com/GitoxideLabs/gitoxide | OK | Approved |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
Crate ecosystem: no postinstall concept; `cargo tree -e normal,build` shows no `cc`, `openssl`, `libz-sys`, `cmake` or `pkg-config` [VERIFIED]. zlib is `zlib-rs`, SHA-1 is `sha1dc`.

### cargo-deny (deny.toml) acceptance

Current policy [VERIFIED: deny.toml:17]: `allow = ["MIT"]`.

Run against the recommended graph with the repo's own `deny.toml` (cargo-deny 0.20.2):
- As-is: `licenses FAILED` (rc 4). Rejected crates: `dunce` (`CC0-1.0 OR MIT-0 OR Apache-2.0`), `foldhash` and `zlib-rs` (`Zlib`), `unicode-bom` (`Apache-2.0`), `unicode-ident` and `unicode-normalization` (`(MIT OR Apache-2.0) AND Unicode-3.0`).
- With `allow = ["Apache-2.0", "MIT", "Unicode-3.0", "Zlib"]`: `advisories ok, bans ok, licenses ok, sources ok`, rc 0. One warning: `found 2 duplicate entries for crate 'hashbrown'`, which is allowed under `multiple-versions = "warn"`.
- Negative control: the as-is run above failed on exactly these licenses.
- The probe's graph had no dev-dependencies. wsx's `tempfile` graph already passes with MIT, so the combined graph is expected to pass. Planner: re-run `scripts/check.sh deps` on the real tree.

## Architecture Patterns

### System Architecture Diagram

```
 process env (GIT_DIR, GIT_INDEX_FILE, GIT_CEILING_DIRECTORIES, HOME/GIT_CONFIG_GLOBAL)
        │
        ▼
 Repo::discover(cwd) ──► gix::ThreadSafeRepository::discover_with_environment_overrides_opts
        │                    (match_ceiling_dir_or_error=false, trust map: no bail)
        ├─ open error ───────────────────────────────► Error::NotARepo
        ├─ git_dir_trust() != Full ──────────────────► Error::ForeignOwner{path}   (safe.directory already applied by gix)
        ├─ extensions.refStorage present && != files ► Error::Reftable
        ▼
 canonicalise worktree / git_dir / common_dir once  ──► Repo { own_worktree, git_dir, common_dir, … }
        │
        ├─ reads (gix): merge_base · ahead_behind · tree_diff · range_walk  ──► wsx ObjectId / Vec<u8> paths
        │      (ref-decode "unsupported storage backend" ──► Error::Reftable backstop)
        │
        ├─ siblings: main_repo() / worktrees() → Proxy::into_repo*  (env overrides OFF)
        │
        └─ Git spawn helper (env snapshot captured once)
               first spawn: `git --version` (≥2.54 else Error::GitTooOld)
                            `git rev-parse --local-env-vars` → removal list (OnceLock)
               per child:  env_clear + envs(snapshot) − (removal list if target ≠ own_worktree)
                           stdin=null, current_dir=target, args as given (no --no-verify by default)
               non-zero exit ──► Error::GitFailed{argv, status, stderr}
```

### Recommended Project Structure
```
src/
├── main.rs            # adds `mod git;` only
└── git/
    ├── mod.rs         # #![cfg_attr(not(test), expect(dead_code, reason = …))], Error enum, wsx types
    ├── repo.rs        # the single gix entry point, refusals, canonical paths, four reads
    ├── spawn.rs       # Git helper: env snapshot, removal list, version gate, null stdin
    └── tests/ (or #[cfg(test)] mod tests in each file)
        ├── fixture.rs # hermetic git fixture builder, scratch-dir resolver, re-exec helper
        └── …
```

### Pattern 1: Single entry point with typed refusals
**What:** Open once through `discover_with_environment_overrides_opts`, then refuse on trust, then on refStorage.
**When to use:** The only way wsx opens a repository.
**Example (compiled and run against gix 0.90.0 this session; names are from the probe):**
```rust
// Source: probe built on gix 0.90.0 (src: gix/src/discover.rs:76, open/options.rs:136,193)
use gix::sec::trust::DefaultForLevel;
let trust_map = gix::sec::trust::Mapping {
    full: gix::open::Options::default_for_level(gix::sec::Trust::Full),
    reduced: gix::open::Options::default_for_level(gix::sec::Trust::Reduced), // no bail: check trust ourselves
};
#[expect(clippy::disallowed_methods, reason = "the single sanctioned way wsx opens a repository")]
let ts = gix::ThreadSafeRepository::discover_with_environment_overrides_opts(
    cwd,
    gix::discover::upwards::Options { match_ceiling_dir_or_error: false, ..Default::default() },
    trust_map,
)?; // map Err -> NotARepo (include the gix message)
let repo = ts.to_thread_local();
if repo.git_dir_trust() != gix::sec::Trust::Full { /* Error::ForeignOwner */ }
match repo.config_snapshot().string("extensions.refStorage") {
    Some(v) if v.as_ref() != "files" => { /* Error::Reftable */ }
    _ => {}
}
let common = std::fs::canonicalize(repo.common_dir())?;   // raw value is "<git-dir>/../.."
```

### Pattern 2: Tree diff without `blob-diff`
```rust
// Source: probe on gix 0.90.0; gix-diff 0.70.0 src/tree/function.rs (`pub fn diff`), recorder.rs
use gix::prelude::FindExt;             // required for find_tree_iter (compile error without it)
let (mut ba, mut bb) = (Vec::new(), Vec::new());
let ia = repo.objects.find_tree_iter(&tree_a, &mut ba)?;
let ib = repo.objects.find_tree_iter(&tree_b, &mut bb)?;
let mut rec = gix::diff::tree::Recorder::default();      // Location::Path by default
gix::diff::tree(ia, ib, gix::diff::tree::State::default(), &repo.objects, &mut rec)?;
// rec.records: recorder::Change::{Addition, Deletion, Modification}{ entry_mode, path: BString, .. }
// Drop entries whose mode is a tree (directory nodes) to match `git diff-tree -r`.
// Modification with differing mode kind == git's `T` (type change).
```

### Pattern 3: Merge-base, ahead/behind, range walk
```rust
// Source: probe on gix 0.90.0 (repository/revision.rs:57, :163; revision/walk.rs:251, :331)
let mb: Option<gix::Id<'_>> = repo.merge_base(a, b)?;              // first of the best bases
let ahead  = repo.rev_walk([a]).with_hidden([b]).all()?.count();   // = git rev-list b..a --count
let behind = repo.rev_walk([b]).with_hidden([a]).all()?.count();
let range: Vec<_> = repo.rev_walk([tip]).with_hidden(hidden_tips).all()?
    .map(|info| info.map(|i| i.id)).collect::<Result<_, _>>()?;  // order is NOT git's; compare as sets
```

### Pattern 4: Spawn helper built on an env snapshot (testable without `set_var`)
**What:** The helper owns an explicit `Vec<(OsString, OsString)>`. Production captures it with `std::env::vars_os()` at construction; tests pass a hermetic one. Every child is `Command::new("git").env_clear().envs(filtered).stdin(Stdio::null()).current_dir(target)`. The variables removed are the `--local-env-vars` list, applied only when `canonical(target) != own_worktree`. A test-only switch disables the removal for the leak control.
**Why:** Removing variables from an explicit snapshot can be unit-tested in-process. The real hook case is proven in a re-exec child (Pattern 5).

### Pattern 5: Re-exec the unit-test binary as a child (env-dependent subjects)
**What:** The parent test spawns `std::env::current_exe()` with `["--exact", "git::…::child_name", "--include-ignored", "--nocapture"]`, a hermetic `env_clear()` env and a null stdin. The child test is `#[ignore = "child half of <parent>; runs only when re-executed"]`. It reads its inputs from env vars and prints a marker line.
**Verified:** Under nextest 0.9.150, `current_exe()` is `target/debug/deps/<crate>-<hash>`, the child ran, and nextest reported the child as skipped (`1 passed, 1 skipped`).
**Trap (executed):** a wrong child name prints `running 0 tests … ok` and **exits 0**. The parent must assert the child's marker line, never the exit status alone.
**Hook variant:** The fixture's hook script `exec`s the test binary with the same arguments. git then supplies the real `GIT_DIR`/`GIT_INDEX_FILE` to wsx's code, and the child prints what wsx resolved.

### Anti-Patterns to Avoid
- **Module-level `#![expect(dead_code, …)]` without `cfg_attr`:** executed. `cargo clippy --all-targets -D warnings` fails with `error: this lint expectation is unfulfilled` on the test target, because the tests use the items. Use `#![cfg_attr(not(test), expect(dead_code, reason = "…"))]`; that passed. Corollary: under `cfg(test)` every item must actually be used by some test, or `dead_code` fires.
- **In-process gix opens that depend on env:** the test process inherits the developer's `~/.gitconfig` (on this host: `commit.gpgsign = true`, `core.hooksPath`, `gpg.format = ssh`), and CI's global config holds a `safe.directory`. Trust, safe.directory, hook and ceiling tests run in a re-exec child with `HOME`/`GIT_CONFIG_GLOBAL`/`GIT_CONFIG_NOSYSTEM` hermetic. Pure query tests may run in-process.
- **`git diff` on the git side of parity:** porcelain `git diff` follows `diff.renames` (default on) and gave `R050 f/x f/z` where gix gave `A f/z` + `D f/x`. Use plumbing `git diff-tree -r --no-renames --name-status` (or `-z`).
- **Comparing walk order:** gix's default sort is breadth-first topological; git's is reverse-chronological. Compare sorted sets.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Ownership + `safe.directory` (`*`, `<dir>/*`, `~` interpolation, realpath) | own `stat().uid` compare + config parse | gix trust (`git_dir_trust()`) | gix already implements git's matching (verified: exact, `*`, `<dir>/*`) |
| Which env vars are repo-local | hard-coded list | `git rev-parse --local-env-vars` (runtime, once) | List follows git version; 15 names on 2.56 incl. `GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_COUNT` |
| Hook env → repo resolution | parsing `GIT_DIR` yourself | `discover_with_environment_overrides_opts` | Handles relative `GIT_INDEX_FILE`, gitfile resolution, ceilings |
| Merge-base | own LCA | `Repository::merge_base` | Criss-cross handling |
| Signing / hooks | invoking ssh-keygen/gpg yourself | `git commit`/`git merge` via the helper | GIT-01 |

## Runtime State Inventory

Not a rename/refactor phase. Omitted.

## Hook environment (executed, git 2.56.0, gix 0.90.0)

Linked worktree `wt1`, hooks in the common `.git/hooks`:

| Hook | `GIT_DIR` | `GIT_INDEX_FILE` | `GIT_WORK_TREE` | `GIT_PREFIX` |
|---|---|---|---|---|
| pre-commit (partial `git commit b`) | `<main>/.git/worktrees/wt1` (absolute) | `<main>/.git/worktrees/wt1/next-index-<pid>.lock` | unset | set, empty |
| pre-commit (`commit -a`) | same | `<…>/worktrees/wt1/index.lock` | unset | empty |
| commit-msg | same | same as pre-commit | unset | empty |
| post-commit | same | `<…>/worktrees/wt1/index` | unset | empty |
| reference-transaction | same | **unset** | unset | empty |

Main worktree: pre-commit had **`GIT_DIR` unset**. `GIT_INDEX_FILE` was absolute `…/.git/index.lock` or `…/.git/next-index-<pid>.lock`, and for a plain commit of staged changes it was the **relative** `.git/index`. gix resolved the relative path correctly against the hook's cwd (the top level).

gix inside the wt1 pre-commit reported `git_dir = …/worktrees/wt1` and `common_dir = …/worktrees/wt1/../..` (canonical: `…/main/.git`). It reported `workdir = …/wt1`, and `index_path = …/next-index-<pid>.lock`, the hook's index. `main_repo()` gave index `…/main/.git/index`. `worktrees()` → `into_repo_with_possibly_inaccessible_worktree()` gave `…/worktrees/wt1/index` and `…/worktrees/wt2/index`, each worktree's own (not leaked).

**Leak and control (criterion 2), from inside wt1's pre-commit:**
- `git -C wt2 rev-parse --absolute-git-dir` with the hook env intact → `main/.git/worktrees/wt1`, and `--git-path index` → `…/wt1/index.lock`. **Leak observed.**
- Same command under `env -u <each name from --local-env-vars>` → `main/.git/worktrees/wt2`. **Scrubbed.**

`git rev-parse --local-env-vars` run from `/` (no repo) exits 0 and prints the same list inside a hook [VERIFIED]:
`GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_CONFIG GIT_CONFIG_PARAMETERS GIT_CONFIG_COUNT GIT_OBJECT_DIRECTORY GIT_DIR GIT_WORK_TREE GIT_IMPLICIT_WORK_TREE GIT_GRAFT_FILE GIT_INDEX_FILE GIT_NO_REPLACE_OBJECTS GIT_REPLACE_REF_BASE GIT_PREFIX GIT_SHALLOW_FILE GIT_COMMON_DIR`. The pinned CI image printed the identical list.

**Null-stdin proof:** `git hash-object --stdin </dev/null` prints `e69de29bb2d1d6434b8b29ae775ad8c2e48c5391` (the empty blob) [VERIFIED]. Test: the parent re-execs the child with piped stdin containing a marker. In the child, the helper's `git hash-object --stdin` must print the empty-blob id. Control: the same command with stdin inherited prints the marker's blob id. nextest already gives tests a null stdin, so the control is meaningless without the re-exec.

## Read queries: gix vs git (executed)

Fixture: a criss-cross history (main merged `feat~1`, feat merged `main~1`) with a clock-skewed commit (committer time older than its parent), a rename, a deletion and a new directory.

| Pair | merge-base (git = gix) | `git merge-base --all` | ahead/behind (git = gix) | walk `A --not B` | diff (`diff-tree --no-renames`) |
|---|---|---|---|---|---|
| main feat | 05eb66a | 05eb66a, c9cd6f3 (two bases) | 2 / 4 | same | same |
| feat main | 05eb66a | two bases | 4 / 2 | same | same |
| main~1 feat~2 | 05eb66a | two bases | 1 / 2 | same | same |
| main feat~3 | c9cd6f3 (control: base moves) | one | 3 / 1 | same | same |
| main~2 feat | 05eb66a | one | 0 / 5 | same | same |

Controls that came out differently, as they must: `git diff -M` vs gix → DIFF (`R050`); walk with another hidden tip → DIFF; the `main feat~3` row moves the merge-base and changes ahead/behind. Type change file→symlink: git `T q`, gix `Modification` with a changed mode, so the mapping is needed. File→directory: both report `D p` + `A p/inner`.

**Limits:** gix picked the same single base as git in this criss-cross case only. That shows it can match git, not that it always picks git's base. The differential test must include the criss-cross shape. If gix's first base ever differs from git's, compare `merge_bases_many` against `git merge-base --all` as sets instead.

## Trust / foreign owner (executed)

| Condition | git 2.56 | gix 0.90 (discover, no bail) |
|---|---|---|
| repo owned by subuid 524288, no safe.directory | `fatal: detected dubious ownership in repository at '<path>'` + `git config --global --add safe.directory <path>`, exit 128 | `trust=Reduced` (with bail: `The git directory at "<path>" is considered unsafe as it's not owned by the current user.`) |
| `safe.directory = <path>` in `GIT_CONFIG_GLOBAL` file | opens | `trust=Full` |
| `safe.directory = *` | opens | `trust=Full` |
| `safe.directory = <parent>/*` | opens | `trust=Full` |
| `git -c safe.directory=<path>` / `GIT_CONFIG_PARAMETERS` | opens | **refused** |
| `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.directory …` | opens | **refused** |
| CI image, root, repo `chown 65534` | refuses (exit 128) | refused; with `GIT_CONFIG_GLOBAL` safe.directory → `trust=Full` |
| root + `SUDO_UID=65534` | opens | `trust=Full` (both honour SUDO_UID as root) |

Source: `Safe::directory_filter` accepts only `Kind::System | Kind::Global` (gix `config/tree/sections/safe.rs`). Under `trust=Reduced` without bail, 0.90.0 still returned the repo-local key (`probe.key=Some("repo-local")`). The earlier note that 0.89 silently drops repo config was not reproduced here. It does not matter, because wsx refuses before reading anything.

### Foreign-owner fixture mechanism (both executed)

1. Build the repo as the test user, then make it world-readable: `chmod -R go+rX <repo>`. The **host umask is 0077**; without this the foreign-owned repo is unreadable, and both tools fail for a different reason (`Permission denied`). That would prove nothing.
2. Detect whether the test runs as root without `libc`: take `metadata(<a file the test just created>).uid()`.
   - **euid 0 (GitHub CI container, `check-in-container.sh` under rootless Docker):** `chown -R 65534:65534 <repo>`, via `std::os::unix::fs::lchown` per entry or a hermetic `chown` spawn. Root deletes it on `TempDir` drop.
   - **euid ≠ 0 (host):** spawn `/usr/bin/unshare --map-root-user --map-users=auto --map-groups=auto chown -R 1:1 <repo>`. Inner uid map: `0→1000 (1)`, `1→524288 (65535)`. Restore before `TempDir` drops with the same command and `0:0`, from a `Drop` guard. Otherwise the host user cannot delete it.
3. Assert `metadata(repo).uid() != euid` after the chown. If the chown or unshare fails (no `newuidmap`, no `/etc/subuid` entry, seccomp), **panic with what is missing**; never skip.
4. Subject: the re-exec child (hermetic HOME/`GIT_CONFIG_GLOBAL`) calls wsx's discover and must get `ForeignOwner`. The git side is `git -C <repo> rev-parse` (exit 128, "dubious ownership"). Control: `safe.directory = <repo>` in the child's `GIT_CONFIG_GLOBAL` file. wsx and git then both open it.

The rootless-Docker container (root = host 1000; 65534 mapped) ran the chown and the refusal. GitHub's job container runs as root with Docker's default capabilities, which include `CHOWN`. That is [ASSUMED]: not run on a GitHub runner this session.

## Reftable (executed)

- `git init --ref-format=reftable` → `extensions.refStorage = reftable`, `core.repositoryformatversion = 1`, `.git/HEAD` = `ref: refs/heads/.invalid`.
- gix 0.90.0 **opens** it (`trust=Full`, `object_hash=Sha1`, `refStorage=Some("reftable")`) from the top level and from a subdirectory. `head_id()` then fails: `The reference at "HEAD" could not be decoded` → `Could not decode reference, input="ref: refs/heads/.invalid\n"` → `This reference uses an unsupported storage backend, such as reftable`. Control: the files-backend repo gives `refStorage=None` and a `head_id`.
- gix 0.90 does **not** validate `extensions.refStorage` itself (source: `repository/mod.rs` lists refStorage as a V1 extension and "Recognizing version 1 does not imply that `gix` supports every extension").
- **Shell oracle on reftable:** the patched `check-workspace-divergence.sh` (applied with `GIT_CEILING_DIRECTORIES`, `WSX_ORACLE_DIR` set) ran on a reftable devflow-like repo with an `origin/develop` remote-tracking ref. Clean: `check-workspace-divergence: every difference from origin/develop is declared.` exit 0. Known-positive (an undeclared `stray`): `undeclared workspace-only path: stray (declare: only stray)` exit 1. Both are identical to the files-backend run. So the oracle **works** on reftable. This ran in scratch, not through the harness; the harness run during execution is the recording.
- **Fixture step:** `git refs migrate --ref-format=reftable` on `WORK` converted a clone, with its remote-tracking refs, to reftable (rc 0). It **refuses with linked worktrees** (`error: migrating repositories with worktrees is not supported yet`, rc 255). The devflow preset's `work` has none.

### Registering the `regression:` scenario (tests/harness)
In-repo facts, read this session:
- `Class` [VERIFIED: tests/harness/scenario.rs:36-44]: `Parity,` / `Fixed { defect: &'static str },` / `Regression { reason: &'static str },`.
- Refusal rule [VERIFIED: scenario.rs:387-389]: `outcome.exit != 0 && outcome.refs == RefsExpect::Unchanged && outcome.new_commits.is_empty()`.
- Guard rule [VERIFIED: scenario.rs:364-365]: `matches!(action, Action::Check { .. } | Action::PrePush { .. })`. A check subject needs a `Role::KnownPositive` and a `Role::Clean` control.
- Existing registration form [VERIFIED: tests/harness/check.rs:296-302]: `self_test: oracle, rust = "Rust side not enabled until phase 5";`.
- Helpers [VERIFIED: check.rs:29-47]: `check_in_work(config, fixture)`, `verdict(exit)` → `Outcome { exit, refs: RefsExpect::Unchanged, new_commits: Vec::new(), files: FilesExpect::Unchanged }`.

Recommended shape, in `tests/harness/check.rs`:
- Subject: `check_in_work(RepoConfig::devflow(), to_reftable)`, where `to_reftable` runs `git refs migrate --ref-format=reftable` in `WORK`. `.expect(verdict(<non-zero>))`, `.class(Class::Regression { reason: "gix cannot read reftable refs; wsx refuses" })`, `.oracle(OracleSide::Recorded(verdict(0)))`. The record must be taken from the harness run; scratch predicts exit 0.
- `Role::Clean`: the same steps without migration, `verdict(0)`. This is the files-backend control.
- `Role::KnownPositive`: needs one, because Check is a guard. Example: files backend plus an undeclared note, `verdict(1)` with `oracle_stderr`. Reusing `self_test`'s known-positive is fine.
- Register `reftable_refused: oracle, rust = "Rust side not enabled until phase 5";`.
- Phase 5 must add `rust_stderr` naming reftable. The rule in `validate_entry` enforces this once the Rust side is enabled. Record it in STATE.md Blockers/Concerns.
- Open point for the executor: confirm `Snapshot`/`observe` in `outcome.rs` reads refs through `git` (works on reftable). It was not read this session.

## Signing + hooks for criterion 1 (executed)

Hermetic env: `HOME`, `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_NOSYSTEM=1`, fixed PATH. Then `ssh-keygen -q -t ed25519 -N "" -C t -f <root>/key` and an `allowed_signers` line `<email> namespaces="git" <type> <base64>`. Local config: `gpg.format=ssh`, `user.signingkey=<root>/key`, `commit.gpgsign=true`, `gpg.ssh.allowedSignersFile=<file>`. Hooks `pre-commit`, `commit-msg` and `pre-merge-commit` append to a log.

| Run | `git log --format=%G?` | hooks fired |
|---|---|---|
| commits + `git merge --no-ff --no-edit side` | `G` ×4 (incl. merge) | pre-commit 3, commit-msg 4, pre-merge-commit 1 |
| control: `git -c commit.gpgsign=false commit --no-verify` | `N` | 0 |

A merge runs `pre-merge-commit` and `commit-msg`, not `pre-commit`. The test must go through wsx's spawn helper. The control (helper given `--no-verify`/`-c commit.gpgsign=false` explicitly) must yield `N` and no hook lines. Pattern source: `tests/harness/fixture.rs:302-360` (`enable_signing`), reproduced rather than imported.

## Common Pitfalls

### Pitfall 1: `expect(dead_code)` unfulfilled on the test target
**What goes wrong:** clippy `-D warnings` fails with `this lint expectation is unfulfilled`.
**How to avoid:** `#![cfg_attr(not(test), expect(dead_code, reason = "…"))]` (executed: rc 0).
**Warning signs:** `error: this lint expectation is unfulfilled --> src/git/…:N`.

### Pitfall 2: A clippy.toml ban that silently stops resolving
**What goes wrong:** a path that does not resolve (for example after a gix rename) only prints `warning: \`gix::does_not_exist\` does not refer to a reachable function`, and `cargo clippy -- -D warnings` still exits 0 [VERIFIED: rc=0].
**How to avoid:** add a `#[cfg(test)]` function that calls **each** banned gix entry point under `#[expect(clippy::disallowed_methods, reason = "pins the ban list")]`. A renamed function then fails to compile, and a mis-spelled path makes the expect unfulfilled. Verified: the sanctioned call site's `expect` was fulfilled, and an unexpected `gix::open` in a test errored `use of a disallowed method \`gix::open\``.
**Ban list (gix 0.90.0 entry points, from `lib.rs`, `open/repository.rs`, `discover.rs`):** `gix::open`, `gix::open_opts`, `gix::discover`, `gix::discover_opts`, `gix::discover_with_environment_overrides`, `gix::open_with_environment_overrides`, `gix::ThreadSafeRepository::{open, open_opts, discover, discover_opts, open_with_environment_overrides, discover_with_environment_overrides, discover_with_environment_overrides_opts}`. The last is the one sanctioned site. All 13 resolved: clippy emitted no unresolved-path warning for them. Existing list form [VERIFIED: clippy.toml:19-22]: `{ path = "std::env::set_var", reason = "…" }`.

### Pitfall 3: GIT_CEILING_DIRECTORIES semantics differ between gix and git
Executed, start dir under fixture `r1`:

| start | ceiling | git | gix (default) | gix `match_ceiling_dir_or_error=false` |
|---|---|---|---|---|
| r1/inner | r1 | not found | **finds r1/.git** | **finds r1/.git** |
| r1/inner | r1/inner | finds r1/.git | error "None of the passed ceiling directories prefixed…" | finds r1/.git |
| r1/inner/deeper | r1 | not found | **finds r1/.git** | **finds r1/.git** |
| r1/inner/deeper | r1/inner | not found | not found | not found |

gix searches the ceiling directory itself; git does not. **How to avoid:** set `match_ceiling_dir_or_error = false` (fixes row 2). In tests, set the ceiling to a directory that is not itself a repo (the harness already uses the scenario root's parent). Row 1/3 remains a divergence; see Open Questions.

### Pitfall 4: `--local-env-vars` removes `GIT_CONFIG_PARAMETERS` and `GIT_CONFIG_COUNT`
Children aimed at a sibling lose every `git -c k=v` and `GIT_CONFIG_COUNT` override, including a `-c safe.directory`. This is git's own semantics, so wsx follows it, but a test must not rely on `-c` for anything a sibling-targeted child needs. Use a `GIT_CONFIG_GLOBAL` file.

### Pitfall 5: `safe.directory` from the command line is ignored by gix
The harness's `-c safe.directory=<root>` (provenance.rs) works for git but not for gix. CI's `git config --global --add safe.directory "$GITHUB_WORKSPACE"` is Global, so gix honours it. wsx's tests create their own repos owned by the test user, so CI's foreign checkout does not affect them unless a test discovers upward into it (Pitfall 3, ceiling).

### Pitfall 6: Host umask 0077
Fixtures are created mode 700/600. Chmod `go+rX` before making a repo foreign-owned.

### Pitfall 7: Relative `GIT_INDEX_FILE` and unset `GIT_DIR` in the main worktree's hooks
Never assume `GIT_DIR` is set inside a hook, or that `GIT_INDEX_FILE` is absolute. Let gix resolve them.

### Pitfall 8: Canonicalisation
`common_dir()` is `<git-dir>/../..` [VERIFIED]. `/home` → `/var/home` on this host. Canonicalise worktree, git dir and common dir once at resolution. Compare `canonical(target)` with `own_worktree` to decide removal.

### Pitfall 9: Unit-test scratch location
There is no `CARGO_TARGET_TMPDIR` in a binary crate's unit tests. `TempDir::new()` would use `/tmp`, which is RAM-backed. **Use** `current_exe()` → parent must be named `deps` (assert it, fail otherwise) → `.parent().parent()` → `join("tmp")`, then `create_dir_all` and `TempDir::new_in`. That is the same `<target>/tmp` cargo uses for `CARGO_TARGET_TMPDIR` [VERIFIED: exe at `target/debug/deps/<crate>-<hash>` under nextest]. It stays inside the target dir under `--target <triple>` and `llvm-cov-target` layouts too [ASSUMED for those two layouts]. In the container `/workspace/target` is the writable volume. Fixtures there sit **inside the wsx checkout**, so every git child and every env-dependent gix child needs `GIT_CEILING_DIRECTORIES` = the scratch root's parent, which is not a repo.

### Pitfall 10: Parent asserting only the child's exit status
A mistyped child name exits 0 (`running 0 tests`) [VERIFIED]. Require a marker line.

### Pitfall 11: SUDO_UID
gix honours `SUDO_UID` for any euid (gix-sec `identity.rs:53-57`); git only as root. The hermetic `env_clear()` removes it in tests. In production it is a narrow divergence (a non-root process with `SUDO_UID` set trusts that uid's repos).

## Code Examples

### Version gate (parse leniently)
```rust
// `git --version` → "git version 2.56.0" (host and CI image). Other builds may append
// ".windows.1" or " (Apple Git-…)": take the token after "git version ", split on '.',
// parse the first two as u32, compare with (2, 54). Report found vs required.
```

### Re-exec child skeleton
```rust
#[test]
fn hook_resolves_linked_worktree() {
    // build fixture (main + wt1 + wt2) with hermetic env; install pre-commit that runs:
    //   exec "<current_exe>" --exact git::tests::hook_child --include-ignored --nocapture
    // run `git commit` in wt1 through the hermetic env; parse the child's MARKER lines; assert.
}
#[test]
#[ignore = "child half of hook_resolves_linked_worktree; runs only inside the fixture's hook"]
fn hook_child() { /* Repo::discover(cwd); print MARKER worktree=… common=… index=…; spawn sibling child */ }
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| gix 0.89.0 (research baseline) | gix 0.90.0 | 2026-10-22 | All needed APIs verified on 0.90.0; 0.89 names no longer relevant |
| "gix cannot open reftable" | gix 0.90 opens reftable, fails at ref decode | observed 0.90.0 | Refusal must be the refStorage config check, not an open error |
| Reduced trust drops repo config (0.89 note) | 0.90.0 still read a repo-local key under Reduced trust | observed | Irrelevant: wsx refuses on trust first |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | GitHub's job container (root, default Docker caps) can `chown` to 65534 | Trust / mechanism | The foreign-owner test fails in CI (loudly, by design); fallback is the `unshare` path or `--cap-add` |
| A2 | `target/<triple>/…/deps` and `llvm-cov-target/…/deps` layouts keep `deps/../../tmp` inside the target dir | Pitfall 9 | Scratch lands elsewhere under target; the `deps` assertion catches unexpected layouts |
| A3 | Harness `outcome.rs` snapshots refs through `git` and therefore works on reftable | Reftable scenario | The oracle-side recording fails; the executor would see it on the first run |
| A4 | The combined wsx graph (gix + tempfile dev) passes cargo-deny with the four-license allow list | cargo-deny | `scripts/check.sh deps` fails; add the license it names |

## Open Questions

1. **The inclusive-ceiling divergence (Pitfall 3, rows 1 and 3).**
   - What we know: with `GIT_CEILING_DIRECTORIES=<dir>`, gix finds `<dir>/.git`; git does not.
   - Unclear: whether wsx must match git here. Only users who set ceilings are affected.
   - Recommendation (planner's call, within discretion): after discovery, when `GIT_DIR` is unset and a ceiling equals the found worktree (canonical) while the start dir differs, refuse as not-a-repo. Add a gix-vs-git test with a control. Otherwise document the divergence.
2. **`-c safe.directory` / `GIT_CONFIG_COUNT` honoured by git but not gix (Pitfall 5).** wsx is stricter (fail-closed), which is consistent with GIT-04's intent. The refusal message should name the global-config fix (`git config --global --add safe.directory <path>`), exactly as git prints it. Recommend accepting this and recording it in the code docs.
3. **The reftable scenario's expected exit code.** IF-01 lands in phase 5. Phase 3 must still put a non-zero `exit` in `verdict(..)` to satisfy `is_refusal`. Use 1 as a placeholder and let phase 5 correct it.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| git | everything | ✓ host / ✓ image | 2.56.0 / 2.56.0 | — |
| rustc | build | ✓ | 1.99.1 (pinned) | — |
| cargo-nextest | tests | ✓ | 0.9.150 | — |
| cargo-deny | deps gate | ✓ | 0.20.2 | — |
| ssh-keygen | signing test | ✓ host / ✓ image | — | — |
| /usr/bin/unshare + newuidmap + /etc/subuid (`denniyahh:524288:65536`) | foreign-owner test, non-root | ✓ host | util-linux 2.41.5 | none: test must fail loudly |
| chown (root path) | foreign-owner test, root | ✓ image (rootless Docker run as root) | — | — |
| Docker (rootless) | check-in-container | ✓ | 29.9.2 | — |

Note: `unshare` on the user PATH resolves to linuxbrew's (`/home/linuxbrew/.linuxbrew/bin/unshare`). Use the hermetic PATH `/usr/local/bin:/usr/bin:/bin` [VERIFIED: tests/harness/env.rs:32] or the absolute `/usr/bin/unshare`.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | libtest unit tests in the binary crate, run by cargo-nextest 0.9.150 |
| Config file | `.config/nextest.toml` (JUnit output); `--no-tests=fail` fixed by `scripts/check.sh` |
| Quick run command | `scripts/check.sh test -E 'test(=git::<module>::<name>)'` |
| Full suite command | `scripts/check.sh all` (+ `scripts/check.sh deps`, + `scripts/check-in-container.sh`) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| GIT-01 | commit + merge via helper run hooks and are signed (`G`); control with explicit `--no-verify`/`gpgsign=false` → `N`, no hooks | unit (spawns git) | `scripts/check.sh test -E 'test(=git::spawn::tests::commit_and_merge_run_hooks_and_sign)'` | ❌ Wave 0 |
| GIT-01 | merge-base / ahead-behind / tree diff / range walk equal git on criss-cross + skew fixture; each with an answer-changing control | unit, differential, in-process gix | one test per query, e.g. `…test(=git::repo::tests::merge_base_matches_git)` | ❌ |
| GIT-02 | inside wt1 pre-commit, wsx resolves wt1 / common dir / hook index | unit, re-exec child via hook | `…test(=git::repo::tests::hook_resolves_linked_worktree)` | ❌ |
| GIT-02 | sibling-targeted child sees sibling git dir; control with removal disabled sees wt1's (leak detected) | unit, re-exec child via hook | `…test(=git::spawn::tests::sibling_child_env_scrubbed)` | ❌ |
| GIT-02 | own-worktree child keeps hook `GIT_INDEX_FILE` | unit, re-exec via hook | `…test(=git::spawn::tests::own_worktree_child_keeps_hook_index)` | ❌ |
| GIT-02 | null stdin: `hash-object --stdin` = empty blob; control inherited stdin = marker blob | unit, re-exec child with piped stdin | `…test(=git::spawn::tests::children_get_null_stdin)` | ❌ |
| GIT-02 | git < 2.54 refused naming found/required | unit (fake `git` script on child PATH via helper env snapshot) | `…test(=git::spawn::tests::old_git_refused)` | ❌ |
| GIT-03 | reftable refused naming reftable; files-backend same history opens | unit | `…test(=git::repo::tests::reftable_refused)` | ❌ |
| GIT-03 | harness `regression:` scenario registered, oracle recorded, Rust disabled | harness | `scripts/check.sh test -E 'test(=check::reftable_refused::oracle)'` (exact generated name per `scenarios!`) | ❌ |
| GIT-04 | really-foreign repo refused by wsx and git; `safe.directory` control opens both | unit, re-exec child | `…test(=git::repo::tests::foreign_owner_refused)` | ❌ |
| — | sha256 repo opens | unit | `…test(=git::repo::tests::sha256_repo_opens)` | ❌ |
| — | ban list pinned (each banned fn called under `expect`) | compile-time (clippy) | `scripts/check.sh clippy` | ❌ |

### Sampling Rate
- **Per task commit:** the task's named tests via `scripts/check.sh test -E 'test(=…)'`, one per command, chained with `&&`.
- **Per wave merge:** `scripts/check.sh all`.
- **Phase gate:** `scripts/check.sh all`, `scripts/check.sh deps` and `scripts/check-in-container.sh all` (which exercises the root foreign-owner path), all green before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `src/git/` test support: scratch-dir resolver (Pitfall 9), hermetic env builder (HOME, `GIT_CONFIG_GLOBAL`, `GIT_CONFIG_NOSYSTEM=1`, `GIT_CEILING_DIRECTORIES`, PATH, fixed dates), fixture builder, re-exec helper with marker parsing, foreign-owner guard with restore-on-drop.
- [ ] `Cargo.toml` gix line + `deny.toml` licenses (`Apache-2.0`, `Unicode-3.0`, `Zlib`) + `clippy.toml` ban entries.
- [ ] `tests/harness/check.rs` reftable scenario + fixture step.

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | credentials stay with git (GIT-01) |
| V3 Session Management | no | — |
| V4 Access Control | yes | repo ownership trust via gix + `safe.directory` (GIT-04) |
| V5 Input Validation | yes | refStorage check before refs; paths as raw bytes; argv passed as `OsStr` args (no shell) |
| V6 Cryptography | no | signing delegated to git/ssh-keygen |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Malicious repo owned by another user injects config (`core.fsmonitor`, hooks) | Elevation of privilege | Refuse when `git_dir_trust() != Full` before any use |
| Hook env redirects a sibling operation to the wrong worktree/index (N4) | Tampering | `--local-env-vars` removal for sibling targets; siblings only via `main_repo()`/`worktrees()` |
| Child git blocks or reads attacker stdin | DoS / Tampering | `Stdio::null()` for every child |
| Shell injection via branch/path names | Tampering | `Command::args`, never `sh -c` |
| Silently disabled refusal (gix "unsupported" error swallowed) | Repudiation | Map the backstop error to `Reftable`; never fail open |

## Sources

### Primary (HIGH confidence)
- gix 0.90.0 crate tarball (static.crates.io), read this session: `src/discover.rs`, `src/open/{repository,options,permissions}.rs`, `src/repository/{revision,diff,location,worktree,mod}.rs`, `src/revision/walk.rs`, `src/diff.rs`, `src/config/tree/sections/safe.rs`, `src/lib.rs`, `Cargo.toml` features.
- Registry sources: gix-diff 0.70.0 (`tree/function.rs`, `recorder.rs`, features), gix-sec 0.16.1 (`identity.rs`), gix-discover 0.57.1 (`upwards/types.rs`), gix-config 0.62.2 (`source.rs`, `file/init/from_env.rs`), gix-ref 0.69.1 (`decode.rs`), gix-traverse 0.64.0.
- crates.io API `crates/gix/versions` (0.90.0, 2026-10-22, rust_version 1.88).
- Executed probes: gix 0.90.0 + git 2.56.0 on host; pinned image `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`; cargo-deny 0.20.2; clippy 1.99.1; nextest 0.9.150.
- Repo files read: `03-CONTEXT.md`, `STATE.md`, `REQUIREMENTS.md`, `CLAUDE.md`, `Cargo.toml`, `clippy.toml`, `deny.toml`, `rust-toolchain.toml`, `scripts/check.sh`, `scripts/check-in-container.sh`, `.github/workflows/ci.yml`, `tests/harness/{scenario,check,env,signing,fixture,registry,oracle}.rs`, `tests/oracle/*`, `.planning/research/{STACK,PITFALLS,SUMMARY}.md`.

### Secondary / Tertiary
- None relied on. Context7 was not used: prior research recorded it as stale for gix, and the released source was read instead.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH. Version from the registry; features compiled; cargo-deny run with a control.
- Architecture: HIGH. Every API in the patterns was compiled and run on 0.90.0.
- Pitfalls: HIGH. Each was reproduced, except A1 and A2 (assumed).

**Probe hygiene:** all probe crates, fixtures and build output were created under the session scratchpad. They were deleted before writing this file (probe, probe2, probe3, src-gix, fx, home, oracle copy, target, target-rel). The foreign-owned fixture was chowned back first, and no non-owned file remained. No containers were left running. The shared `~/.cargo/registry` cache gained the gix 0.90.0 crates.

**Research date:** 2026-10-26
**Valid until:** 2026-11-09. gix ships breaking releases roughly monthly; the exact pin makes this stable until a deliberate upgrade.
