# Phase 1: Development foundation - Context

**Gathered:** 2026-10-22
**Status:** Ready for planning

Smart discuss ran in autonomous mode. Every decision below follows from REQUIREMENTS.md, PROJECT.md
or a host check made on 2026-10-22; the orchestrator resolved them from that evidence (owner's
standing instruction: answer what the evidence settles, escalate only real choices). None needed
an owner choice.

<domain>
## Phase Boundary

The Rust workspace builds, tests and gates the same way on the host, in the pinned container and in
CI, and every supply-chain scanner is proven to bite (DEV-01..06, SUP-01..05, HARN-04, REL-04).
No wsx functionality is built here: the crate only needs enough code for the gates to run against
(a binary with at least one real test, so `--no-tests=fail` has something to pass on).

</domain>

<decisions>
## Implementation Decisions

### Toolchain and gates
- Pin rustc **1.99.1**, not the 1.99.0 written in DEV-02. DEV-02's rule is "the host's version";
  the host is on 1.99.1 (rustup, 2026-10-13) as of 2026-10-22. DEV-02 and roadmap criterion 1
  were corrected to match.
- One package at the repo root, binary crate `wsx`, no library crate (PROJECT.md Key Decision,
  2026-10-20). Edition 2024.
- `scripts/check.sh {all,fmt,clippy,test,deps}` is ported from devflow's `scripts/check.sh` and
  is the only gate entry point. A missing tool exits non-zero and names the tool; nothing is
  skipped.
- `test` runs `cargo nextest run --no-tests=fail`. `clippy` runs
  `cargo clippy --workspace --all-targets -- -D warnings`. `clippy.toml` disallows
  `std::env::set_var` and `std::env::remove_var` (HARN-04).
- The Python 0.1.0 (`bin/wsx`, `tests/sandbox.sh`) stays untouched; it is deleted in phase 15
  (REL-02). The gates cover the Rust code only.

### Container and CI
- `.devcontainer/devcontainer.json` pins an exact `mcr.microsoft.com/devcontainers/rust` tag
  (verify the current tag against the registry; never a floating tag), same family as devflow.
  `rust-toolchain.toml` supplies the exact rustc inside it.
- Port devflow's `scripts/check-in-container.sh` (worktree-aware: bind-mounts the common git dir,
  per-checkout cargo volumes) and `scripts/assert-image-parity.sh`. Host runtime: rootless Docker
  29.9.1 (verified). devflow's CPU-pinned sequential job and its `Build + test in devcontainer`
  job are not ported (DEV-06, REL-04 list only test, clippy, fmt).
- `.github/workflows/ci.yml`: jobs named `Test`, `Clippy`, `Format` (devflow's names, which the
  rulesets will reference), each in the pinned image, on push and on PRs to `develop` and `main`.
  The parity check runs in `Test`. Every action pinned to a commit SHA.
- `cargo deny` runs in its own advisory workflow (push, PR, weekly schedule), never a required
  check (SUP-01).

### Supply chain
- One install script pins scanner and tool versions at the host's verified versions: cargo-deny
  0.20.2, gitleaks 8.30.1, cargo-nextest 0.9.148. A missing scanner is a hard failure (SUP-04).
- `deny.toml` covers advisories, licenses, bans, sources. `Cargo.lock` committed.
- Secret scan: a tracked setup script registers a git config-based pre-commit hook
  (`hook.<name>.command` / `hook.<name>.event`, git 2.56.0 on the host) running
  `gitleaks git --pre-commit --staged --redact`, alongside global hooks (SUP-03).
- `.github/dependabot.yml`: `github-actions` and `devcontainers` ecosystems, weekly, grouped,
  `target-branch: develop`; no per-crate updates (SUP-02).
- Negative controls (SUP-05): a planted fake secret must be refused by the hook, and a
  known-vulnerable or disallowed crate in a scratch copy must fail `cargo deny check`. Both run in
  scratch space, are removed afterwards, and their failing output is recorded as evidence. The same
  rule applies to criteria 1 and 2: a tool removed from `PATH`, a no-match nextest filter and a
  planted `set_var` must each fail with output quoted in the verification.

### Docs and repo config
- Root `CLAUDE.md` (currently an interim placeholder, commit 4bc4411) and `CONTRIBUTING.md` are
  rewritten for Rust: devflow's repo traps (DEV-03) and its AI change acceptance bar with the four
  rejected test shapes (DEV-04), taken from devflow's own CONTRIBUTING.md. Both public-safe.
- Tracked `.gitconfig` (included with `git config --local include.path ../.gitconfig`): SSH
  signing for commits and tags, git-flow branch names. No `user.*`, no signing key, nothing
  personal (DEV-05).

### Outward-facing actions (owner gate)
- Executors do not push, open PRs, or change GitHub settings or rulesets. Criterion 3's CI run on
  a PR to `develop` and the ruleset update (REL-04) are prepared, then left for the owner's
  approval at the end of the phase. The owner asked to approve GitHub-side changes before they
  happen (2026-10-21). Verification marks only those two items as pending the owner.

### Decisions after research (2026-10-22)
- **Image pin lives in `.devcontainer/Dockerfile`** (`FROM mcr.microsoft.com/devcontainers/rust:<exact tag>`),
  referenced from `devcontainer.json` via `build.dockerfile`, with a Dependabot `docker` ecosystem
  entry. Research confirmed from dependabot-core that the `devcontainers` ecosystem updates only
  Features and never `image`, so pinning in `devcontainer.json` would make SUP-02's image updates a
  silent no-op. The parity check reads the Dockerfile and the workflow files, so a Dependabot bump
  fails CI until the workflows match (fail closed, not silent drift). DEV-06's wording was updated to
  match; its intent (one exact pinned tag, CI in the same image) is unchanged.
- The hook setup script is run against this repo's real config once the scratch controls pass
  (SUP-03 requires the repo to scan its own commits).
- Controls must use the discriminating forms research found: `NEXTEST_NO_TESTS=pass` set and
  `check.sh test` still failing on a no-match filter; an explicit minimal `PATH` (not a filtered
  one, because mise shims also provide the tools); `GIT_CONFIG_GLOBAL=/dev/null` for the DEV-05
  signing check.
- No contact details in any request header (research hygiene note).

### Claude's Discretion
- File layout of scripts beyond the names fixed above, the minimal placeholder `main.rs` and its
  test, nextest config location, exact `deny.toml` license list (must accept the crates actually
  used).

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- devflow (`~/Github/devflow`, read-only reference): `scripts/check.sh` (105 lines),
  `scripts/check-in-container.sh` (154), `scripts/assert-image-parity.sh` (43),
  `scripts/install-dep-tools.sh`, `.devcontainer/devcontainer.json`, `.github/workflows/ci.yml`
  (header comment explains the ruleset/required-check trap), `deny.toml`, CONTRIBUTING.md.
- `scripts/workspace/` (personal, `only` in `.wsx.conf`): the shell oracle; not touched here.

### Established Patterns
- Repo is Python 0.1.0 today: `bin/wsx`, `tests/sandbox.sh`, README.md, CONTRIBUTING.md (Python).
- Git ≥ 2.54 config-based hooks are the hook model for the whole project (PROJECT.md).

### Integration Points
- `scripts/check.sh` is what later phases' CI, hooks and humans call.
- Phase 2 (oracle harness) builds on nextest + the `set_var` ban.

</code_context>

<specifics>
## Specific Ideas

- Copy devflow's choices where they hold; do not inherit its CPU-pinned job or the devcontainer
  build job (improve-over-prior-art mandate: keep only what the requirements ask for).

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope.

</deferred>
