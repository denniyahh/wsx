# wsx

## What This Is

wsx manages a developer's **personal workspace branch** in a git repo: the integration branch
(`develop`, `next`, `main`) plus tracked personal files (planning docs, agent config, tooling).
Feature work happens in worktrees branched from that workspace branch, so personal files and
any planning committed alongside code stay with the work. wsx cuts clean PR branches that carry
only the code, and guards every push so personal files never reach a shared branch. It
formalises the "Workflow B" setup the author runs by hand in devflow, gsd-core, gsd-pi and wsx
itself, and it is useful in any dev repo, with or without AI agents.

**wsx and devflow are complementary, with no overlap.** wsx manages and maintains the repo:
- the workspace branch and manifest
- syncing with the integration branch
- worktrees and branches
- PR cuts, divergence checks, guards and hooks

devflow manages AI agents and GSD: phases, ROADMAP/STATE, GSD config, agent sessions, gates, ship
approval and versioning. devflow drives wsx through its JSON interface for repo operations
instead of reimplementing them. wsx knows nothing about GSD, devflow or agent runtimes
(REQUIREMENTS.md: Boundary, IF-01, IF-02).

wsx 0.1.0 (Python, a copy-sync model) is frozen. See Key Decisions.

## Core Value

wsx must never let a personal file reach a shared branch or PR, and must never lose work while
moving it between the workspace branch, feature worktrees and PR branches. A leaked file, or a
cut that silently drops code, is the unacceptable failure.

## Requirements

### Validated

<!-- Proven in daily use as shell scripts (devflow Workflow B), now the oracle. Their GSD-specific parts (ROADMAP lookup, phase auto-detection) stay with devflow. -->

- ✓ Workspace branch synced from the integration branch by merge (`sync-workspace.sh`): existing (devflow)
- ✓ Phase worktrees in `.worktrees/phase-N`, branched from the workspace branch, phase validated against ROADMAP.md, idempotent (`phase-worktree.sh`): existing (devflow)
- ✓ Clean PR branches: replay by phase scope or fork point, strip personal paths, leak audit, fidelity report (`cut-pr-branch.sh`): existing (devflow)
- ✓ Divergence manifest and check: every workspace difference declared `only`/`modified`, stale declarations and non-executable hooks fail (`check-workspace-divergence.sh`): existing (devflow)
- ✓ Push guard over every commit's touched paths in the range, fail closed on an unreadable range (devflow `pre-push`): existing (devflow)

### Active

<!-- Milestone 1: Rust parity + hardened against the shell oracle. See REQUIREMENTS.md (49). -->

- [ ] Rust implementation of workspace sync, worktrees, PR cut, divergence check and guards, at parity with the shell oracle except deliberate fixes
- [ ] Black-box harness against the vendored shell oracle and Rust, with explicit expectations and positive/negative controls
- [ ] One resolved personal-path list shared by guard, cut, check and status; configurable integration branch, workspace branch, base and push remotes, worktree directory
- [ ] Hooks installed as git config-based hooks (git ≥ 2.54), coexisting with `core.hooksPath`, husky and global hooks; pre-push reads the real pushed refs
- [ ] Oracle defects fixed: quoted-name bypass, pattern-limited push guard, hard-coded `origin`, swallowed cherry-pick errors, partial PR branch on conflict
- [ ] `wsx migrate`: new `.wsx.conf` from 0.1.0 `.wsx.conf` or `.workspace-divergence`; 0.1.0 copy-sync leftovers removed only when proven committed
- [ ] `--json` output and stable exit codes for every command, so devflow can drive wsx; wsx reads no GSD, devflow or agent files
- [ ] Cutover in wsx and devflow; shell oracle and Python 0.1.0 deleted (`v0.1.0` tag kept)

### Out of Scope

- Copy-sync (0.1.0's model): dropped 2026-10-19
- Legacy manifest grammars outside `wsx migrate`
- Public release, packaging, distribution, hook fallback for git < 2.54: milestone 2
- Agent session integrations, phases/ROADMAP, GSD config checks, the off-worktree guard, ship approval and versioning: devflow's domain, not wsx's
- Changing devflow's code to call wsx (`devflow start` → `wsx new`, Ship → `wsx land`): devflow's own roadmap
- Protected-branch commit refusal, off-worktree guard: milestone 2
- macOS and Windows: Linux only for now
- Interactive prompts; resolving conflicts for the user
- Broader "other developer repo needs": explored after the baseline ships

## Context

- **Origin:** the workspace-branch workflow was built by hand in devflow (`~/Github/devflow`, CONTRIBUTING "Workflow B"), and is documented in `research/DEVFLOW-SETUP.md`. The same model runs informally in gsd-core and gsd-pi, which have no manifest and already carry undeclared divergence.
- **Oracle:** `scripts/workspace/*.sh` on `workspace/denniyahh` (ported 2026-10-19), plus devflow's divergence check and pre-push policy, to be vendored. Targeted research is in `research/WORKSPACE-MODE.md`.
- **Prior work kept for reference:** the copy-sync research (`research/STACK|FEATURES|ARCHITECTURE|PITFALLS|SUMMARY.md`), the codebase map of 0.1.0, and the copy-sync adversarial review. The git-adapter, hook-environment and harness findings still apply.
- **Audience (eventual):** developers who keep personal files (notes, planning, editor and agent config) in a repo and work in many worktrees, with or without AI agents. devflow is one consumer.

## Constraints

- **Language**: Rust.
- **Git access**: the `git` binary performs every history-changing operation (merge, cherry-pick, branch and worktree creation, commit, fetch), so the user's hooks, signing, merge drivers and credentials apply. gix does read-only analysis. See `research/WORKSPACE-MODE.md` R1.
- **Dependencies**: each crate justified, `cargo deny` clean, Rust-native preferred.
- **Never write shared branches.** wsx creates commits only on the workspace branch (merges, migrate), on branches it creates, and on PR branches it cuts. It never pushes.
- **Platform**: Linux, git ≥ 2.54, for milestone 1.
- **Non-interactive**: never prompt. Hooks read stdin only where git gives each hook its own copy (config-based pre-push).
- **Verification**: clippy `-D warnings` over all targets, fmt, nextest `--no-tests=fail`, proptest for the manifest and path matcher, `cargo mutants` on the matcher, classifier and guard range logic.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rewrite in Rust now, not harden the Python first | Avoid fixing the defects twice; Rust suits the reliability goal | Still holds (Rust) |
| Python 0.1.0 is the test oracle, then gets deleted | A black-box harness run against both binaries proves parity; known defects are the only allowed divergence | Superseded 2026-10-19: oracle is the shell scripts |
| Use gix for git access | Pure-Rust, no subprocess parsing; fall back to the `git` binary only where gix has gaps | Revised 2026-10-19: git binary writes, gix reads |
| Migrate once: compatible manifest/config, versioned `base.json` | Keeps existing worktrees working while freeing internal state for the stat cache | Superseded 2026-10-19: copy-sync dropped; `wsx migrate` |
| Milestone 1 = Rust parity + hardened, no public release | Hardening before features; distribution and baseline features come in milestone 2 | — Pending |
| Linux only for milestone 1 | The author's platform; macOS/Windows file-locking and symlink semantics add scope | — Pending |
| Guard fails closed on internal error, but only once the repo is confirmed wsx-configured | A guard that silently passes leaks personal files. The hook is machine-wide, so unrelated or refused repos must never be blocked | — Pending |
| Back up every destructive write, keep the newest 10 per path, store under the common git dir | Core Value. Backups must survive `wsx rm` | Superseded 2026-10-19: copy-sync dropped |
| Mass-deletion brake (default 10, `wsx.maxDeletions`); hooks skip only the deletes | Catches `git clean`-style accidents without freezing all sync | Superseded 2026-10-19: copy-sync dropped |
| Tracked paths, symlinks (either side) and foreign `*.wsx-remote` files: skip and warn | wsx never writes or deletes what it doesn't own | Superseded 2026-10-19: copy-sync dropped |
| Corrupt or unknown `base.json` stops wsx (hooks skip); never treated as empty | Overrides the research's "treat as empty": an empty base resurrects deletions | Superseded 2026-10-19: copy-sync dropped |
| Non-UTF-8 paths supported; `lww` mtime tie is a conflict | Correctness over the oracle's accidental behaviour | Partly holds: raw-byte paths kept (CONF-01); lww gone |
| Reftable repos refused in milestone 1, resolved before the public release | gix 0.89 can't open them; none of the author's 27 repos use reftable | — Pending |
| Pre-push guard checks every commit in the range | The oracle's tip-to-tip diff lets an added-then-removed file through | — Pending |
| Parity oracle is `origin/develop`, not the 0.1.0 tag | develop holds merged fixes and the plain git hook guard | Superseded 2026-10-19: oracle is the shell scripts |
| Agents push at the end of every turn, for all four runtimes | Codex (3 s max) and Antigravity (no session-end event) need it; one behaviour everywhere is simpler to reason about and test | Moved out of wsx 2026-10-19: agent sessions are devflow's domain |
| Roadmap built as horizontal layers | Parity-gated rewrite: each layer is verified against the oracle before the next starts | — Pending |
| wsx adopts the workspace-branch model (devflow Workflow B) as its product; copy-sync is dropped and 0.1.0 frozen | Every repo the owner uses runs the workspace-branch model (devflow, gsd-core, gsd-pi, wsx); copy-sync is incompatible with GSD committing planning docs, and carries most of the data-loss risk. Owner decision 2026-10-19 | — Pending |
| Milestone 1 is re-scoped around workspace-branch mode, with `scripts/workspace/` (and devflow's hooks) as the oracle | Build what is used; the shell ports play the role Python 0.1.0 played | — Pending |
| New `.wsx.conf` grammar (`only`/`modified`); legacy formats only inside `wsx migrate` | Little legacy support, easy upgrades. Owner decision 2026-10-19 | — Pending |
| Hooks as git config-based hooks (≥ 2.54) | Run alongside `core.hooksPath` and global hooks, and get the real pushed refs (tested 2026-10-19) | — Pending |
| No GSD-generated `.claude/CLAUDE.md` | A preview was 286 lines describing the dropped copy-sync design; devflow removed project `.claude/` configs too. Root CLAUDE.md is rewritten for Rust in phase 1. Owner decision 2026-10-19 | — Pending |
| wsx manages the repo; devflow manages agents and GSD; no overlap; wsx is standalone | Owner intent 2026-10-19. wsx has no GSD/agent knowledge (IF-02); devflow drives wsx via `--json` (IF-01); phase concepts, agent integrations, ship/versioning stay in devflow | — Pending |

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check: still the right priority?
3. Audit Out of Scope: reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-10-19 after re-scoping to the workspace-branch model*
