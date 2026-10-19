# M1 planning — adversarial external review

**Date:** 2026-10-19
**Targets:** `.planning/PROJECT.md`, `.planning/REQUIREMENTS.md` (uncommitted draft, 48 v1), `.planning/research/SUMMARY.md`, checked against `bin/wsx` and the research files
**Lanes:** agy (gemini-3.8-flash-high), codex (gpt-5.6-terra, high, read-only sandbox), pi (deepseek-flash). All three returned (exit 0). None dropped.
**Depth:** agy 10 confirmed + 2 suspected; codex 9 + 1; pi 12 + 8. Every finding below was checked by the host against the cited lines.

## Confirmed — requirement changes needed

| # | Finding | Lanes | Evidence checked | Fix |
|---|---------|-------|------------------|-----|
| 1 | The negative control dies at cutover: HARN-04 requires "fails on the oracle", and CUT-03 deletes the oracle. The research already warned against a differential-only harness | agy, pi | ARCHITECTURE.md:502-506 | Every scenario carries explicit expected values. The oracle check runs only before cutover. Oracle outcomes are frozen as fixtures |
| 2 | GUARD-06 reads stdin `cwd`, but the PROJECT constraint says "never prompt or read stdin" | pi | PROJECT.md:84 | Narrow the constraint: only agent session hooks read a bounded JSON payload from stdin; git hooks never read stdin (husky-init.sh:9-12, 29) |
| 3 | Backup retention is in M1 (SAFE-05) but listed as M2 in PROJECT and FEATURES | all 3 | PROJECT.md:62 | Remove it from PROJECT Out of Scope, and state the number kept per path |
| 4 | Backups live in the dev worktree's git-dir, which `wsx rm` deletes, so source backups from the final push are destroyed | pi | bin/wsx:13, :144, :335 | Keep backups under the common dir (`<common-dir>/wsx/backups/<worktree-id>/`) |
| 5 | SAFE-03 requires backup on every delete, but the research's `DeleteRemote` action has no backup field, and the oracle deletes the source with no backup | codex | ARCHITECTURE.md:172; bin/wsx:473-475 | Every destructive action variant carries a backup instruction, and each one is tested separately |
| 6 | The mass-deletion brake has no threshold, no counting rule, no `--force` on sync/push/pull (only `rm` has it), and no hook-mode behaviour | all 3 | bin/wsx:1117-1139 | A default threshold and a config key; count delete-local plus delete-remote before applying; hook mode applies non-delete actions, skips the deletes and records the trip where `status`/`doctor` show it; `--force` on sync/push/pull; boundary tests |
| 7 | Symlinks: the research design maps `Obs::Symlink` to "absent", which plans deletions. A *source-side* leaf symlink makes the oracle plan a local delete. SYNC-07 protects only the local side and gives no rule for telling a migration link from a user link | codex, agy, pi | ARCHITECTURE.md:159; bin/wsx:268, :374-398 | Symlink → `Blocked` on either side. Only the legacy layout (a local link that resolves into the source worktree) migrates |
| 8 | A user file named `*.wsx-remote` is invisible to sync and gets overwritten or deleted as a conflict marker | codex, pi | bin/wsx:264-265, :436-438, :476-478 | Report a marker collision and leave the file alone |
| 9 | SYNC-06 (tracked paths) has no data source in the planner input or the Git trait | codex | ARCHITECTURE.md:150, :336 | The snapshot includes tracked-path membership from the index and HEAD |
| 10 | Corrupt `base.json`: CORE-05 says stop, but the research says treat it as empty (0.1.0 behaviour). An empty base resurrects local deletions | codex | ARCHITECTURE.md:304; bin/wsx:212-215 | Keep CORE-05 (stop). Hook mode: skip, exit 0, record the reason for `status`/`doctor` |
| 11 | Guard fail-closed scope: husky-init runs `wsx hook` in every repo on the machine with `\|\| exit 1`. An "internal error" before wsx knows the repo is configured (e.g. a reftable repo refused by GIT-03) would block commits everywhere | pi (agy partly) | husky-init.sh:28-29; bin/wsx:993-994 | Fail closed only after the repo is confirmed wsx-configured. Unconfigured or refused repos exit 0 |
| 12 | Worktree ID collision: a linked worktree named `main` shares `exclude.wsx-main` with the main checkout, and pruning it unlinks the main checkout's ignore file | agy | bin/wsx:504-509 | Add GIT-06: collision-free IDs, and pruning that recognises legacy names |
| 13 | mtime preservation on copy isn't required anywhere, but lww and the stat cache depend on it (`copy2`); `std::fs::copy` doesn't preserve it | agy | bin/wsx:320; PITFALLS.md:309 | Add an explicit requirement |
| 14 | SAFE-02 says "downgrade to a conflict", which is undefined for deletes (no source content for the marker) and is decision logic in the executor | agy, pi | ARCHITECTURE.md:161-175 | A failed compare-and-swap aborts that one action (no write, no base change) and is reported. No marker |
| 15 | SAFE-01 "about 2 s" isn't falsifiable, and the report is lost because agent hooks discard stdout/stderr | pi | PITFALLS.md:350 | Exact bound; record skipped runs in state that `status`/`doctor` surface |
| 16 | The guard never runs in repos without `.husky/pre-push` (husky exits first); no M1 requirement detects this | pi | PITFALLS.md:152, :158 | `doctor` reports a missing pre-push wiring (a LIFE-03 addition) |
| 17 | HARN-04's "passes on Rust, fails on the oracle" cannot express a deliberate regression (reftable: oracle works, Rust refuses) | agy | STACK.md:45 | Add a `regression:` scenario class with inverted expectations |

## Disputed / rejected

- **agy #3 "guard falls back to origin/main and fails closed":** wrong. `base_ref()` raises rather than defaulting (bin/wsx:803-812), and `cmd_guard` doesn't use it (bin/wsx:992-1000). The unborn-HEAD part is plausible and is covered by fix 11 plus a first-commit scenario.
- **pi #6 "crash after copy, before base save, gives a permanent false conflict":** wrong for pull/push. Both sides are then equal, and the planner returns "converged" first (bin/wsx:380). The point that rename isn't fsync-durable stands as a SAFE-06 test-design note.
- **agy #10 "resolve --theirs deadlock":** overstated. Deleting the local file by hand converges. It's still a real usability gap: `--theirs` errors when the source was deleted (bin/wsx:707-708). This is parity behaviour, deferrable.
- **Upgrade serde_json surrogates (agy #4):** already covered by CORE-03 and research N5. The suggested wording only sharpens it.

## Residual risks / not settled by this review

- llvm-cov through a scrubbed environment (agy S11, unverified): HARN-02 must allow-list `LLVM_*`, to be checked in the harness phase.
- `lww` on a live SQLite file with WAL (codex S1, pi S8): unverified, documentation risk.
- "Zero surviving mutants" as a gate (pi S2): needs a scope (the planner module) and a rule for equivalent mutants.
