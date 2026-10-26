---
phase: 02-oracle-harness
plan: 09
subsystem: testing
tags: [rust, nextest, oracle, harness, git, hermeticity, hooks]

requires:
  - phase: 02-oracle-harness (02-02)
    provides: hermetic_env, ScenarioDirs, command, run_with_env, run_case
  - phase: 02-oracle-harness (02-07)
    provides: cut::self_test and guard::self_test with their oracle mappings
  - phase: 02-oracle-harness (02-01, 02-04)
    provides: the vendored oracle pre-push hook (tests/oracle/hooks/pre-push)
provides:
  - PlantedHost and plant_host, a host plant covering global, XDG and system config, core.hooksPath, config hooks, init.templateDir, GIT_CONFIG_PARAMETERS and a decoy GIT_DIR
  - hermeticity::planted_host_config_never_reaches_scenarios and its wired control
  - Env builder edge tests (LLVM_ prefix adjacency, override and duplicate keys, empty inheritance)
  - The oracle pre-push unset trap pinned against the vendored statement, with a control
  - git_version_at_least and the hermetic git version check
affects: [02-10, 02-11, 02-12, 02-13, 02-14, 02-15, phase-6, phase-7, phase-9]

actuals:
  tokens: 5800
  tasks: 2
  commits: 3
plan_head_before: 2f1fd6a2b91aea1d4f6eeaabdad8227c1d69b255
plan_head_after: b9aa3479581091b7de7413b49f6e34868ef3bd10

tech-stack:
  added: []
  patterns:
    - "A planted host is a ScenarioDirs tree of its own; the proof feeds its env to hermetic_env through run_with_env and never mutates the test process"
    - "Planted programs (hooks, the signing program) are recorder scripts that append a line to one marker file, so the control can say which of them ran"
    - "A test that needs a host program to fail plants a recording stand-in rather than running the real one, so no daemon is started"

key-files:
  created: []
  modified:
    - tests/harness/hermeticity.rs

key-decisions:
  - "The planted config sets gpg.program to a recording script that fails. The plan's real gpg with a missing key started a keyboxd daemon under the scenario HOME that outlived the test and its TempDir"
  - "plant_host takes a name and creates its own harness-hermeticity-<name>- ScenarioDirs. The plan's plant_host(dir) left the directory's owner open"
  - "HARNESS_KEYS, the hermetic PATH and MIN_GIT are restated as literals in hermeticity.rs instead of exported from env.rs. The plan edits only hermeticity.rs, and an independent literal pins the value rather than echoing it"

patterns-established:
  - "Each hermeticity proof has a separate control test that must produce the opposite result, with a comment naming what the control rules out"

requirements-completed: [HARN-02]

coverage:
  - id: D1
    description: "Under a planted host, sync::self_test, cut::self_test and guard::self_test (subject and both controls) reproduce their explicit expected outcomes on the oracle side. No planted hook runs, the decoy repository's refs and config are unchanged, and the built env keeps only LLVM_PROFILE_FILE from the plant"
    requirement: HARN-02
    verification:
      - kind: integration
        ref: "tests/harness/hermeticity.rs#hermeticity::planted_host_config_never_reaches_scenarios"
        status: pass
    human_judgment: false
  - id: D2
    description: "With only GIT_CONFIG_GLOBAL wired to the planted file, the planted hooks and signing program run and sync::self_test's fixture commit fails"
    requirement: HARN-02
    verification:
      - kind: integration
        ref: "tests/harness/hermeticity.rs#hermeticity::wired_to_planted_config_fires_hook"
        status: pass
    human_judgment: false
  - id: D3
    description: "Only LLVM_-prefixed keys pass through (LLVM, LLVMX_A and XLLVM_B are dropped). A planted HOME, PATH or GIT_CONFIG_GLOBAL never overrides the harness value, no key appears twice, and an empty inheritance gives exactly the harness table"
    requirement: HARN-02
    verification:
      - kind: unit
        ref: "tests/harness/hermeticity.rs#hermeticity::env_passes_through_only_llvm_keys"
        status: pass
      - kind: unit
        ref: "tests/harness/hermeticity.rs#hermeticity::env_from_an_empty_inheritance_is_complete"
        status: pass
    human_judgment: false
  - id: D4
    description: "The oracle pre-push hook's own unset statement leaves git's config listing and origins unchanged under the harness env, and GIT_CONFIG_NOSYSTEM=1 survives it. With GIT_CONFIG_GLOBAL elsewhere, the same unset visibly switches the source"
    requirement: HARN-02
    verification:
      - kind: integration
        ref: "tests/harness/hermeticity.rs#hermeticity::pre_push_unset_leaves_config_sources_unchanged"
        status: pass
      - kind: integration
        ref: "tests/harness/hermeticity.rs#hermeticity::pre_push_unset_switches_source_when_global_is_elsewhere"
        status: pass
    human_judgment: false
  - id: D5
    description: "The git the hermetic PATH resolves is at least 2.54, and the version check rejects 2.53.0"
    requirement: HARN-02
    verification:
      - kind: integration
        ref: "tests/harness/hermeticity.rs#hermeticity::hermetic_git_is_at_least_2_54"
        status: pass
      - kind: unit
        ref: "tests/harness/hermeticity.rs#hermeticity::version_check_rejects_an_older_git"
        status: pass
    human_judgment: false

duration: 9min
completed: 2026-10-26
status: complete
---

# Phase 2 Plan 09: Hermeticity Proof Summary

**A planted host (global, XDG and system config, core.hooksPath, config hooks, init.templateDir, GIT_CONFIG_PARAMETERS, a decoy GIT_DIR) leaves three oracle scenarios' outcomes unchanged and fires nothing, while a control wired to the same plant fires its hooks. The env builder's edges, the oracle pre-push hook's unset trap and the hermetic git version are pinned, each with a control that must fail.**

## Performance

- **Duration:** about 9 min
- **Started:** 2026-10-26T18:38:51Z
- **Completed:** 2026-10-26T18:47:15Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- `planted_host_config_never_reaches_scenarios`: these three scenarios passed on the oracle side under the plant, printed by the test:
  - `sync::self_test reproduced its expected outcome under the plant`
  - `cut::self_test reproduced its expected outcome under the plant`
  - `guard::self_test reproduced its expected outcome under the plant`
  - After them, the marker file did not exist. The decoy's `show-ref --head` and `.git/config` bytes equalled the recorded ones.
- `wired_to_planted_config_fires_hook`: the control's marker contents, from `--success-output immediate`, after the final commit:
  ```
  hookspath:reference-transaction
  hookspath:reference-transaction
  hookspath:reference-transaction
  hookspath:reference-transaction
  hookspath:reference-transaction
  hookspath:reference-transaction
  config-hook
  hookspath:pre-commit
  hookspath:commit-msg
  gpg:sign
  ```
  The run failed in the fixture preset: `git commit ... exited Some(128); stderr: error: gpg failed to sign the data: (no gpg output) fatal: failed to write commit object`.
- The six Task 2 tests pass. The hermetic git is `git version 2.56.0`, which is `/usr/bin/git` through `/usr/local/bin:/usr/bin:/bin`.
- `scripts/check.sh all` exited 0 with `==> check.sh: all OK` after each commit: 110 tests after Task 1, then 116 after Task 2 and after the fix, 8 skipped each time.

## Task Commits

1. **Task 1: Planted host never reaches a scenario; the wired control does** - `91f1da6` (test)
2. **Task 2: Env builder edges, the oracle hook's unset trap, and the hermetic git version** - `0037b12` (test)
3. **Fix: route planted signing through a recording gpg** - `b9aa347` (fix, deviation 1)

## Negative Controls Run (throwaway, not committed)

Each mutation was applied to `tests/harness/env.rs` or `hermeticity.rs` in the working tree, run through nextest, and reverted. `git status` was clean afterwards.

| Mutation | Test | Result |
|---|---|---|
| `GIT_CONFIG_PARAMETERS` passes through | planted proof | failed: `the planted "GIT_CONFIG_PARAMETERS" reached the env` |
| `GIT_DIR`, `GIT_WORK_TREE` pass through | planted proof | failed: `the planted "GIT_DIR" reached the env` |
| The proof's env-value check disabled, plus `GIT_CONFIG_PARAMETERS` passes through | planted proof | failed on `a planted hook ran: .../marker` |
| The env-value check disabled, plus `GIT_DIR`/`GIT_WORK_TREE` pass through | planted proof | failed in `sync::self_test`: `fatal: GIT_WORK_TREE (or --work-tree=<directory>) not allowed with` (`git init --bare`) |
| The env-value check disabled, plus `GIT_CONFIG_GLOBAL` passes through | planted proof | failed in `sync::self_test`: gpg signing failed |
| The env-value check disabled, plus `XDG_CONFIG_HOME` passes through | planted proof | failed in `sync::self_test`: a fixture commit failed |
| The env-value check disabled, plus `SSH_AUTH_SOCK` passes through | planted proof | failed on `SSH_AUTH_SOCK reached the env` |
| The env-value check disabled, plus `GIT_CONFIG_SYSTEM` passes through | planted proof | **passed**: `GIT_CONFIG_NOSYSTEM=1` keeps git from reading it, so it has no effect |
| The env-value check disabled, plus `HOME` passes through | planted proof | **passed**: see limits |
| `starts_with(b"LLVM")` (no underscore) | `env_passes_through_only_llvm_keys` | failed: `LLVM` and `LLVMX_A` in the key set |
| `HOME` passes through | `env_passes_through_only_llvm_keys` | failed: `HOME` listed twice |
| `GIT_CONFIG_NOSYSTEM` removed from the builder | `env_from_an_empty_inheritance_is_complete` | failed: key table mismatch |
| `GIT_CONFIG_NOSYSTEM` removed from the builder | `pre_push_unset_leaves_config_sources_unchanged` | failed: after the unset the listing gained `file:/etc/gitconfig filter.lfs.*` from this host |
| `MIN_GIT = (2, 57)` | `hermetic_git_is_at_least_2_54` | failed: `the hermetic git is older than (2, 57): "git version 2.56.0\n"` |

## What These Checks Do Not Establish

- **Other config sources.** The plant covers the documented sources: system, global, XDG, `GIT_CONFIG_PARAMETERS`, `init.templateDir`, `core.hooksPath` and config hooks. That does not prove git has no other source.
- **The host's `/etc/gitconfig` was not planted** (no root). The system half of the unset trap rests on `GIT_CONFIG_NOSYSTEM=1` surviving the unset and on the origin listing. The NOSYSTEM mutation above shows this host's `/etc/gitconfig` is read without it, so the test bites on this host. On a host with an empty `/etc/gitconfig`, the same mutation would still fail the key-table test but not the listing comparison.
- **The env-value check catches every leak, but the behavioural checks miss some.** The marker, decoy and outcome checks did not catch a leaked `HOME` in these three scenarios. With `HOME` leaked, `$HOME/.gitconfig` is read only after the pre-push hook's unset, and the hook's git commands are read-only, so no planted hook fires and no outcome changes. A leaked `GIT_CONFIG_SYSTEM` is masked by `GIT_CONFIG_NOSYSTEM`. The decoy assertion was never shown to fail on its own: a leaked `GIT_DIR` broke the scenario first.
- **`core.quotePath=false` is planted but not observable here.** None of the three scenarios has a non-ASCII path, so a leak of that key alone would not change their outcomes.
- **One machine, single runs.** Each test ran a handful of times on this host (git 2.56.0, GnuPG 2.4.9), not in the container image. That is not a reliability bound.
- **The prohibition (no real global or system config, SSH agent, network or foreign repository).** Config and the SSH agent are covered by the tests above. Network (`GIT_ALLOW_PROTOCOL=file`) and the ceiling at the scenario root come from 02-02 and are not re-tested here.

## Files Created/Modified

- `tests/harness/hermeticity.rs`: `PlantedHost`, `plant_host`, `DecoyState`, `git_version_at_least`, and the eight tests.

## Decisions Made

See `key-decisions` in the frontmatter.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The wired control left a gpg keyboxd daemon running**
- **Found during:** cleanup check after Task 2
- **Issue:** the plant's `commit.gpgsign=true` made the wired control's fixture commit run the host's real `gpg` under the scenario HOME. GnuPG 2.4.9 started `keyboxd --homedir .../target/tmp/harness-sync-self_test-OUMUVC/home/.gnupg --daemon` (PID 3552803). It was still running 229 s later, after its TempDir had been removed.
- **Fix:** the planted config sets `gpg.program = <plant>/bin/gpg`, a script that appends `gpg:sign` to the marker and exits 1. The commit still fails on signing, and no gpg process starts. The control now also asserts the `gpg:sign` line. The daemon was stopped by its PID; it was this run's own, since its homedir named this test's TempDir.
- **Files modified:** `tests/harness/hermeticity.rs`
- **Verification:** after the fix, `ps` showed no `keyboxd` or `gpg-agent` process after the wired control, two full gate runs and the mutation runs. The stderr reads `(no gpg output)`.
- **Committed in:** `b9aa347`

**2. [Additive] Checks beyond the plan's list**
- The decoy's `.git/config` bytes are compared as well as its refs. A retargeted git can flip `core.bare`, which the pre-push hook's comment cites as a past incident.
- `SSH_AUTH_SOCK` is asserted absent from the built env (T-02-24).
- The version control also checks `2.100.1` (a numeric, not lexical, compare), `3.0.0` and two non-version strings.

---

**Total deviations:** 1 auto-fixed bug, plus additive checks and the `plant_host(name)` signature noted in key-decisions.
**Impact on plan:** every planned test name exists and passes. Only `tests/harness/hermeticity.rs` changed.

## Issues Encountered

- `rg -q 'init.templateDir'` failed on the first Task 1 draft, because the config is written in section form (`[init] templateDir`). The `planted_config` doc comment now names the key `init.templateDir`.

## Acceptance Checks

- `rg -q` found `fn planted_host_config_never_reaches_scenarios`, `fn wired_to_planted_config_fires_hook`, `GIT_CONFIG_PARAMETERS`, `init.templateDir`, `local-env-vars` and `fn git_version_at_least`: each exited 0.
- `git diff --name-only $(git merge-base HEAD workspace/denniyahh) HEAD -- tests/oracle` lists only the eight vendored files. `git log` over that range and path shows only `2cfc2f2` (02-01) and `fd18911` (02-04).
- Each of the eight tests passed alone through `scripts/check.sh test -E 'test(=hermeticity::<name>)'`.

## User Setup Required

None.

## Next Phase Readiness

- HARN-02 now has its planted-host proof. Later scenarios with non-ASCII paths could join `PROVEN_SCENARIOS`, which would make the planted `core.quotePath=false` observable.
- Cleanup: no `target/tmp/harness-*` tree remains, and no `keyboxd` or `gpg-agent` process runs. The scratch directory `/tmp/claude-1000/-var-home-denniyahh-Github-wsx/97fae308-abb4-46c4-af94-89ae86614ec1/scratchpad/exec-02-09/` (logs and file backups) is removed after the final commit.

---
*Phase: 02-oracle-harness*
*Completed: 2026-10-26*

## Self-Check: PASSED

- `tests/harness/hermeticity.rs` exists and is changed in `2f1fd6a..b9aa347`.
- Commits `91f1da6`, `0037b12` and `b9aa347` are ancestors of HEAD. The count measured from the plan ledger is 3.
