---
phase: 01-development-foundation
verified: 2026-10-25T11:01:08Z
status: passed
score: 5/5 roadmap success criteria verified; 68/69 plan truths verified (1 by accepted override AR-02); 1 backstop truth insufficient_spec (Dependabot behaviour, observable only after merge to main)
covered_files: [".config/nextest.toml", ".devcontainer/Dockerfile", ".devcontainer/devcontainer.json", ".gitconfig", ".github/dependabot.yml", ".github/workflows/ci.yml", ".github/workflows/deny.yml", ".gitignore", ".planning/phases/01-development-foundation/01-01-PLAN.md", ".planning/phases/01-development-foundation/01-01-SUMMARY.md", ".planning/phases/01-development-foundation/01-02-PLAN.md", ".planning/phases/01-development-foundation/01-02-SUMMARY.md", ".planning/phases/01-development-foundation/01-03-PLAN.md", ".planning/phases/01-development-foundation/01-03-SUMMARY.md", ".planning/phases/01-development-foundation/01-04-PLAN.md", ".planning/phases/01-development-foundation/01-04-SUMMARY.md", ".planning/phases/01-development-foundation/01-05-PLAN.md", ".planning/phases/01-development-foundation/01-05-SUMMARY.md", ".planning/phases/01-development-foundation/01-06-PLAN.md", ".planning/phases/01-development-foundation/01-06-SUMMARY.md", ".planning/phases/01-development-foundation/01-07-PLAN.md", ".planning/phases/01-development-foundation/01-07-SUMMARY.md", "CLAUDE.md", "CONTRIBUTING.md", "Cargo.lock", "Cargo.toml", "clippy.toml", "deny.toml", "rust-toolchain.toml", "scripts/assert-image-parity.sh", "scripts/check-in-container.sh", "scripts/check.sh", "scripts/install-tools.sh", "scripts/setup-repo.sh", "src/main.rs", "tests/cli.rs", "tests/fixtures/env-ban/Cargo.lock", "tests/fixtures/env-ban/Cargo.toml", "tests/fixtures/env-ban/src/main.rs", "tests/gate.rs", "tests/gate_controls.rs", "tests/image_parity.rs", "tests/repo_policy.rs", "tests/setup_repo.rs"]
covered_digest: "v3:sha256:0d5b823241611bc4829ba770354047569e31dc0cdeccd563f52837aeab2b211e"
behavior_unverified: 0
overrides_applied: 1
overrides:
  - must_have: "After the scratch controls pass, setup-repo.sh has been run against this repository's real config, and `git hook list pre-commit` in the phase worktree lists `gitleaks-staged`"
    reason: "Accepted risk AR-02 (01-SECURITY.md:83): the repo-local hook was removed from the owner's clone to avoid scanning twice; the owner's global fail-closed gitleaks hook covers this clone (`git hook list pre-commit` shows `hook from hookdir`). The owner's clone runs `setup-repo.sh policy` (policy copied, `check` exits 0). Fresh clones get the repo hook through `setup-repo.sh all`, proven in a scratch repo during this verification"
    accepted_by: "owner (01-SECURITY.md AR-02)"
    accepted_at: "2026-10-22"
human_verification:
  - test: "Decide whether the Dependabot behaviour check stays open or is accepted as a deferred follow-up. If kept open: after phase 1 first reaches main, open Insights > Dependency graph > Dependabot"
    expected: "github-actions, devcontainers and docker (/.devcontainer) are each checked weekly; any PR is grouped and targets develop; a week with nothing newer opens no PR"
    why_human: "Backstop truth (01-04). Dependabot reads dependabot.yml only from the default branch, and `.github/dependabot.yml` is not on main (contents API 404 on 2026-10-25), so the behaviour cannot be observed yet. 01-UAT.md test 2 already records it as skipped (deferred follow-up), decided by the orchestrator; no tracked todo or backlog phase carries the follow-up. If the owner accepts the deferral, add an override for this truth and the status becomes passed"
---

# Phase 1: Development foundation Verification Report

**Phase Goal:** The Rust workspace builds, tests and gates the same way on the host, in the pinned container and in CI, and every supply-chain scanner is proven to bite
**Verified:** 2026-10-25T11:01:08Z on feature/phase-1 at afe4739
**Status:** human_needed
**Re-verification:** No. The previous report (2026-10-23) had no `gaps:` section, so this is a fresh initial verification that replaces it. It checks DEV-05 against the amended text (copy, not include; owner decision 2026-10-24).

Nothing was found broken. All five roadmap success criteria hold on the current tree. The one open item is the Dependabot behaviour check: it cannot be observed until the config reaches main, and nobody has formally accepted deferring it (see Human Verification).

What this verification does not establish:
- The commit f1f1303 delta (46 lines in check.sh, setup-repo.sh, assert-image-parity.sh, check-in-container.sh and tests/setup_repo.rs) has not run in CI and was not run in the real container. I was told not to run check-in-container.sh. CI proved the d8456e5 tree, which equals develop (baa95f9) and differs from HEAD only by f1f1303 and workspace-only paths. On the host the delta passes 49/49 tests. The changed root-`cd` line of check-in-container.sh is exercised by `container_gate_mounts_its_own_checkout_not_the_callers` with a stub docker, not a real `docker run`.
- I did not re-run any container success criterion (SC1 container half, 01-03 truths). That evidence is the 2026-10-23 verifier's observation on c7668e0, plus the CI Test job on d8456e5, which runs check.sh in the pinned image.

## Goal Achievement

### Roadmap success criteria

| # | Success criterion | Status | Evidence |
|---|---|---|---|
| 1 | `check.sh all` passes on the host with rustc 1.99.1 and through `check-in-container.sh` from a linked worktree; a missing tool fails, naming it | VERIFIED (container half carried, see limits) | Host, this run: `scripts/check.sh all` exit 0, `==> rustc 1.99.1`, fmt, then clippy, then nextest `--no-tests=fail`, 49 tests run, 49 passed, `==> check.sh: all OK`. Missing-tool controls are standing tests that passed in this run: `missing_cargo_fails_every_cargo_target_by_name`, `missing_rustc_…`, `missing_rustfmt_…`, `missing_clippy_…`, `missing_nextest_…`, `missing_cargo_deny_fails_deps_by_name`. Container: 2026-10-23 observation on c7668e0 (worktree gitdirs bound ro, rustc 1.99.1 against the image default 1.99.0, exit 0). CI Test on d8456e5 ran in the pinned image |
| 2 | A no-match filter fails `check.sh test`; a planted set_var fails clippy | VERIFIED | Standing tests passed in this run. `no_match_filter_fails_even_when_env_allows_no_tests` compares the gate with a bare nextest run as its negative control. `repo_clippy_toml_bans_env_mutation` has the control `env_mutation_passes_clippy_without_the_ban`. `scripts/check.sh test -E 'binary(setup_repo)'`: exit 0, 11 tests |
| 3 | CI on a PR to develop runs test, clippy and fmt in the devcontainer image; a mismatched tag fails parity; rulesets require the checks, up to date | VERIFIED | Read-only `gh`, observed directly. PR #2 is MERGED into develop, head d8456e5, merge commit baa95f9. Run 40089958293 (pull_request, d8456e5): Format, Clippy and Test success. Its Test log shows `image parity OK: mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie (4 job container image(s))`, `==> rustc 1.99.1` and 47 of 47 passed. Run 40185820734 (push, develop, baa95f9): Format, Clippy and Test success. Locally, `git diff origin/develop d8456e5` is empty. Rulesets 25445011 and 25445012 are active. Their rules are exactly pull_request, deletion, non_fast_forward and required_status_checks, with contexts Test, Clippy and Format @15368 and strict true. Parity: `assert-image-parity.sh` exit 0 on the tree. The mismatch cases are standing tests that passed, including `drifted_job_image_fails_naming_both_tags` and `non_exact_dockerfile_tag_fails` |
| 4 | Scanners bite; deny advisory on push/PR/weekly; lockfile; SHA pins; one pin site; Dependabot config | VERIFIED (config). Dependabot behaviour is a backstop item | **gitleaks**, scratch repo, empty global config, `setup-repo.sh all`: a runtime-generated token is refused (exit 1, `leaks found: 1`, HEAD unchanged). The same commit with `hook.gitleaks-staged.enabled=false` exits 0. A clean commit exits 0 (`no leaks found`). **cargo-deny**: a scratch copy with `smallvec = "=1.6.0"` gives `check.sh deps` exit 1, `RUSTSEC-2021-0003`, `advisories FAILED`. The real tree gives exit 0, `advisories ok, bans ok, licenses ok, sources ok`. A first attempt failed for the wrong reason (a duplicate `[dependencies]` table) and was discarded, not counted. Both controls lived only in scratch, now deleted. `deny.yml`: push, pull_request and cron `23 5 * * 1`, with no `continue-on-error` key. 4 of 4 `uses:` are pinned to SHA 3d3c42e5…. Versions 0.9.148/0.20.2/8.30.1 appear only in `install-tools.sh:34-38`. `dependabot.yml`: 3 ecosystems, weekly, grouped, target develop. API: alerts on (204), automatic security fixes `enabled:false` |
| 5 | CLAUDE.md and CONTRIBUTING.md carry the workflow, traps and acceptance bar; after `setup-repo.sh` copies `.gitconfig`, commits and tags are SSH-signed and git-flow names are set; nothing personal | VERIFIED | Scratch repo, global config with `commit.gpgsign=false` and `tag.gpgsign=false`. **Negative control:** before setup, the commit and tag are unsigned and gitflow.branch.master is unset. After `setup-repo.sh policy` (10 keys copied, 0 include.path) plus a scratch signing key, the commit has `gpgsig` and the tag has an SSH signature. master=`main` (od: `m a i n \n`), develop=`develop`. With no signingkey: exit 128 naming user.signingkey, HEAD unchanged. `check`: exit 1 before policy, 0 after. Exit 1 again after a local `commit.gpgsign=false`, naming the key, and 0 after re-apply. A tampered `.gitconfig` adding `core.pager` is refused with local config byte-identical. The owner's clone: `setup-repo.sh check` exit 0, no include.path. Docs: CLAUDE.md is 56 lines and carries the traps, including "never include.path". The CONTRIBUTING acceptance section has 5 requirements and 4 rejected shapes. A personal-data scan of .gitconfig, CLAUDE.md and CONTRIBUTING.md finds nothing; its known-positive sample trips 4/4 |

### Plan truths

69 plan truths, all checked.
- **68 verified**, one of them through the AR-02 override.
- **1 abstained:** the Dependabot behaviour backstop (insufficient_spec, so it is a human item).

How the checks were done:
- **Second backstop truth** (schedules start only on main): verified by the contents-API 404 for `dependabot.yml` on main.
- **01-05 truths written for the include design:** truth 1 (include signs), truth 4 (adds include.path), and the adjacency, empty and ordering edges. They were checked against the amended DEV-05 and roadmap SC5, by the scratch controls above and by standing tests that passed:
  - `policy_copies_every_tracked_key_and_adds_no_include`
  - `policy_removes_the_legacy_include_and_keeps_others`
  - `rerunning_policy_changes_nothing`
  - `key_outside_the_policy_is_refused_before_anything_is_written`
  - `policy_replaces_a_key_that_already_has_several_local_values`
  - `all_registers_the_hook_and_applies_the_policy`
- **01-07 truth 1** (container half) and the **01-03 container truths:** carried from the 2026-10-23 observation (see limits above).
- **01-07 control-artifact scan:** re-run over 101 commits since main. No 36-character `gh?_` token (the known-positive sample trips), and no `smallvec` in non-planning history.

### Required Artifacts / Key Links

`verify.artifacts`: plans 01-01 to 01-06 pass 23/23. Plan 01-07 declares none (exit 66, not applicable).

`verify.key-links` misses, each checked by behaviour instead:
- **01-02:** the check.sh → install-tools.sh link now uses `verify-cargo cargo-deny` (the CR-02 fix). The deps log shows `==> cargo deny: cargo-deny 0.20.2 OK` before the scan.
- **01-05:** the `hook.$HOOK_NAME.` link is literal-pattern drift; the scratch `git hook list pre-commit` lists `gitleaks-staged`. The include.path link was retired on purpose by the DEV-05 amendment. Its replacement (copy, legacy include removed) is verified above.
- **01-07:** the rulesets source is the API, read directly (SC3).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full gate | `scripts/check.sh all` | exit 0, 49/49, rustc 1.99.1 | PASS |
| Setup-repo tests | `scripts/check.sh test -E 'binary(setup_repo)'` | exit 0, 11/11 | PASS |
| Advisory scan, clean tree vs vulnerable copy | `scripts/check.sh deps` | 0 vs 1 (RUSTSEC-2021-0003) | PASS |
| Image parity | `scripts/assert-image-parity.sh` | exit 0, 4 images | PASS |
| Secret hook, planted vs disabled vs clean | scratch `git commit` | 1 / 0 / 0 | PASS |
| Signing after copy vs before | scratch commit and tag | signed vs unsigned | PASS |
| Scripts lint | `bash -n` + `shellcheck scripts/*.sh` | clean; all 5 tracked 100755 | PASS |

### Probe Execution

No `scripts/*/tests/probe-*.sh` exists and no plan declares one. Not applicable.

### Requirements Coverage

| Req | Status | Evidence |
|---|---|---|
| DEV-01 | SATISFIED | check.sh gate; missing-tool tests; `all` order fmt, clippy, test |
| DEV-02 | SATISFIED | `==> rustc 1.99.1` on host and in CI; `rustc_at_repo_root_is_the_pinned_channel` |
| DEV-03 | SATISFIED | CLAUDE.md traps section |
| DEV-04 | SATISFIED | CONTRIBUTING.md 5+4 items |
| DEV-05 (amended 2026-10-24) | SATISFIED | copy-based setup-repo.sh: allowlist, `all\|policy\|check`, never includes; scratch signing controls; `check` reports drift |
| DEV-06 | SATISFIED | Dockerfile pin; parity; CI in image; container run carried from 2026-10-23 |
| SUP-01 | SATISFIED | deny.yml triggers, advisory, smallvec control |
| SUP-02 | SATISFIED (config) | dependabot.yml; API settings; behaviour is the backstop item |
| SUP-03 | SATISFIED | scratch hook registration and refusal |
| SUP-04 | SATISFIED | Cargo.lock tracked; SHA pins; single pin site |
| SUP-05 | SATISFIED | both controls re-run and removed |
| HARN-04 | SATISFIED | `--no-tests=fail`; clippy.toml ban; tests above |
| REL-04 | SATISFIED | CI runs and rulesets read via API |

All 13 phase-1 IDs appear in plan frontmatter. No orphans.

### Prohibitions

Nothing was found broken. These are sign-offs the GSD policy requires, answered from evidence:
- **01-02, no identifying data in downloads (judgment):** settled by code. `install-tools.sh:151` uses `curl --disable` with no header, user-agent or auth option (no matches for `-H`, `-A` or `-u`).
- **01-07, only the owner pushes or changes settings (judgment):** resolved in 01-UAT.md test 3, from the conversation record.
- **Test-tier prohibitions:**
  - 01-03 read-only mounts: covered by a standing test asserting `:/workspace:ro`.
  - 01-04 no failure tolerance: no `continue-on-error` key.
  - 01-05 no personal data in .gitconfig: covered by `gitconfig_policy_check_rejects_unsigned_and_personal_config`.
  - 01-06 no personal data in docs: covered by the scan with a positive control.
  - 01-07 no control artifacts in history: covered by the history scan with a positive control.
  - Limit: 01-06 and 01-07 rest on this verifier's one-off scans. No standing test guards them.

### Anti-Patterns

No TBD, FIXME, XXX, TODO or HACK in the files changed since the last verification (the scan's known-positive sample trips). Code review disposition: 13 findings, 0 open. Five of them are deferred with recorded reasons, all info-level gaps in stand-in scripts or tests.

### Human Verification Required

### 1. Dependabot behaviour after merge to main

**Test:** Decide whether this stays open or is accepted as deferred. If it stays open, after the first merge to main, open Insights > Dependency graph > Dependabot.
**Expected:** The three ecosystems are checked weekly; PRs are grouped and target develop; a quiet week opens none.
**Why human:** This cannot be observed before main. 01-UAT.md already marks it skipped (deferred), but the orchestrator decided that, and no todo or backlog phase tracks the re-check. If the owner accepts the deferral, an override for this truth turns the status to passed.

### Gaps Summary

There are no gaps. The phase goal is achieved on the evidence above. The CI and container evidence for the last 46 lines (f1f1303) arrives when the next PR is cut, since develop does not yet carry it.

---

_Verified: 2026-10-25T11:01:08Z_
_Verifier: Claude (gsd-verifier)_
