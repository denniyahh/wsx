# Phase 2: Oracle harness - Context

**Gathered:** 2026-10-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Phase 2 delivers the black-box scenario harness (HARN-01, -02, -03, -05, -06):
- **The vendored oracle.** devflow's `sync-workspace.sh`, `phase-worktree.sh`, `cut-pr-branch.sh`,
  `check-workspace-divergence.sh` and `scripts/hooks/pre-push`, vendored under `tests/oracle/`.
- **A hermetic runner under `cargo nextest`.** Each scenario runs against the oracle, the Rust
  binary or both, and states its expected outcomes explicitly.
- **The HARN-06 case set.** Every case is recorded against the oracle.

Phase 2 builds no wsx command. Each scenario's Rust side stays disabled until the phase that
implements it (5-10) turns it on.

</domain>

<decisions>
## Implementation Decisions

### How scenarios are written
- **Scenarios are Rust test code in `tests/`.** Each is built with a small builder and states its
  expected outcomes in a typed struct. There are no data files and no parser dependency.
  - **Why:**
    - Several HARN-06 cases need non-UTF-8 and newline path names, which raw bytes in code can
      hold and TOML cannot.
    - A fixture is a sequence of steps (merges, a conflict resolution, fork remotes).
    - A misspelled expectation fails to compile instead of silently checking nothing.
  - On failure, the runner prints the scenario's expected outcome in plain words, so a
    non-programmer can read it.
- **A scenario names an abstract action** (sync, phase worktree, cut, divergence check, pre-push
  guard), not a command line. Each implementation has an adapter:
  - the oracle runs `bash` on the patched script;
  - Rust runs `wsx <command>`.

  A scenario therefore stays valid after the oracle is deleted, and after wsx's CLI deliberately
  differs from the scripts (for example, `wsx cut` never touches the user's checkout).
- **Every scenario declares its repo config as an explicit field.** The config covers:
  - the `.wsx.conf` manifest contents (`only` and `modified` paths);
  - the integration branch;
  - the base and push remotes;
  - the repo-local git config the scenario needs, such as signing.

  This keeps wsx config-driven (owner, 2026-10-26: "not hardcoded to treat every project the
  same").
  - **The oracle runs a scenario only where it implements that behaviour** (success criterion 5).
    The scripts hard-code devflow's personal paths, so a custom personal path is Rust-only. The
    runner labels Rust-only scenarios by name.
  - **Corrected 2026-10-26 after research.** This bullet first said every non-devflow config was
    Rust-only, which contradicted criterion 5. Research found the oracle partly implements two
    such cases, so each gets an oracle-recorded variant where the oracle can run it:
    - **Fork layout.** sync, cut and the divergence check take a remote argument.
    - **A repo with no GSD files.** `phase-worktree.sh` refuses there, a recorded refusal that
      doubles as IF-02's negative control.

    A part the oracle lacks stays Rust-only.
  - Phase 2 only lets a scenario *state* a config; wsx's config reader is phase 4 (CONF-01..05,
    -07).
- **In phase 2, every scenario's Rust side is disabled.** The runner reports each disabled
  scenario by name (success criterion 5).

### The vendored oracle
- **Source is devflow's originals, not wsx's `scripts/workspace/` ports.** The pin is devflow
  `origin/develop` at vendoring time, currently `ee6ec792d03f1495fc8c4d90d5894ff08b968d80`. The SHA
  is recorded in `tests/oracle/README.md`, with MIT attribution (devflow `LICENSE`, © Dennis Kim).
  - **Why:** checked 2026-10-26, wsx's `cut-pr-branch.sh` differs from devflow's by 80 diff lines
    and `phase-worktree.sh` by 21; `sync-workspace.sh` is identical. wsx's copy carries phase-1
    fixes that devflow lacks: the non-ASCII path leak (`core.quotePath`, quoted-name refusal) and
    the refusal to overwrite a branch without `--force`.
  - Those defects are listed `fixed:` cases, so the oracle must still exhibit them. A `fixed:`
    scenario's negative control is the oracle failing it.
- **The vendored files stay byte-identical to devflow's.** A single documented `oracle.patch`
  adapts every hard-coded path. It is applied at test time, with `git apply`, to a scratch copy.
  - A test checks that each vendored file's blob matches the recorded devflow commit.
  - A test checks that the patch applies cleanly.
- **The pre-push hook is vendored whole.** It is driven the way git drives it: arguments are
  remote name and URL, and stdin carries `<local ref> <local sha> <remote ref> <remote sha>`
  lines. The personal-path policy is never extracted into a separate function.
- **Each script has a self-test scenario** that proves it runs against a fixture (HARN-01).

### Hermetic environment and signing
- **Environment.** Every spawned process starts from `env_clear()`. It gets:
  - a per-scenario HOME and XDG dirs;
  - `GIT_CONFIG_GLOBAL` and `GIT_CONFIG_SYSTEM` pointed at harness-owned files;
  - fixed `GIT_AUTHOR_DATE` and `GIT_COMMITTER_DATE`;
  - an explicit `PATH`;
  - only `LLVM_*` passed through (HARN-02).

  This follows the `hermetic()` pattern in `tests/setup_repo.rs`. It must also scrub the
  repository-local git variables (`GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, ...), but
  `env_clear()` already does that.
- **The hermeticity proof (success criterion 2).**
  - A global hook and global git config are planted where the host's would be. A scenario's
    commits and outcomes are unchanged, and the hook never fires.
  - A control scenario wired to the planted config does see the hook fire.
- **Signing is off by default.** Local config sets `commit.gpgsign=false`.
  - The scenarios where wsx or the oracle creates commits (cut replay, the sync merge) sign with a
    throwaway SSH key generated per run, and assert signature presence separately (HARN-03).
  - The CI image has `/usr/bin/ssh-keygen` and git 2.56.0 (checked 2026-10-26).
- **Temp directories come from `tempfile`, added as a dev-dependency only,** so scratch trees are
  removed even when a test panics.
  - They are created with `TempDir::new_in(env!("CARGO_TARGET_TMPDIR"))`, never the default
    `/tmp`. On this host `/tmp` is RAM-backed tmpfs, and leftover scratch there helped get a full
    test run OOM-killed on 2026-10-10.
  - The dependency needs a written justification and a clean `scripts/check.sh deps`.

### Coverage tooling
- **`cargo-llvm-cov` is pinned in `scripts/install-tools.sh`** at the host's version, and its
  absence is a hard failure, as for the other tools.
- **`llvm-tools` is added to `components` in `rust-toolchain.toml`.**
  - **Why:** research, confirmed 2026-10-26 with `rustup component list --installed`, found that
    the pinned 1.99.1 lacks the component, so `cargo llvm-cov` fails.
  - A one-time `rustup component add` on the host would not reach the container. The toolchain
    file is read by every environment.
  - Runs set `CARGO_LLVM_COV_SETUP=no`, so a missing component fails loudly instead of being
    installed silently.
- **A one-time proof shows coverage sees code that only the spawned binary executes** (success
  criterion 2). The proof records a negative control: the same line is uncovered when the binary
  is not spawned.
- **No CI coverage job in phase 2.** Revisit once real commands exist (phase 5 onward); a threshold
  set now would be meaningless.

### Claude's Discretion
- Module layout under `tests/` (one integration-test binary or several), the builder API's shape,
  and how outcomes are rendered.
- The fixed dates, and the identities used for author and committer.
- The exact wording of `oracle.patch`, provided it is the only change and is documented.
- How the difference classes (`fixed:`, `regression:`) and guard controls (known-positive, clean)
  are encoded, provided the runner enforces success criterion 4's rejection rules.

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `tests/setup_repo.rs`: `hermetic()` builds an `env_clear()` command with
  `GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`, a scratch HOME and a stub-bin PATH
  prefix. It makes its fixture repos under `CARGO_TARGET_TMPDIR`.
- `tests/gate.rs`: the house style for asserting on exit code, stderr and stdout with
  descriptive messages.
- `scripts/install-tools.sh`: the single place for tool pins (cargo-deny, gitleaks,
  cargo-nextest) and `verify-cargo`.
- `tests/fixtures/env-ban`: a scratch-crate fixture pattern for controls.

### Established Patterns
- Rust 2024, one binary crate, no library crate. Integration tests spawn processes and never
  link the implementation.
- `clippy.toml` bans `std::env::set_var` and `remove_var`, so environment is scoped per child
  with `Command::env`.
- **Gating:**
  - nextest runs with `--no-tests=fail`;
  - a single test is gated with `-E 'test(=module::name)'`, one test per command;
  - every check comes with a negative control.
- Commit subjects carry the phase scope `type(02-NN): ...`, because the PR cut replays only
  phase-scoped commits.

### Integration Points
- `tests/oracle/` is new and must reach `develop`. It must not go under `scripts/workspace/`,
  which is stripped from PRs.
- `Cargo.toml` gets `[dev-dependencies] tempfile`. `deny.toml` must accept its license and its
  transitive dependencies.
- `scripts/check.sh test` already runs the whole nextest suite, so harness scenarios join the
  gate automatically.

</code_context>

<specifics>
## Specific Ideas

- **wsx is config-driven** (owner, 2026-10-26). The harness must make it cheap to run the same
  action under different repo configs. That is how per-project behaviour gets tested, and
  scenario code must never assume devflow's layout except in a scenario that declares it.
- **The owner is not a git expert.** A failing scenario should say in plain words what was
  expected and what happened.

</specifics>

<deferred>
## Deferred Ideas

- A CI coverage job with a threshold (revisit at phase 5 or later).
- wsx's config reader and `.wsx.conf` parsing (phase 4, CONF-01..05, -07).

</deferred>
