---
last_mapped_commit: a686ea156dac3e4c93d9f9d180ee4a35e340c4f5
last_mapped_at: 2026-11-18
---
# External Integrations

**Analysis Date:** 2026-11-18

## APIs & External Services

**Version control:**
- git CLI (>= 2.54) - spawned as a child process only through `Git` in `src/git/spawn.rs`; used from `src/git/refs.rs`, `src/git/objects.rs`, `src/git/config.rs`
  - SDK/Client: `std::process::Command` wrapper; `NO_LAZY_FETCH` set on reads so no promisor fetch happens
  - Auth: none (no network operations)
- gix `=0.90.0` - in-process repository open (only `open_gix` in `src/git/repo.rs`) and reads; no network features

No HTTP APIs, SDKs or remote services.

## Data Storage

**Databases:**
- None. State lives in the git repository itself (refs, commits, worktrees) and the `.wsx.conf` manifest (`src/model/manifest.rs`)

**File Storage:**
- Local filesystem only

**Caching:**
- None

## Authentication & Identity

**Auth Provider:**
- None
  - Implementation: repository ownership re-checked against the effective uid via `rustix` (`src/git/repo.rs`), stricter than gix's `SUDO_UID` trust

## Monitoring & Observability

**Error Tracking:**
- None

**Logs:**
- stderr messages and `ExitCode` from `src/main.rs`; non-interactive, never prompts

## CI/CD & Deployment

**Hosting:**
- Not applicable (unpublished binary)

**CI Pipeline:**
- GitHub Actions in the pinned devcontainer image (workflows outside the mapped paths); `scripts/assert-image-parity.sh` and `tests/image_parity.rs` keep workflow images equal to `.devcontainer/Dockerfile`
- Dependabot updates the Dockerfile `FROM` line (reason `devcontainer.json` stays Feature-free)
- gitleaks pre-commit hook registered by `scripts/setup-repo.sh` (`tests/setup_repo.rs`, `tests/repo_policy.rs`)

## Environment Configuration

**Required env vars:**
- None. Read when set: `GIT_DIR`, `GIT_INDEX_FILE`, `GIT_CEILING_DIRECTORIES`, `SUDO_UID` (`src/git/repo.rs`); `PROPTEST_CASES` in tests

**Secrets location:**
- None used

## Webhooks & Callbacks

**Incoming:**
- git hooks: wsx runs inside hooks and honours the hook's `GIT_DIR`/`GIT_INDEX_FILE` (rationale in `clippy.toml`); oracle hooks in `tests/oracle/hooks/`

**Outgoing:**
- None

---

*Integration audit: 2026-11-18*
