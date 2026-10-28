# Memory Recall (MemPalace)

_Wing: wsx · Mode: augment · Transport: cli_

The `wsx` wing holds no drawers yet (`mempalace wake-up --wing wsx`: "No memories yet"; a targeted
search returned nothing). A search across all wings works and returns devflow drawers, so the
palace itself is reachable. The items below come from devflow, whose scripts are this project's
oracle and which already shipped a scrubbed-environment git helper.

## Prior decisions
- devflow phase 27 scrubbed `GIT_DIR` from every git child it spawned through one constructor,
  with "no escape hatch, mechanism-only proof". Source: devflow/milestones, `27-02-SUMMARY.md`
  (key-decisions D-01/D-03).

## Patterns
- **A test that proves the scrub has two halves.** (a) A structural assertion that the `Command`
  the helper builds marks `GIT_DIR` for removal. (b) A real spawn of the unmodified code path,
  asserting the correct answer. Source: devflow/milestones, `27-01-SUMMARY.md`.
- **A hostile-`GIT_DIR` regression test.** It runs the test with `GIT_DIR=<hostile>/.git` and
  asserts the helper still resolves the caller's own root (`worktree::tests::list_resolves_caller_root_under_a_hostile_git_dir`).
  Source: devflow/milestones, `27-02-SUMMARY.md`.

## Surprises / gotchas
- **Whether `GIT_DIR` redirects a command depends on the command.** Verified on git 2.55.0:
  `git worktree list --porcelain` and `git merge-base --is-ancestor` are redirected by an explicit
  `GIT_DIR` re-injected after the scrub, while `git rev-parse --show-toplevel` falls back to the
  cwd when `GIT_WORK_TREE` is unset. A leak control built on `--show-toplevel` can therefore pass
  while the leak is real. Pick a control command that git actually redirects (03-RESEARCH.md uses
  `git -C wt2 rev-parse --absolute-git-dir` and `--git-path index` inside wt1's pre-commit; both
  were observed leaking wt1's values). Source: devflow/milestones,
  `27-01-SUMMARY.md` and `27-02-SUMMARY.md`.
