# Phase 3: Git adapter - Context

**Gathered:** 2026-10-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 3 delivers wsx's git layer (GIT-01..04) inside the binary crate:
- **Repo resolution.** wsx finds its own worktree, git dir and common dir from any worktree, any
  subdirectory, or inside a hook that exports `GIT_DIR`/`GIT_INDEX_FILE`.
- **Reads through gix.** Discovery plus the four read queries success criterion 1 names:
  merge-base, ahead/behind, tree diff, range walk.
- **Writes through the `git` binary.** One spawn helper, so the user's hooks, signing, merge
  drivers and credentials apply.
- **Refusals.** Reftable repos, repos owned by another user, and git older than 2.54.

Phase 3 builds no wsx command. The first command is phase 5's `wsx check`, which also sets the
exit-code contract (IF-01). Config and blob reads for the repo model are phase 4's.

</domain>

<decisions>
## Implementation Decisions

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

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `tests/harness/signing.rs`: per-run throwaway SSH signing key, useful as a pattern for
  criterion 1's signing proof (it lives in the harness, so unit tests reproduce the pattern rather
  than import it).
- `tests/harness/env.rs`: hermetic `env_clear()` child environment (per-scenario HOME,
  `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM`, fixed dates, explicit PATH). Unit tests need the same
  hermeticity.
- `tests/harness/hermeticity.rs:430`: an existing use of `unset $(git rev-parse --local-env-vars)`.
- `tests/harness/provenance.rs:559-593`: `-c safe.directory=<root>` and
  `GIT_TEST_ASSUME_DIFFERENT_OWNER=1`, already used to model CI's foreign-owned checkout.
- `tests/harness/scenario.rs`: the `regression:` class, `rust_stderr`, and the rejection rules a
  `regression:` case must satisfy (non-zero exit, refs unchanged, no new commit).
- `.planning/research/STACK.md` gix capability table: executed API names and traps for 0.89.0
  (env-override discover, `common_dir` not canonical, detached HEAD gives `None`, `worktrees()`
  excludes the main worktree and is unsorted, `PreviousValue::MustNotExist` trap).

### Established Patterns
- Rust 2024, one binary crate, no library crate. `src/main.rs` only prints `--version`.
- `clippy.toml` bans `std::env::set_var`/`remove_var`, with reasoned `expect` exceptions. The
  gix-open ban joins the same list.
- `#[expect(..., reason = "...")]`, never `allow`, so an exception cannot outlive its reason.
- Test temp dirs under `CARGO_TARGET_TMPDIR`, never `/tmp` (RAM-backed on this host). Unit tests
  have no `CARGO_TARGET_TMPDIR`; research must pick an equivalent non-tmpfs location.
- Every check has a negative control; nextest runs with `--no-tests=fail`.
- Commit subjects carry the phase scope `type(03-NN): ...`.

### Integration Points
- `Cargo.toml` `[dependencies]` gets gix (the first runtime dependency). `deny.toml` must accept
  gix's license and transitive graph.
- `clippy.toml` gets the `gix::open`/`gix::discover` ban.
- `tests/harness/registry.rs` gets the reftable `regression:` scenario (Rust side disabled).
- CI's container checks out the repo under another owner: tests that open this checkout must
  trust it explicitly, as phase 2 learned.

</code_context>

<specifics>
## Specific Ideas

- Inside a hook, wsx must behave as git does for its own worktree and never leak the hook's
  `GIT_DIR`/`GIT_INDEX_FILE` into a sibling worktree. The oracle's N4 defect (deleting siblings'
  exclude files) is the failure this phase exists to prevent.
- A refusal must not be silent: a reftable or foreign-owned repo must never open with dropped
  config. gix silently drops config in a repo with reduced trust; git refuses.

</specifics>

<deferred>
## Deferred Ideas

- Config reads (`wsx.sourceBranch`, `wsx.baseRef`) and reading `<tip>:.wsx.conf` from the object
  database: phase 4.
- Exit codes and `--json` for refusals: phase 5 (IF-01).
- Enabling the reftable `regression:` scenario's Rust side: phase 5, with `wsx check`.
- Reftable support itself: before the public release (PROJECT.md key decision).

</deferred>
