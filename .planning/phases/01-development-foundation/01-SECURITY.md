---
phase: "1"
slug: "development-foundation"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-10-23"
---

# Phase 1 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Release downloads → host/container | `scripts/install-tools.sh` fetches nextest, cargo-deny, gitleaks | Executables (sha256-pinned) |
| Container registry → CI/devcontainer | `mcr.microsoft.com/devcontainers/rust` image | Build environment (exact tag, parity-checked) |
| Third-party GitHub Actions → CI | `actions/checkout` | Code running with the job token (SHA-pinned, read-only permissions) |
| crates.io → build | Cargo dependencies (none yet) | Source (lockfile, `--locked`, deny sources) |
| Workspace branch → `develop` (public) | PR cut replays phase commits | Personal/agent files must not cross |
| Developer commit → history | Staged changes | Secrets (gitleaks pre-commit, push protection) |

---

## Threat Register

Built from the `<threat_model>` blocks of 01-01..01-07-PLAN.md (33 threats, all `mitigate`). Evidence per threat: gsd-security-auditor verdict, 2026-10-23.

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-01-01 | T | check.sh test | medium | mitigate | `--no-tests` refused, flag fixed before args; tests/gate.rs | closed |
| T-01-02 | R | check.sh tools | high | mitigate | `require_tool` exit 1; tests/gate_controls.rs (C1) | closed |
| T-01-03 | T | no-match filter | medium | mitigate | `--no-tests=fail` outranks env; gate_controls.rs (C2) | closed |
| T-01-04 | T | env mutation | medium | mitigate | clippy.toml ban, `--all-targets`; gate_controls.rs (C3) | closed |
| T-01-SC(01) | T | crate deps | high | mitigate | no deps, Cargo.lock tracked, `--locked` | closed |
| T-01-05 | T | tool downloads | high | mitigate | https/TLS1.2, sha256 before extract | closed |
| T-01-06 | I | curl requests | medium | mitigate | `curl --disable`, no identifying headers | closed |
| T-01-07 | T | cargo-deny binary | medium | mitigate | version verified through cargo itself (`install-tools.sh verify-cargo`, d179d7a; the earlier CARGO_HOME/bin-first check was bypassable by PATH order, code review CR-02) | closed |
| T-01-08 | S | dependency sources | medium | mitigate | deny.toml: unknown registries and git denied | closed |
| T-01-SC(02) | T | tool versions | high | mitigate | pinned in install-tools.sh only | closed |
| T-01-09 | T | container mount | high | mitigate | source mounted read-only; target volume (decided deviation) | closed |
| T-01-10 | T | image pin | high | mitigate | exact tag in Dockerfile; parity check | closed |
| T-01-11 | T | devcontainer target volume | medium | mitigate | check-in-container.sh hashes full path; devcontainer.json keys on basename | open — below high threshold (non-blocking); accepted, see AR-01 |
| T-01-SC(03) | T | container tools | high | mitigate | installed only via install-tools.sh | closed |
| T-01-12 | T | actions | high | mitigate | SHA-pinned (v7.0.1 verified); tests/repo_policy.rs | closed |
| T-01-13 | E | workflow token | medium | mitigate | read-only permissions, `persist-credentials: false`, no `pull_request_target` | closed |
| T-01-14 | T | workflow injection | medium | mitigate | `${{ }}` only in concurrency keys | closed |
| T-01-15 | T | image drift | high | mitigate | assert-image-parity.sh counts only `jobs.<id>.container.image` (9246940; nested `image:` under container accepted before, code review WR-02); tests/image_parity.rs | closed |
| T-01-16 | R | advisory deny job | medium | mitigate | no continue-on-error; not a required check | closed |
| T-01-SC(04) | T | Dependabot | high | mitigate | targets develop, which requires a PR | closed |
| T-01-17 | I | secrets in commits | high | mitigate | global hook (fail-closed, `--redact`) in this clone; repo hook via setup-repo.sh in fresh clones; push protection on | closed (see AR-02) |
| T-01-27 | E | tracked .gitconfig read live | high | mitigate | copied by setup-repo.sh from a key allowlist, never included; legacy include removed; tests/setup_repo.rs (code review CR-01, b16287f) | closed |
| T-01-18 | I | .gitconfig personal data | high | mitigate | no `user.*`; tests/repo_policy.rs | closed |
| T-01-19 | R | hook registration | medium | mitigate | setup-repo.sh refuses git < 2.54, checks `git hook list` | closed |
| T-01-20 | I | secret in hook output | medium | mitigate | `--redact` in repo and global hooks | closed |
| T-01-21 | I | docs personal data | medium | mitigate | CLAUDE.md, CONTRIBUTING.md scanned clean | closed |
| T-01-22 | T | docs drift | low | mitigate | every script path named in docs exists | closed |
| T-01-SC(06) | T | version drift in docs | high | mitigate | versions only in install-tools.sh | closed |
| T-01-23 | E | outward actions | high | mitigate | blocking-human checkpoint; owner approved push/PR/rulesets 2026-10-23 | closed |
| T-01-24 | S | required checks | medium | mitigate | rulesets require Test/Clippy/Format from app 15368, strict | closed |
| T-01-25 | I | PR cut leak | high | mitigate | forbidden-path strip + audit, core.quotePath=false, quoted names refused; PR #2 head clean | closed |
| T-01-26 | T | leftover controls | medium | mitigate | controls generated at runtime and removed; see note | closed |
| T-01-SC(07) | T | unpinned installs | high | mitigate | nothing installs outside install-tools.sh | closed |

Note on T-01-26: the 01-07 history scan's literal list predates commit 1da1794, so re-run as written it reports `2.1.2-1-trixie` in tests/image_parity.rs. That is the intentional drifted-tag fixture, not a leftover control; the real workflows pin 2.1.3.

*Status: open · closed · open — below {block_on} threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-01 | T-01-11 | Two checkouts with the same folder name share the devcontainer's target volume (stale build output). Local devcontainer only; CI and `check-in-container.sh` are unaffected. Fix tracked as backlog 999.3 | owner (backlogged this item 2026-10-23) | 2026-10-23 |
| AR-02 | T-01-17 | Repo-local gitleaks hook removed from the owner's clone to avoid scanning twice; the global hook (`gitleaks git --pre-commit --staged --redact --verbose`, fail-closed) covers this clone. Holds only while that hook stays installed. 01-05-SUMMARY's note that the global hook skips without gitleaks is stale since 2026-10-23 | owner | 2026-10-22 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-23 | 33 | 32 | 1 (medium, accepted) | gsd-security-auditor (ASVS L1, block_on high) |
| 2026-10-23 | 33 | 32 | 1 (medium, accepted) | orchestrator: T-01-07/T-01-15 re-closed on the code-review fixes (CR-02, WR-02); re-verified by tests and a fake-binary control |
| 2026-10-24 | 34 | 33 | 1 (medium, accepted) | orchestrator: T-01-27 added and closed on CR-01 (owner decision: copy, not include) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-23
