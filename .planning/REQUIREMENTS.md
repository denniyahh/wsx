# Requirements: wsx

**Defined:** 2026-10-19 (re-scope). The superseded copy-sync requirements are in git history at
commit `e5bfbcf` (`.planning/REQUIREMENTS.md`).
**Core Value:** wsx must never let a personal file reach a shared branch or PR, and must never
lose work while moving it between the workspace branch, feature worktrees and PR branches.

**Boundary with devflow (owner intent, 2026-10-19).** wsx manages and maintains the **repo**:
- the workspace branch and its manifest
- syncing it with the integration branch
- worktrees and branches
- PR cuts, divergence checks, guards and hooks

devflow manages **AI agents and GSD**: phases, ROADMAP/STATE, GSD config, agent sessions, gates,
ship approval and versioning. Three rules follow:
1. wsx has no knowledge of GSD, devflow or any agent runtime, and is fully useful in a repo that
   has none of them.
2. devflow uses wsx for repo operations (through IF-01) rather than reimplementing them.
3. Neither tool implements a capability the other owns.

**The model.** Each repo has an **integration branch** (e.g. `develop`, `next`, `main`) and one
personal **workspace branch** (e.g. `workspace/<handle>`), which is the integration branch plus
tracked personal files: planning, agent config, tooling. Work happens in worktrees branched
**from the workspace branch**. Clean PR branches are cut from the integration branch by replaying
code commits and stripping personal paths. Guards stop personal paths from being pushed anywhere
except the workspace branch.

**Milestone 1 = Rust parity + hardened**, against the shell oracle, minus its devflow/GSD-specific parts
(the ROADMAP lookup in `phase-worktree.sh`, phase auto-detection in `cut-pr-branch.sh`):
- `scripts/workspace/{sync-workspace,phase-worktree,cut-pr-branch}.sh`
- devflow's `check-workspace-divergence.sh`
- the personal-path policy in devflow's `scripts/hooks/pre-push`

"Parity" means the same exit codes, refs, commits and trees. Exact wording does not count. Every
intended difference is a fixed oracle defect or a decision listed here, and has a harness
scenario. Research: `research/WORKSPACE-MODE.md`, `research/DEVFLOW-SETUP.md`. The earlier
STACK/PITFALLS research still applies to the git adapter and the harness.

## v1 Requirements

### Harness (black-box parity suite)

- [ ] **HARN-01**: The same scenario suite runs against the shell oracle and the Rust binary, selected per scenario. The oracle (devflow's `sync-workspace.sh`, `phase-worktree.sh`, `cut-pr-branch.sh`, `check-workspace-divergence.sh`, and the personal-path policy from `scripts/hooks/pre-push`) is vendored under `tests/oracle/` on `develop` at a recorded devflow commit, MIT-attributed. Every path it hard-codes is adapted by one documented patch, and each script has a self-test scenario proving it runs unmodified against a fixture
- [ ] **HARN-02**: Every scenario is hermetic: HOME, XDG, `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` and hooks are isolated, signing is pinned locally, and the host's global hooks never fire. `LLVM_*` passes through so coverage sees the spawned binary
- [ ] **HARN-03**: Every scenario asserts explicit expected outcomes, never only "same as the oracle": exit code, ref names, commits compared as subject + author + author date + tree (SHAs aren't reproducible under signing), and files on disk. Signature presence is asserted separately. The oracle's outcome is recorded in the scenario, so the evidence survives the oracle's removal. Two classes of deliberate difference:
  - `fixed:` scenarios pass on Rust and fail on the oracle
  - `regression:` scenarios are deliberate refusals
- [ ] **HARN-04**: The suite runs from `cargo nextest` with `--no-tests=fail`, so coverage and mutation testing see it. `clippy.toml` disallows `std::env::set_var`/`remove_var`
- [ ] **HARN-05**: Every guard scenario has a known-positive control that must still be refused, and a clean control that must pass. A guard only ever observed refusing, or only ever observed passing, fails review
- [ ] **HARN-06**: The scenario set covers, each with a control:
  - merge commits (integration merged into a feature branch, a conflict resolved by adding a personal file)
  - squash-merged PRs
  - a fork layout (base `upstream`, push `origin`, no `origin/<integration>`)
  - scoped and unscoped commits in one cut
  - a repo with no GSD, devflow or agent files at all (IF-02)
  - non-ASCII, newline and non-UTF-8 path names
  - an existing PR branch with extra commits

### Configuration and manifest

- [ ] **CONF-01**: `.wsx.conf` on the workspace branch declares `only <path>` (workspace-only) and `modified <path>` (a shared file deliberately changed). Paths are raw bytes, written C-quoted the way git quotes them when they contain special bytes (newline, quote, backslash, non-ASCII), so any git path round-trips (proptest: newline, non-UTF-8, spaces, prefix boundaries). A path matches itself and anything under it, never a mere name prefix (`.planning` ≠ `.planningX`). Unknown line kinds are an error naming the line
- [ ] **CONF-02**: The only built-in personal paths are wsx's own: `.wsx.conf` and the worktree directory. Every other personal path (agent dirs, `.planning`, `scratch`, …) is declared in the manifest; `wsx init` presets can write the common ones (MIG-01). The guard, the cut, the divergence check and `status` all read that single resolved list (fixes the oracle's three diverging, agent-specific lists). A repo-specific file like root `CLAUDE.md` is personal only if declared: devflow declares it, wsx doesn't
- [ ] **CONF-03**: The integration branch is resolved from `wsx.baseRef`, falling back to a chain that prefers `develop`, then `next`, then the remote default branch. It's never taken blindly from `origin/HEAD`
- [ ] **CONF-04**: The base remote (where the integration branch lives) and the push remote (where PR branches go) are configured separately, so fork layouts work (gsd-core: `upstream/next` base, `origin` push). Both default to `origin`
- [ ] **CONF-05**: The workspace branch is resolved from `wsx.sourceBranch`, which `wsx init`/`wsx migrate` write. With no config, wsx falls back to the only local `workspace/*` or `personal/*` branch, ignoring branches recorded as made by `wsx new`. Zero or several candidates is an error that lists them (gsd-core has `personal/workspace` and `personal/workspace-backup`)
- [ ] **CONF-06**: Branch names are chosen by the caller: a human, or an orchestrator such as devflow. wsx reads no phase concept or branch template from any other tool. Worktrees live under `wsx.worktreeDir` (default `.worktrees/`), named after the branch
- [ ] **CONF-07**: The authoritative manifest is `<workspace-branch tip>:.wsx.conf`, read from the object database, never the working tree. Guard, cut, check and status use it from any checkout, including clean PR branches that don't carry it. A repo is "configured" when local git config has `wsx.sourceBranch`. Once configured, a missing or invalid manifest object fails closed

### Workspace sync

- [ ] **WSYNC-01**: `wsx sync` merges `<base-remote>/<integration>` into the workspace branch with a real merge, never a rebase, and only from the workspace branch. Detached HEAD, another branch, or a dirty tree are refused
- [ ] **WSYNC-02**: Already up to date is a no-op with exit 0. A conflicted merge is left in git's normal conflict state with a message naming the files, and wsx never resolves it
- [ ] **WSYNC-03**: A local integration branch not checked out in any worktree is fast-forwarded to its remote. One checked out somewhere is left alone (parity)
- [ ] **WSYNC-04**: wsx records the integration commit it last merged. If the integration branch was rewritten (the new tip doesn't descend from the recorded one), `wsx sync` refuses and explains, rather than merging a second lineage that later cuts would replay

### Worktrees

- [ ] **WTREE-01**: `wsx new <branch>` syncs the workspace (unless `--no-sync`, or when the main checkout isn't on the workspace branch, which it reports) and creates the branch and its worktree **from the workspace branch**. It never reads ROADMAP.md or any other planning file; validating that a phase exists is the caller's job (devflow)
- [ ] **WTREE-02**: Before creating anything, `wsx new` asserts the workspace branch tracks at least one personal path, and refuses otherwise
- [ ] **WTREE-03**: Re-running `wsx new` for an existing worktree reports its path and exits 0. It fast-forwards only when the branch has no commits of its own, and never fails because work has started. An existing branch with no worktree is attached. `wsx update <name>` brings a started branch up to date by merging the workspace branch into it (a real merge; refuses on conflict and leaves git's conflict state)
- [ ] **WTREE-04**: `wsx rm <branch>` refuses a dirty worktree. It also refuses unless both:
  - the branch is merged into the workspace branch (so its personal records are kept; merging is done by the user or the orchestrator, e.g. devflow's Ship)
  - every non-personal change has landed on the integration branch, matched by patch-id or content so squash merges count

  Then it removes the worktree and branch. `--force` overrides and prints what's discarded. wsx doesn't do the merge itself in milestone 1
- [ ] **WTREE-05**: `wsx new` and `wsx cut` record what they create under `refs/wsx/`: branch origin, cut source tip, PR branch, replay mapping. Guard, `rm` and `status` identify branches by these records, never by name patterns

### PR cut

- [ ] **PRCUT-01**: `wsx cut` creates `<pr-branch>` from the freshly fetched `<base-remote>/<integration>` and replays commits onto it.
  - With `--scope <s>` it replays only commits whose conventional-commit scope is `s` or `s-NN`, wherever they were committed. devflow passes the phase number; wsx has no phase concept and never infers a scope from the branch name.
  - Otherwise it replays the branch's commits since its fork from the workspace branch.
  - Merge commits: a merge whose merged-in side is the integration or workspace branch is skipped, because its content is already on the base. If such a merge carries its own resolution changes in non-personal paths, the cut refuses and names it. Any other merge commit refuses.
- [ ] **PRCUT-02**: Personal paths (CONF-02) are removed from every replayed commit. Commits that touch only personal paths are dropped. `modified` paths are workspace-only edits and are never replayed. A rename or copy from a personal path into a non-personal path refuses the cut, because the content would leak under a clean name. Replayed commits keep the original author, date and message, and are committed through `git` (so signed per the user's config, as the oracle's `git commit -C` already does)
- [ ] **PRCUT-03**: Before writing anything, wsx replays each selected commit in memory (gix), with personal paths stripped, onto the evolving base tree, and refuses on any predicted conflict, naming the commit and paths. All failures, predicted or not (including git errors, panics and Ctrl-C), leave the original branch and tree checked out, and the target ref at its pre-cut value. A scenario injects a failure mid-replay. This is a deliberate difference: the oracle can leave a partial branch
- [ ] **PRCUT-04**: After replay, an audit over every commit in the PR range must find zero personal paths, or the cut fails. The audit uses `-z`/raw-byte names, so quoted non-ASCII names can't slip through
- [ ] **PRCUT-05**: Fidelity fails by default. The PR branch's diff against the base must equal the source branch's diff against the base, excluding personal and `modified` paths. Any path missing or different fails the cut, naming the unreplayed commits that touched it; unscoped code commits are the usual cause. `--allow-incomplete "<reason>"` overrides and prints the reason with the gap. A scenario with one scoped and one unscoped code commit must refuse. This is a deliberate difference: the oracle only warns
- [ ] **PRCUT-06**: `wsx cut` never pushes. It returns to the original branch on success, failure, panic and Ctrl-C. A dirty working tree is refused up front
- [ ] **PRCUT-07**: Cut errors are never swallowed. A failed cherry-pick that isn't a predicted conflict aborts with git's error shown (fixes the oracle's `cherry-pick … || true`)
- [ ] **PRCUT-08**: If the target PR branch already exists, the cut refuses, unless it's the recorded result of a previous cut of this source with no commits added since. `--force` prints the commits it would discard and keeps them under `refs/wsx/backup/` (fixes the oracle's unconditional `git branch -f`)
- [ ] **PRCUT-09**: `wsx cut` runs the divergence check first and refuses if it fails, so an undeclared new path can't be replayed into a PR

### Divergence check

- [ ] **DIV-01**: `wsx check` fails when the workspace branch differs from `<base-remote>/<integration>` in any way it hasn't declared. An added path must be under a built-in or `only` entry; a modified or deleted shared file must be declared `modified`
- [ ] **DIV-02**: A stale or redundant declaration fails too: an `only` path the integration branch now carries, a `modified` path that no longer differs, or a declaration already covered by a built-in
- [ ] **DIV-03**: Every tracked hook and top-level script is checked for committed mode `100755` (or `120000`, a symlink to a tracked executable). The committed mode is what counts, not the working tree's
- [ ] **DIV-04**: The check runs against the local ref with no network, and says how stale that ref is relative to its last fetch

### Guards

- [ ] **GUARD-01**: The pre-push guard reads the real pushed refs from stdin (config-based hooks give every hook the full list). For each updated ref other than the workspace branch, both checks must pass:
  - **(a)** the pushed tip's tree contains no personal path
  - **(b)** every commit in the range touches no personal path. Merge commits are diffed against their first parent, and renames count in both directions. The range is bounded by the remote ref's old value and `<base-remote>/<integration>`, never by remote workspace refs

  Scenarios: a new feature branch with no commits of its own (tip carries `.planning`) must refuse; a merge whose conflict resolution adds `.planning/STATE.md` must refuse; the fork layout must work. Both oracle bypasses are reproduced in `reviews/m1-ws-adversarial/SUMMARY.md`. This is a deliberate difference: the oracle checks a fixed list of branch patterns, hard-codes `origin`, and misses both cases
- [ ] **GUARD-02**: Path names are read NUL-separated, repo-relative and as raw bytes, so quoting and `diff.relative` can't hide a path (fixes the oracle's quoted-name bypass)
- [ ] **GUARD-03**: The guard fails closed once the repo is configured (CONF-07). An unreadable range (e.g. an unfetched remote object) or an internal error refuses the push, with the reason and the fix. Unconfigured repos exit 0. `WSX_SKIP=1` is the named bypass, and `--no-verify` is documented as disabling every guard at once
- [ ] **GUARD-04**: The pre-commit guard refuses personal paths staged on the integration branch, on `main`, and on PR branches cut by wsx. It allows them on the workspace branch and on branches recorded as created by `wsx new` (WTREE-05), where tools commit planning as they work. PR branches are identified by their cut record, not their name. The push guard (GUARD-01) is the boundary for everything else

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
  - devflow `.workspace-divergence` lines carry over, and devflow's built-in list is written out as explicit `only` lines, since wsx has no agent built-ins (CONF-02)
  - `skip` lines are reported with a suggested `.gitignore` line, never guessed

  It shows a dry run first. With `--apply` it writes and commits the result on the workspace branch and removes the old file. Legacy grammars are parsed **only** inside `migrate`. A repo with no manifest (gsd-core, gsd-pi) uses `wsx init`. It lists every difference from the integration branch and requires each path to be classified explicitly (`--only`, `--modified`, or an edited proposal file). Opt-in presets (`--preset agents` for agent-tool dirs, `--preset gsd` for `.planning`/`.gsd*`) classify the common ones as `only`; they are data in wsx, not behaviour. Any unclassified path aborts with no change. `init`/`migrate` write `wsx.sourceBranch` and `wsx.baseRef`
- [ ] **MIG-02**: In a 0.1.0 copy-sync repo, `migrate --apply` removes copy-sync's leftovers: per-worktree `core.excludesFile` and `extensions.worktreeConfig` when wsx set them, the `exclude.wsx-*` files, and `<git-dir>/wsx/`. A leftover personal-file copy in a dev worktree is deleted only if its exact content exists in a commit on the workspace branch. Anything else is reported and kept
- [ ] **MIG-03**: Each of the owner's repos (devflow, wsx, gsd-core, gsd-pi, and the 0.1.0 projects, location to be given by the owner) has a recorded expected result, and `wsx migrate --verify`/`wsx init --dry-run` against its real state produces it before anything is applied
- [ ] **MIG-04**: `migrate --apply` refuses while that repo's shell tooling (`scripts/workspace/`, devflow's hooks) still reads `.workspace-divergence`, unless `--keep-legacy`. That keeps the old file alongside until REL-01 retires the tooling, so the cut never loses its `only` list and the divergence check never silently skips

### Status and doctor

- [ ] **STAT-01**: `wsx status` shows the resolved integration, workspace, base remote and push remote; the workspace branch's ahead/behind against the integration branch; the divergence result; and each worktree with its branch and whether it has been cut and landed
- [ ] **STAT-02**: `wsx doctor` reports repo-level problems only: missing or non-executable hooks, git < 2.54, an unfetched base ref, undeclared divergence. Checking GSD or agent config is devflow's job

### Interface and boundary

- [ ] **IF-01**: Every command has `--json` output with a versioned, documented schema, plus documented, stable exit codes, so an orchestrator (devflow) can drive wsx without parsing prose. A contract test pins the schema
- [ ] **IF-02**: wsx reads no GSD, devflow or agent-runtime files or config: no `ROADMAP.md`/`STATE.md` contents, `.planning/config.json`, `devflow.toml`, or `.claude`/`.codex` settings. A personal path is only a path to wsx. This is enforced two ways: a source-scan test, and a scenario running every command in a repo with none of those files

### Development setup (adopted from devflow, owner decision 2026-10-19)

- [ ] **DEV-01**: `scripts/check.sh {all,fmt,clippy,test,deps}` is the single gate entry point that CI, hooks and humans call. A missing tool is a hard failure
- [ ] **DEV-02**: `rust-toolchain.toml` pins an exact rustc (1.99.0, the host's version) with clippy and rustfmt
- [ ] **DEV-03**: Root `CLAUDE.md` is rewritten for Rust (still minimal and public-safe), including devflow's repo traps: `nextest --no-tests=fail -E 'test(=path)'` one test per command; `rg -c` prints nothing on zero matches; zsh has no `PIPESTATUS`; prefer a named Rust test over a shell pipeline
- [ ] **DEV-04**: `CONTRIBUTING.md` is rewritten for Rust, with devflow's AI change acceptance bar (a regression test that fails first, a boundary assertion, proof it failed for the intended reason, full gates, independent review) and its four rejected test shapes
- [ ] **DEV-05**: A tracked `.gitconfig`, pulled in with `git config --local include.path ../.gitconfig`, sets SSH signing for commits and tags and the git-flow branch names. It contains nothing personal
- [ ] **DEV-06**: Container parity. `.devcontainer/devcontainer.json` pins an exact image tag. `scripts/check-in-container.sh` runs `check.sh` in that image and is worktree-aware (it bind-mounts the common git dir) with per-checkout cargo volumes. CI jobs run in the same image, and an image-tag parity check fails CI if the two drift. Requires a Docker daemon (rootless Docker 29.9 on the host). devflow's CPU-pinned sequential job is not ported

### Supply chain (global policy, 2026-10-19)

- [ ] **SUP-01**: `cargo deny check` (advisories, licenses, bans, sources) runs on every push and PR and on a weekly schedule. It is advisory, not a required check. The weekly job takes effect once merged to `main`, the default branch. No second scanner (one ecosystem; osv-scanner would read the same advisory data)
- [ ] **SUP-02**: `.github/dependabot.yml` updates GitHub Actions and the devcontainer image weekly, grouped, with `target-branch: develop`. No per-crate update PRs. Dependabot alerts are on (enabled 2026-10-19). Automatic security-fix PRs stay off, because they target `main` and would bypass `develop`
- [ ] **SUP-03**: The repo runs its own secret scan: a tracked setup script registers a config-based pre-commit hook running `gitleaks git --pre-commit --staged --redact`, alongside any global hook. Server-side push protection (already on) is the backstop for `--no-verify`
- [ ] **SUP-04**: `Cargo.lock` is committed. CI actions are pinned to commit SHAs. Scanner versions are pinned in one install script. A missing scanner is a hard failure, never a skip
- [ ] **SUP-05**: Each scanner is proven to bite before it counts as enabled: a planted fake secret is refused by the hook, and a known-vulnerable or disallowed crate in a scratch copy fails `cargo deny`. Both controls are cleaned up afterwards

### Cutover and gates

- [ ] **REL-01**: `install.sh` installs the Rust binary. In the wsx and devflow repos, wsx replaces the shell scripts and the devflow hook's personal-path policy, with `.wsx.conf` migrated (MIG-01) at that point and not before
- [ ] **REL-02**: The vendored shell oracle (`tests/oracle/`) stays until the harness passes on Rust with every recorded expectation intact, then is deleted. Python 0.1.0 `bin/wsx` is deleted from `develop`, with a `v0.1.0` tag kept
- [ ] **REL-03**: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` and `cargo deny check` pass. `cargo mutants` on the path matcher, commit classifier (scope and personal-path) and guard range logic leaves zero surviving mutants, apart from any listed as equivalent with a justification
- [ ] **REL-04**: CI runs test, clippy and fmt on every PR to `develop` and `main`. Once it has run, those check names are added as required checks (up to date before merge) to the `develop-merge-or-squash` and `main-squash-only` rulesets created 2026-10-19

## v2 Requirements (milestone 2: public baseline)

- **HOOK-04**: Hook fallback for git < 2.54 (chaining from a `core.hooksPath` hook to the displaced one)
- **GUARD-05**: Refuse commits on PR-protected branches at commit time (devflow pre-commit)
- **HYG-01**: Conventional-commit `commit-msg` hook installed by `install-hooks` (devflow's validator, generalised)
- **WTREE-06**: `wsx land <branch>`: merge a finished feature branch back into the workspace branch (`--no-ff`, ancestry proven), so the loop closes without an orchestrator. devflow's Ship would call it instead of merging itself
- **GEN-01**: Reftable support (gix or a git-binary read fallback)
- **DIST-01**: Versioned release binaries and an install documented for strangers
- **SIGN-01**: Release-key policy for tags and `main`, by fingerprint (devflow pre-push)
- **WTREE-07**: Initialise submodules in new worktrees
- **WSYNC-05**: A recovery command for a rewritten integration branch (WSYNC-04 only refuses)

## Out of Scope

| Feature | Reason |
|---------|--------|
| Copy-sync (0.1.0's model) | Dropped 2026-10-19; 0.1.0 is frozen and migrated away from (MIG-01, MIG-02) |
| Legacy manifest grammars outside `wsx migrate` | Owner decision 2026-10-19 |
| A narrow mirror of agent config across live worktrees | Revisit only if mid-task config drift across worktrees is felt |
| Resolving merge or cherry-pick conflicts for the user | wsx reports, git and the user resolve |
| macOS / Windows | Linux only for now |
| Interactive prompts | wsx runs inside git hooks |
| GSD plan-text lints (bashism scanner), STATE.md drift warning | devflow's domain |
| Phases: ROADMAP validation, phase naming, scope inference from phase branches | devflow's domain. devflow passes a branch name to `wsx new` and a scope to `wsx cut` |
| Off-worktree guard (phase source committed outside the phase worktree) | devflow's domain: it needs the active phase from STATE.md |
| GSD/agent config checks in `doctor` | devflow's domain |
| Agent session integrations (Claude Code, Copilot, Codex, Antigravity) | devflow's domain; the workspace model has no copies to push |
| Ship approval, versioning and tags | devflow's domain (gate-driven). Revisit only if wsx gains release management |
| Changing devflow's code to call wsx (`devflow start` → `wsx new`, Ship → `wsx land`) | devflow's own roadmap; wsx supplies IF-01 |
| Gemini CLI adapter | Antigravity replaces it |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|

**Coverage:**
- v1 requirements: 69 total
- Mapped to phases: 0 (roadmap pending)
- Unmapped: 69

---
*Last updated: 2026-10-19 after the wsx/devflow boundary review*
