# Changelog

All notable changes to wsx are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] — 2026-09-18

First extraction into a standalone repository. Developed in place against a
multi-service work repository, where it replaced a symlink-based scheme.

### Added

- Three-way sync for text (`sync`) against a per-worktree base snapshot, so a
  file changed in both places is reported as a conflict rather than silently
  overwritten. The source version is written alongside as `<file>.wsx-remote`
  for inspection, and `wsx resolve --mine|--theirs` settles it.
- `lww` policy for state that cannot be merged (SQLite databases, append-only
  logs): newest mtime wins and the overwritten copy is kept under
  `<git-dir>/wsx/backups/`.
- `pull` policy for vendored, read-only reference content.
- `skip` policy, plus longest-match precedence, so a nested entry can carve an
  unmergeable path out of an otherwise synced tree.
- Deletion propagation in both directions.
- Per-worktree ignore rules via `extensions.worktreeConfig` and
  `core.excludesFile`, so managed paths are ignored in dev worktrees while
  remaining committable on `personal-workspace`. The existing global ignore
  file is inlined, since git honours only one `core.excludesFile`.
- `pre-commit` and `pre-push` guards that block managed paths on any branch
  other than `personal-workspace`.
- Automatic migration from a symlink-based layout: paths reached through a
  symlink at any level are detected and replaced with real files, and symlinked
  parent directories are materialised before writing so a write can never
  follow the link back into the source worktree.
- `wsx new` — create a worktree, arm the commit guard, and copy personal state
  in, as one command. Restores `package-lock.json`, which npm rewrites as a
  side effect, so a new worktree does not start out dirty.
- `wsx rm` — push personal state back before removing a worktree, refusing on
  unresolved conflicts or uncommitted tracked changes unless `--force`.
- `wsx list` — every worktree and its sync state.
- Self-healing: any command reinstalls missing or stale ignore rules, and the
  agent session-start hook arms the commit guard, so a worktree created with a
  bare `git worktree add` repairs itself.
- Orphaned per-worktree exclude files are pruned when their worktree is gone.
- Fish completions, and a narrow `git worktree remove` wrapper that routes
  through `wsx rm` to prevent unrecoverable loss of unpushed personal state.
- Session-boundary hooks for GitHub Copilot CLI and Claude Code.
- `tests/sandbox.sh`, a 28-assertion end-to-end suite run against a throwaway
  repository.

### Notes

- Everything except the guard fails open: a sync problem reports and moves on,
  and can never block a commit.
- The husky personal init redirects from `/dev/null`, because it is *sourced*
  and would otherwise consume the stdin that `pre-push` reads its refs from.

[Unreleased]: https://github.com/denniyahh/wsx/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/denniyahh/wsx/releases/tag/v0.1.0
