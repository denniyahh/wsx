# Workspace-branch mode: targeted research

**Date:** 2026-10-19. **Why:** milestone 1 was re-scoped from copy-sync to the workspace-branch
model (PROJECT.md Key Decisions). This answers the three open questions before requirements are
redrafted. Inputs: the gix 0.89.0 source in the local cargo registry, a scratch-repo experiment
with git 2.56.0, and the four repos on this machine that use a workspace branch. See also
`DEVFLOW-SETUP.md`.

## R1. gix vs the git binary

**Finding.** gix 0.89.0 has in-memory `merge_trees` / `merge_commits` (returns a tree and a list
of conflicts), `merge_base*`, revision walks, tree diffs and commit-signing options. It has
**no** cherry-pick, no rebase, no porcelain merge that leaves conflict state (index stages,
`MERGE_HEAD`, conflict markers) for the user to resolve, and **no hook execution** (0 hook
call sites; already recorded in STACK.md).
*(Source read: `gix-0.89.0/src/repository/merge.rs:126,181`, `revision.rs:57-150`,
`config/mod.rs:158-166`; `rg` for cherry/rebase in gix and gix-merge 0.22.0 returned nothing.)*

**Consequence.** The workspace model mutates history: it merges the integration branch into the
workspace, replays commits to cut PR branches, creates branches and worktrees, and commits.
Those operations must honour the user's own hooks (commit-msg, gitleaks), signing, merge drivers
and rerere, and leave conflicts in a state normal git tooling can resolve. Only the `git` binary
does all of that.

| Operation | Use | Why |
|---|---|---|
| merge integration → workspace, cherry-pick replay, `worktree add -b`, commit, fetch | **git binary** | hooks, signing, conflict state, credentials |
| guard range walks, path classification, divergence diff, merge-base, ahead/behind | **gix** | read-only, fast, no subprocess parsing |
| ~~predict cut-PR conflicts before touching anything~~ | ~~gix `merge_commits` in memory~~ | Dropped 2026-10-20: gix can't run merge drivers, so it would refuse cuts git completes. The cut replays with git in a temporary worktree instead (PRCUT-03) |

This stays within the existing constraint (the git binary is allowed where gix lacks the
operation), but in this mode the git binary does most of the writing, not a few edge cases.

## R2. Installing hooks without shadowing the user's

**Finding (executed, git 2.56.0, scratch repo, isolated `GIT_CONFIG_GLOBAL`).** Config-based hooks
(`hook.<id>.command` + `hook.<id>.event`) run **in addition to** a `core.hooksPath` hook. They
don't replace it.

| Case | Result |
|---|---|
| global `core.hooksPath` pre-commit only (control) | hooksPath hook ran, commit ok |
| + config pre-commit hook | **both ran** (config first), commit ok |
| config hook exits 1 | both ran, **commit refused (rc 1)** |
| same, from a linked worktree | config hook applies there too (repo config is shared) |
| pre-push config hook + global hooksPath pre-push | **both received the full ref list on stdin** (1 line each) and the remote args |
| failing pre-push config hook | **push refused (rc 1)** |
| `git hook list pre-commit` | `wsxguard`, `hook from hookdir` |

**Consequences:**
- wsx installs its hooks as repo-level config entries. That writes no tracked file, never
  overwrites a hook file, and coexists with husky, `core.hooksPath` and the author's global
  gitleaks hook. This removes the reason 0.1.0's guard never armed on this machine.
- **The pre-push guard can read the real pushed refs from stdin.** That closes the gap 0.1.0 and
  devflow both live with (`git push origin <other-branch>` was checked against HEAD).
- Git appends the hook arguments to the command string, so the command must be a plain program
  invocation (`wsx hook pre-push`). A shell snippet ending in `exit 1` broke here (`exit: too many
  arguments`). Write the implementation tests against this.
- Needs git ≥ 2.54. M1 is the author's Linux machine (2.56). A fallback for older git (chaining
  from a `core.hooksPath` hook) belongs to the public release.

## R3. What has to be configurable (four repos on this machine)

| Repo | Workspace branch | Integration branch | Remotes | Manifest | Personal paths seen |
|---|---|---|---|---|---|
| devflow | `workspace/denniyahh` | `develop` (origin/HEAD is `main`) | origin | `.workspace-divergence` | `.planning`, `graphify-out`, `scripts/*` (declared), `skills-lock.json`, `devflow.toml` |
| wsx | `workspace/denniyahh` | `develop` (origin/HEAD is `main`) | origin | `.workspace-divergence` | `.planning`, `scripts/workspace` |
| gsd-core | `personal/workspace` (+ `-backup`) | `next` | **origin = fork, upstream = canonical** | none | `.planning`, `scratch`, `AGENTS.md`, `.agents`, `.mcp.json`, `mise.toml`; six shared files modified, undeclared |
| gsd-pi | `personal/workspace` | `main` | origin | none | `scripts`, `scratch`; main checkout currently on a `fix/` branch |

**Consequences:**
- **The integration branch can't be inferred from `origin/HEAD`.** It's `main` in both git-flow
  repos, where the integration branch is `develop`. It needs config (0.1.0 already has
  `wsx.baseRef`) and a detection chain that prefers `develop`/`next` over the default branch.
- **The integration branch can be on a different remote from where you push.** In gsd-core's fork
  layout, PR branches are cut from `upstream/next` and pushed to `origin`. devflow's
  `cut-pr-branch.sh` assumes one remote (`BASE_REMOTE` env only).
- **Workspace branch names vary** (`workspace/<handle>`, `personal/workspace`). 0.1.0 has
  `wsx.sourceBranch`; keep it, and detect "the only local `workspace/*` or `personal/*` branch"
  as devflow does.
- **Personal-path lists vary per repo, and two of the four repos declare nothing.** Their
  workspaces already carry undeclared changes to shared files, exactly what a divergence check
  exists to surface. The built-in list (agent dirs, `.planning`, `scratch`?) has to be
  extendable per repo.
- **Phase naming is not wsx's concern** (boundary decision, 2026-10-19). devflow and GSD use
  `feature/phase-{N}`; devflow passes the branch name to `wsx new` and the scope to `wsx cut`.
  wsx reads no GSD config.
