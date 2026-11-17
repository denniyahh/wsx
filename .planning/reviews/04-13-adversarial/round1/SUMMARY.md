# 04-13 Tier A adversarial review, round 1

Lanes: codex (gpt-6.1-sol, high), exit 0, 4 findings; pi (deepseek-flash), exit 0, 4 findings plus 4 suspected.
Snapshot: `git archive` of db35b5c.

## Confirmed, folded into 04-13

- **Critical (codex 1):** a loose ref that is a symlink is read differently by git and gix. With
  `core.preferSymlinkRefs`, git reads a symlink whose target starts with `refs/` as a symbolic ref.
  gix's `ref_contents` (`gix-ref-0.69.1/src/store/file/find.rs:338`) opens the path with
  `File::open`, which follows the link on the filesystem, so it reads another ref's contents. The
  orchestrator checked the gix source; codex measured the manifest redirect (git `only .planning`, wsx
  `only b`). Fold: refuse by presence (D-22 shape) when a ref wsx reads or lists is a symlink.
- **Major (codex 2):** `SortedLoosePaths` (`loose/iter.rs:47`) skips non-regular files, so gix drops
  a symlinked branch git lists, and D-21's ambiguity check is lost. The same symlink refusal on the
  listed prefixes closes it.
- **Major (pi 1, codex 3):** the allow-list matches names, not directories, so a regular file named
  `bisect`, `rewritten` or `worktree` in a linked worktree's `refs/` passes, and gix lists it while git
  does not. Fold: exempt an entry only when `symlink_metadata` says it is a real directory.
- **Major (pi 2, 3):** T-04-46 and the doc sentence claim more than the placement covers. Object
  reads outside `build`/`root_entry` (`peel_commit`, `reads.rs`) and `Repo::siblings()` repos are
  unguarded, and gix builds its replacement map at open. Fold: narrow the claims to the guarded paths
  and extend STATE's phase-3/5 obligation line to the linked-worktree store and to sibling repos.
- **Minor (pi 4):** the "least entry name" clause and the "whatever kind" clause are untested. Fold: one test with two strays,
  one regular file and one directory, asserting the name.
- **Minor (codex 4, suspected):** `root_entry`'s own call has no independent negative control. Fold:
  a deletion check on `root_entry`'s call with a fixture that reaches it.

## Recorded, not folded

- pi suspected 3: the unreadable arm is tested only as ENOTDIR. Folded only as message wording if
  cheap; otherwise backlog.
- pi suspected 4: `Run wsx init.` is a weak remedy for an unconfigured repo with a stray ref. The
  message already names the fix; backlog.
