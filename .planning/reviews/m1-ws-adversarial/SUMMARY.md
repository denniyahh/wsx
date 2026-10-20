# Workspace-mode M1 requirements: adversarial external review

**Date:** 2026-10-19
**Targets:** REQUIREMENTS.md (49 v1, re-scope draft at `8b55f25`), PROJECT.md, research/WORKSPACE-MODE.md and DEVFLOW-SETUP.md, checked against `scripts/workspace/*.sh` and devflow's `pre-push`, `pre-commit`, `check-workspace-divergence.sh` and `lint-phase-worktree.sh` (copied into the snapshot as `oracle-devflow/`)
**Lanes:** agy (gemini-3.8-flash-high), codex (gpt-5.6-terra, high, read-only), pi (deepseek-flash). All returned (exit 0); none dropped.
**Depth:** agy 8 confirmed + 2 suspected, codex 8 + 2, pi 10 + 3. Overlap is heavy: the first four themes below were found by all three lanes.

## Reproduced by the host (scratch repo, git 2.56)

| Bypass | Result |
|---|---|
| New phase branch with no commits of its own, workspace branch already on the remote: oracle/GUARD-01 range `git log <tip> --not --remotes=origin` | **empty**, while the pushed tip tree contains `.planning/P.md` → leak |
| Merge whose conflict resolution adds `.planning/STATE.md`: `git log -1 --name-only` | **no paths**. Control `-m --first-parent` shows `.planning/STATE.md` → leak |

## Confirmed and folded into REQUIREMENTS.md (now 56)

| Theme | Lanes | Change |
|---|---|---|
| Push guard bypass on new refs (range bounded by remote workspace refs); fork layout | all 3 | GUARD-01: tip-tree check, plus a range bounded by the old remote value and `<base-remote>/<integration>` |
| Merge commits invisible to the guard | pi (reproduced) | GUARD-01: diff merges against their first parent; scenario |
| Phase cut silently drops unscoped code; fidelity only warns, and is one-directional | all 3 | PRCUT-05: fail by default on any non-personal source/PR difference; `--allow-incomplete "<reason>"` |
| Re-cut force-moves an existing PR branch (`git branch -f`) | all 3 | PRCUT-08: refuse, or `--force` with a backup ref |
| Merge commits in the cut undefined; cherry-pick of a merge fails | agy, codex | PRCUT-01: skip integration/workspace merges, refuse if they carry resolution changes, refuse other merges |
| `wsx rm` can't prove "cut"; squash merges break ancestry | all 3 | WTREE-05: landed = patch-id or content on the integration branch; archive personal paths as one commit |
| No record of which branches `new`/`cut` made | pi, agy | WTREE-06: `refs/wsx/` records; GUARD-04, `rm`, `status` use them |
| Hook has no authoritative manifest on clean branches | codex, agy | CONF-07: manifest read from the workspace tip object; configured = `wsx.sourceBranch` |
| No bootstrap for manifest-less repos (gsd-core, gsd-pi) | all 3 | MIG-01: `wsx init` with explicit classification, aborting on anything unclassified |
| Migration before cutover blinds the still-installed oracle | pi | MIG-04: refuse while legacy tooling is active, or `--keep-legacy` |
| `CLAUDE.md` is personal in devflow, shared in wsx | pi | CONF-02: built-ins are defaults; `shared <path>` override; DIV-02 rejects redundant declarations |
| Line grammar can't carry every raw path | codex | CONF-01: git-style C-quoting, plus a round-trip proptest |
| `wsx new` not idempotent once a phase has commits; no mid-phase update | agy, pi | WTREE-03: report and exit 0; `wsx update <name>` |
| ROADMAP read from the current checkout | pi | WTREE-01: read from the workspace tip |
| Vendored oracle can't run (hard-coded paths), and it was workspace-only while the gates run on `develop` | pi | HARN-01: `tests/oracle/` on `develop`, one documented patch, self-tests |
| Signing makes commit SHAs unreproducible | pi | HARN-03: compare subject + author + date + tree; signatures asserted separately |
| Undeclared new path replayed into a PR | pi | PRCUT-09: cut runs the divergence check first |
| Rename or copy from a personal path into a shared path leaks content | codex (suspected, accepted) | PRCUT-02: refuse |
| In-memory conflict prediction as a whole-branch merge is wrong | agy (suspected, accepted) | PRCUT-03: per-commit replay with stripping; one restore path for every failure |
| Integration branch rewritten | codex (suspected, accepted) | WSYNC-04 refuses; recovery command is v2 WSYNC-05 |
| Symlinked hooks (mode 120000) | agy | DIV-03 accepts symlinks to tracked executables |
| Submodules not initialised in new worktrees | agy | v2 WTREE-07 |

## Rejected or corrected

- **pi #9 "the oracle does not sign":** wrong. The oracle commits with `git commit -C`, which honours `commit.gpgsign`. The reproducibility point stands (HARN-03).
- **pi #4 "`oracle-devflow/` is an undeclared personal path, live leak":** an artifact of the review snapshot. The host copied devflow's scripts there; the directory isn't in the repo. The underlying point (an undeclared new path gets replayed) was accepted as PRCUT-09.
- **pi #11 detail:** the harm assumed a user resolving the ambiguity by picking a branch. Accepted only as CONF-05 tightening.
