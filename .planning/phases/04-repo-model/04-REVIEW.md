---
phase: 04-repo-model
reviewed: 2026-11-08T19:15:41Z
depth: deep
files_reviewed: 11
files_reviewed_list:
  - src/git/config.rs
  - src/git/mod.rs
  - src/git/quote.rs
  - src/git/refs.rs
  - src/git/repo.rs
  - src/git/testing.rs
  - src/main.rs
  - src/model/manifest.rs
  - src/model/mod.rs
  - tests/harness/fixture.rs
  - Cargo.toml
findings:
  critical: 1
  warning: 4
  info: 2
  total: 7
  necessity: 0
status: issues_found
---

# Phase 4: Code Review Report

**Reviewed:** 2026-11-08T19:15:41Z
**Depth:** deep
**Files Reviewed:** 11
**Status:** issues_found

## Summary

Reviewed the phase 4 diff (`bdcc0a8~1..HEAD`) of the eleven listed files: the production code in full, and the test code where it bears on how reliable the tests are. Cross-file traces covered `model::resolve` → `build` → `integration` / `ref_commit` / `root_entry` → `refuse_unfollowable_replacements`, error propagation from `git::Error` into `model::Error` and `RepoModel::require`, and the gix 0.90.0 / gix-object 0.66.1 / gix-quote 0.10.1 sources these call.

The main defect: the D-22 replacement guard only looks at replace refs **directly** under the replace base. git applies a nested replace ref (`refs/replace/<dir>/<hex>`), and gix ignores it. So under a nested replace ref, wsx reads other `.wsx.conf` bytes than git, and nothing stops it. A scratch probe against gix 0.90.0 and git 2.56.0 reproduced this, loose and packed. That is the silent mismatch D-22 exists to prevent, and it contradicts the "closed" status of T-04-30 in 04-SECURITY.md. It is not covered by AR-04-03, which accepts only a replace ref that changes during one run.

The other findings: the same guard has a contract violation when the replace base has no trailing slash. Raw control bytes reach messages through `refs.rs`'s `lossy`, which breaks the no-raw-bytes invariant the model's own messages keep. `build` reads the integration branch before the manifest, so a stray candidate branch with no manifest reports "no integration branch" and not D-16's "not configured". A symbolic workspace branch is followed, while a symbolic integration rung is refused.

Not done: the test suite was not run for this review. Every "traced" item comes from reading the code, not from running wsx: no production entry point calls the model yet. The gix side of CR-01 and WR-01 was reproduced through a standalone gix 0.90.0 probe with the same features as `Cargo.toml`. wsx's own guard was not run on it; its result is traced from `refs.rs:283-287` and `refs.rs:349`.

Necessity: the rubric at `~/.claude/gsd-core/references/necessity-review.md` was read. No necessity finding met its evidence rule.

## Critical Issues

### CR-01: Nested replace refs bypass the D-22 guard; wsx and git read different `.wsx.conf` bytes

**File:** `src/git/refs.rs:349-376` (with the one-level filter at `src/git/refs.rs:283-287`)
**Contradicts:** D-22, and the T-04-30 mitigation recorded as closed in 04-SECURITY.md. The doc comments at `src/git/refs.rs:294-296` ("returns the bytes git reads or nothing at all") and `src/git/refs.rs:322-323` ("a differing base whose only refs are nested ... neither reader follows") are false for git.

**Issue:** When the git and gix replace bases are equal, `refuse_unfollowable_replacements` lists the base with `refs_directly_under`. That drops every ref whose remainder contains `/`. The two readers treat such a ref in opposite ways:
- git keys a replace ref on its **last** path component. Its `register_replace_ref` uses `strrchr(refname, '/')`, so it applies `refs/replace/team/<hex>`.
- gix builds its map from `strip_prefix(base)` and then `ObjectId::from_hex` (`gix-0.90.0/src/open/repository.rs:539-545`). `team/<hex>` fails to parse, so gix ignores the ref.

The guard never sees the ref and returns `Ok`. `root_entry` then reads the original blob, while every git command reads the replacement.

Reproduced (git 2.56.0, gix 0.90.0 probe built with this crate's lockfile and features). The repo held `.wsx.conf` blob A = `only secret` and blob B = `only other`:
- `refs/replace/<A>` → B: git `only other`, gix `only other`. This is the control: they agree.
- `refs/replace/sub/<A>` → B: git `only other`, gix `only secret`.
- `refs/replace/team/<A>` → B after `git pack-refs --all`: git `only other`, gix `only secret`.

Failure scenario: a team shares replacements with the usual glob refspec `+refs/replace/*:refs/replace/*`. `*` matches across `/`, so the nested `refs/replace/team/<blob-of-.wsx.conf>` from the remote is fetched as it is. From then on, wsx's personal-path list comes from bytes that `git show workspace/dev:.wsx.conf` and every git child wsx later runs do not see, with no error. A nested ref also hides chains and duplicates from the guard: a direct `refs/replace/X`→Y plus a nested `refs/replace/n/Y`→Z is a chain git follows two hops. A direct `refs/replace/X` plus a nested `refs/replace/n/X` is git's `duplicate replace ref` die.

**Fix:** List the base at any depth and apply git's last-component rule. Refuse any nested ref whose last component is a full-length hex id, because gix never applies it. Build the duplicate and chain keys from the last component across all depths:
```rust
// in refuse_unfollowable_replacements, equal-base branch
let all = self.refs_with_prefix(&git_base)?; // prefixed(), no one-level filter
let mut keyed: BTreeMap<Vec<u8>, &Vec<u8>> = BTreeMap::new();
for full in &all {
    let rest = full.strip_prefix(git_base.as_slice()).unwrap_or(full);
    let last = rest.rsplit(|byte| *byte == b'/').next().unwrap_or(rest);
    if rest.contains(&b'/') && is_full_hex(last, self.repo.object_hash()) {
        // git applies it (strrchr), gix ignores it (from_hex on the whole rest)
        return Err(Error::ReplacementRef { name: full.clone(), target: last.to_vec() });
    }
    if let Some(first) = keyed.insert(last.to_ascii_lowercase(), full) {
        return Err(Error::ReplacementRefDuplicate { first: first.clone(), second: full.clone() });
    }
}
// chain/symbolic loop unchanged, over `all`
```
Add a model test that commits a manifest, plants `refs/replace/sub/<manifest blob>` → another blob, and asserts `resolve` fails closed naming the ref. Pair it with the git differential (`git cat-file -p <blob>` prints the replacement) as the control.

## Warnings

### WR-01: Replace base without a trailing `/` violates `refs_directly_under`'s precondition, so the guard checks nothing

**File:** `src/git/refs.rs:333-349` (precondition stated at `src/git/refs.rs:260-261`)
**Issue:** `git_base` comes straight from `GIT_REPLACE_REF_BASE`, and gix maps that variable onto `gitoxide.objects.replaceRefBase`. With `GIT_REPLACE_REF_BASE=refs/replace`, both bases are `refs/replace`, so the equal-base path runs `refs_directly_under(b"refs/replace")`. That function's doc says the prefix "must end in `/`". Every real ref `refs/replace/<hex>` has the remainder `/<hex>`, which contains `/`, so it is filtered out and the guard returns `Ok`. Reproduced on the git and gix sides with a direct `refs/replace/<A>` → B:
- the probe's `config_snapshot().string(...)` reports `Some("refs/replace")`, and gix applies no replacement (reads A);
- git aborts: `BUG: refs.c:1908: ref pattern must end in a trailing slash when trimming`.

There is no silent byte mismatch here, since git cannot read at all. But wsx resolves a model in an environment where every git read aborts, and the guard's `Ok` rests on a broken precondition, not on a check. The guard's own result is traced, not run.
**Fix:** Enforce the precondition where it is stated, so no caller can pass a bare base:
```rust
if !prefix.ends_with(b"/") {
    return Err(failed("the prefix does not end in /".to_owned()));
}
```
Or refuse in `refuse_unfollowable_replacements` when `git_base` does not end in `/`, naming `GIT_REPLACE_REF_BASE`.

### WR-02: git-layer messages print ref names raw, so a control byte from config reaches the terminal

**File:** `src/git/refs.rs:79-82` (`lossy`), used at `src/git/refs.rs:125, 172, 181, 219, 269, 384`. It reaches users through `src/model/mod.rs:208`.
**Issue:** `lossy` is `String::from_utf8_lossy`, and ESC is valid UTF-8. That breaks the invariant `src/model/mod.rs:182-183` and `src/git/mod.rs:323-324` state: "a control byte from a blob or a config value never reaches the terminal raw". `escape_bytes_in_a_value_render_quoted` pins that invariant only for model variants.

Failure scenario (traced, not run end to end): local config holds `[remote "a<ESC>[2J"] url = …` and `wsx.baseRemote = a<ESC>[2J`. git accepts the section, and `git remote add` with that name exits 3, so `remote_configured` is true. `integration` builds `refs/remotes/a<ESC>[2J/develop`. `find_ref` fails in `FullName::try_from`, and gix's own text escapes the byte. But `what` is `read refs/remotes/a<ESC>[2J/develop` with the byte raw, so the rendered error clears the screen. The same path takes a raw `GIT_REPLACE_REF_BASE` (`refs.rs:269`, `384`).
**Fix:** Use the quoting helper `git/mod.rs` already has:
```rust
// refs.rs
use super::shown;            // c_quote + lossy
let failed = |message: String| Error::Gix { what: format!("read {}", shown(full)), message };
```
Then delete `lossy`. Add a test in `src/git/mod.rs` that renders `Error::Gix` from a `find_ref` on a name holding ESC and asserts no raw `\x1b`.

### WR-03: `build` resolves the integration branch before the manifest, so a stray candidate reports the wrong failure (D-16)

**File:** `src/model/mod.rs:477-501`
**Contradicts:** D-16 ("A missing or invalid manifest there reads as 'not configured, run `wsx init`'").
**Issue:** `build` runs `remote`, `integration` and `worktree_dir` before `ref_commit`/`root_entry`. Take an unrelated repo: no `wsx.sourceBranch`, one stray `personal/foo` with no `.wsx.conf`, and no `develop`, `next` or `origin/HEAD`, which is common for a `main`-only clone. There, `inferred` is `Err(NoIntegrationBranch)`, not `ManifestMissing`. `require` passes it through unchanged, because only the manifest variants are wrapped (`mod.rs:117-128`). The user is told to run `git remote set-head origin --auto` or set `wsx.baseRef`. After doing that, the next run says the repo is not configured and to run `wsx init`. The guard is unaffected, because it reads the variant. Traced from the code order, not run.
**Fix:** In `build`, read the tip and the manifest first, right after the existence check, then the remotes, integration and worktree dir. In configured mode both orders fail closed, so only which error is shown changes. Add a test: one candidate with no manifest and no integration branch; `require()` returns `NotConfigured { reason: ManifestMissing }`.

### WR-04: A symbolic workspace branch is followed, though a symbolic integration rung is refused

**File:** `src/model/mod.rs:474-485`
**Issue:** `build` checks `find_ref(&full)` only for `is_none()`. `ref_commit` then follows up to four symbolic hops. So `refs/heads/workspace/dev` → `refs/heads/main` (or → `refs/remotes/origin/main`) gives a model whose `workspace_branch` is `workspace/dev`. Its `workspace_tip` and manifest come from the target. `integration` refuses exactly this shape, because "following it can land on a local commit" (`mod.rs:550-555`). The inverse risk applies here: a later phase that updates `refs/heads/<workspace_branch>` without `--no-deref` moves the target branch. A candidate listing includes such a ref, dangling ones too (`refs.rs:265-266`). No test covers a symbolic workspace branch. This is a judgment finding: no phase 4 code writes the ref, so the harm is in later phases (unverified).
**Fix:** Use the one `find_ref` read for both the existence check and the tip. This also removes the double read the `resolve` docs describe:
```rust
let workspace_tip = match repo.find_ref(&full)? {
    None => return Err(Error::WorkspaceBranchMissing { branch }),
    Some(RefTarget::Symbolic(target)) => {
        return Err(Error::WorkspaceRefSymbolic { full_ref: full, target })
    }
    Some(RefTarget::Object(id)) => repo.peel_commit(&id).map_err(unreadable)?,
};
```
If following is intended, record it as a decision and add a test that pins it.

## Info

### IN-01: `NotConfigured` around `ManifestMissing` says "run wsx init" twice

**File:** `src/model/mod.rs:292-296`, `src/model/mod.rs:317-321`
**Issue:** The rendered text ends `…Commit one on personal/x, or run wsx init.\nRun wsx init.` The existing test (`not_configured_message_wraps_the_reason_and_says_run_init`) wraps `ManifestInvalid`, so it does not show the duplicate.
**Fix:** Drop ", or run wsx init" from `ManifestMissing`, or skip the trailer when the reason already names it.

### IN-02: `WorkspaceBranchMissing` puts C quotes in the middle of a ref name

**File:** `src/model/mod.rs:275-280`
**Issue:** `refs/heads/{branch}` with `branch = shown(branch)` renders `refs/heads/"caf\303\251"` for a non-ASCII branch. That is neither the ref's name nor git's quoting of it.
**Fix:** Quote the whole ref: `shown(&branch_ref(branch))`.

---

_Reviewed: 2026-11-08T19:15:41Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
