---
last_mapped_commit: 5e4217fb6a940bf5ca5f53685fd2245097150676
last_mapped_at: 2026-10-17
---
# Codebase Concerns

**Analysis Date:** 2026-10-17

Scope: `bin/wsx` (1181 lines, stdlib Python), `tests/sandbox.sh`, `install.sh`, `integrations/`, `shell/fish/`.

Evidence legend:
- **[REPRODUCED]**: confirmed with a throwaway-repo probe on 2026-10-17 (scratch repo, since deleted). Each probe had a control case that behaved correctly.
- **[CODE-READ]**: follows from reading the code. Not executed.
- **[SPECULATIVE]**: a plausible risk that has not been demonstrated.

Baseline: `./tests/sandbox.sh` gives `passed: 28 failed: 0`. The suite never touches any of the three reproduced defects below, so a green run says nothing about them.

## Known Bugs

**An unreadable local file deletes the source copy on push [REPRODUCED]. Data loss.**
- Symptoms: `chmod 000 .p/A; wsx push` prints `delete .p/A (sync: changed locally)` and removes `A` from the source worktree. The readable sibling `B` was left alone (control).
- Files: `bin/wsx:237-248` (`digest()` returns `None` on any `OSError`), `bin/wsx:406-412` (if `None` matches the base, the planner reads it as a local deletion, so the action becomes `delete-remote`), `bin/wsx:474-476`.
- Trigger: any read error on the local side: EACCES, EIO, a file locked by another process, a transient FS error. The same thing runs automatically from the `pre-commit`, `pre-push` and `session-end` hooks (`bin/wsx:1043,1051,1064`).
- Fix approach: make `digest()` tell "absent" (`FileNotFoundError`) apart from "unreadable" (any other `OSError`). For unreadable, return a sentinel or raise, and have `plan()` emit `conflict` or skip, never a delete. Add a sandbox case with `chmod 000`.

**`lww` deletes local-only files on pull, with no backup [REPRODUCED]. Data loss.**
- Symptoms: a new file created under an `lww` path in the dev worktree (`.l/newlocal`), then `wsx pull`, prints `delete .l/newlocal (lww: gone upstream)`. The file is gone and no backup is written.
- Files: `bin/wsx:396-398`. The `lww` branch never consults `base`, so "never existed upstream" and "deleted upstream" look identical.
- Trigger: any runtime state file created in a worktree (new SQLite db, new log) followed by a pull or an inbound hook (`post-checkout`, `post-merge`, `session-start`, `bin/wsx:1053-1061`).
- Fix approach: in the `lww` branch, delete locally only when `base.get(rel)` is set, i.e. the file was synced before. Otherwise emit `push`. Back up before every `lww` `delete-local`.

**Deleting a file on both sides leaves a poisoned base entry, and recreating it is reported as a conflict [REPRODUCED].**
- Symptoms: remove `.p/B` both locally and in the source, run `wsx sync`, recreate `.p/B` locally, and `wsx status` reports `conflict .p/B (changed on both sides)`.
- Files: `bin/wsx:380-382` (both hashes `None`, so the file counts as converged), `bin/wsx:429-431` (`base[rel] = digest(dst_path) or ""` writes `""`), `bin/wsx:365` (base keys stay candidates forever).
- Workaround: `wsx resolve <path> --mine`.
- Fix approach: in `apply()`, when a converged file is absent on both sides, `base.pop(rel, None)` instead of storing `""`.

**The `pre-push` guard is skipped entirely on a branch's first push [CODE-READ].**
- Files: `bin/wsx:1045-1051`. When `@{upstream}` is empty (a new branch: `git push -u origin feature`), no guard runs at all and the hook goes straight to `quiet_sync`.
- Impact: commits made with `--no-verify`, or made before the hook was armed, can push managed paths on the most common push. The docstring choice to never read stdin (`bin/wsx:1023`) is also why the pushed refs are unknown.
- Fix approach: when there is no upstream, check `<base_ref>..HEAD` (from `base_ref()`, `bin/wsx:803`) or `HEAD --not --remotes`.

**`global_ignore_file()` loses a custom global `core.excludesFile` [CODE-READ].**
- Files: `bin/wsx:496-501`. After setup, `git config --get core.excludesFile` returns the worktree-level value (`exclude.wsx-*`). The code then falls back to `$XDG_CONFIG_HOME/git/ignore` rather than the user's configured global path. A user whose `~/.gitconfig` sets `core.excludesFile = ~/.gitignore_global` has it silently dropped from the inlined block on every regeneration.
- Fix approach: read `git config --global --get core.excludesFile` (plus `--system`) explicitly.

**Smaller defects [CODE-READ]:**
- `lww` mtime tie goes to `push` (`bin/wsx:402`, strict `>`). `copy2` preserves mtime, so ties do happen.
- `wsx resolve --theirs` writes with `copy_file()` without calling `materialise_parents()` (`bin/wsx:710`), which breaks the landmine rule. It only bites if the user resolves a path under a still-symlinked parent.
- `cmd_resolve` rejects outside paths with `rel.startswith("..")` (`bin/wsx:691`). That also rejects a legitimate entry named `..foo`.
- `backup()` stamps are per second (`bin/wsx:334`). Two backups of the same `rel` within one second overwrite each other.
- `cmd_new` derives the fetch remote with `base.split("/",1)[0]` (`bin/wsx:873`). A local base such as `feature/x` gives the remote `feature`. Only a warning is printed.
- `cmd_rm` detects the main worktree with `common_dir.parent` (`bin/wsx:918`), which is wrong under `--separate-git-dir`. `resolve_worktree` returns resolved paths for existing directories but unresolved `list_worktrees` paths otherwise, so the comparisons at `bin/wsx:916-918` can mismatch through symlinked paths.
- `KeyboardInterrupt` returns 130 even in hook mode (`bin/wsx:1171-1172`), which blocks the commit. That contradicts design constraint 2. Low impact.

## Landmine Compliance (CLAUDE.md)

| Landmine | Honoured? | Evidence |
|---|---|---|
| `via_symlink()` used, not just `is_symlink()` | Yes, in `plan()` | `bin/wsx:378`, checked before the converged shortcut at `:380`. `cmd_doctor` checks only top-level manifest entries with `is_symlink()` (`:782`), so a symlink nested below an entry is not reported. |
| `materialise_parents()` before every pull write | Partly | Called in `apply()` for `pull` (`:463`). Not called for conflict-marker writes (`:438`; conflicts cannot arise via symlinks because `:386` short-circuits, so this is safe in practice) or for `resolve --theirs` (`:710`, a real gap). `delete-local` for a symlinked path (`:472`) calls `remove_file()`, whose parent `rmdir` loop (`:328-330`) can hit a symlinked ancestor; untested. |
| Base snapshot updated on every applied action | Yes, with a defect | `:465,470,473,476`, saved at `:484`. Converged absent-both entries store `""` (see Known Bugs). |
| `cmd_hook` must not raise | Effectively yes | `cmd_hook` itself only catches `Problem` (`:1036`, `:1090`). Other exceptions are caught by `main()` in hook mode (`:1173-1176`). Consequence: an unexpected exception inside `cmd_guard` also returns 0, so the guard **fails open** on errors (e.g. a manifest parse `Problem` at `:1164-1168`). |
| Never write tracked files | Mostly | wsx writes only `.git/info/exclude.wsx-*`, per-worktree config and `.git/.../wsx/`. Exceptions: `arm_husky()` runs `npm install` in the worktree (`:834-839`), which runs arbitrary `prepare` scripts and may touch tracked files. Only `package-lock.json` is restored (`:842-844`), and only when it existed beforehand. `git config extensions.worktreeConfig true` (`:563`) changes shared repo config for every worktree. |

## Tech Debt

**Dead code:**
- `where` is computed and then discarded with `_ = where` (`bin/wsx:453,481`).
- `if line is not None` in `render_exclude` cannot filter anything (`bin/wsx:536`).
- Conditional expressions are used as statements: `backup(...) if digest(...) else None` (`bin/wsx:702,709`).

**Stringly-typed actions:**
- `Action.kind` and `policy` are free strings (`bin/wsx:346-353`). Direction mapping (`"in" if kind in ("pull","delete-local")`) is duplicated at `bin/wsx:442` and `bin/wsx:1077`.
- Fix approach: one helper `direction_of(kind)`, or an `Enum`.

**Hard-coded team-specific defaults:**
- `origin/devtest` is first in `base_ref()` (`bin/wsx:807`), and husky/npm are assumed by `guard_report`/`arm_husky` (`bin/wsx:724,826-849`). The guard reports `INACTIVE` in any repo without husky, so non-husky repos have no supported guard path.

## Security Considerations

**The guard fails open by design:**
- Risk: any exception, misconfiguration or missing hook lets managed paths be committed. `--no-verify` bypasses it, and the first push skips it (see above).
- Files: `bin/wsx:1022-1051`, `bin/wsx:1159-1177`.
- Current mitigation: the per-worktree exclude file keeps paths untracked. `git add -f` still works.
- Recommendations: fix the first-push gap. Consider having the guard alone fail closed on internal errors (still fail open on sync errors), in line with constraint 2's "except the guard".

**`npm install` runs from hooks:**
- Risk: the `session-start` hook (`bin/wsx:1057-1060`) and `wsx new` run `npm install` in the worktree, executing lifecycle scripts from whatever branch is checked out.
- Files: `bin/wsx:815-849`.
- Recommendation: document it. Consider `--ignore-scripts` plus an explicit `npx husky` call.

**Manifest paths are written raw into gitignore syntax [SPECULATIVE]:**
- Files: `bin/wsx:531-534`. Entries containing `*`, `[`, `!`, a leading `#`, or trailing spaces change their meaning as ignore patterns. The manifest parser (`bin/wsx:193-203`) does not reject or escape them.

## Performance Bottlenecks

**Full rehash on every hook:**
- Problem: `plan()` sha256-hashes every managed file on both sides for every `pre-commit`, `pre-push`, `post-checkout`, `post-merge`, `post-rewrite` and session hook. `ensure_installed()` also spawns about 3 git subprocesses and re-renders the exclude file each time.
- Files: `bin/wsx:356-416`, `bin/wsx:573-597`.
- Cause: no mtime/size cache in `base.json`.
- Improvement path: store `(size, mtime_ns, hash)` in base and skip hashing when the stat is unchanged. `cmd_list` multiplies the cost by the number of worktrees (`bin/wsx:958-976`).

## Fragile Areas

**No locking between concurrent runs:**
- Files: `bin/wsx:315-321` (fixed `.wsx-tmp` name), `bin/wsx:219-229`.
- Why fragile: hooks in two worktrees (or an agent `session-end` racing a commit) can write the same source file at the same time. Two runs in one worktree share the `name.wsx-tmp` and `base.json.tmp` names. A crash also leaves `*.wsx-tmp` files that `walk()` does not skip (`bin/wsx:265`), so they get synced as real files.
- Safe modification: add an `fcntl.flock` on `<common_dir>/wsx.lock` around plan+apply, use unique temp names (`tempfile.mkstemp` in the same directory), and skip `*.wsx-tmp` in `walk()`.

**`prune_orphan_excludes()`:**
- Files: `bin/wsx:539-556`. It runs `git rev-parse` with `check=True` inside every live worktree, so one broken worktree makes `wsx setup`/`wsx rm` raise. IDs come from the git-dir basename (`bin/wsx:504-505`), so a linked worktree whose admin dir is named `main` collides with the main worktree's ID.

**Symlink migration paths:**
- Files: `bin/wsx:285-312`, `bin/wsx:376-389`.
- Why fragile: these are the documented landmines and they have zero test coverage.

## Scaling Limits

**Managed tree size:**
- Current capacity: fine for planning docs.
- Limit: large `lww` binary state (multi-GB SQLite) is fully hashed twice per hook, and each `lww` overwrite writes a full backup with no retention (`bin/wsx:333-338`). `.git/.../wsx/backups/` grows without bound.
- Scaling path: a stat cache, plus backup pruning (keep N per path).

## Dependencies at Risk

- Not detected (stdlib only). The external tools assumed are `git` (needs `worktree list --porcelain`, `extensions.worktreeConfig`, `config --worktree`, i.e. git >= 2.20), `npm` and husky. No version check exists.

## Missing Critical Features

- No backup-retention or cleanup command.
- No lock and no recovery for an interrupted apply. `base.json` is written only at the end (`bin/wsx:483-484`), so a crash mid-apply leaves the files written but the base stale. The next run then sees "changed upstream/locally" correctly in most cases, but this has not been verified.

## Test Coverage Gaps

`tests/sandbox.sh` has 28 `check` calls covering setup, push, pull, conflict, resolve, deletion, `pull` policy, `lww` newer-wins, `guard` on the index, and the source-refusal path.

**Untested, high priority:**
- Symlink migration (`via_symlink`, `materialise_parents`, the `replacing symlink` path), `bin/wsx:285-312,376-389`. These are the landmines CLAUDE.md calls load-bearing.
- `cmd_hook` for every hook name, including the fail-open behaviour and the `pre-push` range and first-push case, `bin/wsx:1022-1092`.
- The three reproduced defects above: unreadable file, `lww` local-only file, recreate after a both-sides delete.

**Untested, medium priority:**
- `wsx new`, `wsx rm` (push-before-remove, conflict refusal, `--force`), `wsx list`, `wsx doctor`, `ensure_installed` self-heal, `prune_orphan_excludes`, `global_ignore_file` inheritance.

**Untested, low priority:**
- `skip` carve-outs and nested-policy precedence (`policy_for`, `bin/wsx:274-282`), manifest parse errors (`bin/wsx:184-207`), the `lww` mtime tie.

**Gaps in the suite itself:**
- The suite runs on the host git config, and global hooks run inside it (a gitleaks pre-commit fired during probing). It does not isolate `HOME`/`XDG_CONFIG_HOME`/`GIT_CONFIG_GLOBAL` (`tests/sandbox.sh:21-26`), so `guard_report` and `global_ignore_file` read the developer's real `~/.config`.

---

*Concerns audit: 2026-10-17*
