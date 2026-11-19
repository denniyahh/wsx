---
phase: 05
review: 05-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "Writing the verdict to a closed stdout panics with exit 101, outside the exit-code contract"
  - id: WR-02
    severity: warning
    disposition: open
    title: "One missing symlink blob anywhere in the tree fails the whole check with exit 6"
  - id: WR-03
    severity: warning
    disposition: open
    title: "The replace-ref guard does not cover the phase's new git children, against the comment's claim"
  - id: WR-04
    severity: warning
    disposition: open
    title: "Impossible-state guard: classifying a replace-ref refusal inside `ManifestUnreadable`"
  - id: WR-05
    severity: warning
    disposition: open
    title: "The IF-02 scan misses a forbidden path built from separate literals"
  - id: IN-01
    severity: info
    disposition: open
    title: "The JSON README contradicts itself on where the verdict goes"
  - id: IN-02
    severity: info
    disposition: open
    title: "`wsx --version <extra>` is reported as an unknown command"
  - id: IN-03
    severity: info
    disposition: open
    title: "`core.hooksPath` spellings that git accepts are refused with a misleading reason"
  - id: IN-04
    severity: info
    disposition: open
    title: "A gitlink under the hooks directory gets impossible advice"
  - id: IN-05
    severity: info
    disposition: open
    title: "Relative `core.hooksPath` in a bare repository is resolved from the tree root, not `$GIT_DIR`"
open: 10
total: 10
recorded: 2026-11-19T01:08:01.543Z
---

# Phase 05: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| WR-04 | warning | open | - |
| WR-05 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
