# Roadmap: wsx

## Overview

Milestone 1 replaces devflow's workspace-branch shell scripts with a Rust `wsx` that is never less
safe than them and is better wherever REQUIREMENTS.md lists an improvement. The roadmap is built as
horizontal layers (owner decision). It starts with the development foundation and the oracle
harness. Next come the git adapter and the repo model. Then it adds one command layer at a time:
divergence check, pre-push guard, sync, worktrees, the PR cut, commit guard and hook installation,
migration, and status. A release gate and the cutover close the milestone. Phases run strictly in
numeric order. Each command layer with an oracle counterpart (Phases 5-10) enables the Rust side of
its harness scenarios and passes them before the next layer starts. The oracle is a floor on safety
outcomes only. **Depends on** names the technical dependency; the numeric order is the execution
order.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

Decimal phases appear between their surrounding integers in numeric order.

- [ ] **Phase 1: Development foundation** - Pinned toolchain, one gate script, container parity, CI with required checks, and supply-chain scanners proven to bite
- [ ] **Phase 2: Oracle harness** - Hermetic scenario suite over the vendored shell oracle and the Rust binary, with recorded expectations and the hard-case matrix
- [ ] **Phase 3: Git adapter** - Correct repo resolution from any worktree or hook; git binary for writes, gix for reads; reftable and foreign-owner refusal
- [ ] **Phase 4: Repo model** - `.wsx.conf` grammar and one resolved model of branches, remotes and personal paths
- [ ] **Phase 5: Divergence check and command contract** - `wsx check`, plus the versioned JSON and exit-code contract every later command joins
- [ ] **Phase 6: Pre-push guard** - Fail-closed push guard over the real pushed refs, every commit in the range and the tip tree
- [ ] **Phase 7: Workspace sync** - `wsx sync` by real merge from any worktree, with integration-rewrite detection
- [ ] **Phase 8: Worktree lifecycle** - `wsx new`, `update`, `land` and `rm`, with origin records under `refs/wsx/`
- [ ] **Phase 9: PR cut replay** - Atomic `wsx cut` replay in a temporary worktree with personal paths stripped
- [ ] **Phase 10: PR cut refusal gates** - Leak audit, commit-level fidelity, PR-branch protection and the divergence pre-check
- [ ] **Phase 11: Commit guard and hook installation** - `wsx install-hooks` beside existing hooks; personal-path and protected-branch commit refusal
- [ ] **Phase 12: Migration and init** - `wsx migrate` and `wsx init` for every owner repo, dry run first
- [ ] **Phase 13: Status and doctor** - Whole-repo state and repo-level health, readable by people and orchestrators
- [ ] **Phase 14: Release gate** - Lints, zero surviving mutants, and every scenario green on Rust before anything outside this repo changes
- [ ] **Phase 15: Cutover and retirement** - wsx repo, then devflow's repo tooling, on the Rust binary; oracle and Python 0.1.0 deleted

## Phase Details

### Phase 1: Development foundation

**Goal**: The Rust workspace builds, tests and gates the same way on the host, in the pinned container and in CI, and every supply-chain scanner is proven to bite
**Depends on**: Nothing (first phase)
**Requirements**: DEV-01, DEV-02, DEV-03, DEV-04, DEV-05, DEV-06, SUP-01, SUP-02, SUP-03, SUP-04, SUP-05, HARN-04, REL-04
**Success Criteria** (what must be TRUE):
  1. `scripts/check.sh all` passes on the host with the pinned toolchain (rustc 1.99.1 with clippy and rustfmt), and through `scripts/check-in-container.sh` in the pinned devcontainer image when run from a linked worktree. With a required tool removed from `PATH`, the matching subcommand exits non-zero naming the tool instead of skipping
  2. The test and lint gates bite: a nextest filter that matches no test fails `scripts/check.sh test` (`--no-tests=fail`), and a planted `std::env::set_var` call fails `scripts/check.sh clippy`
  3. CI on a PR to `develop` runs test, clippy and fmt in the same image tag as the devcontainer, and a deliberately mismatched tag fails the image-tag parity check. After that first run, the `develop-merge-or-squash` and `main-squash-only` rulesets list test, clippy and fmt as required, up-to-date checks
  4. Each scanner bites before it counts as enabled. A planted fake secret is refused by the repo's own config-based gitleaks pre-commit hook (registered by a tracked setup script, alongside global hooks). A known-vulnerable or disallowed crate in a scratch copy fails `cargo deny check`. Both controls are removed afterwards. `cargo deny` runs as an advisory job on push, PR and a weekly schedule; `Cargo.lock` is committed; CI actions are pinned to SHAs; scanner versions are pinned in one install script; `.github/dependabot.yml` updates Actions and the devcontainer image weekly, grouped, against `develop`
  5. A contributor reading the root `CLAUDE.md` and `CONTRIBUTING.md` gets the Rust workflow, devflow's repo traps and the AI change acceptance bar with its four rejected test shapes. After `git config --local include.path ../.gitconfig`, commits and tags are SSH-signed and the git-flow branch names are set, and the tracked `.gitconfig` contains nothing personal

**Plans**: 7/7 plans executed
**Wave 1**
- [x] 01-01-PLAN.md — Tracer: `wsx --version` crate gated end to end by `scripts/check.sh`; rustc 1.99.1 pin; set_var ban

**Wave 2** *(blocked on Wave 1 completion)*
- [x] 01-02-PLAN.md — Pinned tool installs, `deny.toml`, `deps` target; cargo-deny control
- [x] 01-05-PLAN.md — Public-safe `.gitconfig`, `scripts/setup-repo.sh` gitleaks hook; secret and signing controls

**Wave 3** *(blocked on Wave 2 completion)*
- [x] 01-03-PLAN.md — Pinned devcontainer Dockerfile and worktree-aware `check-in-container.sh`

**Wave 4** *(blocked on Wave 3 completion)*
- [x] 01-04-PLAN.md — CI (Test, Clippy, Format) in the pinned image, image-parity check, advisory deny, Dependabot

**Wave 5** *(blocked on Wave 4 completion)*
- [x] 01-06-PLAN.md — CLAUDE.md and CONTRIBUTING.md rewritten for Rust

**Wave 6** *(blocked on Wave 5 completion)*
- [x] 01-07-PLAN.md — Full gate suite, PR-cut rehearsal, owner checkpoint (push, PR, rulesets)

### Phase 2: Oracle harness

**Goal**: Any wsx behaviour can be pinned as a hermetic scenario that runs against the vendored shell oracle or the Rust binary, with explicit expected outcomes that outlive the oracle
**Depends on**: Phase 1
**Requirements**: HARN-01, HARN-02, HARN-03, HARN-05, HARN-06
**Success Criteria** (what must be TRUE):
  1. `tests/oracle/` holds devflow's `sync-workspace.sh`, `phase-worktree.sh`, `cut-pr-branch.sh`, `check-workspace-divergence.sh` and the `pre-push` personal-path policy from a recorded devflow commit. They are MIT-attributed and adapted by one documented patch, and each script's self-test scenario runs it against a fixture and passes. One scenario definition runs against the oracle, the Rust binary or both, selected per scenario, from `cargo nextest`
  2. Scenarios are hermetic. With a global hook and global git config planted where the host's would be, a scenario's commits and outcomes are unchanged and the hook never fires, while a control scenario wired to the planted config sees it fire. Signing is pinned locally, and `cargo llvm-cov` reports coverage for code that only the spawned binary executes
  3. Every scenario states its expected outcome explicitly: exit code, ref names, commits as full message + author + author date + committer name/email + tree, signature presence, and files on disk. The oracle's outcome is recorded in the scenario itself. Changing any one expected field makes the scenario fail, and the runner rejects a scenario whose only expectation is "same as the oracle"
  4. The runner enforces the difference classes and the guard controls. A `fixed:` scenario fails the suite if it passes on the oracle, or, once its Rust side is enabled, if it fails on Rust. A `regression:` scenario must be a refusal. A guard scenario lacking either a known-positive control or a clean control is rejected
  5. Each HARN-06 case is a scenario with a control, run against the oracle with its outcome recorded wherever the oracle implements that behaviour; a case it lacks is marked Rust-only. The cases are: merge commits (including a conflict resolved by adding a personal file), squash-merged PRs, fork remotes (base `upstream`, push `origin`, no `origin/<integration>`), scoped and unscoped commits in one cut, a repo with no GSD, devflow or agent files, non-ASCII, newline and non-UTF-8 path names, and an existing PR branch with extra commits. The runner reports by name every scenario whose Rust side is not yet enabled

**Plans**: TBD

### Phase 3: Git adapter

**Goal**: wsx opens the right repository from any worktree or hook, reads it with gix, and makes every history change through the `git` binary so the user's hooks, signing, merge drivers and credentials apply
**Depends on**: Phase 2
**Requirements**: GIT-01, GIT-02, GIT-03, GIT-04
**Success Criteria** (what must be TRUE):
  1. A commit and a merge made through wsx's git layer run the user's own commit hooks and are signed per the user's config, in a scenario with both configured. Read-only queries (merge-base, ahead/behind, tree diff, range walk) answer from gix with the same results `git` gives on the same fixtures
  2. Started inside a hook that exports `GIT_DIR` and `GIT_INDEX_FILE` from a linked worktree, wsx resolves that worktree and the common dir correctly. A git child aimed at a sibling worktree runs with the variables `git rev-parse --local-env-vars` lists removed and a null stdin. A control run with the removal disabled shows the scenario detects the leak
  3. A reftable repository is refused with a message naming reftable (a `regression:` scenario), while the same history in a files-backend repository opens (control)
  4. A repository owned by another user is refused, matching git's own refusal on that repository

**Plans**: TBD

### Phase 4: Repo model

**Goal**: Every command resolves one repo model: the integration branch, the workspace branch, the base and push remotes, and a single personal-path list read from the workspace branch tip's `.wsx.conf`
**Depends on**: Phase 3
**Requirements**: CONF-01, CONF-02, CONF-03, CONF-04, CONF-05, CONF-07
**Success Criteria** (what must be TRUE):
  1. A proptest shows that any git path (newline, quote, backslash, spaces, non-ASCII, non-UTF-8) written C-quoted to `.wsx.conf` parses back to the same bytes. A declared `.planning` matches `.planning/x` and not `.planningX`, and an unknown line kind is an error naming the line
  2. With `origin/HEAD` at `main` and a `develop` branch present, the integration branch resolves to `develop` (then `next`, then the remote default), and `wsx.baseRef` overrides the chain. With fork remotes, the base remote resolves to `upstream` and the push remote to `origin`; both default to `origin`
  3. `wsx.sourceBranch` names the workspace branch. Without it, the only local `workspace/*` or `personal/*` branch is used, and zero or several candidates (`personal/workspace` with `personal/workspace-backup`) is an error that lists them
  4. The personal-path list comes from `<workspace tip>:.wsx.conf` in the object database. An uncommitted edit to the working-tree file changes nothing, and a clean PR-branch checkout without the file resolves the same list. The only built-ins are `.wsx.conf` and the worktree directory: an undeclared `.planning` or `CLAUDE.md` is not personal, and once declared it is
  5. A repo with `wsx.sourceBranch` set but a missing or invalid manifest object fails closed with the reason; a repo without `wsx.sourceBranch` is reported as not configured

**Plans**: TBD

### Phase 5: Divergence check and command contract

**Goal**: `wsx check` reports, offline, every way the workspace branch differs from the integration branch that the manifest doesn't declare, and it sets the versioned JSON and exit-code contract every later command must join
**Depends on**: Phase 4
**Requirements**: DIV-01, DIV-02, DIV-03, DIV-04, IF-01, IF-02
**Success Criteria** (what must be TRUE):
  1. An added path outside every built-in and `only` entry, or a modified or deleted shared file not declared `modified`, fails `wsx check` naming the path, and a fully declared workspace branch passes (control). Each case records the oracle's outcome, and Rust never passes an undeclared difference the oracle catches. With the remote unreachable, the check runs against the local ref and reports how long ago that ref was last fetched
  2. Stale declarations fail, each named: an `only` path the integration branch now carries, a `modified` path that no longer differs, and a declaration already covered by a built-in
  3. With `core.hooksPath` at a tracked directory, a hook committed as `100644` fails even when it is executable on disk, and a `120000` symlink to a tracked executable passes. A non-hook `scripts/*.sh` committed `100644` does not fail (a recorded difference from the oracle)
  4. `wsx check --json` matches a versioned, documented schema, and its exit codes are documented. A contract test enumerates the command tree and fails for any command without a pinned schema or whose output drifts from it (control: a field change without a version bump fails), so every later command must join the contract
  5. A source-scan test fails on any read of `ROADMAP.md`/`STATE.md` contents, `.planning/config.json`, `devflow.toml` or `.claude`/`.codex` settings (control: a planted read trips it), and every command runs in a repo that has none of those files

**Plans**: TBD

### Phase 6: Pre-push guard

**Goal**: No push carries a personal path to any branch except the workspace branch, including the pushes the oracle lets through, and the guard fails closed once a repo is configured
**Depends on**: Phase 4
**Requirements**: GUARD-01, GUARD-02, GUARD-03
**Success Criteria** (what must be TRUE):
  1. Through a real `git push` with a config-based pre-push hook, the guard reads the pushed refs from stdin. It refuses each known-positive control: a new feature branch with no commits of its own whose tip carries `.planning`; a merge whose conflict resolution adds `.planning/STATE.md`; a range that adds and then removes a personal file; a rename into or out of a personal path. A clean PR branch and the workspace branch itself push (clean controls)
  2. A personal path with a quoted, non-ASCII or non-UTF-8 name, or one hidden by `diff.relative`, is refused. The oracle's quoted-name bypass and the two bypasses in `reviews/m1-ws-adversarial/SUMMARY.md` are `fixed:` scenarios that fail on the oracle and pass on Rust
  3. With fork remotes (base `upstream`, push `origin`, no `origin/<integration>`), a clean branch pushes and a branch carrying a personal path is refused. The range is bounded by the remote ref's old value and `<base-remote>/<integration>`, never by remote workspace refs
  4. In a configured repo, an unreadable range (an unfetched remote object), an injected internal error, or a missing manifest object refuses the push with the reason and the fix. An unconfigured repo exits 0. `WSX_SKIP=1` lets a push through and says so, and the docs state that `--no-verify` disables every guard at once
  5. Every pre-push scenario has a refused known-positive control and a passing clean control, and replacing the guard's decision with "always pass" or with "always refuse" makes at least one scenario fail

**Plans**: TBD

### Phase 7: Workspace sync

**Goal**: `wsx sync` keeps the workspace branch current with the integration branch by real merges, from any worktree, without leaving a checkout stuck mid-merge
**Depends on**: Phase 4
**Requirements**: WSYNC-01, WSYNC-02, WSYNC-03, WSYNC-04
**Success Criteria** (what must be TRUE):
  1. Run from any worktree, `wsx sync` merges `<base-remote>/<integration>` into the workspace branch in the worktree that has it checked out, with a fixed message and the previous workspace tip as first parent (never a rebase). Run from the workspace checkout, the result matches the oracle's recorded merge. When already up to date it exits 0 and creates no commit
  2. It refuses, naming the fix and changing nothing, when the workspace worktree is dirty or when the workspace branch is not checked out anywhere
  3. On a conflict run from another worktree, the merge is aborted so no checkout is left mid-merge, and the conflicting files and the command to run in the workspace worktree are reported. Run in the workspace worktree, git's conflict state is left with the files named. wsx resolves nothing
  4. A local integration branch checked out nowhere is fast-forwarded to its remote. One checked out in a worktree is left alone and reported stale with that worktree's path (a `fixed:` scenario against the oracle's silent skip)
  5. After the integration branch is rewritten so its tip no longer descends from the last merged commit, `wsx sync` refuses and explains, with no merge made, while an ordinary advance (control) merges

**Plans**: TBD

### Phase 8: Worktree lifecycle

**Goal**: A user can run the whole new → update → land → rm loop on feature worktrees branched from the workspace branch, with no orchestrator
**Depends on**: Phase 7
**Requirements**: WTREE-01, WTREE-02, WTREE-03, WTREE-04, WTREE-05, WTREE-06, CONF-06
**Success Criteria** (what must be TRUE):
  1. `wsx new <branch>` syncs first (a refused sync is reported, not fatal; `--no-sync` skips it). It then creates the branch from the workspace branch, and its worktree under `wsx.worktreeDir` (default `.worktrees/`) named after the branch exactly as given, matching the recorded `phase-worktree.sh` outcome minus its ROADMAP lookup. It refuses before creating anything if the workspace branch tracks no personal path, and it reads no ROADMAP or planning file
  2. Re-running `wsx new` for an existing worktree reports its path and exits 0, fast-forwarding only a branch with no commits of its own, and an existing branch without a worktree is attached. `wsx update <name>` merges the workspace branch into a started branch, and on a conflict refuses and leaves git's conflict state
  3. `wsx land <branch>` merges a branch made from the workspace branch, including one made before the last sync, into the workspace branch with `--no-ff` and a fixed message. A branch with no commit that is on the workspace branch and not on the integration branch is refused
  4. `wsx rm <branch>` refuses a dirty worktree, a branch not merged into the workspace branch, and a branch whose non-personal changes have not landed on the integration branch. A squash-merged PR counts as landed, so the worktree and branch are removed. `--force` overrides and prints what it discards
  5. `wsx new` records each branch's origin under `refs/wsx/`. A hand-made branch with the same name pattern has no record and is not treated as wsx-made, and a `wsx new` branch under `workspace/` or `personal/` is not counted as a workspace-branch candidate

**Plans**: TBD

### Phase 9: PR cut replay

**Goal**: `wsx cut` builds a PR branch from the freshly fetched integration branch by replaying the right commits with personal paths stripped, atomically and without touching the user's checkout
**Depends on**: Phase 8
**Requirements**: PRCUT-01, PRCUT-02, PRCUT-03, PRCUT-06, PRCUT-07
**Success Criteria** (what must be TRUE):
  1. With `--scope s`, only commits whose conventional-commit scope is `s` or `s-NN` are replayed, wherever they were committed. Without it, the branch's commits since its fork from the workspace branch are replayed. The PR branch is the given name or `<source-branch>-pr` verbatim: a `workspace/<handle>-x` source keeps every hyphenated word, pinning devflow backlog 999.146. A merge of the integration or workspace branch is skipped, a merge carrying its own non-personal resolution changes refuses naming it, and any other merge refuses
  2. Personal paths are removed from every replayed commit, and personal-only commits are dropped. A change to a `modified` path replays as that commit's own delta and is listed in the cut report. A rename or copy from a personal path into a non-personal path refuses the cut. Replayed commits keep author, date and message, are signed per the user's config, and are made with `--no-verify`
  3. Replay runs with `git cherry-pick` in a temporary detached worktree under the common git dir. A custom merge driver that resolves a conflict lets the cut succeed. An unresolved conflict refuses naming the commit and paths, with git's error shown (the oracle's `cherry-pick … || true` is a `fixed:` scenario)
  4. A failure injected mid-replay (git error, panic, Ctrl-C) leaves the target ref at its pre-cut value, and the temporary worktree is removed, or pruned by the next wsx run (the oracle's partial branch is a `fixed:` scenario)
  5. The user's branch, index and uncommitted changes are identical before and after a cut, and `wsx cut` never pushes. Uncommitted tracked changes in the source worktree refuse the cut unless `--allow-dirty`, which lists them. The cut records its source tip, PR branch and replay mapping under `refs/wsx/`

**Plans**: TBD

### Phase 10: PR cut refusal gates

**Goal**: A cut that would leak a personal path, silently drop code, overwrite an existing PR branch's work, or replay undeclared divergence refuses instead, with the target ref untouched
**Depends on**: Phase 9, Phase 5
**Requirements**: PRCUT-04, PRCUT-05, PRCUT-08, PRCUT-09
**Success Criteria** (what must be TRUE):
  1. After replay, an audit over every commit in the PR range reads names NUL-separated as raw bytes and must find zero personal paths. A personal path re-introduced after stripping, including a quoted non-ASCII name, is caught and the cut refuses with the target ref unchanged
  2. A source branch with one scoped and one unscoped code commit refuses a `--scope` cut, naming the left-out commit, and `--allow-incomplete "<reason>"` lets it through and prints the reason with the gap. A `--scope` cut from a branch that also carries other landed work passes. A PR branch that changes a path no replayed commit touched fails. The oracle only warns, so these are `fixed:` scenarios
  3. An existing target PR branch refuses the cut, unless it is the recorded result of a previous cut of this source with no commits added since. With extra commits on it, `--force` prints the commits it would discard and keeps them under `refs/wsx/backup/` (the oracle's unconditional `git branch -f` is a `fixed:` scenario)
  4. An undeclared new path on the workspace branch makes `wsx cut` refuse before any replay, because the divergence check runs first

**Plans**: TBD

### Phase 11: Commit guard and hook installation

**Goal**: One command arms wsx's commit and push guards in any repo next to the user's existing hooks, and a commit that puts personal paths on a shared or PR branch, or lands directly on a protected branch, is refused while still staged
**Depends on**: Phase 6, Phase 8, Phase 9
**Requirements**: GUARD-04, GUARD-05, HOOK-01, HOOK-02, HOOK-03
**Success Criteria** (what must be TRUE):
  1. `wsx install-hooks` registers the pre-commit and pre-push hooks as repo config hooks (`hook.wsx-*.command` / `.event`), writing no tracked file and no hook file. A second run changes nothing, and `--uninstall` leaves the repo config exactly as it was before install. On git < 2.54 it refuses, naming the version found and the minimum, and installs nothing
  2. With a global `core.hooksPath` hook present, and with husky in a variant, a real `git commit` and `git push` run both the user's hook and wsx's. The hook commands tolerate git's appended arguments, and a refusal is attributed to wsx's own message
  3. Staging a personal path is refused on the integration branch, on `main`, and on a PR branch identified by its cut record (known-positive controls), and allowed on the workspace branch and on a `wsx new` branch (clean controls). A hand-made branch named like a PR branch is judged by its record, not its name
  4. A commit made directly on the integration branch or on the remote's default branch is refused while staged, naming `git switch -c <branch>` as the fix. `wsx.protectedBranches` replaces the list, and an empty value lets a commit on `main` through (control)
  5. Replacing either pre-commit check's decision with "always pass" or with "always refuse" makes at least one scenario fail

**Plans**: TBD

### Phase 12: Migration and init

**Goal**: Each of the owner's repos can move onto the new `.wsx.conf` from 0.1.0's manifest, from devflow's `.workspace-divergence`, or from no manifest at all, with a dry run first and no file lost
**Depends on**: Phase 5
**Requirements**: MIG-01, MIG-02, MIG-03, MIG-04
**Success Criteria** (what must be TRUE):
  1. `wsx migrate` shows a dry run first. 0.1.0 `sync`/`pull`/`lww` entries become `only`; `.workspace-divergence` lines carry over, with devflow's built-in list written out as explicit `only` lines; `skip` lines are reported with a suggested `.gitignore` line. `--apply` commits the result on the workspace branch, removes the old file, and writes `wsx.sourceBranch` and `wsx.baseRef`. Legacy grammars are rejected everywhere outside `migrate`
  2. `wsx init` in a repo with no manifest lists every difference from the integration branch and aborts with no change while any path is unclassified. `--only`, `--modified`, an edited proposal file, and the opt-in `--preset agents` and `--preset gsd` classify paths
  3. In a 0.1.0 copy-sync fixture, `migrate --apply` removes the per-worktree `core.excludesFile` and `extensions.worktreeConfig` that wsx set, the `exclude.wsx-*` files and `<git-dir>/wsx/`. A leftover personal-file copy is deleted only when its exact content is in a workspace-branch commit; a copy that differs is reported and kept (control)
  4. `migrate --apply` refuses while `scripts/workspace/` or devflow's hooks still read `.workspace-divergence`, unless `--keep-legacy`, which keeps the old file alongside
  5. devflow, wsx, gsd-core and gsd-pi each have a recorded expected result that `wsx migrate --verify` or `wsx init --dry-run` reproduces against the repo's real state without writing anything. The 0.1.0 projects on the owner's work PC get the same check there

**Plans**: TBD

### Phase 13: Status and doctor

**Goal**: A person or an orchestrator can see a repo's whole wsx state and its repo-level problems without parsing prose
**Depends on**: Phase 5, Phase 8, Phase 9, Phase 11
**Requirements**: STAT-01, STAT-02
**Success Criteria** (what must be TRUE):
  1. `wsx status` shows the resolved integration branch, workspace branch, base remote and push remote; the workspace branch's ahead/behind against the integration branch; the divergence result; and each worktree with its branch and whether it has been cut and landed, taken from `refs/wsx/` records rather than names
  2. `wsx doctor` reports each repo-level problem from a fixture that has it: missing wsx hooks, non-executable hook files, git < 2.54, an unfetched base ref, and undeclared divergence. It reports nothing for a healthy repo (control), and it reads no GSD or agent config
  3. `wsx status --json` and `wsx doctor --json` pass the command contract test, so an orchestrator reads the resolved branches, remotes and worktree states from JSON

**Plans**: TBD

### Phase 14: Release gate

**Goal**: The evidence that the Rust binary is never less safe than the oracle, and better where listed, is complete and stands on its own before anything outside this repo changes
**Depends on**: Phase 13
**Requirements**: REL-03
**Success Criteria** (what must be TRUE):
  1. `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` and `cargo deny check` pass on the milestone tip
  2. `cargo mutants` on the path matcher, the commit classifier (scope and personal-path) and the guard range logic leaves zero surviving mutants, apart from any listed as equivalent with a justification
  3. No scenario is left oracle-only: the runner reports zero scenarios without their Rust side enabled, every scenario passes on Rust with its recorded expectation intact, and every `fixed:` scenario still fails on the oracle

**Plans**: TBD

### Phase 15: Cutover and retirement

**Goal**: The wsx repo, and then devflow's repo tooling, run on the Rust binary, and the shell oracle and Python 0.1.0 are gone
**Depends on**: Phase 14, Phase 12
**Requirements**: REL-01, REL-02
**Success Criteria** (what must be TRUE):
  1. `install.sh` installs the Rust binary, and the installed `wsx` reports the Rust build
  2. In the wsx repo, `.wsx.conf` is migrated at this point and not before, `scripts/workspace/` is retired, and wsx's hooks are installed. A push of a branch carrying `.planning` is refused by wsx, and a clean PR branch pushes
  3. Only after criterion 2 holds, devflow's shell scripts and its `pre-push` personal-path policy are replaced by wsx, with devflow's `.wsx.conf` migrated, and the same refused and clean push controls pass there. devflow's own Rust code is unchanged
  4. `tests/oracle/` is deleted only after the full harness passes on Rust with every recorded expectation intact. Python `bin/wsx` is deleted from `develop`, and the `v0.1.0` tag still resolves to the 0.1.0 code

**Plans**: TBD

## Progress

**Execution Order:**
Phases execute in numeric order: 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11 → 12 → 13 → 14 → 15

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Development foundation | 7/7 | In Progress | - |
| 2. Oracle harness | 0/TBD | Not started | - |
| 3. Git adapter | 0/TBD | Not started | - |
| 4. Repo model | 0/TBD | Not started | - |
| 5. Divergence check and command contract | 0/TBD | Not started | - |
| 6. Pre-push guard | 0/TBD | Not started | - |
| 7. Workspace sync | 0/TBD | Not started | - |
| 8. Worktree lifecycle | 0/TBD | Not started | - |
| 9. PR cut replay | 0/TBD | Not started | - |
| 10. PR cut refusal gates | 0/TBD | Not started | - |
| 11. Commit guard and hook installation | 0/TBD | Not started | - |
| 12. Migration and init | 0/TBD | Not started | - |
| 13. Status and doctor | 0/TBD | Not started | - |
| 14. Release gate | 0/TBD | Not started | - |
| 15. Cutover and retirement | 0/TBD | Not started | - |

## Backlog

### Phase 999.1: Deduplicate CI runs on PR branches (BACKLOG)

**Goal:** ci.yml and deny.yml trigger on every `push:` and on `pull_request`, and the concurrency group keys on `github.ref`, so a push to a PR branch starts two runs (refs/heads/<branch> and refs/pull/N/merge) that never cancel each other: same results, double runner minutes. Limit `push` to main and develop. Source: external adversarial review, 2026-10-22 (agy).
**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.2: Cache pinned tools in container checks (BACKLOG)

**Goal:** scripts/check-in-container.sh runs `install-tools.sh install cargo-nextest cargo-deny` in a throwaway container, so every run downloads both again (about 17 s) and the check fails offline. Cache the verified binaries in a per-checkout volume, keeping the sha256 and version checks. Source: external adversarial review, 2026-10-22 (agy).
**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with /gsd-review-backlog when ready)

### Phase 999.3: Per-checkout devcontainer target volume (BACKLOG)

**Goal:** .devcontainer/devcontainer.json names its target volume `wsx-target-${localWorkspaceFolderBasename}`, so two checkouts with the same folder name (two clones named wsx, or phase-1 worktrees of different repos) share build output, the stale-artifact hazard check-in-container.sh avoids with a full-path hash. Key it on the full checkout path. Source: external adversarial review, 2026-10-22 (codex).
**Requirements:** TBD
**Plans:** 0 plans

Plans:
- [ ] TBD (promote with /gsd-review-backlog when ready)
