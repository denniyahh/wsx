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
