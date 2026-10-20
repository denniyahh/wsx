## CONFIRMED

1. **New shared refs can bypass the guard when their commits already exist on the remote workspace branch.**

   - **Citation:** `.planning/REQUIREMENTS.md:81`; `oracle-devflow/pre-push:92-105`; `.planning/research/DEVFLOW-SETUP.md:17-24`.
   - **Problem:** GUARD-01 prescribes `--not --remotes=<push-remote>` for a new ref. Since the workspace branch is itself pushed to that remote, a new `feature/*` ref based on the workspace tip has all its personal-file commits excluded as “already remote.” The guard sees an empty range and permits the new shared branch.
   - **Why it matters:** This directly violates the boundary: a phase branch forked from the workspace branch can be published as a shared branch with personal history reachable from it.
   - **Suggested requirement fix:** For a new non-workspace ref, exclude only remote refs permitted to contain its ancestry (integration/shared refs), never `workspace/*` or `personal/*`. Add a negative control: remote workspace already contains `.planning/x`; pushing a new `feature/leak` at that commit must refuse. A clean branch rooted at integration must pass.

2. **Phase cutting permits silently incomplete code PRs.**

   - **Citation:** `.planning/REQUIREMENTS.md:62-69`; `scripts/workspace/cut-pr-branch.sh:131-145`; `scripts/workspace/cut-pr-branch.sh:264-286`.
   - **Problem:** Phase mode selects only commits whose subject matches the phase scope. A required code commit without that scope is omitted. The fidelity report merely warns when the PR differs from the source branch; it still exits successfully.
   - **Why it matters:** “Never lose work” is incompatible with successful output that knowingly lacks source-branch code. A warning is not a safe cut gate.
   - **Suggested requirement fix:** Make any non-personal source/PR difference fail by default, with an explicit, recorded `--allow-incomplete <reason>` escape hatch. Negative control: a phase commit depends on an unscoped helper commit; cut must refuse, not emit a successful PR branch.

3. **The promised raw-byte manifest grammar is underspecified and cannot be line-based without an encoding.**

   - **Citation:** `.planning/REQUIREMENTS.md:39`; `scripts/workspace/cut-pr-branch.sh:121-126`.
   - **Problem:** CONF-01 says paths are raw bytes while defining a line grammar, `only <path>` / `modified <path>`. Git permits filenames containing newline, carriage return, leading spaces, and arbitrary non-NUL bytes. A literal line grammar cannot round-trip these names.
   - **Why it matters:** A malformed or ambiguously decoded declaration makes the path matcher disagree with Git’s raw path, creating a personal-file leak or an erroneous refusal.
   - **Suggested requirement fix:** Specify an injective representation for path bytes (for example, a strict ASCII hex or percent encoding) and require decoder round-trip tests for newline, non-UTF-8, spaces, and prefix-boundary names.

4. **Parity leaves an existing local PR branch exposed to destructive reset.**

   - **Citation:** `.planning/REQUIREMENTS.md:20-22,62-69`; `scripts/workspace/cut-pr-branch.sh:167-183`.
   - **Problem:** The oracle does `git branch -f "$PR_BRANCH" "$BASE_REF"`. Requirements demand parity/hardened behavior but do not say that `wsx cut` must refuse an existing target branch or preserve it. The “no half-made PR branch” promise applies only to predicted conflicts.
   - **Why it matters:** An unpushed commit on an existing PR branch can be removed from the named ref before the cut begins. Reflog recovery is not “never lose work.”
   - **Suggested requirement fix:** Refuse an existing target ref by default. If replacement is retained, require an explicit flag, a backup ref, and a test proving an existing sentinel commit remains reachable after every failed cut.

5. **Merge commits have no defined cut semantics.**

   - **Citation:** `.planning/REQUIREMENTS.md:62-67,70`; `scripts/workspace/cut-pr-branch.sh:141,164,199-200`.
   - **Problem:** Phase mode excludes merges with `--no-merges`; non-phase mode includes them, but `git cherry-pick <merge>` requires a mainline parent and the oracle swallows that failure. PRCUT-07 removes the swallowing but does not define whether a merge is rejected, flattened, or replayed with a chosen parent.
   - **Why it matters:** A merge-resolution-only code change can be silently absent under oracle behavior or make a normal feature branch uncuttable under hardened behavior.
   - **Suggested requirement fix:** State one safe policy: preferably detect any selected merge before writes and refuse with its SHA and required recovery procedure. Add cases for a merge containing a resolution and for a phase branch that merged integration.

6. **The hook has no authoritative source for repo-specific personal paths.**

   - **Citation:** `.planning/REQUIREMENTS.md:39-45,65,81-84`; `.planning/PROJECT.md:68-70`.
   - **Problem:** `.wsx.conf` is personal and removed from clean PR branches, while the config-installed hook can run from any pushing worktree. The requirements say every component uses one resolved list, but do not say whether the hook reads the workspace ref, a repo config value, or the current branch.
   - **Why it matters:** A clean PR branch normally lacks the custom `only` declarations needed to reject a newly added repo-specific personal path. Falling back to built-ins leaks custom paths; silently accepting a missing manifest defeats fail-closed behavior.
   - **Suggested requirement fix:** `install-hooks` must persist an authoritative workspace-ref/config locator. Hooks must load `<workspace-ref>:.wsx.conf` independent of the current checkout and refuse once configured if that object is absent or invalid. Test a clean PR checkout with a custom `only private-tools` declaration.

7. **Migration cannot safely cut over gsd-core and gsd-pi as described.**

   - **Citation:** `.planning/PROJECT.md:56-60`; `.planning/research/WORKSPACE-MODE.md:68-84`; `.planning/REQUIREMENTS.md:101-108`.
   - **Problem:** MIG-01 converts existing manifests, but gsd-core and gsd-pi have no manifest and have undeclared personal/shared divergence. MIG-03 says real-repo migration is verified, but gives no classification procedure or falsifiable expected result for those repos.
   - **Why it matters:** Automatic guessing can mark shared changes personal or vice versa; refusing without a bootstrap workflow means the stated three-repo replacement cannot complete.
   - **Suggested requirement fix:** Add a noninteractive bootstrap mode that inventories every raw-byte divergence and requires explicit `only`/`modified` classification before writing or committing `.wsx.conf`. Unclassified paths must abort with no mutation. Test a no-manifest fork layout with both personal additions and shared-file modifications.

8. **`wsx rm` can neither prove “cut to a PR” nor define what it preserves.**

   - **Citation:** `.planning/REQUIREMENTS.md:54-58,112-113`; `oracle-devflow/CONTRIBUTING-workflow-b.md:32-43`.
   - **Problem:** WTREE-05 says to merge “planning records” and delete the worktree/branch unless commits were merged or cut to a PR. It does not define the path set, how squashed PRs are recognized, what durable cut evidence exists, or conflict/dirty-workspace behavior. The oracle workflow instead manually merges the complete source branch before deletion.
   - **Why it matters:** A false “cut” determination deletes source commits; a whole-branch merge can reintroduce code already squash-merged; a narrow planning-only merge can lose personal artifacts outside an assumed directory.
   - **Suggested requirement fix:** Define durable local cut metadata containing source tip, PR ref, and replay mapping; define exactly which resolved personal paths are retained; refuse deletion when proof is absent. Test both squash-merged and never-pushed PR branches.

## SUSPECTED

1. **A force-pushed/rebased integration branch can resurrect superseded code into the workspace and later PRs.**

   - **Citation:** `.planning/REQUIREMENTS.md:48-50,62-64`; `scripts/workspace/sync-workspace.sh:25-38`.
   - **Problem:** Sync always merges the freshly fetched integration ref. If that branch was rewritten, the workspace retains its former integration ancestry and merges the new lineage. A later cut starts from the new integration tip and can replay commits unique to the abandoned lineage.
   - **Why it matters:** This is a plausible code resurrection/data-loss path in repos that allow rebases or force pushes. The documents do not state that integration branches are immutable.
   - **Suggested requirement fix:** Record the last accepted integration OID and refuse an integration rewrite unless an explicit recovery command is used. Add a force-update remote fixture as the negative control. This is conditional on a repository permitting integration rewrites.

2. **A rename or copy from a personal path into a shared path lacks an explicit rejection rule.**

   - **Citation:** `.planning/REQUIREMENTS.md:65,81-82`; `scripts/workspace/cut-pr-branch.sh:202-209,253-260`.
   - **Problem:** GUARD-01 explicitly requires detection of a rename *into* a personal path, not out of one. PR filtering/auditing is path-based; once a personal file is renamed to `src/...`, the resulting path may no longer match the personal list.
   - **Why it matters:** The contents of a personal file could be published under a clean path even though ordinary path auditing succeeds.
   - **Suggested requirement fix:** Require rejection of rename/copy records where either endpoint is personal, unless an explicit manual reconstruction occurs. Add a raw `-z` rename test in both guard and cut suites.
