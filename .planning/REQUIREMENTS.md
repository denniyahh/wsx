# Requirements: wsx

**Defined:** 2026-10-19 (re-scope). The superseded copy-sync requirements are in git history at
commit `e5bfbcf` (`.planning/REQUIREMENTS.md`).
**Core Value:** wsx must never let a personal file reach a shared branch or PR, and must never
lose work while moving it between the workspace branch, phase worktrees and PR branches.

**The model.** Each repo has an **integration branch** (e.g. `develop`, `next`, `main`) and one
personal **workspace branch** (e.g. `workspace/<handle>`), which is the integration branch plus
tracked personal files: planning, agent config, tooling. Work happens in worktrees branched
**from the workspace branch**. Clean PR branches are cut from the integration branch by replaying
code commits and stripping personal paths. Guards stop personal paths from being pushed anywhere
except the workspace branch.

**Milestone 1 = Rust parity + hardened**, against the shell oracle:
- `scripts/workspace/{sync-workspace,phase-worktree,cut-pr-branch}.sh`
- devflow's `check-workspace-divergence.sh`
- the personal-path policy in devflow's `scripts/hooks/pre-push`

"Parity" means the same exit codes, refs, commits and trees. Exact wording does not count. Every
intended difference is a fixed oracle defect or a decision listed here, and has a harness
scenario. Research: `research/WORKSPACE-MODE.md`, `research/DEVFLOW-SETUP.md`. The earlier
STACK/PITFALLS research still applies to the git adapter and the harness.

## v1 Requirements

### Harness (black-box parity suite)

- [ ] **HARN-01**: The same scenario suite runs against the shell oracle and the Rust binary, selected per scenario. The oracle scripts are vendored under `scripts/workspace/` at a recorded devflow commit
- [ ] **HARN-02**: Every scenario is hermetic: HOME, XDG, `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` and hooks are isolated, signing is pinned locally, and the host's global hooks never fire. `LLVM_*` passes through so coverage sees the spawned binary
- [ ] **HARN-03**: Every scenario asserts explicit expected outcomes (exit code, refs, commit list, trees, files on disk), never only "same as the oracle". The oracle's outcome is recorded in the scenario, so the evidence survives the oracle's removal. Two classes of deliberate difference:
  - `fixed:` scenarios pass on Rust and fail on the oracle
  - `regression:` scenarios are deliberate refusals
- [ ] **HARN-04**: The suite runs from `cargo nextest` with `--no-tests=fail`, so coverage and mutation testing see it. `clippy.toml` disallows `std::env::set_var`/`remove_var`
- [ ] **HARN-05**: Every guard scenario has a known-positive control that must still be refused, and a clean control that must pass. A guard only ever observed refusing, or only ever observed passing, fails review

### Configuration and manifest

- [ ] **CONF-01**: `.wsx.conf` on the workspace branch declares `only <path>` (workspace-only) and `modified <path>` (a shared file deliberately changed). Paths are raw bytes. A path matches itself and anything under it, never a mere name prefix (`.planning` ≠ `.planningX`). Unknown line kinds are an error naming the line
- [ ] **CONF-02**: Built-in personal paths (agent-tool dirs, `.planning`, `.worktrees`, `.wsx.conf`, …) are defined once in wsx, and the manifest extends them. The guard, the cut, the divergence check and `status` all read that single resolved list (fixes the oracle's three diverging lists)
- [ ] **CONF-03**: The integration branch is resolved from `wsx.baseRef`, falling back to a chain that prefers `develop`, then `next`, then the remote default branch. It's never taken blindly from `origin/HEAD`
- [ ] **CONF-04**: The base remote (where the integration branch lives) and the push remote (where PR branches go) are configured separately, so fork layouts work (gsd-core: `upstream/next` base, `origin` push). Both default to `origin`
- [ ] **CONF-05**: The workspace branch is resolved from `wsx.sourceBranch`, falling back to the only local `workspace/*` or `personal/*` branch. Zero or several candidates is an error that lists them
- [ ] **CONF-06**: The phase branch template comes from `wsx.phaseBranchTemplate`, falling back to GSD's `git.phase_branch_template` and then `feature/phase-{phase}`. Worktrees live at `.worktrees/<name>`

### Workspace sync

- [ ] **WSYNC-01**: `wsx sync` merges `<base-remote>/<integration>` into the workspace branch with a real merge, never a rebase, and only from the workspace branch. Detached HEAD, another branch, or a dirty tree are refused
- [ ] **WSYNC-02**: Already up to date is a no-op with exit 0. A conflicted merge is left in git's normal conflict state with a message naming the files, and wsx never resolves it
- [ ] **WSYNC-03**: A local integration branch not checked out in any worktree is fast-forwarded to its remote. One checked out somewhere is left alone (parity)

### Worktrees

- [ ] **WTREE-01**: `wsx new --phase N` checks for a `### Phase N:` heading in ROADMAP.md, syncs the workspace (unless `--no-sync`, or when the main checkout isn't on the workspace branch, which it reports), and creates the phase branch and worktree **from the workspace branch**
- [ ] **WTREE-02**: Before creating anything, `wsx new` asserts the workspace branch tracks at least one personal path, and refuses otherwise
- [ ] **WTREE-03**: Re-running `wsx new` for an existing worktree fast-forwards it to the workspace branch, or refuses if that isn't a fast-forward. It never rewrites phase history. An existing branch with no worktree is attached
- [ ] **WTREE-04**: `wsx new <name>` (non-phase) creates `workspace/<name>`-style feature worktrees the same way
- [ ] **WTREE-05**: `wsx rm <name>` merges the worktree's planning records back into the workspace branch, then removes the worktree and branch. It refuses if the branch has commits that are neither merged into the workspace branch nor cut to a PR branch, unless `--force`

### PR cut

- [ ] **PRCUT-01**: `wsx cut` creates `<pr-branch>` from the freshly fetched `<base-remote>/<integration>` and replays commits onto it.
  - In phase mode (`--phase N`, or automatic on a phase branch) it replays only commits whose conventional-commit scope names phase N, wherever they were committed.
  - Otherwise it replays the branch's commits since its fork from the workspace branch.
- [ ] **PRCUT-02**: Personal paths (CONF-02) are removed from every replayed commit. Commits that touch only personal paths are dropped. Replayed commits keep the original author, date and message, and are signed per the user's git config
- [ ] **PRCUT-03**: Before writing anything, wsx predicts code conflicts in memory (gix merge). On a real conflict in a non-personal path, it aborts with the commit and paths named, and leaves the repo on the original branch with the original tree and no half-made PR branch. This is a deliberate difference: the oracle can leave a partial branch
- [ ] **PRCUT-04**: After replay, an audit over every commit in the PR range must find zero personal paths, or the cut fails. The audit uses `-z`/raw-byte names, so quoted non-ASCII names can't slip through
- [ ] **PRCUT-05**: A fidelity report fails if the PR branch changes a path no replayed commit touched, and warns for each path whose content differs from the source branch
- [ ] **PRCUT-06**: `wsx cut` never pushes. It returns to the original branch on success, failure, panic and Ctrl-C. A dirty working tree is refused up front
- [ ] **PRCUT-07**: Cut errors are never swallowed. A failed cherry-pick that isn't a predicted conflict aborts with git's error shown (fixes the oracle's `cherry-pick … || true`)

### Divergence check

- [ ] **DIV-01**: `wsx check` fails when the workspace branch differs from `<base-remote>/<integration>` in any way it hasn't declared. An added path must be under a built-in or `only` entry; a modified or deleted shared file must be declared `modified`
- [ ] **DIV-02**: A stale declaration fails too: an `only` path the integration branch now carries, or a `modified` path that no longer differs
- [ ] **DIV-03**: Every tracked hook and top-level script is checked for committed mode `100755`. The committed mode is what counts, not the working tree's
- [ ] **DIV-04**: The check runs against the local ref with no network, and says how stale that ref is relative to its last fetch

### Guards

- [ ] **GUARD-01**: The pre-push guard reads the real pushed refs from stdin (config-based hooks give every hook the full list). For each updated ref other than the workspace branch, it checks every commit's touched paths in the range: add, modify, delete and rename into a personal path. A new ref is bounded by `--not --remotes=<push-remote>`. This is a deliberate difference: the oracle checks only a fixed list of branch-name patterns and hard-codes `origin`
- [ ] **GUARD-02**: Path names are read NUL-separated, repo-relative and as raw bytes, so quoting and `diff.relative` can't hide a path (fixes the oracle's quoted-name bypass)
- [ ] **GUARD-03**: The guard fails closed once the repo is confirmed wsx-configured. An unreadable range (e.g. an unfetched remote object) or an internal error refuses the push, with the reason and the fix. Unconfigured repos exit 0. `WSX_SKIP=1` is the named bypass, and `--no-verify` is documented as disabling every guard at once
- [ ] **GUARD-04**: The pre-commit guard refuses personal paths staged on the integration branch, on `main`, and on PR branches cut by wsx. It allows them on the workspace branch and on branches created from it by `wsx new`, where GSD commits planning as it works. The push guard (GUARD-01) is the boundary for everything else

### Hook installation

- [ ] **HOOK-01**: `wsx install-hooks` registers wsx's pre-commit and pre-push hooks as repo-level config hooks (`hook.wsx-*.command` / `.event`, git ≥ 2.54). It writes no tracked file, overwrites no hook file, and runs alongside husky, `core.hooksPath` and global hooks (verified in `research/WORKSPACE-MODE.md` R2). Re-running is idempotent. `--uninstall` removes exactly what it added
- [ ] **HOOK-02**: Hook commands are plain program invocations that tolerate git's appended arguments. A scenario installs them and drives a real `git commit` and `git push` with a global `core.hooksPath` hook present, asserting both hooks ran and that a refusal is attributed to wsx's own message
- [ ] **HOOK-03**: On git < 2.54, `install-hooks` refuses with the version found and the minimum needed. It never half-installs

### Git adapter

- [ ] **GIT-01**: History-changing operations (merge, cherry-pick, branch and worktree creation, commit, fetch) go through the `git` binary, so the user's hooks, signing, merge drivers and credentials apply. Read-only analysis uses gix
- [ ] **GIT-02**: Child git processes get the repo-local variables `git rev-parse --local-env-vars` lists removed when they target another worktree, plus a null stdin. wsx resolves its own worktree and common dir correctly when a hook exports `GIT_DIR`/`GIT_INDEX_FILE`
- [ ] **GIT-03**: A reftable repo is refused with a clear message (gix 0.89 can't open it; a `regression:` scenario)
- [ ] **GIT-04**: A repo owned by another user is refused, as git does

### Migration

- [ ] **MIG-01**: `wsx migrate` converts an existing manifest to the new `.wsx.conf`:
  - 0.1.0 `.wsx.conf` `sync`/`pull`/`lww` entries become `only`
  - devflow `.workspace-divergence` lines carry over
  - `skip` lines are reported with a suggested `.gitignore` line, never guessed

  It shows a dry run first. With `--apply` it writes and commits the result on the workspace branch and removes the old file. Legacy grammars are parsed **only** inside `migrate`
- [ ] **MIG-02**: In a 0.1.0 copy-sync repo, `migrate --apply` removes copy-sync's leftovers: per-worktree `core.excludesFile` and `extensions.worktreeConfig` when wsx set them, the `exclude.wsx-*` files, and `<git-dir>/wsx/`. A leftover personal-file copy in a dev worktree is deleted only if its exact content exists in a commit on the workspace branch. Anything else is reported and kept
- [ ] **MIG-03**: Migration of the owner's real repos is verified on real state: devflow, gsd-core, gsd-pi, and the 0.1.0 projects (location to be given by the owner)

### Status and doctor

- [ ] **STAT-01**: `wsx status` shows the resolved integration, workspace, base remote and push remote; the workspace branch's ahead/behind against the integration branch; the divergence result; and each worktree with its branch, phase and whether it has been cut
- [ ] **STAT-02**: `wsx doctor` reports:
  - missing or non-executable hooks, and git < 2.54
  - GSD config that contradicts the model (`commit_docs` off, `use_worktrees` on, a phase template that doesn't match CONF-06)
  - an unfetched base ref
  - undeclared divergence

### Cutover and gates

- [ ] **REL-01**: `install.sh` installs the Rust binary. In the wsx and devflow repos, wsx replaces the shell scripts and the devflow hook's personal-path policy, with `.wsx.conf` migrated (MIG-01) at that point and not before
- [ ] **REL-02**: The vendored shell oracle stays until the harness passes on Rust with every recorded expectation intact, then is deleted. Python 0.1.0 `bin/wsx` is deleted from `develop`, with a `v0.1.0` tag kept
- [ ] **REL-03**: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` and `cargo deny check` pass. `cargo mutants` on the path matcher, commit classifier (scope and personal-path) and guard range logic leaves zero surviving mutants, apart from any listed as equivalent with a justification

## v2 Requirements (milestone 2: public baseline)

- **HOOK-04**: Hook fallback for git < 2.54 (chaining from a `core.hooksPath` hook to the displaced one)
- **GUARD-05**: Refuse commits on PR-protected branches at commit time (devflow pre-commit)
- **GUARD-06**: Off-worktree guard: refuse source committed outside the active phase's worktree (devflow `lint-phase-worktree.sh`)
- **AGNT-01..04**: Agent session integrations for Claude Code, Copilot, Codex and Antigravity. Whether they still need a per-turn action under the workspace model has to be re-derived: there are no copies to push, so the 2026-10-19 per-turn push decision may not apply
- **GEN-01**: Reftable support (gix or a git-binary read fallback)
- **DIST-01**: Versioned release binaries and an install documented for strangers
- **SIGN-01**: Release-key policy for tags and `main`, by fingerprint (devflow pre-push)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Copy-sync (0.1.0's model) | Dropped 2026-10-19; 0.1.0 is frozen and migrated away from (MIG-01, MIG-02) |
| Legacy manifest grammars outside `wsx migrate` | Owner decision 2026-10-19 |
| A narrow mirror of agent config across live worktrees | Revisit only if mid-phase config drift is felt |
| Resolving merge or cherry-pick conflicts for the user | wsx reports, git and the user resolve |
| macOS / Windows | Linux only for now |
| Interactive prompts | wsx runs inside git hooks |
| GSD plan-text lints (bashism scanner) | devflow/GSD-specific, not workspace management |
| Gemini CLI adapter | Antigravity replaces it |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|

**Coverage:**
- v1 requirements: 49 total
- Mapped to phases: 0 (roadmap pending)
- Unmapped: 49

---
*Last updated: 2026-10-19 after re-scoping to the workspace-branch model*
