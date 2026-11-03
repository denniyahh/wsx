---
phase: 4
reviewers: [codex, agy]
reviewed_at: 2026-11-02T16:40:00Z
plans_reviewed: [04-01-PLAN.md, 04-02-PLAN.md, 04-03-PLAN.md, 04-04-PLAN.md, 04-05-PLAN.md, 04-06-PLAN.md, 04-07-PLAN.md, 04-08-PLAN.md, 04-09-PLAN.md, 04-10-PLAN.md, 04-11-PLAN.md]
models:
  codex: "gpt-6.1-sol (reasoning=high)"
  agy: "gemini-3.8-flash-high (effort=high)"
model_sources:
  codex: "command line"
  agy: "command line"
---

# Cross-AI Plan Review — Phase 4, second round

Lens: adversarial. Snapshot: 9e8b2fc, the replan from the first round (733713f, whose REVIEWS.md is
in git history). Run through the external-review skill, not `/gsd-review`. Reviewers were told not
to repeat first-round findings unless the rewrite left them unfixed.

<!-- gsd:plan-revision-conflicts:begin -->
## Plan-Revision Conflicts
<!-- gsd:plan-revision-conflicts:end -->

## Consensus Summary

Read this section first: it is the driving agent's verdict on every lane finding, and only the rows
marked "fix" or "document" are for the replan. The rejected rows were reproduced false; do not act
on the lane text for them. Every verdict comes from a re-check on gix 0.90.0, git 2.56.0 and
cargo-mutants 27.1.0, each with a control. Phase 4 is not implemented, so the probes reimplement
the plans' wording; the guard probe listed refs with a filtered `references().all()`, not
`refs_directly_under`.

| # | Finding | Raised by | Re-check | Severity | Disposition |
|---|---------|-----------|----------|----------|-------------|
| 1 | Two replace refs whose names differ only in hex case pass the guard; git refuses to read | Codex | Reproduced | major | fix |
| 2 | `gitoxide.objects.replaceRefBase` in config moves gix's replace base but not git's | Codex | Reproduced | major | fix |
| 3 | The `--iterate` mutation gate certifies a survivor after a production fix | Codex | Reproduced | medium | fix |
| 4 | `encode_path` leaves edge whitespace before a trailing slash bare, and `parse` refuses it | agy | Traced in plan text | minor | fix |
| 5 | `NoIntegrationBranch` says none of `tried` exists when `<remote>/HEAD` exists but is unusable | agy | Traced in RESEARCH code | minor | fix |
| 6 | `ref_commit` validates a ref, then resolves it again by name | both | Reproduced | minor | fix |
| 7 | The guard reads live refs while gix reads with the replacement map loaded at open | Codex | Reproduced; downgraded from major | low | document |
| - | `workspace_tip` becomes the replacement commit under a commit replacement | agy | Reproduced false | - | reject |
| - | A slashless `GIT_REPLACE_REF_BASE` makes gix follow a chain | agy | Reproduced false | - | reject |
| - | A commit replacement cycle fails in `ref_commit` before the guard | agy | Reproduced false | - | reject |
| - | `6946ae9` in acceptance commands is unreachable | agy | False: an ancestor of HEAD | - | reject |
| - | `require` hides that an inferred repo is not configured | agy | By design, D-15 and D-16 | - | reject |

### Findings to fix

**1. Case-only duplicate replace refs (`04-11-PLAN.md:132`).** With `refs/replace/<a>` to B and
`refs/replace/<A upper-cased>` to C, neither target replaced: the planned guard passed, gix read B's
manifest, and `git cat-file blob` exited 128 with `fatal: duplicate replace ref`. Control: deleting
the upper-case ref gave agreement (both read B). Collapsing names into a lower-cased set discards
the duplicate. Fix: refuse when two names under the base are equal once lower-cased, with an error
naming both refs and the `git update-ref --no-deref -d` fix. Add a named test with that control.

**2. gix-only replace base key (`04-11-PLAN.md:132`).** With no `GIT_REPLACE_REF_BASE`,
`git config --local gitoxide.objects.replaceRefBase refs/other/` and `refs/other/<A>` to C: the guard
passed with one ref, gix read C's manifest and git read A's. Control: unsetting the key gave
agreement on A. The guard trusts gix's base, so it cannot see that git uses another one. Fix: the
base git uses is `GIT_REPLACE_REF_BASE`, else `refs/replace/`; refuse when gix's effective
`gitoxide.objects.replaceRefBase` differs from that, with an error naming the key and how to unset
it. Add a named test with the unset control. The existing `GIT_REPLACE_REF_BASE=refs/other/` test
must still pass, since there both readers agree.

**3. Mutation gate after a fix (`04-10-PLAN.md:103`, and the limit admitted at `04-10-PLAN.md:190`).**
cargo-mutants 27.1.0 on a toy crate: all four mutants caught; the implementation and its test then
changed in place; `--iterate` exited 0 having tested zero mutants; a fresh run exited 2 with
`replace + with * in bump` missed. So the final `missed.txt` does not cover the final source. Fix:
the run the second `<automated>` block reads is a fresh full pass, without `--iterate`, after the
last commit that touches production code. `--iterate` may still be used while closing gaps.

**4. Edge whitespace before a trailing slash (`04-RESEARCH.md:771-774`, `04-04-PLAN.md:110`).**
`encode_path` tests `path.last()`, which is `/` for `a /`, so it returns `a /` bare; `parse` strips
one trailing `/` and refuses `a ` as `EdgeWhitespace`. It fails closed, but wsx refuses its own
output. The proptest's `git_path()` never generates a trailing slash. Fix: test edge whitespace on
the path with one trailing `/` removed; let the generator add an optional trailing `/`; name a test
that `encode_path(b"a /")` round-trips through `parse`.

**5. `NoIntegrationBranch` wording (`04-RESEARCH.md:720-760`, `04-01-PLAN.md:116`).** `tried` gets
`<prefix>HEAD` pushed unconditionally, and the message says `none of {tried} exists`, which is false
when `HEAD` exists but holds an object id or points outside the remote (`04-07-PLAN.md:115-116`).
Fix: word it so it is true in every case that reaches it, for example that none of them names an
integration branch, and pin the wording with the outside-target test.

**6. Second lookup in `ref_commit` (`04-01-PLAN.md:174`, used by `04-07-PLAN.md:134`).** The probe
validated `refs/remotes/origin/develop` as a direct ref to the remote commit, then made it symbolic
to `refs/heads/workspace/dev` before the second lookup, which returned the workspace commit. Control
without the change returned the remote commit. This was the first round's "resolve its validated
target object" recommendation (733713f, Codex finding 3, line 64), not applied. It also makes
`04-09-PLAN.md:235` ("it reads each ref once") false. Fix: `ref_commit` peels the id from
`RefTarget::Object`, through its hex as `root_entry` already does, and `found` in `integration` uses
that id, so each ref is read once. No deterministic test can place the change between two reads, so
the evidence is the code shape and 04-10's mutation pass; say so in the plan.

### Finding to document, not fix

**7. Replacement map frozen at open (`04-11-PLAN.md:75`, `:132`).** With A->B->C, deleting both
replace refs after `gix::open` and before the guard: guard passed, the open handle read B, git read
A. A fresh open agreed with git. Adding one replacement after open gave the opposite split, but the
handle's answer there was true at open, which 04-09's consistency contract already allows. Only a
replacement changed during one run can produce this, and git has no snapshot to read against.
Disposition (operator, 2026-11-03): accept. State it in 04-09's consistency contract and in the
guard's doc comment: gix loads the replacement map at open, the guard lists refs later, and a
replacement changed in between can make them disagree.

### Rejected

- **agy 1, `workspace_tip` under a commit replacement.** `peel_to_commit` kept the original id, equal
  to `git show-ref --verify`, and the manifest bytes matched git's. The planned assertion holds.
- **agy 3, slashless `GIT_REPLACE_REF_BASE=refs/replace`.** gix read the original, not a chain hop;
  git died with `BUG: refs.c:1908`. The guard does see no refs, but nothing reads differently from
  a git that can read. `04-11-PLAN.md:82` already records it as not covered.
- **agy 5, a commit cycle fails before the guard.** The tip read succeeded and the guard refused. Its
  other half, that an error such as `NoIntegrationBranch` can surface before the replacement
  refusal, is true, but the model still stops, so D-22 holds.
- **agy 6, `6946ae9`.** `git merge-base --is-ancestor 6946ae9 HEAD` succeeds; it is not superseded.
- **agy suspected, `require` and inferred repos.** D-15 and D-16 let commands use an inferred branch,
  and `RepoModel` still carries the not-configured state.

## Codex Review

## CONFIRMED

1. **Major — The symbolic-ref fix still permits a local workspace commit to become the integration base.**

   **Citations:** `.planning/phases/04-repo-model/04-07-PLAN.md:134`: “`RefTarget::Object(_)` is an `Integration` whose tip comes from `ref_commit`.”  
   `.planning/phases/04-repo-model/04-01-PLAN.md:174`: “otherwise `self.resolve_commit(full)`”.  
   This contradicts `04-09-PLAN.md:235`: “it reads each ref once”.

   **Evidence checked:** `src/git/repo.rs:162–187` resolves the name again and peels its current target. The new wrapper’s existence check does not prohibit following a symbolic target.

   **Reproduce:** Read a direct `refs/remotes/origin/develop`, then run:
   ```bash
   git symbolic-ref refs/remotes/origin/develop refs/heads/workspace/dev
   ```
   Continue through the prescribed `ref_commit`. With gix 0.90.0, the first lookup returned the remote commit; the second returned the distinct workspace commit. Without the update, it returned the remote commit.

   **What breaks:** D-06’s remote-only base restriction. This is an **unfixed part of the earlier review**: `04-REVIEWS.md:64` specifically recommended resolving the validated object instead of looking up the name again. The rewrite checks the name but still discards the validated object.

2. **Major — Lowercasing replacement names conceals duplicate replacements that make Git abort.**

   **Citation:** `.planning/phases/04-repo-model/04-11-PLAN.md:132`: “collects the lower-cased names after the base into a `BTreeSet`, because git and gix both accept uppercase hex”.

   **Evidence checked:** Executed the prescribed listing, set construction and target checks against gix 0.90.0 and Git 2.56.0.

   **Reproduce:** For manifest blob A and two different valid manifest blobs B and C:
   ```bash
   git update-ref refs/replace/<lowercase-A> <B>
   git update-ref refs/replace/<uppercase-A> <C>
   ```
   Neither target is itself replaced. The planned guard returned success; gix read B. Git exited **128** with `fatal: duplicate replace ref`. Removing the uppercase ref restored agreement.

   **What breaks:** The model accepts a manifest where Git refuses to read one. Case-insensitive object identity requires duplicate detection; collapsing names into a set removes exactly that evidence. The uppercase-chain tests do not exercise this collision.

3. **Major — The replacement guard validates live refs against an object reader using an older replacement map.**

   **Citations:** `.planning/phases/04-repo-model/04-11-PLAN.md:75`: “gix loads them at open”.  
   The same plan, line 132: “It lists the refs directly under the base”.

   **Evidence checked:** gix’s cached replacement-map construction in `gix-0.90.0/src/open/repository.rs:531–560`; wsx’s repository opening in `src/git/repo.rs:32–67`; the proposed guard and manifest-read calls.

   **Reproduce:** Install A→B→C, open the repository, then delete both replacement refs **before any manifest object read**. Run the planned guard and read through that existing handle.

   Measured:
   ```text
   guard: success
   existing gix handle: only intermediate
   git: only original
   freshly opened gix handle: only original
   ```

   **What breaks:** The earlier chained-replacement defect remains reachable through a controlled interleaving. Scanning an empty live namespace cannot certify the cached replacement map. The replacement tests create everything before discovery and therefore miss this mismatch.

4. **Major — A gix-only config key produces wrong manifest bytes with no chain, cycle or symbolic ref.**

   **Citation:** `.planning/phases/04-repo-model/04-11-PLAN.md:132`: “Its base is the string at `gitoxide.objects.replaceRefBase` from `self.repo.config_snapshot()`”.

   **Evidence checked:** gix’s replacement-base selection in `gix-0.90.0/src/open/repository.rs:668–687`, the proposed guard, and the manifest read path. `src/git/repo.rs:32–67` adds no normalization of this setting to Git’s behavior.

   **Reproduce:** With `GIT_REPLACE_REF_BASE` absent:
   ```bash
   git config --local gitoxide.objects.replaceRefBase refs/other/
   git update-ref refs/other/<manifest-blob-A> <manifest-blob-C>
   ```
   The planned guard accepted the single replacement. gix read `only secret`; Git read `only original`. Unsetting the config key restored agreement.

   **What breaks:** The guard validates gix’s namespace without establishing that Git uses that namespace. This violates the plan’s line-25 prohibition against reading different manifest bytes. Testing the environment-variable override does not test the reader-specific config key.

5. **Medium — The rewritten mutation gate can certify a survivor on the final source.**

   **Citation:** `.planning/phases/04-repo-model/04-10-PLAN.md:103`: “Re-run with `--iterate` added, which skips mutants already caught”. The same step permits production fixes between runs.

   **Evidence checked:** Reproduced with cargo-mutants **27.1.0**, nextest, the plan’s test filter, and its survivor predicate.

   **Reproduce:** Start with:
   ```rust
   pub fn bump(value: u32) -> u32 { value + 1 }
   // Test: bump(2) == 3
   ```
   All four generated mutants were caught. Change the implementation to `value + 2` and update the expected result to `4`, preserving source locations.

   Results:
   ```text
   --iterate: exit 0; zero mutants tested; four previously caught
   planned survivor gate: exit 0
   fresh run: exit 2; “replace + with * in bump” survived
   ```
   The cached count and current `--list` count both equalled four, so the proposed count audit also passed.

   **What breaks:** The final `missed.txt` does not establish mutation coverage of the final source. Line 190 acknowledges this limitation but leaves the acceptance gate unchanged. A fresh pass after production changes is required to support its claim.

## SUSPECTED

None retained. The concurrency findings use deterministic interleavings; their frequency during ordinary use was not measured.

## COVERAGE

- **Read in full:** All eleven plans, `04-CONTEXT.md`, `04-RESEARCH.md`, `04-VALIDATION.md`, `04-REVIEWS.md`, the Phase 4 roadmap section, `CLAUDE.md`, and `CONTRIBUTING.md`.
- **Read selectively:** Requirements covering configuration and downstream consumers; Rust discovery, resolution, reads and test isolation; frozen `bin/wsx` configuration/manifest/base resolution; harness fixtures and oracle setup; relevant script sections. `scripts/check.sh` and `scripts/check-in-container.sh` were read in full.
- **Verification limits:** Ran isolated reproductions of the prescribed algorithms and mutation gate. No completed Phase 4 model exists to exercise. The snapshot lacks Git metadata; no commit identity, full project gate or container gate was verified.
- **Cleanup:** Snapshot file bytes matched the pre-probe hashes. Review scratch was removed, probe processes exited, and no cargo-mutants temporary directories remained.

## Agy Review

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
