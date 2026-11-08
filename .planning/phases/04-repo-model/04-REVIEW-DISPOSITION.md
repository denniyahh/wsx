---
phase: 04
review: 04-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: open
    title: "Nested replace refs bypass the D-22 guard; wsx and git read different `.wsx.conf` bytes"
  - id: WR-01
    severity: warning
    disposition: open
    title: "Replace base without a trailing `/` violates `refs_directly_under`'s precondition, so the guard checks nothing"
  - id: WR-02
    severity: warning
    disposition: open
    title: "git-layer messages print ref names raw, so a control byte from config reaches the terminal"
  - id: WR-03
    severity: warning
    disposition: open
    title: "`build` resolves the integration branch before the manifest, so a stray candidate reports the wrong failure (D-16)"
  - id: WR-04
    severity: warning
    disposition: open
    title: "A symbolic workspace branch is followed, though a symbolic integration rung is refused"
  - id: IN-01
    severity: info
    disposition: open
    title: "`NotConfigured` around `ManifestMissing` says \"run wsx init\" twice"
  - id: IN-02
    severity: info
    disposition: open
    title: "`WorkspaceBranchMissing` puts C quotes in the middle of a ref name"
open: 7
total: 7
recorded: 2026-11-08T19:40:00.000Z
---

# Phase 04: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | open | - |
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| WR-04 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
