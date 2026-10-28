---
phase: 03
review: 03-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "A `GIT_DIR` that gix opens as bare, but git gives the current directory as its worktree, is accepted, including inside a hook"
  - id: WR-01
    severity: warning
    disposition: fixed
    title: "gix ignores `GIT_COMMON_DIR`, so git and wsx read different object stores, and only a relative value is refused"
  - id: WR-02
    severity: warning
    disposition: fixed
    title: "The new install hint is wrong whenever cargo resolves `$CARGO_HOME/bin` first; the old hint was right there"
  - id: IN-01
    severity: info
    disposition: open
    title: "The maintenance counterpart test still starts a detached background writer (carried from iteration 2)"
  - id: IN-02
    severity: info
    disposition: open
    title: "check.sh's own \"tool missing\" hints ignore `WSX_TOOLS_DIR`"
  - id: IN-03
    severity: info
    disposition: open
    title: "Three new refusals, and refusing wsx in receive-pack hooks, are not recorded for phase 5"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "gix ignores `GIT_OBJECT_DIRECTORY`, so wsx reads a different object store than git"
  - id: WR-04
    severity: warning
    disposition: fixed
    title: "The container gate runs the host's `.tools/bin` binaries ahead of the tools it just installed"
  - id: WR-05
    severity: warning
    disposition: fixed
    title: "A stale `.tools/bin` gets a fix suggestion that cannot fix it"
open: 3
total: 9
recorded: 2026-10-28T00:37:22.373Z
---

# Phase 03: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | fixed | 03-REVIEW-FIX.md |
| WR-01 | warning | fixed | 03-REVIEW-FIX.md |
| WR-02 | warning | fixed | 03-REVIEW-FIX.md |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| WR-03 | warning | fixed | 03-REVIEW-FIX.iter3.md (not in the current review) |
| WR-04 | warning | fixed | 03-REVIEW-FIX.iter3.md (not in the current review) |
| WR-05 | warning | fixed | 03-REVIEW-FIX.iter3.md (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
