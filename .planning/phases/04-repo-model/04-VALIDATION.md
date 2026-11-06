---
phase: "4"
slug: "repo-model"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
# audit-milestone §5.5 distinguishes NOT-VALIDATED (draft) from PARTIAL (validated + nyquist_compliant: false) (#2117)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-02"
---

# Phase 4 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | libtest unit tests in the binary crate, proptest 1.12.0 (added in this phase), run by cargo-nextest 0.9.148 |
| **Config file** | `.config/nextest.toml` (exists); `--no-tests=fail` is fixed by `scripts/check.sh` |
| **Quick run command** | `scripts/check.sh test -E 'test(=<module::tests::name>)'`, one test per command, chained with `&&` |
| **Full suite command** | `scripts/check.sh all`, plus `scripts/check.sh deps` |
| **Estimated runtime** | Not timed for the whole suite. A research prototype's unit tests alone took 4.4 s; the harness was not included in that figure |

---

## Sampling Rate

- **After every task commit:** Run the task's named tests through the quick run command
- **After every plan wave:** Run `scripts/check.sh all`
- **Before `/gsd-verify-work`:** `scripts/check.sh all`, `scripts/check.sh deps` and `scripts/check-in-container.sh all` must be green
- **Max feedback latency:** Not measured; see Estimated runtime

---

## Per-Task Verification Map

Filled by the planner at the revision after phase 03.1 (code at `6fbea02`), from plans 04-01 to 04-11 as revised then, and updated at the third-review revision (`2c191bf`): 04-05 gained Task 3, and rows 04-01 T1 and T2, 04-04 T2 (the 64 KiB limit, after the operator's ruling of 2026-11-05), 04-07 T1 and T2, 04-09 T1 and 04-11 T1 gained tests or threat references. Each command in a cell runs one test (`scripts/check.sh test -E 'test(=<module::tests::name>)'`), chained with `&&` or in the plan's `for` loop; the plan's `<verify>` holds the exact chain and the count of names. "File Exists" says whether the module holding the tests exists before the task: ❌ W0 means the task creates it (04-01, Wave 0 below), ✅ means an earlier task or phase did.

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 04-01 T1 | 04-01 | 1 | CONF-07, CONF-02 | T-04-01, T-04-33, T-04-37 (accepted) | the model's manifest comes from the workspace tip in the object database, not the working tree; `ref_commit` reads a ref by exact name, never through a revspec parser (03.1 D-04's read); the integration tip is told apart from the other rungs' distinct tips | tracer, unit (repo-backed, RED first) + source lint | `model::tests::configured_repo_resolves_from_the_object_database` and the eight others in Task 1's chain, including `git::refs::tests::ref_commit_reads_the_commit_a_describe_shaped_ref_holds`; the `src/git/refs.rs` source check, a lint and not evidence for T-04-37 | ❌ W0 (`src/model/`, `src/git/config.rs`); ✅ `src/git/refs.rs` (03.1) | ⬜ pending |
| 04-01 T2 | 04-01 | 1 | CONF-07 | T-04-02, T-04-03, T-04-36 | every `model::Error` names its cause and fix with bytes C-quoted; a printed command's arguments go through `shell_word` and are safe to paste; the model suite runs under a runner `GIT_DIR`, and the suite list is pinned to the modules under `src/git` and `src/model` that call `rerun_scrubbed` | unit + `/bin/sh` paste test with its control + scratch control | `model::tests::escape_bytes_in_a_value_render_quoted`, the eighteen message tests, `model::tests::command_lines_quote_a_value_for_the_shell`, `model::manifest::tests::escape_bytes_in_a_line_render_quoted`, `git::quote::tests::shell_word_quotes_what_a_shell_would_read`, `shell_word_has_no_form_for_bytes_a_quote_cannot_carry`, `pasted_command_removes_a_ref_named_with_shell_syntax`, `display_quoting_leaves_a_command_substitution_live`, `git::testing::discovering_tests_pass_with_a_runner_git_dir`, `git::testing::discovering_suites_are_the_modules_that_rerun_scrubbed`; the guard-stripped scratch-copy control | ❌ W0 (`src/git/quote.rs`); ✅ edit (`src/git/testing.rs`) | ⬜ pending |
| 04-02 T1 | 04-02 | 1 | CONF-03, CONF-04 | T-04-04 | the harness fixture writes a bare `wsx.baseRef` and explicit remotes, and no oracle record moves | harness test (RED first) + full suite | `fixture::devflow_layout_matches_the_preset`, `fixture::fork_layout_has_no_integration_branch_on_the_push_remote`, then `scripts/check.sh all` | ✅ edit | ⬜ pending |
| 04-05 T1 | 04-05 | 2 | CONF-07 | T-04-11 | only this repository's own config file is read: no global, included, per-worktree or command-line value | unit + re-executed child (RED first) | `git::config::tests::included_key_is_not_read` and the twelve others in Task 1's loop; `git::testing::discovering_suites_are_the_modules_that_rerun_scrubbed`, `git::testing::discovering_tests_pass_with_a_runner_git_dir` (the config suite joins the list here) | ✅ (04-01) | ⬜ pending |
| 04-05 T2 | 04-05 | 2 | CONF-07 | T-04-11, T-04-12 | a repeated or empty key read here fails closed; a global or included `wsx.sourceBranch` leaves the repo not configured | unit + child (RED first) | `model::tests::repeated_source_branch_fails_closed` and the six others in Task 2's loop | ✅ | ⬜ pending |
| 04-05 T3 | 04-05 | 2 | CONF-07 | T-04-34 | no `Repo` from `Repo::discover` reads refs through a namespace git does not use: `discover` refuses one from config or `GIT_NAMESPACE` (siblings are 04-05's stated limit) | unit + child (RED first), git's tip asserted first, two controls | `git::repo::tests::refs_namespace_in_config_is_refused`, `without_a_namespace_refs_read_as_git_reads`, `git_namespace_variable_is_refused`, `empty_git_namespace_opens`; `git::tests::refs_namespace_message_names_it_and_the_fix` | ✅ edit (`src/git/repo.rs`, `src/git/mod.rs`) | ⬜ pending |
| 04-03 T1 | 04-03 | 3 | CONF-01 | T-04-05, T-04-06 | quoting and unquoting equal git's on every byte value; text after the closing quote is refused | unit (RED first), git differential | `git::quote::tests::c_unquote_reverses_git_output_on_every_byte` and the seven others in Task 1's loop | ✅ (04-01) | ⬜ pending |
| 04-03 T2 | 04-03 | 3 | CONF-01 | T-04-05, T-04-07, T-04-27, T-04-SC | any git path round-trips through `.wsx.conf`; a run with no cases fails | proptest + unit (RED first) + scratch controls + deps | `model::manifest::tests::any_git_path_round_trips`, `property_runs_its_configured_cases` and the fourteen others in Task 2's loop; the encoder control; the `PROPTEST_CASES=0` and `4096` runs; the `cargo deny` ban control; `scripts/check.sh deps` | ✅ (04-01) | ⬜ pending |
| 04-06 T1 | 04-06 | 3 | CONF-04 | T-04-14 | a remote exists only when this repository defines it, as `git remote add` decides | unit + child (RED first) | `git::config::tests::keyless_local_remote_section_is_not_a_remote` and the six others in Task 1's loop | ✅ | ⬜ pending |
| 04-06 T2 | 04-06 | 3 | CONF-04 | T-04-13 | base and push remotes come from their own keys or `origin`; `remote.pushDefault` and `upstream` are never read or guessed; a resolved base's tip is told apart from the other remote's distinct `develop` | unit + child (RED first) | `model::tests::fork_layout_resolves_each_remote` and the twelve others in Task 2's loop | ✅ | ⬜ pending |
| 04-04 T1 | 04-04 | 4 | CONF-01 | T-04-09 | every line rule refuses naming the line; nothing is truncated or reinterpreted | unit, one per rule (RED first) | `model::manifest::tests::dot_dot_component_is_refused` and the thirty-nine others in Task 1's loop | ✅ | ⬜ pending |
| 04-04 T2 | 04-04 | 4 | CONF-01 | T-04-08, T-04-10 | a path cannot be both `only` and `modified` in either line order; a manifest over 64 KiB is refused before any line is read (operator ruling under D-12, 2026-11-05); the matcher equals git's literal pathspec | unit + proptest + git differential (RED first) | `model::manifest::tests::only_with_modified_under_it_overlaps` and the nineteen others in Task 2's loop, including `manifest_over_64_kib_is_refused`, its control `manifest_of_exactly_64_kib_parses` and `size_limit_is_checked_before_any_line` | ✅ | ⬜ pending |
| 04-07 T1 | 04-07 | 4 | CONF-03 | T-04-15, T-04-16, T-04-17, T-04-28, T-04-38 (accepted) | the chain reads the base remote's tracking refs only; a symbolic rung is refused; each resolved tip equals `git show-ref --verify` of its ref and differs from every rejected candidate's distinct tip | unit (RED first) + source lint | `model::tests::next_follows_develop` and the twenty-one others; `git::refs::tests::full_name_read_is_exact`, `tag_does_not_satisfy_a_branch_name`, `ref_commit_matches_show_ref`; the `integration` source check, a lint and not evidence for T-04-38 | ✅ | ⬜ pending |
| 04-07 T2 | 04-07 | 4 | CONF-03 | T-04-28, T-04-38 (accepted) | `wsx.baseRef` is a bare branch name read as one exact ref, with no fallback (03.1 D-04's read) | unit (RED first) + git differential + source lint | `model::tests::missing_base_ref_names_the_ref` and the seven others; `git::refs::tests::branch_names_agree_with_git`; the `integration` source check | ✅ | ⬜ pending |
| 04-08 T1 | 04-08 | 5 | CONF-05 | T-04-20 | the branch listing equals `git for-each-ref`'s one-level glob; a corrupt ref is an error, never a shorter list | unit (RED first) | `git::refs::tests::refs_directly_under_matches_for_each_ref` and the six others in Task 1's loop | ✅ | ⬜ pending |
| 04-08 T2 | 04-08 | 5 | CONF-05, CONF-07 | T-04-18, T-04-19, T-04-29 | only one local candidate is inferred and the repo stays not configured; zero or several are listed; `require` reads an unusable inferred manifest as run `wsx init` | unit (RED first) | `model::tests::only_candidate_is_inferred` and the twenty-seven others in Task 2's loop | ✅ | ⬜ pending |
| 04-09 T1 | 04-09 | 6 | CONF-07 | T-04-21, T-04-22, T-04-35 | only a regular file at the tip is a manifest; a root tree out of git's order is refused, never read; a configured repo fails closed with the reason | unit (RED first), `git ls-tree`, `git cat-file` and `git fsck` asserted first | `git::refs::tests::root_entry_reports_a_symlink` and the ten others, including `root_entry_refuses_a_tree_out_of_git_order` and its two controls; `model::tests::missing_manifest_fails_closed` and the nine others, including `unsorted_tree_manifest_fails_closed`; `git::tests::tree_not_sorted_message_names_the_tree_and_the_fix` | ✅ | ⬜ pending |
| 04-09 T2 | 04-09 | 6 | CONF-02 | T-04-22 | the only built-ins are `.wsx.conf` and the worktree dir; `wsx.worktreeDir` takes D-14's path rules | unit (RED first) | `model::tests::custom_worktree_dir_replaces_the_default` and the fourteen others in Task 2's loop | ✅ | ⬜ pending |
| 04-09 T3 | 04-09 | 6 | CONF-07, CONF-02 | T-04-23, T-04-31 | the list comes from the object database from any checkout; resolving writes nothing; the tip and the list come from one commit | unit + child (pins existing behaviour) | `model::tests::working_tree_edit_changes_nothing` and the eight others; `git::refs::tests::manifest_read_stays_on_the_commit_it_was_given` | ✅ | ⬜ pending |
| 04-11 T1 | 04-11 | 7 | CONF-07 | T-04-30, T-04-32, T-04-36 | a replacement gix and git follow differently is refused before any manifest read; one replacement reads the bytes git prints; the removal commands quote the ref name for the shell | unit + child (RED first) | `model::tests::replacement_chain_fails_closed` and the seventeen others; `git::tests::replacement_ref_message_names_the_ref_and_the_fix` and the four others, including `replacement_fix_commands_quote_a_ref_name_for_the_shell` | ✅ | ⬜ pending |
| 04-10 T1 | 04-10 | 8 | CONF-01, CONF-02, CONF-03, CONF-04, CONF-05, CONF-07 | T-04-24 | every mutant of the phase's core code is caught, unviable, timed out or explained as equivalent | mutation pass + survivor and fresh-pass checks | Task 1's three `<automated>` blocks, then `scripts/check.sh all` | — | ⬜ pending |
| 04-10 T2 | 04-10 | 8 | CONF-01, CONF-02, CONF-03, CONF-04, CONF-05, CONF-07 | T-04-25, T-04-26 | the gate is green on the host and in the pinned container; the model holds no gix type; the obligations are in STATE.md | gate + coverage + source checks | `scripts/check.sh all`, `scripts/check.sh deps`, `scripts/check-in-container.sh all` (which runs 04-01's paste tests under the image's dash), `scripts/check.sh coverage`, the two STATE.md checks (the obligations, and the phase 6 discovery-refusal line) | — | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `Cargo.toml`, `Cargo.lock`: add proptest with a justification comment; `scripts/check.sh deps` clean (04-03)
- [ ] `src/git/mod.rs`: `#[cfg(test)] pub(crate) mod testing;`, `mod config;` and `mod quote;` (`mod refs;` exists since 03.1), `pub use repo::Repo;` and the re-exports 04-01's own code uses (04-01); `RefTarget` joins with 04-07's `found` and `c_unquote` with 04-03's `parse`, each with its first user, since an unused re-export fails the clippy gate
- [ ] `src/main.rs`: `mod model;` (04-01)
- [ ] `src/model/mod.rs`, `src/model/manifest.rs`, `src/git/config.rs`: signatures with wrong-answer stubs, so RED comes from an assertion; `src/model/mod.rs` carries the `dead_code` expectation (04-01)
- [ ] `src/git/refs.rs`: wrong-answer stubs for `find_ref`, `ref_commit`, `root_entry` and `is_branch_name` beside 03.1's `peel_commit` and `peeled_commit`, which are reused, not stubbed (04-01)
- [ ] `src/git/testing.rs`: `DISCOVERING_SUITES` gains `model::tests::` and `discovering_suites_are_the_modules_that_rerun_scrubbed` scans `src/model` (04-01); the config suite joins with its first guarded test (04-05)
- [ ] Shared model-test fixtures: a helper building a repo with a remote, `main`, `develop` and `next` each on its own commit, and a `workspace/dev` holding a `.wsx.conf` (04-01); `testing::describe_shaped_ref` (03.1) for the describe-shaped ref read

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|

*All phase behaviors have automated verification.*

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency measured and recorded
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
