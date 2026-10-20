## CONFIRMED

### C1. PRCUT-06 allows dirty worktrees during PR cut, silently omitting uncommitted work
- **Requirement:** PRCUT-06 "`wsx cut` never pushes, and never touches the user's checkout: branch, index and uncommitted changes stay as they were (PRCUT-03), so a dirty tree is allowed and reported as not included. This is an improvement: the oracle checks the PR branch out in the user's tree and refuses a dirty one" ([REQUIREMENTS.md:106](wsx/.planning/REQUIREMENTS.md#L106))
- **Prior art:** [cut-pr-branch.sh:109-112](wsx/scripts/workspace/cut-pr-branch.sh#L109-L112)
  ```bash
  echo "==> Validating working tree..."
  if ! git diff-index --quiet HEAD --; then
      echo "error: working tree has uncommitted changes. Commit or stash them before cutting a PR branch." >&2
      exit 1
  fi
  ```
- **Problem:** The oracle's dirty-tree refusal was an essential safety guard preventing an engineer or agent from cutting and publishing an incomplete PR branch while working edits remained uncommitted. PRCUT-06 presents allowing dirty checkouts as an "improvement". However, because PRCUT-05's fidelity check compares committed `source-branch..base` against `PR-branch..base`, uncommitted code in the active worktree is silently omitted from the cut PR branch without tripping fidelity. In automated workflows or agent sessions, uncommitted changes are dropped from the PR without a failure. Calling this an improvement trades a safe refusal for silent omission of work.
- **Better alternative / cost / milestone:** Refuse `wsx cut` whenever the source worktree has uncommitted tracked changes unless `--allow-dirty` is explicitly passed. Low cost (small check prior to replay). Milestone 1.

---

### C2. PRCUT-02 refuses replayed commits touching `modified` paths or truncates them with `--allow-incomplete`
- **Requirement:** PRCUT-02 "A replayed commit that touches a `modified` path refuses the cut, naming the commit and the path: wsx can't separate a shared edit from the workspace-only edit in the same file. `--allow-incomplete` drops that path's change and reports it. This is an improvement: the oracle replays such changes unchecked." ([REQUIREMENTS.md:102](wsx/.planning/REQUIREMENTS.md#L102))
- **Prior art:** [cut-pr-branch.sh:121-126](wsx/scripts/workspace/cut-pr-branch.sh#L121-L126)
  ```bash
  if MANIFEST="$(git show "$CURRENT_BRANCH:.workspace-divergence" 2>/dev/null)"; then
      while read -r KIND ENTRY _; do
          if [ "$KIND" = "only" ] && [ -n "$ENTRY" ]; then
              FORBIDDEN_PATHS+=("$ENTRY")
          fi
      done <<< "$MANIFEST"
  fi
  ```
  and [cut-pr-branch.sh:280-287](wsx/scripts/workspace/cut-pr-branch.sh#L280-L287)
- **Problem:** Files declared `modified` are shared tracked files (e.g., `.gitignore`, `Cargo.toml`, `package.json`) that the workspace branch legitimately alters relative to upstream. Feature work frequently makes legitimate additions to those same shared files (e.g. adding a new build dependency or an ignore pattern). The oracle replays the commit's diff directly onto the integration base; if it applies cleanly, it succeeds, and reports any divergence in the fidelity check. PRCUT-02 turns this into an outright cut refusal. Even worse, its override `--allow-incomplete` "drops that path's change", silently mutilating the commit and stripping necessary project edits (causing broken builds or test failures on the PR branch).
- **Better alternative / cost / milestone:** Attempt replay of the commit's diff against `<base-remote>/<integration>`. If git applies the diff cleanly without conflict, include it and report the path in the fidelity summary; only refuse if there is an unresolvable patch conflict against the clean base tree. Low cost. Milestone 1.

---

### C3. WSYNC-01 cross-worktree merge leaves remote worktrees in an abandoned conflict state
- **Requirement:** WSYNC-01 "`wsx sync` merges `<base-remote>/<integration>` into the workspace branch with a real merge, never a rebase. It runs from any worktree of the repo: the merge happens in the worktree that has the workspace branch checked out. If that worktree is dirty, or the workspace branch isn't checked out anywhere, it refuses and names the fix. This is an improvement: the oracle only runs from a checkout of the workspace branch" ([REQUIREMENTS.md:76](wsx/.planning/REQUIREMENTS.md#L76))
- **Prior art:** [sync-workspace.sh:16-19](wsx/scripts/workspace/sync-workspace.sh#L16-L19)
  ```bash
  if [[ "$CURRENT_BRANCH" != workspace/* && "$CURRENT_BRANCH" != personal/* ]]; then
      echo "error: sync-workspace.sh must be run from a workspace/* or personal/* branch (currently on '$CURRENT_BRANCH')." >&2
      echo "  For standard branches, use standard git merge or rebase workflows." >&2
      exit 1
  fi
  ```
  and [phase-worktree.sh:57-59](wsx/scripts/workspace/phase-worktree.sh#L57-L59)
- **Problem:** If a user or agent is working in Worktree A (`.worktrees/phase-48`) and runs `wsx sync`, wsx executes a `git merge` inside Worktree B (where `workspace` is checked out). If the merge conflicts, WSYNC-02 leaves git's conflict state in Worktree B. Worktree B is now dirty and blocked with merge conflict markers, while the caller remains in Worktree A. The caller cannot resolve or commit the merge in Worktree B without manually switching directories/contexts. Furthermore, Worktree A is not updated by `wsx sync` at all. This cross-worktree side effect violates state hermeticity and abandons conflicts in distant checkouts.
- **Better alternative / cost / milestone:** When executed from a worktree other than the workspace branch, refuse and display the path of the workspace worktree where sync must be run (preserving prior art's clean isolation), OR perform the merge out-of-tree (e.g. in a temporary detached worktree/index) and only fast-forward the workspace ref if conflict-free; if conflicted, abort without leaving any worktree dirty. Medium cost. Milestone 1.

---

### C4. WTREE-06 `wsx land` fails when workspace has advanced, and mutates another worktree
- **Requirement:** WTREE-06 "`wsx land <branch>` merges a finished branch back into the workspace branch with `--no-ff`, after proving the branch descends from the workspace branch. It runs where the workspace branch is checked out (as WSYNC-01), refuses a dirty tree there, and leaves git's conflict state on conflict. This closes the new → cut → land → rm loop without an orchestrator; devflow's Ship would call it instead of merging itself (moved from v2, 2026-10-20)" ([REQUIREMENTS.md:92](wsx/.planning/REQUIREMENTS.md#L92))
- **Prior art:** [DEVFLOW-SETUP.md:153](wsx/.planning/research/DEVFLOW-SETUP.md#L153)
  `Ship merges the phase branch into the base branch (git.rs merge --no-ff)`
- **Problem:** Two critical flaws:
  1. In Git terminology, A descends from B means B is an ancestor of A (`git merge-base --is-ancestor B A`). If the workspace branch has received *any* commit or merge (e.g. `wsx sync` merged an upstream commit, or a preceding feature landed) since `<branch>` was created, `<branch>` does *not* descend from the workspace tip. `wsx land` will falsely refuse legitimate completed branches.
  2. Running `wsx land` from a feature worktree executes `git merge --no-ff` inside the workspace worktree. If personal files conflict, the workspace worktree is left in an uncommitted conflict state while the developer remains in their feature worktree.
- **Better alternative / cost / milestone:** Prove the branch was forked from the workspace branch (via `refs/wsx/` records or `git merge-base` against the workspace branch's fork point), not that it descends from the workspace tip. If run from a feature worktree, require switching to the workspace worktree to land, or run the merge without leaving uncommitted conflicts in foreign checkouts. Low cost. Milestone 1.

---

### C5. WTREE-08 & GUARD-04 break external tools by requiring manual `wsx adopt` for unrecorded worktrees
- **Requirement:** WTREE-08 "`wsx adopt <branch>` records an existing branch made by hand (e.g. `git worktree add`) as if `wsx new` had made it, after checking it descends from the workspace branch. When the pre-commit guard (GUARD-04) refuses personal paths on an unrecorded branch that descends from the workspace branch, its message names `wsx adopt <branch>`" ([REQUIREMENTS.md:93](wsx/.planning/REQUIREMENTS.md#L93))
  and GUARD-04 ([REQUIREMENTS.md:127](wsx/.planning/REQUIREMENTS.md#L127))
- **Prior art:** [oracle-devflow/hooks/pre-commit:68-76](oracle-devflow/hooks/pre-commit#L68-L76)
  ```bash
  current_branch="$(git symbolic-ref --short HEAD 2>/dev/null || true)"
  if [ -n "$current_branch" ] && [[ "$current_branch" != workspace/* && "$current_branch" != personal/* ]]; then
      staged_personal="$(git diff --cached --name-only | grep -E '^(\.agents|\.bg-shell|\.claude|\.codex|\.gemini|\.omx|\.opencode|CLAUDE\.md|\.mcp\.json|skills/|skills-lock\.json|\.gsd|\.gsd-backups|\.gsd-id|\.gsd-worktrees|\.devflow|\.worktrees)' || true)"
      if [ -n "$staged_personal" ]; then
  ```
  and [wsx/bin/wsx:1164-1165](wsx/bin/wsx#L1164-L1165) (automatic worktree repair).
- **Problem:** The fixed constraints establish that wsx has no knowledge of GSD or external runtimes and must be useful standalone. External tools (e.g. `devflow start`, Claude Code subagents, IDE git integrations, or direct `git worktree add`) routinely create worktrees without invoking `wsx new`. When an agent or developer attempts to commit planning or workspace files on that branch, `GUARD-04` blocks the commit and demands manual execution of `wsx adopt <branch>`. Non-interactive tools and scripts crash on this failure. Prior art in `pre-commit` did not block planning commits on feature branches (enforcing the boundary at push time), and `bin/wsx` auto-repaired unrecorded worktrees automatically.
- **Better alternative / cost / milestone:** Instead of blocking commits on unrecorded branches that descend from the workspace branch, GUARD-04 should dynamically verify ancestry against the workspace branch (or auto-record the ref into `refs/wsx/`) without halting the commit. Low cost. Milestone 1.

---

### C6. PRCUT-05 fidelity check asserts total branch equality, making `--scope` fail by default
- **Requirement:** PRCUT-05 "Fidelity fails by default. The PR branch's diff against the base must equal the source branch's diff against the base, excluding personal and `modified` paths. Any path missing or different fails the cut, naming the unreplayed commits that touched it; unscoped code commits are the usual cause. `--allow-incomplete "<reason>"` overrides and prints the reason with the gap. A scenario with one scoped and one unscoped code commit must refuse. This is a deliberate difference: the oracle only warns" ([REQUIREMENTS.md:105](wsx/.planning/REQUIREMENTS.md#L105))
- **Prior art:** [cut-pr-branch.sh:267-275](wsx/scripts/workspace/cut-pr-branch.sh#L267-L275)
  ```bash
  TOUCHED_SET="$(for HASH in "${INCLUDED_HASHES[@]}"; do git diff-tree --no-commit-id --name-only -r "$HASH"; done | grep -v -E "$FORBIDDEN_REGEX" | sort -u || true)"
  CHANGED_SET="$(git diff --name-only "$BASE_REF" "$PR_BRANCH" | sort -u)"
  UNEXPLAINED="$(comm -13 <(echo "$TOUCHED_SET") <(echo "$CHANGED_SET") | grep -v '^$' || true)"
  if [ -n "$UNEXPLAINED" ]; then
      echo "error: '$PR_BRANCH' changes paths no replayed commit touched:" >&2
      echo "$UNEXPLAINED" | sed 's/^/  /' >&2
      git checkout "$ORIG_BRANCH" --quiet
      exit 1
  fi
  ```
- **Problem:** PRCUT-01 provides `--scope <s>` specifically to replay a subset of commits (e.g. scoping a PR to a specific component or phase) when the source branch contains commits from other scopes or unrelated work. But PRCUT-05 requires the PR branch diff to equal the source branch diff against base. When `--scope` is used on a branch with any other commits, the diffs will NEVER match. Thus, PRCUT-05 forces `--scope` to fail by default on every invocation unless overridden with `--allow-incomplete "<reason>"`. The oracle's fidelity check correctly verified that the PR branch did not contain unexplained touched paths (leak check) and warned if overlapping unreplayed commits modified the same file.
- **Better alternative / cost / milestone:** When `--scope` is passed, fidelity should assert that all files modified by the *replayed commits* match the source branch (and no unexplained files were modified), rather than asserting that the PR branch contains every code change present on the source branch. Low cost. Milestone 1.

---

### C7. Prior art prevented direct commits on PR-protected branches; requirements deferred this to v2
- **Requirement:** GUARD-05 "- **GUARD-05**: Refuse commits on PR-protected branches at commit time (devflow pre-commit)" ([REQUIREMENTS.md:191](wsx/.planning/REQUIREMENTS.md#L191))
  and GUARD-04 ([REQUIREMENTS.md:127](wsx/.planning/REQUIREMENTS.md#L127))
- **Prior art:** [oracle-devflow/hooks/pre-commit:44-53](oracle-devflow/hooks/pre-commit#L44-L53)
  ```bash
  case "$branch" in
      develop | main)
          echo "pre-commit: refusing to commit directly on '$branch'." >&2
          echo "  '$branch' is PR-protected — changes must be made through a pull request," >&2
          echo "  so this commit would be rejected at push time." >&2
          echo "  Create a branch first (your staged changes carry over):" >&2
          echo "      git switch -c feature/<name>   # or release/, fix/, docs/, ..." >&2
          exit 1
          ;;
  esac
  ```
- **Problem:** In git-flow repositories (`develop` / `main`), direct pushes to integration branches are rejected by GitHub rulesets. Prior art caught direct commits on `develop` and `main` at commit time, when changes were still staged and could be moved to a feature branch with a trivial `git switch -c`. In wsx, GUARD-04 only refuses personal files staged on `develop`/`main`, while refusing code commits on protected branches was relegated to v2 (GUARD-05). Developers and agents will commit code directly onto `develop` or `main`, succeed, and only discover at push time that their commits are rejected, requiring complicated reflog and branch surgery.
- **Better alternative / cost / milestone:** Move GUARD-05 into Milestone 1 pre-commit hook. It is a trivial branch-name check (`develop`, `main`, or configured integration branch) that saves painful recovery. Minimal cost (5 lines of Rust). Milestone 1.

---

### C8. DIV-03 copies devflow-specific permission checks for `scripts/` that cause false failures on general repos
- **Requirement:** DIV-03 "Every tracked hook and top-level script is checked for committed mode `100755` (or `120000`, a symlink to a tracked executable). The committed mode is what counts, not the working tree's" ([REQUIREMENTS.md:115](wsx/.planning/REQUIREMENTS.md#L115))
- **Prior art:** [check-workspace-divergence.sh:95-99](oracle-devflow/check-workspace-divergence.sh#L95-L99)
  ```bash
  while read -r MODE _ _ FILE; do
      if [ "$MODE" != "100755" ]; then
          problems+=("not committed executable ($MODE): $FILE (git update-index --chmod=+x $FILE)")
      fi
  done < <(git ls-files -s -- ':(glob)scripts/hooks/**' ':(glob)scripts/*.sh')
  ```
- **Problem:** The stated role of wsx divergence check (`DIV-01`) is verifying that differences between the workspace branch and the integration branch are declared in `.wsx.conf`. In devflow, `check-workspace-divergence.sh` bundled a devflow-specific lint checking that `scripts/*.sh` and `scripts/hooks/**` are committed `100755`. In general repositories (outside devflow), scripts directory layouts vary (`bin/`, `tools/`), and repositories routinely contain sourceable shell libraries, environment files, or python helpers committed with mode `100644`. Imposing a hard-coded executable check on all tracked scripts in wsx causes false divergence failures in repos that do not follow devflow's exact convention.
- **Better alternative / cost / milestone:** Restrict DIV-03 to hooks installed or managed by wsx (and hooks explicitly declared in `.wsx.conf`), or make script path patterns configurable rather than globally asserting on all scripts. Low cost. Milestone 1.

---

### C9. HARN-03 acceptance criteria cannot detect commit message body or committer regressions
- **Requirement:** HARN-03 "Every scenario asserts explicit expected outcomes, never only 'same as the oracle': exit code, ref names, commits compared as subject + author + author date + tree (SHAs aren't reproducible under signing), and files on disk. Signature presence is asserted separately." ([REQUIREMENTS.md:50](wsx/.planning/REQUIREMENTS.md#L50))
- **Prior art:** [cut-pr-branch.sh:236](wsx/scripts/workspace/cut-pr-branch.sh#L236)
  `git commit -C "$HASH" --no-verify --quiet`
- **Problem:** The shell oracle's `git commit -C "$HASH"` preserved the entire original commit object message verbatim (including multi-line commit descriptions, breaking change footers, issue references, `Co-authored-by` tags, and GSD metadata trailers), as well as author metadata. HARN-03 defines commit equality as `subject + author + author date + tree`. Because it checks only `subject` (the first line), an implementation of `wsx cut` that drops commit message bodies, truncates commit messages, or corrupts trailers will pass HARN-03 and all harness scenarios without detection. Furthermore, it does not assert on committer metadata.
- **Better alternative / cost / milestone:** Update HARN-03 to assert on full commit message content (including body and trailers) and committer name/email. Minimal cost. Milestone 1.

---

## SUSPECTED

### S1. CONF-07 reading manifest strictly from object database blinds `wsx check` during staged edits
- **Requirement:** CONF-07 "The authoritative manifest is `<workspace-branch tip>:.wsx.conf`, read from the object database, never the working tree. Guard, cut, check and status use it from any checkout, including clean PR branches that don't carry it. A repo is 'configured' when local git config has `wsx.sourceBranch`. Once configured, a missing or invalid manifest object fails closed" ([REQUIREMENTS.md:72](wsx/.planning/REQUIREMENTS.md#L72))
- **Prior art:** [check-workspace-divergence.sh:39](oracle-devflow/check-workspace-divergence.sh#L39)
  ```bash
  if [ -f .workspace-divergence ]; then
      while read -r KIND ENTRY _; do
  ```
- **Problem:** When working on the workspace branch, a developer adding a new personal file or tool edits `.wsx.conf` in their working tree or stages it in the index. Under CONF-07, `wsx check`, `wsx status`, and the pre-commit hook are mandated to read `<workspace-branch tip>:.wsx.conf` from the object database and "never the working tree". As a result, running `wsx check` to validate changes before committing will ignore the edited `.wsx.conf`, falsely failing. Furthermore, committing `.wsx.conf` together with a newly declared personal file will be rejected by the pre-commit hook because the tip commit does not yet carry the declaration.
- **Better alternative / cost / milestone:** When executing in a worktree where the workspace branch is checked out, read `.wsx.conf` from the index/working tree (falling back to `<workspace-branch tip>:.wsx.conf` when run from other worktrees or clean branches). Low cost. Milestone 1.

---

## NO CHANGE NEEDED

- **Keeping `git commit --no-verify` for replayed commits ([cut-pr-branch.sh:236](wsx/scripts/workspace/cut-pr-branch.sh#L236)):** Sound because replaying existing commits onto a clean base is history replay rather than new authoring; running `pre-commit` or `commit-msg` hooks per replayed commit re-triggers slow linters and test suites and causes failures on intermediate WIP commits or off-worktree guards, while safety is fully verified at the boundary by PRCUT-04 (post-cut range audit) and GUARD-01 (pre-push guard).
- **In-memory conflict prediction via `gix` before replay ([REQUIREMENTS.md:103](wsx/.planning/REQUIREMENTS.md#L103)):** Sound because pre-validating tree-level cherry-pick merges in memory catches upstream conflicts instantly without leaving partial git state or advancing refs.
- **Tracking created worktrees and cut PR branches under `refs/wsx/` ([REQUIREMENTS.md:91](wsx/.planning/REQUIREMENTS.md#L91)):** Sound because recording origin and replay mapping under dedicated ref namespaces avoids brittle branch name-parsing heuristics and survives local directory renames.
- **Unified personal-path manifest resolving from one source ([REQUIREMENTS.md:67](wsx/.planning/REQUIREMENTS.md#L67)):** Sound because unifying `FORBIDDEN_PATHS`, `.workspace-divergence`, and hook filters into a single resolved list directly eliminates the 3-way list drift present in prior art.
- **Raw-byte C-quoted path grammar with round-trip proptests ([REQUIREMENTS.md:66](wsx/.planning/REQUIREMENTS.md#L66)):** Sound because matching git's native C-quoting ensures paths with newlines, spaces, quotes, and non-UTF-8 bytes round-trip cleanly without parsing ambiguities.
