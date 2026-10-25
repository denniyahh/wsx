---
phase: 01
review: 01-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "`setup-repo.sh policy` aborts part-way with exit 5 when a policy key already has several local values, and `check` then points at a fix that cannot succeed"
  - id: IN-01
    severity: info
    disposition: fixed
    title: "The IN-01 fix kept a usage example that names the wrong branch"
  - id: IN-02
    severity: info
    disposition: deferred
    title: "`--force` re-cut keeps an upstream the branch already had"
  - id: IN-03
    severity: info
    disposition: fixed
    title: "An exported `CDPATH` redirects every script's repo-root resolution (pre-existing; WR-03's guarantee depends on it)"
  - id: IN-04
    severity: info
    disposition: deferred
    title: "Several `setup-repo.sh` refusal branches have no test; one is only proven where gitleaks is absent"
  - id: IN-05
    severity: info
    disposition: deferred
    title: "`check` cannot see a key that was dropped from `.gitconfig`, and it prints unset and empty values alike"
  - id: IN-06
    severity: info
    disposition: deferred
    title: "The PR cut counts merge commits as \"planning-only\" and drops any content they carry (pre-existing, same class as CR-03)"
  - id: IN-07
    severity: info
    disposition: deferred
    title: "Gate accepted forms are still untested, and a positional name is still a substring match (prior IN-04, carried)"
  - id: CR-02
    severity: critical
    disposition: fixed
    title: "The pin check verifies one binary while cargo runs another"
  - id: CR-03
    severity: critical
    disposition: fixed
    title: "The PR cut silently drops code commits whose cherry-pick fails"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "The parity check passes a job that runs in an unpinned image"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "`check-in-container.sh` gates the repo under the current directory"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "The PR cut force-resets any branch it is given"
open: 0
total: 13
recorded: 2026-10-25T10:54:57.230Z
---

# Phase 01: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | fixed | f1f1303: --replace-all in the write loop; test policy_replaces_a_key_that_already_has_several_local_values (fails with the old line) |
| IN-01 | info | fixed | 570dd6b: example now says feature/45 |
| IN-02 | info | deferred | stand-in script, retired by wsx phases 9-10; current cuts use --no-track and feature/phase-1-pr is unaffected (owner decision 2026-10-23: harden stand-ins only for safety) |
| IN-03 | info | fixed | f1f1303: CDPATH cleared for the root cd in check.sh, setup-repo.sh, assert-image-parity.sh, check-in-container.sh; test an_exported_cdpath_does_not_move_the_script_to_another_tree (fails without it) |
| IN-04 | info | deferred | policy is a stand-in until UDE backlog 999.2; the refusal branches print and exit before any write, which the refusal test pins for the security-relevant one |
| IN-05 | info | deferred | UDE backlog 999.2 owns drift reporting for all projects; the stand-in stays minimal |
| IN-06 | info | deferred | stand-in script; wsx PRCUT-02 specifies merge-commit handling for the replacement (phases 9-10). Phase cuts use --no-merges and the fidelity check warns on any path that differs |
| IN-07 | info | deferred | carried from the first review (prior IN-04); a test gap only, no defect found; the refused forms are tested in tests/gate.rs, the accepted single-selector forms are not tested anywhere |
| CR-02 | critical | fixed | 01-REVIEW-FIX.md (not in the current review) |
| CR-03 | critical | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-02 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-03 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-04 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
