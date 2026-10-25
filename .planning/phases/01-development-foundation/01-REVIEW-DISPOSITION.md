---
phase: 01
review: 01-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "`setup-repo.sh policy` aborts part-way with exit 5 when a policy key already has several local values, and `check` then points at a fix that cannot succeed"
  - id: IN-01
    severity: info
    disposition: open
    title: "The IN-01 fix kept a usage example that names the wrong branch"
  - id: IN-02
    severity: info
    disposition: open
    title: "`--force` re-cut keeps an upstream the branch already had"
  - id: IN-03
    severity: info
    disposition: open
    title: "An exported `CDPATH` redirects every script's repo-root resolution (pre-existing; WR-03's guarantee depends on it)"
  - id: IN-04
    severity: info
    disposition: open
    title: "Several `setup-repo.sh` refusal branches have no test; one is only proven where gitleaks is absent"
  - id: IN-05
    severity: info
    disposition: open
    title: "`check` cannot see a key that was dropped from `.gitconfig`, and it prints unset and empty values alike"
  - id: IN-06
    severity: info
    disposition: open
    title: "The PR cut counts merge commits as \"planning-only\" and drops any content they carry (pre-existing, same class as CR-03)"
  - id: IN-07
    severity: info
    disposition: open
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
open: 8
total: 13
recorded: 2026-10-25T10:54:57.230Z
---

# Phase 01: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |
| IN-06 | info | open | - |
| IN-07 | info | open | - |
| CR-02 | critical | fixed | 01-REVIEW-FIX.md (not in the current review) |
| CR-03 | critical | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-02 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-03 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |
| WR-04 | warning | fixed | 01-REVIEW-FIX.md (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
