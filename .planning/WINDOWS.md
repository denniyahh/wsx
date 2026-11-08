---
schema_version: 1
open_count: 1
waived_count: 0
fixed_count: 1
total_count: 2
last_updated: 2026-11-08T15:28:18.088Z
---

# Broken Windows Ledger

> Cross-phase defect register. With `workflow.windows_enforce` enabled, `/gsd-ship` blocks while `open_count > 0`.
> Waive with `gsd-tools windows waive <id> "<reason>"` (reason required).
> Mark fixed with `gsd-tools windows fixed <id>`.

| id | phase | kind | file | line | description | status | reason | recorded_at | resolved_at |
|----|-------|------|------|------|-------------|--------|--------|-------------|-------------|
| 1 | 2 | lint-warning | tests/harness/main.rs |  | cargo fmt does not reach the scenario modules declared inside harness_modules!, so check.sh fmt passes over unformatted sync.rs, worktree.rs and cut.rs | fixed |  | 2026-10-26T18:59:54.892Z | 2026-10-26T19:04:21.018Z |
| 2 | 04 | stub | src/model/mod.rs | 296 | resolve with no wsx.sourceBranch returns NotConfigured with an empty candidate list; workspace-branch candidate selection (D-17, D-21) is plan 04-08's | open |  | 2026-11-08T15:28:18.088Z |  |

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
  }
]
````
