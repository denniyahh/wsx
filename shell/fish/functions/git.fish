# Narrow safety net around `git worktree remove`.
#
# Removing a worktree deletes its personal-state copies, and those copies are
# the only place their edits exist until `wsx push` runs. Everything else is
# recoverable; this is not. So this wrapper intercepts exactly that one
# subcommand in repos that use wsx, routes it through `wsx rm` (push first,
# then remove), and passes every other git invocation through untouched.
#
# It is a fish function, so it applies only to interactive use -- scripts,
# hooks and other programs still call the real git binary directly.
#
# Escape hatch: `WSX_SKIP=1 git worktree remove ...` runs plain git.

function git --wraps git --description 'git, with a wsx guard on worktree removal'
    if test "$argv[1]" != worktree; or test "$argv[2]" != remove
        command git $argv
        return $status
    end

    if test "$WSX_SKIP" = 1; or not command -q wsx
        command git $argv
        return $status
    end

    # Only intervene where a personal workspace is actually configured.
    set -l source_branch (command git config --get wsx.sourceBranch 2>/dev/null)
    test -n "$source_branch"; or set source_branch personal-workspace
    set -l manifest ''
    for line in (command git worktree list --porcelain 2>/dev/null)
        switch $line
            case 'worktree *'
                set -f candidate (string replace 'worktree ' '' -- $line)
            case "branch refs/heads/$source_branch"
                set manifest "$candidate/.wsx.conf"
        end
    end
    if test -z "$manifest"; or not test -f "$manifest"
        command git $argv
        return $status
    end

    set -l rest $argv[3..-1]
    set -l forwarded
    for arg in $rest
        switch $arg
            case --force -f
                set -a forwarded --force
            case '*'
                set -a forwarded $arg
        end
    end

    echo "git: routing through 'wsx rm' so this worktree's personal state is saved first." >&2
    echo "     (WSX_SKIP=1 git worktree remove ... to bypass)" >&2
    wsx rm $forwarded
    return $status
end
