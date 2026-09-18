# Contributing to wsx

## Design constraints

These are the rules the tool is built around. Breaking one is a bug, even if the
tests still pass.

1. **Never lose a user's edit.** When both sides changed, stop and report.
   Overwriting is only ever acceptable under an explicitly chosen policy
   (`pull`, `lww`), and `lww` must keep a backup.
2. **Fail open everywhere except the guard.** A sync problem must never block a
   commit. The *only* non-zero exit from `wsx hook` is the guard refusing
   managed paths.
3. **Never consume stdin in hook paths.** `~/.config/husky/init.sh` is
   *sourced*, sharing stdin with the hook that follows it, and `pre-push` reads
   its refs from stdin. Anything that swallows it silently breaks pushes.
4. **Touch nothing the team owns.** No writes to tracked files — `.gitignore`,
   `.husky/`, committed config. Personal tooling stays personal.
5. **Idempotent.** Every command must be safe to run twice.
6. **Standard library only.** `bin/wsx` is a single Python file with no
   third-party imports, so it runs anywhere Python does.

## Development

The install is symlinked by default, so an edit to `bin/wsx` is live
immediately — there is no build or reinstall step.

```sh
./tests/sandbox.sh          # full end-to-end suite
python3 -m py_compile bin/wsx
```

`tests/sandbox.sh` builds a throwaway repository under `$(mktemp -d)` and cleans
up after itself. It never touches a real worktree, so it is safe to run at any
time.

### Adding a test

Each assertion uses `check <description> <expected> <actual>`. Group related
assertions under an `echo "== n. topic =="` heading and keep them ordered so
later cases can build on earlier state.

### Manual verification

Anything touching ignore rules, the guard or hooks should also be exercised
against a real repository, because those paths depend on git behaviour the
sandbox only partly reproduces:

```sh
wsx doctor          # in a dev worktree: expect 0 problems
wsx new test/scratch
wsx rm test/scratch
```

## Style

- Comments explain *why*, never *what*. If a line needs a "what" comment, it
  probably needs a better name instead.
- Every non-obvious constraint gets a comment naming the failure it prevents —
  the stdin redirect and the symlink-parent materialisation are the model here.
- User-facing output is aligned, coloured only when stdout is a TTY, and says
  what to do next when it reports a problem.

## Commits

Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `chore:`).
The body should explain the failure mode being fixed, not restate the diff.
