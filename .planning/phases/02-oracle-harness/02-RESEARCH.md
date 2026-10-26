# Phase 2: Oracle harness - Research

**Researched:** 2026-10-26
**Domain:** Black-box scenario harness in Rust integration tests (cargo-nextest), vendored bash oracle, git hermeticity, SSH signing, cargo-llvm-cov
**Confidence:** HIGH for oracle behaviour, hermeticity, signing, coverage and tempfile (each executed this session against git 2.56.0, nextest 0.9.148, cargo-llvm-cov 0.9.1 and cargo-deny 0.20.2). MEDIUM for the registry and runner design, which is a recommendation built from verified nextest behaviour.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### How scenarios are written
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
  - **The oracle runs only scenarios whose config is devflow-shaped.** The scripts hard-code
    devflow's paths and `origin`.
  - **Any other config is Rust-only:** a custom personal path, a repo with no agent files
    (IF-02), or a fork layout without `origin/<integration>`. The runner labels these scenarios
    Rust-only by name.
  - Phase 2 only lets a scenario *state* a config; wsx's config reader is phase 4 (CONF-01..05,
    -07).
- **In phase 2, every scenario's Rust side is disabled.** The runner reports each disabled
  scenario by name (success criterion 5).

#### The vendored oracle
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

#### Hermetic environment and signing
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

#### Coverage tooling
- **`cargo-llvm-cov` is pinned in `scripts/install-tools.sh`** at the host's version, and its
  absence is a hard failure, as for the other tools.
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

### Deferred Ideas (OUT OF SCOPE)
- A CI coverage job with a threshold (revisit at phase 5 or later).
- wsx's config reader and `.wsx.conf` parsing (phase 4, CONF-01..05, -07).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| HARN-01 | One scenario suite runs against the oracle and the Rust binary, selected per scenario. The devflow oracle is vendored under `tests/oracle/` at a recorded commit, MIT-attributed, adapted by one documented patch, and each script has a self-test scenario | Oracle inventory (per-script CLI, hard-coded items); a candidate `oracle.patch` with 4 substitutions, built and applied this session; all 5 patched scripts self-tested against a fixture; blob SHAs recorded; `git apply` subdirectory trap found |
| HARN-02 | Hermetic: HOME, XDG, global and system config, and hooks isolated; signing pinned locally; host global hooks never fire; `LLVM_*` passed through | Env table; the oracle pre-push `unset GIT_CONFIG_GLOBAL GIT_CONFIG_SYSTEM` trap (reads `/etc/gitconfig` unless `GIT_CONFIG_NOSYSTEM=1`); `GIT_CEILING_DIRECTORIES`; `GIT_ALLOW_PROTOCOL=file`; planted-hook proof design and its control, executed; coverage under `env_clear()` measured both ways |
| HARN-03 | Explicit expected outcomes: exit code, ref names, commits (full message, author, author date, committer name/email, tree), signature presence, files on disk; oracle outcome recorded; `fixed:` and `regression:` classes | Outcome model; raw `git cat-file commit` parsing; signature presence via the `gpgsig` header (verified); deterministic dates (verified); a per-field mutation meta-test design |
| HARN-05 | Every guard scenario has a known-positive control that must still be refused and a clean control that must pass | Inline guard-control encoding; the oracle's known-positive and clean controls both reproduce (pre-push rc=1 and rc=0) |
| HARN-06 | Merge commits, squash-merged PRs, fork layout, scoped plus unscoped commits in one cut, a no-GSD repo, non-ASCII, newline and non-UTF-8 names, an existing PR branch with extra commits | A case-by-case map of what the oracle implements, with 9 oracle defects reproduced this session to serve as real negative controls |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- One package, binary crate `wsx`, edition 2024, **no library crate**. Integration tests spawn processes and never link the implementation.
- `scripts/check.sh all` is the gate (fmt, clippy, test). `scripts/check.sh deps` (cargo-deny) must be clean after adding `tempfile`.
- nextest always runs `--no-tests=fail`. Gate one test per command: `-E 'test(=module::name)'`, chained with `&&`. Never use `test(=a) | test(=b)`.
- `rg -c` prints nothing on zero matches; check absence with a Rust test or `rg -q`, not with a shell count.
- Capture the exit code of the command that matters, not the pipeline tail, and run gates under bash.
- Prefer a named Rust test to a shell pipeline.
- `std::env::set_var` and `remove_var` are banned by `clippy.toml`. Scope environment with `Command::env`, `env_clear` or `env_remove`.
- Never run `cargo init` or `cargo new` in the repo. Add new `.gitignore` lines at the **top**.
- Scripts that must reach develop never go under `scripts/workspace/`. `tests/oracle/` is correct.
- Commit subjects carry the phase scope `type(02-NN): ...`.
- The container mounts the checkout read-only and builds into a volume over `target/`. Anything that writes must use a scratch dir; `CARGO_TARGET_TMPDIR` lives in that volume.
- Add no dependency without a justification and a clean `scripts/check.sh deps`. The tool never prompts. Never push or change repo settings from an agent session.
- Tool versions live only in `scripts/install-tools.sh`.

## Summary

The five devflow scripts at `ee6ec792` run in a hermetic fixture after a 4-substitution patch, which was built and applied this session. Three of the substitutions redirect relative `scripts/...` paths to the vendored copy. The fourth removes the pre-push hook's CI-mirror tail, which would otherwise run `scripts/check-in-container.sh` inside the fixture. `cut-pr-branch.sh` and `sync-workspace.sh` need no patch. Without the patch, two scripts give a false green: `phase-worktree.sh` silently skips its sync step (exit 0), and `check-workspace-divergence.sh` fails because it cannot find `scripts/cut-pr-branch.sh`.

Nine oracle defects were reproduced in hermetic fixtures, so the `fixed:` scenarios have real negative controls:
- the quoted-name leak in both the cut and the guard (non-ASCII, newline and non-UTF-8 names);
- the pattern-limited push guard;
- a new branch with no commits of its own passing the guard;
- a merge-resolution personal file passing the guard;
- the `.planningX` name-prefix over-refusal;
- the unconditional `git branch -f` losing a reviewer's commit;
- a partial PR branch left after a conflict;
- an unscoped commit silently dropped from a scoped cut.

The guard's known-positive control (rc=1) and clean control (rc=0) both behave correctly on the oracle, so a guard bypass recorded against the oracle is a real bypass, not a broken hook.

Three hermeticity traps were found by experiment. The CONTEXT env list does not cover them:
- **(1)** The oracle's `pre-push` runs `unset GIT_CONFIG_GLOBAL GIT_CONFIG_SYSTEM`. Git then reads the host's `/etc/gitconfig`, plus `$XDG_CONFIG_HOME/git/config` and `$HOME/.gitconfig`. `GIT_CONFIG_NOSYSTEM=1` survives the unset and blocks the host file, and the harness's global config must live **at** `$HOME/.gitconfig`.
- **(2)** `git apply` run inside a subdirectory of any repo prints `Skipped patch` and exits **0** without applying anything. `CARGO_TARGET_TMPDIR` sits inside the wsx checkout, both on the host and in the container. Set `GIT_CEILING_DIRECTORIES`.
- **(3)** Coverage: a binary spawned with `env_clear()` and no `LLVM_*` passthrough records **0** coverage and drops a `default_*.profraw` into its cwd, which is the fixture. The same spawn with `LLVM_*` passed through is covered.

The pinned toolchain `1.99.1` has **no llvm-tools component**, so `cargo llvm-cov` on this repo fails until one is installed. This is an owner decision.

**Primary recommendation:** Build one integration-test binary, `tests/harness/`. Scenarios are plain functions that return a typed `Scenario`. A `scenarios!` macro generates `<scenario>::oracle` and `<scenario>::rust` tests per scenario, with disabled Rust sides marked `#[ignore]` so nextest counts them as skipped. A report test lists the disabled names, made visible through a nextest per-test `success-output = "final"` override (verified). Each generated test validates its own scenario before running it. Start with a tracer: one sync scenario through the oracle adapter, with explicit expectations.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Scenario definition (config, fixture steps, action, expected outcome, oracle record, class) | Harness: scenario modules | — | Data owned by the test, never derived from a run (anti-pattern: differential-only) |
| Scenario validation (class rules, guard controls, oracle side vs. macro flag) | Harness: runner | Registry meta-test | Enforced inside every generated test, so an unregistered module cannot escape |
| Hermetic process environment | Harness: env builder | — | Single function; every spawn (git, bash, ssh-keygen, wsx) goes through it |
| Fixture construction (repos, remotes, commits with deterministic clock, worktrees) | Harness: fixture builder (spawns `git`) | — | Byte paths via `OsStrExt`; never links wsx |
| Action execution | Adapters: oracle (`bash <patched script>`) / Rust (`wsx <cmd>`) | — | Scenario names an abstract action; adapter maps it or reports "not implemented" |
| Outcome observation (exit, refs, commits, signature, files) | Harness: observer (spawns `git cat-file`, `for-each-ref`, walks fs) | — | Raw bytes, `-z` everywhere |
| Oracle provenance (blob match, patch applies) | Harness: provenance tests | `tests/oracle/README.md` | Recorded SHAs are the source of truth |
| Disabled-Rust-side reporting | nextest (`#[ignore]` → "skipped" count) + report test | `.config/nextest.toml` override | nextest prints skipped names only at a noisy status level (verified) |
| Coverage of the spawned binary | cargo-llvm-cov (`LLVM_PROFILE_FILE`) | env builder passthrough | Verified mechanism |

## Standard Stack

### Core
| Library / tool | Version | Purpose | Why standard |
|---------|---------|---------|--------------|
| `tempfile` (dev-dependency only) | 3.27.0 | `TempDir::new_in(CARGO_TARGET_TMPDIR)`, removed on drop, including on panic unwind | Locked by CONTEXT. [VERIFIED: `cargo info tempfile` → `version: 3.27.0`, `license: MIT OR Apache-2.0`, `rust-version: 1.63`] |
| cargo-nextest | 0.9.148 (existing pin) | Runs the suite; `#[ignore]` → skipped; per-test overrides | [VERIFIED: scripts/install-tools.sh:34 `CARGO_NEXTEST_VERSION=0.9.148`; `cargo nextest --version` → `cargo-nextest 0.9.148`] |
| cargo-llvm-cov | 0.9.1 (new pin) | One-time spawned-binary coverage proof | Locked by CONTEXT. [VERIFIED: `cargo llvm-cov --version` → `cargo-llvm-cov 0.9.1`; GitHub latest release `v0.9.1`, published 2026-09-06] |
| git | 2.56.0 | Fixtures, oracle, observation | [VERIFIED: `/usr/bin/git --version` → `git version 2.56.0`] |
| bash | host 5.3.9 (`/usr/bin/bash`) | Runs the oracle | [VERIFIED: `/usr/bin/bash --version`] |
| ssh-keygen | `/usr/bin/ssh-keygen` | Per-run signing key | [VERIFIED: present; signing executed this session] |

### Supporting
| Item | Purpose | When to use |
|---------|---------|-------------|
| std `std::os::unix::ffi::{OsStrExt, OsStringExt}` | Byte-exact path names (newline, non-UTF-8) | Fixture writers and observers |
| std `std::process::{Command, Stdio}` | Every spawn; `Stdio::null()` stdin except the pre-push feed | Always |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `scenarios!` `macro_rules!` registry | `inventory` / `linkme` crates | Adds a dependency with link-section tricks; not needed, because one macro per module gives a registry and the tests from one list |
| `#[ignore]` for disabled Rust sides | Runtime early return | A runtime return shows as **PASS**: "a skipped check prints nothing a log reader can tell apart from a passing one" (global policy). Rejected |
| `status-level = "skip"` in nextest config to list disabled names | — | Verified noisy: with a `-E` filter it prints every filtered-out test as SKIP too, indistinguishable from ignored. Rejected |
| Tree SHA as the expected tree | Expected `(path bytes, mode, content)` list | A SHA literal is unreadable to the owner and hard to author before the run. Recommend entries, rendered as a path diff |

**Installation:**
```toml
# Cargo.toml
[dev-dependencies]
tempfile = "3.27.0"
```
Then `cargo generate-lockfile`/build (the lock updates 13 packages) and `scripts/check.sh deps`.

**Version verification:** `cargo info tempfile` → 3.27.0. `cargo-llvm-cov` latest = v0.9.1 (gh api `releases/latest`).

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| tempfile | crates.io | since 2015-04-14 | 17.4M/wk | github.com/Stebalien/tempfile | OK | Approved (dev-dependency) |
| cargo-llvm-cov (tool binary, not a crate dependency) | GitHub releases | — | — | github.com/taiki-e/cargo-llvm-cov | not run through the seam (not a crate dependency) | Pin by sha256 in install-tools.sh |

[VERIFIED: `gsd-tools query package-legitimacy check --ecosystem crates tempfile` → `"verdict": "OK"`, `"weeklyDownloads": 17412977`, `"repoUrl": "https://github.com/Stebalien/tempfile"`]

**Transitive graph (resolved this session, Rust 1.99.1):** tempfile 3.27.0 → fastrand 2.5.0, getrandom 0.4.3 (cfg-if, libc, r-efi 6.0.0), once_cell 1.21.4, rustix 1.1.5 (bitflags, errno, libc, linux-raw-sys 0.12.1, windows-sys 0.61.2 → windows-link 0.2.1). Every one offers MIT as an SPDX alternative.

**cargo-deny result:** with wsx's own `deny.toml` (`allow = ["MIT"]`, `[graph] all-features = true`) plus tempfile, `cargo deny --locked check` → `advisories ok, bans ok, licenses ok, sources ok`, rc=0. [VERIFIED: scratch copy of Cargo.toml + deny.toml]

**Finding, needs awareness:** cargo-deny 0.20.2 does **not** license-check dev-dependencies by default. `cargo deny list` showed only `wsx@0.0.0`. With `allow = ["ISC"]`, only wsx was rejected. Adding `include-dev = true` under `[licenses]` made it reject all 13 packages under `ISC`, and pass all of them under `["MIT"]`. So the license gate currently proves nothing about tempfile. Whether to add `include-dev = true` is a policy choice (Open Question 4); either way the gate passes.

**cargo-llvm-cov release asset (for install-tools.sh):**
- URL: `https://github.com/taiki-e/cargo-llvm-cov/releases/download/v0.9.1/cargo-llvm-cov-x86_64-unknown-linux-musl.tar.gz`
- sha256 `3fca950394a3c49457657c158b1619cec8dfd2647ae5b48746734c0ab969a522`. [VERIFIED: the GitHub API `digest` field and a local `sha256sum` of the downloaded tarball agree]
- The archive holds a single member, `cargo-llvm-cov`, at its root (`tar -tzvf`).
- The gnu asset is sha256 `b3f68e625481fed9b16444174f3fa5ebcdbde4a1878803a35eabe2dcefcdc41a` (API digest only).
- **Not verified:** the downloaded binary's `--version` output. Running it was blocked by the session's safety classifier. The host's installed copy prints `cargo-llvm-cov 0.9.1`, so the install-tools pattern is `^cargo-llvm-cov 0\.9\.1$` [ASSUMED for the release binary].

## Oracle inventory (question 1)

Source: `git -C ~/Github/devflow show origin/develop:<path>` at `ee6ec792d03f1495fc8c4d90d5894ff08b968d80` (commit "Merge pull request #228 from denniyahh/feature/dependabot", 2026-10-23). The devflow repo and working tree were not modified.

**Blob SHAs to record in `tests/oracle/README.md`.** [VERIFIED: `git ls-tree -l origin/develop ...`, and `git hash-object --no-filters` on the extracted copies matched all five]

| devflow path | mode | blob | size |
|---|---|---|---|
| `scripts/sync-workspace.sh` | 100755 | `82ed2d7572e03c7527f3deced2a51562ba891e85` | 1657 |
| `scripts/phase-worktree.sh` | 100755 | `ae6e47022137768a6ff947151c7169b898781719` | 4759 |
| `scripts/cut-pr-branch.sh` | 100755 | `d77eda55b84aa67215ecf51ddde2b38c982a6989` | 12443 |
| `scripts/check-workspace-divergence.sh` | 100755 | `0041f8a50fae25a2b88b1fe4ac9393314697909f` | 4215 |
| `scripts/hooks/pre-push` | 100755 | `67ef847ecf98dfaa1b124be5b10179b7a0134dc9` | 14183 |
| `LICENSE` | 100644 | `9fde56fe9dc9a61107fb774925149bd11942de67` | 1224 |

`LICENSE` text: "MIT License / Copyright (c) 2026 Dennis Kim", followed by "Alternatively, this software may be used under the terms of the Apache License, Version 2.0". Vendor it byte-identical as `tests/oracle/LICENSE`.

### Per-script facts

**`sync-workspace.sh`** (39 lines)
- CLI: `sync-workspace.sh [BASE_REMOTE=origin] [BASE_BRANCH=develop]`.
- Must run from a checkout whose HEAD is `workspace/*` or `personal/*`. Otherwise exit 1; detached HEAD exits 1.
- `git fetch $BASE_REMOTE $BASE_BRANCH`, then fast-forwards a local `develop` only if no worktree shows `[develop]`. That step is `|| true`, so a failure is silent.
- Merges with `-m "chore: sync $BASE_BRANCH into $CURRENT_BRANCH"`. If already an ancestor it is a no-op, exit 0. A conflict exits git merge's code via `set -e` and leaves the conflict state.
- Hard-coded: `origin`, `develop`, the branch-name patterns.
- **Patch: none.**

**`phase-worktree.sh`** (104 lines)
- CLI: `phase-worktree.sh <phase-number> [--no-sync]`; env `WORKSPACE_BASE`. Otherwise it uses the only `refs/heads/workspace/*` branch; any other count exits 2. A bad phase number exits 2.
- **GSD-specific:** requires a `^### Phase N:` heading in `.planning/ROADMAP.md`, read from the working tree. Otherwise exit 1.
- Calls the relative `scripts/sync-workspace.sh` (lines 54, 61), and **silently skips the sync (exit 0) when the file is absent**. [VERIFIED: an unpatched run printed `==> scripts/sync-workspace.sh not found; skipping sync`, rc=0]
- Sync runs only if HEAD equals `WORKSPACE_BASE`; otherwise it is skipped with a message.
- Asserts the workspace tracks at least one `.planning/` file.
- Creates `${ROOT}/.worktrees/phase-N` on `feature/phase-N`. That directory shows as untracked `?? .worktrees/` in the main checkout (verified).
- An existing worktree is advanced with `merge --ff-only`. If the branch has its own commits, exit 1 (verified).
- **Patch: redirect `scripts/sync-workspace.sh`.**

**`cut-pr-branch.sh`** (300 lines)
- CLI: `[--phase N|--phase=N] [PR_BRANCH] [BASE_BRANCH=develop] [WORKSPACE_BASE]`; env `PHASE`, `BASE_REMOTE` (default `origin`), `WORKSPACE_BASE`.
- Phase mode is automatic on `feature/phase-N`. The scope regex is `^[a-z]+\((phase-)?0*N(-[0-9]+)?\)!?: `, applied to commits from `git log --reverse --no-merges BASE_REF..HEAD`.
- The default PR name strips `workspace/`, `personal/` and the leading `<word>-`, which is backlog 999.146.
- Refuses a dirty tracked tree (`diff-index`). Fetches the base, then runs `git branch -f PR BASE_REF` **unconditionally**.
- **Checks the PR branch out in the user's tree.**
- Per commit:
  - `diff-tree --name-only`, which is quoted and shows no files for a merge commit;
  - `cherry-pick --no-commit ... || true`;
  - `git rm` of forbidden paths;
  - `commit -C HASH --no-verify`.
- Audits with `git log --name-only`, which is quoted.
- Fidelity: "changes paths no replayed commit touched" → exit 1; paths that differ from the source only warn.
- `FORBIDDEN_PATHS` is hard-coded (line 119), plus the `only` lines of `<CURRENT_BRANCH>:.workspace-divergence`, split on whitespace.
- Needs: a remote with the base branch, and a checkout of the source branch.
- **Patch: none.**

**`check-workspace-divergence.sh`** (106 lines)
- CLI: `[BASE_REF=origin/develop]`.
- Reads `BUILTIN` with `sed` from the **relative** `scripts/cut-pr-branch.sh` (line 30). Missing → exit 1.
- Reads `.workspace-divergence` from the **working tree**.
- Compares `git diff --name-status --no-renames BASE_REF HEAD`, which is quoted.
- Flags stale declarations.
- Requires mode `100755` for `scripts/hooks/**` and `scripts/*.sh`.
- Exit 1 on any problem.
- A workspace that has not synced since the base moved fails with "undeclared change to a develop file" (verified). Fixtures must sync first, or record that outcome.
- **Patch: redirect `scripts/cut-pr-branch.sh`.**

**`hooks/pre-push`** (250 lines)
- argv `<remote> <url>`; stdin is ref lines.
- `unset $(git rev-parse --local-env-vars) GIT_NAMESPACE GIT_DISCOVERY_ACROSS_FILESYSTEM GIT_CONFIG_GLOBAL GIT_CONFIG_SYSTEM` (lines 34-36). This is the hermeticity trap described under question 4.
- The policy checks only `refs/heads/{develop,main,feature/*,fix/*,chore/*,release/*}` (line 88).
- Range: `remote_sha..local_sha`, or `local_sha --not --remotes=origin` for a new ref (line 105, **hard-coded origin**).
- Matches `git log --name-only` output (quoted, no merge diffs) against a prefix regex with no boundary (line 115).
- Then the release-key policy runs; it is a no-op while `devflow.releaseSigningKey` is unset.
- Then `./scripts/check-workspace-divergence.sh origin/develop`, only if `.workspace-divergence` exists in cwd (line 209; **relative path and hard-coded origin/develop**).
- Then the `pre-push.d` drop-ins under `$(dirname "$0")`; none exist in the scratch copy.
- Then `scripts/check-in-container.sh all`, or `scripts/check.sh all` when `DEVFLOW_SKIP_CONTAINER_CHECK=1` (lines 233-250).
- **Patch: redirect line 209; remove the CI-mirror tail.**

**External tools the oracle uses:** bash ≥4 (`mapfile`), git, grep -E, sed, awk (gawk on the host; the container image's awk was not checked), sort, comm, paste, head, tail, ssh-keygen (only for the release-key branch, which never runs). All are in `/usr/bin` on the host. The CI image was not re-verified this session (see Environment Availability).

### Verified candidate `oracle.patch` (4 substitutions, 72 lines)

It was built with exact-match substitutions (each `old` string occurs exactly once) and `git diff --no-index`. `git apply -v` on a pristine copy reported `Applied patch ... cleanly` for all three files, and the results were byte-identical to the intended files. Re-applying failed (`patch does not apply`), as it should. [VERIFIED this session]

| File | Old (verbatim from devflow) | New (proposed) |
|---|---|---|
| `check-workspace-divergence.sh` line 30 | `' scripts/cut-pr-branch.sh)"` | `' "${WSX_ORACLE_DIR:?}/cut-pr-branch.sh")"` |
| `phase-worktree.sh` line 54 | `if [ -x scripts/sync-workspace.sh ]; then` | `if [ -x "${WSX_ORACLE_DIR:?}/sync-workspace.sh" ]; then` |
| `phase-worktree.sh` line 61 | `        scripts/sync-workspace.sh` | `        "${WSX_ORACLE_DIR:?}/sync-workspace.sh"` |
| `hooks/pre-push` line 209 | `    ./scripts/check-workspace-divergence.sh origin/develop` | `    "${WSX_ORACLE_DIR:?}/check-workspace-divergence.sh" origin/develop` |
| `hooks/pre-push` lines 233-250 | from `# Verify in the pinned devcontainer image` to EOF | a 2-line comment + `exit 0` |

- `WSX_ORACLE_DIR` is a **proposed** name (Claude's discretion). `${VAR:?}` fails loud: without it, pre-push exited 1 with `WSX_ORACLE_DIR: parameter null or not set` (verified). That is the negative control showing the adapter's variable is load-bearing.
- `origin/develop` and `origin` stay hard-coded on purpose: they are oracle behaviour, not paths.
- The patch must not touch `cut-pr-branch.sh` line 119 (`FORBIDDEN_PATHS=(...)`), because the divergence check `sed`s it.
- **Self-tests passed with the patched copy** in a hermetic fixture (verified):
  - `phase-worktree.sh 1` now syncs (`==> syncing workspace/dev`, `Merging origin/develop`) and creates the worktree (rc 0);
  - `cut-pr-branch.sh` from the worktree prints `Audit passed` (rc 0);
  - `pre-push` from the workspace checkout runs the divergence check (`every difference from origin/develop is declared`, rc 0);
  - the divergence check alone gives rc 1 with the expected "undeclared" line on an undeclared commit (a known-positive control).

**ROADMAP lookup: do not patch it out.** It is behaviour, not a path. The oracle adapter's devflow-shaped fixture writes `.planning/ROADMAP.md` containing `### Phase N:` on the workspace branch. `new <branch>` maps to the oracle only for `feature/phase-N` names; any other name is Rust-only.

## Oracle defects reproduced (question 2)

All were run in hermetic fixtures with these settings:
- `env -i`, `PATH=/usr/bin:/bin`;
- `GIT_CONFIG_GLOBAL=$HOME/.gitconfig` with `GIT_CONFIG_NOSYSTEM=1`;
- `GIT_CEILING_DIRECTORIES`;
- deterministic dates.

Each was run against the **unpatched** devflow script; for pre-push, only the CI tail was cut.

| # | Defect | Script | Observed (this session) | Real negative control for |
|---|---|---|---|---|
| D1 | Quoted-name leak in the cut | cut | `feat(1)` commit adding `src/b.txt`, `.planning/é.md` and `.planning/plain.md` → rc 0, "Audit passed"; the PR tree contains **`.planning/é.md`**, while `plain.md` was stripped | PRCUT-04, HARN-06 names |
| D2 | Merge commit silently dropped | cut (non-phase) | Topic branch with a `--no-ff` merge → `Excluded: 1 planning/environment-only commit(s)`; the merge was mislabelled and dropped, rc 0 | PRCUT-01 merges, HARN-06 merges |
| D3 | Unconditional `git branch -f` | cut | Reviewer commit `fix(1): review fixup` on `feature/phase-1-pr`; a re-cut gives rc 0 and the **commit is lost** from the branch | PRCUT-08, HARN-06 existing PR branch |
| D4 | Partial PR branch on conflict | cut | Second commit conflicts with develop → rc 1 (`unresolved code conflict ... src/a.txt`); **`feature/phase-1-pr` left holding `feat(1): add b`** | PRCUT-03 |
| D5 | Unscoped commit silently dropped | cut (phase mode) | `feat(1): scoped` + `fix: unscoped code` → rc 0, `Included: 1`, **no warning**, `src/c.txt` absent | PRCUT-05, HARN-06 scoped+unscoped |
| P1 | Quoted-name bypass (non-ASCII, newline, non-UTF-8) | pre-push | `.planning/é.md`, `.planning/a\nb.md` and `.planning/\377.md` each → **rc 0**; log prints `".planning/\377.md"` | GUARD-02, HARN-06 names |
| P2 | Pattern-limited ref list | pre-push | `.planning/y.md` pushed to `refs/heads/wip/y` → **rc 0** | GUARD-01 |
| P3 | New branch with no own commits | pre-push | branch at workspace tip (tree has `.planning`) pushed as new `feature/noown` → **rc 0** | GUARD-01(a) |
| P4 | Merge resolution adds a personal file | pre-push | merge resolving `src/a.txt` and adding `.planning/STATE.md` → **rc 0**; `git log --name-only` lists no files for the merge | GUARD-01(b), HARN-06 merges |
| P5 | Name-prefix over-refusal | pre-push | `.planningX/z.md` → **rc 1 REFUSING** | CONF-01 prefix boundary |
| Controls | Guard works on the plain case | pre-push | clean (`src/clean.txt`) rc 0; known-positive (`.planning/x.md`) rc 1 REFUSING | HARN-05 |
| W1 | Re-run with own commits fails | phase-worktree | rc 1 "not a fast-forward ... refusing" | WTREE-03 (wsx: exit 0) |

**Not reproduced (derived from reading the code):**
- **Hard-coded origin in the guard range** (line 105, `--remotes=origin`). This is expected to over-scan the whole history when pushing to a remote without `origin/*` refs (`[VERIFIED: hooks/pre-push:105 read via git show]`; outcome not executed).
- **The swallowed cherry-pick error**: `|| true` on line 199. D2 is its visible effect for merges.
- **The divergence check flagging quoted names as undeclared.** This is an over-refusal; not executed.

**Doc discrepancy (needs awareness):** REQUIREMENTS PRCUT-05 says "the oracle only warns about left-out commits". devflow's original prints no warning for the dropped commit (D5). The only warning is the fidelity "paths do not match" line, which fires only when a replayed path differs. wsx's own `scripts/workspace/` port behaves the same way (`rg` shows only that fidelity warning). Fix the requirement text, or record the oracle outcome accurately.

## HARN-06 case map (question 3)

Under the locked CONTEXT rule, the oracle runs devflow-shaped configs only.

| Case | Oracle implements? | Scenario disposition | Notes |
|---|---|---|---|
| Merge commits: integration merged into a feature branch; conflict resolved by adding a personal file | **Yes** (cut drops the merge, D2; guard misses the resolution, P4) | Oracle-recorded, `fixed:` for cut and guard | Guard scenario: known-positive = plain `.planning/x.md` commit; clean = merge with no personal file |
| Squash-merged PRs | Partly: sync and re-cut run; there is no `rm` | Re-cut after a squash merge and sync after a squash merge: oracle-recorded. `rm` after a squash merge: **Rust-only** (no oracle `rm`) | Expected oracle re-cut: commits already applied cherry-pick to empty, so they are "excluded", and with nothing left the cut ends "PR branch is empty", exit 1 [ASSUMED from code lines 231-249; not executed] |
| Fork layout (base `upstream`, push `origin`, no `origin/<integration>`) | Partly: sync takes `$1` remote, cut takes `BASE_REMOTE` env, check takes `BASE_REF`. But `phase-worktree` calls sync with no args, and pre-push hard-codes `origin/develop` and `--remotes=origin` | **Rust-only** (locked) | See Open Question 1: the oracle could record the cut and sync with `upstream` |
| Scoped and unscoped commits in one cut | **Yes** (D5) | Oracle-recorded, `fixed:` | The oracle supports numeric scopes only (`(N)`, `(N-PP)`, `(phase-N)`); a non-numeric `--scope` is Rust-only |
| Repo with no GSD, devflow or agent files (IF-02) | sync, cut and guard need none of them; `phase-worktree` refuses (no ROADMAP heading) | **Rust-only** (locked) | See Open Question 1: the `new` refusal is a ready negative control |
| Non-ASCII, newline, non-UTF-8 names | **Yes** (D1, P1) | Oracle-recorded, `fixed:` for cut and guard; divergence check outcome to be recorded | Fixture writes names through `OsStr::from_bytes` |
| Existing PR branch with extra commits | **Yes** (D3) | Oracle-recorded, `fixed:` | |

## Hermetic environment (question 4)

### Environment for every spawned process

| Variable | Value | Why |
|---|---|---|
| (base) | `Command::env_clear()` | Drops `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_CONFIG_PARAMETERS`, `GIT_CONFIG_COUNT`, HOME, SSH_AUTH_SOCK and the rest. [VERIFIED: `git rev-parse --local-env-vars` → `GIT_ALTERNATE_OBJECT_DIRECTORIES GIT_CONFIG GIT_CONFIG_PARAMETERS GIT_CONFIG_COUNT GIT_OBJECT_DIRECTORY GIT_DIR GIT_WORK_TREE GIT_IMPLICIT_WORK_TREE GIT_GRAFT_FILE GIT_INDEX_FILE GIT_NO_REPLACE_OBJECTS GIT_REPLACE_REF_BASE GIT_PREFIX GIT_SHALLOW_FILE GIT_COMMON_DIR`] |
| `HOME` | `<scenario>/home` | |
| `GIT_CONFIG_GLOBAL` | **`<scenario>/home/.gitconfig`**: harness-written, at the default location | The oracle's pre-push unsets `GIT_CONFIG_GLOBAL`, and git then reads `$HOME/.gitconfig`. With the file at the default path, the unset changes nothing. [VERIFIED: probe A1-A3] |
| `XDG_CONFIG_HOME`, `XDG_DATA_HOME`, `XDG_CACHE_HOME`, `XDG_STATE_HOME` | `<scenario>/xdg/{config,data,cache,state}` (empty) | After the unset, git also reads `$XDG_CONFIG_HOME/git/config`, and **always** reads `git/ignore` and `git/attributes` there |
| `GIT_CONFIG_SYSTEM` | `<scenario>/system.gitconfig` (empty) | Locked by CONTEXT |
| **`GIT_CONFIG_NOSYSTEM`** | `1` | **Required in addition.** After the oracle's unset, git read the host's `/etc/gitconfig` (`filter.lfs.required=true` on this host); `GIT_CONFIG_NOSYSTEM=1` survives the unset and blocks it. [VERIFIED: probe A2 vs A3] |
| **`GIT_CEILING_DIRECTORIES`** | the scenario `TempDir` path (canonicalised) | Fixtures and the oracle copy live under `CARGO_TARGET_TMPDIR`, which is inside the wsx checkout. Without the ceiling, a git command in a non-repo scratch dir discovers **the wsx repo**, and `git apply` silently skips (see Pitfall 1). Not in the local-env-vars list, so it survives the hook's unset |
| **`GIT_ALLOW_PROTOCOL`** | `file` | Fixture remotes are local paths. With this set, `https` fails: `fatal: transport 'https' not allowed`, rc 128; the control without it reached GitHub. Belt and braces: no scenario can ever touch the network. [VERIFIED] |
| `GIT_TERMINAL_PROMPT` | `0` | Never block on credentials |
| `GIT_EDITOR`, `GIT_SEQUENCE_EDITOR` | `false` | An editor launch fails loud instead of hanging. The oracle sync and cut ran fine with it (verified) |
| `GIT_MERGE_AUTOEDIT` | `no` | `git merge -m` never opens an editor |
| `LC_ALL` | `C` | Stable git messages; matches `tests/setup_repo.rs:44` `.env("LC_ALL", "C")` |
| `TZ` | `UTC` | Display formats; stored dates carry an explicit `+0000` |
| `PATH` | `/usr/local/bin:/usr/bin:/bin` (proposal) | The host's linuxbrew bash and ssh-keygen come first on the user's PATH and must not be picked up. Add a test asserting that the hermetic `git --version` is ≥ 2.54 |
| `GIT_AUTHOR_DATE`, `GIT_COMMITTER_DATE` | fixture: `@<1767225600 + 60·n> +0000`, incremented per git invocation; action: one fixed later instant | 1767225600 = 2026-01-01T00:00:00Z. A ticking clock avoids identical timestamps across branchy history (see Pitfall 5). Format accepted (verified: the commit header showed `1767226500 +0000`) |
| Identity | `user.name`/`user.email` in the harness `$HOME/.gitconfig`; per-step overrides via `-c` or `--author` | Identity in config, not env, so a leaked planted `user.name` is detectable (it changes the author). The probe used env identities; both work |
| `LLVM_*` | copied from the parent process only | Coverage (question 6) |
| `WSX_ORACLE_DIR` | patched oracle copy dir (oracle adapter only) | |
| stdin | `Stdio::null()`, except the pre-push ref lines | |

Existing pattern to extend: `tests/setup_repo.rs:30` `fn hermetic(program: &str, dir: &Path, stub_bin: &Path) -> Command`, which sets `.env("GIT_CONFIG_GLOBAL", "/dev/null")` (line 42), `.env("GIT_CONFIG_NOSYSTEM", "1")` (line 43) and `.env("LC_ALL", "C")` (line 44). [VERIFIED: read via `cat`/`rg -n` this session] The harness version differs:
- it uses a real global config file at `$HOME/.gitconfig` instead of `/dev/null`, so it survives the oracle's unset and can carry identity;
- it builds PATH from scratch rather than prefixing the parent's.

### Hermeticity proof design (success criterion 2), verified mechanism

**Make the env builder a pure function of the inherited environment:**
- signature: `fn hermetic_env(inherited: &[(OsString, OsString)], dirs: &ScenarioDirs) -> Vec<(OsString, OsString)>`;
- production passes `std::env::vars_os()`;
- the proof passes a **planted host environment**.

That tests the real risk: the builder letting anything from the parent through. No `set_var` is needed, and the user's real `~/.gitconfig` and `~/.config/git/hooks` are never touched. This host does have a global `core.hooksPath` (PITFALLS Pitfall 7).

**Planted host** (a fake host home inside the scenario TempDir):
- `HOME/.gitconfig` and `GIT_CONFIG_GLOBAL` pointing at a planted file with:
  - `core.hooksPath=<planted>/hooks`, holding `reference-transaction`, `pre-commit`, `commit-msg`, `post-commit`, `post-checkout`, `pre-merge-commit` and `pre-push`, each appending its name to a marker file;
  - `hook.planted.command=echo config-hook >> <marker>` with `hook.planted.event=pre-commit` (and pre-push);
  - `init.templateDir=<planted>/tmpl`, with `hooks/post-commit`;
  - `user.name=Host Person`;
  - `core.quotePath=false`. That would *hide* the oracle's quoted-name defect, so the D1 scenario's recorded outcome would change;
  - `commit.gpgsign=true` with `user.signingkey=/nonexistent`. Any commit through a leaked config then fails.
- `XDG_CONFIG_HOME` → planted `git/config`, plus `git/ignore` containing `src/`.
- `GIT_CONFIG_SYSTEM` → planted.
- `GIT_CONFIG_PARAMETERS="'core.hookspath'='<planted>/hooks'"`.
- `GIT_DIR` → a planted decoy repo.
- `LLVM_PROFILE_FILE=<x>`, which must come through.

**Assertions:**
- The tracer sync scenario, a signing cut scenario and a pre-push scenario each reproduce their **explicit expected outcome**.
- The marker file does not exist.
- The decoy repo's refs are unchanged.
- The built env contains `LLVM_PROFILE_FILE` and no other inherited key.

**Control ("wired to the planted config"):** the same scenario, run with the planted global config passed through (`GIT_CONFIG_GLOBAL=<planted file>`, through a test-only builder flag). The marker **must** contain hook names and the outcome must differ (author `Host Person`, or the commit fails on signing). Verified by probe:
- the hermetic run gave author `Harness`, `quotePath unset` and no marker;
- the wired run gave author `Host Person`, `quotePath=false`, a marker with `config-hook` ×1, `hookspath:pre-commit` ×1 and `hookspath:reference-transaction` ×10, and the template copied `post-commit` into `.git/hooks/`.

**What the proof does not establish:** that git has no other config source. The planted set covers the documented ones: system, global, XDG, `GIT_CONFIG_PARAMETERS`/`COUNT`, `init.templateDir`, `core.hooksPath` and config hooks. `include.path` chains from the planted file are covered transitively.

## SSH signing in a fixture (question 5)

Verified on git 2.56.0 with `/usr/bin/ssh-keygen`. [VERIFIED this session]

```text
ssh-keygen -q -t ed25519 -N '' -C harness -f <scenario>/signkey  </dev/null      # rc 0
<scenario>/allowed_signers:   <committer email> namespaces="git" <type> <base64 pubkey>
git config gpg.format ssh
git config user.signingkey <scenario>/signkey          # PRIVATE key path: env_clear() removes SSH_AUTH_SOCK
git config commit.gpgsign true
git config gpg.ssh.allowedSignersFile <scenario>/allowed_signers
```

Results:
- **The oracle cut signs replayed commits.** The replayed commit header has a `gpgsig -----BEGIN SSH SIGNATURE-----` line, and `%G?` is `G`. The unsigned source commit has no `gpgsig` and `%G?` is `N`. `git verify-commit` gives rc 0 on the replayed commit and rc 1 on the unsigned one.
- The replayed commit keeps the source author and author date: `author Ada Author <ada@example.invalid> 1767226500 +0000`. The committer comes from the action env.
- **The oracle sync merge signs:** parents=2, `%G?=G`, subject `chore: sync develop into workspace/dev`.
- **How to assert signature presence:** parse `git cat-file commit <sha>` raw headers. Present means a `gpgsig` (or `gpgsig-sha256`) header exists. Validity is optional: `git verify-commit` exit 0 needs `allowedSignersFile`. Record presence as `bool`, separate from commit identity, because signatures make SHAs non-reproducible (the per-run key).
- **Default config is `commit.gpgsign=false`** (locked). The planted-host proof relies on that: leaked `commit.gpgsign=true` with `/nonexistent` makes commits fail.

## Coverage with spawned binaries (question 6)

Probe crate in scratch (stable 1.99.1 with llvm-tools, cargo-llvm-cov 0.9.1, nextest 0.9.148). [VERIFIED this session]
- `cargo llvm-cov show-env` sets `LLVM_PROFILE_FILE='<target>/probe-%p-%4m.profraw'`, `RUSTC_WRAPPER=cargo-llvm-cov`, `CARGO_LLVM_COV=1` and others. Under `cargo llvm-cov nextest`, the test saw `LLVM_PROFILE_FILE=<target>/llvm-cov-target/probe-%p-%4m.profraw`.
- **Spawn with `env_clear()` plus `LLVM_*` copied:** the function only the binary runs was covered (`1|      1|fn only_spawned()`), and no `.profraw` appeared in the cwd.
- **Spawn with `env_clear()` and no passthrough:** coverage was lost (the run counted `main` once although it was spawned twice), and **`default_16279743982090015093_0_2288518.profraw` appeared in the spawned process's cwd.** In the harness that cwd is the fixture, which would corrupt the files-on-disk expectation.
- **Negative controls:** running only the no-passthrough test gave `only_spawned` count **0** and `main` count 0. Running only a test that never spawns the binary gave count 0.
- `CARGO_BIN_EXE_<name>` points at the instrumented binary under llvm-cov, so no harness change is needed beyond the passthrough.

**Blocking fact for this repo:** `rust-toolchain.toml:12-13` pins `channel = "1.99.1"` with `components = ["clippy", "rustfmt"]`. That toolchain dir has no `llvm-cov` or `llvm-profdata`; only `stable` has them. [VERIFIED: `ls ~/.rustup/toolchains/1.99.1-.../lib/rustlib/x86_64-unknown-linux-gnu/bin` → `gcc-ld rust-lld rust-objcopy wasm-component-ld`]
- `CARGO_LLVM_COV_SETUP=no cargo +1.99.1 llvm-cov nextest` → `error: failed to find llvm-tools-preview, please install llvm-tools-preview with rustup component add llvm-tools-preview --toolchain 1.99.1-x86_64-unknown-linux-gnu`, rc 1.
- **Caution:** with `CARGO_LLVM_COV_SETUP` unset and stdin not a TTY, cargo-llvm-cov reads an empty answer as "yes" and runs `rustup component add` itself (source `src/context.rs`, CITED). **Always run it with `CARGO_LLVM_COV_SETUP=no`**, so it never installs anything silently. See Open Question 2.

**install-tools.sh pin (pattern to copy from `cargo-deny`).** `verify-cargo cargo-llvm-cov` asks `cargo llvm-cov --version`.
```bash
CARGO_LLVM_COV_VERSION=0.9.1
CARGO_LLVM_COV_SHA256=3fca950394a3c49457657c158b1619cec8dfd2647ae5b48746734c0ab969a522
# tool_info case:
#   url="https://github.com/taiki-e/cargo-llvm-cov/releases/download/v${version}/cargo-llvm-cov-x86_64-unknown-linux-musl.tar.gz"
#   member="cargo-llvm-cov"; version_arg="--version"; pattern="^cargo-llvm-cov ${version//./\\.}\$"
```
Existing pins for comparison: `scripts/install-tools.sh:34` `CARGO_NEXTEST_VERSION=0.9.148` and `:36` `CARGO_DENY_VERSION=0.20.2`. The usage text and the `tool_info` unknown-tool list must gain the new name.

**One-time proof (run on the host, record the output in the plan SUMMARY).**
1. Use a harness test that spawns `env!("CARGO_BIN_EXE_wsx")` through the hermetic env builder with `--version`. The spawned binary is the only executor of `src/main.rs`'s `--version` arm, because integration tests never link the crate.
2. Run `CARGO_LLVM_COV_SETUP=no cargo llvm-cov nextest --no-tests=fail -E 'test(=<that test>)' --text`. The `println!("wsx {}", ...)` line count must be ≥ 1.
3. Negative control A: `-E 'test(=<a test that does not spawn wsx>)'` gives the same line count 0.
4. Negative control B: a test-only builder flag without the `LLVM_*` passthrough gives count 0, plus a stray `*.profraw` in the scenario dir.

Also add a permanent post-scenario invariant: no `*.profraw` file in the fixture tree. It only bites under coverage, and catches a lost passthrough.

## tempfile (question 7)

- 3.27.0, `MIT OR Apache-2.0`, MSRV 1.63. Default feature `getrandom`. [VERIFIED: `cargo info`]
- `TempDir::new_in<P: AsRef<Path>>(dir: P) -> Result<TempDir>`: "Attempts to make a temporary directory inside of `dir`. The directory and everything inside it will be automatically deleted once the returned `TempDir` is destroyed." `with_prefix_in(prefix, dir)` adds a name prefix; `disable_cleanup` and `keep` exist. [CITED: docs.rs/tempfile/3.27.0 TempDir]
- `dir` must already exist. Cargo sets `CARGO_TARGET_TMPDIR` for integration tests, and the existing tests already create paths under it (`tests/setup_repo.rs:15` `const TMP_ROOT: &str = env!("CARGO_TARGET_TMPDIR");`).
- Cleanup runs on drop, including during panic unwinding (the default `panic = "unwind"` in the test profile [ASSUMED]). It does **not** run when nextest kills a timed-out test, so leftovers then stay in `target/tmp`. That is disk, not tmpfs.
- Use `with_prefix_in("<scenario-name>-", ...)` so a leftover is attributable.
- deny: see the Package Legitimacy Audit (clean; dev-dependency licenses unchecked by default).

## nextest naming and the scenario registry (question 8)

**nextest naming (verified on a probe):**
- the binary id is `<crate>::<test-target>`, e.g. `probe::spawn`;
- the test name is the function's module path inside that target;
- so `tests/harness/main.rs` with `mod sync { mod merges_integration { fn oracle() } }` gives `wsx::harness sync::merges_integration::oracle`;
- gate one test with `-E 'test(=sync::merges_integration::oracle)'`.

**`#[ignore = "..."]` behaviour (verified):**
- counted as skipped: `Starting 2 tests across 2 binaries (1 test skipped)` … `2 tests run: 2 passed, 1 skipped`;
- not named at the default status level;
- a filter selecting **only** an ignored test fails: `error: no tests to run`, rc 4, under `--no-tests=fail`. So `-E 'test(=x::rust)'` on a disabled side can never pass silently;
- `status-level = "skip"` names it, but also names every filtered-out test (verified noise). Do not use it.

**Report by name (verified):**
- a test `registry::rust_sides_not_enabled` prints the list;
- `.config/nextest.toml` gains:
  ```toml
  [[profile.default.overrides]]
  filter = "test(=registry::rust_sides_not_enabled)"
  success-output = "final"
  ```
- nextest then prints its stdout in the final summary of every full run. Probe output: `PASS ... rust_sides_not_enabled` / `stdout ───` / `Rust side not enabled (2): sync_up_to_date [phase 7], cut_quoted_name [phase 9]`;
- without the override, passing tests' output was not shown.

**Registry mechanism (no new dependency).** Use one `macro_rules!` invocation per scenario module. From a single list it generates:
1. per scenario, a module with `#[test] fn oracle()` (only for oracle-capable scenarios) and `#[test] fn rust()` carrying `#[ignore = "Rust side not enabled until phase N"]` while disabled;
2. `pub fn registry() -> Vec<Entry>`.

A top-level `harness_modules!(sync, worktree, cut, check, guard, ...)` both declares the `mod`s and concatenates their registries. A module cannot be declared without being registered.

**Every generated test calls `scenario.validate(&entry)` before running**, so a malformed scenario fails its own test with a plain-words reason, whether or not any meta-test runs. `registry::every_scenario_is_well_formed` also iterates all entries.

Phases 5-10 enable a Rust side by deleting the `disabled = "phase N"` token in the macro list. That is one obvious diff per scenario.

## Architecture Patterns

### System Architecture Diagram

```text
 scenario fn ──► Scenario { config, fixture steps, action, expected, oracle record, class, guard controls }
      │
      ▼ (generated #[test] <name>::oracle  /  <name>::rust [#[ignore] while disabled])
 validate(scenario, macro flags) ──malformed──► panic: plain-words reason (test FAILS)
      │ ok
      ▼
 TempDir::with_prefix_in(name, CARGO_TARGET_TMPDIR)
      │
      ▼
 hermetic_env(inherited = vars_os, dirs) ──► env list (env_clear + fixed set + LLVM_* passthrough)
      │
      ▼
 FixtureBuilder: bare remotes, work repo, commits (ticking clock), branches, merges,
                 worktrees, local config (signing key if requested), byte-named files
      │
      ├── snapshot "before" (refs, files) ─────────────────────────────┐
      ▼                                                                │
 Adapter (by side):                                                     │
   oracle: copy tests/oracle/* → TempDir, git apply oracle.patch       │
           (GIT_CEILING_DIRECTORIES), map Action → bash script + args   │
           (or NotImplemented → only valid for Rust-only scenarios)     │
   rust:   map Action → wsx <cmd> (CARGO_BIN_EXE_wsx)                   │
      │                                                                │
      ▼                                                                │
 Observer: exit code; refs (for-each-ref); commits (raw cat-file);      │
           signature presence; files on disk (fs walk, byte paths) ◄────┘
      │
      ▼
 compare(expected_for_side, observed) ──► Vec<FieldMismatch>
      │
      ├─ class rules: fixed: on oracle → observed must equal recorded defect AND differ from Rust expected
      │               regression: → expected must be a refusal (non-zero exit, refs unchanged)
      ▼
 PASS, or FAIL rendering "Expected: <plain words>  /  Got: <plain words>  /  Differences: ..."
```

### Recommended Project Structure

```text
tests/
├── oracle/                       # vendored, byte-identical to devflow ee6ec792 (reaches develop)
│   ├── LICENSE                   # devflow LICENSE, verbatim
│   ├── README.md                 # source commit, blob table, attribution, what the patch changes and why
│   ├── oracle.patch              # the single adaptation (4 substitutions)
│   ├── sync-workspace.sh  phase-worktree.sh  cut-pr-branch.sh  check-workspace-divergence.sh
│   └── hooks/pre-push
└── harness/                      # ONE integration-test binary: `wsx::harness`
    ├── main.rs                   # harness_modules!(...) ; registry report + well-formed meta-tests
    ├── env.rs                    # hermetic_env(inherited, dirs) — pure; Spawn helpers
    ├── fixture.rs                # FixtureBuilder (devflow-shaped preset + generic steps), deterministic clock
    ├── oracle.rs                 # copy + patch (ceiling) ; Action → script mapping
    ├── rust.rs                   # Action → wsx mapping (all disabled in phase 2)
    ├── outcome.rs                # Outcome/Expected types, observer, comparator, plain-words Display
    ├── scenario.rs               # Scenario, Class, RepoConfig, Action, GuardControls, validate(), scenarios! macro
    ├── provenance.rs             # blob-match, patch-applies, fail-loud tests
    ├── hermeticity.rs            # planted-host proof + control
    └── sync.rs  worktree.rs  cut.rs  check.rs  guard.rs   # scenario modules
```
`tests/oracle/` has no `main.rs`, so cargo makes no test target from it. Cargo auto-discovers `tests/*/main.rs` as a target named after the directory [ASSUMED: Cargo target auto-discovery].

### Pattern 1: Typed expectations, exhaustively compared
- **What:** `Expected` is a struct with every HARN-03 field required. The comparator destructures it **without `..`**, so adding a field without comparing it is a compile error.
- **When:** every scenario.
- **Example:**
```rust
// Sketch, not verified code. Names are proposals.
pub struct CommitExpect { pub message: &'static [u8], pub author: Ident, pub author_date: &'static str,
                          pub committer_name: &'static str, pub committer_email: &'static str,
                          pub tree: Vec<TreeEntry>, pub signed: bool }
pub struct Outcome { pub exit: i32, pub refs: Vec<&'static str>,
                     pub commits: Vec<(&'static str /*ref*/, Vec<CommitExpect>)>, pub files: Vec<FileEntry> }
fn compare(exp: &Outcome, got: &Observed) -> Vec<Mismatch> {
    let Outcome { exit, refs, commits, files } = exp;   // no `..`: a new field must be handled here
    /* ... */
}
```

### Pattern 2: Oracle record lives in the scenario
- `oracle: OracleSide::{SameAsExpected, Recorded(Outcome), NotImplemented { why }}`. `SameAsExpected` is legal only *alongside* an explicit `expected`; it can never stand alone.
- The oracle test asserts that the actual oracle outcome equals `expected` (parity) or `Recorded` (fixed). When the oracle is deleted (REL-02), the `Recorded` data and its comment are the surviving evidence.

### Pattern 3: Guard controls inline
- A guard scenario (action `PrePush`, later `PreCommit` and `Check`) carries `GuardControls { known_positive: Case, clean: Case }`.
- The generated test runs the subject plus both controls against the same implementation. `validate()` rejects a guard action without both.
- On the oracle, P1-P4 record "allowed" while the known-positive is refused and the clean control passes, which proves the bypass is real.

### Pattern 4: Deterministic fixture clock
- Every fixture git invocation gets `GIT_AUTHOR_DATE`/`GIT_COMMITTER_DATE = @<base + 60·n> +0000`. The action runs at a fixed instant after the fixture.
- Expected author dates are then known constants. Without signing, commit SHAs repeat across runs (D4's error named the same SHA each run), which helps debugging but must not be asserted (HARN-03).

### Anti-Patterns to Avoid
- **Differential-only harness** (`run(oracle) == run(rust)`): every assertion disappears at cutover, and shared defects pass (ARCHITECTURE.md anti-pattern). Use explicit `expected` plus the recorded oracle side.
- **A runtime "skip" for a disabled side:** it shows as PASS. Use `#[ignore]`.
- **`git apply` without a ceiling inside `target/`:** a silent no-op (Pitfall 1).
- **Running the unpatched pre-push in a fixture:** it runs `scripts/check-in-container.sh` and fails for an unrelated reason, or with `DEVFLOW_SKIP_CONTAINER_CHECK=1` it runs `scripts/check.sh` against the fixture.
- **Comparing stderr text:** "exact wording never counts" (REQUIREMENTS). Keep stdout and stderr for the failure rendering only.
- **Parsing `git log --name-only` or other quoted output in the observer:** use `-z` and raw `cat-file`, or the observer inherits the oracle's bug.

## Don't Hand-Roll

| Problem | Don't build | Use instead | Why |
|---------|-------------|-------------|-----|
| Scratch dir cleanup on panic | manual `remove_dir_all` in each test | `tempfile::TempDir` | Locked; drop runs on unwind |
| Blob hashing of vendored files | SHA-1 in Rust (needs a crate) | `git hash-object --no-filters <file>` (works outside a repo; verified) | No dependency; `--no-filters` avoids attribute and eol conversion |
| Applying the adaptation | sed or Rust string replace at test time | `git apply` of the tracked `oracle.patch` with a ceiling | Locked; reviewable diff |
| Test registry | `inventory`/`linkme` | `macro_rules!` per module | No dependency, no link-section magic |
| Coverage of the child | custom profraw handling | cargo-llvm-cov plus `LLVM_*` passthrough | Verified |
| Signature checks | parsing armor | `gpgsig` header presence; `git verify-commit` for validity | Verified |

**Key insight:** the oracle and git already contain the tricky behaviour. The harness should observe them byte-exactly and add no parsing of quoted output of its own.

## Common Pitfalls

### Pitfall 1: `git apply` silently skips inside a repo subdirectory
- **What goes wrong:** with cwd under any git work tree, `git apply` resolves patch paths from that repo's top level. It prints `Skipped patch 'script.sh'.` and exits **0**. [VERIFIED: probe B1]
- **Why:** `CARGO_TARGET_TMPDIR` is `<wsx checkout>/target/tmp`. In the container it is `/workspace/target/...`, and the gitdir is bind-mounted.
- **Avoid:** set `GIT_CEILING_DIRECTORIES=<TempDir>` (verified: `Applied patch script.sh cleanly`). Also assert each patched file's **resulting blob hash** equals a recorded value, not just the exit code.
- **Negative control:** the same apply without the ceiling, in a nested-repo scratch dir, must leave the file unchanged.

### Pitfall 2: The oracle's pre-push re-exposes the host's system config
- `unset GIT_CONFIG_GLOBAL GIT_CONFIG_SYSTEM` makes git read `/etc/gitconfig`, `$XDG_CONFIG_HOME/git/config` and `$HOME/.gitconfig` (verified).
- **Avoid:** `GIT_CONFIG_NOSYSTEM=1`, and keep the harness global config **at** `$HOME/.gitconfig`.
- **Test:** a pre-push scenario under the planted-host proof, with planted `core.quotePath=false` in a fake system file reached through the unset path. The recorded quoted-name outcome must not change.

### Pitfall 3: Unpatched `phase-worktree.sh` passes without syncing
- It prints "not found; skipping sync" and exits 0 (verified). A self-test that only checks rc and the worktree's existence passes vacuously.
- **Avoid:** the self-test expects the sync merge commit on the workspace branch (explicit commit expectation).

### Pitfall 4: Lost coverage, and profraw debris in fixtures
- `env_clear()` without `LLVM_*` gives 0 coverage plus `default_*.profraw` in the cwd (verified).
- **Avoid:** passthrough, plus the no-profraw invariant.

### Pitfall 5: Equal timestamps reorder history walks
- The oracle cut uses `git rev-list --reverse FORK..HEAD` without `--topo-order`. Git's default order is by commit date, and only `--date-order`/`--topo-order` promise "no parents before all of its children" [ASSUMED from git-rev-list docs]. With every fixture commit at the same instant, branchy fixtures could replay in a surprising order.
- **Avoid:** the ticking clock (Pattern 4).

### Pitfall 6: Divergence check against a stale workspace
- The oracle compares `BASE_REF` with `HEAD` trees, so a workspace behind the base reports "undeclared change to a develop file: src/d.txt" (verified).
- **Avoid:** check fixtures sync first, unless the stale case is the subject.

### Pitfall 7: A guard "fixed" scenario with a dead hook
- If the adapter breaks (bad `WSX_ORACLE_DIR`, wrong cwd), every push looks "allowed", and every bypass scenario records a fake defect.
- **Avoid:** the inline known-positive control (Pattern 3). It is rc 1 on the oracle (verified).

### Pitfall 8: Filter-selected disabled side
- `-E 'test(=x::rust)'` on an ignored test fails with rc 4 (verified). This is correct, but surprising to a plan's verify step. Plans must gate oracle sides, or the meta-tests, by name.

### Pitfall 9: Non-ASCII names on observation
- Observers must use `ls-tree -r -z`, `for-each-ref` (ref names are restricted ASCII-safe anyway) and `std::fs::read_dir` with `OsStrExt::as_bytes`. Render bytes with `escape_ascii()` for plain-words output.

## Code Examples

### Hermetic env builder (sketch)
```rust
// Sketch. Values quoted elsewhere in this document: GIT_CONFIG_NOSYSTEM "1", LC_ALL "C",
// CARGO_TARGET_TMPDIR (tests/setup_repo.rs:15), GIT_ALLOW_PROTOCOL "file", WSX_ORACLE_DIR (proposed).
pub fn hermetic_env(inherited: &[(OsString, OsString)], d: &ScenarioDirs) -> Vec<(OsString, OsString)> {
    let mut env: Vec<(OsString, OsString)> = vec![
        ("HOME".into(), d.home.clone().into()),
        ("GIT_CONFIG_GLOBAL".into(), d.home.join(".gitconfig").into()),
        ("GIT_CONFIG_SYSTEM".into(), d.system_config.clone().into()),
        ("GIT_CONFIG_NOSYSTEM".into(), "1".into()),
        ("XDG_CONFIG_HOME".into(), d.xdg_config.clone().into()),
        ("GIT_CEILING_DIRECTORIES".into(), d.root.clone().into()),
        ("GIT_ALLOW_PROTOCOL".into(), "file".into()),
        ("GIT_TERMINAL_PROMPT".into(), "0".into()),
        ("GIT_EDITOR".into(), "false".into()),
        ("GIT_MERGE_AUTOEDIT".into(), "no".into()),
        ("LC_ALL".into(), "C".into()),
        ("TZ".into(), "UTC".into()),
        ("PATH".into(), "/usr/local/bin:/usr/bin:/bin".into()),
    ];
    env.extend(inherited.iter()
        .filter(|(k, _)| k.as_encoded_bytes().starts_with(b"LLVM_"))
        .cloned());
    env
}
// Spawn: Command::new(p).env_clear().envs(env).stdin(Stdio::null()).current_dir(cwd)
```

### Driving the oracle pre-push the way git does
```rust
// stdin line format from hooks/pre-push:158-159: "<local ref> <local sha> <remote ref> <remote sha>"
let mut child = oracle_cmd("bash").arg(oracle_dir.join("hooks/pre-push")).arg(remote).arg(url)
    .current_dir(push_worktree).env("WSX_ORACLE_DIR", &oracle_dir)
    .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
child.stdin.take().unwrap().write_all(format!("{lref} {lsha} {rref} {rsha}\n").as_bytes())?;
```
A new ref uses the all-zero remote sha `0000000000000000000000000000000000000000`. The hook skips a deletion (all-zero local sha).

### scenarios! macro shape (sketch)
```rust
macro_rules! scenarios {
    ($( $name:ident => $kind:ident $( , rust_disabled = $why:literal )? ; )*) => {
        $( pub mod $name {
               scenarios!(@oracle $kind, $name);
               #[test] $( #[ignore = $why] )?
               fn rust() { crate::runner::run(crate::Side::Rust, super::$name(), super::entry(stringify!($name))) }
           } )*
        pub fn registry() -> Vec<crate::Entry> { vec![ $( crate::Entry::new(module_path!(), stringify!($name),
            stringify!($kind), [$($why)?].first().copied(), $name) ),* ] }
    };
    (@oracle oracle, $name:ident) => { #[test] fn oracle() { /* run Side::Oracle */ } };
    (@oracle rust_only, $name:ident) => {};
}
```
A function `fn x()` and a `mod x` can share a name (value vs type namespace).

## Suggested plan decomposition (question 9)

Fine granularity. Each plan is TDD (RED first). Commits are `type(02-NN): ...`.

| Wave | Plan | Delivers | Key files | Depends on |
|---|---|---|---|---|
| 1 | **02-01 Tracer** | `tempfile` dev-dep (justification in the commit and Cargo.toml comment; `scripts/check.sh deps` clean). Vendor all five scripts plus LICENSE, README and `oracle.patch`. Minimal harness: `env.rs` (full table above, including the `LLVM_*` passthrough), a devflow-shaped `fixture.rs` preset, the oracle adapter for **Sync** only, `outcome.rs` covering exit, refs, commits (message, author, author date, committer, tree) and files, the comparator, plain-words Display, and the minimal `scenarios!`. **One scenario, `sync::merges_integration_into_workspace`**: oracle enabled with explicit expectations (exit 0, refs, the merge commit `chore: sync develop into workspace/dev` with 2 parents, files), Rust side `#[ignore]`d | Cargo.toml, Cargo.lock, tests/oracle/*, tests/harness/{main,env,fixture,oracle,outcome,scenario,sync}.rs | — |
| 2 | **02-02 Oracle provenance and self-tests** (HARN-01) | Blob-match test (control: one flipped byte in a scratch copy is detected). Patch-applies test with recorded post-patch blob hashes (control: the no-ceiling apply leaves the file unchanged). Fail-loud test without `WSX_ORACLE_DIR`. Self-test scenarios for phase-worktree (expects the sync merge, Pitfall 3), cut, check (known-positive and clean) and pre-push (known-positive and clean) | provenance.rs, worktree.rs, cut.rs, check.rs, guard.rs, oracle.rs (adapter mappings) | 02-01 |
| 2 | **02-03 Outcome completeness** (HARN-03) | Raw `cat-file` parse (full message, body and trailers), signature-presence field, byte-path files and tree entries, ref scoping. **Per-field mutation meta-test**: for one observed real outcome, mutate each field once and assert a mismatch names that field (control: the unmutated outcome gives zero mismatches) | outcome.rs | 02-01 |
| 2 | **02-05 Hermeticity proof** (HARN-02) | Planted host plus decoy repo; scenario outcomes unchanged and marker absent; control wired to the planted config fires; `GIT_CONFIG_NOSYSTEM` pre-push trap test; PATH git ≥ 2.54 assertion | hermeticity.rs, env.rs (test-only leaky flag) | 02-01 |
| 2 | **02-07 Coverage tooling** | install-tools.sh `cargo-llvm-cov` pin (install, verify, verify-cargo, usage); a harness spawn-`wsx --version` test; no-profraw invariant; **one-time proof with both negative controls recorded in SUMMARY** (blocked on Open Question 2) | scripts/install-tools.sh, tests/harness/coverage.rs | 02-01 |
| 3 | **02-04 Signing** | Per-run SSH key in the fixture; local signing config for commit-creating scenarios; signed cut-replay and sync-merge scenarios assert `signed: true`; an unsigned control asserts `false` | fixture.rs, cut.rs, sync.rs | 02-03 |
| 3 | **02-06 Runner rules** (HARN-03 classes, HARN-05) | `validate()`: missing expectation, `fixed:` without a recorded differing oracle outcome, `regression:` not a refusal, guard without both controls, macro flag vs `OracleSide` mismatch. Each rejection has a negative-control test asserting its exact variant. Final `scenarios!`/`harness_modules!`; `registry::rust_sides_not_enabled` plus the nextest override; `every_scenario_is_well_formed`. Pre-create empty HARN-06 module stubs, so Wave 4 plans touch disjoint files | scenario.rs, main.rs, .config/nextest.toml | 02-03 |
| 4 | **02-08 Guard cases** | P1 non-ASCII, P1 newline, P1 non-UTF-8, P2, P3, P4, P5 (each `fixed:` with recorded oracle outcome plus inline controls); fork and no-GSD guard (Rust-only) | guard.rs | 02-06 |
| 4 | **02-09 Cut cases** | D1, D2 (merge; conflict resolved by adding a personal file), D3, D4, D5, squash-merged re-cut, fork and no-GSD (Rust-only), signed replay already in 02-04 | cut.rs | 02-06, 02-04 |
| 4 | **02-10 Sync, worktree and check cases** | squash-merged then sync; worktree re-run with own commits (W1, `fixed:`); check with non-ASCII, newline and non-UTF-8 names (record outcome); stale-workspace check; no-GSD and fork (Rust-only) | sync.rs, worktree.rs, check.rs | 02-06 |

The tracer 02-01 is large because vendoring and the minimum harness are inseparable for an end-to-end run. If the planner wants it smaller, move only the README and LICENSE into 02-02; the scripts and patch must stay in 02-01.

## State of the Art

| Old approach | Current approach | Impact |
|---|---|---|
| Python-oracle L4 harness design (ARCHITECTURE.md: `WSX_IMPL=oracle\|rust\|both`, `Divergence` enum) | Per-scenario side selection through generated tests, with the oracle record inside the scenario | No env var selects implementations (and `set_var` is banned); nextest filters by name instead |
| Hooks via `core.hooksPath` | git ≥2.54 config hooks (`hook.<id>.command`/`event`) also fire, and run alongside hooksPath | The planted-host proof plants both (verified both fire) |

## Assumptions Log

| # | Claim | Section | Risk if wrong |
|---|-------|---------|---------------|
| A1 | The CI image's `git` is on `/usr/local/bin:/usr/bin:/bin` and is ≥2.54, and awk, comm and paste exist there (CONTEXT says git 2.56.0 and `/usr/bin/ssh-keygen`; not re-verified this session) | Environment | Oracle scenarios fail only in CI; the PATH assertion test catches it loudly |
| A2 | Cargo auto-discovers `tests/harness/main.rs` as test target `harness` | Structure | Needs an explicit `[[test]]` entry |
| A3 | `git rev-list` default order can place a parent before a child when timestamps tie | Pitfall 5 | Low; the ticking clock is harmless either way |
| A4 | The test profile unwinds on panic, so `TempDir` drops | tempfile | Leftover dirs in `target/tmp` (disk, not tmpfs) |
| A5 | The cargo-llvm-cov release binary prints `cargo-llvm-cov 0.9.1` (the host copy does; the downloaded copy was not executed) | Coverage pin | The verify pattern needs adjusting; caught at the first `verify-cargo` |
| A6 | `rustup component add llvm-tools` (vs `llvm-tools-preview`) is accepted for 1.99.1 | Coverage | Use the name cargo-llvm-cov's error message gives: `llvm-tools-preview` |
| A7 | Re-cut after a squash merge ends "PR branch is empty", exit 1, on the oracle | HARN-06 map | Only the recorded value changes; the scenario records what it observes |
| A8 | nextest `binary(=harness)` filter syntax for per-binary overrides such as `slow-timeout` (if used) | Pitfalls | A config parse error, caught at once |

## Open Questions

1. **Fork layout and no-GSD repo: Rust-only (locked) vs. oracle-recorded where the oracle runs.** Decision for the owner.
   - What we know: CONTEXT locks both as Rust-only, on the grounds that "the scripts hard-code devflow's paths and origin". The scripts do in fact accept a remote: sync `$1`, cut `BASE_REMOTE`, check `BASE_REF`. And `phase-worktree.sh` in a no-GSD repo refuses, which is precisely the defect IF-02 fixes, so it would be a ready negative control.
   - What's unclear: whether the owner wants those extra oracle recordings. Success criterion 5 says "wherever the oracle implements that behaviour".
   - Recommendation: honour the lock in phase 2 (Rust-only, labelled by name). Ask the owner whether to add oracle-recorded variants for "cut with `BASE_REMOTE=upstream`" and "`new` in a no-GSD repo (oracle refuses)".
2. **llvm-tools for the pinned toolchain.** Decision for the owner; it blocks plan 02-07's proof.
   - Option A: add `"llvm-tools"` (or `"llvm-tools-preview"`) to `rust-toolchain.toml` components. Reproducible, but every container toolchain install downloads it, and that file is phase-1 territory.
   - Option B (recommended for phase 2, since there is no CI coverage job): a documented one-time host step, `rustup component add llvm-tools-preview --toolchain 1.99.1-x86_64-unknown-linux-gnu`, run by the owner. The proof then always runs with `CARGO_LLVM_COV_SETUP=no`.
   - Option C: `LLVM_COV`/`LLVM_PROFDATA` pointing at the `stable` toolchain's tools (same LLVM 23.1.1). Fragile.
3. **Rust-side expectations before their contracts exist.**
   - wsx's exit codes are fixed in phase 5 (IF-01), and `refs/wsx/*` records in phases 8-10.
   - Recommendation: Rust expectations state exact exit codes now (best guess; harmless while disabled), and the enabling phase revises them in the same commit that enables the side.
   - Ref-name comparison defaults to `refs/heads`, `refs/remotes` and `refs/tags`; `refs/wsx` is opted in per scenario. The planner can decide this within discretion.
4. **`deny.toml` `[licenses] include-dev = true`?** Today tempfile's licenses are unchecked. Adding it passes with MIT-only (verified). A policy choice; recommend yes, as a one-line change in plan 02-01.
5. **Requirement text drift** (awareness, not blocking):
   - PRCUT-05 claims the oracle "warns" about left-out commits; it does not (D5).
   - REQUIREMENTS line 35 still names `scripts/workspace/{...}` as the oracle; CONTEXT supersedes it with devflow's originals.

## Environment Availability

| Dependency | Required by | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| git | everything | ✓ host `/usr/bin/git` | 2.56.0 | — |
| bash ≥4 | oracle | ✓ `/usr/bin/bash` | 5.3.9 | — |
| ssh-keygen | signing | ✓ `/usr/bin/ssh-keygen` | not probed (`-V` unsupported) | — |
| cargo-nextest | suite | ✓ | 0.9.148 | — |
| cargo-deny | deps gate | ✓ | 0.20.2 | — |
| cargo-llvm-cov | coverage proof | ✓ `~/.cargo/bin` | 0.9.1 | — |
| llvm-tools for 1.99.1 | coverage proof | **✗** (only `stable` has them) | — | Open Question 2 |
| CI image git / awk / ssh-keygen | oracle in CI | not re-verified (CONTEXT says git 2.56.0 and ssh-keygen present) | — | A PATH/version assertion test fails loud |

**Missing with no fallback:** none for phases other than the coverage proof. **Missing with a fallback:** llvm-tools (Open Question 2).

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | libtest `#[test]` run by cargo-nextest 0.9.148 |
| Config file | `.config/nextest.toml` (exists: JUnit only); gains the report-test override |
| Quick run command | `scripts/check.sh test -E 'test(=sync::merges_integration_into_workspace::oracle)'` |
| Full suite command | `scripts/check.sh all` |

### Phase Requirements → Test Map

| Req / criterion | Behavior | Type | Automated command (one per command) | Negative control | File exists? |
|---|---|---|---|---|---|
| HARN-01 / SC1 | Vendored blobs equal recorded devflow blobs | integration | `-E 'test(=provenance::vendored_files_match_devflow_blobs)'` | a scratch copy with one flipped byte is reported | ❌ Wave 0 |
| HARN-01 / SC1 | Patch applies cleanly to a scratch copy and yields the recorded patched blobs | integration | `-E 'test(=provenance::oracle_patch_applies_cleanly)'` | the apply without a ceiling in a nested repo leaves the files unpatched | ❌ |
| HARN-01 / SC1 | Patched scripts fail loud without `WSX_ORACLE_DIR` | integration | `-E 'test(=provenance::patched_scripts_require_oracle_dir)'` | with the var set, the same call passes | ❌ |
| HARN-01 / SC1 | Self-test per script (5) | scenario | `-E 'test(=<module>::self_test::oracle)'` ×5 | the phase-worktree self-test expects the sync merge (catches a skipped sync) | ❌ |
| HARN-01 / SC1 | One definition runs on oracle or Rust | scenario | the `<name>::oracle` run, plus the `<name>::rust` run being skipped | selecting only `::rust` gives rc 4 "no tests to run" | ❌ |
| HARN-02 / SC2 | Planted host: outcomes unchanged, hook never fires | integration | `-E 'test(=hermeticity::planted_host_config_never_reaches_scenarios)'` | `hermeticity::wired_to_planted_config_fires_hook` must see the marker | ❌ |
| HARN-02 / SC2 | pre-push unset cannot reach system config | integration | `-E 'test(=hermeticity::oracle_pre_push_unset_keeps_system_config_out)'` | without `GIT_CONFIG_NOSYSTEM` the planted system value is visible | ❌ |
| HARN-02 / SC2 | Coverage sees the spawned binary | one-time, recorded | `CARGO_LLVM_COV_SETUP=no cargo llvm-cov nextest --no-tests=fail -E 'test(=coverage::spawned_wsx_reports_version)' --text` | (A) a non-spawning test: same line 0; (B) no passthrough: 0, plus stray profraw | ❌ |
| HARN-03 / SC3 | Every expected field is compared | integration | `-E 'test(=outcome::changing_any_expected_field_fails)'` | the unmutated outcome gives zero mismatches | ❌ |
| HARN-03 / SC3 | Signature presence asserted separately | scenario | `-E 'test(=cut::signed_replay::oracle)'` | the unsigned control expects `signed: false` | ❌ |
| HARN-03 / SC3 | Runner rejects "same as oracle only" | integration | `-E 'test(=scenario::rejects_scenario_without_explicit_expectation)'` | a well-formed scenario validates OK | ❌ |
| SC4 | `fixed:` fails if it passes on the oracle | integration | `-E 'test(=scenario::rejects_fixed_matching_expected_on_oracle)'` | D1 scenario passes as `fixed:` | ❌ |
| SC4 / HARN-05 | `regression:` must be a refusal; guard needs both controls | integration | `-E 'test(=scenario::rejects_regression_that_is_not_a_refusal)'`, `-E 'test(=scenario::rejects_guard_without_known_positive)'`, `-E 'test(=scenario::rejects_guard_without_clean_control)'` | valid counterparts accepted | ❌ |
| SC5 / HARN-06 | Each case has a scenario plus control, recorded or Rust-only | scenario | `-E 'test(=guard::quoted_non_ascii_name::oracle)'` (etc., one per case) | inline guard controls; `fixed:` oracle record | ❌ |
| SC5 | Disabled Rust sides reported by name | integration | `-E 'test(=registry::rust_sides_not_enabled)'` (output visible through the override) | `registry::every_scenario_is_well_formed` | ❌ |
| Universal | No `*.profraw`, no stray files in the fixture after each scenario | built into the runner | (every scenario) | only bites under coverage | ❌ |

Test names are proposals; plans fix them.

### Sampling Rate
- **Per task commit:** the plan's named tests, one `scripts/check.sh test -E 'test(=...)'` per command, chained with `&&`.
- **Per wave merge:** `scripts/check.sh all`.
- **Phase gate:** `scripts/check.sh all`, then `scripts/check.sh deps`, then `scripts/check-in-container.sh test` (CI parity for the oracle's tools), all green, before `/gsd-verify-work`.

### Wave 0 Gaps
- [ ] `tests/harness/` binary (does not exist)
- [ ] `tests/oracle/` vendored set (does not exist)
- [ ] `tempfile` dev-dependency
- [ ] `.config/nextest.toml` override for the report test
- [ ] `scripts/install-tools.sh` cargo-llvm-cov pin
- [ ] llvm-tools for 1.99.1 (Open Question 2)

## Security Domain

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes (path bytes) | Raw bytes, `-z`, no quoted-output parsing in observers |
| V6 Cryptography | partly (signing) | `ssh-keygen` ed25519 throwaway key per run; never a real key; no `SSH_AUTH_SOCK` |
| V10 Malicious code / V14 Configuration | yes | `tempfile` legitimacy OK and deny-clean; cargo-llvm-cov pinned by sha256; oracle provenance by blob SHA |

### Known Threat Patterns for this harness

| Pattern | STRIDE | Standard mitigation |
|---------|--------|---------------------|
| Harness git command retargets the real wsx repo (discovery from `target/tmp`, or inherited `GIT_DIR`) | Tampering | `env_clear()`, `GIT_CEILING_DIRECTORIES`; decoy-repo assertion in the hermeticity proof |
| Host global or system hooks or config execute during tests | Elevation / Tampering | `GIT_CONFIG_GLOBAL` at the harness `$HOME/.gitconfig`, `GIT_CONFIG_NOSYSTEM=1`, scratch XDG; planted-host proof |
| A scenario reaches the network (oracle `git fetch origin` against a real remote) | Information disclosure | `GIT_ALLOW_PROTOCOL=file` (verified to block https) |
| Vendored oracle silently altered | Tampering | Blob-SHA test against the recorded devflow commit |
| The patch silently not applied | Tampering (false green) | Ceiling plus recorded post-patch blob hashes |
| Real signing key used in tests | Information disclosure | Per-run key in the TempDir; signing config only in the fixture's local config |

## Sources

### Primary (HIGH confidence)
- devflow `origin/develop` @ `ee6ec792`: `scripts/{sync-workspace,phase-worktree,cut-pr-branch,check-workspace-divergence}.sh`, `scripts/hooks/pre-push`, `LICENSE`. Read in full via `git show`; blob SHAs via `git ls-tree` and `git hash-object`.
- Executed probes this session (scratch, hermetic): oracle defects D1-D5, P1-P5 and W1; the patched self-tests; the config-source probe A1-A4; the `git apply` trap B1-B3; SSH signing; the planted-hook proof and control; `GIT_ALLOW_PROTOCOL`; nextest ignore, skip and override behaviour (0.9.148); cargo-llvm-cov passthrough with negative controls (0.9.1); cargo-deny with tempfile (0.20.2), including the `include-dev` control.
- wsx repo files: `tests/setup_repo.rs`, `tests/gate.rs`, `tests/gate_controls.rs`, `tests/cli.rs`, `src/main.rs`, `Cargo.toml`, `deny.toml`, `clippy.toml`, `rust-toolchain.toml`, `.config/nextest.toml`, `scripts/install-tools.sh`, `scripts/check.sh`, `.devcontainer/Dockerfile`, `scripts/check-in-container.sh`; `.planning` CONTEXT, REQUIREMENTS, ROADMAP, STATE and research docs.
- `cargo info tempfile`; `gh api repos/taiki-e/cargo-llvm-cov/releases/{tags/v0.9.1,latest}`; a local `sha256sum` of the musl tarball.

### Secondary (MEDIUM confidence)
- Context7 `/taiki-e/cargo-llvm-cov` README: env vars, `CARGO_LLVM_COV_SETUP`, `LLVM_COV`/`LLVM_PROFDATA`.
- `https://raw.githubusercontent.com/taiki-e/cargo-llvm-cov/main/src/context.rs`: the setup prompt logic (read through WebFetch's summarising model, so verbatim quotes are only as faithful as that model).
- Context7 `/websites/rs_tempfile_3_27_0_tempfile`: `TempDir::new_in`, `with_prefix_in`.

### Tertiary (LOW confidence)
- git-rev-list ordering semantics with tied timestamps (A3); Cargo `tests/*/main.rs` auto-discovery (A2). From memory, not re-read.

## Metadata

**Confidence breakdown:**
- Oracle inventory, patch and defects: HIGH. Every script was read in full; patch built and applied; 12 behaviours executed with controls.
- Hermeticity and signing: HIGH. Executed with negative controls on git 2.56.0.
- Coverage: HIGH for the mechanism (probe with two negative controls); the repo-specific blocker (llvm-tools) is verified, and its resolution is an owner decision.
- Registry and runner design: MEDIUM. nextest behaviours verified; the macro is a sketch, not compiled.
- CI-image parity: LOW. Not re-verified this session.

**Not established by this research:**
- The oracle's behaviour on cases not executed (A7; the hard-coded-origin over-scan; the divergence check on quoted names).
- That the macro sketch compiles under clippy `-D warnings`.
- That the oracle's tools behave identically in the trixie CI image (mawk vs gawk was not exercised).

**Process note:** the GSD research-plan, cache and graphify seams were not used. Evidence came from direct reads and executed probes. Nothing was committed: the orchestrator said to modify no file except this one.

**Research date:** 2026-10-26
**Valid until:** 2026-11-25 (stable tools pinned); re-check if the devflow pin moves off `ee6ec792`.
