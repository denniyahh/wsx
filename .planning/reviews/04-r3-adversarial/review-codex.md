## CONFIRMED

1. **High — An exact-name check does not prevent namespace redirection.**

   **Citation:** `.planning/phases/04-repo-model/04-01-PLAN.md:187`:
   > “A returned reference whose `name()` differs from `full` reads as `None`”

   **Evidence checked:** Actual `Repo::discover` and `open_gix` in `src/git/repo.rs:32–67`, `peel_commit` in `src/git/refs.rs:10`, and the prescribed reference/tree/blob sequence, using gix 0.90.0 and Git 2.56.0.

   **Reproduce:** Create ordinary `workspace/dev` and `develop` refs at commit X, plus corresponding refs under `refs/namespaces/foo/refs/heads/` at commit Y. Give X and Y different manifests. Set:
   ```bash
   git config --local wsx.sourceBranch workspace/dev
   git config --local gitoxide.core.refsNamespace foo
   ```

   Measured: gix returned Y while reporting the name **`refs/heads/workspace/dev`**, so the planned equality check accepted it. Its manifest was `only hidden`; `git show-ref --verify` returned X and `git cat-file blob refs/heads/workspace/dev:.wsx.conf` printed `only original`. `GIT_NAMESPACE=foo` reproduced the discrepancy. Removing the setting restored agreement.

   **What breaks:** The model can select another workspace tip, integration tip and manifest without an error. Neither exact-name comparison nor the revspec ban prevents this. The plan needs explicit namespace handling and a differential control. This reproduction exercised actual discovery and the prescribed reads; the Phase 4 model itself does not exist yet.

2. **High — The corrupt-tree limitation understates a fail-open case.**

   **Citations:** `.planning/phases/04-repo-model/04-09-PLAN.md:97`:
   > “it could read as `Absent`, which still fails closed when configured”

   `.planning/phases/04-repo-model/04-RESEARCH.md:449`:
   > `let Some(entry) = tree.find_entry(name) else {`

   **Evidence checked:** Actual discovery and commit peeling, gix’s `Tree::find_entry`, and the planned mode/blob checks. `find_entry` searches by iteration; it does not establish Git-equivalent lookup on an unsorted tree.

   **Reproduce:** Write a raw tree containing these valid entries in this order:
   ```text
   100644 z.txt\0<20-byte blob ID>
   100644 .wsx.conf\0<20-byte manifest blob ID>
   ```
   Store it with `git hash-object --literally -t tree -w --stdin`, wrap it with `git commit-tree`, and point `workspace/dev` and local `develop` at that commit.

   Measured: actual `Repo::discover` and `peel_commit` succeeded; the prescribed gix lookup returned the manifest bytes. Git’s `cat-file blob refs/heads/workspace/dev:.wsx.conf` exited **128**, reporting that the path did not exist. Sorting the same entries restored agreement.

   **What breaks:** The planned reader accepts a manifest Git cannot resolve. This requires a malformed tree, not an ordinary working-tree commit, but corrupt-object handling is explicitly in scope. The current limitation describes an imprecise refusal where this case produces successful parsing instead.

3. **Medium — The proposed repair commands allow shell execution from a ref name.**

   **Citation:** `.planning/phases/04-repo-model/04-11-PLAN.md:144`:
   > “`git update-ref --no-deref -d {name}`”
   >
   > “Every byte field goes through `c_quote`.”

   **Evidence checked:** The exact gix quoting call prescribed by `04-RESEARCH.md:408–410`, Git ref-name validation, and execution of the rendered command in isolated scratch space.

   **Reproduce:** Create a symbolic replacement ref whose literal name contains:
   ```text
   refs/replace/$(touch${IFS}marker)
   ```
   Git accepts this ref name. `c_quote` leaves its shell metacharacters unchanged. The planned symbolic-replacement refusal therefore prints an executable command substitution in its repair command.

   Measured with a scratch marker path: pasting the proposed command created the marker, then `git update-ref` exited **1**. Passing the literal name as one subprocess argument removed the ref with exit **0** and created no marker.

   **What breaks:** Following the diagnostic can execute ref-controlled shell text and fail to remove the offending ref. This is conditional on someone running the printed repair command; wsx does not execute it automatically. Display quoting and shell-argument quoting need separate handling.

4. **Medium — The integration fixtures do not discriminate the commit’s provenance.**

   **Citations:** `.planning/phases/04-repo-model/04-01-PLAN.md:137`:
   > “`commit_all` at `T0`, then `git branch develop` and `git branch next`”

   `.planning/phases/04-repo-model/04-06-PLAN.md:78`:
   > “`fork-source` (a clone of `source` with `develop`)”

   `.planning/phases/04-repo-model/04-06-PLAN.md:135`:
   > “`integration.full_ref` is `refs/remotes/upstream/develop`”

   **Evidence checked:** Reproduced the fixture recipe and inspected the chain assertions in `04-07-PLAN.md:113–115`.

   **Reproduce:** Build the prescribed source and fork fixtures, then compare the IDs of `origin/main`, `origin/develop`, `origin/next` and `upstream/develop`.

   Measured: all held the same commit. Independent set counting and pairwise comparisons agreed. A control commit made only on upstream’s `develop` produced distinct tips.

   **What breaks:** The prescribed assertions can identify the correct branch label while failing to distinguish a tip taken from the wrong rung or remote. Require divergent commits and assert the selected tip against Git, including inequality with the rejected candidates. This is a confirmed weakness in the planned acceptance signal; no implemented Phase 4 suite was available to run a wrong-tip implementation against.

## SUSPECTED

None retained.

## COVERAGE

- **Plans read in full:** All eleven, `04-01-PLAN.md` through `04-11-PLAN.md`. None omitted.
- **Supporting review:** Read `04-CONTEXT.md`, prior-review dispositions and the Phase 4 roadmap section; inspected relevant research, patterns, validation, requirements, Rust code, `clippy.toml`, `deny.toml`, frozen `bin/wsx`, contributor guidance and scripts.
- **Verification limits:** Probes used Git 2.56.0, the pinned gix build, actual discovery/peeling code and prescribed future reads. No full Phase 4, mutation or container gate was established. `scripts/check.sh fmt` exited **1** because the snapshot’s `.git` directory contains no usable repository metadata.
- **Cleanup:** No repository files were edited. All probe commands exited, scratch was removed, and no generated `target/` remained.
