---
phase: 03-git-adapter
reviewed: 2026-10-27T03:30:30Z
depth: deep
files_reviewed: 10
files_reviewed_list:
  - Cargo.toml
  - clippy.toml
  - deny.toml
  - src/git/mod.rs
  - src/git/reads.rs
  - src/git/repo.rs
  - src/git/spawn.rs
  - src/git/testing.rs
  - src/main.rs
  - tests/harness/check.rs
findings:
  critical: 2
  warning: 4
  info: 5
  total: 11
status: issues_found
---

# Phase 3: Code Review Report

**Reviewed:** 2026-10-27T03:30:30Z
**Depth:** deep
**Files Reviewed:** 10
**Status:** issues_found

## Summary

I reviewed the git layer (`src/git/`) across `repo.rs`, `spawn.rs`, `reads.rs` and `testing.rs`,
plus the config, deny and harness changes. I traced the call chains into gix 0.90.0 and
gix-diff 0.70.0 source in the cargo registry.

The `Repo::discover` entry point and the spawn helper's env removal are sound for the cases the
tests exercise. Two fail-open paths sit outside what the tests reach. I reproduced both with a
scratch gix 0.90.0 probe under `target/` (since removed), each with a negative control:

1. `Repo::siblings()` opens a sibling worktree owned by another user with gix `Trust::Reduced`
   and returns it with no refusal. git refuses the same worktree with "detected dubious
   ownership". The refusal applies only to the repository `discover` opened, never to siblings.
2. `Repo::range_walk` does not peel revisions. An annotated tag given as a hidden revision is
   silently ignored, so the walk returns commits git excludes. An annotated tag given as a tip
   or to `merge_base` is an error, where git peels it and answers.

Neither has a caller until phase 5, which is the only reason they are not live defects today.

What this review does not establish:
- I did not run the phase's test suite.
- The bare-repository ceiling gap (WR-02) and the relative-environment divergence (WR-03) come
  from reading the code and git/gix semantics. I did not reproduce them.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: `siblings()` opens a foreign-owned sibling worktree with reduced trust, without refusing it

**File:** `src/git/repo.rs:111-137` (no trust check on the siblings it opens; compare `refuse_untrusted` at `:258-266`, applied only in `discover`)

**Issue:** `siblings()` opens each sibling through `main_repo()` and
`Proxy::into_repo_with_possibly_inaccessible_worktree()`. Both clone the parent's options, and
those carry `git_dir_trust = Some(Full)` because the parent passed `refuse_untrusted`. Each
sibling then goes into `open_from_paths`. There, a worktree not owned by the current user is
checked against `safe.directory`. With `bail_if_untrusted: false`, which `default_for_level`
sets, a failed check silently lowers trust to `Reduced` (gix-0.90.0
`src/open/repository.rs:397-423`). `siblings()` never reads `git_dir_trust()`.

The result is that a foreign-owned sibling comes back as an ordinary `Repo`, and gix has
silently dropped its config. That is exactly the case CONTEXT `<specifics>` rules out: "a
foreign-owned repo must never open with dropped config. gix silently drops config in a repo with
reduced trust; git refuses."

Also, `main_repo()` passes the pre-set `Full` trust to `open_opts`. That skips the git-dir
ownership probe (`options.git_dir_trust.is_none()` at `open/repository.rs:110-115`). A bare main
repository owned by another user is therefore never downgraded at all.

Reproduced with gix 0.90.0, using the same trust map and discover call as `open_gix`. Fixture:
`main`, plus linked worktrees `wt1` and `wt2`; `wt2` was chowned to subordinate uid 524288 with
`unshare --map-root-user --map-users=auto`, and the probe was opened from `wt1`.
- Control, everything owned by the test user: both siblings are `trust Full`.
- With `wt2` foreign: `sibling .../sib/wt2 owner uid 524288 trust Reduced`. The open succeeded.
- `git -C wt2 status` in the same state fails with `fatal: detected dubious ownership`.

No test covers a foreign-owned sibling. The foreign-owner tests only discover the foreign
repository directly.

**Fix:** Apply the same refusal to every repository `siblings()` opens, before wrapping it. Add a
test with a `ForeignOwned` linked worktree, plus a control that lists it in global
`safe.directory`.

```rust
for repo in opened {
    refuse_untrusted(&repo)?;          // ForeignOwner naming the sibling, as git would
    let sibling = Repo::from_gix(repo)?;
    // ...
}
```

`refuse_untrusted` alone does not cover the bare-main case. There, the git-dir ownership of the
main repository should be checked explicitly, for example with
`gix::sec::Trust::from_path_ownership(common_dir)` when `main.workdir()` is `None`.

### CR-02: `range_walk` silently ignores an annotated tag in `hidden`, and tags as tips or merge-base inputs fail where git answers

**File:** `src/git/repo.rs:151-160` (`resolve` returns the unpeeled object), `src/git/reads.rs:80-105` (`range_walk`), `src/git/reads.rs:49-64` (`merge_base`), `src/git/reads.rs:69-74` (`ahead_behind`)

**Issue:** `resolve` returns whatever `rev_parse_single` names, without peeling. For an annotated
tag, that is the tag object. The commit queries then pass that id straight to gix, and git peels
it in the same place. Reproduced with gix 0.90.0 on a two-commit repo with `v1` an annotated tag
on the first commit:

| Query | git | gix with the unpeeled tag id |
|---|---|---|
| `rev-list HEAD --not v1` | 1 commit | `rev_walk([HEAD]).with_hidden([tag])` returns **both** commits, `Ok` (silently wrong) |
| same, hidden = `v1^{commit}` (control) | 1 commit | 1 commit |
| `rev-list v1` | 1 commit | `Err("Expected object of kind commit but got tag")` |
| `merge-base v1 HEAD` | the first commit | `Err("Commit <tag-id> could not be found for merge-base traversal")` |

The hidden-side case is a silent wrong answer: `range_walk(&[b"main"], &[b"v1.0"])` returns
history that `v1.0` already contains. Phase 9's replay ("commits on the workspace not on the
base") is the kind of caller that depends on exactly this set. `ahead_behind` fails closed only
by accident, because its second walk puts the tag on the tip side and errors. The `merge_base`
error message also says the commit "could not be found", which misleads.

**Fix:** Peel to a commit for every commit query. Keep `tree_of`'s own `peel_to_tree`. Add
differential tests with an annotated tag as a tip, as a hidden revision, and as a merge-base
input.

```rust
/// The commit a revspec names, peeling tags as git's commit-taking commands do.
pub(super) fn resolve_commit(&self, rev: &[u8]) -> Result<gix::ObjectId, Error> {
    let id = self.resolve(rev)?;
    self.repo
        .find_object(id)
        .and_then(|object| object.peel_to_commit())   // map errors to Error::Gix naming `rev`
        .map(|commit| commit.id)
        .map_err(/* ... */)
}
```

Use `resolve_commit` in `merge_base` and `resolve_all`.

## Warnings

### WR-01: `tree_diff` turns the reftable refusal into a generic `Error::Gix`

**File:** `src/git/reads.rs:122-123`, `src/git/reads.rs:155-160`

**Issue:** `tree_of` maps every `resolve` error to `String` (`.map_err(|err| err.to_string())`),
and `tree_diff` wraps that string in `Error::Gix { what: "diff ..", .. }`. The `resolve`
docstring promises that a backend decode failure is "the reftable refusal, never a generic read
error". `merge_base` and `range_walk` keep that promise by propagating with `?`; `tree_diff`
does not. When the backstop fires (for example, a repo where `refuse_ref_storage` was bypassed
through a gix/git difference in config parsing), the typed `Error::Reftable` is lost. Phase 5
then maps it to the wrong IF-01 exit code. `NotARepo`, `Io` and the other variants are flattened
the same way.

**Fix:** Make `tree_of` return `Result<gix::ObjectId, Error>`. Propagate `resolve` errors
unchanged, and map only the `find_object`/`peel_to_tree` failures to `Error::Gix`:

```rust
fn tree_of(&self, rev: &[u8]) -> Result<gix::ObjectId, Error> {
    let id = self.resolve(rev)?;
    let gix_err = |err: &dyn std::fmt::Display| Error::Gix { what: format!("tree of {}", String::from_utf8_lossy(rev)), message: err.to_string() };
    let object = self.repo.find_object(id).map_err(|e| gix_err(&e))?;
    Ok(object.peel_to_tree().map_err(|e| gix_err(&e))?.id)
}
```

Add a test: on the reftable twin opened through `open_unchecked`, `tree_diff(b"HEAD", b"HEAD")`
is `Error::Reftable`.

### WR-02: The `GIT_CEILING_DIRECTORIES` check is skipped for bare repositories, so wsx opens a repository git would not find

**File:** `src/git/repo.rs:212-221`

**Issue:** `refuse_at_ceiling` returns `Ok(())` when `repo.workdir()` is `None`. gix's
divergence (it searches the ceiling directory itself) is not specific to worktrees. Starting
inside `r.git/objects` with ceiling `r.git`, gix would find the bare `r.git` and git would not.
CONTEXT ("Decided after research") says the check "refuses as not-a-repo where git would", with
no bare exception. The docstring records the gap, but that does not make it consistent with the
locked decision. Not reproduced; inferred from the measured table in the docstring and from the
code path.

**Fix:** Compare the ceiling entries against the directory discovery actually found: the
worktree for a non-bare repo, otherwise the canonical `git_dir`. Refuse when that equals a
ceiling entry and the start is not that directory. Add a bare-repo case with a git-side oracle
and a control.

### WR-03: Own-worktree children keep relative `GIT_DIR`/`GIT_INDEX_FILE`/`GIT_WORK_TREE` but run in a different directory than the one gix resolved them from

**File:** `src/git/spawn.rs:155-161` (`from_process` snapshot), `src/git/spawn.rs:234-245`, `src/git/spawn.rs:92-105` (`current_dir(worktree)`)

**Issue:** `Repo::discover` resolves relative `GIT_DIR` and `GIT_INDEX_FILE` against the process
cwd (`repo.rs:290-292`; gix anchors at the cwd when opening). The spawn helper gives an
own-worktree child the raw snapshot unchanged, but runs it with `current_dir` set to the
worktree root. If wsx runs from a subdirectory with a relative value (for example,
`cd sub && GIT_DIR=../.git wsx ...`), its gix reads use one repository and index while its git
writes resolve another. In hooks, git runs at the worktree root, so the two agree. That is why
the hook tests pass, and also why they cannot catch this. Not reproduced.

**Fix:** In `from_process`, rewrite the repository-path variables in the snapshot to absolute
values: `GIT_DIR` and `GIT_COMMON_DIR` as `repo.git_dir()`/`common_dir()`, `GIT_INDEX_FILE` as
`repo.index_path()`, and `GIT_WORK_TREE` as the worktree. Rewrite each only when it was set.
Alternatively, refuse a relative value when the process cwd is not the worktree root. Add a
re-exec test from a subdirectory with a relative `GIT_INDEX_FILE`.

### WR-04: One stale (prunable) linked worktree makes `siblings()` fail for the whole repository

**File:** `src/git/repo.rs:113-122`

**Issue:** `into_repo_with_possibly_inaccessible_worktree()` deliberately opens a worktree whose
directory is gone. `Repo::from_gix` then calls `canonical(workdir)` and fails with `Error::Io`,
and `?` aborts the whole list. A worktree directory deleted by hand, without
`git worktree prune`, is common. `git worktree list` shows it as `prunable` and carries on. Once
phase 5+ calls `siblings()`, every wsx command in such a repository fails, and the error names a
`canonicalize` call rather than the stale worktree.

**Fix:** Decide the policy explicitly:
- Skip missing-directory siblings and report them as prunable through a wsx type, the way
  `git worktree list` does.
- Or keep failing closed, but with a dedicated error naming the worktree and suggesting
  `git worktree prune`.

Either way, add a test that deletes `wt2` and asserts the chosen behaviour.

## Info

### IN-01: Any non-empty `--local-env-vars` answer is accepted as the removal list

**File:** `src/git/spawn.rs:211-222`; enshrined by `one_name_list_query_lets_the_command_run` at `src/git/spawn.rs:1068-1079`

**Issue:** The fail-closed guard rejects only an empty list. A broken or substituted `git` that
prints only `GIT_DIR` passes, and `GIT_INDEX_FILE` then reaches a sibling's child. Test support
already demands `GIT_DIR` on the list (`testing.rs:640`); production is weaker than its own test
harness.

**Fix:** Require at least `GIT_DIR`, `GIT_INDEX_FILE`, `GIT_WORK_TREE` and `GIT_COMMON_DIR` on
the list. Otherwise return a refusal naming what is missing. Invert the one-name test into a
refusal test.

### IN-02: `run` spawns `git --version` before it canonicalises the target

**File:** `src/git/spawn.rs:285-286`

**Issue:** If the target directory is missing, the first error is
`spawn git --version in <dir>: No such file or directory`, from the version check. The message
blames the version probe rather than the bad target.

**Fix:** Canonicalise `worktree` (the first line of `command`) before `ensure_version`, or have
the version check run in a directory that exists.

### IN-03: The clippy ban list's "every public open/discover entry point" claim omits `gix::init*`, and the sanctioned sibling paths bypass the refusals

**File:** `clippy.toml:27-35`

**Issue:** `gix::init`, `gix::init_bare` and the `ThreadSafeRepository::init*` functions return
an open repository at an arbitrary path, and they are not banned. `worktrees_including_main()`
and `Proxy::into_repo()` are allowed, and they reach the same unchecked sibling path as CR-01.

**Fix:** Ban the `init*` functions, or narrow the comment's claim. Note in the reasons that the
sibling paths need the `siblings()` refusal from CR-01.

### IN-04: The stderr of a successful git child is captured and discarded

**File:** `src/git/spawn.rs:74-88`

**Issue:** `Command::output()` captures both streams. On exit 0, only stdout is returned. Hook
output, `git commit` warnings and signing notices from the user's own hooks never reach the
user. This is a visible change from "a git command the user typed" (module doc, `mod.rs:9-11`).

**Fix:** Inherit stderr for write commands, or return it alongside stdout so the caller can
relay it. Record the choice before phase 9's replay depends on it.

### IN-05: In-process discovering tests scrub only the `--local-env-vars` variables

**File:** `src/git/testing.rs:662-699`

**Issue:** Tests that call `rerun_scrubbed()` run `Repo::discover` in the test process. That
process still has the runner's `HOME`, `XDG_CONFIG_HOME`, `GIT_CONFIG_GLOBAL`,
`GIT_CEILING_DIRECTORIES` and `SUDO_UID`, which gix reads. Unlike the re-executed children, the
in-process tests are not hermetic for global config or ceilings. A developer config (for example
`safe.directory = *`, or a ceiling above `target/`) can change their outcome.

**Fix:** Have `rerun_scrubbed` re-execute whenever the process environment differs from the
scratch environment's keys, or re-execute these tests with `scratch.env` as the hook tests do.

---

_Reviewed: 2026-10-27T03:30:30Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
