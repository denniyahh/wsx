# Round 4 (operator-authorised, codex only) — collapsed guard, e3370bc

Operator chose on 2026-11-16 to refuse any replace ref (D-22 amended) and to review the rewrite with one codex lane.

- U1 major: an absolute `GIT_REPLACE_REF_BASE` failed in `Repo::discover` as "not a git repository". Folded (f5fbbfb): discover refuses a non-default variable before gix opens. The config-key case is a stated residual (wrong text, no byte divergence).
- U2 major (suspected, accepted): no test pinned the guard's early call in `build`. Folded: `replaced_workspace_commit_is_refused_before_it_is_peeled` plus a delete-the-call check that must fail with `got ManifestUnreadable`.
- U3 backlog: a fifo `packed-refs` hangs discovery before the guard (pre-existing, no byte divergence).
- No admitted byte divergence reproduced for loose, nested, packed, linked-worktree or custom-base replacements.
