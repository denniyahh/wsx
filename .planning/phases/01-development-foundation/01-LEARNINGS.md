---
phase: 01
phase_name: "Development foundation"
project: "wsx"
generated: "2026-11-17"
counts:
  decisions: 7
  lessons: 6
  patterns: 5
  surprises: 4
missing_artifacts: []
---

# Phase 01 Learnings: Development foundation

## Decisions

### Pin rustc 1.99.1, not the 1.99.0 written in DEV-02
DEV-02's rule is "the host's version"; the host was on 1.99.1 on 2026-10-22, so DEV-02 and roadmap criterion 1 were corrected to match.

**Rationale:** The requirement's intent (host parity) outranks its literal number.
**Source:** 01-CONTEXT.md

### Image pin lives in `.devcontainer/Dockerfile`, not `devcontainer.json`
`FROM mcr.microsoft.com/devcontainers/rust:<exact tag>` is referenced through `build.dockerfile`, with a Dependabot `docker` entry; the parity check reads the Dockerfile and the workflows.

**Rationale:** Research showed the `devcontainers` Dependabot ecosystem updates only Features, never `image`, so a pin in `devcontainer.json` would make image updates a silent no-op. A bump now fails CI until the workflows match (fail closed).
**Source:** 01-CONTEXT.md

### Container gate: read-only source, target volume mounted at `/workspace/target` (orchestrator Option A)
`/ctarget` and `CARGO_TARGET_DIR` were dropped; the host creates a git-ignored `target/` before `docker run`.

**Rationale:** nextest's store defaults to `target/nextest` under the workspace root and no env var moves it, so a read-only checkout exits 110. The locked constraint is a read-only source, and `target/` is git-ignored.
**Source:** 01-03-SUMMARY.md

### `.gitconfig` is copied through an allowlist, never `include.path`'d (owner decision 2026-10-24, CR-01)
`setup-repo.sh` copies allowlisted keys into local config, refuses any other key, removes the legacy include, and gained `policy` and `check` modes.

**Rationale:** An include reads the file live from any checked-out branch; a tracked `core.fsmonitor` ran on `git status` under the old include, and is refused under the copy.
**Source:** 01-REVIEW-FIX.md, 01-VERIFICATION.md

### Pin checks verify the binary cargo runs (`verify-cargo`)
`install-tools.sh verify-cargo TOOL` asks `cargo nextest --version` / `cargo deny --version`; `check.sh` no longer prepends `CARGO_HOME/bin` to PATH.

**Rationale:** Verifying the PATH copy while cargo ran another let a stub pass `deps OK` over a fake scan (CR-02, defeats T-01-07). This supersedes the earlier `CARGO_HOME/bin`-first PATH prepend of 01-02.
**Source:** 01-REVIEW-FIX.md, 01-02-SUMMARY.md

### Repo-local gitleaks hook removed from the owner's clone, kept for fresh clones (accepted risk AR-02)
The owner's global fail-closed hook scans each commit once; `setup-repo.sh all` still registers the repo hook for fresh clones.

**Rationale:** Avoids scanning twice. Recorded as a verification override accepted by the owner on 2026-10-22.
**Source:** 01-07-SUMMARY.md, 01-VERIFICATION.md

### Review findings on stand-in scripts are deferred when wsx phases 9-10 replace them
IN-02, IN-04, IN-05, IN-06 and IN-07 were deferred with reasons; stand-ins are hardened only for safety (owner decision 2026-10-23).

**Rationale:** The scripts are retired by later phases and UDE backlog 999.2; fixing them is spend on code with a short life.
**Source:** 01-REVIEW-DISPOSITION.md

---

## Lessons

### `set -e` does not catch a failing command substitution inside an argument
`echo "==> $(rustc --version)"` prints an empty line on failure. The fix assigns to a local first, then echoes.

**Context:** Found in 01-01 Task 2; the failing-rustc path itself was not exercised.
**Source:** 01-01-SUMMARY.md

### curl and tar leak their own exit codes under `set -e`
A failed curl exited 63; tar's fatal 2 collides with the usage-error code. Each call is wrapped in `if ! ...; then ...; exit 1; fi`. A `.curlrc` could also add headers, so `--disable` goes first.

**Context:** Found by the curlrc control in 01-02.
**Source:** 01-02-SUMMARY.md

### A reused target volume gives a false failure after a mount-layout change
Reusing the old volume failed on a stale `CARGO_BIN_EXE_wsx` path; old `wsx-ci-target-*` volumes must be deleted. Documented in the script and CONTRIBUTING.

**Context:** First prototype of the 01-03 read-only fix.
**Source:** 01-03-SUMMARY.md

### Plan claims must be narrowed to what was proven
CONTRIBUTING was asked to say every scanner refused a planted problem; only gitleaks (01-05) and cargo-deny (01-02) were shown to. Push protection was checked as enabled, and Dependabot is not a refusing scanner.

**Context:** 01-06 accuracy deviation.
**Source:** 01-06-SUMMARY.md

### `rg` with no path argument reads stdin and hangs in a piped agent shell
Three plan acceptance runs hung until killed by PID; with `</dev/null` they pass. Any `rg` without a path in an automated block should carry it.

**Context:** Found running 01-06's single-pin-site check.
**Source:** 01-06-SUMMARY.md

### The PR cut silently dropped code commits whose cherry-pick failed (CR-03)
An untracked file blocking a pick exited 0 with `Included: 1` and lost `src/a.rs`. A non-conflict pick failure now aborts the cut naming the commit. Related: it also force-reset any branch given (WR-04); protected branches are refused and an existing PR branch needs `--force`.

**Context:** Code review of the stand-in cut script; fixed with shell controls in a scratch repo.
**Source:** 01-REVIEW-FIX.md

---

## Patterns

### Revert-the-fix negative control
Each fix carries a control where the old code fails and the new code passes (shadow binary, curlrc, stub binary, mutant script). Where the code reaches develop the control is a Rust test; for `scripts/workspace/` it is a shell control in a scratch repo.

**When to use:** Any gate or guard change; proves the check can fail.
**Source:** 01-REVIEW-FIX.md, 01-02-SUMMARY.md

### Mutate each refusal branch and delete the whole block
A mutant that kept the error message let the test survive; deleting the entire block made `no_workflow_files_fails` fail.

**When to use:** Proving a refusal branch has a test that bites.
**Source:** 01-REVIEW-FIX.md

### Stub `docker` / stub tool earlier on PATH in fixture tests
A stub `docker` records its arguments (two fixture repos prove the script mounts its own checkout); a stub earlier on PATH proves which binary cargo runs.

**When to use:** Testing scripts that wrap external tools without installing them in CI.
**Source:** 01-REVIEW-FIX.md

### Parse with bash builtins only in CI scripts
Leaves `dirname` as the only external command, shrinking the surface where a missing tool is swallowed.

**When to use:** Gate scripts whose failure must not be masked.
**Source:** 01-04-SUMMARY.md

### Delete scratch by literal printed path, not `trap rm -rf "$VAR"`
Each control prints `S=<path>` and the directory is removed afterwards by that literal path, as the owner asked and because Claude Code's safety check refused the trap form.

**When to use:** Any scratch cleanup run by an agent.
**Source:** 01-05-SUMMARY.md, 01-02-SUMMARY.md

---

## Surprises

### cargo runs `$CARGO_HOME/bin/cargo-deny` ahead of PATH
A probe showed a fake in a scratch `CARGO_HOME/bin` won over the PATH copy, so the verified binary and the scanning binary differed.

**Impact:** Led to the 01-02 PATH-prepend fix, then to the `verify-cargo` redesign (CR-02).
**Source:** 01-02-SUMMARY.md, 01-REVIEW-FIX.md

### nextest writes its JUnit report to `target/nextest`, not under `CARGO_TARGET_DIR`
RESEARCH's "nextest passed with /workspace:ro" predated 01-01's nextest config.

**Impact:** Forced an escalated mount-interface change in 01-03.
**Source:** 01-03-SUMMARY.md

### CI checks run twice per PR head
Push and pull_request both trigger the workflow.

**Impact:** Harmless; tracked as backlog 999.1. PR #2's re-cut run showed 47 tests, 38 expected plus 9 setup-repo tests added for CR-01.
**Source:** 01-07-SUMMARY.md, 01-UAT.md

### Dependabot behaviour cannot be observed before merge to main
Dependabot reads its config only from the default branch; the contents API returned 404 for `.github/dependabot.yml` on main on 2026-10-25.

**Impact:** UAT test 2 skipped as a deferred follow-up; re-check the update log after the first release merge to main.
**Source:** 01-UAT.md, 01-VERIFICATION.md
