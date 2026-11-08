---
phase: 04-repo-model
plan: 05
subsystem: repo-model
tags: [rust, gix, config, refs, namespace, tdd]

requires:
  - phase: 04-01
    provides: "Repo::local_config_values (narrow, Source::Local only), model::single (last value), the full model::Error interface with KeyRepeated and KeyEmpty, repo_with_remote fixture, DISCOVERING_SUITES with model::tests::"
provides:
  - "Repo::local_config_values restricted to Source::Local at include level 0 (D-01, A1)"
  - "model::single returning KeyRepeated / KeyEmpty, per key read (D-18, A4)"
  - "git::Error::RefsNamespace and private refuse_refs_namespace, called in Repo::discover"
  - "DISCOVERING_SUITES at five suites, git::config::tests:: first"
  - "child halves git::config::tests::count_child and model::tests::state_child; test helper namespaced_repo"
affects: [04-06, 04-07, 04-09, 04-10, phase-5-check, phase-6-guard]

actuals:
  tokens: 11284
  tasks: 3
  commits: 6
plan_head_before: 3039f9e4232ba4da062195082ce76b91b7a9534a
plan_head_after: d15bbdb52429ab53795282666dd14f2b74115d53

tech-stack:
  added: []
  patterns:
    - "config read: strings_filter with source == Local && level == 0, never string_filter"
    - "a scope test pairs gix's filtered answer with a control that gix did load the value (values_from in-process, the child's all= marker)"
    - "a gix setting git's reads ignore is refused at Repo::discover from gix's resolved value, not from the variables that set it"

key-files:
  created: []
  modified:
    - src/git/config.rs
    - src/git/testing.rs
    - src/model/mod.rs
    - src/git/repo.rs
    - src/git/mod.rs

key-decisions:
  - "A1 applied: an include.path value (gix Source::Local at level 1) does not configure the repo; git config --local --get agrees (exit 1)"
  - "A4 applied: D-18 refuses a repeated or empty value only for the key single reads; an empty wsx.protectedBranches and a repeated wsx.other leave resolve Configured"
  - "A ref namespace is refused in Repo::discover (Error::RefsNamespace) from repo.namespace(), not cleared after open"
  - "Scope tests carry a second control beyond the plan's: gix's unfiltered or source-filtered read must see the value the filter drops, so a gix that never loaded the file cannot pass the test"

patterns-established:
  - "config::tests use their own child_path (git::config::tests::) and START_VAR, and a child half prints count= and all="

requirements-completed: [CONF-07]

coverage:
  - id: D1
    description: "Only the repository's own config file configures it: include, global, config.worktree and GIT_CONFIG_COUNT values are not returned, each beside a git control that does see them; a linked worktree reads the common config"
    requirement: CONF-07
    verification:
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::included_key_is_not_read"
        status: pass
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::global_key_is_not_read"
        status: pass
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::local_key_is_read_by_the_same_child"
        status: pass
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::worktree_config_key_is_not_read"
        status: pass
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::command_line_config_key_is_not_read"
        status: pass
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::linked_worktree_reads_the_common_config"
        status: pass
      - kind: integration
        ref: "src/model/mod.rs#model::tests::global_source_branch_does_not_configure"
        status: pass
      - kind: integration
        ref: "src/model/mod.rs#model::tests::local_source_branch_configures_the_same_child"
        status: pass
      - kind: integration
        ref: "src/model/mod.rs#model::tests::included_source_branch_does_not_configure"
        status: pass
    human_judgment: false
  - id: D2
    description: "Value shapes equal git config --local --get / --get-all: repeated (one section and across sections), no-= as one empty value, absent, quoted and commented values, non-UTF-8 bytes, subsection key"
    requirement: CONF-07
    verification:
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::repeated_key_returns_every_value"
        status: pass
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::key_repeated_across_sections_returns_both"
        status: pass
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::key_without_a_value_reads_as_one_empty_value"
        status: pass
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::absent_key_reads_as_no_values"
        status: pass
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::value_is_normalised_as_git_does"
        status: pass
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::non_utf8_value_is_returned_raw"
        status: pass
      - kind: integration
        ref: "src/git/config.rs#git::config::tests::subsection_key_is_not_read"
        status: pass
    human_judgment: false
  - id: D3
    description: "D-18 per key: a repeated, empty or value-less wsx.sourceBranch is an Err from resolve naming the key; keys this phase does not read are not refused"
    requirement: CONF-07
    verification:
      - kind: integration
        ref: "src/model/mod.rs#model::tests::repeated_source_branch_fails_closed"
        status: pass
      - kind: integration
        ref: "src/model/mod.rs#model::tests::empty_source_branch_fails_closed"
        status: pass
      - kind: integration
        ref: "src/model/mod.rs#model::tests::source_branch_without_a_value_fails_closed"
        status: pass
      - kind: integration
        ref: "src/model/mod.rs#model::tests::keys_this_phase_does_not_read_are_not_refused"
        status: pass
    human_judgment: false
  - id: D4
    description: "Repo::discover refuses a ref namespace from local config or GIT_NAMESPACE, with git's plain tip asserted first; controls show gix reads another commit under the plain name once a namespace is set, and an empty GIT_NAMESPACE opens"
    verification:
      - kind: integration
        ref: "src/git/repo.rs#git::repo::tests::refs_namespace_in_config_is_refused"
        status: pass
      - kind: integration
        ref: "src/git/repo.rs#git::repo::tests::without_a_namespace_refs_read_as_git_reads"
        status: pass
      - kind: integration
        ref: "src/git/repo.rs#git::repo::tests::git_namespace_variable_is_refused"
        status: pass
      - kind: integration
        ref: "src/git/repo.rs#git::repo::tests::empty_git_namespace_opens"
        status: pass
      - kind: unit
        ref: "src/git/mod.rs#git::tests::refs_namespace_message_names_it_and_the_fix"
        status: pass
    human_judgment: false
  - id: D5
    description: "git::config::tests:: runs under a runner GIT_DIR, and DISCOVERING_SUITES equals the modules calling rerun_scrubbed (five suites)"
    verification:
      - kind: unit
        ref: "src/git/testing.rs#git::testing::discovering_suites_are_the_modules_that_rerun_scrubbed"
        status: pass
      - kind: integration
        ref: "src/git/testing.rs#git::testing::discovering_tests_pass_with_a_runner_git_dir"
        status: pass
    human_judgment: false

duration: 11min
completed: 2026-11-08
status: complete
---

# Phase 4 Plan 05: Config scope, repeated keys and ref namespaces Summary

**A `wsx.*` read now sees only the repository's own config file (`Source::Local` at include level 0), a repeated or empty `wsx.sourceBranch` is a named `Err` from `resolve` instead of git's last-one-wins, and `Repo::discover` refuses any ref namespace gix applies (`Error::RefsNamespace`), because gix would read other tips and replace refs than git.**

## Performance

- **Duration:** about 11 min
- **Started:** 2026-11-08T15:37:49Z
- **Completed:** 2026-11-08T15:48Z
- **Tasks:** 3 (each RED then GREEN)
- **Files modified:** 5

## Accomplishments

- An `include.path` file, global config, `config.worktree` and `GIT_CONFIG_COUNT` can no longer make a repository count as configured. Each case is checked beside git's own answer and beside a control showing gix did load the value the filter drops.
- `single` refuses a repeated key, listing every value in order, and refuses an empty value, including a key line with no `=`. It reads only the key it is asked for, so GUARD-05's empty `wsx.protectedBranches` stays usable.
- A ref namespace, set by `GIT_NAMESPACE` or `gitoxide.core.refsNamespace`, is refused when the repository is opened. The message names the namespace, both settings, and the command that shows where the key is set.
- `git::config::tests::` is held to the runner-`GIT_DIR` check, which now covers five suites.

## Task Commits

1. **Task 1 RED:** `57e3d5b` test(04-05): add failing config scope tests
2. **Task 1 GREEN:** `94fa10e` feat(04-05): read wsx config from this repository's own file only
3. **Task 2 RED:** `c7bbb72` test(04-05): add failing repeated-key tests
4. **Task 2 GREEN:** `788f53b` feat(04-05): refuse a repeated or empty wsx key
5. **Task 3 RED:** `271c876` test(04-05): add failing ref namespace tests
6. **Task 3 GREEN:** `d15bbdb` feat(04-05): refuse a ref namespace when opening a repository

No REFACTOR commits.

## TDD Evidence

**Task 1 RED:** `gsd_run check tdd-red-evidence target/gsd/04-05-red-evidence.json --raw` printed `"verdict": "RED_EVIDENCE_OK"` (`reason: target_test_failed`) for `git::config::tests::included_key_is_not_read`, nextest exit 100. Semantic assessment: the fixture and all three controls passed first: `git config --local --includes --get` printed `workspace/dev`, `--local --get` exited 1, and gix loaded the included value under `Source::Local`. The test then failed on its final assertion (`src/git/config.rs:184`): `local_config_values` returned `[b"workspace/dev"]` where `[]` was expected. That is the planned reason, since 04-01's filter checked only the source. In the same RED tree the other 12 config tests passed, as the plan expected.

**Task 2 RED:** `target/gsd/04-05-red-evidence-2.json` gave `RED_EVIDENCE_OK` (`target_test_failed`) for `model::tests::repeated_source_branch_fails_closed`, nextest exit 100. It failed with `resolve: expected Err(KeyRepeated), got Ok(Configured(Model { workspace_branch: b"workspace/dev", ... }))`: `single` took the last value. In the same tree, `empty_source_branch_fails_closed` and `source_branch_without_a_value_fails_closed` failed on their own assertions, getting `Err(NotABranchName { value: [] })` instead of `KeyEmpty`. The four other Task 2 tests passed, as planned. Task 1's level filter had already landed, so the global and include tests passed in RED.

**Task 3 RED:** `target/gsd/04-05-red-evidence-3.json` gave `RED_EVIDENCE_OK` (`target_test_failed`) for `git::repo::tests::refs_namespace_in_config_is_refused`, nextest exit 100. The git-side assertion passed first (`git show-ref --verify refs/heads/main` printed X). The test then failed at `src/git/repo.rs:3230` with `Repo::discover opened a repository whose refs gix reads in a namespace`, meaning `Repo::discover` answered `Ok`. `git_namespace_variable_is_refused` also failed: the child's `result` was not `refused`. The controls passed, as planned: `without_a_namespace_refs_read_as_git_reads`, `empty_git_namespace_opens` and the message test.

**Gate order:** each `test(04-05)` commit comes before its `feat(04-05)` commit (`57e3d5b` before `94fa10e`, `c7bbb72` before `788f53b`, `271c876` before `d15bbdb`).

## Gate runs

| Command | Exit |
|---|---|
| `scripts/check.sh all` (log form, `target/gsd/04-05-all.log`) after Task 1 GREEN, before a doc-only reflow | 0, last line `==> check.sh: all OK` |
| `scripts/check.sh all` after Task 1 GREEN, final text | 0, `==> check.sh: all OK`, 380 passed, 59 skipped |
| Task 1 loop, 13 tests, one `-E 'test(=git::config::tests::...)'` per command | 0 (13 x `1 test run: 1 passed`) |
| `discovering_suites_are_the_modules_that_rerun_scrubbed && discovering_tests_pass_with_a_runner_git_dir` | 0 |
| `scripts/check.sh all` after Task 2 GREEN | 0, `==> check.sh: all OK`, 387 passed, 60 skipped |
| Task 2 loop, 7 tests | 0 (7 x `1 test run: 1 passed`) |
| `scripts/check.sh all` after Task 3 GREEN | 0, `==> check.sh: all OK`, 392 passed, 60 skipped |
| Task 3 loop, 4 repo tests and the message test | 0 (5 x `1 test run: 1 passed`) |
| Every acceptance-criteria `rg` check for Tasks 1 to 3 | 0 |

## Files Created/Modified

- `src/git/config.rs`: `meta.level == 0` in `local_config_values` and its measured doc comment; the test module with 13 tests, the helpers, and `count_child`.
- `src/git/testing.rs`: `DISCOVERING_SUITES` is now `[&str; 5]`, with `git::config::tests::` first.
- `src/model/mod.rs`: `single` returns `Result<Option<Vec<u8>>, Error>` with D-18's refusals, and `resolve` propagates them. Adds 7 tests, `state_child`, and the helpers `append_config`, `resolved`, `git_config` and `child_state`.
- `src/git/repo.rs`: `refuse_refs_namespace` and its call in `Repo::discover`; `namespaced_repo`, `git_main_tip` and 4 tests.
- `src/git/mod.rs`: `Error::RefsNamespace` with its `Display` arm and `refs_namespace_message_names_it_and_the_fix`.

## Decisions Made

- The ordering and wording of the plan's message text were kept verbatim. `{namespace}` goes through `c_quote` and lossy UTF-8.
- `keys_this_phase_does_not_read_are_not_refused` and `source_branch_without_a_value_fails_closed` assert git's view of the fixture first: an empty `--get-all` line, two values for `wsx.other`, and `--local --get` printing an empty line. This way a fixture that failed to write the key cannot pass the test.

## Deviations from Plan

**1. [Rule 2 - Missing control] Scope tests also check that gix loaded the value it filters out**
- **Found during:** Task 1
- **Issue:** each scope test's planned control was git's answer only. If gix never loaded the include, global, `config.worktree` or `GIT_CONFIG_COUNT` value, the "not returned" assertion would pass without showing anything.
- **Fix:** in-process tests assert `values_from(repo, Source::Local | Source::Worktree)` sees the value. `count_child` also prints `all=`, the unfiltered count, and the global and `GIT_CONFIG_COUNT` tests require `count=0, all=1`. The local-child control requires `count=1, all=1`.
- **Files modified:** `src/git/config.rs`
- **Commit:** `57e3d5b`

**2. [Rule 2 - Missing control] `subsection_key_is_not_read` also reads `wsx.sub.sourceBranch`**
- **Found during:** Task 1
- **Issue:** without this, an empty result for `wsx.sourceBranch` cannot be told apart from a write that did not land.
- **Fix:** git `--local --get wsx.sub.sourceBranch` prints `x`, and `local_config_values("wsx.sub.sourceBranch")` equals it.
- **Commit:** `57e3d5b`

**Total deviations:** 2, both test-only and additive. **Impact:** the tests are stricter than the plan required. No production behaviour beyond the plan.

## Limits of what this shows

- The system-scope source (`/etc/gitconfig`) is not tested on its own. The filter excludes every source but `Local`, and the hermetic environment sets `GIT_CONFIG_NOSYSTEM=1`. Results were measured on git 2.56.0 and gix-config 0.62.2 only.
- A1 and A4 are readings of decisions, recorded for plan review. `keys_this_phase_does_not_read_are_not_refused` would fail under a section scan by construction, but no section-scan mutant was run against it. 04-10's mutation pass is where that is shown.
- The namespace refusal is tested from local config and from `GIT_NAMESPACE`. Global config and `GIT_CONFIG_COUNT` set the same gix value according to the planning-time measurement; that was **not re-measured or tested in this session**. The check reads `repo.namespace()`, not the source.
- The replace-map finding is carried from the plan's context into `refuse_refs_namespace`'s doc comment: gix reads a namespaced replacement, and `clear_namespace()` after open still reads it. It was **not re-measured in this session**. Neither was `git rev-parse --verify` ignoring the namespace. The tests here measure only `git show-ref --verify`.
- Cost: wherever gix applies a namespace, this refuses every wsx command, configured repository or not. A value in global config, or an exported `GIT_NAMESPACE`, stops wsx in every repository on the machine. git's own commands are unaffected. 04-10 Task 2 records phase 6's choice of how the guard treats a discovery refusal.
- `Repo::siblings` opens other worktrees with environment overrides off, so the variable does not reach them. A key in their shared config is refused at `discover` first. A key set only in another worktree's `config.worktree` is not checked, and the model reads no sibling.
- A runner with `GIT_NAMESPACE` exported fails every in-process discovering test with this refusal, loudly. `rerun_scrubbed` does not remove it, because `git rev-parse --local-env-vars` does not list it.
- The `discovering_tests_pass_with_a_runner_git_dir` pass shows `git::config::tests::` passes under a decoy `GIT_DIR`. No scratch-copy control with a guard stripped from `src/git/config.rs` was run in this plan. 04-01 ran that control for `model::tests::`.

## Issues Encountered

- First Task 3 compile: `gix::oid` has no `Display` (only `&oid` does), so `.map(ToString::to_string)` did not compile. Changed to `id.to_hex().to_string()` before the RED run.

## Known Stubs

None added. 04-01's narrow reads of `single` and `local_config_values` are replaced here.

## User Setup Required

None.

## Next Phase Readiness

04-06, 04-07 and 04-09 can call `single` for `wsx.baseRemote`, `wsx.pushRemote`, `wsx.baseRef` and `wsx.worktreeDir` and get D-18's refusals. 04-03 (wave 3) edits `src/git/testing.rs` after this plan's `DISCOVERING_SUITES` change.

## Self-Check: PASSED

- Files: `src/git/config.rs`, `src/git/testing.rs`, `src/model/mod.rs`, `src/git/repo.rs`, `src/git/mod.rs` and the three RED records `target/gsd/04-05-red-evidence{,-2,-3}.json` were all FOUND.
- Commits: `57e3d5b`, `94fa10e`, `c7bbb72`, `788f53b`, `271c876` and `d15bbdb` were all FOUND as ancestors of HEAD. `git rev-list --count 3039f9e..HEAD` = 6.
- All three RED records were re-classified at SUMMARY time and each gave `RED_EVIDENCE_OK`.
