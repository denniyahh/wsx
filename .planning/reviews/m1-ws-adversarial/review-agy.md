## CONFIRMED

### 1. `GUARD-01` Leaks Personal Files to Remotes via `--not --remotes=<push-remote>`
* **Citation:** [.planning/REQUIREMENTS.md:81-83](.planning/REQUIREMENTS.md#L81-L83), [oracle-devflow/pre-push:105](oracle-devflow/pre-push#L105), [.planning/REQUIREMENTS.md:42-43](.planning/REQUIREMENTS.md#L42-L43).
  > "A new ref is bounded by `--not --remotes=<push-remote>`. This is a deliberate difference: the oracle checks only a fixed list of branch-name patterns and hard-codes origin"
* **Problem:** When pushing a newly created ref (where `remote_sha` is all zeroes), bounding the revision walk by `--not --remotes=<push-remote>` excludes all commits reachable from any ref under `refs/remotes/<push-remote>/*`. Under the workspace model, the developer routinely pushes their workspace branch to `<push-remote>` (e.g. `origin/workspace/denniyahh`). Because feature and phase branches fork from `workspace/<handle>`, all commits containing `.planning/`, `.claude/`, and other personal files that already exist on `<push-remote>/workspace/<handle>` are excluded from `git log`. If a developer or subagent accidentally pushes a raw phase branch or feature branch directly (`git push origin feature/phase-1`), the guard evaluates only commits not yet present on `<push-remote>`. The existing personal-path commits are not checked; the push succeeds; and personal files are published on a shared `feature/*` branch.
* **Why it matters:** Breaks the Core Value ("wsx must never let a personal file reach a shared branch or PR"). A direct push of an un-cut feature or phase branch bypasses the guard.
* **Suggested requirement fix:** In `GUARD-01`, for any non-workspace branch, bound the lower limit of a new ref against `--not <base-remote>/<integration>` (e.g. `origin/develop` or `upstream/next`), never `--not --remotes=<push-remote>`.

---

### 2. Silent Code Loss via Scope Filtering and False Fidelity Check (`PRCUT-01`, `PRCUT-05`)
* **Citation:** [.planning/REQUIREMENTS.md:62-64](.planning/REQUIREMENTS.md#L62-L64), [.planning/REQUIREMENTS.md:68-69](.planning/REQUIREMENTS.md#L68-L69), [scripts/workspace/cut-pr-branch.sh:135-145, 267-287](scripts/workspace/cut-pr-branch.sh#L135-L145).
  > `PRCUT-01`: "In phase mode (--phase N, or automatic on a phase branch) it replays only commits whose conventional-commit scope names phase N, wherever they were committed."
  > `PRCUT-05`: "A fidelity report fails if the PR branch changes a path no replayed commit touched, and warns for each path whose content differs from the source branch"
* **Problem:** In phase mode, commits are filtered strictly by conventional-commit scope matching phase N (`SCOPE_RE`). Commits with unscoped subjects (`fix: off-by-one`), different scopes (`fix(parser): bug`), or Git fixup/squash commits (`fixup! feat(48): ...`) are omitted from `COMMIT_LIST`. The fidelity check in `PRCUT-05` (and oracle lines 267–287) computes `UNEXPLAINED` as `comm -13 TOUCHED_SET CHANGED_SET` (paths on PR branch not touched by any replayed commit) and only iterates over `CHANGED_SET`. If an excluded commit introduced or modified a source file that no replayed commit touched, that file is neither in `CHANGED_SET` nor `TOUCHED_SET`. `UNEXPLAINED` is empty, the content warning loop never encounters the file, the cut passes cleanly, and code committed on the phase branch is silently dropped from the PR.
* **Why it matters:** Breaks the Core Value ("a cut that silently drops code, is the unacceptable failure").
* **Suggested requirement fix:** In `PRCUT-05`, the fidelity check must compute the non-personal tree diff between `CURRENT_BRANCH` and `BASE_REF`, and compare it directly to the diff between `PR_BRANCH` and `BASE_REF`. Any non-personal path modified on the source branch that is missing from or differs on `PR_BRANCH` must fail the cut unless explicitly confirmed.

---

### 3. Cherry-Pick of Merge Commits Crashes `wsx cut` Once Oracle Errors Are Handled (`PRCUT-01`, `PRCUT-07`)
* **Citation:** [.planning/REQUIREMENTS.md:62-64](.planning/REQUIREMENTS.md#L62-L64), [.planning/REQUIREMENTS.md:70-71](.planning/REQUIREMENTS.md#L70-L71), [scripts/workspace/cut-pr-branch.sh:164, 200](scripts/workspace/cut-pr-branch.sh#L164).
  > `PRCUT-07`: "Cut errors are never swallowed. A failed cherry-pick that isn't a predicted conflict aborts with git's error shown (fixes the oracle's cherry-pick … || true)"
* **Problem:** In non-phase mode, `cut-pr-branch.sh:164` builds `COMMIT_LIST` with `git rev-list --reverse "$FORK_POINT".."$CURRENT_BRANCH"`, without `--no-merges`. When a branch merges `develop` or another feature branch, `git cherry-pick "$HASH"` without `-m` fails with `error: commit <hash> is a merge but no -m option was given` (exit code 128). In the oracle script, this was swallowed by `git cherry-pick ... || true`. `PRCUT-07` mandates that cherry-pick errors abort immediately. Any non-linear history on a feature branch will hard-crash `wsx cut`. If `--no-merges` is applied instead, conflict resolutions made during branch merges are discarded.
* **Why it matters:** Merging integration branches or parallel sub-branches into feature branches is normal Git usage; `wsx cut` will fail or lose conflict resolutions.
* **Suggested requirement fix:** Specify merge handling in `PRCUT-01`: either require branches to be rebased linearly before cutting, flatten merge commits into synthetic diffs against the primary parent, or specify `-m 1` behavior for cherry-picking merge commits.

---

### 4. Re-cutting a PR Branch Overwrites and Destroys Commits (`PRCUT-01`)
* **Citation:** [.planning/REQUIREMENTS.md:62-64](.planning/REQUIREMENTS.md#L62-L64), [scripts/workspace/cut-pr-branch.sh:170-171](scripts/workspace/cut-pr-branch.sh#L170-L171).
  > `cut-pr-branch.sh:171`: `git branch -f "$PR_BRANCH" "$BASE_REF"`
* **Problem:** When re-running `wsx cut` on a branch where `<pr-branch>` already exists, the script force-resets `<pr-branch>` to `$BASE_REF` via `git branch -f`. If a developer checked out `<pr-branch>` to make adjustments addressing PR review comments or CI fixes directly on the PR branch, re-cutting unconditionally overwrites the ref.
* **Why it matters:** Causes direct data loss of commits made on the PR branch, violating Core Value ("never lose work while moving it between... PR branches").
* **Suggested requirement fix:** `PRCUT-01` must check whether `<pr-branch>` has commits not reachable from `<base-remote>/<integration>` or `<current-branch>`, and abort unless `--force` is passed.

---

### 5. `WTREE-05` Unmerged Commit Check and Planning Merge are Physically Unimplementable as Specified
* **Citation:** [.planning/REQUIREMENTS.md:58-59](.planning/REQUIREMENTS.md#L58-L59), [.planning/REQUIREMENTS.md:111-112](.planning/REQUIREMENTS.md#L111-L112).
  > `WTREE-05`: "wsx rm <name> merges the worktree's planning records back into the workspace branch, then removes the worktree and branch. It refuses if the branch has commits that are neither merged into the workspace branch nor cut to a PR branch, unless --force"
  > `STAT-01`: "and each worktree with its branch, phase and whether it has been cut"
* **Problem:** Git does not have a native selective path merge command: `git merge` operates on commit trees, bringing in all code commits alongside planning docs. If `wsx rm` creates a synthetic commit copying `.planning/`, Git's commit graph does not register the branch as merged. Furthermore, when commits are replayed to cut a PR branch, cherry-pick creates new commit SHAs. Standard Git ancestry checks (`merge-base`, `branch --merged`) cannot tell whether a commit was "cut to a PR branch". There is no requirement defining persistent cut metadata, git-notes, or state files anywhere in `REQUIREMENTS.md`.
* **Why it matters:** `WTREE-05` and `STAT-01` cannot be implemented against Git's object model as written, leading to false refusals or deleting un-cut code commits under `--force`.
* **Suggested requirement fix:** Define the exact mechanism for tracking cut status (e.g. `git patch-id` comparison or an internal state file in `.git/wsx/`), and clarify whether `wsx rm` performs a full branch merge into `workspace` (as documented in [oracle-devflow/CONTRIBUTING-workflow-b.md:37](oracle-devflow/CONTRIBUTING-workflow-b.md#L37)) or an index-level file checkout of personal paths.

---

### 6. `WTREE-03` Fast-Forward Requirement Breaks Idempotency Once Any Work Has Started
* **Citation:** [.planning/REQUIREMENTS.md:56-57](.planning/REQUIREMENTS.md#L56-L57), [scripts/workspace/phase-worktree.sh:82-86](scripts/workspace/phase-worktree.sh#L82-L86), [.planning/PROJECT.md:28](.planning/PROJECT.md#L28).
  > `WTREE-03`: "Re-running wsx new for an existing worktree fast-forwards it to the workspace branch, or refuses if that isn't a fast-forward. It never rewrites phase history."
  > `PROJECT.md:28`: "Phase worktrees in .worktrees/phase-N... idempotent (phase-worktree.sh): existing (devflow)"
* **Problem:** As implemented in [scripts/workspace/phase-worktree.sh:82](scripts/workspace/phase-worktree.sh#L82) (`git -C "$existing" merge --ff-only "$WORKSPACE_BASE"`), as soon as a single commit is made on the phase branch, the phase branch is ahead of `WORKSPACE_BASE`. If `WORKSPACE_BASE` moves ahead (via `wsx sync`), the branches have diverged. A fast-forward merge cannot succeed. Running `wsx new` on an active worktree will fail with an error (`not a fast-forward -- the branch has its own commits. Resolve by hand; refusing to rewrite phase history`). It is not idempotent once work begins.
* **Why it matters:** Automated agent workflows or subagent invocations calling `wsx new --phase N` to ensure a worktree exists will error out.
* **Suggested requirement fix:** Define idempotency such that if the worktree and branch exist, `wsx new` verifies and reports the worktree path without attempting a failing fast-forward merge unless `--sync` is explicitly passed.

---

### 7. Migration Contradiction: `MIG-03` Mandates Migrating `gsd-core` and `gsd-pi`, but `MIG-01` Has No Ingest for Manifest-less Repos
* **Citation:** [.planning/REQUIREMENTS.md:101-107](.planning/REQUIREMENTS.md#L101-L107), [.planning/REQUIREMENTS.md:108](.planning/REQUIREMENTS.md#L108), [.planning/research/WORKSPACE-MODE.md:68-70](.planning/research/WORKSPACE-MODE.md#L68-L70).
  > `MIG-03`: "Migration of the owner's real repos is verified on real state: devflow, gsd-core, gsd-pi, and the 0.1.0 projects"
  > `WORKSPACE-MODE.md:68-69`: "gsd-core ... Manifest: none ... gsd-pi ... Manifest: none"
  > `MIG-01`: "wsx migrate converts an existing manifest to the new .wsx.conf: 0.1.0 .wsx.conf ... devflow .workspace-divergence ... Legacy grammars are parsed only inside migrate"
* **Problem:** `MIG-03` requires migrating `gsd-core` and `gsd-pi`. However, `WORKSPACE-MODE.md` records that neither repository has an existing manifest (`.workspace-divergence` or 0.1.0 `.wsx.conf`), while both carry undeclared personal files and modified shared files. `MIG-01` only specifies converting existing manifests. There is no requirement or command to bootstrap `.wsx.conf` from an existing diverged workspace tree.
* **Why it matters:** `MIG-03` cannot be satisfied. Running `wsx check` in `gsd-core` and `gsd-pi` will immediately fail with undeclared divergence errors.
* **Suggested requirement fix:** Add a bootstrap mode to `MIG-01` (e.g. `wsx migrate --init` or `wsx init --from-divergence`) that inspects diffs between HEAD and `<base-remote>/<integration>`, matches against built-ins, and populates `only` and `modified` entries in `.wsx.conf`.

---

### 8. Worktree Creation Ignores Submodules and Symlinked Hooks Fail Divergence (`WTREE-01`, `DIV-03`)
* **Citation:** [.planning/REQUIREMENTS.md:54](.planning/REQUIREMENTS.md#L54), [.planning/REQUIREMENTS.md:76](.planning/REQUIREMENTS.md#L76), [oracle-devflow/check-workspace-divergence.sh:95-99](oracle-devflow/check-workspace-divergence.sh#L95-L99).
  > `WTREE-01`: "creates the phase branch and worktree from the workspace branch"
  > `DIV-03`: "Every tracked hook and top-level script is checked for committed mode 100755. The committed mode is what counts, not the working tree's"
* **Problem:**
  1. `git worktree add` leaves submodules uninitialized and empty in new worktrees. Neither `WTREE-01` nor [scripts/workspace/phase-worktree.sh](scripts/workspace/phase-worktree.sh) executes `git submodule update --init --recursive`.
  2. In Git repositories where hooks or scripts are symlinks (e.g. pointing to a shared binary or script), Git stores the committed mode as `120000` (symlink). `DIV-03` strictly checks for `100755` and rejects mode `120000` as non-executable.
* **Why it matters:** Builds fail in worktrees for projects using Git submodules; symlinked hooks fail divergence checks.
* **Suggested requirement fix:** `WTREE-01` should initialize submodules in newly created worktrees; `DIV-03` should accept mode `120000` if the symlink target is executable.

---

## ## SUSPECTED

### 9. SPECULATION: `PRCUT-03` In-Memory Conflict Prediction via `gix` Cannot Reliably Model Filtered Multi-Commit Replay
* **Citation:** [.planning/REQUIREMENTS.md:66-67](.planning/REQUIREMENTS.md#L66-L67), [.planning/research/WORKSPACE-MODE.md:11, 29](.planning/research/WORKSPACE-MODE.md#L11).
  > `PRCUT-03`: "Before writing anything, wsx predicts code conflicts in memory (gix merge)."
  > `WORKSPACE-MODE.md:11`: "gix 0.89.0 has in-memory merge_trees / merge_commits... It has no cherry-pick, no rebase..."
* **Problem:** In a phase PR cut, commits are cherry-picked individually onto `BASE_REF` after stripping personal paths. Calling `gix merge_commits(CURRENT_BRANCH, BASE_REF)` evaluates a monolithic 2-way branch merge of the entire unstripped branch tree. It does not evaluate intermediate cherry-pick conflicts, does not account for skipped commits, and will report conflicts on personal paths that are stripped during replay.
* **Why it matters:** False positives will block valid cuts; false negatives will allow unpredicted conflicts that abort mid-execution.
* **Suggested requirement fix:** Specify that conflict prediction must iteratively replay individual commit tree diffs in memory onto the base tree with personal paths excluded, rather than relying on a whole-branch merge.

---

### 10. SPECULATION: `GUARD-03` "Confirmed wsx-configured" Check Disarms on Clean Branches
* **Citation:** [.planning/REQUIREMENTS.md:83-84](.planning/REQUIREMENTS.md#L83-L84), [.planning/REQUIREMENTS.md:39-40](.planning/REQUIREMENTS.md#L39-L40).
  > `GUARD-03`: "The guard fails closed once the repo is confirmed wsx-configured... Unconfigured repos exit 0."
  > `CONF-01`: ".wsx.conf on the workspace branch declares only <path> ... and modified <path>"
* **Problem:** `.wsx.conf` exists solely on the workspace branch. When a developer or subagent is on a clean PR branch (`feature/phase-1-pr`) or the integration branch (`develop`), `.wsx.conf` is not in the worktree or committed on HEAD. If `wsx hook pre-push` confirms configuration by probing for `.wsx.conf` in the working directory, it will classify the repository as "unconfigured" and exit 0, disabling the guard on the very branches where personal files must be blocked.
* **Why it matters:** Pushing from clean PR worktrees or the integration branch may run unguarded.
* **Suggested requirement fix:** Explicitly define "confirmed wsx-configured" in `GUARD-03`: verify whether local repository configuration contains `wsx.*` keys (`git config --local --get-regexp '^wsx\.'`) or whether local refs matching `refs/heads/workspace/*` exist in the repository, rather than checking the working tree for `.wsx.conf`.
