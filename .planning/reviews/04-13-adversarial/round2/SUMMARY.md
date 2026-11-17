# 04-13 Tier A adversarial review, round 2

Lanes: codex (gpt-6.1-sol, high), exit 0, 3 confirmed and 1 suspected; pi (deepseek-flash), exit 0,
4 confirmed and 4 suspected. Snapshot: `git archive` of 4e79ac6.

## Confirmed

- **Critical (codex 1):** gix maps `main-worktree/<name>` to `<common>/<name>`
  (`gix-ref-0.69.1/src/store/file/find.rs:270`, the `MainRef | MainPseudoRef` arm). The planned
  symlink check looks at `<common>/main-worktree/<name>`, so a symlinked ref behind that alias still
  redirects the manifest. The orchestrator checked the source. Codex measured git exit 128 while wsx
  answered `Configured` with `only b`. Even with a regular file behind the alias, gix resolves a
  source git rejects.
- **Major (codex 2):** gix reads `HEAD` at open (`gix-0.90.0/src/open/repository.rs:302`) for
  `includeIf.onbranch`. A symlinked `HEAD` changes which conditional config gix loads (codex measured
  remotes `None` where git loads `origin`). In a linked worktree it fails `discover`.
- **Major (codex 3):** a dangling text symref under `refs/heads/workspace/` is listed by gix and
  omitted by git (`src/git/refs.rs:265` already documents it). Candidate parity is therefore still
  false.
- **pi 1-4 (major/minors):** the `git_dir` half of the symlink rule is never exercised; the routing
  of the new error kind contradicts the plan's own truth; the controls are miscounted; a fifo loose
  ref hangs both git and gix (parity).

## Suspected

- **codex 4:** a check/open race between `symlink_metadata` and gix's `File::open`.
- **pi 5-8:** listing-failure routing, symlinked namespace dirs, ENOTDIR-only unreadable arm, and a
  cross-worktree private-store write.

## Assessment

Each round has found another place where gix maps or reads a ref differently from git: replace refs,
then worktree-private stores, then symlinks, now name aliases, `HEAD` at open and dangling symrefs.
Refusing shapes one by one has not converged. The strategy is raised to the operator before round 3.

## Operator decision (2026-11-17)

Pivot. 04-13 as planned is not executed. The model's inputs (refs, listings, the manifest blob and
the `wsx.*` and remote config) move to the `git` binary, so they match git by construction. gix keeps
object work by commit id (merge-base, walks, tree diffs). To be recorded through `/gsd-discuss-phase 04`,
proved by a spike (every shape from both review rounds, with negative controls, plus a latency
measurement), then 04-13 is replanned.
