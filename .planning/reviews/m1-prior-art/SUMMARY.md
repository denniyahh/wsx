# Milestone 1 requirements: "improve over prior art" external review

**Date:** 2026-10-20
**Targets:** REQUIREMENTS.md (71 v1 at `09deb52`), PROJECT.md, research/WORKSPACE-MODE.md and DEVFLOW-SETUP.md, checked against `scripts/workspace/*.sh`, `bin/wsx` (0.1.0) and devflow's `check-workspace-divergence.sh`, `pre-commit` and `pre-push` (devflow `d69c268`, copied beside the snapshot)
**Lens:** owner mandate: improve on devflow and 0.1.0; never keep worse behaviour to match them.
**Lanes:** agy (gemini-3.8-flash-high): 9 confirmed + 1 suspected. codex (gpt-5.6-terra, high, read-only): 5 confirmed + 1 suspected. Both exit 0; none dropped. Static review only: no fixtures were run by the lanes.

## Accepted and folded in (now 72)

| Finding | Lanes | Change |
|---|---|---|
| Refusing replayed changes to `modified` paths was wrong: cherry-pick applies only the commit's delta, so the edits are separable | both | PRCUT-02: replay normally, list them in the cut report for review |
| `land`/`adopt` required the current workspace tip as ancestor, refusing every branch made before the last sync | both | WTREE-06/08: "made from workspace" = contains a commit on the workspace branch and not on the integration branch |
| gix conflict prediction can't run merge drivers or rerere, so it would refuse cuts git completes | codex | PRCUT-03: git cherry-pick in the temporary worktree is the authority; merge-driver scenario |
| Fidelity as whole-branch diff equality always fails with `--scope` when the source carries other work | agy | PRCUT-05: commit coverage over the source branch's own commits, plus the oracle's no-unexplained-paths check |
| Sync/land from another worktree would leave that checkout mid-merge | agy | WSYNC-01/02, WTREE-06: abort there and report; conflict state only when run in the workspace worktree |
| Allowing a dirty source worktree silently omits uncommitted work | agy | PRCUT-06: refuse unless `--allow-dirty` (the checkout is still never touched) |
| devflow refuses commits on `develop`/`main` at commit time; wsx had deferred it | agy | GUARD-05 moved to v1, configurable via `wsx.protectedBranches` |
| DIV-03 copied devflow's `scripts/*.sh` executable rule | agy | DIV-03: only files under a tracked `core.hooksPath` |
| HARN-03 compared only the subject line | agy | full message, trailers, committer name/email |
| Merges must not open an editor | codex | `--no-edit` with a fixed message. The claim that null stdin doesn't prevent an editor is **wrong** (host test, git 2.56: null stdin merged without the editor; tty control invoked it); kept as cheap hardening |

## Rejected

- **codex S1, exclude refs known at the push remote for new-ref ranges:** that reintroduces the bypass reproduced in `m1-ws-adversarial` (remote workspace refs would mask a workspace-derived branch). GUARD-01's base-remote bound stays.
- **agy S1, read the manifest from the working tree on the workspace branch:** the divergence check compares committed branches, so the committed manifest is the consistent input, and GUARD-04 already allows personal paths on the workspace branch.
- **agy C5, GUARD-04 blocks unrecorded branches:** GUARD-04 never refused them (it refuses only integration, `main` and cut PR branches); the contradiction was WTREE-08's text, now fixed. Whether to keep `wsx adopt` at all is open with the owner.

## Disputed, decided by host

- **`--no-verify` on replayed commits:** codex says prohibit; agy says keep. Kept and stated in PRCUT-02: the originals already ran the hooks, per-commit reruns repeat slow checks and can rewrite messages, and the PRCUT-04 audit plus GUARD-01 are the boundary.
