---
phase: 03-git-adapter
fixed_at: 2026-10-27T08:30:20Z
review_path: .planning/phases/03-git-adapter/03-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 3: Code Review Fix Report

**Fixed at:** 2026-10-27T08:30:20Z
**Source review:** .planning/phases/03-git-adapter/03-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 6 (2 critical, 4 warning; the 5 info findings are out of scope)
- Fixed: 6
- Skipped: 0

**Where verification ran:** in the phase-3 checkout itself
(`/var/home/denniyahh/Github/wsx/.worktrees/phase-3`, branch `feature/phase-3`).
`workflow.use_worktrees` is `false`, so no isolated review-fix worktree was created. Each
commit was gated on `scripts/check.sh all` exiting 0, with the pinned cargo-nextest from
`target/gsd-tools` on PATH. The baseline gate before any change also exited 0. Each new test
was also run alone with `--no-tests=fail -E 'test(=...)'`, before the fix (failing where it
reproduces the finding) and after it (passing).

## Fixed Issues

### CR-01: `siblings()` opens a foreign-owned sibling worktree with reduced trust, without refusing it

**Status:** fixed: requires human verification (carries a recorded divergence, below)
**Files modified:** `src/git/repo.rs`
**Commit:** 18e8ff8
**Applied fix:** `siblings()` now passes every non-self sibling through
`refuse_untrusted_sibling`. That function applies `discover`'s `refuse_untrusted` (gix's
trust verdict). For a bare sibling, which has no worktree and so gets no check from gix, it
also requires `Trust::from_path_ownership(git_dir)` to be `Full`.
**Tests (each with git as the oracle):**
- `a_foreign_owned_sibling_worktree_is_refused`: wt1 is really chowned with `ForeignOwned`,
  and git refuses it. Before the fix wsx returned it (`result=opened`); now `ForeignOwner`
  names wt1.
- `a_foreign_sibling_worktree_in_safe_directory_opens` (control): wt1 is listed in global
  `safe.directory`; git and wsx both open it. Passes before and after.
- `a_foreign_owned_bare_main_repository_is_refused_as_a_sibling`: a foreign bare `r.git`,
  reached from its linked worktree that the user trusts through `safe.directory`. git opens
  the worktree and refuses `r.git`. Before the fix wsx opened `r.git`; now it refuses.
- `an_owned_bare_main_repository_opens_as_a_sibling` (control).
- `safe_directory_does_not_trust_a_foreign_bare_main_sibling`: a recorded divergence. With
  `r.git` itself listed in `safe.directory`, git opens it and wsx still refuses. gix applies
  `safe.directory` to a sibling only through its worktree, and the option-cloning call that
  could re-check it (`without_repository_environment_overrides`) is `pub(crate)` in gix
  0.90.0. This is the stricter direction. The refusal message still prints git's
  `safe.directory` fix, which does not help wsx in this one case.

### CR-02: `range_walk` silently ignores an annotated tag in `hidden`, and tags as tips or merge-base inputs fail where git answers

**Status:** fixed
**Files modified:** `src/git/repo.rs`, `src/git/reads.rs`
**Commit:** 7b97a03
**Applied fix:** added `Repo::resolve_commit`, which resolves a revision and then peels it to a
commit. `merge_base` and `resolve_all` (so `range_walk` and `ahead_behind`) now use it.
`tree_of` keeps its own `peel_to_tree`.
**Tests:** `range_walk_hides_an_annotated_tag_like_git` checks that `v1` really is a tag
object. Its control asserts that git's `rev-list feat --not v1` differs from
`rev-list feat`, so a hidden tag that is silently ignored cannot pass. Before the fix gix
returned 6 commits against git's 3. `range_walk_from_an_annotated_tag_matches_git` failed
before with "Expected object of kind commit but got tag".
`merge_base_and_ahead_behind_peel_an_annotated_tag_like_git` failed before with "Commit ...
could not be found for merge-base traversal". All three now equal git.

### WR-01: `tree_diff` turns the reftable refusal into a generic `Error::Gix`

**Status:** fixed
**Files modified:** `src/git/reads.rs`, `src/git/repo.rs` (tests)
**Commit:** 192b3f9
**Applied fix:** `tree_of` now returns `Result<_, Error>`. It passes `resolve` errors through
unchanged and maps only `find_object`/`peel_to_tree` failures to `Error::Gix` ("tree of
<rev>"). `tree_diff` uses `?`.
**Tests:** `tree_diff_keeps_the_reftable_refusal` runs `tree_diff(HEAD, HEAD)` on the reftable
twin opened through `open_unchecked`. It failed before with the refusal text wrapped in
"diff HEAD HEAD: ..." and now gets `Error::Reftable`. Control:
`files_repo_diffs_head_through_the_same_path` gives an empty diff on the files twin.

### WR-02: The `GIT_CEILING_DIRECTORIES` check is skipped for bare repositories

**Status:** fixed
**Files modified:** `src/git/repo.rs`
**Commit:** 1eed531
**Applied fix:** `refuse_at_ceiling` compares the ceiling entries with the directory that
discovery found: the worktree, or the git dir of a bare repository (`refusal_path`). It no
longer returns early when there is no worktree. The docstring records the bare case.
**Tests:** `ceiling_at_a_bare_repository_hides_it_like_git` starts in `r.git/objects` with the
ceiling at `r.git`. git exits 128. Before the fix wsx opened it (reproduced; the review had
inferred this without running it). Controls: `ceiling_above_a_bare_repository_lets_wsx_and_git_find_it`
and `starting_in_a_bare_repository_that_is_the_ceiling_opens_it`. The existing non-bare
ceiling tests still pass.

### WR-03: Own-worktree children keep relative `GIT_DIR`/`GIT_INDEX_FILE`/`GIT_WORK_TREE` but run in a different directory

**Status:** fixed: requires human verification (the fix departs from the review's first suggestion, on measured evidence)
**Files modified:** `src/git/repo.rs`, `src/git/mod.rs`
**Commit:** 650e9a1
**What was measured (git 2.56.0, from `main/sub`):**
- With a relative `GIT_INDEX_FILE=alt.idx`, git reads `main/alt.idx`: it resolves the path
  against the worktree root, not the cwd. gix (wsx's `index_path`) resolves
  `main/sub/alt.idx`.
- With a relative `GIT_DIR=../.git` and no `GIT_WORK_TREE`, git takes `main/sub` as the
  worktree, and gix takes `main`. A spawn-helper child running at `main` then fails with
  "not a git repository: '../.git'".

The review's first suggestion was to rewrite these variables to gix's absolute paths. That
would have carried gix's cwd-relative reading into git's writes, so wsx would have
disagreed with git everywhere. I applied the review's alternative instead: refuse.
**Applied fix:** a new `Error::RelativeRepoEnv { name, value, cwd, worktree }`, with a message
test. `Repo::discover` refuses when `GIT_DIR`, `GIT_WORK_TREE`, `GIT_INDEX_FILE` or
`GIT_COMMON_DIR` holds a relative value and gix's current directory is not the worktree root.
git runs hooks at the worktree root, so a hook's relative `GIT_INDEX_FILE` still applies:
`hook_resolves_main_worktree_with_relative_index` and the spawn hook tests still pass.
**Tests:** `a_relative_index_below_the_worktree_root_is_refused` and
`a_relative_git_dir_below_the_worktree_root_is_refused` both pin git's answer first. Before
the fix both opened (index `main/sub/alt.idx` while wsx's own git child read `b.txt`; and
"not a git repository"). Controls: `a_relative_index_at_the_worktree_root_opens_like_git`
(wsx's index and its git child both use `main/alt.idx`, the same as git) and
`an_absolute_index_below_the_worktree_root_opens`.

### WR-04: One stale (prunable) linked worktree makes `siblings()` fail for the whole repository

**Status:** fixed: requires human verification (a policy choice)
**Files modified:** `src/git/repo.rs`, `src/git/mod.rs`
**Commit:** 6ba45b5
**Applied fix:** I chose the review's fail-closed option. `siblings()` still fails, but now
with a new `Error::StaleWorktree { worktree, git_dir }` that names the missing worktree and
its git dir and suggests `git worktree prune` "if it is gone for good". It is raised before
canonicalisation, from a `NotFound` on the worktree directory. The other option, skipping
prunable siblings and reporting them the way `git worktree list` does, would change what
phase 5's callers receive. That is left for an owner decision.
**Tests:** `a_stale_sibling_worktree_is_named_with_its_fix` deletes wt1 and first asserts that
`git worktree list --porcelain` marks it `prunable`. Before the fix the error was
"canonicalize .../wt1: No such file or directory"; it now names wt1 and the prune. Control:
`siblings_open_every_present_linked_worktree`.

## Observed during fixing, not fixed (outside the in-scope findings)

- **A tree as a walk tip.** `git rev-list main^{tree}` exits 0 and prints nothing, but
  `range_walk` returns `Error::Gix` (gix did this before CR-02 too, and still does). This
  fails closed. Measured.
- **A linked sibling whose private git dir (`.git/worktrees/<name>`) is foreign-owned while
  its worktree is owned.** git checks the git dir's ownership too; `siblings()` does not.
  Inferred from git's `ensure_valid_ownership` and gix's `open_from_paths`. Not reproduced.
- **Starting inside a non-bare `.git` directory with the ceiling at the worktree.**
  `refuse_at_ceiling` would refuse there, and git probably finds the repository. This
  behaviour existed before WR-02 and WR-02 does not change it. Not reproduced.

---

_Fixed: 2026-10-27T08:30:20Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
