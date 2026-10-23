---
phase: 01-development-foundation
plan: 07
subsystem: infra
tags: [ci, github-actions, rulesets, required-checks, pr-cut, supply-chain, docker, verification]

requires:
  - phase: 01-development-foundation
    provides: "scripts/check.sh, install-tools.sh, check-in-container.sh, assert-image-parity.sh, setup-repo.sh (01-01..01-05), CI Test/Clippy/Format jobs and the advisory deny workflow (01-04), the Rust CLAUDE.md and CONTRIBUTING.md (01-06)"
provides:
  - "Evidence that every local gate passes on the final tree, on the host and in the pinned image"
  - "A rehearsed, then owner-run, PR cut: PR #2 (feature/phase-1-pr -> develop) with no personal or planning paths"
  - "The first CI run: Test, Clippy and Format green from GitHub Actions on PR #2's head"
  - "Rulesets 25445011 (develop) and 25445012 (main) requiring Test, Clippy and Format from app 15368, strict"
affects: [phase 2 onwards (every PR to develop or main now needs the three checks), release to main (weekly cron and Dependabot start there)]

actuals:
  tokens: 6640
  tasks: 3
  commits: 8
plan_head_before: e067e9039e3bf66bdeaaa3beab3364e09eca2ee3
plan_head_after: d99f325ccab4497e23459065f01fe5c88db23e83

tech-stack:
  added: []
  patterns:
    - "Rehearse an outward-facing step in a scratch clone with an isolated git config before the owner runs it"
    - "Dry-run a ruleset PUT body with GET plus jq, with controls that the current ruleset fails the check and that the program is idempotent"

key-files:
  created:
    - .planning/phases/01-development-foundation/01-07-SUMMARY.md
  modified: []

key-decisions:
  - "Task 1's evidence went into a separate 01-07-TASK1-EVIDENCE.md before the checkpoint, because a 01-07-SUMMARY.md on disk marks the plan finished; it is folded into this SUMMARY and removed in the same commit"
  - "Task 1's gate block was re-run after the checkpoint, because three fix commits landed after it; the parity OK string in the plan's check was updated to the script's new wording, `(4 job container image(s))`"
  - "Owner-approved local exception: the repo-local gitleaks hook was removed from this repository's .git/config; secret scanning on this machine runs once, through the owner's global hook"

patterns-established:
  - "A plan check that quotes a script's output literally goes stale when the script changes; Task 3's prefix match survived the parity script's rewording, Task 1's full-line match did not"

requirements-completed: [REL-04, DEV-06, SUP-05]

coverage:
  - id: D1
    description: "On the final tree, check.sh all and deps pass on the host and in the pinned image, image parity passes on the host and in the image, no `command not found`, the five scripts pass shellcheck and bash -n and are tracked 100755"
    requirement: DEV-06
    verification:
      - kind: other
        ref: "Task 1 verify block 1, re-run 2026-10-23 on d99f325 with the parity string `(4 job container image(s))` (exit 0); the plan's literal `(4 workflow image(s))` no longer matches after fe762f4"
        status: pass
    human_judgment: false
  - id: D2
    description: "The PR branch carries exactly the phase's non-planning, non-workspace changes, identical content except .gitignore, which has /target/ and no workspace block"
    requirement: REL-04
    verification:
      - kind: other
        ref: "Task 1 verify block 2 (scratch-clone rehearsal, exit 0, with negative controls) and the same assertions against the pushed head 34e1b54 (23 paths, .gitignore differs as the control requires)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Test, Clippy and Format check runs from GitHub Actions (app 15368) concluded success on PR #2's current head, and the Test job log shows the parity OK line and rustc 1.99.1"
    requirement: DEV-06
    verification:
      - kind: other
        ref: "Task 3 verify blocks 1 and 2 (exit 0) on head 34e1b54d224a0923377a44faa5736e80c6937b9d"
        status: pass
    human_judgment: false
  - id: D4
    description: "Rulesets 25445011 and 25445012 keep pull_request, deletion and non_fast_forward and add one strict required_status_checks rule with Test, Clippy and Format pinned to app 15368, set after CI reported; the branch rules endpoints list it"
    requirement: REL-04
    verification:
      - kind: other
        ref: "Task 3 verify blocks 3 and 4 (exit 0); before-snapshot diff shows the other rules, conditions and bypass actors unchanged"
        status: pass
    human_judgment: false
  - id: D5
    description: "No negative-control artifact in the phase's code history or in the PR's history, no 36-character ghp_ token in the phase history, the phase's Docker volumes removed"
    requirement: SUP-05
    verification:
      - kind: other
        ref: "Task 1 verify block 3 re-run on d99f325 (40 commits, exit 0) with positive samples; same scan over develop..34e1b54 (16 commits); Task 1 verify block 4 re-run after the last container run (exit 0)"
        status: pass
    human_judgment: false
  - id: D6
    description: "The weekly cargo-deny schedule and Dependabot updates take effect"
    verification: []
    human_judgment: true
    rationale: "Both read their files from the default branch, main; they are configured, but cannot be observed until the phase is released to main"

duration: 11min
completed: 2026-10-23
status: complete
---

# Phase 1 Plan 07: First CI run on a PR to develop and required checks on both rulesets Summary

**PR #2 (`feature/phase-1-pr` to `develop`) ran CI for the first time: Test, Clippy and Format
passed in the pinned image on head `34e1b54`, and the Test log shows the parity OK line and
`rustc 1.99.1`. Both rulesets now require those three checks from GitHub Actions, with the
strict up-to-date policy, and keep their earlier rules. The PR carries exactly the phase's code
changes, all 16 commits signed, with no planning or personal path. The owner did the push, the
PR and the ruleset update. The executor made only GET calls.**

## What these results do and do not establish

- **The PR head and the phase branch's final tree match for code.** The 23 paths PR #2 changes
  are identical to `feature/phase-1` at `d99f325`, except `.gitignore` (the expected workspace
  block). The CI run on `34e1b54` is therefore a run of the final code.
- **One green CI run on one head shows the workflows run. It does not show they stay green.**
  Each of the three checks ran twice on that head (push and pull_request events; backlog 999.1),
  and all six runs passed. That is two runs, one commit.
- **Task 1's gate evidence from 2026-10-22 was for an older tree** (`e067e90`). After the
  checkpoint, three fix commits landed (`dfa6c20`, `fe762f4`, `fa4f3eb`), so I re-ran the gates,
  the history scan and the volume cleanup on `d99f325`. **The plan's Task 1 parity check no
  longer passes as written.** `fe762f4` changed the script's wording to
  `(4 job container image(s))`. The re-run asserts the new wording and records that the old
  literal fails (Deviation 2).
- **The rehearsal used the local `develop`,** `d693c9e`. `git ls-remote` gave the same commit
  for GitHub's `develop` at the time, and it still does. The owner's real cut also passed its
  own audit.
- **The weekly cargo-deny cron and Dependabot are configured, but take effect only after
  release to `main`.** Neither was observed.
- **The history scan matches fixed strings.** It finds the five named control artifacts and
  classic 36-character `ghp_` tokens. It would not find a fine-grained `github_pat_` token or a
  renamed artifact.
- **Required-check enforcement was read back, not exercised.** No PR was shown to be blocked
  by a failing or missing check. The configuration is right; the block itself was not
  observed.
- **The Docker volumes are gone only until the next container run.** Any
  `scripts/check-in-container.sh` run from this worktree re-creates them (see Cleanup).

## Performance

- **Duration:** about 11 min of execution: Task 1 from 2026-10-22T20:27:20Z to 20:31:58Z, and
  Task 3 plus the re-runs from 2026-10-23T19:48:39Z to about 19:56Z. The owner checkpoint sits
  in between.
- **Tasks:** 3 of 3 (Task 2 by the owner)
- **Files modified:** this SUMMARY. `01-07-TASK1-EVIDENCE.md` was added at the checkpoint and
  is removed here. No tracked code file was changed by this plan.

## Accomplishments

- All gates pass on the final tree, on the host and in the pinned image (re-run on `d99f325`).
- The PR cut was rehearsed in a scratch clone. The owner then cut and pushed `feature/phase-1-pr`
  and opened PR #2 to `develop`.
- CI's Test, Clippy and Format passed on PR #2's head. The advisory `cargo deny (advisory)` job
  passed too.
- Both rulesets require Test, Clippy and Format, pinned to GitHub Actions, with the strict
  policy.
- No control artifact is in the phase's code history or in the PR's history. The phase's
  volumes and scratch directories are removed.

## Task Commits

1. **Task 1: phase gate, PR-cut rehearsal, history scan, cleanup, owner's commands** - `bf0d7b6`
   (docs; evidence file)
2. **Task 2: owner step** - no executor commit. The orchestrator committed `187bf40`
   (`docs(01-07): record phase state before the PR cut`, `.planning/config.json` only).
3. **Task 3: read-only verification** - no tracked change. The evidence is in this SUMMARY.

**Plan metadata:** the docs commit that adds this SUMMARY.

Measured `git rev-list --count e067e90..d99f325` = 8. Only `bf0d7b6` is an executor commit of
this plan. The other seven were made between Task 1 and Task 3 by the orchestrator or owner:
- `187bf40`, the planning state commit;
- `dfa6c20`, `fe762f4` and `fa4f3eb`, the review fixes;
- three backlog entries.

## Task 3 evidence (2026-10-23, every gh call a GET)

PR: `{"baseRefName":"develop","headRefName":"feature/phase-1-pr","headRefOid":"34e1b54d224a0923377a44faa5736e80c6937b9d","isDraft":false,"mergeStateStatus":"CLEAN","number":2,"state":"OPEN"}`.
`git ls-remote` shows `refs/heads/feature/phase-1-pr` at the same SHA.

**Block 1 (exit 0).** Check runs on head `34e1b54d224a0923377a44faa5736e80c6937b9d`. All come
from app 15368; there are 0 check runs from any other app and 0 legacy commit statuses.
```
Test app=15368 completed/success started=2026-10-23T19:45:59Z completed=2026-10-23T19:46:45Z
Format app=15368 completed/success started=2026-10-23T19:45:59Z completed=2026-10-23T19:46:44Z
Clippy app=15368 completed/success started=2026-10-23T19:45:58Z completed=2026-10-23T19:46:59Z
cargo deny (advisory) app=15368 completed/success started=2026-10-23T19:45:58Z completed=2026-10-23T19:46:54Z
Test app=15368 completed/success started=2026-10-23T19:45:53Z completed=2026-10-23T19:46:39Z
Format app=15368 completed/success started=2026-10-23T19:45:53Z completed=2026-10-23T19:46:38Z
Clippy app=15368 completed/success started=2026-10-23T19:45:53Z completed=2026-10-23T19:46:39Z
cargo deny (advisory) app=15368 completed/success started=2026-10-23T19:45:53Z completed=2026-10-23T19:46:45Z
```
That gives `Test=success`, `Clippy=success` and `Format=success`, with no other conclusion for
those names. Advisory deny job: `success` on both runs.

**Block 2 (exit 0).** Test job log. Both Test runs show the same lines:
```
image parity OK: mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie (4 job container image(s))
==> rustc 1.99.1 (58cf8f9c3 2026-10-13)
Summary [   0.022s] 7 tests run: 7 passed, 0 skipped
==> check.sh: test OK
```

**Block 3 (exit 0).** Rulesets and branch rules:
```
25445011 develop-merge-or-squash enforcement=active updated_at=2026-10-23T15:48:06.474-04:00 types=["deletion","non_fast_forward","pull_request","required_status_checks"] contexts=["Test@15368","Clippy@15368","Format@15368"] strict=[true] bypass=[] include=["refs/heads/develop"]
25445012 main-squash-only enforcement=active updated_at=2026-10-23T15:48:07.134-04:00 types=["deletion","non_fast_forward","pull_request","required_status_checks"] contexts=["Test@15368","Clippy@15368","Format@15368"] strict=[true] bypass=[] include=["refs/heads/main"]
develop: [{"contexts":["Test@15368","Clippy@15368","Format@15368"],"ruleset_id":25445011,"strict":true}]
main: [{"contexts":["Test@15368","Clippy@15368","Format@15368"],"ruleset_id":25445012,"strict":true}]
```
Ruleset 25445013 (`workspace-no-delete-no-force`) is unchanged, at updated_at
`2026-10-19T21:01:18.973-04:00`.

I compared the orchestrator's before snapshots (`ruleset-<id>.before.json`, whose `updated_at`
values equal what I read on 2026-10-22) with the current GET. Name, target, enforcement,
conditions, bypass actors and every rule other than the new one are unchanged for both
rulesets.

**Block 4 (exit 0). Ordering.** The latest `completed_at` of the Test, Clippy and Format runs is
`2026-10-23T19:46:59Z`. The rulesets were updated at `19:48:06Z` and `19:48:07Z`, so the checks
were required only after CI had reported.

**The PR as pushed, checked locally against `34e1b54`.** Each check was run once.
- `develop..34e1b54` has 16 commits, all with signature status `G`. The history scan over them
  is clean.
- The changed set against `develop` (23 paths) equals the phase's changes since `08b7293`,
  excluding `.planning` and `scripts/workspace`.
- No stripped path is in the PR.
- Every path except `.gitignore` is identical to `feature/phase-1` at `d99f325`. The PR's
  `.gitignore` has `/target/` and no workspace block.
- Control: the same content check reports `.gitignore` as different.

## Task 1 evidence

### Gates on the final tree (re-run 2026-10-23 on `d99f325`, exit 0)

Host `scripts/check.sh all` and `deps` passed, then both through `scripts/check-in-container.sh`,
then parity in the image and on the host. shellcheck and `bash -n` were clean for all five
scripts, and all five are tracked `100755`. In the container:
```
==> image:  mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie
==> rustc 1.99.1 (58cf8f9c3 2026-10-13)
==> cargo-nextest 0.9.148 OK
     Summary [   0.026s] 7 tests run: 7 passed, 0 skipped
==> check.sh: all OK
==> cargo-deny 0.20.2 OK
advisories ok, bans ok, licenses ok, sources ok
==> check.sh: deps OK
image parity OK: mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie (4 job container image(s))
```
No `command not found` appeared. The plan's literal `(4 workflow image(s))` does not match this
output. The first run, on 2026-10-22 at `e067e90`, matched it, with 2 tests.

### PR-cut rehearsal (2026-10-22, scratch clone of `e067e90`, exit 0)

This was a `git clone -b feature/phase-1` of the common git dir, with an empty global config and
`GIT_CONFIG_NOSYSTEM=1`. A local `workspace/denniyahh` was created from
`origin/workspace/denniyahh`, then `scripts/workspace/cut-pr-branch.sh --phase 1` ran.
Rehearsal base commit: `d693c9ec18b2720f9a8a78fe8f8f76a180c1daa9`, the local `develop`, equal
to `origin/develop` and to GitHub's `develop`.
```
==> Source branch: feature/phase-1 (30 commits scoped to phase 1 not yet in origin/develop)
==> Commit classification summary:
    Included: 14 code commit(s)
    Excluded: 16 planning/environment-only commit(s)
==> Audit passed: 'feature/phase-1-pr' carries ZERO personal/agent artifacts.
==> Fidelity: 22 path(s) differ from origin/develop, all from replayed commits.
warning: these paths do not match feature/phase-1; review each before pushing:
  .gitignore
```

Each negative control in the clone failed as it should:
- the set-equality check fails when `.planning` is included;
- the stripped-path pattern matches 3 of 3 planted sample paths;
- the content check fails on `.gitignore`.

After the run, no `feature/phase-1-pr` ref existed in the real repository. The orchestrator
reports that the owner's real cut, after the fixes, included 16 commits, excluded 18 and had
23 fidelity paths. I checked the result above against the pushed head, not the cut's output.

### History scan (re-run 2026-10-23 on `d99f325`, exit 0)

The scan covered 40 commits since `08b7293`.
- Code changes (non-planning) contain none of: the vulnerable test crate name, the planted
  env-var name, the mismatched image tag, the planted missing-tool call, or a 36-character
  `ghp_` token.
- The whole phase history has no such token.
- A runtime-generated sample token matches the pattern. On 2026-10-22, one positive sample for
  each of the other four literals matched the same alternation.
- The control literals do appear in `.planning` files (plans and summaries naming the controls).
  The scan excludes `.planning` by design.

### Ruleset dry run (2026-10-22, GET only, exit 0)

Each body kept the three existing rules byte for byte and added exactly one
`required_status_checks` rule. Controls:
- the then-current rulesets failed the four-rule check;
- applying the program to its own output still gave one rule.

The owner's PUT used the same program, and the read-back above matches these bodies.

Body for 25445011:
```json
{"name":"develop-merge-or-squash","target":"branch","enforcement":"active","conditions":{"ref_name":{"exclude":[],"include":["refs/heads/develop"]}},"bypass_actors":[],"rules":[{"type":"pull_request","parameters":{"required_approving_review_count":0,"dismiss_stale_reviews_on_push":false,"required_reviewers":[],"require_code_owner_review":false,"require_last_push_approval":false,"required_review_thread_resolution":false,"require_extra_approval_for_unattributed_changes":true,"allowed_merge_methods":["merge","squash"]}},{"type":"deletion"},{"type":"non_fast_forward"},{"type":"required_status_checks","parameters":{"strict_required_status_checks_policy":true,"do_not_enforce_on_create":false,"required_status_checks":[{"context":"Test","integration_id":15368},{"context":"Clippy","integration_id":15368},{"context":"Format","integration_id":15368}]}}]}
```
Body for 25445012:
```json
{"name":"main-squash-only","target":"branch","enforcement":"active","conditions":{"ref_name":{"exclude":[],"include":["refs/heads/main"]}},"bypass_actors":[],"rules":[{"type":"pull_request","parameters":{"required_approving_review_count":0,"dismiss_stale_reviews_on_push":false,"required_reviewers":[],"require_code_owner_review":false,"require_last_push_approval":false,"required_review_thread_resolution":false,"require_extra_approval_for_unattributed_changes":true,"allowed_merge_methods":["squash"]}},{"type":"deletion"},{"type":"non_fast_forward"},{"type":"required_status_checks","parameters":{"strict_required_status_checks_policy":true,"do_not_enforce_on_create":false,"required_status_checks":[{"context":"Test","integration_id":15368},{"context":"Clippy","integration_id":15368},{"context":"Format","integration_id":15368}]}}]}
```

### Owner's command sequence (verbatim from the plan, Task 2)

Run these in a bash shell (your interactive shell is fish; the loop in step 5 is bash syntax), from the phase-1 worktree:

0. `git status --porcelain --untracked-files=no` must print nothing: `cut-pr-branch.sh` refuses on any uncommitted change to a tracked file, and the planning updates made at this checkpoint (for example STATE.md or ROADMAP.md) count. If it lists files, commit them first with `git add -u .planning && git commit -m "docs(01-07): record phase state before the PR cut"`; the cut classifies a commit that touches only `.planning` as planning-only and leaves it out of the PR. Untracked files (such as a SUMMARY draft) do not block the cut.
1. `scripts/workspace/cut-pr-branch.sh --phase 1`, then check its output shows `Audit passed` and that the only fidelity warning is `.gitignore` (expected: the PR copy has no workspace block).
2. `git push -u origin feature/phase-1-pr`
3. `gh pr create --base develop --head feature/phase-1-pr --title "Phase 1: development foundation" --body "Rust crate and gates, container and CI parity, supply-chain scanning. Evidence in the phase summaries."`
4. `gh pr checks <PR> --watch` until Test, Clippy and Format have finished; all three must pass. The `cargo deny (advisory)` job is advisory: report its result, but it does not block this step.
5. Only after step 4, update both rulesets (keeps every existing rule, replaces any earlier required-checks rule, pins each context to GitHub Actions):
   `for id in 25445011 25445012; do gh api "repos/denniyahh/wsx/rulesets/$id" | jq '{name, target, enforcement, conditions, bypass_actors, rules: ([.rules[] | select(.type != "required_status_checks")] + [{type: "required_status_checks", parameters: {strict_required_status_checks_policy: true, do_not_enforce_on_create: false, required_status_checks: [{context: "Test", integration_id: 15368}, {context: "Clippy", integration_id: 15368}, {context: "Format", integration_id: 15368}]}}])}' | gh api -X PUT "repos/denniyahh/wsx/rulesets/$id" --input - >/dev/null && echo "updated $id" || echo "FAILED $id"; done`
   If GitHub rejects `integration_id`, rerun without the three `integration_id` fields and say so in your reply. Do not touch ruleset 25445013.

The owner replied `done PR=2`, with `integration_id` kept. The read-back confirms it.

## Who did the outward-facing steps

| Step | Done by |
|---|---|
| PR cut in the real repository, push of `feature/phase-1-pr` | owner (Task 2) |
| PR #2 creation | owner (Task 2) |
| Ruleset PUTs on 25445011 and 25445012 | owner (Task 2) |
| Removing the repo-local gitleaks hook from `.git/config` | owner, through the orchestrator |
| Everything in this plan's Tasks 1 and 3 | executor, local commands and GitHub GETs only |

## Owner-approved local exception: repository gitleaks hook

At the owner's request, the orchestrator removed the repo-local hook from this repository's
shared `.git/config` (`git config --local --remove-section hook.gitleaks-staged`). Checked on
2026-10-23:
- `git config --local --get-regexp '^hook\.'` is empty;
- `git hook list pre-commit` shows only `hook from hookdir`;
- the owner's global hook (`core.hooksPath`) runs
  `gitleaks git --pre-commit --staged --redact --verbose` and refuses the commit when gitleaks
  is missing.

On this machine every commit is therefore scanned once, by the global hook. `scripts/setup-repo.sh`
still registers the repo hook, so a fresh clone that follows CONTRIBUTING gets it. It was not
re-run here. The `include.path ../.gitconfig` entry is still present.

## Decisions Made

- I kept the pre-checkpoint evidence in a non-SUMMARY file, so GSD would not count the plan as
  finished while Task 3 was open. It is folded in here and deleted.
- I re-ran the Task 1 gates, history scan and cleanup on the final tree rather than citing the
  older run.

## Deviations from Plan

**1. [Procedural] Rehearsal scratch removal by explicit path**
- **Found during:** Task 1, block 2.
- The `trap 'rm -rf "$S"' EXIT` was replaced by printing `S`. The clone was then removed by its
  literal path, following the owner's instruction to delete scratch by explicit full path (as in
  01-05). The assertions ran unchanged, and the cut output was also copied to a log.

**2. [Rule 1 - Stale check] Task 1's parity literal no longer matches the script**
- **Found during:** Task 3. The Test job log reads `(4 job container image(s))`.
- **Issue:** `fe762f4` (an orchestrator review fix after the checkpoint) changed the
  parity script's OK wording. The plan's block 1 asserts `(4 workflow image(s))` and now fails.
  Task 3's block 2 matches only the prefix and passes.
- **Fix:** I re-ran block 1 on `d99f325` with the new wording (exit 0) and recorded that the old
  literal does not match. No file was changed. The plan text is left as written.

**3. [Added] Evidence file at the checkpoint**
- `01-07-TASK1-EVIDENCE.md` (`bf0d7b6`) held Task 1's output, because the plan has Task 1
  write into a SUMMARY. It is deleted in this plan's SUMMARY commit. The deletion is intentional.

**Precondition note.** Task 1 step 1 asks for a clean tree apart from untracked `.planning`
files. On 2026-10-22, `.planning/config.json` was modified and uncommitted: the orchestrator's
change, which I left alone. It affected neither the gates nor the rehearsal. The orchestrator
committed it as `187bf40` before the cut.

**Total deviations:** 3 (1 procedural, 1 stale check re-run, 1 added file, now removed).
**Impact on plan:** none on the outcome. Every truth was checked against the final tree or the
PR head.

## Cleanup

- **Docker volumes the phase created:**
  - `wsx-ci-target-4fa63246f2d5` and `wsx-ci-registry-4fa63246f2d5` were removed on 2026-10-22.
    My 2026-10-23 gate re-run created them again, so they were removed again after that last
    container run (block 4, exit 0). `docker volume ls` lists no `wsx*` volume, and
    `docker ps -a` lists no container.
  - 01-03's other volumes had already been removed by name in 01-03.
- **These volumes come back with any later container run.** A later `scripts/check-in-container.sh`
  run from this worktree, for example the phase verifier re-checking criterion 1, re-creates
  `wsx-ci-target-<hash>` and `wsx-ci-registry-<hash>`. Afterwards, run:
  ```bash
  docker volume rm wsx-ci-target-4fa63246f2d5 wsx-ci-registry-4fa63246f2d5
  ```
- **Kept on purpose:** the image `mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie`. It was
  pulled during research for this project, not created by an executor.
- **Scratch:** the rehearsal clone (`exec-01-07/cut.E2tWhA`), `exec-01-07/` and `exec-01-07-t3/`
  under the session scratchpad were removed by explicit paths. The orchestrator's
  `ruleset-*.{before,after}.json` files are its own and were read, not removed.

## Issues Encountered

- Each CI check runs twice per PR head (push and pull_request). This is harmless, and it is
  already in the backlog as 999.1.

## User Setup Required

None.

## Next Phase Readiness

- Every PR to `develop` or `main` now needs Test, Clippy and Format from GitHub Actions, up to
  date with its base. A job rename in `ci.yml` blocks merges; its header comment says so.
- PR #2 is open and mergeable (`CLEAN`). Merging it is outside this plan.
- The weekly cargo-deny run and Dependabot start once the files reach `main`.

## Self-Check: PASSED

- FOUND: `.planning/phases/01-development-foundation/01-07-SUMMARY.md` (this file).
- FOUND on this branch: `bf0d7b6` (Task 1 evidence commit).
- Task 3 blocks 1-4: exit 0 each, on head `34e1b54`.
- Task 1 blocks 1, 3 and 4 re-run on `d99f325`: exit 0 each (block 1 with the updated parity
  string). Block 2 passed on 2026-10-22, and its assertions hold against the pushed head.
- Commits measured from the ledger: `git rev-list --count e067e90..d99f325` = 8 before this docs
  commit.

---
*Phase: 01-development-foundation*
*Completed: 2026-10-23*
