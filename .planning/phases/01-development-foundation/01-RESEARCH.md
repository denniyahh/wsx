# Phase 1: Development foundation - Research

**Researched:** 2026-10-22
**Domain:** Rust workspace gates, container/CI parity, supply-chain scanning, repo git policy
**Confidence:** HIGH for everything executed locally (toolchain, gates, controls, hooks, container); MEDIUM for GitHub-side behaviour that can only be proven by the owner-gated first CI run and first Dependabot run

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

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

### Claude's Discretion
- File layout of scripts beyond the names fixed above, the minimal placeholder `main.rs` and its
  test, nextest config location, exact `deny.toml` license list (must accept the crates actually
  used).

### Deferred Ideas (OUT OF SCOPE)
None — discussion stayed within phase scope.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DEV-01 | `scripts/check.sh {all,fmt,clippy,test,deps}` single gate entry point; missing tool is a hard failure | Pattern 1; missing-tool control recipe C1 (PATH must drop the mise shims too, Pitfall 3) |
| DEV-02 | `rust-toolchain.toml` pins exact rustc 1.99.1 with clippy and rustfmt | Verified: rustup 1.29.1 in the image auto-installs 1.99.1 + components from the file (17 s, needs network) |
| DEV-03 | Root `CLAUDE.md` for Rust with devflow's repo traps | Trap list quoted from devflow CLAUDE.md:18-26; adapt, drop devflow-only traps |
| DEV-04 | `CONTRIBUTING.md` for Rust with the acceptance bar and four rejected test shapes | Quoted verbatim from devflow CONTRIBUTING.md:298-325; drop its `.claude/skills` reference |
| DEV-05 | Tracked `.gitconfig` via `include.path`: SSH signing, git-flow names, nothing personal | Verified in scratch: signs commits and annotated tags; include from a linked worktree resolves to the MAIN checkout's file (Pitfall 5); global config already signs (Pitfall 6) |
| DEV-06 | Container parity: exact image tag, worktree-aware `check-in-container.sh`, CI in same image, parity check | Image `2.1.3-1-trixie` verified (git 2.56.0, rustup 1.29.1, rustc 1.99.0 default); common-dir mount verified with negative control; nextest is NOT in the image (Pitfall 2) |
| SUP-01 | `cargo deny check` on push, PR, weekly; advisory only | deny.yml pattern; weekly cron runs only from `main` |
| SUP-02 | Dependabot: Actions + devcontainer image weekly, grouped, `target-branch: develop` | **The `devcontainers` ecosystem cannot update the image** (source-verified). Needs owner decision: see Open Question 1 |
| SUP-03 | Tracked setup script registers config-based gitleaks pre-commit hook alongside global hooks | Verified on git 2.56.0: config hook runs before the `core.hooksPath` hook, both run, refusal blocks the commit, works in linked worktrees, missing gitleaks refuses |
| SUP-04 | `Cargo.lock` committed, actions pinned to SHAs, scanner versions in one install script, missing scanner hard failure | `actions/checkout` v7.0.1 = `3d3c42e5aac5ba805825da76410c181273ba90b1`; release tarball sha256s verified three ways |
| SUP-05 | Each scanner proven to bite; controls cleaned up | Recipes C4 (gitleaks, runtime-generated token) and C5 (`smallvec = "=1.6.0"`, RUSTSEC-2021-0003) both executed here |
| HARN-04 | nextest `--no-tests=fail`; `clippy.toml` disallows `set_var`/`remove_var` | nextest's default already fails on no tests, so the naive control cannot prove the flag; discriminating control C2 verified; planted `set_var` needs `unsafe {}` (Pitfall 8) |
| REL-04 | CI runs test/clippy/fmt on PRs to develop/main; ruleset required checks after first run | Ruleset IDs and current rules read via `gh api`; prepared PUT command with `integration_id: 15368` (GitHub Actions app, verified) |
</phase_requirements>

## Summary

Everything this phase needs exists on the host at the pinned versions, and every gate and scanner control was executed in scratch space during this research, with a discriminating negative control each time. The devflow prior art ports cleanly, with four changes the requirements or the evidence force: `test` moves from `cargo test` to `cargo nextest run`, the CPU pin and the devcontainer build job are dropped, nextest has to be installed into the container (the devcontainer image does not ship it), and actions get SHA pins (devflow uses floating `@v7`).

Two findings change the plan:
1. **Dependabot cannot update the devcontainer image as CONTEXT.md specifies.** dependabot-core's `devcontainers` parser reads only Features, and its `docker` parser reads Dockerfiles and Kubernetes-shaped YAML. It reads neither `devcontainer.json`'s `image` nor a workflow's `container.image`. SUP-02's "updates the devcontainer image" is only reachable by moving the pin into a `.devcontainer/Dockerfile` `FROM` line. This is an owner decision (Open Question 1).
2. **Two of the roadmap's controls do not discriminate as written.** nextest 0.9.148 already fails a no-match filter by default, and on this host `cargo-deny` and `gitleaks` are also on `PATH` through mise shims. The naive versions of criterion 1's and 2's controls would pass whether or not the gate is correct. Discriminating recipes are given below and were run.

Devflow's `assert-image-parity.sh` design has a latent defect that should not be ported. The CI step passes the image as a literal argument, a second copy next to `container.image` in the same job (`ci.yml:45` and `:62`), so bumping one without the other passes the check while CI runs a different image. Read the workflow files instead.

**Primary recommendation:** Port devflow's scripts with the changes above. Pin `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`. Install nextest and cargo-deny from checksum-pinned GitHub release tarballs through one `scripts/install-tools.sh`. Prove every gate with the discriminating controls C1 to C6, run in scratch copies.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Gate definition (fmt/clippy/test/deps) | Repo scripts (`scripts/check.sh`) | — | Single entry point called by humans, container and CI (DEV-01) |
| Toolchain pin | `rust-toolchain.toml` (read by rustup) | Container image (rustup 1.29.1 auto-install) | Image default is 1.99.0; the file overrides it |
| Tool/scanner versions | `scripts/install-tools.sh` | `check.sh` presence checks | One pin site (SUP-04) |
| Host/container parity | `scripts/check-in-container.sh` + Docker | `.devcontainer/` pin | Verification runs in the pinned image |
| CI execution | GitHub Actions container jobs | `check.sh` | CI calls the same script |
| Image-tag drift detection | `scripts/assert-image-parity.sh` | CI `Test` job | Reads the files that define the pins |
| Merge gating | GitHub rulesets (owner-gated) | CI job names | Context names must equal job `name:` |
| Secret scan at commit | Repo `.git/config` hook entry (written by setup script) | Global hook, server push protection | Config hooks coexist with `core.hooksPath` |
| Advisory/license scan | `cargo deny` via `check.sh deps` in `deny.yml` | Dependabot alerts (already on) | Advisory, never required |
| Dependency update PRs | Dependabot (config on `main` only) | Parity check catches half-done image bumps | Version updates target `develop` |
| Repo git policy | Tracked `.gitconfig` via `include.path` | Contributor's local `user.signingkey` | Public repo: nothing personal tracked |

## Standard Stack

### Core
| Tool | Version | Purpose | Why Standard |
|------|---------|---------|--------------|
| rustc / cargo (rustup) | 1.99.1 (host, `rustc 1.99.1 (58cf8f9c3 2026-10-13)`) | Compiler | Locked decision. [VERIFIED: `rustc --version` on host] |
| clippy, rustfmt | toolchain components of 1.99.1 (`clippy 0.1.99`, `rustfmt 1.10.0-stable`) | Lint/format gates | [VERIFIED: in-container auto-install output] |
| cargo-nextest | 0.9.148 (released 2026-10-08) | Test runner; `--no-tests=fail` | Process-per-test; supported by llvm-cov and mutants. [VERIFIED: host `cargo nextest --version`; gh release API] |
| cargo-deny | 0.20.2 (released 2026-07-09) | advisories, licenses, bans, sources | Locked. [VERIFIED: host; gh release API] |
| gitleaks | 8.30.1 (released 2026-03-21) | Staged-diff secret scan | Locked. [VERIFIED: host `gitleaks version`; gh release API] |
| Docker (rootless) | 29.9.1 | `check-in-container.sh` | [VERIFIED: `docker info`] |
| git | 2.56.0 host, 2.56.0 in image | Config-based hooks (≥ 2.54) | [VERIFIED: both] |
| Devcontainer image | `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`, digest `sha256:269f7cd057eaccb52c17e9461f42ac5cc1bff564b0723be49ea91726f5753bb7` | Pinned CI and local verification env | Newest exact tag in MCR's `tags/list` today (2.1.3). Debian 13.7, git 2.56.0 at `/usr/local/bin/git`, rustup 1.29.1, default rustc 1.99.0, `/bin/sh` is dash, runs as root, has curl/sha256sum/tar/jq/python3, no shellcheck. [VERIFIED: registry tags list, manifest HEAD, `docker run` probes] |
| actions/checkout | v7.0.1 = `3d3c42e5aac5ba805825da76410c181273ba90b1` (lightweight tag → commit; `v7` → same commit) | Only third-party action needed | [VERIFIED: `gh api repos/actions/checkout/git/ref/tags/v7.0.1`] |

**Image choice (decided within discretion):** trixie over bookworm. Both 2.1.3 variants exist, but the trixie image's git (2.56.0) equals the host's. Phase 2+ harness tests need git ≥ 2.54 for config-based hooks; devflow's cached `2.0.13-1-bookworm` has git 2.55.0. `2.1.3-trixie`, `2-1-trixie`, `2.1-1-trixie` and `1-trixie` currently resolve to the same digest; only the exact `2.1.3-1-trixie` tag is acceptable (never floating).

### Release artefacts for `scripts/install-tools.sh` (x86_64 Linux)
Each sha256 below was computed locally with `sha256sum` and matched both GitHub's asset `digest` field and the project's own sidecar checksum. [VERIFIED: three-way match, 2026-10-22]

| Tool | URL | sha256 | Archive layout |
|------|-----|--------|----------------|
| cargo-nextest 0.9.148 | `https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-0.9.148/cargo-nextest-0.9.148-x86_64-unknown-linux-gnu.tar.gz` | `223ba936714cc861a9cdedad5a8eb664dfdd69d6fbdf6cea6aa8184f523290dc` | `cargo-nextest` at archive root |
| cargo-deny 0.20.2 | `https://github.com/EmbarkStudios/cargo-deny/releases/download/0.20.2/cargo-deny-0.20.2-x86_64-unknown-linux-musl.tar.gz` | `9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f` | `cargo-deny-0.20.2-x86_64-unknown-linux-musl/cargo-deny` |
| gitleaks 8.30.1 | `https://github.com/gitleaks/gitleaks/releases/download/v8.30.1/gitleaks_8.30.1_linux_x64.tar.gz` | `551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb` | `gitleaks` at archive root |

Both the nextest (glibc) and cargo-deny (musl) binaries ran inside the trixie image (`cargo-nextest 0.9.148 (cd1d6d546 2026-10-07)`, `cargo-deny 0.20.2`). Version output formats, for a `verify` mode: `cargo nextest --version` → first line `cargo-nextest 0.9.148[ (hash date)]` (the brew build omits the parenthetical); `cargo deny --version` → `cargo-deny 0.20.2`; `gitleaks version` → `8.30.1`. [VERIFIED]

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Release tarball + pinned sha256 | `cargo install --locked` (devflow's way) | Minutes of compile per CI job and container run; no artefact checksum |
| Release tarball | `taiki-e/install-action` | CI-only (no help for `check-in-container.sh`); one more action SHA to pin; versions live outside the install script |
| `cargo deny` via install script | `EmbarkStudios/cargo-deny-action` | Its cargo-deny version is tied to the action version, not the install script (breaks SUP-04's single pin site) |
| trixie image | `2.1.3-1-bookworm` | Same family as devflow; git version not probed |

**Installation:** none from crates.io. Phase 1 adds **zero crate dependencies**. Tools come from the release tarballs above, through `scripts/install-tools.sh`.

## Package Legitimacy Audit

Phase 1 installs no crate dependencies. The tools below are fetched as release binaries, and `smallvec` is used only inside a deleted scratch copy as a negative control.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| cargo-nextest | crates.io (binary from GitHub release) | since 2022-02 | 13.4M total, 1.84M recent | github.com/nextest-rs/nextest | OK | Approved (sha256-pinned tarball) |
| cargo-deny | crates.io (binary from GitHub release) | since 2019-05 | 6.66M total, 2.02M recent | github.com/EmbarkStudios/cargo-deny | OK | Approved (sha256-pinned tarball) |
| smallvec =1.6.0 | crates.io | since 2015-04 | 1.34B total | github.com/servo/rust-smallvec | OK | Negative control only, in a deleted scratch copy; never added to wsx |
| gitleaks | GitHub release (Go; not a crate) | — | — | github.com/gitleaks/gitleaks | n/a (seam has no Go ecosystem) | Approved (sha256-pinned tarball) |
| actions/checkout | GitHub Actions | — | — | github.com/actions/checkout | n/a | Approved, pinned to commit SHA |

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none
*Seam: `gsd-tools query package-legitimacy check --ecosystem crates cargo-nextest cargo-deny smallvec` → all `OK`. All three are named in project research (STACK.md) and their official repos.*

## Architecture Patterns

### System Architecture Diagram

```
 developer / agent                     GitHub (owner-gated in this phase)
 ─────────────────                     ─────────────────────────────────
 git commit ──► .git/config hook ──► gitleaks git --pre-commit --staged --redact
     │          (setup script)          │ leak → commit refused
     │          then global hooksPath ──┘ (global gitleaks; independent)
     ▼
 scripts/check.sh <target> [args]      push / PR to develop|main
   ├─ require_tool (names missing tool, exit≠0)         │
   ├─ fmt    → cargo fmt --check                         ▼
   ├─ clippy → cargo clippy --workspace --all-targets   ci.yml (container: pinned image)
   │           -- -D warnings  (clippy.toml bans set_var) ├─ Test:  install-tools nextest
   ├─ test   → cargo nextest run --no-tests=fail "$@"     │         assert-image-parity.sh
   └─ deps   → cargo deny --locked check                  │         check.sh test
        ▲                                                 ├─ Clippy: check.sh clippy
        │ same script, pinned image                       └─ Format: check.sh fmt
 scripts/check-in-container.sh                          deny.yml (push, PR, weekly cron on main)
   ├─ image ← .devcontainer pin                           └─ install-tools cargo-deny; check.sh deps
   ├─ mounts: worktree→/workspace (ro), common git dir (ro), per-checkout volumes
   ├─ install-tools.sh (nextest[, cargo-deny]) inside container
   └─ exec scripts/check.sh <target>                    rulesets (REL-04, after first run):
                                                          required: Test, Clippy, Format (strict)
 rust-toolchain.toml 1.99.1 ──► rustup auto-installs in image (1.99.0 default)
 Dependabot (reads .github/dependabot.yml from main only) ──► PRs → develop
```

### Recommended Project Structure
```
Cargo.toml               # package wsx, edition 2024, license "MIT", publish = false
Cargo.lock               # committed (SUP-04)
rust-toolchain.toml      # channel "1.99.1", components clippy+rustfmt, profile minimal
clippy.toml              # disallowed-methods set_var/remove_var
deny.toml                # advisories, licenses, bans, sources
src/main.rs              # placeholder binary with one real test
.gitconfig               # tracked repo policy (DEV-05)
.gitignore               # add "/target/" ABOVE "# Python" (Pitfall 9)
.devcontainer/devcontainer.json   (+ Dockerfile if Open Question 1 → option A)
.github/workflows/ci.yml          # Test, Clippy, Format
.github/workflows/deny.yml        # advisory cargo deny
.github/dependabot.yml
scripts/check.sh
scripts/check-in-container.sh
scripts/assert-image-parity.sh
scripts/install-tools.sh          # the one pin site for nextest/cargo-deny/gitleaks
scripts/setup-repo.sh             # include.path + config-based gitleaks hook (name is discretion)
CLAUDE.md, CONTRIBUTING.md        # rewritten
```
New scripts must NOT go under `scripts/workspace/`, which `.workspace-divergence` declares `only` and so `cut-pr-branch.sh` strips. [VERIFIED: `.workspace-divergence` (read via cat), `cut-pr-branch.sh:121-126`]

### Pattern 1: `scripts/check.sh`, ported with explicit tool checks and nextest
**What:** devflow's structure (`set -euo pipefail`, a `run_*` per target, `all` = fmt → clippy → test, deps excluded from `all`), plus a `require_tool` before each target and argument pass-through for `test` only.
**Why deps stays out of `all`:** `cargo deny` reads the live RustSec database, so a new advisory would turn every local `all` red on unrelated work. SUP-01 makes it advisory. This is devflow's reasoning, `scripts/check.sh:68-72`.
```bash
#!/usr/bin/env bash
# Source: adapted from devflow scripts/check.sh (read 2026-10-22)
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/.."   # repo root without needing git (devflow's parity script fell back to pwd silently)

require_tool() {   # $1 = binary, $2 = how to get it
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "error: '$1' is required for 'check.sh $TARGET' but is not on PATH; $2" >&2
        exit 1
    fi
}

TARGET="${1:-all}"; shift || true

run_fmt()    { require_tool cargo "install rustup"; require_tool cargo-fmt "rustup component add rustfmt"
               echo "==> cargo fmt --check"; cargo fmt --check; }
run_clippy() { require_tool cargo "install rustup"; require_tool cargo-clippy "rustup component add clippy"
               echo "==> cargo clippy --locked --workspace --all-targets -- -D warnings"
               cargo clippy --locked --workspace --all-targets -- -D warnings; }
run_test()   { require_tool cargo "install rustup"; require_tool cargo-nextest "run scripts/install-tools.sh install cargo-nextest"
               echo "==> cargo nextest run --locked --workspace --no-fail-fast --no-tests=fail $*"
               cargo nextest run --locked --workspace --no-fail-fast --no-tests=fail "$@"; }
run_deps()   { require_tool cargo-deny "run scripts/install-tools.sh install cargo-deny"
               echo "==> cargo deny --locked check"; cargo deny --locked check; }
# case: all) run_fmt; run_clippy; run_test ;;  test) run_test "$@" ;;  others reject extra args
```
- `--no-fail-fast`: nextest defaults to fail-fast. devflow learned that one run should report everything broken (`check.sh:44-48`). [VERIFIED flag exists: `cargo nextest run --help`]
- `--locked` makes CI fail if `Cargo.lock` is stale or missing (SUP-04). Accepted by nextest (`--locked`) and cargo-deny (top-level `cargo deny --locked check`). [VERIFIED: `--help` of both]
- `--no-tests=fail` must come before `"$@"` and nothing may override it. Use control C2, not the naive control.

### Pattern 2: `scripts/install-tools.sh`, the single pin site
**What:** variables `CARGO_NEXTEST_VERSION`/`_SHA256`, `CARGO_DENY_VERSION`/`_SHA256`, `GITLEAKS_VERSION`/`_SHA256` (table above). Two modes:
- `install <tool>...`: curl to a `mktemp -d`, `sha256sum -c`, extract one binary into `${WSX_TOOLS_DIR:-${CARGO_HOME:-$HOME/.cargo}/bin}`, `trap` cleanup. Refuse non-x86_64-Linux with a message.
- `verify <tool>...`: the tool is on PATH and reports the pinned version; otherwise exit non-zero naming tool and version.

CI and the container use `install`. On the host the brew-installed copies already match the pins: `cargo-nextest 0.9.148`, `cargo-deny 0.20.2`, `gitleaks 8.30.1`, all in `/home/linuxbrew/.linuxbrew/bin`. [VERIFIED]
**Recommended:** have `check.sh deps` call `scripts/install-tools.sh verify cargo-deny` before running. Brew upgrades silently, so without this the scanner pin is unenforced on the host.

### Pattern 3: `scripts/check-in-container.sh`, ported (CPU pin removed)
Keep from devflow:
- image read from the `.devcontainer` pin
- `docker info` reachability check with its explanatory error
- per-checkout volume suffix `sha256(REPO_ROOT)[:12]` (devflow HYG-02: a shared volume gave a false RED and a false GREEN)
- gitdir and common-dir mounts with path dedup
- `exec docker run`

Change:
- Remove `scripts/lib/ci-cpus.sh`, `cpu_pin_prefix` and `CPU_PIN` (DEV-06).
- Mount the worktree and git dirs **read-only** (`:ro`). Verified here: `git rev-parse --show-toplevel` works through `:ro` mounts. fmt, clippy and nextest passed with `/workspace:ro` because `CARGO_TARGET_DIR=/ctarget` and `--locked` mean nothing writes to the checkout. Verification must not be able to mutate the source.
- Install tools inside the container before `check.sh` (Pitfall 2), e.g. `docker run ... "$IMAGE" bash -c 'scripts/install-tools.sh install cargo-nextest cargo-deny >/dev/null && exec scripts/check.sh "$@"' _ "$TARGET" "$@"`. This takes about 2 s from GitHub releases. Install into the container's `/usr/local/cargo/bin` (root-writable, already on PATH).
- Pass `-t` only when stdout is a TTY: `[ -t 1 ] && TTY=(-t)`. Without a TTY, `-t` works and propagates the exit code, but output gets `\r\n` line endings (verified), which pollutes agent and CI logs.
- Optional: a per-checkout volume at `/usr/local/rustup` (name it with the image-tag hash too). Without it, every run re-downloads 1.99.1 (17 s, network). Docker seeds an empty named volume from the image on first mount (verified: the volume listed the image's `1.99.0` toolchain). Including the image hash prevents a stale toolchain set surviving an image bump.

Worktree mount detail, verified on this phase worktree:
- gitdir `/var/home/denniyahh/Github/wsx/.git/worktrees/phase-1`
- common dir `/var/home/denniyahh/Github/wsx/.git`, which contains the gitdir

Mounting the common dir at the same path makes git work. Without it, git fails: `fatal: gitfile does not point to a valid repository: /workspace/.git` (exit 128). Rootless Docker maps the host uid to container root, so files show `0:0` and no `safe.directory` is needed locally. [VERIFIED]

### Pattern 4: `scripts/assert-image-parity.sh`, improved to read files, not an argument
devflow passes the image as an argument, a duplicate of `container.image` in the same job. Quoted verbatim from devflow `.github/workflows/ci.yml`:
- line 45: `image: mcr.microsoft.com/devcontainers/rust:2.0.13-1-bookworm`
- line 62: `run: scripts/assert-image-parity.sh "mcr.microsoft.com/devcontainers/rust:2.0.13-1-bookworm"`

Bump line 45 alone and the job runs a new image while the check still passes. [VERIFIED: devflow ci.yml:40-64]

Recommended design:
- Take no arguments. Resolve the root from the script's own directory, not `git rev-parse ... || pwd` (devflow `assert-image-parity.sh:13`).
- Read the pin from the devcontainer source (devflow's comment-stripping `sed` for JSON `"image"`, or the `FROM` line if option A).
- Collect every `image:` value under `.github/workflows/*.yml`.
- Fail if zero workflow images are found (fail closed), or if any differs from the pin.
- Run it in the `Test` job, and it is cheap enough to run locally at any time.

This also covers `deny.yml`'s container image, which devflow's per-job argument never checked.

### Pattern 5: CI workflow (`ci.yml`)
```yaml
# Source: adapted from devflow .github/workflows/ci.yml; SHA verified 2026-10-22
name: CI
on:
  push:
  pull_request:
    branches: [main, develop]
permissions:
  contents: read
concurrency:
  group: ${{ github.workflow }}-${{ github.ref }}
  cancel-in-progress: true
defaults:
  run:
    shell: bash          # container jobs default to `sh -e` = dash (devflow ci.yml:100-107)
jobs:
  test:
    name: Test           # ruleset context name; renaming wedges merges
    runs-on: ubuntu-24.04
    container:
      image: mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie
    timeout-minutes: 30
    steps:
      - uses: actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 # v7.0.1
        with:
          persist-credentials: false
      - name: Trust the workspace
        run: git config --global --add safe.directory "$GITHUB_WORKSPACE"
      - name: Assert CI image matches the devcontainer pin
        run: scripts/assert-image-parity.sh
      - name: Install pinned tools
        run: scripts/install-tools.sh install cargo-nextest
      - run: scripts/check.sh test
  # clippy (name: Clippy) and fmt (name: Format): same container/checkout/trust steps, then check.sh clippy|fmt
```
- `deny.yml`: `on: push`, `pull_request: branches: [main, develop]`, `schedule: - cron: '<weekly>'`. One job in the same container: install `cargo-deny`, then `scripts/check.sh deps`. Same `permissions`, `defaults` and checkout pin. **No `continue-on-error`**: advisory means "not in the ruleset", not "green on failure". A job that is green on failure hides findings.
- Job names `Test`, `Clippy`, `Format` become the ruleset contexts. GitHub Actions check runs report the bare job name. devflow's ruleset contexts are `{"context":"Test"},{"context":"Clippy"},{"context":"Format"},...` and its check runs are named `Test`/`Clippy`/`Format`, from app `github-actions` id `15368`. [VERIFIED: `gh api repos/denniyahh/devflow/rulesets/19616771`, check-runs API]
- The safe.directory step: CI runs as root over a checkout owned by uid 1001, and git refuses with "dubious ownership" (devflow `ci.yml:49-60`). Keep it even though Phase 1's `check.sh` does not need git: later harness phases will.
- No path filters on `ci.yml`. A required check whose workflow is filtered out never reports and wedges merges.

### Pattern 6: Config-based gitleaks hook (setup script)
Verified on git 2.56.0 in an isolated scratch repo (`GIT_CONFIG_GLOBAL`, `GIT_CONFIG_NOSYSTEM=1`) with `core.hooksPath` set:
- `git hook list pre-commit` printed the config hook, then `hook from hookdir`.
- On commit, the config hook ran first, then the hooksPath hook. With the config hook failing, the hooksPath hook still ran and the commit exited 1.
- The hook fired from a linked worktree too (config is in the common `.git/config`).
- With gitleaks absent from PATH: `gitleaks: command not found`, commit exit 1 (fails closed).

[VERIFIED: executed]

From the local git 2.56 manual (`git help config`): "`hook.<friendly-name>.event` ... This is a multi-valued key", and "The `<friendly-name>` must not be the same as a known hook event name (e.g. do not use `hook.pre-commit.event`). Using a known event name as a friendly-name is a fatal error". Executed: `fatal: hook friendly-name 'pre-commit' collides with a known event name`, exit 128. [VERIFIED: local man page + execution]
```bash
# scripts/setup-repo.sh (name is discretion) — idempotent, local config only
set -euo pipefail
HOOK=gitleaks-staged   # not an event name; not likely in global config; not in wsx's future product namespace
git config --local "hook.$HOOK.command" "gitleaks git --pre-commit --staged --redact"
git config --local --replace-all "hook.$HOOK.event" pre-commit
# include.path is multi-valued: add only if absent (plain `git config include.path X` errors when 2+ values exist)
git config --local --get-all include.path | grep -qxF ../.gitconfig \
  || git config --local --add include.path ../.gitconfig
```
The exact hook command string is locked (SUP-03). Observation, not a decision: with this command, gitleaks prints only `leaks found: 1` and no file or rule. `--verbose` would add them, still redacted.

### Pattern 7: Tracked `.gitconfig` (DEV-05)
Port devflow's `[gpg] format = ssh`, `[commit] gpgsign = true`, `[tag] gpgsign = true`, `[gitflow "branch"] master = main / develop = develop` and `[gitflow "prefix"] feature = feature/ ...`, with the "do not run `git flow init -d`" comment.

Drop from devflow's file:
- the `devflow.releaseSigningKey` block (release-key policy is milestone 2, DEVFLOW-SETUP D11)
- the `[alias]` block (devflow-specific scripts)
- anything naming a path, key or person

Verified in a scratch repo with `GIT_CONFIG_GLOBAL=/dev/null`:
- With the include: the commit carries `BEGIN SSH SIGNATURE` (1 line) and an annotated tag is signed (1 line); `gitflow.branch.master=main`.
- After `--unset include.path`: commit 0 lines, tag 0 lines.

[VERIFIED: executed]

### Pattern 8: Ruleset update for REL-04 (prepared, never executed by executors)
Current state, read via `gh api`:
- `25445011 develop-merge-or-squash` and `25445012 main-squash-only`, both `active`, `bypass_actors: []`
- rules: `pull_request` (develop: `allowed_merge_methods ["merge","squash"]`; main: `["squash"]`), `deletion`, `non_fast_forward`
- no `required_status_checks` rule yet

[VERIFIED: gh api GET]

The REST docs list `rules` as "An array of rules within the ruleset" and do not say whether PUT replaces or merges. Send the full array, existing rules plus the new one, which is correct under either semantics, then re-read.
```bash
# OWNER-RUN ONLY, after CI has reported Test/Clippy/Format at least once
for id in 25445011 25445012; do
  gh api "repos/denniyahh/wsx/rulesets/$id" \
  | jq '{name, target, enforcement, conditions, bypass_actors,
         rules: (.rules + [{type: "required_status_checks", parameters: {
           strict_required_status_checks_policy: true, do_not_enforce_on_create: false,
           required_status_checks: [{context: "Test", integration_id: 15368},
                                    {context: "Clippy", integration_id: 15368},
                                    {context: "Format", integration_id: 15368}]}}])}' \
  | gh api -X PUT "repos/denniyahh/wsx/rulesets/$id" --input -
done
gh api repos/denniyahh/wsx/rules/branches/develop --jq '.[] | select(.type=="required_status_checks")'
gh api repos/denniyahh/wsx/rules/branches/main    --jq '.[] | select(.type=="required_status_checks")'
```
- `integration_id: 15368` pins the context to the GitHub Actions app, so a commit status from another source cannot satisfy it. devflow omits it. Discretion; recommended.
- Verify through `rules/branches/<b>`, not `branches/<b>/protection`, which does not show ruleset checks (devflow `ci.yml:14-19`).

### Anti-Patterns to Avoid
- **Literal fake secret anywhere tracked** (plans, VERIFICATION.md, scripts): the global and repo gitleaks hooks will refuse the planning commit, and a "secret" would land in public history. Generate it at runtime (recipe C4) and record only gitleaks' redacted output.
- **`cargo init` / `cargo new` in the repo:** appends `/target` at the end of `.gitignore`, next to the workspace-only block, which conflicts when the commit is cherry-picked onto `develop` (Pitfall 9).
- **Porting devflow's `.claude/skills/ai-change-acceptance/` reference** into CONTRIBUTING: wsx has no such skill, and `.claude` is a forbidden path that `cut-pr-branch.sh` strips (`FORBIDDEN_PATHS=(.agents .bg-shell .claude ...` at line 120).
- **A pre-push hook that runs `check.sh`** (devflow has one): it needs the `GIT_*` scrub (devflow incident: a worktree push flipped the repo to `core.bare=true`). Not required here; do not add it in Phase 1.
- **`continue-on-error: true` on the deny job.**

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Secret detection | regex grep in a hook | gitleaks 8.30.1 | Rule set, entropy, allowlists |
| Advisory/license audit | parsing Cargo.lock against RustSec | cargo-deny 0.20.2 | Maintained DB, license expressions, sources |
| Test runner no-match guard | counting `cargo test` output | nextest `--no-tests=fail` | `cargo test <filter>` exits 0 on no match (verified: "1 filtered out", exit 0) |
| Toolchain provisioning in container | custom rustup calls | `rust-toolchain.toml` + rustup auto-install | Verified in the image |
| Action/image update PRs | custom tag-polling workflow | Dependabot | See Open Question 1 for its image limits |
| Hook chaining | wrapper that calls the global hook | git config-based hooks | Coexist with `core.hooksPath` natively |

**Key insight:** every gate here was at some point silently green in devflow. The value of this phase is the discriminating controls, not the scripts.

## Runtime State Inventory

Not a rename phase. One category applies, because the setup script writes state outside git:

| Category | Items Found | Action Required |
|----------|-------------|-----------------|
| Stored data | None (no datastores) | none |
| Live service config | GitHub rulesets 25445011/25445012 (no required checks yet); Dependabot alerts on, security updates off, push protection on (verified via `gh api`) | Ruleset PUT is owner-gated (Pattern 8); nothing else changes |
| OS-registered state | Docker named volumes created by `check-in-container.sh` (per-checkout, persist after a worktree is removed); the pulled image `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie` (pulled during this research, kept for execution) | Document cleanup: `docker volume ls --filter name=wsx-ci-`; executors name the volumes they created |
| Secrets/env vars | None. Signing key stays in local config (`user.signingkey` is set in the wsx `.git/config` today) | none |
| Repo-local git config | Running the setup script on the real repo writes `hook.<name>.*` and `include.path` into `/var/home/denniyahh/Github/wsx/.git/config`, which is shared by the main checkout and every worktree | Run controls in scratch clones. Whether executors run the setup script against the real repo is a local, non-outward action; report it when done |

## Common Pitfalls

### Pitfall 1: Dependabot cannot bump the devcontainer image as configured
**What goes wrong:** `package-ecosystem: devcontainers` updates only Features. `devcontainer.json`'s `image` and the workflows' `container.image` never get a PR, so SUP-02's image half silently does nothing.
**Evidence:**
- dependabot-core `devcontainers/lib/dependabot/devcontainers/file_parser.rb`: `parse` iterates `parse_features(...)` only.
- The `docker` ecosystem's `shared_file_fetcher.rb` keeps a YAML file only if `likely_kubernetes_resource?` (has `apiVersion` and `kind`) or it is a Helm chart, so workflow files are skipped.
- Its Dockerfile regex is `/dockerfile|containerfile/i`.

[VERIFIED: dependabot-core source read via gh api, 2026-10-22; not exercised by a real Dependabot run]
**How to avoid:** Open Question 1.

### Pitfall 2: nextest is not in the devcontainer image
**What goes wrong:** `check-in-container.sh test` and the CI `Test` job fail with the missing-tool error. `/usr/local/cargo/bin` holds `cargo cargo-clippy cargo-fmt cargo-miri clippy-driver rls rust-analyzer rust-gdb rust-gdbgui rust-lldb rustc rustdoc rustfmt rustup`. [VERIFIED: `ls` in image]
**How to avoid:** `scripts/install-tools.sh install cargo-nextest` before `check.sh test`, in the container script and in CI.

### Pitfall 3: "remove the tool from PATH" controls that do not remove it
**What goes wrong:** on this host `cargo-deny` and `gitleaks` exist twice: `/home/linuxbrew/.linuxbrew/bin/` and `~/.local/share/mise/shims/`, both reporting the pinned versions. Dropping only the brew dir leaves the tool reachable, and the control "passes" without testing anything. [VERIFIED: `type -a -p`, shim `--version`]
**How to avoid:** build a minimal explicit PATH rather than filtering (recipe C1). `bash` exists at `/usr/bin/bash` and `cargo` only at `~/.cargo/bin`.

### Pitfall 4: nextest already fails on no tests by default
**What goes wrong:** `cargo nextest run -E 'test(=no_such_test)'` exits 4 (`error: no tests to run`) **without** `--no-tests=fail` (default `auto`, "defaulting to fail"). Criterion 2's naive control cannot tell whether `check.sh` passes the flag. [VERIFIED: executed]
**How to avoid:** use control C2 (`NEXTEST_NO_TESTS=pass` must still fail). Executed: env `pass` without the flag → exit 0; env `pass` with `--no-tests=fail` → exit 4.

### Pitfall 5: `include.path ../.gitconfig` from a linked worktree reads the MAIN checkout's file
**What goes wrong:** the include lives in the common `.git/config`, so `../.gitconfig` resolves to `/var/home/denniyahh/Github/wsx/.gitconfig` (main checkout, on `workspace/denniyahh`), not this worktree's new file. A missing include target is silently ignored. Verified in scratch: worktree had `.gitconfig` and the main checkout did not, so `commit.gpgsign` read as empty. [VERIFIED: executed]
**How to avoid:** verify DEV-05 in a scratch clone whose main checkout holds the file. On the real repo, the include takes effect only once `.gitconfig` is on the branch checked out in the main checkout.

### Pitfall 6: Global config makes the signing check pass regardless
**What goes wrong:** `~/.gitconfig` already sets `gpg.format ssh`, `commit.gpgsign true`, `tag.gpgsign true`, so "commits are signed" is true with or without the tracked file. [VERIFIED: `git config --show-origin`]
**How to avoid:** run the DEV-05 check with `GIT_CONFIG_GLOBAL=/dev/null GIT_CONFIG_NOSYSTEM=1`, plus the no-include negative control (recipe C6).

### Pitfall 7: The global gitleaks hook masks the repo hook
**What goes wrong:** the global `core.hooksPath=/home/denniyahh/.config/git/hooks` pre-commit runs `gitleaks protect --staged`, so a refused commit does not prove the **repo** hook bit. That global hook also skips with a warning when gitleaks is missing.
**How to avoid:** run C4 with `GIT_CONFIG_GLOBAL` pointing at an empty file, and include the `hook.<name>.enabled=false` control (same commit must succeed).

### Pitfall 8: Planted `set_var` fails for the wrong reason
**What goes wrong:** in edition 2024 a bare `std::env::set_var(...)` is `error[E0133]: call to unsafe function ... requires unsafe block`, a compile error, not the lint. [VERIFIED: executed]
**How to avoid:** plant `unsafe { std::env::set_var("WSX_PLANTED", "1") };` inside a `#[cfg(test)]` module, then assert the output contains ``use of a disallowed method `std::env::set_var` ``. Verified discriminators:
- without `clippy.toml`, the same plant passes clippy (exit 0)
- without `--all-targets`, it passes (exit 0)
- with both, it fails (exit 101)

If any extra lint such as `unsafe_code = "forbid"` is added, the reason check is what keeps this control honest.

### Pitfall 9: `.gitignore` hunk placement breaks the PR cut
**What goes wrong:** `.gitignore` is `modified` on the workspace branch: develop's file ends at `*.orig` (verified `git show origin/develop:.gitignore`), and the workspace adds the "Personal workspace" block (lines 19-23). A `/target/` line appended at the end, which is what `cargo init` does (verified), has workspace-only context and conflicts when `cut-pr-branch.sh` cherry-picks onto `origin/develop`.
**How to avoid:** add a `# Rust` / `/target/` block at the top of the file, above `# Python`.

### Pitfall 10: Commits without the phase scope are silently left out of the PR
**What goes wrong:** `cut-pr-branch.sh` phase mode selects commits by `SCOPE_RE="^[a-z]+\((phase-)?0*${PHASE_ESC}(-[0-9]+)?\)!?: "` (line 135). A `chore(ci): ...` commit is not replayed.
**How to avoid:** every code commit in this phase uses scope `(01)` or `(01-NN)`.

### Pitfall 11: Scheduled jobs and Dependabot only act from `main`
`main` is the default branch [VERIFIED: gh api]:
- The weekly `deny.yml` cron runs only once the workflow is on `main` (SUP-01 already says so).
- Dependabot reads `.github/dependabot.yml` only from the default branch. [CITED: docs.github.com GHES 3.10/3.13 "About the dependabot.yml file"]

Neither is verifiable in this phase; mark both as "takes effect after release to main".

### Pitfall 12: Dependabot's docker tag heuristics on devcontainer tags
dependabot-core#15530 (open, 2026-07-28): the docker updater "upgraded" `javascript-node:4.0.13-24-trixie` to the less specific `4-22-trixie`. If option A is chosen, review Dependabot's FROM bumps by hand. The parity check cannot catch a "valid" but floating tag. [CITED: github.com/dependabot/dependabot-core/issues/15530]

## Code Examples

### Negative-control recipes (run each in scratch; never in the tracked tree)
All scratch paths go under the session scratchpad, not `/tmp` (tmpfs on this host). Remove them afterwards and say so in the summary. Each records the failing output **and** its discriminating counterpart.

**C1, missing tool names the tool (criterion 1)**
```bash
MIN_PATH="$HOME/.cargo/bin:/usr/bin:/bin"          # cargo yes; nextest/deny/gitleaks no (brew + mise shims excluded)
env PATH="$MIN_PATH" scripts/check.sh test; echo "exit=$?"   # expect ≠0 and "cargo-nextest" in stderr
env PATH="$MIN_PATH" scripts/check.sh deps; echo "exit=$?"   # expect ≠0 and "cargo-deny"
env PATH="/usr/bin:/bin" scripts/check.sh fmt; echo "exit=$?" # expect ≠0 and "cargo"
scripts/check.sh test; echo "exit=$?"                         # counterpart: full PATH passes
```

**C2, `--no-tests=fail` is really on the command line (criterion 2, HARN-04)**
```bash
NEXTEST_NO_TESTS=pass scripts/check.sh test -E 'test(=no_such_test)'; echo "exit=$?"  # expect 4 ("no tests to run")
# counterpart proving the env var works at all (run directly, not via check.sh):
NEXTEST_NO_TESTS=pass cargo nextest run -E 'test(=no_such_test)'; echo "exit=$?"     # expect 0
```

**C3, planted `set_var` (criterion 2)**: in a scratch copy (`git worktree add` to the scratchpad, or `cp -r`), append the Pitfall 8 plant to `src/main.rs`, then run `scripts/check.sh clippy`. Expect exit ≠0 and the disallowed-method message. Counterpart: the same plant with `clippy.toml` moved aside → exit 0.

**C4, gitleaks hook refuses a fake secret (SUP-05)**
```bash
# scratch clone, isolated from the global hook
export GIT_CONFIG_GLOBAL="$SCRATCH/empty.gitconfig" GIT_CONFIG_NOSYSTEM=1; : > "$GIT_CONFIG_GLOBAL"
git -C "$SCRATCH/clone" config user.name t; git -C "$SCRATCH/clone" config user.email t@example.invalid
(cd "$SCRATCH/clone" && scripts/setup-repo.sh)
tok="ghp_$(head -c 300 /dev/urandom | tr -dc A-Za-z0-9 | head -c 36)"   # generated, never written to tracked files
printf 'k=%s\n' "$tok" > "$SCRATCH/clone/leak.txt"; git -C "$SCRATCH/clone" add leak.txt
git -C "$SCRATCH/clone" commit -m leak; echo "exit=$?"                   # expect 1, "leaks found: 1"
git -C "$SCRATCH/clone" -c hook.gitleaks-staged.enabled=false commit -qm leak; echo "exit=$?"  # counterpart: 0
```
Executed with the same command string: `WRN leaks found: 1`, exit 1; with the hook disabled, exit 0. When recording output, pipe it through `sed 's/ghp_[A-Za-z0-9]*/ghp_<scrubbed>/g'` as a belt-and-braces measure; `--redact` already hides the match. [VERIFIED]

**C5, cargo deny fails on a vulnerable crate (SUP-05)**
```bash
cp -r . "$SCRATCH/deny-control" && cd "$SCRATCH/deny-control"
sed -i 's/^\[dependencies\]$/[dependencies]\nsmallvec = "=1.6.0"/' Cargo.toml   # requires a [dependencies] header in Cargo.toml
cargo generate-lockfile && cargo deny --locked check; echo "exit=$?"
# executed: "error[vulnerability]: Buffer overflow in SmallVec::insert_many", "ID: RUSTSEC-2021-0003",
#           "advisories FAILED, bans ok, licenses ok, sources ok", exit 1
```
Counterpart: the unmodified tree → `advisories ok, bans ok, licenses ok, sources ok`, exit 0. [VERIFIED]

**C6, tracked `.gitconfig` signs (DEV-05)**: Pattern 7's scratch procedure. Requires: `GIT_CONFIG_GLOBAL=/dev/null`, a throwaway `ssh-keygen -t ed25519 -N ''` key as local `user.signingkey`, the file present in the main checkout (Pitfall 5), and the `--unset include.path` counterpart.

**C7, parity check bites (criterion 3)**: in a scratch copy, change one workflow's `image:` to `...:2.1.2-1-trixie`, run `scripts/assert-image-parity.sh`, expect ≠0 naming both values. Counterpart: the untouched tree passes.

### `clippy.toml` (port verbatim, devflow `clippy.toml:19-22`)
```toml
disallowed-methods = [
  { path = "std::env::set_var", reason = "process-global and racy across concurrent tests and the children they spawn; scope it to the child with Command::env instead. Test-only exceptions need a reasoned expect(clippy::disallowed_methods)." },
  { path = "std::env::remove_var", reason = "process-global and racy across concurrent tests and the children they spawn; scope it to the child with Command::env_remove or Command::env_clear instead. Test-only exceptions need a reasoned expect(clippy::disallowed_methods)." },
]
```

### `deny.toml` (validated with cargo-deny 0.20.2: no warnings; passes clean, fails C5)
```toml
[graph]
all-features = true

[advisories]
unmaintained = "all"
yanked = "deny"
ignore = []

[licenses]
allow = ["MIT"]             # Phase 1 has no deps; later phases add (STACK.md list: Apache-2.0, Apache-2.0 WITH LLVM-exception, Unlicense, Zlib, Unicode-3.0, BSD-3-Clause)
confidence-threshold = 0.93

[bans]
multiple-versions = "warn"  # STACK.md: gix brings hashbrown x3, syn x2
wildcards = "deny"
allow-wildcard-paths = true

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```
Set `license = "MIT"` in `Cargo.toml` (LICENSE is MIT, "Copyright (c) 2026 Dennis Kim"). Executed: without a license field, `error[unlicensed]: wsx = 0.0.0 is unlicensed`, licenses FAILED, exit 4. Unused allowances only warn (`license-not-encountered`), so keep the list to what is used. [VERIFIED]

### `rust-toolchain.toml`
```toml
[toolchain]
channel = "1.99.1"
components = ["clippy", "rustfmt"]
profile = "minimal"
```

### Placeholder crate (discretion)
A binary whose `main` prints `wsx <CARGO_PKG_VERSION>` for `--version`, with one test asserting real output. Prefer an integration test in `tests/` that spawns `env!("CARGO_BIN_EXE_wsx")` with `--version` and asserts stdout. That is a public-boundary assertion, so it avoids rejected shape 1, "asserts only constants". `CARGO_BIN_EXE_<name>` is set for integration tests of a package with that binary. [ASSUMED from Cargo docs knowledge; trivially confirmed by the first `check.sh test` run]

### `.github/dependabot.yml`
Shape common to all entries:
```yaml
version: 2
updates:
  - package-ecosystem: github-actions
    directory: /
    target-branch: develop
    schedule: { interval: weekly }
    groups: { github-actions: { patterns: ["*"] } }
  - package-ecosystem: devcontainers          # locked by CONTEXT; updates Features only (none planned → no PRs)
    directory: /
    target-branch: develop
    schedule: { interval: weekly }
    groups: { devcontainers: { patterns: ["*"] } }
  # Option A only (Open Question 1): image pin moved to .devcontainer/Dockerfile FROM
  - package-ecosystem: docker
    directory: /.devcontainer
    target-branch: develop
    schedule: { interval: weekly }
    groups: { devcontainer-image: { patterns: ["*"] } }
```
- No `cargo` entry (SUP-02: no per-crate PRs).
- Dependabot rewrites SHA pins and their `# vX.Y.Z` comments together. [CITED: github.blog changelog 2022-10-31]
- `target-branch` applies to version updates only. Security updates always use the default branch, which is why automatic security fixes stay off. [CITED: docs.github.com dependabot-options-reference] They are off today: `automated-security-fixes` → `{"enabled":false}`. [VERIFIED]

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| `core.hooksPath` + chaining shim (devflow) | `hook.<name>.command/event` config hooks | git 2.54 | Coexists with global hooks without shims |
| `gitleaks protect --staged` (global hook) | `gitleaks git --pre-commit --staged` | gitleaks v8.19+ | `protect` is the legacy spelling |
| nextest `--no-tests` default `pass`/`warn` | default `auto` = fail | before 0.9.148 | The flag is now about pinning against env/config overrides |
| `cargo install` tools in CI | checksum-pinned release binaries | — | Faster, artefact-verified |
| actions `@v7` tags (devflow) | full commit SHA + version comment | — | Tags are mutable |

**Deprecated/outdated:** devflow's `cargo test` gate (no no-match guard); devflow's argument-based parity check; its CPU-pin job (not ported per DEV-06).

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Ruleset PUT replaces the `rules` array (docs silent) | Pattern 8 | None if the full array is sent as recommended; the re-read verifies |
| A2 | Dependabot's docker updater handles `FROM ...:2.1.3-1-trixie` sensibly | Dependabot, Pitfall 12 | A wrong or floating-tag PR; human review and parity catch half of it |
| A3 | `devcontainers` ecosystem with zero Features opens no PRs and raises no errors | Dependabot | Noise only |
| A4 | Dependabot reads config only from the default branch (cited from GHES docs, not github.com) | Pitfall 11 | Timing of when updates start |
| A5 | `actions/checkout@v7.0.1` works in the trixie container (devflow proves `@v7` in the bookworm image, runs green 2026-10-10) | Pattern 5 | First CI run fails at checkout |
| A6 | `CARGO_BIN_EXE_wsx` available to integration tests | Placeholder crate | Test fails to compile; first `check.sh test` shows it |
| A7 | GitHub accepts `integration_id: 15368` for these contexts (the app id is verified from devflow check runs) | Pattern 8 | PUT rejected; drop `integration_id` |

## Open Questions (RESOLVED)

1. **How should Dependabot keep the devcontainer image current?** RESOLVED 2026-10-22: Option A, decided by the orchestrator on this evidence (Option B would drop half of SUP-02; A keeps every requirement's intent). Recorded in 01-CONTEXT.md "Decisions after research"; reported to the owner for review.
   - What we know: CONTEXT locks `github-actions` + `devcontainers`. Source code shows `devcontainers` updates only Features, and no Dependabot ecosystem reads `devcontainer.json`'s `image` or a workflow `container.image`. The only Dependabot-updatable image pin is a Dockerfile `FROM`.
   - Option A (recommended):
     - Add `.devcontainer/Dockerfile` with only `FROM mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`. `devcontainer.json` uses `"build": {"dockerfile": "Dockerfile"}`.
     - Add a `docker` ecosystem entry for `/.devcontainer`. `check-in-container.sh` and the parity script read the `FROM` line.
     - A Dependabot PR bumps the Dockerfile only. Its `Test` job then fails the parity check until a human copies the tag into the workflows in the same PR, so the PR is red by design.
     - Deviates from DEV-06's wording "devcontainer.json pins an exact image tag": the pin moves one file over.
   - Option B: keep `"image"` in `devcontainer.json` exactly as DEV-06 words it, and amend SUP-02 to "Actions and devcontainer Features". Image bumps become manual.
   - Recommendation: A. It is the only way SUP-02's stated outcome happens at all, and the parity check turns the half-automation into a hard stop rather than drift.

2. **Run the setup script against the real repo's `.git/config` during execution?** RESOLVED 2026-10-22: yes, decided by the orchestrator (within executor remit; recorded in 01-CONTEXT.md). Inside the executor's remit (local, not outward-facing). Recommend yes, after the scratch controls pass, reported as done. It changes nothing visible, because the global gitleaks hook already runs.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| rustc/cargo via rustup | all gates | ✓ | 1.99.1 (rustup 1.29.1) | — |
| cargo-nextest | `check.sh test` | ✓ (brew) | 0.9.148 | install-tools |
| cargo-deny | `check.sh deps` | ✓ (brew + mise shim) | 0.20.2 | install-tools |
| gitleaks | pre-commit hook | ✓ (brew + mise shim) | 8.30.1 | install-tools |
| Docker daemon (rootless) | `check-in-container.sh` | ✓ | 29.9.1 | none (DEV-06 requires it) |
| Devcontainer image | container + CI | ✓ pulled | `2.1.3-1-trixie` | — |
| git | hooks, worktrees | ✓ | 2.56.0 | — |
| shellcheck | linting new scripts | ✓ host only (not in image) | 0.11.0 | `bash -n` |
| gh | read-only API checks | ✓ | 2.102.0 | — |
| Network (GitHub releases, crates.io, static.rust-lang.org, RustSec DB) | install-tools, toolchain auto-install, C5 | ✓ | — | none |
| actionlint | workflow linting | ✗ | — | none locally; the first owner-gated CI run is the real validation (optionally `docker run rhysd/actionlint:1.7.12`, image name [ASSUMED]) |

**Missing dependencies with no fallback:** none that block. Workflow YAML correctness is only proven by the owner-gated first CI run.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | cargo-nextest 0.9.148 (Rust); negative-control recipes C1-C7 (shell, scratch space); `shellcheck` + `bash -n` for scripts |
| Config file | none needed (no `.config/nextest.toml`; all behaviour on the `check.sh` command line) |
| Quick run command | `scripts/check.sh test` |
| Full suite command | `scripts/check.sh all && scripts/check-in-container.sh all && scripts/assert-image-parity.sh && shellcheck scripts/*.sh` |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DEV-01 | each target runs; missing tool named | smoke + control | `scripts/check.sh all`; recipe C1 | ❌ Wave 0 |
| DEV-02 | pinned rustc used | smoke | `rustc --version` in repo root → `1.99.1`; in container: `scripts/check-in-container.sh fmt` log shows auto-install of 1.99.1 | ❌ |
| DEV-03/04 | docs contain traps / bar / four shapes | manual review | reviewer reads; no grep-as-test (rejected shape 4) | ❌ |
| DEV-05 | signing + git-flow via include | scripted control | recipe C6 | ❌ |
| DEV-06 | container parity from a linked worktree | smoke + control | `scripts/check-in-container.sh all` from `.worktrees/phase-1`; recipe C7 | ❌ |
| SUP-01 | deny job exists, advisory | manual + owner CI run | `scripts/check.sh deps` locally | ❌ |
| SUP-02 | dependabot config | manual (effective only on `main`) | YAML review | ❌ |
| SUP-03/05 | hook refuses secret; deny fails vuln crate | scripted control | recipes C4, C5 | ❌ |
| SUP-04 | lock committed, SHAs pinned, versions in one script | smoke | `cargo metadata --locked >/dev/null`; review `uses:` lines carry 40-hex SHAs | ❌ |
| HARN-04 | no-tests fail, set_var banned | scripted control | recipes C2, C3 | ❌ |
| REL-04 | required checks | owner-gated | Pattern 8 command + re-read | n/a |

### Sampling Rate
- **Per task commit:** `scripts/check.sh all` (seconds for a placeholder crate)
- **Per wave merge:** add `scripts/check-in-container.sh all` and `shellcheck scripts/*.sh`
- **Phase gate:** full suite plus every control C1-C7 with quoted output, before `/gsd-verify-work`

### Wave 0 Gaps
- [ ] `Cargo.toml`, `src/main.rs` (+ `tests/cli.rs`), `Cargo.lock`, `rust-toolchain.toml`: nothing exists yet
- [ ] `scripts/check.sh`, `scripts/install-tools.sh`: the gate itself
- [ ] No framework install needed on the host (all tools present)

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | partly (CI token) | `permissions: contents: read`; `persist-credentials: false`; `pull_request` (never `pull_request_target`) |
| V5 Input Validation | partly (workflow expressions) | no `${{ github.event.* }}` interpolated into `run:` |
| V6 Cryptography | partly (signing, checksums) | git SSH signing; `sha256sum -c` on every downloaded tool |
| V10 Malicious Code / V14 Config (supply chain) | yes | SHA-pinned actions, pinned image tag, checksum-pinned tools, cargo-deny, gitleaks, Dependabot alerts |

### Known Threat Patterns for this stack
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Mutable action tag repointed | Tampering | Full commit SHA pin + Dependabot updates |
| Tampered tool download | Tampering | Pinned sha256, verified before extract |
| Secret committed | Information disclosure | Repo gitleaks hook + global hook + server push protection |
| Required check spoofed by another status source | Spoofing | `integration_id: 15368` on ruleset contexts |
| Verification mutating source | Tampering | `:ro` mounts in `check-in-container.sh` |
| Personal data in public `.gitconfig` | Information disclosure | No `user.*`, key paths or names; reviewer check |
| Host email or identity leaked to third-party services | Information disclosure | Never put contact details in request headers (see Metadata) |

## Project Constraints (from CLAUDE.md)

Project `CLAUDE.md` (interim):
- `bin/wsx` (Python 0.1.0) is frozen: read for reference only; do not change it or follow its old constraints (standard library only, copy-sync model).

Operator global rules that bind executors in this phase:
- Every measurement has a negative control; quote the failing output.
- Distinguish absent from false; a skipped check is a failure.
- `rg`, not `grep`, for searches run by agents. Script internals may use POSIX tools.
- Bash scripts: `set -euo pipefail`, quoted expansions, `shellcheck` / `bash -n`.
- Scratch work under the session scratchpad, not `/tmp`. Remove it, stop recorded PIDs, report cleanup.
- Gate commits on exit codes, not on reading output.
- Conventional Commits with phase scope (Pitfall 10).
- Supply-chain policy (scan on change and on schedule, pin what you pull, prove each scanner bites): this phase implements it.
- No outward-facing actions (push, PR, settings, rulesets).

## Sources

### Primary (HIGH confidence; executed or read this session)
- Local execution, 2026-10-22: rustup auto-install in `2.1.3-1-trixie`; nextest no-tests matrix; clippy disallowed-methods matrix; cargo-deny clean, vulnerable and unlicensed runs; git 2.56 config hooks with hooksPath, worktree and missing binary; gitleaks refusal and disabled control; `.gitconfig` include, signing and worktree resolution; container ro mounts with and without the common dir; Docker volume seeding; `-t` without a TTY
- `git help config` (git 2.56.0): `hook.<friendly-name>.*`
- MCR `https://mcr.microsoft.com/v2/devcontainers/rust/tags/list` and manifest digests
- GitHub API (read-only): actions/checkout tags; nextest, cargo-deny and gitleaks release assets and digests; wsx and devflow rulesets; devflow check runs; wsx security settings
- dependabot-core source: `devcontainers/.../file_parser.rb`, `docker/.../file_fetcher.rb`, `docker/.../file_parser.rb`, `docker/lib/dependabot/shared/shared_file_fetcher.rb`
- devflow (read-only, `workspace/denniyahh` @ 31b0a01): `scripts/check.sh`, `check-in-container.sh`, `assert-image-parity.sh`, `install-dep-tools.sh`, `.devcontainer/*`, `.github/workflows/*`, `deny.toml`, `clippy.toml`, `rust-toolchain.toml`, `.gitconfig`, `CLAUDE.md`, `CONTRIBUTING.md`. devflow has **no** `.config/nextest.toml` and **no** `.github/dependabot.yml` (checked)

### Secondary (MEDIUM)
- [Dependabot options reference](https://docs.github.com/en/code-security/dependabot/working-with-dependabot/dependabot-options-reference): target-branch, groups, ecosystems
- [REST: repository rulesets](https://docs.github.com/en/rest/repos/rules?apiVersion=2022-11-28#update-a-repository-ruleset): `required_status_checks` schema
- [GitHub changelog 2022-10-31](https://github.blog/changelog/2022-10-31-dependabot-now-updates-comments-in-github-actions-workflows-referencing-action-versions/): SHA-pin comment updates
- [GHES "About the dependabot.yml file"](https://docs.github.com/en/enterprise-server@3.13/code-security/dependabot/working-with-dependabot/dependabot-options-reference): default-branch requirement
- [Supported ecosystems](https://docs.github.com/en/enterprise-cloud@latest/code-security/dependabot/ecosystems-supported-by-dependabot/supported-ecosystems-and-repositories): devcontainers = Features

### Tertiary (LOW)
- [dependabot-core#15530](https://github.com/dependabot/dependabot-core/issues/15530): devcontainers tag mis-upgrade (single report)

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH (versions, checksums and image probed directly)
- Architecture: HIGH for local gates and container; MEDIUM for CI (first run is owner-gated)
- Pitfalls: HIGH (each reproduced, except 11 and 12, which are cited)
- Dependabot: MEDIUM (source-read; never exercised end to end)

**Research hygiene note:** one crates.io metadata request in this session sent the operator's email in its User-Agent header. That should not have happened and will not be repeated. Executors must not put contact details in request headers.

**Research date:** 2026-10-22
**Valid until:** 2026-11-05 (image and tool releases move weekly; re-check the MCR tag list and action SHA before execution if later)
