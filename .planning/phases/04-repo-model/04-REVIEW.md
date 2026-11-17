---
phase: 04-repo-model
reviewed: 2026-11-17T03:18:00Z
depth: deep
files_reviewed: 4
files_reviewed_list:
  - src/git/mod.rs
  - src/git/refs.rs
  - src/git/repo.rs
  - src/model/mod.rs
findings:
  critical: 1
  warning: 1
  info: 2
  total: 4
  necessity: 0
status: issues_found
---

# Phase 4: Code Review Report

**Reviewed:** 2026-11-17T03:18:00Z
**Depth:** deep
**Files Reviewed:** 4
**Status:** issues_found

## Summary

The review covered the changes since 7a067d3 (plans 04-09 to 04-12) in the four listed files. It weighed them against 04-12's rule: in a configured repo, refuse before any object read when a replace ref exists or a replace base is set. The refusal's order, its placement in `build` before `ref_commit`, the env check in `discover` before `open_gix`, the `is_replacement_refusal` routing in `build` and `require`, and the messages all hold. The rule is not over-broad.

One store the guard does not check breaks CONF-07's prohibition. In a linked worktree, gix also lists the loose refs in that worktree's own git dir (`.git/worktrees/<id>/refs/...`), not just the ones in the common dir, and git ignores refs there. A replace ref planted in that directory is applied by gix, passes the guard, and wsx resolves a manifest git does not read. The same gix behaviour also lets a private `refs/heads/workspace/*` entry show up as a workspace-branch candidate that git does not see. Both were measured with throwaway probe tests in a scratch `git archive` copy, built in a scratch target dir. The copy has been removed and no repository file was changed.

The guard's own reads were also checked for gaps, and none were found:

- **Other ways to move the stores:** `GIT_COMMON_DIR` and `GIT_NAMESPACE` are refused at `discover`. Reftable repositories are refused too.
- **Config filtering:** `config_snapshot().string` is unfiltered, while gix's open uses `string_filter`. The two agree once `refuse_untrusted` guarantees full trust (gix-0.90.0 `src/config/mod.rs:61-63`).
- **`packed-refs` location:** gix reads `packed-refs` only from the common dir.

Necessity was assessed against `necessity-review.md`. There is a second `refuse_replace_base_env` call inside `refuse_replace_refs`, which is redundant because `discover` (`src/git/repo.rs:34`) is the only `Repo` constructor. It is a deliberate second layer of defence, so under the rubric it is not a necessity finding.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: A replace ref in a linked worktree's own git dir passes the guard, and wsx reads bytes git does not

**File:** `src/git/refs.rs:320-324`, `src/git/refs.rs:347`
**Issue:** `refuse_replace_refs` checks only `<common dir>/refs/replace` and `<common dir>/packed-refs`. Its doc comment (lines 320-324) says "neither reader takes a `refs/replace/` from a linked worktree's own git dir". That is true of git and false of gix:

- **What gix does:** in gix-ref-0.69.1 `src/store/file/overlay_iter.rs`, `iter_prefixed_packed` (391-416) builds a loose iterator over the store's `git_dir` as well as its `common_dir`. `peek_loose` (116-138) filters out worktree-private categories only from the common-dir iterator (`advance_to_non_private`); the git-dir iterator is not filtered at all.
- **Where that reaches the reads:** gix-0.90.0 `src/open/repository.rs:531-549` builds its replacement map from `refs.iter()?.prefixed("refs/replace/")`. So in a linked worktree, a file at `.git/worktrees/<id>/refs/replace/<hex>` goes into gix's replacement map. git ignores it, because for a linked worktree it reads only `refs/worktree/`, `refs/bisect/` and `refs/rewritten/` from the private git dir.

Measured with a probe test in a scratch copy (removed). The fixture was `chain_repo`, configured, with `git worktree add --detach wt` and `<work>/.git/worktrees/wt/refs/replace/<manifest blob>` holding the id of the `only b` blob. From `wt`:

- `git cat-file blob refs/heads/workspace/dev:.wsx.conf` printed `only .planning`.
- `resolve` returned `RepoModel::Configured` with the manifest `only b`.

The control, a replace ref in the common dir read from the same linked worktree, was refused as `ReplaceRefsPresent`. That is exactly what the plan's prohibition rules out ("MUST NOT read manifest bytes other than the ones git reads"). wsx's own feature worktrees are linked worktrees, so this is the normal place it runs.

**Fix:** Also refuse by presence in the worktree's private git dir when it differs from the common dir, and correct the doc comment. Reuse `holds_an_entry`, so a fifo or unreadable entry there is still `ReplaceRefsUnreadable`:
```rust
let replace_dir = self.common_dir().join("refs/replace");
if holds_an_entry(&replace_dir)? {
    return Err(Error::ReplaceRefsPresent { path: replace_dir, packed: false });
}
if self.git_dir() != self.common_dir() {
    // gix lists loose refs under a linked worktree's own git dir for every
    // prefix, while git reads only refs/worktree/, refs/bisect/ and
    // refs/rewritten/ there.
    let private_dir = self.git_dir().join("refs/replace");
    if holds_an_entry(&private_dir)? {
        return Err(Error::ReplaceRefsPresent { path: private_dir, packed: false });
    }
}
```
Pin it with a model test: plant the private ref, assert git prints `only .planning` from the worktree, then assert `resolve` refuses naming `<work>/.git/worktrees/wt/refs/replace`.

## Warnings

### WR-01: Candidate inference in a linked worktree lists branches git does not have

**File:** `src/git/refs.rs:267-291` (`refs_directly_under`), `src/model/mod.rs:393-406` (`candidates`)
**Issue:** The gix iteration behind CR-01 also feeds `refs_directly_under`, which lists names through `platform.prefixed(prefix)`. In a linked worktree, a loose `refs/heads/workspace/<x>` under `.git/worktrees/<id>/` is listed as a candidate, although git does not see it there.

Measured with a probe test in the same scratch copy (removed). The fixture was `chain_repo` with `wsx.sourceBranch` unset, a linked worktree `wt`, and `<work>/.git/worktrees/wt/refs/heads/workspace/ghost` holding the tip id. From `wt`:

- `git for-each-ref 'refs/heads/workspace/*'` listed only `refs/heads/workspace/dev`.
- `resolve` answered `NotConfigured { inferred: Err(WorkspaceCandidates { candidates: [workspace/dev, workspace/ghost] }) }`.

wsx therefore reports two candidates where git has one, and D-21's inference no longer matches git. It was not measured what happens when the private entry is the only candidate, that is, whether `build`'s `find_ref` then finds it or returns `WorkspaceBranchMissing`.

**Fix:** Once the CR-01 check exists, generalise it to a single presence check on the linked worktree's private ref store. When `git_dir != common_dir`, refuse (fail closed, with the path named) any entry under `<git_dir>/refs/` other than `worktree/`, `bisect/` and `rewritten/`. That covers both findings and every later listing through gix. Alternatively, filter `refs_directly_under`'s results to refs whose loose file is not under `git_dir` when it differs from the common dir. Add a test that compares against `git for-each-ref` from the linked worktree with a planted private branch.

## Info

### IN-01: `resolve`'s doc still describes a check that lists replace refs

**File:** `src/model/mod.rs:350-356`
**Issue:** The comment says "a later check that lists replace refs can disagree with that map in that window". After 04-12 the guard lists no refs: it checks for presence in the stores (`refuse_replace_refs`, `src/git/refs.rs:295-337`). The race it describes still exists, but the wording names the removed mechanism.
**Fix:** Reword it to "the replace-ref guard, which reads the ref stores later, can disagree with that map in that window", matching `src/git/refs.rs:333-336`.

### IN-02: The `GIT_REPLACE_REF_BASE` refusal tells the user to remove a config key that may not be set

**File:** `src/git/mod.rs:312-322`
**Issue:** `ReplaceBaseSet` renders one remedy for both of its settings: "Run wsx with GIT_REPLACE_REF_BASE unset, and remove gitoxide.objects.replaceRefBase from the config file that sets it", followed by a `git config --show-origin` line for the key. When `name` is `GIT_REPLACE_REF_BASE`, the refusal comes from the environment, before any config is read (`refuse_replace_base_env`, `src/git/refs.rs:369-377`). The key usually is not set, so the second half of the remedy and its command line point the user at nothing.
**Fix:** Branch on `name` as `ReplaceRefsPresent` branches on `packed`. For the variable, say "Run wsx with GIT_REPLACE_REF_BASE unset". For the key, keep the remove-the-key text and the `--show-origin` command. Then update `replace_base_set_message_names_the_setting_and_the_fix` (`src/git/mod.rs:590-603`).

---

_Reviewed: 2026-11-17T03:18:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
