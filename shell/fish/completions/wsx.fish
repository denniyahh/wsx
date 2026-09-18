# Completions for wsx (personal workspace sync).
#
# Discoverability is the point: `wsx <TAB>` lists the verbs and describes them,
# so day-to-day use needs no memory of the command surface.

set -l wsx_commands setup pull push sync status list doctor new rm resolve guard hook

function __wsx_no_subcommand -V wsx_commands
    not __fish_seen_subcommand_from $wsx_commands
end

# Managed paths, for `wsx resolve <TAB>` -- only those with a live conflict.
function __wsx_conflicts
    wsx status 2>/dev/null | string match -r '^\s+conflict\s+\S+' | string replace -r '^\s+conflict\s+' ''
end

function __wsx_worktrees
    git worktree list --porcelain 2>/dev/null | string match -r '^worktree .*' | string replace 'worktree ' ''
end

complete -c wsx -f

complete -c wsx -n __wsx_no_subcommand -a setup -d 'Prepare a worktree: copy state in, install ignore + guard'
complete -c wsx -n __wsx_no_subcommand -a pull -d 'Apply source -> worktree changes'
complete -c wsx -n __wsx_no_subcommand -a push -d 'Apply worktree -> source changes'
complete -c wsx -n __wsx_no_subcommand -a sync -d 'Apply changes in both directions'
complete -c wsx -n __wsx_no_subcommand -a status -d 'Show pending changes and conflicts'
complete -c wsx -n __wsx_no_subcommand -a list -d 'Show every worktree and its sync state'
complete -c wsx -n __wsx_no_subcommand -a doctor -d 'Verify guards, ignores and hooks are active'
complete -c wsx -n __wsx_no_subcommand -a new -d 'Create a fully set-up dev worktree'
complete -c wsx -n __wsx_no_subcommand -a rm -d 'Push personal state back, then remove a worktree'
complete -c wsx -n __wsx_no_subcommand -a resolve -d 'Settle a conflict'

complete -c wsx -n '__fish_seen_subcommand_from setup pull push sync' -s n -l dry-run \
    -d 'Show what would change without writing'

complete -c wsx -n '__fish_seen_subcommand_from resolve' -a '(__wsx_conflicts)' -d 'Conflicted path'
complete -c wsx -n '__fish_seen_subcommand_from resolve' -l mine -d "Keep this worktree's copy"
complete -c wsx -n '__fish_seen_subcommand_from resolve' -l theirs -d "Take the source worktree's copy"

complete -c wsx -n '__fish_seen_subcommand_from new' -l base -r -d 'Base ref (default: origin/devtest)'
complete -c wsx -n '__fish_seen_subcommand_from new' -l no-fetch -d 'Skip fetching the base ref first'

complete -c wsx -n '__fish_seen_subcommand_from rm' -a '(__wsx_worktrees)' -d Worktree
complete -c wsx -n '__fish_seen_subcommand_from rm' -l force \
    -d 'Remove despite conflicts or uncommitted changes'
