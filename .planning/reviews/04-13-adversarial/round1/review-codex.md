## CONFIRMED

1. **Critical — Git-created symlink refs still redirect the manifest past this guard.**

   **File:** `.planning/phases/04-repo-model/04-13-PLAN.md:208–209`  
   **Quote:** “`find_ref` and `ref_commit` read what git reads and only listings take the private store.”

   That claim is false. In `gix-ref-0.69.1/src/store/file/find.rs:338–344`, gix opens the reference’s filesystem path and reads its contents. Git can instead interpret a symlink beginning with `refs/` as a repository-relative symbolic ref.

   **Reproduce:** In linked worktree `W` of repository `R`, let commit `A` contain `only .planning` and commit `B` contain `only b`; configure `workspace/dev` as the source branch:

   ```bash
   git -C "$W" update-ref refs/worktree/x "$A"
   git -C "$W" update-ref refs/heads/workspace/refs/worktree/x "$B"
   git -C "$W" -c core.preferSymlinkRefs=true symbolic-ref \
     refs/heads/workspace/dev refs/worktree/x
   ```

   **Checked:** Git 2.56.1 created the symlink. Git’s manifest read printed `only .planning`; the snapshot’s `model::resolve` returned `Configured` with `only b`. The proposed name filter found no forbidden private entry: only an allowed `worktree/` directory was involved.

   **Opposite control:** Replacing the symlink with textual `ref: refs/worktree/x` made both readers select `A`.

   **What breaks:** CONF-07 remains violated after the proposed check. The plan explicitly leaves the affected lookup code unchanged at line 453. This is a surviving pre-existing gap, reproduced using Git’s own supported—though deprecated—configuration.

2. **Major — Candidate inference still silently drops a branch Git lists.**

   **File:** `.planning/phases/04-repo-model/04-13-PLAN.md:138–140`  
   **Quote:** “WR-01 is closed where the listing happens.”

   The private-directory check does not address symlinks in the shared ref store. `gix-ref-0.69.1/src/store/file/loose/iter.rs:47–48` skips entries whose filesystem type is not a regular file.

   **Reproduce:** Keep a regular `workspace/dev` branch, unset `wsx.sourceBranch`, and run:

   ```bash
   git -C "$W" update-ref refs/heads/workspace/refs/heads/main "$A"
   git -C "$W" -c core.preferSymlinkRefs=true symbolic-ref \
     refs/heads/workspace/other refs/heads/main
   git -C "$W" for-each-ref --format='%(refname)' \
     'refs/heads/workspace/*'
   ```

   The nested ref makes the symlink’s filesystem target exist.

   **Checked:** Git listed `workspace/dev` and `workspace/other`. The proposed filter found no forbidden private entry. `resolve` returned `NotConfigured` with a successfully inferred `workspace/dev` model.

   **Opposite control:** Converting `other` to a textual symbolic ref preserved Git’s two candidates and changed wsx’s result to `WorkspaceCandidates`.

   **What breaks:** The D-21 ambiguity check disappears because gix omits a valid candidate. This is a separate iterator failure from finding 1’s lookup failure.

3. **Minor — The whitelist permits files that gix lists and Git ignores.**

   **File:** `.planning/phases/04-repo-model/04-13-PLAN.md:130–131`  
   **Quote:** “Entry names are matched whole, whatever kind of entry they are.”

   **Reproduce:** Put a commit ID in a regular file at `<private-git-dir>/refs/bisect`, without a trailing namespace component. Repeat with `rewritten` or `worktree`.

   **Checked:** The proposed predicate accepted all three names. Gix listed `refs/bisect`, `refs/rewritten`, and `refs/worktree`; Git’s listing omitted them. Through the snapshot’s public API, `refs_directly_under(b"refs/")` returned the planted leaf.

   **Opposite control:** Replacing the leaf with Git-created `refs/bisect/x` made both readers list the namespaced ref.

   **What breaks:** The check does not establish its claimed listing boundary. The allowed namespaces are directory prefixes; accepting identically named regular files leaves another listing discrepancy. This probe did **not** redirect either current candidate prefix or the manifest.

## SUSPECTED

4. **Minor — The claimed independent protection in `root_entry` has no independent negative control.**

   **File:** `.planning/phases/04-repo-model/04-13-PLAN.md:58`  
   **Quote:** “`build`’s call before any object read and `root_entry`’s own call both refuse a linked worktree’s stray refs.”

   **Evidence checked:** Every proposed refusal fixture at lines 298–300 stops in `build` or candidate inference. The deletion check at line 338 removes the new call inside the shared `refuse_replace_refs` function, rather than removing `root_entry`’s invocation at `src/git/refs.rs:217`.

   **Reproduce after GREEN:** Delete only `root_entry`’s `self.refuse_replace_refs()?;` and run the six new model tests.

   **Prediction:** Those tests still pass: refusal fixtures stop earlier, and controls contain no forbidden entry. A direct `root_entry` fixture with private replacement refs would distinguish the cases.

   **Unverified:** I did not execute this deletion against the not-yet-written tests. The finding concerns the proposed verification, not a demonstrated missing call.

## COVERAGE

- Read **04-13-PLAN.md, 04-REVIEW.md, 04-CONTEXT.md, 04-12-SUMMARY.md, and CLAUDE.md in full**.
- Read relevant portions of **04-12-PLAN.md**, **REQUIREMENTS.md**, `src/git/`, and `src/model/mod.rs`; not every source file or test.
- Inspected relevant vendored gix lookup, iteration, packed-ref, categorisation, and repository-opening code. Runtime probes used gix 0.90.0 and Git 2.56.1.
- Checked installed Git documentation for the three private namespaces. In the tested private `packed-refs` branch case, both readers ignored that file.
- Probes linked unchanged snapshot modules with a separate implementation of the proposed name predicate. No full suite, container gate, mutation pass, or proposed shell verification block was run.
- Scratch repositories and binaries were removed; timed-out children were killed and reaped. Target and source hashes stayed unchanged. `.planning/STATE.md` changed during the session; no reviewer command wrote it, and its writer is unverified.
