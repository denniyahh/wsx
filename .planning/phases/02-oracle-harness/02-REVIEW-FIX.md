---
phase: 02-oracle-harness
fixed_at: 2026-10-26T21:14:45Z
review_path: .planning/phases/02-oracle-harness/02-REVIEW.md
iteration: 2
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 2: Code Review Fix Report

**Fixed at:** 2026-10-26T21:14:45Z
**Source review:** .planning/phases/02-oracle-harness/02-REVIEW.md
**Iteration:** 2 (this report covers iterations 1 and 2)

**Summary:**
- Iteration 2 findings in scope: 2 warnings. The 8 info findings are out of scope for
  `critical_warning`. IN-07 (two stale docs) was also fixed, because the orchestrator asked for it.
- Fixed: 2 warnings, plus IN-07.
- Skipped: 0.
- Iteration 1: 7 of 7 fixed (table below).

**Where verification ran:** in the main checkout of `feature/phase-2`
(`/var/home/denniyahh/Github/wsx/.worktrees/phase-2`). `workflow.use_worktrees` is `false`, so no
isolated worktree was created. Every commit was gated on `scripts/check.sh all` exiting 0 in that
checkout. The last gate ran 186 tests: 186 passed, 41 skipped (the disabled Rust sides). For both
warnings, the new tests were also run with the fix temporarily reverted. Each negative control
then failed with an assertion, and each positive control still passed. The fix was then restored
byte for byte (`cmp`).

**What this does not establish:**
- Every Rust side is still disabled. The new comparisons and the Rust-side reason rule have run
  only against oracle runs and synthetic cases. No wsx run has exercised them.
- `scripts/check-in-container.sh` was not run. The upstream entries now pinned for cut come from
  the host's git (`branch.autoSetupMerge` defaults to `true`). A different git in the CI image was
  not checked.
- `scripts/check.sh coverage` and `deps` were not run.
- The negative controls change the fixture after the oracle's real run, by hand. They show that
  the comparator sees each kind of change. They do not show that a real defective implementation
  produces exactly that change.

## Iteration 1 findings (carried forward)

| ID | Title | Commit | Status |
|----|-------|--------|--------|
| CR-01 | The observer was blind to HEAD and the index | 9c1f77e | fixed under `FilesExpect::Unchanged`; extended to `Exactly` by iteration-2 WR-01 |
| WR-01 | Refusals and `fixed:` defects pinned only the exit code | b835827 | fixed on the oracle side; Rust side closed by iteration-2 WR-02 |
| WR-02 | `GIT_CEILING_DIRECTORIES` did not cover git run in the scenario root | 4c1e4c0 | fixed |
| WR-03 | Remote repositories, other ref namespaces and git config were unobserved | 18f8dfe | fixed under `RefsExpect::Unchanged`; extended to `Exactly` by iteration-2 WR-01 |
| WR-04 | `CARGO_LLVM_COV_SETUP=no` was claimed but never set | 790a8a2 | fixed |
| WR-05 | `spawn_and_wait` failed on an early exit and could deadlock | c4841dc | fixed: requires human verification (concurrency) |
| WR-06 | `check.sh fmt` depended on git silently | 0d64008 | fixed |

The full iteration-1 report is kept, untracked, as `02-REVIEW-FIX.iter1.md`.

## Fixed Issues (iteration 2)

### WR-01: The `Exactly` expectations missed checkout state, remote refs and config

**Files modified:** `tests/harness/outcome.rs`, `tests/harness/sync.rs`, `tests/harness/worktree.rs`, `tests/harness/cut.rs`, `tests/harness/scenario.rs`, `tests/harness/signing.rs`
**Commit:** a0956e6
**Status:** fixed: requires human verification. The change widens what both `Exactly` forms mean,
and it records the oracle cut's upstream setting as expected wsx behaviour.
**Applied fix:**
- `FilesExpect::Exactly` is now `{ files, checkouts }`. `checkouts` lists every checkout that
  exists after the action, each with the branch its HEAD must be on (`CheckoutExpect::on`).
- Under `FilesExpect::Exactly`, each checkout's index must also hold exactly its HEAD's tree
  (nothing staged, no conflict entries), with no operation in progress. The observer now records
  HEAD's tree per checkout (`ls-tree -r -z --full-tree HEAD`) for this check.
- `RefsExpect::Exactly` is now `{ refs, config_added }`. The local config must be what it was,
  plus exactly the listed entries. `RefsExpect::exactly(refs)` builds the form with an unchanged
  config.
- Both `RefsExpect` forms now require every remote repository's refs to be unchanged.
- `observed()` transcribes checkouts and added config entries. A detached checkout and a removed
  config entry have no expectation form, so they are left out; the comparison still reports them.

**Observed on the oracle:**
- Sync and new-worktree leave every checkout on its branch, with a clean index, no pending
  operation and the config unchanged. The new checkout is on `feature/phase-1`.
- Every oracle cut that creates a PR branch runs `git checkout -b <pr> <remote>/develop`. That
  command writes `branch.<pr>.remote=<remote>` and `branch.<pr>.merge=refs/heads/develop`. The
  oracle's stdout says "set up to track 'origin/develop'". The cut expectations now state those
  two entries. This applies to `cut_outcome` (`origin`), `cut::self_test` (`origin`) and
  `cut::fork_layout` (`upstream`).
- No scenario changed class.
- `no_gsd_custom_personal_path` is Rust-only and uses `cut_outcome`, so it inherits the same
  entries. That case was not observed; it carries the oracle's behaviour by extension.

**Tests:**
- Positive control: `exactly_reports_nothing_for_the_sync_as_run`.
- Negative controls. Each one alters the fixture after the oracle's real sync, and each reports
  exactly one field:
  - `work` left detached → `CheckoutHead`
  - `git read-tree develop` → `CheckoutIndex`
  - a planted `MERGE_HEAD` → `CheckoutPending`
  - an extra worktree → `Files` and `CheckoutSet`
  - an upstream config entry → `Config`
  - a push to an untracked ref on `origin` → `RemoteRefs`
- `files_exactly_reports_a_new_checkout_left_detached`: the new phase checkout, detached, reports
  `CheckoutHead`.
- `refs_exactly_compares_the_config_entries_it_lists`: `cut::self_test` passes as written, and
  fails on `Config` alone once its listed entries are removed.

All 8 negative controls failed with the comparator change reverted.

### WR-02: Rust-side refusals pinned only the exit code, and no rule forced a reason

**Files modified:** `tests/harness/scenario.rs`, `tests/harness/registry.rs`, `tests/harness/outcome.rs`
**Commit:** 6bd5229
**Status:** fixed: requires human verification (new scenario rule and a new check in `judge`)
**Applied fix:**
- `Case` gains `rust_stderr` and a `.rust_stderr(b"..")` builder. `judge` checks it on the Rust
  side, the way it checks `oracle_stderr` on the oracle side. The oracle pin is not applied to the
  Rust side.
- New `validate_entry(entry, scenario)`:
  - It runs `validate`.
  - When the entry's Rust side is enabled (`rust_enabled` in `scenarios!`), it also rejects any
    case that expects a non-zero exit without a non-empty `rust_stderr`. The new rejection is
    `Rejection::RustRefusalWithoutReason`.
  - The runner (`try_run_with_env`) and `registry::every_scenario_is_well_formed` now validate with
    it.
- `Field::Stderr` now reads "the action's stderr".
- The synthetic entries that the scenario tests run on the oracle side only (`local_entry`) had
  claimed an enabled Rust side. The new rule rejected two of them, so they now declare that side
  disabled. No assertion changed.

**Tests:**
- `rejects_enabled_rust_refusal_without_reason`: a synthetic `rust_enabled` refusal is rejected
  with no pin, and again with an empty pin. The runner also rejects it before running.
- `accepts_enabled_rust_refusal_with_reason_and_disabled_without`: this is the positive control.
- `rust_side_is_judged_on_its_own_stderr`: a real oracle refusal, judged as the Rust side, passes
  with its own reason pinned. With another reason pinned, it fails on `Stderr` alone.

Both negative tests failed with the fix reverted.

### IN-07 (requested by the orchestrator): two stale docs

**Files modified:** `tests/oracle/README.md`, `tests/harness/outcome.rs`
**Commit:** 07c67ee
**Applied fix:**
- The README now says the ceiling is "the scenario root's parent, so git never searches above the
  root".
- `Outcome::new_commits` now says "from some ref or detached HEAD".

**Not tested:** this is a documentation-only change, so no test can fail without it.

The orchestrator's note said the README text is hash-pinned. It is not. The provenance tests
check that the README contains the pinned devflow commit and every recorded blob, plus the
README's file mode. Its full text is not pinned: neither the old blob id (`0e5b97e…`) nor the
new one (`55165fd…`) appears in any test, script or workflow. No constant needed updating, and
`readme_records_the_pinned_commit_and_blobs` still passes.

---

_Fixed: 2026-10-26T21:14:45Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_

## Iteration 3 (owner decision follow-up): PR branches are cut with no upstream

**Source:** the owner's decision of 2026-10-26, recorded in PRCUT-06 by 17f6abc. It is not a
review finding. It reverses one part of iteration-2 WR-01 (a0956e6), which had recorded the
oracle's upstream as expected wsx behaviour.
**Files modified:** `tests/harness/cut.rs`, `tests/harness/signing.rs`, `tests/harness/outcome.rs`
**Commit:** e245be0 (`fix(02): expect PR branches cut with no upstream`)
**Status:** fixed: requires human verification. Sixteen cases changed class.

**Applied change:**
- `cut_outcome`, `cut::self_test` and `cut::fork_layout` now expect the local config to be
  unchanged, so the PR branch has no upstream. This also covers the Rust-only
  `no_gsd_custom_personal_path` and both `signing::cut_replay_signed` cases, which inherit
  `cut::self_test`.
- New helpers in `cut.rs`:
  - `tracked(outcome, pr_ref, remote)` adds the two `branch.<pr>.remote` / `.merge` entries to an
    outcome.
  - `upstream_fixed`, `topic_upstream_fixed` and `phase_upstream_fixed` build a `fixed:` case.
    Its record is the expectation plus those entries, and its defect text names only the upstream.
- Each record of the 7 cases that were already `fixed:` is now wrapped in `tracked`. The record
  content is the same as before this change.
- `signing::with_signatures` now applies signatures to a recorded oracle outcome as well as to
  the expected one. Without that, `cut_replay_signed`'s inherited record would still say the
  replayed commit is unsigned.
- Wrong doc corrected: a0956e6 said the oracle runs `git checkout -b <pr> <remote>/develop`. The
  vendored `cut-pr-branch.sh:170` actually runs `git branch -f "$PR_BRANCH" "$BASE_REF"`. That
  command sets the upstream whether or not the branch already exists.

**Observed on the oracle (where verification ran: the main checkout of `feature/phase-2`, no
isolated worktree, `workflow.use_worktrees` false).**

First, every cut expectation and record had its upstream entries removed temporarily, and every
`cut::*` and `signing::cut_replay_signed` oracle case was run. All 23 cases that write a PR branch
failed, each with exactly one difference: the local config. The extra entries were:
- `branch.<pr>.remote=origin` and `branch.<pr>.merge=refs/heads/develop`;
- `remote=upstream` for `fork_layout`'s subject.

After the change, every one of those cases passes. The judge enforces two checks on each `fixed:`
case:
- the run must fail the no-upstream expectation (`FixedPassesOnOracle` otherwise);
- the run must equal the record exactly.

| Case | Before | After |
|------|--------|-------|
| cut::self_test subject | Parity | fixed: (upstream only) |
| cut::quoted_personal_names subject | fixed: | fixed: (record unchanged; defect text now also names the upstream) |
| cut::quoted_personal_names counterpart | Parity | fixed: (upstream only) |
| cut::topic_merge_dropped subject | fixed: | fixed: (record unchanged) |
| cut::topic_merge_dropped counterpart | Parity | fixed: (upstream only) |
| cut::integration_merged_into_feature subject | Parity | fixed: (upstream only) |
| cut::integration_merged_into_feature counterpart | Parity | fixed: (upstream only) |
| cut::merge_resolution_adds_personal_file subject | fixed: | fixed: (record unchanged) |
| cut::merge_resolution_adds_personal_file counterpart | Parity | fixed: (upstream only) |
| cut::merge_resolution_only_personal_file subject | Parity | fixed: (upstream only) |
| cut::merge_resolution_only_personal_file counterpart | fixed: | fixed: (record unchanged) |
| cut::existing_pr_branch_with_extra_commits subject | fixed: | fixed: (record unchanged) |
| cut::existing_pr_branch_with_extra_commits counterpart | Parity | fixed: (upstream only) |
| cut::conflict_leaves_partial_pr_branch subject | fixed: | fixed: (record unchanged) |
| cut::conflict_leaves_partial_pr_branch counterpart | Parity | fixed: (upstream only) |
| cut::scoped_and_unscoped_commits subject | fixed: | fixed: (record unchanged) |
| cut::scoped_and_unscoped_commits counterpart | Parity | fixed: (upstream only) |
| cut::fork_layout subject | Parity | fixed: (upstream only, `upstream/develop`) |
| cut::fork_layout counterpart (refusal) | Parity | Parity |
| cut::recut_after_squash_merge subject | Parity | fixed: (upstream only) |
| cut::recut_after_squash_merge counterpart | Parity | fixed: (upstream only) |
| cut::no_gsd_repo subject | Parity | fixed: (upstream only) |
| cut::no_gsd_repo counterpart (refusal) | Parity | Parity |
| cut::no_gsd_custom_personal_path (both, Rust-only) | no oracle side | no oracle side; expectation now has no upstream |
| signing::cut_replay_signed subject | Parity | fixed: (upstream only, inherited) |
| signing::cut_replay_signed counterpart | Parity | fixed: (upstream only, inherited) |

No case's observed class differed from what the decision predicted.

The count was checked two ways:
- From the run: 23 failures with the upstream stripped, all in the config alone.
- From the code: 14 `*upstream_fixed` call sites in `cut.rs`, plus the 2 inherited signing
  cases, gives 16 newly `fixed:` cases. With the 7 `tracked` records, that makes 23.

**Tests:**
- `outcome::refs_exactly_fails_a_cut_that_writes_an_upstream` replaces
  `refs_exactly_compares_the_config_entries_it_lists`. It is the negative control:
  - The oracle's real `cut::self_test` run, judged as the Rust side, stands for a wsx that writes
    the upstream. It fails on `Field::Config` alone.
  - The positive half: the same expectation with exactly the two entries listed reports nothing.
  - Run against the previous `cut.rs` (upstream expected), the test fails with an assertion
    ("must fail on its config: Ok(())"). The new `cut.rs` was then restored, and `cmp` confirmed
    it byte for byte.
- Gate: `scripts/check.sh all` exited 0 before the commit: 186 tests run, 186 passed, 41 skipped
  (the disabled Rust sides).

**What this does not establish:**
- No wsx cut exists yet; every Rust side is still disabled. Only the oracle's run, judged as the
  Rust side, shows that an upstream-writing cut fails. Nothing shows that wsx's cut writes no
  upstream.
- The upstream comes from the host git's `branch.autoSetupMerge` default (`true`). The hermetic
  environment does not pin that key. A git with a different default, or the CI image, was not
  checked: `scripts/check-in-container.sh` was not run. Under such a git, the 16 newly `fixed:`
  cases would fail on the oracle with `FixedPassesOnOracle`, and the 7 `fixed:` records would
  stop matching.
- The negative control covers one case, `cut::self_test`. The other cases rely on the same
  comparator, which the control exercises, and on their own oracle runs.
- `scripts/check.sh coverage` and `deps` were not run.

_Iteration 3 recorded: 2026-10-26T21:28:48Z_
