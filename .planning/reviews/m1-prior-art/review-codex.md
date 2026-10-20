## CONFIRMED

### C1. gix conflict preflight can reject a replay Git would resolve

- Requirement: **PRCUT-03** “wsx replays each selected commit in memory (gix) … and refuses on any predicted conflict”
- Prior art: `wsx/scripts/workspace/cut-pr-branch.sh:200` “`git cherry-pick "$HASH" --no-commit`”
- Problem: gix’s in-memory merge cannot run the user’s merge drivers, rerere, or hooks. A custom merge driver can make Git’s replay succeed while the preflight rejects it. This conflicts with the stated Git-binary rationale.
- Better alternative / cost / milestone: Treat gix as advisory only; validate each replay with `git cherry-pick --no-commit` inside the disposable worktree, then abort/reset it before publishing the target ref. Add a custom merge-driver control that must succeed. Moderate; milestone 1.

### C2. Replay commits can still silently bypass local security and policy hooks

- Requirement: **PRCUT-02** “Replayed commits … are committed through `git`”
- Prior art: `wsx/scripts/workspace/cut-pr-branch.sh:236` “`git commit -C "$HASH" --no-verify --quiet`”
- Problem: “through git” permits retaining `--no-verify`; signing still works with that flag, so the stated signing outcome does not detect skipped gitleaks, commit-msg, or other pre-commit checks.
- Better alternative / cost / milestone: Explicitly prohibit `--no-verify` for replay commits and add a hook control that records invocation and rejects a planted replay. Low; milestone 1.

### C3. Non-interactive merge behavior is not specified for sync or land

- Requirement: **WSYNC-01** “`wsx sync` merges … with a real merge”; **WTREE-06** “merges a finished branch back … with `--no-ff`”
- Prior art: `wsx/scripts/workspace/sync-workspace.sh:37` “`git merge "$BASE_REMOTE/$BASE_BRANCH" -m "chore: sync …"`”
- Problem: a non-fast-forward merge may invoke an editor unless the command fixes the message and disables editing. Null stdin does not prevent an editor process. That violates the fixed non-interactive constraint and can hang hooks.
- Better alternative / cost / milestone: Require `git merge --no-edit -m <deterministic message>` for both operations, with a failing `$GIT_EDITOR` control proving it is never invoked. Low; milestone 1.

### C4. Refusing every replayed change to a `modified` path is an avoidable loss of standalone usefulness

- Requirement: **PRCUT-02** “A replayed commit that touches a `modified` path refuses the cut”
- Prior art: `wsx/scripts/workspace/cut-pr-branch.sh:121-126` adds only manifest `only` paths to `FORBIDDEN_PATHS`; `:199-200` replays every remaining commit.
- Problem: a `modified` file may have a workspace-only baseline edit plus a later legitimate feature edit. The feature commit delta is separable by replaying from its parent; refusing solely on its pathname rejects valid cuts and makes `--allow-incomplete` the only escape, which deliberately drops code.
- Better alternative / cost / milestone: Replay normally in the disposable worktree, strip only `only` paths, and require the resulting PR diff to contain only the selected post-fork change. If it cannot apply cleanly, report a genuine conflict. Moderate; milestone 1.

### C5. `land` and `adopt` require current-tip ancestry, rejecting ordinary branches after workspace sync

- Requirement: **WTREE-06** “after proving the branch descends from the workspace branch”; **WTREE-08** “after checking it descends from the workspace branch”
- Prior art: `wsx/scripts/workspace/cut-pr-branch.sh:149` “`git merge-base "$CURRENT_BRANCH" "$WORKSPACE_BASE"`”
- Problem: create a feature branch at workspace commit `C`; then sync workspace to `W`; feature `F` still descends from `C`, but `W` is not an ancestor of `F`. Both commands refuse even though a normal merge can carry `F` and `W` together. This makes the advertised standalone completion/adoption flow needlessly fragile.
- Better alternative / cost / milestone: Record the branch’s creation base for `new`; for `adopt`, validate an unambiguous common workspace ancestor rather than requiring the current workspace tip. Let the normal merge detect real conflicts. Add the `C → {W,F}` control. Low to moderate; milestone 1.

## SUSPECTED

### S1. New-ref range semantics are under-specified for separate base and push remotes

- Requirement: **GUARD-01** “The range is bounded by the remote ref’s old value and `<base-remote>/<integration>`”
- Prior art: `oracle-devflow/hooks/pre-push:90-105` distinguishes an existing remote tip from an all-zero new-ref tip, using `--not --remotes=origin` for the latter.
- Problem: the requirement does not define the all-zero remote-tip algorithm when the integration remote and push remote differ. Using the base remote for exclusion can over-scan unrelated commits; using only the integration base can miss the exact “commits introduced to this push” boundary.
- Better alternative / cost / milestone: Specify: existing refs use `old..new`; new refs enumerate `new` minus refs known at the configured **push** remote, with unreadable objects failing closed. Add fork-layout controls for both a clean new branch and a known-positive personal-path branch. Low; milestone 1.

## NO CHANGE NEEDED

- **PRCUT-03/06 temporary detached replay** is a sound improvement over mutating the caller’s checkout; retain it after replacing gix as the conflict authority in C1.
- **WSYNC-01 from any worktree** is sound: targeting the workspace checkout is more useful than the oracle’s cwd refusal, provided C3’s non-interactive command rule is added.
- **PRCUT-02 refusal of a rename/copy from a personal path into a shared path** is sound; it prevents content leakage under a new name.
- **WTREE-06/08 as standalone commands** are justified; only their current-tip ancestry predicate needs correction.
- The oracle’s `--no-verify` should not be retained; C2 is a specification-and-harness gap, not a reason to preserve the bypass.

Static review only: no fixture or implementation was created, so these are specification and oracle-comparison findings, not runtime test results.


