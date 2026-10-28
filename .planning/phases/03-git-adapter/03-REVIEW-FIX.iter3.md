---
phase: 03-git-adapter
fixed_at: 2026-10-27T18:08:31Z
review_path: .planning/phases/03-git-adapter/03-REVIEW.md
iteration: 2
findings_in_scope: 5
fixed: 5
skipped: 0
status: all_fixed
---

# Phase 3: Code Review Fix Report (iteration 2)

**Fixed at:** 2026-10-27T18:08:31Z
**Source review:** .planning/phases/03-git-adapter/03-REVIEW.md
**Iteration:** 2

**Summary:**
- Findings in scope: 5 (WR-01..WR-05; fix_scope critical_warning, so IN-01 and IN-02 are out of scope)
- Fixed: 5
- Skipped: 0

Every fix was test-first. Each new test was run alone and failed for the reason the finding
states before the fix, then passed after it. Each has a counterpart that must give the opposite
result. The three git-parity fixes are pinned against real git 2.56.0: every oracle test asserts
git's own result first, so a git that behaved differently would fail on the git side.

## Fixed Issues

### WR-01: The ceiling check compares against the worktree, not the directory where discovery found the repository

**Files modified:** `src/git/repo.rs`
**Commit:** b6102ed
**Applied fix:** `refuse_at_ceiling` now compares the ceilings with `discovery_dir`. That is the
git dir itself when the start is inside it. Otherwise it is the nearest ancestor of the start
whose `.git` leads to the canonical git dir gix found, either as a directory or as a `gitdir:`
file (read with `gix::discover::path::from_gitdir_file`). If no ancestor leads there, discovery
fails closed as `NotARepo`.
**Tests (git oracle):**
- `ceiling_hides_the_repository_when_git_work_tree_points_elsewhere` and
  `ceiling_hides_the_repository_when_core_worktree_points_elsewhere`: git exits 128, and wsx
  now refuses. Before the fix wsx opened (`opened` vs `not_a_repo`).
- `starting_inside_the_git_dir_below_a_ceiling_opens_it`: git opens `r1/.git`, and wsx now
  opens. Before the fix wsx refused.
- Controls:
  - `ceiling_above_the_repository_opens_it_with_git_work_tree_elsewhere`;
  - `ceiling_at_a_linked_worktree_hides_it_like_git` and
    `ceiling_above_a_linked_worktree_lets_wsx_and_git_find_it`;
  - all 8 existing ceiling tests, still green.
- Mutation check: a mutant of `discovery_dir` that ignores `gitdir:` files fails the
  linked-worktree control. The mutant was restored, not committed.

**Limits:** these tests cover the rows the review measured, plus linked worktrees. Two cases
are not tested:
- a nested non-repository `.git` directory between the start and the repository;
- a submodule's `gitdir:` file.

### WR-02: An absolute `GIT_DIR` without `GIT_WORK_TREE` gives git and wsx different worktrees below the root

**Files modified:** `src/git/repo.rs`, `src/git/mod.rs`, `src/git/testing.rs`
**Commit:** 64bd096
**Applied fix:**
- New `refuse_implicit_worktree`, called after `refuse_relative_repo_env`. It refuses when
  `GIT_DIR` is set, `GIT_WORK_TREE` is not, the repository's own config (`Source::Local`) sets
  no `core.worktree`, and the current directory is not the worktree root. That is exact parity
  with git where the result is the same, and fail-closed where git would pick another worktree.
- The refusal is the new typed `Error::ImplicitWorkTree`. It names `GIT_DIR` and gives the fix:
  set `GIT_WORK_TREE`, or run from the root.
- A `core.worktree` set outside the repository's own config does not exempt the repository.
  That is the stricter direction.

**Tests (git oracle):**
- `an_absolute_git_dir_below_the_worktree_root_is_refused`: git's `--show-toplevel` is
  `main/sub`. Before the fix wsx opened (`opened` vs `refused`).
- Controls that open, each with git agreeing on `main`:
  - `an_absolute_git_dir_at_the_worktree_root_opens_like_git`;
  - `an_absolute_git_dir_with_git_work_tree_below_the_root_opens`;
  - `an_absolute_git_dir_with_core_worktree_below_the_root_opens`.
- The existing hook tests stay green, and so does
  `git::tests::error_messages_name_their_cause`, which now covers the new variant.

**Test fixture change:** the decoy children in `testing.rs` now re-execute from the decoy's
worktree root, which is where git runs a hook. Before that change, the full suite showed the
new refusal firing on `a_runner_git_dir_redirects_unscrubbed_discovery`, because that child ran
with a decoy `GIT_DIR` from a directory that is not the decoy's worktree. That is the exact
divergence WR-02 describes. The control still asserts that the decoy redirects unscrubbed
discovery.

### WR-03: gix ignores `GIT_OBJECT_DIRECTORY`, so wsx reads a different object store than git

**Files modified:** `src/git/repo.rs`, `src/git/mod.rs`
**Commit:** 4f3b68c
**Applied fix:** `Repo::discover` refuses `GIT_OBJECT_DIRECTORY` and
`GIT_ALTERNATE_OBJECT_DIRECTORIES` through `refuse_object_store_env`, which runs after the ceiling
check. The refusal is the new typed `Error::ObjectStoreEnv` naming the variable. This includes a
receive-pack quarantine (`pre-receive`/`update` hooks), where wsx is now refused by design.
CONTEXT.md was not edited.
**`GIT_ALTERNATE_OBJECT_DIRECTORIES` measurement:** gix diverges here too.
- With the variable naming another repository's store, `git cat-file -t <other's commit>`
  printed `commit`, exit 0.
- wsx's `resolve_commit` failed (`couldn't parse revision`).
- Without the variable, wsx gave the same failure, so gix 0.90.0 ignores the variable.

Following the decision, it is refused the same way.
**Tests (git oracle):**
- `git_object_directory_is_refused`: git cannot read `HEAD`. Before the fix wsx opened and
  read `HEAD`.
- `alternate_object_directories_are_refused`: git reads the other repository's commit. Before
  the fix wsx could not read it.
- Controls:
  - `the_repository_object_store_reads_like_git`: wsx's `HEAD` equals git's.
  - `another_repository_commit_is_unreadable_without_alternates`: both git and wsx fail to
    read the commit, and wsx opens rather than refusing.

### WR-04: The container gate runs the host's `.tools/bin` binaries ahead of the tools it just installed

**Files modified:** `scripts/check-in-container.sh`, `tests/gate_controls.rs`
**Commit:** 9589841
**Applied fix:** when `$REPO_ROOT/.tools` exists, `docker run` gets
`--tmpfs /workspace/.tools:ro`, an empty read-only mask. Otherwise it gets nothing, because
Docker cannot create a mount point inside the read-only bind.
**Tests (stub docker):**
- `container_gate_masks_a_checkout_tools_dir`: the mask appears as two consecutive arguments.
  It failed before the fix.
- `container_gate_adds_no_tools_mask_without_a_tools_dir`: no `/workspace/.tools` mount
  without the directory.
- The existing `container_gate_mounts_its_own_checkout_not_the_callers` was refactored onto
  the shared fixture and stays green.
- `bash -n` and `shellcheck` are clean.

**Limits:** the tests only assert the argument list given to a stub docker. Whether a real
daemon accepts a tmpfs mounted over a directory inside a read-only bind, and whether check.sh
in the container then sees no `.tools/bin`, has not been run, because the brief excludes
`check-in-container.sh`.

### WR-05: A stale `.tools/bin` gets a fix suggestion that cannot fix it

**Files modified:** `scripts/check.sh`, `scripts/install-tools.sh`, `tests/gate_controls.rs`
**Commit:** 54c0576
**Applied fix:**
- When check.sh prepends `.tools/bin` to PATH, it now exports `WSX_TOOLS_DIR="$PWD/.tools/bin"`
  too.
- install-tools.sh has a new `install_hint`. When `WSX_TOOLS_DIR` is set, its three install
  messages print `WSX_TOOLS_DIR=<dir> scripts/install-tools.sh install <tool>`, with `<dir>`
  quoted by bash's `printf %q`. Checked by hand: `/a dir/x` prints as `/a\ dir/x`.
  install-tools.sh still uses only the tools its header lists, since `printf` is a bash builtin.
- IN-02's comment wording in check.sh was corrected in the same commit: the version checks
  cover only the three pinned tools.

**Tests (scratch checkout, empty CARGO_HOME):**
- `stale_repo_tools_nextest_hint_installs_into_repo_tools`: stderr must name the `.tools/bin`
  install command. It failed before the fix.
- `shadowing_nextest_hint_names_no_tools_dir_without_one`: without `.tools/bin`, the hint is
  the plain command and names no `.tools`.
- `checkout_check_sh_test` now removes an inherited `WSX_TOOLS_DIR`. Without that, an outer
  check.sh run from a checkout that has `.tools/bin`, like this one, would leak its own
  directory into the scratch hints.
- `bash -n` and `shellcheck` are clean on both scripts.

## Verification

Every gate ran on the host, in the main checkout of the phase-3 worktree
(`/var/home/denniyahh/Github/wsx/.worktrees/phase-3`, branch `feature/phase-3`). No isolated
review-fix worktree was created: the orchestrator pinned this tree.

- Before each commit, the fix's named tests ran one per command, chained with `&&`, with
  `scripts/check.sh fmt` and, for WR-04/05, `clippy`. Each commit was gated on those exit codes.
- After the last fix, `scripts/check.sh all` exited **0**, with nextest Summary
  `300 tests run: 300 passed, 56 skipped` and `==> check.sh: all OK`.

What this does not establish:

- `scripts/check-in-container.sh` was not run, as the brief requires. The orchestrator runs it.
- The host run used this checkout's `.tools/bin` tools.
- IN-01 (the detached maintenance writer in the counterpart test) was not addressed. It is out
  of scope.

---

_Fixed: 2026-10-27T18:08:31Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 2_
