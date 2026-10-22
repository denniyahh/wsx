---
phase: 01-development-foundation
plan: 04
subsystem: infra
tags: [github-actions, ci, dependabot, cargo-deny, devcontainer, image-parity, bash]

requires:
  - phase: 01-01
    provides: "scripts/check.sh {fmt,clippy,test,all}"
  - phase: 01-02
    provides: "scripts/install-tools.sh (sha256-pinned cargo-nextest, cargo-deny), scripts/check.sh deps"
  - phase: 01-03
    provides: ".devcontainer/Dockerfile, the single image pin FROM mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie"
provides:
  - "scripts/assert-image-parity.sh: no-argument check that the Dockerfile FROM pin equals every workflow `image:`; exit 1 on mismatch, non-exact tag, no workflow files, zero images or a job without a container image; exit 2 on any argument"
  - ".github/workflows/ci.yml: workflow CI, jobs test/Test, clippy/Clippy, format/Format in the pinned image, push + PRs to main/develop, no path filters"
  - ".github/workflows/deny.yml: workflow Dependency audit, job deny/`cargo deny (advisory)`, push + PRs + cron `23 5 * * 1`, no failure tolerance"
  - ".github/dependabot.yml: weekly grouped github-actions (/), devcontainers (/), docker (/.devcontainer) updates against develop; no cargo entry"
affects: [01-06 docs (CONTRIBUTING/CLAUDE.md describe CI and the parity check), 01-07 owner-gated first CI run and ruleset contexts Test/Clippy/Format]

actuals:
  tokens: 2959
  tasks: 2
  commits: 2
plan_head_before: 8558779bfc5e042f1a8e9c2571aeac079705694d
plan_head_after: 93fc00f3cc894d54476f22ac269fa30787aaea66

tech-stack:
  added: [actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1 (v7.0.1)]
  patterns:
    - "Every workflow job runs in `container: image:` equal to the Dockerfile FROM pin; the parity script fails closed on any job without one"
    - "Shell that CI runs inside the pinned image parses with bash builtins only, and is proven in the image, not just on the host"
    - "Advisory jobs are not required contexts but still go red on findings (no continue-on-error)"

key-files:
  created:
    - scripts/assert-image-parity.sh
    - .github/workflows/ci.yml
    - .github/workflows/deny.yml
    - .github/dependabot.yml
  modified: []

key-decisions:
  - "assert-image-parity.sh parses the Dockerfile and workflows with bash builtins only (no grep/sed/awk); `dirname` is its one external command, so a tool the image lacks cannot be silently swallowed"
  - "Steps that run scripts/check.sh carry no step `name:`, so no step is named Test/Clippy/Format beside the job names that become ruleset contexts"

patterns-established:
  - "A Dependabot docker PR that bumps only the Dockerfile is red until its workflow images are updated in the same PR (by design)"

requirements-completed: [DEV-06, SUP-01, SUP-02, SUP-04, REL-04]

coverage:
  - id: D1
    description: "assert-image-parity.sh passes on the clean tree and fails with its message on every drift shape (drifted ci.yml naming both tags, Dockerfile-only bump, floating tag, no workflow files, job without a container), on the host and inside the pinned image"
    requirement: DEV-06
    verification:
      - kind: other
        ref: "Task 1 verify #1, #4, #5 (host C7) and #6 (in-image C7 + planted rg control), run verbatim from the plan; all exit 0"
        status: pass
      - kind: other
        ref: "Final-tree rerun of every case on host and in image: clean OK (4 workflow image(s)), each failure case rc=1 in both, planted rg host rc=0 / image rc=127"
        status: pass
    human_judgment: false
  - id: D2
    description: "ci.yml has jobs named exactly Test, Clippy, Format in the pinned image, contents: read, shell bash, push + PRs to main/develop, no path filters, SHA-pinned checkout with persist-credentials false, no github.event interpolation, and the Test job runs parity, nextest install and check.sh test"
    requirement: REL-04
    verification:
      - kind: other
        ref: "Task 1 verify #2 (ruby structure check) -> `ci.yml structure OK`; negative control: a copy with Format renamed to Fmt aborts `job names`"
        status: pass
      - kind: other
        ref: "Task 1 verify #3: gh api repos/actions/checkout/git/ref/tags/v7.0.1 -> commit 3d3c42e5aac5ba805825da76410c181273ba90b1, and ci.yml carries it with `# v7.0.1`"
        status: pass
    human_judgment: false
  - id: D3
    description: "deny.yml runs check.sh deps in the pinned image on push, PRs and weekly, is not a required context and tolerates no failure; dependabot.yml has exactly the three weekly grouped entries against develop"
    requirement: SUP-01
    verification:
      - kind: other
        ref: "Task 2 verify #1 (4 workflow image(s)), #2 (`deny.yml and dependabot.yml structure OK`), #3 (drifted deny.yml rc!=0 naming deny.yml); negative controls: continue-on-error -> `tolerates failure`, added cargo entry -> `ecosystems`, job named Test -> `name collides with a required context`"
        status: pass
    human_judgment: false
  - id: D4
    description: "Dependabot alerts on and automatic security-fix PRs off (read-only API)"
    requirement: SUP-02
    verification:
      - kind: other
        ref: "Task 2 verify #4: automated-security-fixes .enabled = false; vulnerability-alerts --silent exit 0"
        status: pass
    human_judgment: false
  - id: D5
    description: "GitHub accepts and runs these workflows, and the weekly cron and Dependabot act once released to main"
    requirement: REL-04
    verification: []
    human_judgment: true
    rationale: "Needs the owner-gated first push/PR (01-07) and a release to main; nothing local can exercise GitHub's runner or Dependabot"

duration: 4min
completed: 2026-10-22
status: complete
---

# Phase 1 Plan 04: CI in the pinned image, image parity check, advisory cargo-deny and Dependabot Summary

**CI now has `Test`, `Clippy` and `Format` jobs, each running `scripts/check.sh` in `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`. `scripts/assert-image-parity.sh` takes no argument. It fails the Test job whenever any workflow image differs from the Dockerfile pin, the pin is not an exact tag, or a job has no container. It is proven on the host and inside the pinned image. Alongside: an advisory weekly cargo-deny workflow and Dependabot updates for actions, Features and the image, all targeting `develop`.**

## What these results do and do not establish

- **Nothing here proves GitHub accepts and runs these workflows.** That needs the owner-gated first PR run in 01-07. The YAML was checked with Ruby's parser and structure assertions. actionlint is not installed, so GitHub's own schema validation has not been applied.
- **The weekly cargo-deny cron and Dependabot take effect only once these files reach `main`,** the default branch. This plan verified their configuration, not a scheduled run or a Dependabot PR.
- **No repository setting was changed.** The only GitHub API calls were read-only GETs:
  - the checkout tag;
  - `automated-security-fixes`, which returned `false`;
  - `vulnerability-alerts`, which returned 204.
- **Parity was proven in the pinned image under local rootless Docker.** GitHub's container job adds its own mounts, user and environment. The image is the same; the runtime is not identical.
- That a failing step fails the CI job relies on GitHub's standard step semantics under `shell: bash`. This was not exercised.
- **The planted control proves one specific thing:** the in-image run catches one missing tool when it is called directly. A missing tool whose failure is swallowed would be caught only if it changed a C7 result or printed `command not found`. The script calls only `dirname` externally, directly, which keeps that surface small.
- **Each control ran once on each side,** twice in total for the parity cases (at Task 1 and on the final tree). That shows each guard can fire. It is not a reliability measurement.
- **Known limit of the parity check:** it cannot tell a "valid" but less specific tag from an intended one beyond the exact-tag regex. Dependabot docker bumps still need a human review of the tag (dependabot-core#15530). The dependabot.yml comment says so.

## Performance

- **Duration:** about 4 min (20:12:05Z to about 20:16Z). Each in-image run takes about 1 s; the image was already pulled.
- **Started:** 2026-10-22T20:12:05Z
- **Completed:** 2026-10-22T20:16Z
- **Tasks:** 2 of 2
- **Files modified:** 4 created

## Accomplishments

- `scripts/assert-image-parity.sh` (mode 100755; `shellcheck` and `bash -n` clean):
  - reads the first `FROM` line exactly as check-in-container.sh does;
  - enforces `^[^[:space:]:@]+:[0-9]+\.[0-9]+\.[0-9]+-[0-9]+-[a-z]+$`;
  - collects every `image:` line and counts every `runs-on:` line in `.github/workflows/*.yml|*.yaml`, with nullglob;
  - fails closed on each drift shape;
  - parses with bash builtins only.
- `.github/workflows/ci.yml`:
  - the ruleset-trap header, with `gh api repos/denniyahh/wsx/rules/branches/<branch>`;
  - `contents: read`, a concurrency group, and workflow-level `shell: bash` with devflow's dash reason;
  - three container jobs, each with the SHA-pinned checkout (`persist-credentials: false`) and the safe.directory step;
  - Test runs parity, then `install-tools.sh install cargo-nextest`, then `check.sh test`.
- `.github/workflows/deny.yml`:
  - an ADVISORY header: not in any ruleset, and red on findings;
  - cron `23 5 * * 1`, which runs only from main;
  - job `cargo deny (advisory)` with no `continue-on-error` and no exit swallowing.
- `.github/dependabot.yml`: three weekly grouped entries, each with `target-branch: develop`, and no cargo entry. It explains why automatic security-fix PRs stay off.

## Task Commits

1. **Task 1: assert-image-parity.sh and ci.yml (Test, Clippy, Format)** - `8616085` (feat). The commit was gated on shellcheck, `bash -n`, the host parity run, the in-image OK line with no `command not found`, and `scripts/check.sh all`.
2. **Task 2: advisory deny.yml and dependabot.yml** - `93fc00f` (feat). The commit was gated on `scripts/assert-image-parity.sh && scripts/check.sh all`.

**Plan metadata:** recorded in the docs commit that adds this SUMMARY.

## Control evidence (quoted from the runs)

**Clean pass, on the host and in the image (Task 1 tree):**
```
image parity OK: mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie (3 workflow image(s))
```
On the final tree the same line reads `(4 workflow image(s))`, on the host and in the image.

**C7 failure messages. Every case exited 1 on the host AND 1 inside the pinned image, with identical output.** The first run, at Task 1, printed the pin line as `.devcontainer/Dockerfile : ...`. I then relabelled it `(the pin):`, and the plan's verify commands were rerun after that edit.
- Drifted `ci.yml`, naming the file and both tags:
  ```
  error: CI image does not match the devcontainer pin.

    .devcontainer/Dockerfile : mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie
    .github/workflows/ci.yml: mcr.microsoft.com/devcontainers/rust:2.1.2-1-trixie

  These must be identical or local checks stop predicting CI. Update every copy in the same change.
  ```
- Dockerfile-only bump, the shape of a Dependabot docker PR. The pin line reads `...rust:2.1.4-1-trixie`, followed by one `.github/workflows/ci.yml: ...rust:2.1.3-1-trixie` line per job. On the final tree a `deny.yml` line is listed as well.
- Floating tag: `error: .devcontainer/Dockerfile pins 'mcr.microsoft.com/devcontainers/rust:2-1-trixie', which is not an exact version tag`
- No workflow files: `error: no workflow files found in .github/workflows; nothing to compare with mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`
- Job without a container: `error: .github/workflows/ci.yml has 4 job(s) but only 3 container image(s); every CI job must run in the pinned image`
- Drifted `deny.yml` (Task 2 verify #3): `.github/workflows/deny.yml: mcr.microsoft.com/devcontainers/rust:2.1.2-1-trixie`, exit 1.

**Planted control (`rg --version >/dev/null` inserted after `set -euo pipefail`):**
- host: exit **0**, `image parity OK: ... (3 workflow image(s))`, because rg is present on the host;
- in the image: exit **127**, `scripts/assert-image-parity.sh: line 26: rg: command not found`.

**Argument:** `scripts/assert-image-parity.sh extra-arg` exits **2** with `usage: scripts/assert-image-parity.sh   (takes no arguments)`.

**Negative controls on the verify commands themselves:**
- The host C7 commands (Task 1 verify #4 and #5), run against an always-pass stub script, both exit 1.
- The ci.yml structure check aborts `job names` on a copy with `Format` renamed to `Fmt`.
- The deny/dependabot structure check aborts as follows:
  - `tolerates failure` for a copy with `continue-on-error: true`;
  - `ecosystems` for a copy with an added cargo entry;
  - `name collides with a required context` for a copy whose deny job is named `Test`.
- Baseline at the plan's start: `.github/` was absent, and the ci.yml structure check failed with `No such file or directory` (exit 1).

**API facts (read-only GETs, 2026-10-22):**
- `repos/actions/checkout/git/ref/tags/v7.0.1` returned `commit 3d3c42e5aac5ba805825da76410c181273ba90b1`.
- `automated-security-fixes` `.enabled` returned `false`.
- `vulnerability-alerts --silent` exited 0 (alerts on).

## Files Created/Modified

- `scripts/assert-image-parity.sh`: the image parity check that CI's Test job runs.
- `.github/workflows/ci.yml`: the required CI jobs Test, Clippy and Format.
- `.github/workflows/deny.yml`: the advisory cargo-deny audit, run on push, on PRs and weekly.
- `.github/dependabot.yml`: weekly grouped version updates against develop.

## Decisions Made

- **Parsing uses bash builtins only.** The plan allowed grep, sed and mawk, but a builtin parser leaves `dirname` as the script's only external command. That shrinks the swallowed-missing-tool surface that the plan's Tools bullet worries about.
- **The `check.sh` steps have no step `name:`,** as in devflow. No step is then labelled Test, Clippy or Format beside the job names that become ruleset contexts.
- **devflow's `options: --init` was not ported.** The plan and RESEARCH Pattern 5 do not include it, and Phase 1's tests spawn no long-lived children.

## Deviations from Plan

None. The plan was executed as written.

## Issues Encountered

None.

## Cleanup

- The scratch directory `/tmp/claude-1000/-var-home-denniyahh-Github-wsx/97fae308-abb4-46c4-af94-89ae86614ec1/scratchpad/exec-01-04/` was removed by explicit paths and confirmed gone.
- No containers remain, because every in-image run used `--rm` with read-only binds. `docker ps -a` lists 0 containers.
- No volumes were created. The two existing per-checkout volumes `wsx-ci-target-4fa63246f2d5` and `wsx-ci-registry-4fa63246f2d5` were not touched.

## User Setup Required

None.

## Next Phase Readiness

- 01-06 docs can describe CI, the parity check, the advisory deny workflow and Dependabot.
- 01-07's ruleset contexts are exactly `Test`, `Clippy` and `Format`. They must match these job `name:` values byte for byte.
- The first real CI run is owner-gated (01-07). Nothing has been pushed.

## Self-Check: PASSED

- All four files exist and are tracked; `scripts/assert-image-parity.sh` is mode 100755.
- Commits `8616085` and `93fc00f` are ancestors of HEAD. `git rev-list --count 8558779..HEAD` was 2 before this docs commit.
- All Task 1 and Task 2 verify commands passed verbatim, and the acceptance criteria hold, as quoted above.

---
*Phase: 01-development-foundation*
*Completed: 2026-10-22*
