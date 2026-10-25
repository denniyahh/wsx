#!/usr/bin/env bash
# One-time local setup for a clone of this repository: register the repo's own
# secret scan and copy the tracked git policy (.gitconfig) into local config.
#
#   scripts/setup-repo.sh [all]   register the hook and apply the policy
#   scripts/setup-repo.sh policy  apply the policy only
#   scripts/setup-repo.sh check   exit 1 if local config differs from the policy
#
# Why the repository scans its own commits (SUP-03): a contributor's global
# pre-commit hook is not something this repository can rely on. A fresh clone,
# another machine or another contributor would otherwise commit unscanned.
# `policy` exists for a clone whose owner already scans every commit with a
# global hook and does not want the scan twice.
#
# Why a config-based hook: git 2.54+ runs every `hook.<name>` entry for an
# event alongside the `core.hooksPath` (or .git/hooks) hook and any global
# config hooks, so the repo hook is added beside them without a chaining shim
# and without touching anyone's own hooks. Older git ignores `hook.<name>`
# silently, which is why this script refuses to run there.
#
# Why the policy is copied, not included: `include.path ../.gitconfig` makes git
# read the tracked file live, from whatever branch the main checkout has
# checked out. A branch could then set any key, including ones that run a
# program (`core.fsmonitor`, `core.pager`, `diff.external`), on the next
# `git status`. Copying applies only the keys in POLICY_KEYS, only when this
# script runs, and an older clone's include is removed. The cost: a change to
# .gitconfig takes effect when this script is run again; `check` reports it.
#
# Idempotent: running it again changes nothing. It writes local config only
# (.git/config), never a tracked file and never global config.
#
# `git commit --no-verify` skips this hook, as it skips every pre-commit hook;
# GitHub push protection is the backstop for that.
#
# Undo:
#   git config --local --remove-section hook.gitleaks-staged
#   git config --local --unset <key>   for each key in .gitconfig
#
# Exit 2 on a usage error, 1 on any failure or, for `check`, any drift.
set -euo pipefail

CDPATH="" cd -- "$(dirname "${BASH_SOURCE[0]}")/.."

# Must not be a hook event name: `hook.pre-commit.*` is a fatal config error.
readonly HOOK_NAME=gitleaks-staged
# Locked by SUP-03; do not change without changing the requirement.
readonly HOOK_COMMAND="gitleaks git --pre-commit --staged --redact"
readonly HOOK_EVENT=pre-commit
readonly POLICY_FILE=.gitconfig
# The only keys the policy may set. None of them runs a program; a key outside
# this list refuses the whole run, so a change to .gitconfig cannot widen it.
readonly POLICY_KEYS='^(gpg\.format|commit\.gpgsign|tag\.gpgsign|gitflow\.branch\.(master|develop)|gitflow\.prefix\.(feature|release|hotfix|support|versiontag))$'
# What clones set up before the policy was copied; removed on sight.
readonly LEGACY_INCLUDE=../.gitconfig
readonly MIN_GIT_MAJOR=2
readonly MIN_GIT_MINOR=54

usage() {
    echo "usage: scripts/setup-repo.sh [all|policy|check]" >&2
    exit 2
}

if [ "$#" -gt 1 ]; then
    usage
fi
mode="${1:-all}"
case "$mode" in
    all | policy | check) ;;
    *) usage ;;
esac

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

if [ "$mode" = all ] && ! command -v gitleaks >/dev/null 2>&1; then
    echo "error: gitleaks is required by the pre-commit hook; run scripts/install-tools.sh install gitleaks" >&2
    exit 1
fi

if [ ! -f "$POLICY_FILE" ]; then
    echo "error: $POLICY_FILE not found in $(pwd)" >&2
    exit 1
fi

# Read the policy as git parses it. With -z each entry is "key\nvalue\0", so a
# value cannot be mistaken for a key; an entry with no newline is a bare
# boolean key, which the policy never uses.
policy_listing="$(mktemp)"
trap 'rm -f "$policy_listing"' EXIT
git config -z --no-includes -f "$POLICY_FILE" --list >"$policy_listing"

policy_keys=()
policy_values=()
while IFS= read -r -d '' entry; do
    if [[ $entry != *$'\n'* ]]; then
        echo "error: $POLICY_FILE: '$entry' has no value" >&2
        exit 1
    fi
    key="${entry%%$'\n'*}"
    value="${entry#*$'\n'}"
    if [[ ! $key =~ $POLICY_KEYS ]]; then
        echo "error: $POLICY_FILE sets '$key', which is not a policy key; nothing was changed" >&2
        exit 1
    fi
    for seen in "${policy_keys[@]}"; do
        if [ "$seen" = "$key" ]; then
            echo "error: $POLICY_FILE sets '$key' more than once" >&2
            exit 1
        fi
    done
    policy_keys+=("$key")
    policy_values+=("$value")
done <"$policy_listing"

if [ "${#policy_keys[@]}" -eq 0 ]; then
    echo "error: $POLICY_FILE sets no keys" >&2
    exit 1
fi

# `git config --get*` exits 1 when the key is unset; anything else is an error.
local_get() {
    local status=0
    git config --local "$@" || status=$?
    if [ "$status" -ne 0 ] && [ "$status" -ne 1 ]; then
        echo "error: git config --local $* failed (exit $status)" >&2
        exit 1
    fi
}

# Read at top level, where `set -e` stops on a failed read; inside an `if`
# condition it would not.
local_includes="$(local_get --get-all include.path)"
legacy_include=0
if grep -qxF -- "$LEGACY_INCLUDE" <<<"$local_includes"; then
    legacy_include=1
fi

if [ "$mode" = check ]; then
    drift=()
    for i in "${!policy_keys[@]}"; do
        key="${policy_keys[$i]}"
        current="$(local_get --get "$key")"
        if [ "$current" != "${policy_values[$i]}" ]; then
            drift+=("$key: local '${current}', policy '${policy_values[$i]}'")
        fi
    done
    if [ "$legacy_include" -eq 1 ]; then
        drift+=("include.path $LEGACY_INCLUDE: still reads $POLICY_FILE live")
    fi
    if [ "${#drift[@]}" -ne 0 ]; then
        {
            echo "error: local git config differs from $POLICY_FILE:"
            for line in "${drift[@]}"; do
                echo "  $line"
            done
            echo "Run scripts/setup-repo.sh policy to apply it."
        } >&2
        exit 1
    fi
    echo "==> local git config matches $POLICY_FILE (${#policy_keys[@]} keys)"
    exit 0
fi

if [ "$mode" = all ]; then
    git config --local "hook.$HOOK_NAME.command" "$HOOK_COMMAND"
    git config --local --replace-all "hook.$HOOK_NAME.event" "$HOOK_EVENT"

    # Trust git's own view, not the config we just wrote: if git does not list
    # the hook, commits are not being scanned.
    if ! hook_list="$(git hook list "$HOOK_EVENT")" ||
        ! grep -qxF -- "$HOOK_NAME" <<<"$hook_list"; then
        echo "error: hook registration not visible to git ('git hook list $HOOK_EVENT' does not list $HOOK_NAME)" >&2
        exit 1
    fi
    echo "==> registered $HOOK_EVENT hook $HOOK_NAME: $HOOK_COMMAND"
fi

# include.path is multi-valued: remove only ours, never anyone else's.
if [ "$legacy_include" -eq 1 ]; then
    git config --local --fixed-value --unset-all include.path "$LEGACY_INCLUDE"
    echo "==> removed include.path $LEGACY_INCLUDE (the policy is copied now, not read live)"
fi

# --replace-all: a key that already has several local values would otherwise
# make git refuse (exit 5) part-way through, after earlier keys were written.
for i in "${!policy_keys[@]}"; do
    git config --local --replace-all "${policy_keys[$i]}" "${policy_values[$i]}"
done

# Trust git's view again: read every key back.
for i in "${!policy_keys[@]}"; do
    key="${policy_keys[$i]}"
    current="$(local_get --get "$key")"
    if [ "$current" != "${policy_values[$i]}" ]; then
        echo "error: $key reads back as '$current', not '${policy_values[$i]}'" >&2
        exit 1
    fi
done
echo "==> copied ${#policy_keys[@]} keys from $POLICY_FILE into local config; run this again after $POLICY_FILE changes"
