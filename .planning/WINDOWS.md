---
schema_version: 1
open_count: 2
waived_count: 0
fixed_count: 1
total_count: 3
last_updated: 2026-11-17T02:47:42.137Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 2 | lint-warning | tests/harness/main.rs |  | cargo fmt does not reach the scenario modules declared inside harness_modules!, so check.sh fmt passes over unformatted sync.rs, worktree.rs and cut.rs | fixed |  | 2026-10-26T18:59:54.892Z | 2026-10-26T19:04:21.018Z |
| 2 | 04 | stub | src/model/mod.rs | 296 | resolve with no wsx.sourceBranch returns NotConfigured with an empty candidate list; workspace-branch candidate selection (D-17, D-21) is plan 04-08's | open |  | 2026-11-08T15:28:18.088Z |  |
| 3 | 04 | deviation | Cargo.lock |  | scripts/check.sh deps fails on RUSTSEC-2026-0486 (rustix 1.1.5); cargo update -p rustix needs the operator, then re-run 04-12's mutation pass | open |  | 2026-11-17T02:47:42.137Z |  |

````json
[
  {
    "id": 1,
    "kind": "lint-warning",
    "phase": "2",
    "file": "tests/harness/main.rs",
    "line": null,
    "description": "cargo fmt does not reach the scenario modules declared inside harness_modules!, so check.sh fmt passes over unformatted sync.rs, worktree.rs and cut.rs",
    "status": "fixed",
    "reason": "",
    "recorded_at": "2026-10-26T18:59:54.892Z",
    "resolved_at": "2026-10-26T19:04:21.018Z",
    "milestone": "v0.1.0"
  },
  {
    "id": 2,
    "kind": "stub",
    "phase": "04",
    "file": "src/model/mod.rs",
    "line": 296,
    "description": "resolve with no wsx.sourceBranch returns NotConfigured with an empty candidate list; workspace-branch candidate selection (D-17, D-21) is plan 04-08's",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-08T15:28:18.088Z",
    "resolved_at": null,
    "milestone": "v0.1.0"
  },
  {
    "id": 3,
    "kind": "deviation",
    "phase": "04",
    "file": "Cargo.lock",
    "line": null,
    "description": "scripts/check.sh deps fails on RUSTSEC-2026-0486 (rustix 1.1.5); cargo update -p rustix needs the operator, then re-run 04-12's mutation pass",
    "status": "open",
    "reason": "",
    "recorded_at": "2026-11-17T02:47:42.137Z",
    "resolved_at": null,
    "milestone": "v0.1.0"
  }
]
````
