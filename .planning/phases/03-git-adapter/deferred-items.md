# Phase 03 deferred items

Out-of-scope discoveries logged during execution. Not fixed by the plan that found them.

## The test suite fails when run with `GIT_DIR` in the test process environment

- **Found during:** 03-04, while stating the limits of the gix ban-list pin test.
- **Observed:** `GIT_DIR=<this checkout's git dir> scripts/check.sh test` gives
  `207 tests run: 190 passed, 17 failed, 42 skipped` (rc 100). Without `GIT_DIR`, all 207 pass.
- **Cause:** `Repo::discover` and the `*_with_environment_overrides*` gix entry points honour the
  process `GIT_DIR` by design (that is the hook-aware open), and the test processes inherit the
  caller's environment. A test that discovers from a scratch path then opens the caller's
  repository instead.
- **Effect:** a false red, never a false green: running the suite from inside a git hook, or
  from any shell that exports `GIT_DIR`, fails tests that are correct.
- **Possible fix (owner's call):** `scripts/check.sh` unsets the repository-local git variables
  (`GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE`, `GIT_COMMON_DIR`, ...) before invoking nextest,
  or tests that need the process environment assert it is clean first.
- **Recount at 03-07 (2026-10-27):** with `GIT_DIR` and `GIT_INDEX_FILE` set to this checkout's,
  `scripts/check.sh test` gives `236 tests run: 212 passed, 24 failed, 44 skipped`: 15 in
  `git::reads::tests` and 9 in `git::repo::tests`, six of them 03-06's reftable tests. No
  `git::spawn` or `git::testing` test fails. 03-07 changed neither `reads.rs` nor `repo.rs`;
  its re-executed children start from `env_clear()`, so they do not see the caller's `GIT_DIR`.
- **RESOLVED in 03-10 (2026-10-27):** `testing::rerun_scrubbed` re-executes each of the 26 tests
  that discover in the test process (16 in `git::reads::tests`, 10 in `git::repo::tests`)
  without the variables `git rev-parse --local-env-vars` lists, whenever the runner has any.
  Production discovery is unchanged and still honours `GIT_DIR`. With this checkout's `GIT_DIR`
  and `GIT_INDEX_FILE` exported, `scripts/check.sh test` went from
  `253 tests run: 229 passed, 24 failed, 50 skipped` (control, at `31697de`) to
  `256 tests run: 256 passed, 52 skipped` (at `142a095`).
  `git::testing::discovering_tests_pass_with_a_runner_git_dir` pins the property: it re-runs the
  two suites with a decoy repository's `GIT_DIR` and `GIT_INDEX_FILE`. Dropping the guard from
  one test makes it fail with `(passed, failed) = (38, 1)`. The fix lives in the tests, not in
  `scripts/check.sh`, so a direct `cargo nextest run` is covered too.

## In-process discovery tests still read the runner's global and system git config

- **Found during:** 03-10, while fixing the item above.
- **Observed:** `rerun_scrubbed` removes only the repository-local variables. The 26 tests that
  discover in the test process still inherit `HOME`, `XDG_CONFIG_HOME`, `GIT_CONFIG_GLOBAL`,
  `GIT_CONFIG_SYSTEM`, `GIT_CONFIG_NOSYSTEM` and `GIT_CEILING_DIRECTORIES` from the runner, and
  gix reads that global and system config when it opens a repository.
- **Effect:** not measured. No run with a changed host config was made, so no failing test is
  known. One case is inferred, not run: before 03-10 a host-global `extensions.refStorage`
  would have failed the files-repository tests, as the 03-10 RED run showed for a child's
  global config. 03-10 made wsx ignore that key, as git does. The
  property the owner's rule asks for (configuration paths aimed at scratch) still does not hold
  for these 26 tests; it does hold for every re-executed child and every git child.
- **Possible fix (owner's call):** run those tests in a child with the scratch environment, as
  the ceiling and foreign-owner tests do, or have `rerun_scrubbed` always re-execute with a
  scratch `HOME` and `GIT_CONFIG_NOSYSTEM=1`.
