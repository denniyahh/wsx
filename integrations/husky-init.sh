#!/usr/bin/env sh
# Personal, machine-wide Husky hook extension point.
# Install to: ~/.config/husky/init.sh
#
# Husky sources this file before EVERY husky-managed hook in EVERY repo on this
# machine, with $n set to the hook name. See:
# https://typicode.github.io/husky/how-to.html#with-a-personal-hooks-file
#
# IMPORTANT: this file is *sourced*, so it shares stdin with the hook that runs
# after it. Hooks such as pre-push read their refs from stdin, so nothing here
# may consume stdin -- hence the `</dev/null` redirect below. Without it, a
# pre-push hook that reads refs silently sees an empty list.

# --- personal workspace sync (wsx) -----------------------------------------
#
# Repos that carry a .wsx.conf on their personal-workspace branch keep planning
# docs, agent rule files and tooling state out of every shared branch. wsx:
#
#   pre-commit  blocks those paths from being committed, then syncs local
#               edits back to the personal-workspace worktree
#   pre-push    same guard across the outgoing commit range, then syncs back
#   post-merge  refreshes this worktree's copies
#
# It fails open: any error other than the guard is reported and ignored, so a
# sync problem can never block a commit. Repos without a .wsx.conf no-op.
#
# Escape hatch: WSX_SKIP=1 git commit ...
if [ "${WSX_SKIP:-0}" != "1" ] && [ -x "$HOME/.local/bin/wsx" ]; then
	"$HOME/.local/bin/wsx" hook "$n" </dev/null || exit 1
fi
