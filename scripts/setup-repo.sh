#!/usr/bin/env bash
# One-time local setup for a clone of this repository: register the repo's own
# secret scan and pull in the tracked git policy (.gitconfig).
#
# Why the repository scans its own commits (SUP-03): a contributor's global
# pre-commit hook is not something this repository can rely on. A fresh clone,
# another machine or another contributor would otherwise commit unscanned.
#
# Why a config-based hook: git 2.54+ runs every `hook.<name>` entry for an
# event alongside the `core.hooksPath` (or .git/hooks) hook and any global
# config hooks, so the repo hook is added beside them without a chaining shim
# and without touching anyone's own hooks. Older git ignores `hook.<name>`
# silently, which is why this script refuses to run there.
#
# Idempotent: running it again changes nothing. It writes local config only
# (.git/config), never a tracked file and never global config.
#
# `git commit --no-verify` skips this hook, as it skips every pre-commit hook;
# GitHub push protection is the backstop for that.
#
# Undo:
#   git config --local --remove-section hook.gitleaks-staged
#   git config --local --unset include.path '^\.\./\.gitconfig$'
#
# Exit 2 on a usage error, 1 on any failure.
set -euo pipefail

cd "$(dirname "${BASH_SOURCE[0]}")/.."

# Must not be a hook event name: `hook.pre-commit.*` is a fatal config error.
readonly HOOK_NAME=gitleaks-staged
# Locked by SUP-03; do not change without changing the requirement.
readonly HOOK_COMMAND="gitleaks git --pre-commit --staged --redact"
readonly HOOK_EVENT=pre-commit
# Relative to the common git dir, so linked worktrees read the main checkout's file.
readonly INCLUDE_PATH=../.gitconfig
readonly MIN_GIT_MAJOR=2
readonly MIN_GIT_MINOR=54

if [ "$#" -ne 0 ]; then
    echo "usage: scripts/setup-repo.sh   (takes no arguments)" >&2
    exit 2
fi

if [ "$(git rev-parse --is-inside-work-tree 2>/dev/null)" != true ]; then
    echo "error: $(pwd) is not inside a git work tree" >&2
    exit 1
fi

# Every refusal below happens before any config is written.
version_line="$(git --version)"
if [[ ! "$version_line" =~ ^git\ version\ ([0-9]+)\.([0-9]+) ]]; then
    echo "error: cannot parse the git version from '$version_line'" >&2
    exit 1
fi
git_major="${BASH_REMATCH[1]}"
git_minor="${BASH_REMATCH[2]}"
if [ "$git_major" -lt "$MIN_GIT_MAJOR" ] ||
    { [ "$git_major" -eq "$MIN_GIT_MAJOR" ] && [ "$git_minor" -lt "$MIN_GIT_MINOR" ]; }; then
    echo "error: git $git_major.$git_minor is older than $MIN_GIT_MAJOR.$MIN_GIT_MINOR, which ignores hook.<name> config and would leave commits unscanned" >&2
    exit 1
fi

if ! command -v gitleaks >/dev/null 2>&1; then
    echo "error: gitleaks is required by the pre-commit hook; run scripts/install-tools.sh install gitleaks" >&2
    exit 1
fi

git config --local "hook.$HOOK_NAME.command" "$HOOK_COMMAND"
git config --local --replace-all "hook.$HOOK_NAME.event" "$HOOK_EVENT"

# include.path is multi-valued: add ours only when absent, and never replace
# or reorder anyone else's. `git config --get-all` exits 1 when the key is
# unset; any other non-zero status is a real error.
include_status=0
existing_includes="$(git config --local --get-all include.path)" || include_status=$?
if [ "$include_status" -ne 0 ] && [ "$include_status" -ne 1 ]; then
    echo "error: could not read include.path from local config (git config exit $include_status)" >&2
    exit 1
fi
if ! grep -qxF -- "$INCLUDE_PATH" <<<"$existing_includes"; then
    git config --local --add include.path "$INCLUDE_PATH"
fi

# Trust git's own view, not the config we just wrote: if git does not list the
# hook, commits are not being scanned.
if ! hook_list="$(git hook list "$HOOK_EVENT")" ||
    ! grep -qxF -- "$HOOK_NAME" <<<"$hook_list"; then
    echo "error: hook registration not visible to git ('git hook list $HOOK_EVENT' does not list $HOOK_NAME)" >&2
    exit 1
fi

echo "==> registered $HOOK_EVENT hook $HOOK_NAME: $HOOK_COMMAND"
echo "==> include.path $INCLUDE_PATH is set; git resolves it against the common git dir, so every worktree reads .gitconfig from the main checkout"
