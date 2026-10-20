# devflow setup: what it is, and what wsx should take from it

**Analyzed:** 2026-10-19, against `~/Github/devflow` on `workspace/denniyahh` (d69c268) and GitHub
(`gh api`). Primary source is devflow's own `.planning/user/DEV-SETUP-CHECKLIST.md` (553 lines,
last trued up 2026-09-13). Each claim below that matters to a decision was re-checked against the
live files today. Claims marked *(checklist)* are taken from that document and were not re-verified.

**Why this exists:** the owner's goal is for wsx to eventually replace some or all of these
customizations, and to be developed with the same guardrails meanwhile. Section 3 feeds wsx's
**product** requirements. Section 4 feeds **how wsx itself is built** (the roadmap). Section 5 is
the status matrix.

---

## 1. The setup in one paragraph

devflow is a public Rust repo (`main` ← `develop` ← `feature/*`, git-flow) worked on almost entirely
by AI agents. The main checkout sits on a **personal workspace branch** (`workspace/denniyahh`,
pushed to the public origin) that is `develop` plus tracked personal files: `.planning/`, agent
configs, a graphify snapshot, and workspace-only scripts and hooks. Phase work happens in
`.worktrees/phase-N` on `feature/phase-N`, **branched from the workspace branch**, so GSD can commit
plans alongside code. At PR time `cut-pr-branch.sh` replays only code commits onto a clean branch
from `develop` and strips personal paths. Tracked repo hooks (`core.hooksPath=scripts/hooks`)
enforce the boundary and mirror CI. CI runs inside the same pinned container the hooks use.
GitHub rulesets protect `develop` and `main`.

## 2. Inventory (as built)

### 2.1 Branch model and the workspace tier
| Piece | What it does |
|---|---|
| `workspace/<handle>` branch | `develop` + personal files, all tracked. Synced from `develop` by **merge** (`sync-workspace.sh`), never rebase, so ancestry holds |
| `.workspace-divergence` | Declares every difference from `develop`: `only <path>` (workspace-only) and `modified <path>` (deliberately changed shared file). Built-ins: `.planning/`, `graphify-out/`, agent-tool paths, the file itself |
| `check-workspace-divergence.sh` (pre-push) | Fails if the branch differs from `develop` in an undeclared way, if a declaration is stale, or if a hook/script is not committed `100755` |
| `phase-worktree.sh N` | The one command for a phase worktree. Checks the phase heading exists in ROADMAP.md, syncs the workspace, branches `feature/phase-N` **from the workspace branch**, asserts `.planning/` is tracked on the base, and is idempotent (fast-forward or refuse) |
| `cut-pr-branch.sh` | Replays commits onto `<branch>-pr` from `origin/develop`. Phase mode selects commits by conventional-commit scope `(N)`/`(N-PP)`, strips forbidden paths, then audits zero leaks and fidelity |
| `lint-phase-worktree.sh` (pre-commit) | Refuses `crates/**` committed outside the active phase's worktree; only warns for phase docs. Escape hatch is a named variable, **not `--no-verify`** |

### 2.2 Hooks (`scripts/hooks/`, tracked)
- **pre-commit:** chains to the global hook first, so gitleaks survives `core.hooksPath`. Refuses
  commits on `develop`/`main` (no override). Refuses agent-tool paths on non-workspace branches
  (`.planning/` is allowed there; the boundary is enforced at push). Refuses bash-only syntax in
  PLAN.md `<automated>` blocks (zsh executors). Runs the off-worktree guard and `pre-commit.d/` drop-ins.
- **pre-push:** scrubs repo-local `GIT_*` using `git rev-parse --local-env-vars` (incident 999.37:
  a worktree push flipped the repo to `core.bare=true`). Enforces the release key by **fingerprint**.
  Personal-path policy scans **every commit in the range with `git log --name-only`**, which catches
  add-then-remove and renames into a forbidden path. Runs the divergence check, the full CI check in
  the pinned container, and `pre-push.d/` drop-ins (which get no stdin).
- **commit-msg:** Conventional Commits, no trailing period, warns past 72 chars, chains to the global hook.
- **post-commit:** warns, never edits, when a `*-SUMMARY.md` lands without STATE.md moving, or when a
  dev-setup file changes without DEV-SETUP-CHECKLIST.md.

### 2.3 Git policy, signing, GitHub
- A tracked `.gitconfig` is pulled in by `include.path ../.gitconfig`. It sets SSH signing for
  commits and tags, the git-flow names (`main`/`develop`/`feature/`; never `git flow init -d`) and
  aliases. Nothing personal goes in it, because the repo is public.
- Two keys: `user.signingkey` = agent key (local), `devflow.releaseSigningKey` = maintainer key,
  enforced for tags and `main` by fingerprint. Both share one email, so only the fingerprint
  distinguishes them.
- GitHub rulesets: `develop-merge-or-squash` (PR required, 4 required checks, strict up-to-date,
  no force-push or delete, no bypass) and `main-squash-only`. Squash title = PR title. Rebase
  merging is off. Head branches are deleted on merge *(checklist; ruleset names re-verified)*.

### 2.4 Verification stack
- `scripts/check.sh {all,fmt,clippy,test,build,deps}` is the single entry point that CI, pre-push
  and humans all call. `check-in-container.sh` runs the same thing in the pinned devcontainer
  image. It is worktree-aware (it bind-mounts the common git dir) and uses per-checkout cargo
  volumes, because a shared volume caused false RED and false GREEN.
- CI: Test, Clippy and Format are required, plus `Build + test in devcontainer` from a second
  workflow. A sequential 2-CPU job and a deps job (`cargo deny` + `cargo machete`, tool versions
  pinned in one script) are advisory. Container jobs must declare `shell: bash`. An image-tag
  parity check runs in CI.
- Pins: an exact `rust-toolchain.toml` (1.97.1) and an exact devcontainer image tag.
  `clippy.toml` disallows `std::env::set_var`/`remove_var`.
- **AI Change Acceptance** (CONTRIBUTING): every change needs:
  - a regression test that fails before the change
  - a boundary assertion
  - proof it failed for the intended reason
  - full gates passing
  - independent review

  Four test shapes are rejected outright.
- **Repo traps** (CLAUDE.md):
  - `cargo test <filter>` exits 0 when nothing matches. Use `nextest --no-tests=fail -E 'test(=path)'`, one test per command.
  - `rg -c` prints nothing on zero matches.
  - zsh has no `PIPESTATUS`.
  - Prefer a named Rust test over a shell pipeline.

### 2.5 Agent / GSD layer
- Root `CLAUDE.md` is deliberately minimal (34 lines, public-safe). Generic rules live in the global
  `~/.config/agents/AGENTS.md`. **Project `.claude/` and `.codex/` configs were eliminated**
  (Phase 47 Retrospective D1). Graphify hooks are global.
- GSD config, notable settings:
  - `commit_docs: true`
  - `git.phase_branch_template: feature/phase-{phase}`
  - `workflow.use_worktrees: false`. Required on Claude Code: executor worktrees fork from
    `origin/HEAD` (claude-code#44965), and the sentinel's 10-minute TTL breaks parallel dispatch (gsd-core#4317)
  - `granularity: fine`
  - `windows_enforce: true` (the broken-windows ship gate)
  - `tdd_mode: true`
  - mempalace enabled
  - `review.default_reviewers`: codex, opencode, antigravity
  - model overrides for small agents
- **`~/.gsd/defaults.json` does not layer under a project config.** Copying keys from it believing
  they're "already in effect" has bitten devflow before.
- Upstream GSD defects are filed on gsd-core's tracker. Issues live in GitHub Issues; `999.x`
  entries in ROADMAP.md are the backlog.

### 2.6 Known gap (devflow's own words)
Everything machine-global (`~/.claude/`, `~/.config/agents/`, `~/.config/git/hooks/`, GitHub
rulesets) is **not version-controlled anywhere**. The checklist names this the first task of a
"capture and replicate" project.

---

## 3. Lessons for the wsx product

These are what devflow learned the hard way that bear directly on wsx's behaviour. "Req" is the
requirement it confirms or changes.

| # | devflow lesson | wsx today | Req | Proposal |
|---|---|---|---|---|
| P1 | The push guard must scan **every commit's touched paths** (`git log --name-only`), because endpoint diffs miss add-then-remove **and renames into a guarded path** | GUARD-02 covers add-then-remove only | GUARD-02 | Add renames to the acceptance tests |
| P2 | Staged names must be read `-z --no-relative`. Quoted non-ASCII names and `diff.relative` both hid paths from a prefix match | GUARD-01 covers quoting | GUARD-01 | Add `diff.relative` to the tests |
| P3 | Scrub repo-local git env with `git rev-parse --local-env-vars`, git's own list, rather than a hand-kept list | GIT-02 says "scrubbed" | GIT-02, HARN-02 | Name the mechanism |
| P4 | **Chain, don't shadow.** A repo or global `core.hooksPath` replaces hooks wholesale; devflow chains to the displaced hook | wsx *skips arming* when `core.hooksPath` is set (GUARD-07 parity). **On the author's own machine the global `core.hooksPath` means wsx's guard never arms in any repo** | GUARD-07, HOOK-01 | Keep parity in M1. Make HOOK-01 (git ≥ 2.54 config hooks, which coexist with `hooksPath`) a top milestone 2 priority, and have `doctor` say *why* the guard is inactive and how to chain it |
| P5 | Hooks committed non-executable run silently as nothing (`commit-msg` was inert for weeks) | Shims are written 0755 | GUARD-07 | Have `doctor` check the hook's mode, not just that it exists |
| P6 | **Named escape hatch, never `--no-verify`.** Agents reach for `--no-verify`, which disables every guard at once | `WSX_SKIP=1` is the hatch | GUARD-04 | Keep. Document that `--no-verify` also kills the guard |
| P7 | **Two ways to keep personal files off team branches.** devflow *tracks* them on a workspace branch and strips at PR time; wsx *copies* them into ignored paths. devflow's way exists because **GSD executors fork from committed HEAD and must see `.planning/`**, and because GSD commits planning docs as it works | wsx's model is incompatible with GSD `commit_docs: true` (found 2026-10-19: ignored `.planning/` makes GSD's commits fail or force-add) | none yet | **New milestone 2+ item:** wsx must support the GSD workflow it is meant to replace. Either a workspace-branch mode (track, declare and strip at PR, as devflow does) or a GSD integration where commits land on the source branch. Decide at milestone 2 |
| P8 | Worktrees created by tools wsx doesn't drive (GSD or harness executor worktrees, `devflow start`) need the personal files too | HOOK-03 (repo-level post-checkout) | HOOK-03 | Raise priority; add GSD and devflow worktrees as named cases |
| P9 | Warn, never edit, in post-commit-type hooks. A hook dirtying tracked files mid-phase breaks scope fences | wsx writes only untracked files | (design rule) | Keep; record as a constraint |
| P10 | Refuse commits on PR-protected branches at commit time, not push time | out of wsx scope | none | Note for "other developer repo needs" (milestone 3 exploration) |
| P11 | The divergence manifest, cut-PR, phase-worktree and sync-workspace scripts are the customizations the owner wants wsx to absorb eventually | ported as shell to `scripts/workspace/` | none yet | Milestone 3 candidates: `wsx pr-cut`, `wsx divergence check`, `wsx phase-worktree`. Keep the shell ports as the oracle, the same pattern as milestone 1 |

## 4. Lessons for how wsx is built (roadmap inputs)

| # | devflow practice | wsx now | Recommendation | When |
|---|---|---|---|---|
| D1 | `scripts/check.sh` as the single gate entry point (fmt, clippy, test, deps) | none | Adopt | Phase 1 (with the cargo workspace) |
| D2 | `clippy.toml` disallows `env::set_var`/`remove_var` | none | Adopt; it enforces HARN-02 mechanically | Phase 1 |
| D3 | Exact `rust-toolchain.toml` pin | none. Research says 1.99.0 (host); devflow pins 1.97.1 | Pin exactly, at the host's version (1.99.0) | Phase 1 |
| D4 | `deny.toml` tracked on `develop`, not only the workspace branch (without it, `cargo deny` fails licenses) | none | Adopt on `develop` via the phase 1 PR | Phase 1 |
| D5 | CI with required checks, plus rulesets on `develop`/`main` | **No CI, no rulesets.** Rebase merge allowed; head branches not deleted on merge | CI (test, clippy, fmt) in phase 1; rulesets and merge settings applied with `gh api` once CI exists | Phase 1 (CI) + an owner-run setting change |
| D6 | Pinned devcontainer + `check-in-container.sh` (worktree-aware, per-checkout volumes) for host/CI parity | none | Defer; CI on a plain runner with the pinned toolchain is enough for one maintainer at milestone 1 | Revisit at milestone 2 (public release) |
| D7 | AI Change Acceptance criteria and the rejected test shapes in CONTRIBUTING | CONTRIBUTING still describes the Python sandbox | Port into the Rust CONTRIBUTING rewrite; it matches HARN-04's spirit | Phase 1 |
| D8 | Repo traps in CLAUDE.md (nextest `--no-tests=fail -E 'test(=…)'`, `rg -c`, zsh `PIPESTATUS`) | CLAUDE.md is Python-era | Port into the Rust CLAUDE.md rewrite; make nextest the plan gate | Phase 1 |
| D9 | Tracked hooks: off-worktree guard, plan bashism lint, commit-msg, divergence check, gitleaks chaining | none (minimal port) | Port when there is Rust source to guard: commit-msg and the gitleaks chain first, then the off-worktree guard. Each needs its scanner as a tested script | After phase 1 lands code |
| D10 | Tracked `.gitconfig` (signing, git-flow names) via `include.path` | wsx has no git-flow config; signing is local only | Adopt on `develop` (public-safe content only) | Phase 1 |
| D11 | Release signing by fingerprint (`releaseSigningKey`) | agent key set 2026-10-19; no release-key policy | Defer to milestone 2 (first public release) | Milestone 2 |
| D12 | GSD config posture: see the next table | partial | Align the git keys now; feature toggles are an owner choice | Before roadmap |
| D13 | Project `.claude/` eliminated; generic rules global | wsx GSD config sets `claude_md_path: ./.claude/CLAUDE.md`, and the pending new-project step would generate it | Follow devflow: skip `.claude/CLAUDE.md`, keep the root CLAUDE.md minimal | Before roadmap |
| D14 | DEV-SETUP-CHECKLIST + post-commit drift warning | this document | Treat this file as wsx's checklist seed; add the warning when hooks are ported | With D9 |

### GSD config differences (wsx vs devflow, as of today)
| Key | devflow | wsx | Note |
|---|---|---|---|
| `git.auto_branch`, `git.auto_cleanup`, `git.develop`, `git.feature_prefix` | set | absent | Not read by GSD 1.16 (devflow's own CLI reads them); not copied |
| `git.milestone_branch_template` | absent | `gsd/{milestone}-{slug}` | Written by GSD's project bootstrap (not in `~/.gsd/defaults.json`); align (remove) |
| `mode` | absent | `yolo` | Copied from `~/.gsd/defaults.json` at bootstrap (verified); check it's intended |
| `workflow.tdd_mode` | true | true | Same |
| `granularity` | fine (top-level and a redundant `workflow.` copy) | fine (top-level, the documented key) | Already aligned. Corrected 2026-10-19: the first comparison read only the `workflow.` copy |
| `workflow.windows_enforce` | true | absent | Owner choice (broken-windows ship gate) |
| `review.default_reviewers` | codex, opencode, antigravity | absent | Owner choice |
| `mempalace.*` | enabled, wing devflow | absent | Owner choice; its MCP server is disconnected this session |
| `model_overrides` | haiku for small agents | absent | Owner choice |
| `claude_md_path` | absent | `./.claude/CLAUDE.md` | See D13 |

## 5. Status after 2026-10-19

| devflow item | In wsx | How |
|---|---|---|
| Workspace branch, tracked `.planning/`, pushed | ✓ | `workspace/denniyahh` |
| sync-workspace / phase-worktree / cut-pr-branch | ✓ (shell ports) | `scripts/workspace/`. `cut-pr-branch.sh` is untested until phase 1 has code |
| `.workspace-divergence` | ✓ (declarations only) | No checker yet (D9) |
| Agent signing key | ✓ | local `user.signingkey` = devflow agent key |
| `use_worktrees: false`, `feature/phase-{phase}`, `commit_docs` | ✓ | |
| Repo hooks, gitleaks chaining, commit-msg | ✗ | D9. The global gitleaks hook still runs because wsx sets no local `hooksPath` |
| check.sh / clippy.toml / toolchain pin / deny.toml / CI | ✗ | D1–D5, phase 1 |
| Rulesets / merge settings | ✓ (2026-10-19) | `develop` and `main` (PR required, no deletion, no force-push), `workspace/*` (no deletion, no force-push), devflow's merge settings. Required checks: REL-04. **devflow's own live rulesets lack the deletion and force-push rules its checklist claims** |
| Container parity | ✗ | D6, deferred |
| Tracked `.gitconfig` | ✗ | D10, phase 1 |
| Release key policy | ✗ | D11, milestone 2 |
