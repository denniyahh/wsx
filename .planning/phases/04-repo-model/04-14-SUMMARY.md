---
phase: 04-repo-model
plan: 14
subsystem: git
tags: [git-binary, config, gix, rust, tdd]

requires:
  - phase: 04-repo-model
    provides: "04-13: the spawn helper Git and the D-22/D-32 replace-ref guards resolve runs before object reads"
provides:
  - "git::ConfigRead (values, defines_remote) and Repo::read_config: one git config -z --show-scope --show-origin --get-regexp child per call"
  - "parse_config_z: D-01 own-file filter for wsx keys, D-36 local/worktree rule for remotes, fail-closed on truncated output"
  - "Git::for_repo(env, repo); Git::from_process now calls it"
  - "git::Error::GitBadOutput and its message"
  - "testing::scratch_env_of for in-process tests that hand wsx's Git the fixture's environment"
  - "model::resolve(repo, git, recorded) reading config once; single/remote/default_remote/worktree_dir/build/integration take &ConfigRead"
affects: [04-15, 04-16, 04-17, 04-18, phase-05]

actuals:
  tokens: 14649
  tasks: 2
  commits: 3
plan_head_before: d8b67135f7b29c29dfc5952830268e0021824002
plan_head_after: cbb182f417ce9ee85e413ddd923ab5652b0bfc60

tech-stack:
  added: []
  patterns:
    - "Config reads through the git binary, parsed as -z bytes, filtered by scope and canonical origin"
    - "In-process model tests build wsx's Git with testing::scratch_env_of so its children see the fixture's hermetic environment"

key-files:
  created: []
  modified:
    - src/git/config.rs
    - src/git/spawn.rs
    - src/git/mod.rs
    - src/git/testing.rs
    - src/model/mod.rs

key-decisions:
  - "parse_config_z keeps only two-part wsx keys, so the ported subsection_key_is_not_read keeps git's answer for wsx.sub.sourceBranch as its control and asserts the read does not take it for wsx.sourceBranch"
  - "count_child prints only count; the gix all-sources marker gave way to the git-side assertions its parents already made"
  - "keyless_local_remote_section_is_not_a_remote gets a positive control: after git remote add gives the section a key, read_config defines it"

patterns-established:
  - "A git read that exits 1 with empty stderr for no match is matched on GitFailed { status: Some(1) } and read as empty; any other failure propagates"

requirements-completed: [CONF-04, CONF-05, CONF-07]

coverage:
  - id: D1
    description: "parse_config_z keeps wsx keys only from the own file at scope local, and counts remotes from local and worktree records of any origin"
    requirement: CONF-04
    verification:
      - kind: unit
        ref: "src/git/config.rs#git::config::tests::parse_config_z_keeps_only_the_own_local_file"
        status: pass
      - kind: unit
        ref: "src/git/config.rs#git::config::tests::parse_config_z_names_a_remote_up_to_the_last_dot"
        status: pass
      - kind: unit
        ref: "src/git/config.rs#git::config::tests::parse_config_z_bare_key_is_one_empty_value"
        status: pass
    human_judgment: false
  - id: D2
    description: "Malformed or truncated config output fails closed: exit 1 with empty stderr is no keys, exit 128 is GitFailed, a partial record is GitBadOutput"
    requirement: CONF-07
    verification:
      - kind: unit
        ref: "src/git/config.rs#git::config::tests::parse_config_z_refuses_a_truncated_record"
        status: pass
      - kind: unit
        ref: "src/git/config.rs#git::config::tests::config_read_with_no_keys_is_empty"
        status: pass
      - kind: unit
        ref: "src/git/config.rs#git::config::tests::config_read_of_a_malformed_file_is_git_failed"
        status: pass
      - kind: unit
        ref: "src/git/mod.rs#git::tests::bad_output_message_names_the_query_and_the_problem"
        status: pass
    human_judgment: false
  - id: D3
    description: "resolve reads keys and remotes from one git child; the S5 shape (symlinked HEAD, includeIf.onbranch remote) gives git's answer, with a control"
    requirement: CONF-05
    verification:
      - kind: unit
        ref: "src/model/mod.rs#model::tests::onbranch_remote_under_a_symlinked_head_counts_as_git_does"
        status: pass
      - kind: other
        ref: "scripts/check.sh all (target/gsd/04-14-all.log ends ==> check.sh: all OK)"
        status: pass
    human_judgment: false
  - id: D4
    description: "gix's local_config_values and remote_configured are deleted; their 20 tests run on read_config under their names, plus the G4 odd-path case"
    requirement: CONF-04
    verification:
      - kind: unit
        ref: "src/git/config.rs#git::config::tests (20 ported tests and config_read_from_a_path_with_a_space_and_a_non_utf8_byte, each run alone)"
        status: pass
      - kind: other
        ref: "rg for pub fn / .call of both names under src: absent; control: both defined at 4234f31"
        status: pass
    human_judgment: false

duration: 12min
completed: 2026-11-17
status: complete
---

# Phase 4 Plan 14: Config read through git Summary

**The model's `wsx.*` keys and remote list now come from one `git config -z --show-scope --show-origin --get-regexp '^(wsx|remote)\.'` child, parsed as bytes with D-01's own-file filter and D-36's remote rule; gix's two config reads are gone.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-11-17T22:27:28Z
- **Completed:** 2026-11-17T22:39:28Z
- **Tasks:** 2
- **Files modified:** 5

## Accomplishments

- `Repo::read_config(&Git)` returns a `ConfigRead`; `resolve(repo, git, recorded)` calls it once and passes it to `single`, `remote`, `default_remote`, `worktree_dir`, `build` and `integration`.
- The S5 shape now resolves as git does: under a `HEAD` symlinked to `refs/heads/main`, the `includeIf.onbranch:main` remote `origin` is the base remote; with `HEAD` aimed at `workspace/dev`, git lists no remote and the model has none.
- Exit 1 with empty stderr is an empty read; exit 128 (a `[bad` line) stays `GitFailed`; a truncated record is `GitBadOutput`.
- `Repo::local_config_values` and `Repo::remote_configured` deleted, their 20 tests ported with git-side controls, plus the space and non-UTF-8 repository path case.

## Task Commits

1. **Task 1 RED: failing tests for the config read through git** - `6657a6d` (test)
2. **Task 1 GREEN: read the model's config through git** - `f75ade7` (feat)
3. **Task 2: delete gix's config reads and run their tests on the git read** - `cbb182f` (refactor)

## TDD Gate Compliance

- **RED** (`6657a6d`): `target/gsd/04-14-red-evidence.json`, `check tdd-red-evidence --raw --pick verdict` printed `RED_EVIDENCE_OK` (reason `target_test_failed`, JUnit, exit 100). Semantic assessment: `onbranch_remote_under_a_symlinked_head_counts_as_git_does` ran, git's assertions (`symbolic-ref HEAD` = `refs/heads/main`, `git remote` = `origin`) passed, and it failed at `configured(&work)` with `resolve: expected RepoModel::Configured, got Err(NoIntegrationBranch { remote: None, ... })`. `Repo::discover` succeeded, so the premise held. Not a build, load or fixture failure.
- RED logs under `target/gsd/04-14-red/`, each ending `exit 100`:
  - `parse_config_z_keeps_only_the_own_local_file`: `assertion \`left == right\` failed` (values empty, expected `[dev]`)
  - `parse_config_z_bare_key_is_one_empty_value`: `assertion \`left == right\` failed: wsx.sourcebranch`
  - `parse_config_z_names_a_remote_up_to_the_last_dot`: `assertion failed: read.defines_remote(b"a.b")`
  - `parse_config_z_refuses_a_truncated_record`: `expected Err for a truncated record, got Ok(ConfigRead { values: [], remotes: {} })`
  - `config_read_of_a_malformed_file_is_git_failed`: `expected Err(GitFailed) with status 128, got Ok(ConfigRead { values: [], remotes: {} })`
  - `config_read_with_no_keys_is_empty`: its first half passed, its control failed: `control: git lists remote.origin.url but read_config defines no origin`
- Passed at RED, as planned: `git::testing::scratch_env_of_a_fixture_path_is_its_scratch_env` and `git::tests::bad_output_message_names_the_query_and_the_problem` (each exit 0).
- **GREEN** (`f75ade7`, descends from the RED commit): all 25 tests of Task 1's second verify pass, each run alone; `scripts/check.sh all` ends `==> check.sh: all OK`.
- **REFACTOR** (`cbb182f`): Task 2; `scripts/check.sh all` ends `==> check.sh: all OK`.
- Tracer gate (interactive, end-of-phase, automated-only verify): re-ran both Task 1 verify commands after the GREEN commit; both exited 0.

## Files Created/Modified

- `src/git/config.rs` - `ConfigRead`, `Repo::read_config`, `parse_config_z`, `remote_name`, `is_own_file`; the gix methods removed; the ported and new tests
- `src/git/spawn.rs` - `Git::for_repo`; `from_process` calls it
- `src/git/mod.rs` - `pub use config::ConfigRead`, `pub use spawn::Git`, `Error::GitBadOutput` and its message test
- `src/git/testing.rs` - `scratch_env_of` and its test
- `src/model/mod.rs` - `resolve` takes `&Git` and reads config once; `git_for` test helper; the S5 test

## Decisions Made

- `subsection_key_is_not_read`: the parser keeps only two-part `wsx` keys, as the plan specifies, so the old reader-side assertion `local_values("wsx.sub.sourceBranch") == [x]` cannot hold. The test keeps git's answer for the subsection key (`x`) as its control and asserts the read returns nothing for `wsx.sourceBranch`.
- `count_child` prints only `count`. Its `all` marker counted gix's values from every source; the parents already assert git's own answer (`git config --get` under the same environment), which is the git-side control the plan asks for.
- `keyless_local_remote_section_is_not_a_remote`: `git config --show-scope --get-regexp` prints nothing for a keyless section, so there is no git-side line showing it exists. The control is positive instead: after `git remote add empty /y` gives the section a key, `read_config` defines `empty`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] The RED verify's `--raw | rg -qx 'RED_EVIDENCE_OK'` cannot match**
- **Found during:** Task 1 RED verify (known from 04-13, named in the orchestrator note)
- **Issue:** `--raw` prints the whole JSON result, so the plan's form exits 1 on a valid record (checked: `plan-form rc=1`).
- **Fix:** ran the same command with `--raw --pick verdict`; the rest unchanged. Negative control: `chk` with the wrong intended text on `parse_config_z_keeps_only_the_own_local_file` exits 1 with `failed without the intended text`.
- **Files modified:** none (the plan file was not edited)

**2. [Rule 3 - Blocking] `Git::for_repo` added in the RED commit**
- **Found during:** Task 1 RED
- **Issue:** the new config tests build their helper with `Git::for_repo`, which the plan adds at GREEN; without it the RED commit does not compile.
- **Fix:** `for_repo` landed in `6657a6d`; `from_process` was rewired to it at GREEN, as planned.
- **Committed in:** 6657a6d

**3. [Rule 1 - Bug] clippy `chunks_exact_to_as_chunks` on the record split**
- **Found during:** Task 1 GREEN gate
- **Fix:** `fields.as_chunks::<3>()`, whose remainder also gives the partial-record check.
- **Committed in:** f75ade7

---

**Total deviations:** 3 auto-fixed (2 blocking, 1 bug). **Impact:** none on behaviour; the first is the plan-text defect 04-13 found, still present in this plan.

## Issues Encountered

None.

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Ready for 04-15. Every caller of `resolve` now passes a `Git`; in-process tests use `git_for`, re-executed children `Git::from_process`.
- Each rule is pinned on one fixture on git 2.56.1 (host). `GIT_CONFIG` in the environment, which makes this child read only that file, is open until 04-17.

---
*Phase: 04-repo-model*
*Completed: 2026-11-17*

## Self-Check: PASSED
