# wsx

## What This Is

wsx keeps a developer's personal, untracked workspace files (planning docs, agent
state, local config) in sync across every git worktree of a repo. One source
worktree on a personal branch is the source of truth, and every dev worktree
pulls from it and pushes back to it. Nothing is ever committed to the team's
branches. It started as scripts in the devflow project. It is being turned into
an open-source CLI for developers who run AI coding agents (Claude Code,
Copilot, Codex, Gemini/Antigravity) across many worktrees.

## Core Value

wsx must never lose or silently clobber a user's personal files. Every sync
decision is either correct, reported as a conflict, or backed up. A sync that
deletes data it should not have is the one unacceptable failure.

## Requirements

### Validated

<!-- Inferred from wsx 0.1.0 (Python, bin/wsx) and its 28-assertion sandbox suite. -->

- ✓ Manifest `.wsx.conf` declares managed paths with per-path policy `sync | pull | lww | skip`; longest match wins: existing
- ✓ Three-way sync (source vs local vs `base.json` snapshot) classifies each file as pull / push / delete / conflict / converged: existing
- ✓ `--dry-run` runs the same planner as a real run (plan/apply split): existing
- ✓ Conflicts never overwrite; the source copy is written to `<file>.wsx-remote` and resolved with `wsx resolve --mine/--theirs`: existing
- ✓ `lww` overwrites are backed up under `<git-dir>/wsx/backups/`: existing
- ✓ Managed paths stay untracked through a per-worktree `core.excludesFile` (`extensions.worktreeConfig`) without writing tracked files: existing
- ✓ Commit/push guard blocks staged managed paths from husky `pre-commit`/`pre-push`: existing
- ✓ Repos without husky get marked `pre-commit`/`pre-push`/`post-merge` shims in the shared `.git/hooks`; foreign hooks and `core.hooksPath` repos are left alone: existing (`origin/develop`)
- ✓ A hook in one worktree no longer prunes other worktrees' exclude files (issue #1): existing (`origin/develop`)
- ✓ Hooks fail open (exit 0) except the guard: existing
- ✓ Worktree lifecycle: `wsx new` (create from base ref, arm husky, set up) and `wsx rm` (push back, then remove; refuse on conflicts): existing
- ✓ Migration away from a symlinked workspace layout (`via_symlink`, `materialise_parents`): existing
- ✓ Commands: `setup pull push sync status list doctor new rm resolve guard hook`: existing
- ✓ Integrations: husky personal init, Claude Code and Copilot session hooks, fish `git worktree remove` wrapper and completions, `install.sh`: existing

### Active

<!-- Milestone 1: Rust parity + hardened. Replaces Python 0.1.0 on the author's machine; no public release. -->

- [ ] Rust implementation of wsx with full command and behaviour parity with 0.1.0, except where a known defect is deliberately fixed
- [ ] Binary-agnostic black-box test harness (grown from `tests/sandbox.sh`) that runs against both the Python oracle and the Rust binary
- [ ] Fix data loss: an unreadable local file must never be planned as a deletion
- [ ] Fix data loss: `lww` must not delete local-only files on pull, and must back up before every `lww` local delete
- [ ] Fix the stale `""` base entry after a both-sides delete, so recreating the file is not a false conflict
- [ ] Guard covers a branch's first push (no upstream)
- [ ] Inherit a user's custom global `core.excludesFile` correctly
- [ ] Locking between concurrent wsx runs, unique temp files, and `*.wsx-tmp` never synced
- [ ] Stat cache in `base.json` v2 (one agreed hash plus a stat record per side) to avoid rehashing every file on every hook
- [ ] Safety net: compare-and-swap before every destructive write, a backup of every overwrite or delete (stored so it survives `wsx rm`), automatic backup retention, and a mass-deletion brake
- [ ] Never touch what wsx doesn't own: tracked paths, symlinks (either side) and foreign `*.wsx-remote` files are skipped and reported
- [ ] `base.json` v1 → v2 automatic one-time upgrade; `.wsx.conf` grammar and `wsx.*` git config keys stay compatible
- [ ] Hermetic tests (isolated HOME / XDG / git global config); coverage of symlink migration, every hook path, and each fixed defect
- [ ] Remaining code-read defects from the concerns audit addressed (lww mtime tie, `resolve --theirs` materialisation, backup stamp collisions, remote derivation in `new`, main-worktree detection in `rm`, Ctrl-C in hook mode)
- [ ] Python `bin/wsx` deleted once the Rust binary passes the harness

### Out of Scope

<!-- Milestone 1 boundaries. Several are planned for later milestones. -->

- Public OSS release, packaging, and distribution: deferred to milestone 2 (the baseline feature set)
- Guard without husky (plain `core.hooksPath` / pre-commit framework): milestone 2 baseline
- Maintenance commands (backup inspection/restore, interrupted-sync recovery, richer `doctor`): milestone 2 baseline. Automatic backup retention is in milestone 1
- Codex CLI and Antigravity CLI session integrations: milestone 2 baseline. Claude Code, Copilot, Codex and Antigravity are required for the public release. Gemini CLI is not supported (Antigravity replaces it)
- Generalising team-specific defaults (`origin/devtest` first in the base-ref chain, husky/npm assumption): milestone 2
- macOS and Windows: Linux only for now
- Broader "other developer repo needs" functionality: explored after the baseline ships
- Interactive prompts: wsx runs inside hooks and must stay non-interactive

## Context

- **Origin:** extracted from the author's devflow project (Rust; `~/Github/devflow`), where the personal-worktree workflow was first scripted.
- **Current codebase:** wsx 0.1.0 is one 1181-line stdlib-only Python file (`bin/wsx`) plus bash/sh/fish integrations. The full map is in `.planning/codebase/`. `origin/develop` is 5 commits ahead of `main` (issue #1 fix, plain `.git/hooks` guard, `--no-track`). The codebase map and research were written against `main`. **The parity oracle is `origin/develop`.**
- **Adversarial review:** `.planning/reviews/m1-adversarial/SUMMARY.md` (2026-10-19, three external models). Its 17 confirmed findings are folded into REQUIREMENTS.md.
- **Known defects:** `.planning/codebase/CONCERNS.md` lists three data-loss or false-conflict bugs reproduced in a scratch repo, a first-push guard gap, no locking, a full rehash per hook, and untested landmines (symlink migration, hook paths). A green 28/28 sandbox run says nothing about any of them.
- **Load-bearing design rules** (`CLAUDE.md`, `CONTRIBUTING.md`) carry over to Rust: all decisions in `plan()`, never in `apply()`; `via_symlink()` is not redundant with `is_symlink()`; `materialise_parents()` before any pull write; update the base snapshot on every applied action; hooks must not raise (except the guard); never write tracked files.
- **Audience (eventual):** developers running AI coding agents in many worktrees at once. Agent session hooks are the headline feature.

## Constraints

- **Language**: Rust. The author prefers it for reliability-critical code, and the Python stdlib-only rule is retired with the Python implementation.
- **Git access**: `gix` (pure-Rust git) rather than shelling out. Shelling out to `git` is an allowed fallback only where gix lacks a needed operation (e.g. worktree add/remove, `config --worktree`); research must confirm what gix covers.
- **Dependencies**: crates are allowed, but each must be justified and must pass `cargo deny`. Prefer Rust-native crates.
- **Compatibility**: `.wsx.conf` grammar, `wsx.sourceBranch` / `wsx.baseRef` keys and the exclude-file mechanism stay compatible with 0.1.0. `base.json` may change format, with an automatic upgrade.
- **Platform**: Linux (x86_64, aarch64) only for milestone 1.
- **Non-interactive**: never prompt. Git hook paths never read stdin (husky's `init.sh` and the `.git/hooks` shims share it with `pre-push`). Agent session hooks are the only exception: they read their bounded JSON payload, and only when stdin isn't a terminal.
- **Never write tracked files** in a managed repo.
- **Verification**: `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt`, proptest for the planner and manifest parser, and mutation testing (`cargo mutants`) on the planner/apply state machine.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| Rewrite in Rust now, not harden the Python first | Avoid fixing the defects twice; Rust suits the reliability goal | — Pending |
| Python 0.1.0 is the test oracle, then gets deleted | A black-box harness run against both binaries proves parity; known defects are the only allowed divergence | — Pending |
| Use gix for git access | Pure-Rust, no subprocess parsing; fall back to the `git` binary only where gix has gaps | — Pending |
| Migrate once: compatible manifest/config, versioned `base.json` | Keeps existing worktrees working while freeing internal state for the stat cache | — Pending |
| Milestone 1 = Rust parity + hardened, no public release | Hardening before features; distribution and baseline features come in milestone 2 | — Pending |
| Linux only for milestone 1 | The author's platform; macOS/Windows file-locking and symlink semantics add scope | — Pending |
| Guard fails closed on internal error, but only once the repo is confirmed wsx-configured | A guard that silently passes leaks personal files. The hook is machine-wide, so unrelated or refused repos must never be blocked | — Pending |
| Back up every destructive write, keep the newest 10 per path, store under the common git dir | Core Value. Backups must survive `wsx rm` | — Pending |
| Mass-deletion brake (default 10, `wsx.maxDeletions`); hooks skip only the deletes | Catches `git clean`-style accidents without freezing all sync | — Pending |
| Tracked paths, symlinks (either side) and foreign `*.wsx-remote` files: skip and warn | wsx never writes or deletes what it doesn't own | — Pending |
| Corrupt or unknown `base.json` stops wsx (hooks skip); never treated as empty | Overrides the research's "treat as empty": an empty base resurrects deletions | — Pending |
| Non-UTF-8 paths supported; `lww` mtime tie is a conflict | Correctness over the oracle's accidental behaviour | — Pending |
| Reftable repos refused in milestone 1, resolved before the public release | gix 0.89 can't open them; none of the author's 27 repos use reftable | — Pending |
| Pre-push guard checks every commit in the range | The oracle's tip-to-tip diff lets an added-then-removed file through | — Pending |
| Parity oracle is `origin/develop`, not the 0.1.0 tag | develop holds merged fixes and the plain git hook guard | — Pending |
| Agents push at the end of every turn, for all four runtimes | Codex (3 s max) and Antigravity (no session-end event) need it; one behaviour everywhere is simpler to reason about and test | — Pending |
| Roadmap built as horizontal layers | Parity-gated rewrite: each layer is verified against the oracle before the next starts | — Pending |

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
*Last updated: 2026-10-19 after the requirements review*
