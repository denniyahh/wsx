---
gsd_state_version: "1.0"
milestone: v0.1.0
current_phase: 04
current_phase_name: Repo model
status: executing
stopped_at: Completed 04-17-PLAN.md
last_updated: "2026-11-17T23:38:48.969Z"
last_activity: 2026-11-17
last_activity_desc: Phase 04 execution started
state_head: c55138471a0dbe1d9f459615276bd9c3fa8ce0cd
progress:
  total_phases: 16
  completed_phases: 4
  total_plans: 54
  completed_plans: 53
---

# Project State

## Project Reference

See: .planning/PROJECT.md (updated 2026-11-04)

**Core value:** wsx must never let a personal file reach a shared branch or PR, and must never lose work while moving it between the workspace branch, feature worktrees and PR branches.
**Current focus:** Phase 04 — Repo model

## Current Position

Phase: 04 (Repo model) — EXECUTING
Plan: 18 of 18
Status: Ready to execute
Last activity: 2026-11-17 — Phase 04 execution started

Progress: [███░░░░░░░] 25%

## Performance Metrics

**Velocity:**
- Total plans completed: 36
- Average duration: -
- Total execution time: 0.0 hours

**By Phase:**

| Phase | Plans | Total | Avg/Plan |
|-------|-------|-------|----------|
| 1 | 7 | - | - |
| 2 | 15 | - | - |
| 3 | 10 | - | - |
| 03.1 | 4 | - | - |

**Recent Trend:**
- Last 5 plans: -
- Trend: -

*Updated after each plan completion*
**Per-Plan Metrics:**

| Plan | Duration | Tasks | Files |
|------|----------|-------|-------|
| Phase 1 P01 | 4 min | 2 tasks | 9 files |
| Phase 01 P02 | 6min | 2 tasks | 3 files |
| Phase 01 P05 | 5min | 2 tasks | 2 files |
| Phase 01 P03 | 9min | 2 tasks | 3 files |
| Phase 01 P04 | 4min | 2 tasks | 4 files |
| Phase 01 P06 | 6min | 2 tasks | 2 files |
| Phase 02 P01 | 3min | 3 tasks | 10 files |
| Phase 02 P02 | 18min | 2 tasks | 9 files |
| Phase 02 P03 | 7min | 3 tasks | 12 files |
| Phase 02 P04 | 7 min | 2 tasks | 2 files |
| Phase 2 P05 | 11 min | 2 tasks | 3 files |
| Phase 02 P06 | 6 min | 2 tasks | 4 files |
| Phase 02 P07 | 37 min | 2 tasks | 6 files |
| Phase 02 P08 | 8min | 2 tasks | 3 files |
| Phase 02 P09 | 9 min | 2 tasks | 1 files |
| Phase 2 P10 | 9 min | 3 tasks | 1 files |
| Phase 02 P11 | 8 min | 2 tasks | 1 files |
| Phase 02 P12 | 9 min | 2 tasks | 2 files |
| Phase 02 P13 | 4min | 2 tasks | 1 files |
| Phase 02 P14 | 10min | 2 tasks | 1 files |
| Phase 02 P15 | 7 min | 2 tasks | 4 files |
| Phase 03 P01 | 13 min | 2 tasks | 9 files |
| Phase 03 P02 | 2 min | 2 tasks | 2 files |
| Phase 03 P03 | 9 min | 3 tasks | 1 files |
| Phase 03 P04 | 4 min | 2 tasks | 2 files |
| Phase 03 P05 | 6 min | 2 tasks | 2 files |
| Phase 03 P06 | 3 min | 2 tasks | 1 files |
| Phase 03 P07 | 8 min | 3 tasks | 3 files |
| Phase 03 P08 | 8min | 3 tasks | 3 files |
| Phase 03 P09 | 5min | 2 tasks | 2 files |
| Phase 03 P10 | 17min | 4 tasks | 5 files |
| Phase 03.1 P01 | 7 min | 2 tasks | 5 files |
| Phase 03.1 P02 | 6 min | 2 tasks | 3 files |
| Phase 03.1 P03 | 5 min | 2 tasks | 2 files |
| Phase 03.1 P04 | 7 min | 3 tasks | 4 files |
| Phase 04 P01 | 17 min | 2 tasks | 8 files |
| Phase 04 P02 | 3 min | 1 tasks | 1 files |
| Phase 04 P05 | 11min | 3 tasks | 5 files |
| Phase 04 P03 | 11 min | 2 tasks | 6 files |
| Phase 04 P06 | 7 min | 2 tasks | 2 files |
| Phase 04 P04 | 8min | 2 tasks | 1 files |
| Phase 04 P07 | 8min | 2 tasks | 3 files |
| Phase 04 P08 | 14 min | 2 tasks | 2 files |
| Phase 04 P09 | 13 min | 3 tasks | 4 files |
| Phase 04 P11 | 12 min | 1 tasks | 3 files |
| Phase 04 P10 | 64min | 2 tasks | 3 files |
| Phase 04 P12 | 32 min | 3 tasks | 5 files |
| Phase 04 P13 | 7 min | 2 tasks | 3 files |
| Phase 04 P14 | 12 min | 2 tasks | 5 files |
| Phase 04 P15 | 25 min | 2 tasks | 5 files |
| Phase 04 P16 | 19 min | 3 tasks | 6 files |
| Phase 04 P17 | 6min | 2 tasks | 3 files |

## Accumulated Context

### Decisions

Decisions are logged in PROJECT.md Key Decisions table.
Recent decisions affecting current work:

- Roadmap: horizontal layers (owner). Phases run strictly in order; each oracle-backed layer (Phases 5-10) enables and passes the Rust side of its scenarios before the next starts
- Roadmap: every HARN requirement lands by Phase 2 (HARN-04, the nextest gate and clippy env-mutation ban, in Phase 1); later layers enable the Rust side of the scenarios Phase 2 records against the oracle
- Roadmap: the `--json`/exit-code contract (IF-01) and the no-GSD rule (IF-02) start with the first command (Phase 5), so every later command joins an enforced contract
- Roadmap: a separate release gate (Phase 14) precedes cutover; cutover order is the wsx repo, then devflow's repo tooling (Phase 15)
- [Phase 1]: tracked `.gitconfig` is copied into local config by `setup-repo.sh` (allowlisted keys; `all|policy|check`), never included (owner, 2026-10-24)
- [Phase 1]: tool pins are verified through `cargo` itself (`install-tools.sh verify-cargo`), not by a PATH or CARGO_HOME search (code review CR-02)
- [Phase 1]: the PR cut aborts on a failed cherry-pick, refuses protected branches, and re-cuts only with `--force` (CR-03, WR-04)
- [Phase 1]: phase-1 fixes made after PR #2 merged (f1f1303) reach develop only through a follow-up PR; a phase-N cut replays only phase-N commits
- [Phase 2]: the vendored oracle stays byte-identical to devflow ee6ec792; oracle.patch is its only adaptation and is applied with git apply under GIT_CEILING_DIRECTORIES (without it, git apply in a nested repo skips and exits 0)
- [Phase 2]: deny.toml license-checks dev-dependencies (include-dev = true); cargo-deny 0.20.2 skips them by default
- [Phase 2]: Harness fixture steps are fn(&mut Fixture) -> Result<(), String> and run_case(side, &Entry, &Case, make_env) returns fixture and adapter errors instead of panicking (02-02)
- [Phase 2]: Adapters return Result<ActionRun, AdapterError { NotImplemented | Spawn }>; an oracle-side NotImplemented panics naming the scenario and action (02-02)
- [Phase 2]: CommitExpect has no committer date per the 02-02 contract; only the author date is pinned to the action instant (resolved 2026-10-26 by the orchestrator: no field — HARN-03 lists commits as message + author + author date + committer name/email + tree and deliberately leaves out the committer date, which a replay or merge sets at run time and so cannot be pinned the same way for oracle and Rust)
- [Phase 2]: Harness registry test allows a scenario module with no scenarios yet (scenarios! {}); a scenario fn left out of scenarios! is rejected by clippy as unused (02-03)
- [Phase 2]: Fixture::commit and Fixture::git_tick take dates from one private Fixture::tick; a git_tick command must create exactly one commit, which is documented but not enforced (02-03)
- [Phase 02]: 02-04: run_script always sets WSX_ORACLE_DIR, so the absent-variable oracle test spawns the same bash command line itself; the with-variable test also requires a stdout marker because phase-worktree.sh exits 0 when the oracle dir is wrong
- [Phase 2]: 02-05: a fixed: case is judged at run time against its expected outcome before its record (judge returns FixedPassesOnOracle when the oracle run meets the expectation); FixedRecordMatchesExpected stays as the early literal-copy check
- [Phase 2]: 02-05: a rust_only scenario's cases must all use OracleSide::NotImplemented; SameAsExpected counts as claiming an oracle side
- [Phase 2]: 02-05: guard control direction is judged by expected exit only (known-positive non-zero, clean 0); a regression: refusal is exit != 0, RefsExpect::Unchanged and no new commits
- [Phase 2]: 02-05: validate runs inside try_run_with_env, so every generated test validates before any fixture is built; reports are '<module>::<name>, <case><class>: <failure>'
- [Phase 02]: 02-06: cargo-llvm-cov is pinned in install-tools.sh with direct_sub=llvm-cov (the binary rejects a bare --version when run directly); install and verify pass that word, verify-cargo does not; its release binary prints exactly 'cargo-llvm-cov 0.9.1', so the pattern keeps the $ anchor
- [Phase 02]: 02-06: llvm-tools comes only from rust-toolchain.toml (rustup toolchain install sufficed on the host; the pinned image installs it too); every cargo llvm-cov run sets CARGO_LLVM_COV_SETUP=no
- [Phase 02]: 02-06: one-time coverage proof recorded: println! count 1 for the spawned subject, 0 for no spawn and 0 for spawn without LLVM_*; no gate or CI job runs coverage
- [Phase 2]: 02-07: worktree::self_test is fixed: by observation (oracle dir .worktrees/phase-1 vs wsx .worktrees/feature/phase-1); cut::self_test is Parity by observation (script returns to the source branch)
- [Phase 02]: 02-08: names_are_observed_byte_exact declares i18n.commitEncoding=ISO-8859-1, because git itself re-encodes a non-UTF-8 message byte under the default UTF-8 encoding
- [Phase 02]: 02-08: comparator needed no fix; all 25 single-field mutations of sync::self_test are reported as their own field
- [Phase 02]: 02-08: signing is declared with RepoConfig::with_signing(); all three signing cases are Parity by observation
- [Phase 02]: 02-09: the planted host config sets gpg.program to a recording script that fails, because a real gpg started a keyboxd daemon under the scenario HOME that outlived the test
- [Phase 02]: 02-09: the planted-host proof's env-value check is the only check that catches a leaked HOME; the behavioural checks (marker, decoy, outcomes) do not in the sync, cut and guard self-tests
- [Phase 2]: 02-10: guard scenario classes come from the first Parity run; P1-P5 reproduced as researched. In the fork fixture --remotes=origin did not over-scan; the hook's tail refused the workspace push on a hard-coded origin/develop instead
- [Phase 2]: cargo fmt does not reach scenario modules declared inside harness_modules!; guard.rs is formatted with rustfmt directly and the gap is logged in deferred-items.md and WINDOWS.md
- [Phase 02]: 02-11: every cut case was classed by its first Parity run; 6 subjects fixed:, integration_merged_into_feature and all 7 counterparts Parity, all as predicted
- [Phase 02]: 02-11: merge_resolution_adds_personal_file is fixed: only for the stray feature/topic-pr ref; the oracle refuses on an earlier replay conflict and never reaches the merge
- [Phase 02]: 02-11: a pre-existing PR branch in a cut fixture is never left checked out in another worktree (git would refuse the oracle's branch -f and checkout)
- [Phase 02]: 02-12: every oracle case classed by its first Parity run: 7 Parity (all 6 sync cases, the worktree re-run counterpart), 5 fixed: (the 3 worktree subjects and 2 directory-name counterparts)
- [Phase 02]: 02-12: worktree::no_gsd_repo's oracle refusal exits 2, not RESEARCH's exit 1 (the error-path grep fails on the missing ROADMAP.md; pipefail and set -e pass 2 through); recorded as observed
- [Phase 02]: 02-12: sync::fork_layout's counterpart (upstream removed) expects exit 128, taken from the oracle run as the plan directs; no requirement fixes that exit code, phase 7 may choose another
- [Phase 02]: 02-13: check::quoted_personal_names and check::no_gsd_repo are fixed: (oracle exit 1, refusing C-quoted .planning names and, without .workspace-divergence, .wsx.conf and notes/todo.md); check::fork_layout is Parity against upstream/develop; no_gsd_custom_personal_path is Rust-only
- [Phase 02]: 02-14: fork_layout advances upstream/develop so the expected PR parent (feat: add d) tells upstream/develop apart from local develop
- [Phase 02]: 02-14: merge_resolution_only_personal_file reads PRCUT-01 own resolution changes as changes against every parent (combined diff); a conflict resolved to develop's side is not one. Phase 9 should confirm
- [Phase 02]: 02-14: all four planned cut oracle scenarios observed Parity as predicted; fork counterpart exit 128 recorded; the orchestrator-added merge counterpart is fixed:
- [Phase 02]: 02-15: harn06_gaps names an untagged HARN-06 kind once (not per action); ORACLE_ACTIONS lists 19 (kind, action) pairs that need an oracle-recorded scenario
- [Phase 02]: 02-15: vendored oracle file modes are checked from the git index (git ls-files -s) in provenance.rs; every scenario module must register a scenario
- [Phase 03]: Owner 2026-10-26: planning granularity stays `fine` for phase 3 onward (question carried from the phase 2 pause; config unchanged)
- [Phase 03]: 03-01: merge-base control pairs main/feat~3 (F1) with main~2/feat (C1); git's single base for main/feat on the criss-cross fixture is F1, so comparing against it cannot catch a constant
- [Phase 03]: 03-01: gix-protocol/gix-transport are unconditional gix 0.90.0 deps; the no-network check asserts gix-transport has only its empty default feature and no client crate (gix-credentials, curl, reqwest) is in the tree
- [Phase 03]: 03-02: the oracle's record on reftable is verdict(0), observed through the harness (exit 0, refs unchanged); it matches RESEARCH's scratch prediction
- [Phase 03]: 03-03: ahead_behind is built on range_walk (two hidden gix walks counted by set size), so the two reads cannot disagree; walk results are BTreeSet<ObjectId>, no order promised
- [Phase 03]: 03-03: tree_diff drops tree-mode records and maps a mode change to TypeChanged only across blob/symlink/submodule; an executable-bit change alone is Modified, pinned against git by an added exec-bit test
- [Phase 03]: The sanctioned gix discover_with_environment_overrides_opts is itself on the clippy ban list, so open_gix's expect holds only while the ban does and a second call anywhere fails the gate
- [Phase 03]: 03-05: git version check and --local-env-vars list are cached per helper, not per process (each helper resolves git via its own snapshot's PATH); production builds one helper per process
- [Phase 03]: 03-05: own-versus-other worktree is exact canonical equality, never a prefix test, because wsx nests sibling worktrees inside its own worktree directory
- [Phase 03]: 03-06: the reftable backstop maps on gix_error is_unsupported(), observed true for the reftable HEAD decode error and false for a missing branch; no message-text match
- [Phase 03]: An empty --local-env-vars list fails closed as the new Error::GitEmptyOutput (GitFailed would render 'exited 0', which does not name the cause)
- [Phase 03]: testing::reexec is the re-exec harness for 03-08..03-10: parents assert a WSXCHILD marker line, never the exit status alone
- [Phase 03]: 03-08: Repo::siblings opens other worktrees only via gix main_repo()/worktrees(), sorted by canonical worktree path; with_removal_disabled is cfg(test) only and skips the whole removal branch
- [Phase 03]: 03-09: ForeignOwner carries the canonical worktree (git dir if bare); git prints the same path, so the fix lines are equal
- [Phase 03]: 03-09: refuse_untrusted refuses unless gix git_dir_trust() is Trust::Full, before refuse_ref_storage; -c/GIT_CONFIG_COUNT safe.directory and SUDO_UID divergences documented
- [Phase 03]: GIT_CEILING_DIRECTORIES: refuse_at_ceiling refuses as NotARepo when a canonical ceiling entry equals the found worktree and the start differs (gix searches the ceiling dir; git does not)
- [Phase 03]: Discovering unit tests call testing::rerun_scrubbed first: re-run without git rev-parse --local-env-vars variables when the runner has any; production discovery still honours GIT_DIR
- [Phase 03]: extensions.refStorage is read from gix Source::Local only; git 2.56.0 ignores it in global config, config.worktree and GIT_CONFIG_COUNT
- [Phase 3]: Review fixes (03-REVIEW-FIX it.2-3): the ceiling check compares the directory git discovery stopped in, not the worktree; GIT_DIR without a worktree setting takes git's worktree; SUDO_UID is honoured only as root (72a4bd4)
- [Phase 03.1]: 03.1-01: merge_base takes git::ObjectId and peels through Repo::peel_commit (src/git/refs.rs); ObjectId::to_gix decodes hex only; test ids come from testing::object_id (git rev-parse --verify)
- [Phase 03.1]: 03.1-01: until 03.1-03, ahead_behind, range_walk and tree_diff still take revspec bytes; each read's doc says so, and to_gix has no hash-kind check until 03.1-02
- [Phase 03.1]: 03.1-02: ObjectId::to_gix takes the repository hash kind and refuses an id of the other kind before lookup (read object id <hex>: a sha1 id in a sha256 repository); peeled_commit and commit_id pass self.repo.object_hash()
- [Phase 03.1]: 03.1-02: peel_commit refuses a branch name, an abbreviated id, an empty id and a describe-shaped ref name at the hex decode, while git resolves them; tests assert the refusal with git as the contrast (D-01, git-layer half of D-04)
- [Phase 03.1]: 03.1-03: ahead_behind, range_walk and tree_diff take git::ObjectId; commit reads peel through commit_id/peel_commit, tree_diff peels to a tree-ish via tree_of(&ObjectId); no read in reads.rs calls Repo::resolve or resolve_commit; read errors name hex ids only
- [Phase 03.1]: 03.1-03: deleted tree_diff_keeps_the_reftable_refusal and files_repo_diffs_head_through_the_same_path (operator decision 2026-11-03: a read on ids reaches no ref); open_unchecked and three backstop tests stay until 03.1-04
- [Phase 03.1]: Reftable backstop retired by the operator on 2026-11-03 (03.1-RESEARCH Open Question 1): Phase 3's backstop, gix's unsupported-storage error mapped to the reftable refusal inside `Repo::resolve`, is removed with `resolve`, amending Phase 3 CONTEXT's backstop clause; `refuse_ref_storage` at discover is the only reftable guard; on reftable, gix's exact ref read returns `None` and its ref listing is empty, never an unsupported error (04-RESEARCH Pitfall 10), so no backstop is reachable and Phase 4 carries no obligation for one.
- [Phase 04]: 04-01: find_ref treats a ref gix returns under another name as absent (gix 0.90.0 falls back to .git/heads/<full>, .git/tags/, .git/remotes/, which git never reads)
- [Phase 04]: 04-01: ref_commit follows symbolic refs only through find_ref, at most 5 reads (git 2.56.0: 4 hops resolve, 5 refused), and peels the id each read returned
- [Phase 04]: 04-01: printed fix commands take byte arguments through shell_word, or a placeholder (<remote>, <ref>) when no shell quoting can carry the bytes; prose uses c_quote
- [Phase 04]: 04-01: tree.find_entry takes the raw &[u8] name; RESEARCH Pattern 6's .as_bstr() argument does not compile on gix 0.90.0
- [Phase 04]: Harness fixture writes wsx.baseRef as the bare integration branch plus explicit wsx.baseRemote and wsx.pushRemote from RepoConfig; the fork preset states baseRemote=upstream — D-02, D-05, D-08: phase 5 Rust sides must start from config the repo model accepts
- [Phase 04]: 04-05: A1 applied - an include.path value (gix Source::Local level 1) does not configure the repo; git config --local --get agrees (exit 1)
- [Phase 04]: 04-05: A4 applied - D-18 refuses a repeated or empty value only for the key single reads; an empty wsx.protectedBranches and a repeated wsx.other leave resolve Configured
- [Phase 04]: 04-05: a ref namespace (GIT_NAMESPACE or gitoxide.core.refsNamespace) is refused in Repo::discover as git::Error::RefsNamespace, read from repo.namespace(), not cleared after open
- [Phase 04]: 04-03: manifest quoting is c_unquote (gix undo plus a refusal of text after the closing quote) and encode_path (c_quote plus quotes around a bare path whose edge, one trailing / set aside, is A6 whitespace); both equal git 2.56.0 on every byte value
- [Phase 04]: 04-03: Problem::BadQuoting gets its own test (only "a"b refused), not in the plan's sixteen; 04-04 adds none
- [Phase 04]: 04-03: every proptest! block in model::manifest takes property_config(); property_runs_its_configured_cases fails below 2048 cases, so PROPTEST_CASES=0 is a named failure
- [Phase 04]: 04-06: remote_configured counts a remote section with a key from Source::Local or Source::Worktree at any include level, matching git remote add (an included section counts, unlike local_config_values)
- [Phase 04]: 04-06: build resolves wsx.baseRemote before wsx.pushRemote, after the workspace-branch check; remote.pushDefault and upstream are never read
- [Phase 04]: 04-04: normalize_path strips exactly one trailing slash; path_value passes the value whole, so only a// is EmptyComponent
- [Phase 04]: 04-04: the overlap check compares each entry with every earlier entry of the other kind, covers in both directions, and names the earlier line
- [Phase 04]: 04-04: a manifest over 65,536 bytes is TooLarge at the line holding byte index 65536, checked before any line, with empty text
- [Phase 04]: 04-07: a symbolic ref on any integration rung (develop, next, the HEAD target, wsx.baseRef, local rungs) is refused with IntegrationRefSymbolic, never followed or skipped; wsx.baseRef is the way out
- [Phase 04]: 04-07: wsx.baseRef is checked with branch_name before any rung and answered by found or BaseRefMissing, never the chain (D-08)
- [Phase 04]: 04-07: is_branch_name refuses HEAD and a leading -, matching git check-ref-format --branch on the 13-name table (A5)
- [Phase 04]: 04-08: A3 adopted: a workspace-branch candidate is exactly one level under refs/heads/workspace/ or refs/heads/personal/, as git for-each-ref's glob reads it
- [Phase 04]: 04-08: build maps a failed tip-commit or root-tree read to ManifestUnreadable on both the configured and the inferred path; a configured repo still fails closed, now with that error
- [Phase 04]: 04-08: a corrupt candidate ref stays in NotConfigured { inferred: Err(Git) } and require passes it through unwrapped; only manifest failures become NotConfigured
- [Phase 04]: 04-09: root_entry decodes the root tree whole and refuses one out of git's tree order as git::Error::TreeNotSorted before reading any entry; build reports it as ManifestUnreadable
- [Phase 04]: 04-09: the tree-entry mode is checked before any blob read; symlink, directory and gitlink are RootEntry::NotABlob, so link text is never parsed as a manifest
- [Phase 04]: 04-09: wsx.worktreeDir is read through single (D-18) and manifest::normalize_path (D-14), refused as WorktreeDirInvalid; unset is .worktrees (D-20)
- [Phase 04]: 04-09: the CONF-07 hook test runs a real pre-commit hook in a linked worktree instead of a simulated environment
- [Phase 04]: 04-11: the replacement refusal reads only the first ref per base from gix's prefixed listing when git's and gix's replace bases differ; one ref is enough to refuse
- [Phase 04]: 04-11: gix's replace base is read with config_snapshot().string, which takes reduced-trust values gix's own open filters out; that difference over-refuses, the safe side
- [Phase 04]: 04-10: the model's test modules render bytes through crate::git::c_quote, not gix::bstr, so rg -qF 'gix::' src/model exits 1 and every gix use stays in src/git
- [Phase 04]: 04-10: final cargo-mutants 27.1.0 fresh pass over the six core files: 177 mutants, 154 caught, 0 missed, 23 unviable, 0 timeout; no equivalent mutants; the local integration-rung order has its own test (local_develop_beats_local_next)
- [Phase 04]: Any replace ref, of any shape, or a replace base other than refs/replace/ stops the model in a configured repo before any object read; GIT_REPLACE_REF_BASE is refused in Repo::discover before gix opens (04-12, D-22 amended 2026-11-16)
- [Phase 04]: 04-17: GIT_CONFIG in wsx's environment: drop, per the operator on 2026-11-17
- [Phase 04]: 04-13: ReplaceBaseSet's remedy branches on refs::REPLACE_BASE_VAR (pub(super)); a linked worktree's private refs/replace is checked loose-only, its packed-refs not scanned (spike 002 B2b)
- [Phase 04]: 04-14: wsx keys and remotes come from one git config child (read_config); parse_config_z keeps only two-part wsx keys from the own file at scope local, remotes from local/worktree records
- [Phase 04]: 04-15: require_passes_a_git_failure_through keeps its assertion with a rung-holding-a-blob fixture, since D-35 makes a corrupt candidate a skipped ref
- [Phase 04]: 04-15: an error reading wsx.baseRemote or wsx.baseRef lists no rung; build raises it at its turn, so error order is unchanged
- [Phase 04]: 04-16: read_tips keeps the root-tree scan; a missing .wsx.conf with an entry in the tree is ManifestUnreadable (ObjectMissing), none is ManifestMissing
- [Phase 04]: 04-16: build raises read_tips answers in the pre-batch order (batch, rung peel, worktreeDir, tip, manifest); two require tests now expect git's ObjectMissing
- [Phase 04]: 04-13 to 04-17: the model's inputs are read through the git binary (D-23): one git config -z read, one git for-each-ref listing and one git cat-file --batch read fed through Git::run_with_input; model::resolve(repo, git, recorded) spawns 4 git children aimed at wsx's own worktree and 5 aimed at another (D-26, D-31); find_ref, ref_commit, refs_directly_under, root_entry, local_config_values, remote_configured and TreeNotSorted are deleted (D-25)

### Pending Todos

- [2026-10-25] [tooling] Re-check Dependabot after first merge to main — [todo file](.planning/todos/pending/2026-10-25-re-check-dependabot-after-first-merge-to-main.md)
- [2026-10-26] [tests] Replace oracle-observed exit 128 under IF-01 — [todo file](.planning/todos/pending/2026-10-26-replace-oracle-observed-exit-128-under-if-01.md)
- [2026-11-04] [planning] Re-verify Phases 1-3 before milestone audit — [todo file](.planning/todos/pending/2026-11-04-re-verify-phases-1-3-before-milestone-audit.md)

### Blockers/Concerns

- Phase 12: MIG-03 covers the 0.1.0 projects on the owner's work PC, which are not on this machine (checked 2026-10-20); their expected results must be recorded and checked there
- Until Phase 15, this repo's own PR cuts still use `scripts/workspace/cut-pr-branch.sh`, which is untested here (research/DEVFLOW-SETUP.md section 5)
- Phases 5-10: each phase that enables a Rust side must pin wsx's own refusal stderr (`rust_stderr`; the registry rule enforces it) and keep the comparator's checkout/config checks green; no wsx side has run as an enabled scenario yet (Phase 2 verification)
- Host linuxbrew cargo-nextest is 0.9.150 (pin 0.9.148) and shadows ~/.cargo/bin; the pinned tools live in .tools/bin, which scripts/check.sh puts first on PATH when it exists (a bare `cargo nextest` outside check.sh still gets 0.9.150, so a scratch control must prefix PATH itself); until the pin or host tool is reconciled
- Phase 5: enable check::reftable_refused's Rust side with wsx check, give it a rust_stderr naming reftable, and replace its placeholder refusal exit 1 with the IF-01 exit code (scenario added in 03-02; the unit-level refusal is proven by git::repo::tests::reftable_repo_is_refused in 03-06)
- Phase 5: assign exit codes to the refusals added in phase 3 review (RelativeRepoEnv, ObjectStoreEnv incl. GIT_COMMON_DIR and receive-pack quarantine, ImplicitWorkTree, StaleWorktree, GitEmptyOutput); see 03-CONTEXT 'Decided during code review'
- Phases 4 and 8 (03.1 D-04): a user's `<branch>` argument (`wsx new|land|rm <branch>`) is read as exactly `refs/heads/<branch>`, after a branch-name check, never through git's DWIM ladder or a revspec parser, so a branch named like a hex prefix (`abc1234`) or like describe output (`rel-1-gdeadbee`) reads that branch. Since 03.1-04, gix's revspec parsers fail `scripts/check.sh clippy`.
- Phase 8: supply the real recorded set to model::resolve's recorded parameter (D-17); production passes an empty set until then, so CONF-05's "ignoring branches made by wsx new" is proven at the input only, not end to end
- Phase 5: assign IF-01 exit codes to every model::Error variant, and remove the dead_code expectation from src/model/mod.rs along with src/git/mod.rs's
- Phase 6: the guard exits 0 on RepoModel::NotConfigured and fails closed on an Err from model::resolve; it reads the state and never classifies errors (D-16, D-21)
- Phase 12: wsx init and wsx migrate write wsx.baseRef as a bare branch name plus wsx.baseRemote and wsx.pushRemote (D-02, D-08), and write manifest paths through manifest::encode_path, because git prints edge spaces bare and D-19 refuses them bare
- Phase 5: git::Error::ReplaceRefsPresent has no model::Error variant of its own, nor have ReplaceBaseSet and ReplaceRefsUnreadable (D-22, amended 2026-11-16 by the operator: any replace ref refuses): resolve returns each inside Error::Git, and require inside NotConfigured's boxed reason, except that Repo::discover returns ReplaceBaseSet for GIT_REPLACE_REF_BASE before any model exists, so a separate exit code for them needs git::Error::is_replacement_refusal checked inside both, or they take whatever code Git and NotConfigured get
- Phase 6: the guard must decide how a Repo::discover refusal is treated in a repository it cannot classify as configured or not configured, because the refusal comes before any model exists and D-16's exit 0 for a not-configured repository is never reached: RefsNamespace (04-05) refuses in every repository on the machine while gitoxide.core.refsNamespace is in global config or GIT_NAMESPACE is exported, phase 3's ObjectStoreEnv refuses wherever GIT_OBJECT_DIRECTORY, GIT_ALTERNATE_OBJECT_DIRECTORIES or GIT_COMMON_DIR is set, a receive-pack quarantine included, and ReplaceBaseSet (04-12) refuses wherever GIT_REPLACE_REF_BASE is set to anything but refs/replace/
- Phase 5: every wsx command gets its model from model::resolve or require before any other Repo read, and later phases keep that order; resolve runs Repo::refuse_replace_refs (the common dir's refs/replace and packed-refs, and a linked worktree's own refs/replace) before its one object read, the git cat-file --batch child, but a gix Repo read made before resolve (merge_base, ahead_behind, range_walk and tree_diff from phase 3) can return other bytes than git while a replace ref exists, with no error
- Phase 5: assign IF-01 exit codes to the git::Error values phase 4 adds outside model::Error: RefsNamespace, which Repo::discover returns before any model exists, like the phase 3 refusals (ObjectStoreEnv and the rest), and ObjectMissing, GitBadOutput and GitFailed from the model's git children, which reach the user inside model::Error::ManifestUnreadable or Error::Git, so each takes that variant's code unless phase 5 matches inside it
- Every phase that calls git::Repo::merge_base, ahead_behind, range_walk or tree_diff: gix 0.90 ignores info/grafts and the shallow file where git honours them (spike 002 B4 and B4b: git found no merge base where gix found one), so in a grafted or shallow repository those reads can differ from git's with no error; before relying on them, that phase decides whether to refuse such repositories or to read through git (D-32)

### Roadmap Evolution

- Phase 03.1 inserted after Phase 3: Resolve ref names as git does: gix rev_parse_single resolves describe-shaped ref names to another object (URGENT)
- Phase 03.1 edited: edited fields: goal, requirements, success_criteria

## Deferred Items

Items acknowledged and deferred at milestone close, most recent first:

| Category | Item | Status | Deferred At | Milestone |
|----------|------|--------|-------------|-----------|
| *(none)* | | | | |

## Session Continuity

Last session: 2026-11-17T23:37:15.184Z
Stopped at: Completed 04-17-PLAN.md
Resume file: None
