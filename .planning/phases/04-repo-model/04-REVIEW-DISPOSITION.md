---
phase: 04
review: 04-REVIEW.md
titles: json
findings:
  - id: CR-01
    severity: critical
    disposition: fixed
    title: "A fifo at `.git/config` or `packed-refs` hangs wsx in-process, though D-39 says it is stopped"
  - id: WR-01
    severity: warning
    disposition: open
    title: "`collect` waits for pipe EOF and kills only the direct child, so a process git leaves behind turns a success into `GitTimedOut` and outlives the kill"
  - id: WR-02
    severity: warning
    disposition: open
    title: "The `RefsNamespace` refusal gives a cause that is no longer true"
  - id: WR-03
    severity: warning
    disposition: fixed
    title: "A repository without `.git/config` fails `resolve` as soon as any global `remote.*` or `wsx.*` key prints"
  - id: IN-01
    severity: info
    disposition: open
    title: "Early error returns in `collect` leave the child running with no deadline"
  - id: IN-02
    severity: info
    disposition: open
    title: "Impossible-state guard: the 4-answer `try_from` check in `read_tips`"
  - id: IN-03
    severity: info
    disposition: open
    title: "The `git` module's doc still says reads go through gix"
  - id: IN-04
    severity: info
    disposition: open
    title: "The test helpers find the real git only in `/usr/local/bin`, `/usr/bin` or `/bin`"
  - id: WR-04
    severity: warning
    disposition: open
    title: "A symbolic workspace branch is followed, though a symbolic integration rung is refused"
open: 7
total: 9
recorded: 2026-11-18T05:12:18.204Z
---

# Phase 04: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| CR-01 | critical | fixed | d8cae67 (test 3d44c24): discover's open held to CHILD_DEADLINE, special packed-refs refused unopened |
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | fixed | 9b996b1 (test before it; verification gap 1): no own keys when the repository has no config file |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| WR-04 | warning | open | - (not in the current review) |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.
