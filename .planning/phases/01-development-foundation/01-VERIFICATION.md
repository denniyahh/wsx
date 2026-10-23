---
phase: 01-development-foundation
verified: 2026-10-23T20:50:00Z
status: human_needed
score: 5/5 roadmap success criteria verified; 67/69 plan truths verified (1 by accepted override AR-02), 1 backstop truth insufficient_spec, 1 CI-on-current-tree item awaiting the owner's PR update
covered_files: [".config/nextest.toml", ".devcontainer/Dockerfile", ".devcontainer/devcontainer.json", ".gitconfig", ".github/dependabot.yml", ".github/workflows/ci.yml", ".github/workflows/deny.yml", ".gitignore", ".planning/phases/01-development-foundation/01-01-PLAN.md", ".planning/phases/01-development-foundation/01-01-SUMMARY.md", ".planning/phases/01-development-foundation/01-02-PLAN.md", ".planning/phases/01-development-foundation/01-02-SUMMARY.md", ".planning/phases/01-development-foundation/01-03-PLAN.md", ".planning/phases/01-development-foundation/01-03-SUMMARY.md", ".planning/phases/01-development-foundation/01-04-PLAN.md", ".planning/phases/01-development-foundation/01-04-SUMMARY.md", ".planning/phases/01-development-foundation/01-05-PLAN.md", ".planning/phases/01-development-foundation/01-05-SUMMARY.md", ".planning/phases/01-development-foundation/01-06-PLAN.md", ".planning/phases/01-development-foundation/01-06-SUMMARY.md", ".planning/phases/01-development-foundation/01-07-PLAN.md", ".planning/phases/01-development-foundation/01-07-SUMMARY.md", "CLAUDE.md", "CONTRIBUTING.md", "Cargo.lock", "Cargo.toml", "clippy.toml", "deny.toml", "rust-toolchain.toml", "scripts/assert-image-parity.sh", "scripts/check-in-container.sh", "scripts/check.sh", "scripts/install-tools.sh", "scripts/setup-repo.sh", "src/main.rs", "tests/cli.rs", "tests/fixtures/env-ban/Cargo.lock", "tests/fixtures/env-ban/Cargo.toml", "tests/fixtures/env-ban/src/main.rs", "tests/gate.rs", "tests/gate_controls.rs", "tests/image_parity.rs", "tests/repo_policy.rs"]
covered_digest: "v3:sha256:6d3a4763d9e86808a5cbb226c4c3453b857b8ccd8c77306c51bb771b9a0073da"
behavior_unverified: 0
overrides_applied: 1
overrides:
  - must_have: "After the scratch controls pass, setup-repo.sh has been run against this repository's real config, and `git hook list pre-commit` in the phase worktree lists `gitleaks-staged`"
    reason: "Accepted risk AR-02 (01-SECURITY.md:82): the repo-local hook was removed from the owner's clone to avoid scanning twice; the owner's global fail-closed gitleaks hook covers this clone; fresh clones still get the repo hook through scripts/setup-repo.sh, proven in a scratch repo during this verification"
    accepted_by: "owner (01-SECURITY.md AR-02)"
    accepted_at: "2026-10-22"
human_verification:
  - test: "After the owner re-cuts and pushes PR #2 so it carries the commits after 34e1b54 (1da1794 tests and the code-review fixes d179d7a..facacf8), run `gh pr checks 2 --repo denniyahh/wsx`"
    expected: "Test, Clippy and Format (app 15368) conclude success on the new head SHA, and the Test job log shows `image parity OK ... (4 job container image(s))`, `==> rustc 1.99.1` and 38 tests run"
    why_human: "Only the owner may push. CI has run only on 34e1b54; the current tree was proven on the host and in the pinned image, not yet in CI. Workflows are unchanged since 34e1b54, check.sh, install-tools.sh, the parity script and the tests are not"
  - test: "Once phase 1 reaches main, open the repository's Dependabot update logs (Insights > Dependency graph > Dependabot)"
    expected: "github-actions, devcontainers and docker (/.devcontainer) are each checked weekly; any PR is grouped and targets develop; a week with nothing newer opens no PR"
    why_human: "Backstop truth (01-04): Dependabot reads dependabot.yml only from the default branch, so its behaviour cannot be observed before merge to main. The configuration itself is verified"
  - test: "Policy sign-off for the 01-07 judgment-tier rule (nothing was found wrong): confirm that the owner, not an executor agent, pushed feature/phase-1-pr, opened PR #2 and added the required checks to rulesets 25445011 and 25445012 at 19:48Z on 2026-10-23"
    expected: "Owner confirms"
    why_human: "Every GitHub action ran under the same account, and the ruleset API records no actor, so the API cannot attribute who acted"
---

# Phase 1: Development foundation Verification Report

**Phase Goal:** The Rust workspace builds, tests and gates the same way on the host, in the pinned container and in CI, and every supply-chain scanner is proven to bite
**Verified:** 2026-10-23T20:50:00Z
**Status:** human_needed
**Re-verification:** No (initial verification)

Nothing was found broken. Every control below was re-run by this verifier in its own process, on the current tree (c7668e0), with a negative control beside each positive one. The status is human_needed for three reasons. CI has not yet run on the current tree. Dependabot behaviour can only be seen after merge to main. One judgment-tier policy sign-off needs a fact only the owner knows.

## Goal Achievement

### Roadmap success criteria

| # | Success criterion | Status | Evidence (re-run by verifier) |
|---|---|---|---|
| 1 | `check.sh all` passes on host with rustc 1.99.1 and through `check-in-container.sh` from a linked worktree; a missing tool fails, naming the tool | VERIFIED | Host: `scripts/check.sh all` exit 0, `==> rustc 1.99.1`, 38/38 tests. Container (from `.worktrees/phase-1`): both gitdir components bound ro, `==> rustc 1.99.1` against an image default of `rustc 1.99.0`, 38/38, exit 0, no CR, no `command not found`; `deps` in container exit 0. Minimal PATH: fmt/clippy/test/deps each exit 1 naming `cargo`. With a shim PATH holding cargo/rustc/fmt/clippy: `test` names `cargo-nextest`, `deps` names `cargo-deny`, and `fmt` passes (positive control) |
| 2 | No-match filter fails `check.sh test`; planted set_var fails clippy | VERIFIED | `NEXTEST_NO_TESTS=pass check.sh test -E 'test(=no_such_test)'` exit 4 versus a bare `cargo nextest run` exit 0. `--no-tests=pass` exit 2. A planted `unsafe { std::env::set_var(..) }` in `#[cfg(test)]` (scratch copy) gives clippy exit 101 with "disallowed method `std::env::set_var`"; with clippy.toml removed, the same plant passes |
| 3 | CI on a PR to develop runs test/clippy/fmt in the devcontainer image; a mismatched tag fails parity; rulesets require the checks, up to date | VERIFIED (for PR head 34e1b54) | PR #2 (feature/phase-1-pr to develop): Test/Clippy/Format/cargo deny success, app 15368, both `push` and `pull_request` runs on 34e1b54. The Test log shows the parity OK line for `rust:2.1.3-1-trixie` and `==> rustc 1.99.1`. Parity in a scratch copy: Dockerfile-only bump exit 1, naming all 4 jobs and both tags; all bumped exit 0; floating tag exit 1. Rulesets 25445011/25445012: rules are pull_request, deletion, non_fast_forward, required_status_checks; contexts Test, Clippy, Format with integration_id 15368; strict true; updated_at 19:48:06Z / 19:48:07Z, after the last check finished at 19:46:59Z. The rules endpoint for develop and main lists required_status_checks |
| 4 | Scanners bite; deny advisory on push/PR/weekly; lockfile; SHA pins; one pin site; Dependabot | VERIFIED | Gitleaks: a scratch repo with an empty global config, set up by `setup-repo.sh`. A runtime token is refused (`leaks found: 1`, HEAD unchanged). With `hook.gitleaks-staged.enabled=false` it goes through. A clean commit passes. cargo-deny: scratch copy with `smallvec =1.6.0` exit 1, `RUSTSEC-2021-0003`, `advisories FAILED`; tree exit 0. Both controls lived only in scratch, now removed. `deny.yml:20-25` push/PR/cron, no continue-on-error. `Cargo.lock` tracked, not ignored. All 4 `uses:` are pinned to the 40-hex SHA 3d3c42e5…. Versions 0.9.148/0.20.2/8.30.1 appear only at `install-tools.sh:34-38`. The bad-sha install is refused with nothing left behind, and the good-sha install succeeds. `dependabot.yml`: 3 entries, weekly, grouped, target develop, no cargo. Alerts on (204), automated fixes `enabled:false` |
| 5 | CLAUDE.md/CONTRIBUTING.md carry the Rust workflow, traps and acceptance bar; the include gives signing and git-flow names; .gitconfig impersonal | VERIFIED | `CLAUDE.md:28-35` traps (nextest `=`, `rg -c`, PIPESTATUS, named test). `CONTRIBUTING.md:137-164` has 5 requirements and 4 rejected shapes. Every `scripts/*.sh` it names exists. Scratch repo with the include: commit and annotated tag SSH-signed even with global gpgsign=false. Without the include the commit is unsigned. With no signingkey the commit is refused (fatal names user.signingkey) and HEAD is unchanged. gitflow master/develop = `main`/`develop`. `git config -f .gitconfig --list` shows exactly the 10 keys. The personal-data scan of .gitconfig, CLAUDE.md and CONTRIBUTING.md finds nothing; its known-positive sample does trip |

Plan truths: all 69 checked; 67 verified (one via the AR-02 override). Commit-literal drift was judged on behaviour. The parity message now reads "job container image(s)". `check.sh deps` now uses `verify-cargo cargo-deny`, and a fake `cargo-deny 0.19.0` first on PATH exits 1 with no `cargo deny --locked check` line. The 01-07 history scan was re-run in adapted form. It finds no `gh?_` 36-char token in the 81 phase commits (known-positive sample trips). The only control-shaped strings are the intentional repeatable fixtures from 1da1794: `tests/image_parity.rs` DRIFTED `2.1.2-1-trixie` and `tests/fixtures/env-ban/src/main.rs:7`. No `smallvec` appears anywhere.

### Required Artifacts / Key Links

`verify.artifacts`: 01-01..01-06 all pass (24/24); 01-07 declares none (exit 66). `verify.key-links` misses are literal-pattern or non-file-source misses, each confirmed by behaviour. 01-02 now uses `verify-cargo` (CR-02). 01-05 setup-repo uses `hook.$HOOK_NAME.` (behaviour proven in scratch). The `.git/config` and ruleset sources are not files, and both were checked by API or git.

### Requirements Coverage

| Req | Status | Evidence |
|---|---|---|
| DEV-01 | SATISFIED | scripts/check.sh:62-189; controls above |
| DEV-02 | SATISFIED | rust-toolchain.toml:12-13; `==> rustc 1.99.1` host, container, CI |
| DEV-03 | SATISFIED | CLAUDE.md:28-35 |
| DEV-04 | SATISFIED | CONTRIBUTING.md:137-164 |
| DEV-05 | SATISFIED | .gitconfig:21-51; scratch signing controls |
| DEV-06 | SATISFIED | .devcontainer/Dockerfile:14; check-in-container.sh:78-147; ci.yml:54,78,93; assert-image-parity.sh |
| SUP-01 | SATISFIED | deny.yml:20-55; deny.toml; smallvec control |
| SUP-02 | SATISFIED (config) | dependabot.yml:11-45; API alerts on, fixes off; behaviour is a backstop item |
| SUP-03 | SATISFIED | setup-repo.sh:69-91; scratch hook coexists with hooksPath; refuses git 2.53 and missing gitleaks |
| SUP-04 | SATISFIED | Cargo.lock tracked; SHA pins; install-tools.sh:34-39 sole pin site |
| SUP-05 | SATISFIED | gitleaks and cargo-deny controls re-run and cleaned |
| HARN-04 | SATISFIED | check.sh:151 `--no-tests=fail`; clippy.toml:19-22; junit at target/nextest/default/junit.xml |
| REL-04 | SATISFIED | PR #2 CI and rulesets above |

No orphaned requirements: all 13 phase-1 IDs appear in plan frontmatter.

### Anti-Patterns

No TBD/FIXME/XXX/TODO/HACK in the 33 phase-changed files. Every script under `scripts/*.sh` passes `bash -n` and shellcheck, and all are tracked 100755.

### Needs awareness (not gaps)

- PR #2 head 34e1b54 does not carry 1da1794 or the code-review fixes (13 files differ outside workspace-only paths). The PR's CI proves the pre-fix tree only (7 tests).
- In the owner's clone, `include.path ../.gitconfig` resolves to `/var/home/denniyahh/Github/wsx/.gitconfig`, which does not exist on the checked-out `workspace/denniyahh`. So the tracked policy is not yet live there: `gitflow.branch.master` is unset, and signing comes from the global config. This is documented at `.gitconfig:11-14`. It bears on the open owner decision CR-01 (live include versus one-time copy), which this report does not resolve.
- Prohibition sign-off for 01-02 (no contact details sent on download) is settled by code: `install-tools.sh:151-152` uses `curl --disable` with no header, user-agent or auth option.

---

_Verified: 2026-10-23T20:50:00Z_
_Verifier: Claude (gsd-verifier)_
