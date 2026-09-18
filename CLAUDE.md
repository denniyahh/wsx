# CLAUDE.md

Agent guidance for the **wsx** repository.

`README.md` explains what the tool does and how to use it; `CONTRIBUTING.md`
holds the design constraints and the test workflow. Read both before changing
`bin/wsx` — most of what looks like an odd implementation choice is load-bearing
and documented there.

## Shape of the code

`bin/wsx` is one Python file, standard library only, in dependency order:

| Section | Contents |
|---|---|
| git helpers | `git()`, `git_ok()`, `list_worktrees()` |
| `Context` | resolves roots, source worktree, manifest, base snapshot |
| file plumbing | `digest()`, `walk()`, `copy_file()`, symlink handling |
| planning | `plan()` produces `Action`s; `apply()` executes them |
| ignore + guard | per-worktree `core.excludesFile`, `ensure_installed()` |
| commands | one `cmd_*` per subcommand |

Sync decisions live **entirely** in `plan()`. `apply()` only executes what it is
given, which is what makes `--dry-run` trustworthy: both paths run the same
planner. Put new decision logic in `plan()`, never in `apply()`.

## Landmines

- **`via_symlink()` is not redundant with `Path.is_symlink()`.** A file inside a
  symlinked directory resolves to the source, so hashing both sides compares a
  file with itself and reports a false "converged". Migration away from a
  symlinked layout depends on catching exactly that.
- **`materialise_parents()` must run before any pull write.** Otherwise the
  write follows a symlinked ancestor back into the source worktree.
- **The base snapshot is what makes conflicts detectable.** Comparing only the
  two live copies cannot distinguish "they changed it" from "I changed it".
  Keep `base.json` updated on every applied action.
- **`cmd_hook` must not raise.** It catches everything and returns 0, except for
  the guard. See constraint 2 in `CONTRIBUTING.md`.
- **Never write to tracked files in a managed repo.** Not `.gitignore`, not
  `.husky/`. Per-worktree git config and generated exclude files only.

## Testing

`./tests/sandbox.sh` is the suite — 28 assertions against a throwaway repo, safe
to run any time. Run it after every change to `bin/wsx`; add a case for any bug
you fix. `python3 -m py_compile bin/wsx` catches syntax errors quickly.

Ignore-rule, guard and hook changes also need a manual pass against a real
repository (`wsx doctor`, `wsx new`, `wsx rm`) — the sandbox only partly
reproduces git's worktree config behaviour.

## Do not

- Add a third-party dependency.
- Make the tool interactive; it runs inside hooks.
- Resolve a conflict on the user's behalf.
