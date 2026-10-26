---
phase: "2"
slug: "oracle-harness"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-10-26"
---

# Phase 2 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Host → scenario process | Every spawned git, bash and wsx process starts from `env_clear()` and gets only the harness keys plus `LLVM_*` (`tests/harness/env.rs`) | Host git config, hooks, identity, SSH agent: must not cross |
| Scenario → network | Fixture remotes are local bare repos; `GIT_ALLOW_PROTOCOL=file` | No network traffic |
| devflow → vendored oracle | `tests/oracle/` holds byte-identical copies of devflow `ee6ec792`, adapted only by `oracle.patch` applied to a scratch copy | Script content, checked against recorded blob ids and modes |
| Release download → tool install | `scripts/install-tools.sh` pins cargo-llvm-cov 0.9.1 by sha256 before extracting | Tool binary |
| Scenario author → recorded outcomes | `validate` and `judge` enforce explicit expectations, difference classes and guard controls | Expected and recorded outcomes |

---

## Threat Register

Audited by gsd-security-auditor at HEAD `fa9b7b2` (ASVS L1, block_on high). Evidence (file:line or test) for each closed threat is in the auditor's verdict, summarised here.

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-02-01 | Tampering | git in fixtures and oracle copy | high | mitigate | `env::command` is the only spawn path: `env_clear()`, ceiling at scenario root; provenance no-ceiling control | closed |
| T-02-02 | Info disclosure | oracle fetch | medium | mitigate | `GIT_ALLOW_PROTOCOL=file`; local bare remotes | closed |
| T-02-03 | EoP | host config and hooks | high | mitigate | Global config at scenario HOME, `GIT_CONFIG_NOSYSTEM`, XDG dirs; planted-host proof + wired control | closed |
| T-02-04 | Tampering | `git apply` of oracle.patch | high | mitigate | `OracleCopy::prepare` refuses an unpatched script | closed |
| T-02-05 | DoS | scratch on /tmp | low | mitigate | TempDir only under `CARGO_TARGET_TMPDIR` | closed |
| T-02-06 | Tampering | vendored files | high | mitigate | Blob-id test with flipped-byte control | closed |
| T-02-07 | Tampering | patch application | high | mitigate | Recorded post-patch blobs; no-ceiling control | closed |
| T-02-08 | Repudiation | README provenance | low | mitigate | README drift test + control | closed |
| T-02-09 | Tampering | scenario definitions | high | mitigate | `validate` rules; `judge` fails a `fixed:` case whose oracle run meets its expectation | closed |
| T-02-10 | Repudiation | disabled Rust sides | medium | mitigate | `#[ignore]` + disabled-side report shown in every run | closed |
| T-02-11 | Tampering | guard cases seen only one way | high | mitigate | Known-positive and clean controls required by `validate` | closed |
| T-02-12 | Tampering | cargo-llvm-cov tarball | high | mitigate | sha256 pin checked before extraction; version check after | closed |
| T-02-13 | Tampering | cargo-llvm-cov installing components | medium | mitigate | `llvm-tools` in rust-toolchain.toml; runs use `CARGO_LLVM_COV_SETUP=no` | closed |
| T-02-14 | DoS | instrumented build output | low | mitigate | `target/llvm-cov-target` removed after the proof | closed |
| T-02-15 | Tampering | dead pre-push hook | high | mitigate | `guard::self_test` known-positive refused | closed |
| T-02-16 | Repudiation | recorded outcomes | high | mitigate | Every oracle test re-runs the script; record equal to expectation rejected | closed |
| T-02-17 | EoP | pre-push CI-mirror tail | medium | mitigate | `oracle.patch` replaces the tail; post-patch blob checked | closed |
| T-02-18 | Info disclosure | signing key | medium | mitigate | Fresh per-run ed25519 key in scenario root; no `SSH_AUTH_SOCK` | closed |
| T-02-19 | Tampering | comparator skipping a field | high | mitigate | Exhaustive destructuring; per-field mutation test | closed |
| T-02-20 | Spoofing | lossy name decoding | medium | mitigate | `names_are_observed_byte_exact` | closed |
| T-02-21 | EoP | host hooks | high | mitigate | Planted hook marker never created; wired control fires | closed |
| T-02-22 | Tampering | inherited `GIT_DIR` | high | mitigate | Decoy repo unchanged under planted `GIT_DIR`/`GIT_WORK_TREE` | closed |
| T-02-23 | EoP | hook's `unset` of config variables | high | mitigate | Unset test + control with global config elsewhere | closed |
| T-02-24 | Info disclosure | SSH agent | medium | mitigate | `SSH_AUTH_SOCK` absent from the hermetic env | closed |
| T-02-25 | Repudiation | recorded bypasses | high | mitigate | Re-run every time; known-positive enforced | closed |
| T-02-26 | Spoofing | special-byte names | medium | mitigate | Raw-byte names via `OsStr::from_bytes` | closed |
| T-02-27 | Repudiation | 02-11 cut outcomes | high | mitigate | Re-run every time; Rust expectations cite PRCUT-01/02/03/05/08 | closed |
| T-02-28 | Tampering | lost reviewer work | low | accept | Runs only inside a scenario TempDir (see Accepted Risks) | closed |
| T-02-29 | Repudiation | recorded sync and worktree outcomes | high | accept | Re-run in place; one oracle-observed exit code accepted (AR-02-02) | closed |
| T-02-30 | Tampering | stale workspace | low | mitigate | Base never advanced; each control changes one variable | closed |
| T-02-31 | Repudiation | HARN-06 case dropped | medium | mitigate | `harn06_gaps` completeness check + controls | closed |
| T-02-32 | DoS | leftover scratch | low | mitigate | TempDir drop; scratch audit with planted control | closed |
| T-02-33 | Tampering | vendored oracle files | high | mitigate | Touched only by 02-01; blob and mode tests | closed |
| T-02-34 | Repudiation | observer reading refs | medium | mitigate | Symbolic refs recorded by target, moved-ref control | closed |
| T-02-35 | Tampering | no-GSD and fork layouts | medium | mitigate | Layout tests with controls | closed |
| T-02-36 | Repudiation | cut outcomes in fork, squash and no-GSD | medium | accept | Re-run and one-variable counterparts in place; one oracle-observed exit code accepted (AR-02-02) | closed |
| T-02-37 | Repudiation | divergence-check outcomes | high | mitigate | Every check scenario has known-positive and clean controls; docs cite DIV-01, CONF-02, CONF-04, IF-02 | closed |
| T-02-SC | Tampering | tempfile and tool installs | high | mitigate | Cargo.lock tracked, `--locked`, `include-dev = true` with two-way control | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-02-01 | T-02-28 | The oracle's `branch -f` overwrite of an existing PR branch is reproduced only inside a scenario TempDir, never in a real checkout (02-11-PLAN.md threat model; `cut.rs`). | Planner disposition; recorded by orchestrator | 2026-10-26 |
| AR-02-02 | T-02-29, T-02-36 | The two fork-layout counterparts (`sync::fork_layout`, `cut::fork_layout`) expect exit 128, the code the oracle was observed returning when the base remote is gone. No requirement fixes a refusal exit code until IF-01 (phase 5), and plans 02-12 and 02-14 directed using the observed code. Both Rust sides are `#[ignore]`d until phases 7 and 9, so nothing compares wsx against 128 today. Condition: each value's doc comment says it is oracle-observed and is revised under IF-01; the phase that enables the Rust side must replace it with the IF-01 code before enabling. | Owner (2026-10-26, on the security gate's question) | 2026-10-26 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-26 | 38 | 36 | 2 (1 blocking) | gsd-security-auditor (ASVS L1, HEAD fa9b7b2) |
| 2026-10-26 | 38 | 38 | 0 | orchestrator, after owner accepted AR-02-02 |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-26
