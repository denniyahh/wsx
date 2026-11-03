## CONFIRMED

### 1. Single commit replacement breaks the planned `workspace_tip` test assertion
* **Severity**: High
* **File:Line**: `.planning/phases/04-repo-model/04-11-PLAN.md:113` (also `04-11-PLAN.md:22`)
* **Quoted text**:
  > "`single_tree_replacement_reads_as_git_does`; `single_commit_replacement_reads_as_git_does`: `T` or `C` replaced by a side tree or commit: `Configured` with the list git prints, and `workspace_tip` still equal to `git show-ref --verify`"
* **Evidence checked**:
  - `src/git/repo.rs:176-190`: `resolve_commit` executes `let id = self.resolve(rev)?;` followed by `self.repo.find_object(id)?.peel_to_commit()?`.
  - `gix-odb-0.87.0/src/store_impls/dynamic/find.rs:69-75`: `find_object` checks `self.store.replacements`. When `id` is present in `replacements`, it substitutes `id` with the replacement commit target `C_replaced`.
  - `04-01-PLAN.md:176`: `model::build` obtains `workspace_tip` through `ref_commit("refs/heads/workspace/dev")`. Because `ref_commit` calls `resolve_commit`, `workspace_tip` receives `C_replaced`.
  - `04-01-PLAN.md:130`: The test helper `tip(scratch, work, full)` runs `git show-ref --verify <full>`. In Git, `git show-ref --verify refs/heads/workspace/dev` inspects the ref store directly and prints the unreplaced commit `C_original`.
* **How to reproduce**:
  1. Create a repository where `refs/heads/workspace/dev` points to commit `C_original`.
  2. Create replacement commit `C_replaced` via `git replace C_original C_replaced`.
  3. Resolve the model via `model::resolve`.
  4. Compare `model.workspace_tip` against the output of `tip(..., "refs/heads/workspace/dev")`.
* **What breaks**:
  The test assertion `workspace_tip == tip(...)` fails (`C_replaced != C_original`). Furthermore, if `workspace_tip` resolves to the replacement commit `C_replaced` while Git's ref store holds `C_original`, any subsequent Phase 6/9/wsx push or update operations using `workspace_tip` will push or reference the replacement commit instead of the branch ref target.

---

### 2. `encode_path` omits quotes on paths with edge whitespace before a trailing slash, breaking parser round-trip
* **Severity**: High
* **File:Line**: `.planning/phases/04-repo-model/04-RESEARCH.md:771-774`, `.planning/phases/04-repo-model/04-03-PLAN.md:173`, `.planning/phases/04-repo-model/04-04-PLAN.md:110`
* **Quoted text**:
  - `04-RESEARCH.md:771-774`:
    ```rust
    if bare
        && (path.first().copied().is_some_and(is_edge_whitespace)
            || path.last().copied().is_some_and(is_edge_whitespace))
    ```
  - `04-04-PLAN.md:110`:
    > "`space_before_the_trailing_slash_is_edge_whitespace` (`only a /`): `EdgeWhitespace`"
  - `04-03-PLAN.md:31`:
    > "`carriage_return_before_the_trailing_slash_is_edge_whitespace` each give `Problem::EdgeWhitespace`"
* **Evidence checked**:
  - In `git` and `c_quote`, spaces and slashes inside or at the end of pathnames are not C-quoted (`04-03-PLAN.md:30`).
  - `encode_path` checks `path.last()` on the raw slice without looking behind any trailing slash. For a directory path with edge whitespace before the slash (e.g. `b"a /"` or `b"my dir /"`), `path.last()` is `b'/'` (not whitespace). `encode_path` therefore returns `b"a /"` bare.
  - In `04-03-PLAN.md:174` and `04-04-PLAN.md:110`, `parse` removes one trailing `/` before checking `is_edge_whitespace`. After removing `/`, the remaining slice `b"a "` ends in whitespace, returning `Err(Problem::EdgeWhitespace)`.
  - The proptest generator `git_path()` (`04-RESEARCH.md:801`) joins components with `/` (`parts.join(&b'/')`) and never generates a trailing slash, leaving this gap undetected.
* **How to reproduce**:
  1. Call `encode_path(b"a /")` -> returns `b"a /"` unquoted.
  2. Pass `b"only a /\n"` to `parse()`.
  3. `parse` strips `/` to `b"a "`, trips `is_edge_whitespace`, and returns `Err(Problem::EdgeWhitespace)`.
* **What breaks**:
  Round-trip serialization contract fails for directory paths with trailing whitespace. When Phase 12 (`wsx init` / `wsx migrate`) formats directory manifest entries via `encode_path`, it will emit unquoted lines like `only a /` that `wsx`'s own manifest parser rejects on startup.

---

### 3. Missing trailing slash in `GIT_REPLACE_REF_BASE` drops all replace refs, bypassing replacement chain refusal
* **Severity**: High
* **File:Line**: `.planning/phases/04-repo-model/04-11-PLAN.md:132`, `.planning/phases/04-repo-model/04-08-PLAN.md:123`, `.planning/phases/04-repo-model/04-11-PLAN.md:82`
* **Quoted text**:
  - `04-11-PLAN.md:132`:
    > "Its base is the string at `gitoxide.objects.replaceRefBase` from `self.repo.config_snapshot()`, where gix maps `GIT_REPLACE_REF_BASE`, else `refs/replace/`, as named constants. It lists the refs directly under the base with `refs_directly_under`..."
  - `04-08-PLAN.md:123`:
    > "The caller's prefix ends in `/`, because `prefixed` without the slash also matches `workspace-x` (measured). Filter entries whose remainder contains `/`."
  - `04-11-PLAN.md:82`:
    > "Not covered: `GIT_REPLACE_REF_BASE=refs/other` with no trailing slash, where git itself dies with a `BUG` line and gix reads the original."
* **Evidence checked**:
  - `refs_directly_under(prefix)` (`04-08-PLAN.md:123`) computes `let remainder = &full[prefix.len()..];` and rejects any entry where `remainder.contains(&b'/')`.
  - When `GIT_REPLACE_REF_BASE` is set in the environment without a trailing slash (e.g. `GIT_REPLACE_REF_BASE=refs/replace` or `refs/other`), `gix` accepts it and populates `gitoxide.objects.replaceRefBase` without a trailing slash.
  - When `refs_directly_under(b"refs/replace")` evaluates ref `refs/replace/<oid>`, `remainder` is `/<oid>`, which starts with `/`.
  - `refs_directly_under` filters out every replace ref and returns `vec![]`.
  - `refuse_unfollowable_replacements` sees an empty list and returns `Ok(())`.
  - `gix-odb`'s internal replacement map remains active (it was loaded from `GIT_REPLACE_REF_BASE=refs/replace`), so `gix` follows the single replacement hop.
* **How to reproduce**:
  1. Create replacement chain `A -> B -> C`.
  2. Set `GIT_REPLACE_REF_BASE=refs/replace`.
  3. Call `Repo::discover` and `resolve()`.
  4. `refuse_unfollowable_replacements` returns `Ok(())` instead of refusing the chain, and `root_entry` reads `B`'s manifest bytes.
* **What breaks**:
  Complete bypass of threat mitigation T-04-30: setting standard environment variable `GIT_REPLACE_REF_BASE=refs/replace` disables chain and cycle detection entirely, causing `wsx` to read corrupted manifest bytes without failing closed.

---

### 4. Incomplete fix of earlier review finding in `ref_commit` (string revision parsing retained)
* **Severity**: Medium
* **File:Line**: `.planning/phases/04-repo-model/04-01-PLAN.md:174` vs `.planning/phases/04-repo-model/04-REVIEWS.md:64`
* **Quoted text**:
  - `04-REVIEWS.md:64`:
    > "Smallest fix: Require the returned reference name to equal the requested full name. Resolve its validated target object rather than passing the name through another revision lookup. Add the crafted loose-file case as a negative control."
  - `04-01-PLAN.md:174`:
    > "Map `target()` without following it to `RefTarget`, gix failures to `Error::Gix { what, message }` naming the ref. `ref_commit`: `None` from `find_ref` is `Error::Gix { what: "read <full>", message: "no such ref" }`, otherwise `self.resolve_commit(full)` through `ObjectId::from_gix`. That second lookup reads the same ref, because the exact name wins over a crafted file, loose and packed (measured)."
* **Evidence checked**:
  - In `find_ref`, the reference is opened, verified against `name() == full`, and yields `RefTarget::Object(id)`.
  - In `ref_commit`, `id` is discarded and `full` is passed to `self.resolve_commit(full)`.
  - `src/git/repo.rs:176-177`: `resolve_commit` passes `rev` to `self.resolve(rev)`, which invokes `self.repo.rev_parse_single(rev)`.
* **How to reproduce**:
  Inspect the implementation specification in `04-01-PLAN.md:174` against `src/git/repo.rs:176-177`.
* **What breaks**:
  The rewrite only implemented name validation in `find_ref` and declined to peel the validated target object in `ref_commit`. Passing `full` through `rev_parse_single` performs redundant Git revision syntax parsing and disk I/O, and triggers `gix-odb` replacement substitution on the commit object.

---

### 5. Placement of `refuse_unfollowable_replacements` inside `root_entry` leaves integration and commit resolution unprotected
* **Severity**: Medium
* **File:Line**: `.planning/phases/04-repo-model/04-11-PLAN.md:41-45,132`, `.planning/phases/04-repo-model/04-CONTEXT.md:104` (D-22), `.planning/phases/04-repo-model/04-10-PLAN.md:140`
* **Quoted text**:
  - `04-11-PLAN.md:41`:
    > "`root_entry`: the replace refs are listed first in `root_entry`, after `build` has read both tips through `ref_commit`, so no manifest read happens under a chain, a cycle or a symbolic replacement"
  - `04-11-PLAN.md:132`:
    > "`build` has already read both tips through `ref_commit` by then, so the refusal comes before any manifest read, not before every read. The check is repo-wide rather than on the three objects read, because gix applies replacements to every kind and later commands read more objects than these."
  - `04-CONTEXT.md:104` (D-22):
    > "A chained or symbolic replacement ref anywhere in the repo stops the model in a configured repo, even among objects the manifest read never touches, and the error says the replacement ref is at fault..."
* **Evidence checked**:
  - In `model::build` (`04-01-PLAN.md:176`), the execution order is:
    1. Verify workspace branch exists.
    2. Check remotes.
    3. Resolve integration branch (invoking `ref_commit` on integration ref).
    4. Resolve workspace tip commit (invoking `ref_commit` on workspace ref).
    5. Call `root_entry(&tip, MANIFEST_NAME)`.
  - `refuse_unfollowable_replacements` is called exclusively inside `root_entry`.
  - If step 3 fails (e.g. `NoIntegrationBranch` or `IntegrationRefSymbolic`), or if a commit replacement cycle on either branch tip triggers an ODB error during step 3 or 4, `build` terminates before `root_entry` is reached.
* **How to reproduce**:
  1. In a configured repository (`wsx.sourceBranch=workspace/dev`), create a replacement chain or cycle.
  2. Remove tracking ref `refs/remotes/origin/develop` and ensure no integration branch candidate exists.
  3. Call `model::resolve`.
* **What breaks**:
  `resolve` returns `Error::NoIntegrationBranch` instead of `Error::Git(ReplacementRef)`, violating D-22's guarantee that a repo-wide replacement ref fault stops the model with `ReplacementRef`. Furthermore, commit replacement cycles fail during `ref_commit` before the replacement refusal check ever runs.

---

### 6. Hardcoded commit SHA `6946ae9` in acceptance criteria breaks verification portability
* **Severity**: Medium
* **File:Line**: `.planning/phases/04-repo-model/04-02-PLAN.md:102`, `.planning/phases/04-repo-model/04-03-PLAN.md:198`, `.planning/phases/04-repo-model/04-10-PLAN.md:161`
* **Quoted text**:
  - `04-02-PLAN.md:102`:
    > "`git diff --name-only 6946ae9 -- tests/harness` prints exactly `tests/harness/fixture.rs` (baseline before the change: prints nothing)"
  - `04-03-PLAN.md:198`:
    > "`out=\"$(git diff 6946ae9 -- Cargo.lock)\" && test -n \"$out\" && ! rg -q '^-[^-]' <<<\"$out\"`"
  - `04-10-PLAN.md:161`:
    > "`out=\"$(git log --no-merges --format=%s 6946ae9.. -- src/model/mod.rs)\" && test -n \"$out\" && ! rg -qv '^[a-z]+\\(04-[0-9]+\\): ' <<<\"$out\"`"
* **Evidence checked**:
  - Inspected repository git history: `6946ae9` is a transient commit from an earlier planning branch, now superseded (`HEAD` is `9e8b2fc`).
  - In shallow clones, rebased branches, or separate worktrees, `6946ae9` is unreachable.
* **How to reproduce**:
  Run any of the cited acceptance commands in a clone or worktree where `6946ae9` is not in the object database.
* **What breaks**:
  `git diff` and `git log` fail with exit 128 (`fatal: ambiguous argument '6946ae9': unknown revision or path not in the working tree`), causing automated acceptance gates to fail falsely.

---

### 7. Direct symbolic escape on `<remote>/HEAD` falsely reports ref non-existence
* **Severity**: Minor
* **File:Line**: `.planning/phases/04-repo-model/04-07-PLAN.md:25,116,134` vs `.planning/phases/04-repo-model/04-01-PLAN.md:116`
* **Quoted text**:
  - `04-07-PLAN.md:116`:
    > "`remote_head_outside_the_remote_is_not_a_default`: `HEAD` symbolic to `refs/heads/main`: the error"
  - `04-07-PLAN.md:134`:
    > "`find_ref(<prefix>HEAD)` must be `RefTarget::Symbolic(target)`, `target` must start with the prefix, and `found` on the rest must find it. An object id, an outside target and a dangling target each fall through to the error, per RESEARCH's table."
  - `04-01-PLAN.md:116`:
    > "`NoIntegrationBranch { remote: Option<Vec<u8>>, tried: Vec<Vec<u8>> }`: with a remote `no integration branch: none of {tried} exists.\nFix it with one of:\n\tgit remote set-head {remote} --auto\n\tgit config --local wsx.baseRef <branch>`"
* **Evidence checked**:
  - In `04-07-PLAN.md:120`, an indirect symbolic escape (`HEAD -> alias -> refs/heads/workspace/dev`) returns `Error::IntegrationRefSymbolic`, stating `{full_ref} is a symbolic ref to {target}, so it could move the base away from where the integration branch lives`.
  - In `04-07-PLAN.md:116,134`, a direct symbolic escape (`refs/remotes/origin/HEAD -> refs/heads/main`) falls through to `Error::NoIntegrationBranch`.
  - `NoIntegrationBranch` lists `tried: [refs/remotes/origin/develop, refs/remotes/origin/next, refs/remotes/origin/HEAD]` and reports that none of them exists.
* **How to reproduce**:
  1. Set `refs/remotes/origin/HEAD` symbolic to `refs/heads/main`.
  2. Call `model::resolve`.
* **What breaks**:
  `wsx` outputs `no integration branch: none of [..., refs/remotes/origin/HEAD] exists`, directly contradicting the repository state where `refs/remotes/origin/HEAD` does exist. Direct symbolic escapes are treated inconsistently with indirect symbolic escapes.

---

## SUSPECTED

### 1. `RepoModel::require` suppresses the "not configured" status for inferred workspace candidates
* **Severity**: Medium
* **File:Line**: `.planning/phases/04-repo-model/04-08-PLAN.md:185` vs `.planning/ROADMAP.md:164` (Success Criterion 5) and `.planning/REQUIREMENTS.md:73` (CONF-07)
* **Quoted text**:
  - `04-08-PLAN.md:185`:
    > "`require`: `Configured(model)` is `Ok(model)`; `NotConfigured { inferred: Ok(model) }` is `Ok(model)`; a `ManifestMissing`, `ManifestNotABlob`, `ManifestUnreadable` or `ManifestInvalid` becomes `NotConfigured { reason: Box::new(err) }`"
  - `ROADMAP.md:164`:
    > "5. A repo with `wsx.sourceBranch` set but a missing or invalid manifest object fails closed with the reason; `a repo without wsx.sourceBranch is reported as not configured`"
  - `REQUIREMENTS.md:73`:
    > "A repo is 'configured' when local git config has `wsx.sourceBranch`."
* **Evidence checked**:
  - `RepoModel::require(self) -> Result<Box<Model>, Error>` unwraps both `Configured(model)` and `NotConfigured { inferred: Ok(model) }` into `Ok(model)`.
  - The returned `Model` struct does not record whether `wsx.sourceBranch` was explicitly configured or inferred via fallback.
* **What could not be verified**:
  Phase 5 is not yet written. Commands in Phase 5 could theoretically inspect `RepoModel` directly instead of calling `require()`, or add a boolean flag to `Model` in Phase 5.
* **What breaks**:
  Any Phase 5 command that uses `RepoModel::require` as prescribed cannot report whether the repository was actually configured or running on fallback, violating Success Criterion 5 / CONF-07.

---

## COVERAGE

### Plans read in full:
- `.planning/phases/04-repo-model/04-01-PLAN.md`
- `.planning/phases/04-repo-model/04-02-PLAN.md`
- `.planning/phases/04-repo-model/04-03-PLAN.md`
- `.planning/phases/04-repo-model/04-04-PLAN.md`
- `.planning/phases/04-repo-model/04-05-PLAN.md`
- `.planning/phases/04-repo-model/04-06-PLAN.md`
- `.planning/phases/04-repo-model/04-07-PLAN.md`
- `.planning/phases/04-repo-model/04-08-PLAN.md`
- `.planning/phases/04-repo-model/04-09-PLAN.md`
- `.planning/phases/04-repo-model/04-10-PLAN.md`
- `.planning/phases/04-repo-model/04-11-PLAN.md`

### Supporting documents read in full:
- `.planning/phases/04-repo-model/04-CONTEXT.md`
- `.planning/phases/04-repo-model/04-RESEARCH.md`
- `.planning/phases/04-repo-model/04-VALIDATION.md`
- `.planning/phases/04-repo-model/04-REVIEWS.md`
- `.planning/phases/04-repo-model/04-PATTERNS.md`
- `.planning/ROADMAP.md` (Phase 4 and dependent Phase 5/8/9 sections)
- `.planning/REQUIREMENTS.md` (CONF, WSYNC, PRCUT, DIV, IF)

### Plans not read:
- None. All Phase 4 plans (04-01 through 04-11) were read in full.
