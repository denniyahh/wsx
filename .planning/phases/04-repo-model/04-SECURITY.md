---
phase: "4"
slug: "repo-model"
status: verified
# threats_open = count of OPEN threats at or above workflow.security_block_on severity (the blocking gate)
threats_open: 0
asvs_level: 1
created: "2026-11-08"
---

# Phase 4 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| workspace branch tip in the object database -> wsx | `.wsx.conf` bytes decide which paths may never reach a shared branch; the branch can arrive through a merge | manifest blob bytes (untrusted, any byte value) |
| local git config -> wsx | `wsx.*` values choose branches and remotes | config values (only this repository's own config file is read) |
| refs and replace refs -> wsx | ref names and targets choose the commits the model reads | ref names, object ids, replacement map |
| model errors -> the user's terminal | refusal messages print bytes that came from a blob or a config value | C-quoted bytes |
| a printed fix command -> the user's shell | the user may paste a command whose arguments came from a ref name or a config value | shell-quoted words |
| crates.io -> the build | proptest 1.12.0 (dev-only) joins the lockfile | dependency source, checked by `cargo deny` |

---

## Threat Register

Evidence is from the gsd-security-auditor run at `4e64219` (ASVS L1: mitigation present plus a named test). "Plan" is where the threat is defined.

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-04-01 | Tampering | 04-01 manifest source | high | mitigate | `src/model/mod.rs` reads `.wsx.conf` from the workspace tip's tree; `configured_repo_resolves_from_the_object_database`, `working_tree_edit_changes_nothing` | closed |
| T-04-02 | Spoofing | 04-01 error text | medium | mitigate | `shown` C-quotes every byte field; `escape_bytes_in_a_value_render_quoted`, `escape_bytes_in_a_line_render_quoted` (residual: `git::Error` text inside `Error::Git` and `ManifestUnreadable` renders unquoted) | closed |
| T-04-03 | Repudiation | 04-01 runner `GIT_DIR` | medium | mitigate | `DISCOVERING_SUITES` lists `model::tests::`; `discovering_tests_pass_with_a_runner_git_dir` (the suite-list test is a source scan) | closed |
| T-04-33 | Tampering | 04-01 `ref_commit` | medium | mitigate | `find_ref` then `peel_commit`, no revspec; clippy ban; `ref_commit_reads_the_commit_a_describe_shaped_ref_holds` | closed |
| T-04-36 | Tampering | 04-01 printed commands | medium | mitigate | `shell_word` on command arguments; `pasted_command_removes_a_ref_named_with_shell_syntax` with control `display_quoting_leaves_a_command_substitution_live` | closed |
| T-04-37 | Tampering | 04-01 `ref_commit` re-read race | low | accept | see Accepted Risks Log | closed |
| T-04-SC | Tampering | proptest dependency (all plans) | high | mitigate | dev-only, lockfile additions only, `--locked` builds, `scripts/check.sh deps` exit 0 | closed |
| T-04-04 | Tampering | 04-02 harness fixture | medium | mitigate | no `wsx.*` key outside `tests/harness/fixture.rs` (rg absence plus positive control); `fixture::devflow_layout_matches_the_preset`, `fixture::fork_layout_has_no_integration_branch_on_the_push_remote` | closed |
| T-04-05 | Information Disclosure | 04-03 path matching | high | mitigate | byte comparison in `covers`; `git_quoted_output_parses_to_git_raw_names`, `c_unquote_reverses_git_output_on_every_byte`, proptest `any_git_path_round_trips` | closed |
| T-04-06 | Tampering | 04-03 `c_unquote` | medium | mitigate | consumed-length check; `c_unquote_refuses_text_after_the_closing_quote`, `text_after_a_quoted_path_is_bad_quoting` | closed |
| T-04-07 | Denial of Service | 04-03 proptest persistence | low | mitigate | `failure_persistence: None` in `property_config()` (config only, no behavioural test) | closed |
| T-04-27 | Repudiation | 04-03 proptest case count | low | mitigate | `property_runs_its_configured_cases` | closed |
| T-04-08 | Tampering | 04-04 `only`/`modified` overlap | high | mitigate | overlap check in both line orders; `only_with_modified_of_the_same_path_overlaps`, `modified_with_only_under_it_overlaps` and four direction tests | closed |
| T-04-09 | Information Disclosure | 04-04 line rules | high | mitigate | every rule refuses naming the line; `trailing_text_is_part_of_the_path`, `line_ending_in_cr_is_refused`, `dot_dot_component_is_refused`, `raw_nul_is_refused` | closed |
| T-04-10 | Denial of Service | 04-04 manifest size | low | mitigate | `MAX_MANIFEST_BYTES` (64 KiB) checked first; `manifest_over_64_kib_is_refused`, `manifest_of_exactly_64_kib_parses`, `size_limit_is_checked_before_any_line` (residual: `root_entry` reads the whole blob first) | closed |
| T-04-11 | Elevation of Privilege | 04-05 config scope | high | mitigate | local file at level 0 only; `global_key_is_not_read`, `included_key_is_not_read`, `worktree_config_key_is_not_read`, `command_line_config_key_is_not_read` | closed |
| T-04-12 | Tampering | 04-05 repeated keys | medium | mitigate | `single` fails closed; `repeated_source_branch_fails_closed`, `repeated_key_message_lists_the_values_and_the_fix` | closed |
| T-04-34 | Tampering | 04-05 refs namespace | high | mitigate | `Repo::discover` refuses a namespace; `refs_namespace_in_config_is_refused`, `git_namespace_variable_is_refused` (limit: `Repo::siblings` is not covered; the model reads no sibling) | closed |
| T-04-13 | Information Disclosure | 04-06 remotes | high | mitigate | own keys or `origin` only; `push_default_is_not_read`, `upstream_is_not_auto_detected` | closed |
| T-04-14 | Spoofing | 04-06 remote existence | medium | mitigate | local and worktree sections only; `global_remote_section_is_not_a_remote` (`git remote add` differential) | closed |
| T-04-15 | Tampering | 04-07 integration chain | high | mitigate | base remote's tracking refs only; `local_develop_does_not_redirect_a_remote_base` | closed |
| T-04-16 | Tampering | 04-07 exact ref names | medium | mitigate | full-name compare; `full_name_read_is_exact`, `tag_does_not_satisfy_a_branch_name` | closed |
| T-04-17 | Repudiation | 04-07 remote HEAD | low | mitigate | `remote_head_outside_the_remote_is_not_a_default`, `dangling_remote_head_is_not_a_default`, `object_remote_head_is_not_a_default` | closed |
| T-04-28 | Tampering | 04-07 symbolic rungs | medium | mitigate | `found` refuses symbolic refs; `symbolic_develop_is_refused`, `symbolic_base_ref_is_refused` (race half accepted as T-04-38) | closed |
| T-04-38 | Tampering | 04-07 `found` re-read race | low | accept | see Accepted Risks Log | closed |
| T-04-18 | Denial of Service | 04-08 candidate inference | high | mitigate | not configured is a value, not an error; `several_candidates_do_not_fail_resolve`, `invalid_inferred_manifest_is_not_configured` | closed |
| T-04-19 | Tampering | 04-08 candidate choice | high | mitigate | `checked_out_branch_gets_no_preference` | closed |
| T-04-20 | Tampering | 04-08 branch listing | medium | mitigate | a corrupt ref is an error; `corrupt_ref_under_the_prefix_is_an_error`, `corrupt_candidate_ref_stays_in_inferred` | closed |
| T-04-29 | Denial of Service | 04-08 `require` | low | mitigate | `require_wraps_an_unreadable_manifest_as_not_configured`, `require_passes_a_git_failure_through` | closed |
| T-04-21 | Tampering | 04-09 manifest entry type | high | mitigate | mode check before blob read; `symlink_manifest_fails_closed`, `root_entry_reports_a_symlink` | closed |
| T-04-35 | Tampering | 04-09 tree order | medium | mitigate | `root_entry_refuses_a_tree_out_of_git_order` (fsck asserted) and controls; `unsorted_tree_manifest_fails_closed` | closed |
| T-04-22 | Information Disclosure | 04-09 fail closed | high | mitigate | every failure is `Err`; `missing_blob_object_fails_closed`, `missing_tip_commit_fails_closed`, `missing_manifest_fails_closed`, `invalid_manifest_fails_closed` | closed |
| T-04-23 | Tampering | 04-09 working tree independence | high | mitigate | object-database read; `working_tree_edit_changes_nothing` (`git status` control) | closed |
| T-04-31 | Tampering | 04-09 one-commit read | low | mitigate | `root_entry` takes the commit id; `manifest_read_stays_on_the_commit_it_was_given` (residual: the two tips can come from different instants) | closed |
| T-04-24 | Repudiation | 04-10 mutation pass | high | mitigate | cargo-mutants 27.1.0 at `09eca8c`: 154 caught, 0 missed, 23 unviable, 0 timeout; no source change since (process evidence, not a test) | closed |
| T-04-25 | Repudiation | 04-10 container gate | medium | mitigate | `scripts/check-in-container.sh all` exit 0 at `b505f2d`, paste tests under dash; docs-only commits since | closed |
| T-04-26 | Repudiation | 04-10 obligations | medium | mitigate | eight Phase 5/6/8/12 obligation lines in STATE.md `### Blockers/Concerns` | closed |
| T-04-30 | Tampering | 04-11 replacement refs | medium | mitigate | guard runs first in `root_entry`; `replacement_chain_fails_closed`, `replacement_cycle_fails_closed`, `symbolic_replacement_fails_closed` and controls (limit: Repo reads before `resolve` are unguarded; Phase 5 obligation in STATE.md). **Reopened by code review CR-01 (04-REVIEW.md):** the guard lists only refs directly under the replace base, but git 2.56.0 also applies a nested `refs/replace/<dir>/<hex>` (reproduced by the orchestrator, with direct-ref and no-ref controls); per the reviewer's gix 0.90.0 probe, gix does not, so wsx and git can read different `.wsx.conf` bytes with no error. WR-01: a replace base without a trailing `/` makes the guard check nothing | open — below high threshold (non-blocking) |
| T-04-39 | Tampering | 04-11 removal commands | medium | mitigate | `ref_word` quotes the ref name (applies T-04-36); `replacement_fix_commands_quote_a_ref_name_for_the_shell` | closed |
| T-04-32 | Tampering | 04-11 replace ref after open | low | accept | see Accepted Risks Log | closed |

*Status: open · closed · open — below high threshold (non-blocking)*
*Severity: critical > high > medium > low — only open threats at or above workflow.security_block_on count toward threats_open*
*Disposition: mitigate (implementation required) · accept (documented risk) · transfer (third-party)*

---

## Accepted Risks Log

| Risk ID | Threat Ref | Rationale | Accepted By | Date |
|---------|------------|-----------|-------------|------|
| AR-04-01 | T-04-37 | A ref changed between two reads inside `ref_commit` needs a concurrent writer of the user's own refs during one run, which can already move any ref wsx reads; git has no multi-ref snapshot; no test can place the change without a seam that exists for no other reason. The one-`find_ref` source check is a lint, not evidence | plan-time disposition (04-01, third-review replan `09bdc2b`); no separate owner sign-off recorded | 2026-11-05 |
| AR-04-02 | T-04-38 | Same reasoning as T-04-37, applied to `integration` and `found`; the source check is a lint only | plan-time disposition (04-07, third-review replan `09bdc2b`); no separate owner sign-off recorded | 2026-11-05 |
| AR-04-03 | T-04-32 | A replace ref changed during one run; git has no snapshot to read against; stated in the guard's doc comment and in `resolve`'s consistency contract | owner (second review, finding 7) | 2026-11-03 |

*Accepted risks do not resurface in future audit runs.*

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-08 | 40 | 40 | 0 | gsd-security-auditor (ASVS L1, block_on high), at `4e64219` |
| 2026-11-08 | 40 | 39 | 0 blocking (1 open below threshold: T-04-30) | orchestrator, from gsd-code-reviewer CR-01 / WR-01 (deep), at `2fc6a29` |

What the audit does not establish: depth beyond L1 (no end-to-end trace; no production entry point exists until phase 5's `wsx check`); 24 of the cited tests were re-run by the auditor, the rest rest on the full suite at `74ba890` (639 passed); T-04-07 is a config line, half of T-04-03 is a source scan and half of T-04-04 is an `rg` absence check; `src/git/repo.rs` was outside the mutation pass; parity evidence is from git 2.56.0 only.

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-08
