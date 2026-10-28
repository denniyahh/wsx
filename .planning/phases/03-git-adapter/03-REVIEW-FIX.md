---
phase: 03-git-adapter
fixed_at: 2026-10-27T18:38:56Z
review_path: .planning/phases/03-git-adapter/03-REVIEW.md
iteration: 3
findings_in_scope: 3
fixed: 3
skipped: 0
status: all_fixed
---

# Phase 3: Code Review Fix Report (iteration 3)

**Fixed at:** 2026-10-27T18:38:56Z
**Source review:** .planning/phases/03-git-adapter/03-REVIEW.md
**Iteration:** 3

**Summary:**
- Findings in scope: 3 (CR-01, WR-01, WR-02). The scope is critical_warning, so IN-01..IN-03
  are out of scope.
- Fixed: 3
- Skipped: 0

Every fix was test-first:
- Each new test was run before the fix and failed for the reason the finding states. After the
  fix it passed.
- Each new test has a counterpart that must give the opposite result.
- The git-parity tests are pinned against real git 2.56.0. Each one asserts git's own answer
  first, so a git that behaved differently would fail on the git side.

## Fixed Issues

### CR-01: A `GIT_DIR` that gix opens as bare, but git gives the current directory as its worktree, is accepted, including inside a hook

**Files modified:** `src/git/repo.rs`, `src/git/mod.rs`
**Commit:** 62b57e5
**Applied fix:** wsx now matches git's worktree exactly, rather than refusing these cases.

`Repo::discover` calls `apply_implicit_worktree` after the single gix open site. When `GIT_DIR`
is set without `GIT_WORK_TREE`, it decides the worktree the way git does, then sets it through
gix's public in-memory `Repository::set_workdir`. gix 0.90 has that method; no second open is
needed. git's rule, measured with git 2.56.0:

- With `core.bare=true`, git has no worktree.
- With `GIT_IMPLICIT_WORK_TREE` false, git has no worktree. That includes an empty value, which
  git reads as false.
- With `core.worktree` set, git uses it.
- Otherwise git uses the current directory, whatever the git dir is named.
- For a linked worktree's git dir (one with a `commondir` file), git ignores `core.bare` and
  `core.worktree` from the common config. Measured: a bare repository's linked worktree, entered
  from `wt/d`, gets `wt/d`.
- git reads these two keys only from the repository's own config and from `config.worktree`. It
  ignores them from `-c`, `GIT_CONFIG_COUNT` and global config.

What wsx does with each case:

- **Iteration 2's refusal is now parity.** 64bd096 refused a `.git`-named `GIT_DIR` below the
  root. That case now opens with git's worktree, `main/sub`. The test was renamed to
  `an_absolute_git_dir_below_the_worktree_root_opens_the_current_directory_like_git`.
- **A local `core.worktree` is left to gix,** as iteration 2 did. Its tests still pass.
- **Fail-closed sub-cases.** These are refused as `Error::ImplicitWorkTree`, which now carries a
  `reason` instead of `cwd` and `worktree`:
  - a `core.worktree` or `core.bare` set in `config.worktree`. git honours them there; wsx does
    not resolve them.
  - a `core.bare` that is not a boolean;
  - a `GIT_IMPLICIT_WORK_TREE` that is not a boolean. git exits 128 on both of these.
- **Ownership.** git checks no ownership with an explicit `GIT_DIR`. This was measured with
  `GIT_TEST_ASSUME_DIFFERENT_OWNER=1`: discovery refused, `GIT_DIR` opened. So the current
  directory gets no ownership check either. wsx's existing git-dir trust check is unchanged.

**Tests (git oracle, real hooks):**

- Fail before, pass after:
  - `a_separate_git_dir_at_its_worktree_root_opens_the_root_like_git`: git says `r`; wsx said
    `<none>` before.
  - `a_separate_git_dir_below_its_root_opens_the_current_directory_like_git`: git says `r/sub`;
    wsx said `<none>` before.
  - `an_absolute_git_dir_below_the_worktree_root_opens_the_current_directory_like_git`: the
    renamed iteration-2 test. Before, wsx refused.
  - `implicit_work_tree_off_leaves_no_worktree_like_git`: the reverse case. git has no worktree;
    wsx said `main` before.
  - `a_linked_worktree_of_a_bare_repository_opens_the_current_directory_like_git`: before, wsx
    refused.
  - `an_unparseable_implicit_work_tree_is_refused`: git exits 128; wsx opened before.
  - `core_worktree_in_worktree_config_is_refused`: git uses `main`. Before, wsx also refused, but
    under iteration 2's message. The test failed only on the message naming `config.worktree`.
- **The `pre-commit` hook of a separate-git-dir repository, during the partial commit
  `git commit -- ../b.txt` run from `r/sub`:**
  - `a_separate_git_dir_hook_resolves_the_worktree_git_reports`: wsx's worktree equals the
    hook's own `git rev-parse --show-toplevel` (`r`). It was `<none>` before.
  - `a_separate_git_dir_hook_child_keeps_the_partial_commit_index`: this proves the index
    consequence the reviewer had only traced.
    - The hook's own `git diff --cached` sees `b.txt`, the partial commit's index.
    - A `Git::from_process` child aimed at wsx's own worktree must see the same.
    - Before the fix it saw `a.txt`: it lost the hook's `GIT_INDEX_FILE` and read the real index.
    - After the fix it sees `b.txt`.
- **Controls, each giving the opposite result:**
  - `core_bare_keeps_a_separate_git_dir_without_a_worktree_like_git`;
  - `implicit_work_tree_on_keeps_the_current_directory_like_git` (`yes`);
  - `a_separate_git_dir_hook_child_without_an_own_worktree_reads_the_real_index`: a helper with
    no own worktree, in the same hook, sees `a.txt`.
  - The existing hook tests stay green: linked-worktree resolution,
    `own_worktree_child_keeps_hook_index`, the sibling-scrub tests and the main-worktree
    relative index.
- **Mutation check:** treating no git dir as linked makes the bare-linked test fail (`<none>`
  vs `wt/d`). The mutant was restored and not committed.
- **Side effect checked:** a throwaway probe, not committed, showed that `siblings()` does not
  list a separate-git-dir repository as its own sibling. That holds with and without `GIT_DIR`.

**Limits:**

- The relative-path refusal still runs first and judges against gix's worktree. So a relative
  `GIT_DIR` or `GIT_INDEX_FILE` below a `.git`-named root is still refused. That is the
  stricter direction, and its tests are unchanged.
- A `config.worktree` that sets `core.bare` without `extensions.worktreeConfig` is refused if
  gix loads that file anyway. git would ignore it. This was not measured.

### WR-01: gix ignores `GIT_COMMON_DIR`, so git and wsx read different object stores, and only a relative value is refused

**Files modified:** `src/git/repo.rs`, `src/git/mod.rs`
**Commit:** 6594f07
**Applied fix:**

- `GIT_COMMON_DIR` joins `OBJECT_STORE_VARS`. Any value is refused as `Error::ObjectStoreEnv`
  naming it, following the orchestrator's decision. This replaces the reviewer's suggestion,
  which allowed a value matching the repository's own common dir.
- It was removed from `REPO_PATH_VARS`, where it can no longer be reached.
- Measured before relying on it: git exports none of `GIT_COMMON_DIR`, `GIT_OBJECT_DIRECTORY`
  or `GIT_ALTERNATE_OBJECT_DIRECTORIES` to a `pre-commit` hook. This was checked in a main
  worktree, a linked worktree and a separate git dir.

**Tests (git oracle):**

- `git_common_dir_is_refused`, with another repository's `.git` as the value:
  - git's `--git-common-dir` is that directory, and `cat-file` reads its commit.
  - Before the fix, wsx opened and could not read that commit (`unreadable`).
- `git_common_dir_naming_the_repository_own_is_refused`: before the fix, wsx opened.
- Control: `another_repository_commit_is_unreadable_without_alternates`. Without the variable,
  neither git nor wsx reads that commit, and wsx opens. Its doc now names it as the control for
  both variables.

### WR-02: The new install hint is wrong whenever cargo resolves `$CARGO_HOME/bin` first; the old hint was right there

**Files modified:** `scripts/install-tools.sh`, `scripts/check.sh`, `tests/gate_controls.rs`
**Commit:** d43d53c

**Applied fix:**

- `check_version` now takes the function that prints the fix:
  - `verify` and `install` use `plain_fix`, which is unchanged behaviour.
  - `verify-cargo` uses the new `cargo_fix`.
- `cargo_fix` checks the copy in `$WSX_TOOLS_DIR`:
  - **That copy reports the pin:** cargo ran another copy. The message says so, names
    `${CARGO_HOME:-$HOME/.cargo}/bin` (searched before PATH while it is not on PATH,
    cargo#11020), and gives the plain command, which installs there.
  - **Otherwise:** the hint is the `WSX_TOOLS_DIR=<.tools/bin>` command, as before.
- The check.sh comment no longer claims that cargo follows PATH order. It now says that holds
  only while `$CARGO_HOME/bin` is on PATH.
- install-tools.sh uses only bash builtins and `[`, and nothing from outside its header's list.
  `bash -n` and `shellcheck` are clean on both scripts.

**Tests (scratch checkout, controlled CARGO_HOME):**

- `stale_cargo_home_nextest_hint_installs_into_cargo_home` uses the reviewer's fixture:
  - a pinned copy in `.tools/bin`;
  - a stale copy in `$CARGO_HOME/bin`, which is not on PATH.

  stderr must name `$CARGO_HOME/bin` and the plain command, and must not contain
  `WSX_TOOLS_DIR=`. Before the fix it printed the `.tools/bin` command.
- Counterpart: `repo_tools_nextest_wins_when_cargo_home_bin_is_on_path`, the same fixture with
  `$CARGO_HOME/bin` on PATH. The `.tools/bin` copy runs and the run exits 0. This checks the
  corrected comment.
- `stale_repo_tools_nextest_hint_installs_into_repo_tools` stays green: a stale `.tools/bin`
  still gets the `WSX_TOOLS_DIR` command. The other four nextest-pin tests and the coverage
  test also stay green.
- **Following the advice works.** On the fixture, a pinned copy was put into `$CARGO_HOME/bin`
  in place of a download. This is the file the plain command would install. `check.sh test`
  then exited 0 and ran the `.tools/bin` copy.

## Verification

Where it ran: every gate ran on the host, in the orchestrator-pinned phase-3 worktree
(`/var/home/denniyahh/Github/wsx/.worktrees/phase-3`, branch `feature/phase-3`). No separate
review-fix worktree was created.

- **Each commit was gated** with `&&` on the exit codes of:
  - its named tests, one per `scripts/check.sh test -E 'test(=...)'`;
  - `scripts/check.sh fmt`;
  - for WR-01 and WR-02, `clippy`;
  - for WR-02, `bash -n` and `shellcheck`.
- **After the last fix,** `scripts/check.sh all` exited **0**:
  - nextest Summary: `315 tests run: 315 passed, 58 skipped`, then `==> check.sh: all OK`;
  - rustc 1.99.1.
  - The difference from iteration 2's 300 passed and 56 skipped is the 15 new tests and the 2
    new ignored child halves.
- **Scratch:** this run's scratch was under `target/rf3-*` and has been removed. So has the
  throwaway siblings probe. No background processes were started.

What this does not establish:

- `scripts/check-in-container.sh` was not run, as the brief requires.
- The host run used this checkout's `.tools/bin` tools.
- The git-parity claims are pinned to git 2.56.0 and gix 0.90.0. These cases were not tested:
  - a relative `core.worktree`;
  - `core.bare` in `config.worktree` without the extension;
  - `GIT_DIR` naming a `gitdir:` file.
- IN-01, IN-02 and IN-03 were not addressed; they are out of scope. IN-03's list of refusal
  variants that are not recorded for phase 5 is unchanged. The variants are still
  `RelativeRepoEnv`, `ObjectStoreEnv` (now also `GIT_COMMON_DIR`) and `ImplicitWorkTree` (now
  narrower).

---

_Fixed: 2026-10-27T18:38:56Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 3_
