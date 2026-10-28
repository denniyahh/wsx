---
phase: "3"
slug: "git-adapter"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-10-27"
---

# Phase 3 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Process environment → repository resolution | `GIT_DIR`, `GIT_INDEX_FILE` and `GIT_CEILING_DIRECTORIES` decide which repository wsx opens (`Repo::discover`, the single gix open site) | Repository paths, index path |
| Git hook environment → git children | a hook's `GIT_DIR`/`GIT_INDEX_FILE` would redirect any git child that inherits them; children aimed at another worktree get git's `--local-env-vars` list removed | Repo-local git variables |
| wsx → `git` child process | argv via `Command::args` (never a shell string), explicit cwd, null stdin; the user's hooks and signing run unchanged | argv, environment, stdin |
| User's PATH → `git` binary | the version gate refuses git < 2.54 or an unreadable version | git version string |
| Repository on disk → trust decision | a repository owned by another uid is refused unless global/system `safe.directory` lists it | Repository config (hooks, `core.fsmonitor`, pager) |
| Repository config → ref backend | `extensions.refStorage` read from the repository's own config; anything but `files` is refused before a ref is read | Ref storage format |
| crates.io → build | gix `=0.90.0` (99-crate graph, no C deps), `Cargo.lock` committed, `cargo deny` sources/advisories/bans/licenses | Third-party code |
| Unit-test fixtures → wsx checkout | fixtures under `target/tmp`, `env_clear()` children with a scratch HOME; discovering tests re-run scrubbed when the runner has repo-local variables | Operator config must not cross |

---

## Threat Register

Built from the `<threat_model>` blocks of 03-01 … 03-10 (authored at plan time) and the SUMMARY threat flags (none raised new threats). Evidence is L1 (grep for the mitigation in the named file, 2026-10-27 at `1a34613`); the behavioural proof is the test listed in `03-VALIDATION.md`, all green in `scripts/check.sh all` (259 passed) and `scripts/check-in-container.sh all`.

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-03-01 | Tampering | `Repo::discover` / `open_gix` | high | mitigate | `discover_with_environment_overrides_opts` in `src/git/repo.rs`; every other gix entry point banned in `clippy.toml` | closed |
| T-03-02 | Tampering | `Git::run` argv | high | mitigate | `Command::args` with `OsStr` in `src/git/spawn.rs` | closed |
| T-03-03 | Denial of service | git child reading stdin | medium | mitigate | `Stdio::null()` in `Git::command` | closed |
| T-03-04 | Information disclosure | unit-test fixtures | medium | mitigate | `env_clear()` fixtures in `src/git/testing.rs`; scrubbed re-run under a runner `GIT_DIR` (03-10) | closed |
| T-03-05 | Tampering | gix dependency graph | high | mitigate | `gix = "=0.90.0"`, `--locked`, `cargo deny` clean (03-10 gate) | closed |
| T-03-06 | Repudiation | `to_reftable` fixture step | medium | mitigate | `git rev-parse --show-ref-format` check in `tests/harness/check.rs` | closed |
| T-03-07 | Tampering | recorded oracle outcome | medium | mitigate | `Class::Regression` judged on every gate | closed |
| T-03-08 | Tampering | read answers diverging from git | high | mitigate | differential tests with answer-changing controls in `src/git/reads.rs` | closed |
| T-03-09 | Repudiation | read failure swallowed | medium | mitigate | `Error::Gix` naming the query; only gix's "no merge-base" is `Ok(None)`, pinned against git | closed |
| T-03-10 | Elevation of privilege | second gix open site | high | mitigate | 14-entry clippy ban + `banned_gix_entry_points_refuse_a_missing_path` | closed |
| T-03-11 | Repudiation | stale ban path after a gix upgrade | medium | mitigate | per-call `#[expect(clippy::disallowed_methods)]` pin | closed |
| T-03-12 | Tampering | sibling child inherits hook index/git dir (N4) | high | mitigate | `--local-env-vars` removal; empty list fails closed (`GitEmptyOutput`, 03-07) | closed |
| T-03-13 | Elevation of privilege | helper disabling hooks/signing | high | mitigate | `child_gets_exact_argv_cwd_and_no_bypass` | closed |
| T-03-14 | DoS / Tampering | git < 2.54 or unknown version | medium | mitigate | version gate, `old_git_refused` | closed |
| T-03-15 | Repudiation | reftable opened with unreadable refs | high | mitigate | `refuse_ref_storage` + unsupported-backend backstop | closed |
| T-03-16 | Tampering | unknown ref backend treated as files | medium | mitigate | byte-exact compare, `miscased_ref_storage_is_refused_as_git_refuses_it` | closed |
| T-03-17 | Denial of service | child waiting on stdin | medium | mitigate | `children_get_null_stdin` with inherited-stdin control | closed |
| T-03-18 | Elevation of privilege | commits/merges without hooks or signature | high | mitigate | `commit_and_merge_run_hooks_and_sign` (`G`), bypass control `N` | closed |
| T-03-19 | Repudiation | re-exec parent passing over a child that never ran | medium | mitigate | marker line; `reexec_of_an_unknown_child_has_no_marker` | closed |
| T-03-20 | Tampering | sibling child from inside a hook | high | mitigate | `sibling_child_env_scrubbed` with the removal-off leak control | closed |
| T-03-21 | Tampering | sibling opened with the hook's index | high | mitigate | `Repo::siblings` via gix main-repo/worktree list | closed |
| T-03-22 | Repudiation | hook test passing without the hook running | medium | mitigate | commit exit + markers; `hook_resolves_main_worktree_with_relative_index` | closed |
| T-03-23 | Elevation of privilege | acting on a foreign-owned repository's config | high | mitigate | `refuse_untrusted` before any other check; proven against git with `safe.directory` control | closed |
| T-03-24 | Spoofing | `safe.directory` via `-c` / `GIT_CONFIG_COUNT` | low | accept | wsx stricter than git (refuses), pinned by `command_line_safe_directory_does_not_trust_a_foreign_repo` | closed (accepted, AR-03-01) |
| T-03-25 | Repudiation | foreign-owner test skipping | medium | mitigate | `ForeignOwned` guard panics naming the missing mechanism; host userns + container root paths both ran | closed |
| T-03-26 | Tampering | discovery past a user's ceiling | medium | mitigate | `refuse_at_ceiling`, pinned against git | closed |
| T-03-27 | Repudiation | phase done on host-only evidence | medium | mitigate | `scripts/check-in-container.sh all` exit 0 recorded in 03-10-SUMMARY | closed |
| T-03-SC | Tampering | package installs | high | mitigate | crates.io only (`deny.toml` sources), Package Legitimacy Audit OK, `--locked` | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-03-01 | T-03-24 | gix reads `safe.directory` only from global and system config, so a `safe.directory` given through `-c` or `GIT_CONFIG_COUNT` does not make wsx trust a foreign-owned repository while git would. Divergence is in the safe direction (wsx refuses), the refusal message prints git's own global-config fix, and the behaviour is pinned by a test. | Plan-time disposition (03-09) under the CONTEXT "Decided after research" decision (orchestrator, 2026-10-26) | 2026-10-27 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-10-27 | 28 | 28 | 0 | /gsd-secure-phase orchestrator (L1 grep evidence, ASVS 1 short-circuit; negative control confirmed the grep reports OPEN on a missing pattern) |

**Not established by this audit:** L1 is presence of the mitigation in code, not proof of its correctness; correctness rests on the tests in `03-VALIDATION.md`. The root `chown 65534` foreign-owner path has run only in the pinned container under rootless Docker on this host, not on a GitHub runner (03-RESEARCH assumption A1).

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-10-27
