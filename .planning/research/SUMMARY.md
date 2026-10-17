# Project Research Summary

**Project:** wsx (Rust rewrite: milestone 1, parity + hardened)
**Domain:** Data-safety-critical two-way file sync CLI for personal, untracked files across git worktrees
**Researched:** 2026-10-17
**Confidence:** MEDIUM-HIGH

> Assembled by the orchestrator from the gsd-research-synthesizer's returned synthesis. The harness
> blocked the subagent from writing the file. Every section traces to STACK.md, FEATURES.md,
> ARCHITECTURE.md or PITFALLS.md. Go to those files for evidence and citations.

## Executive Summary

wsx is a small sync engine, and its main risk is data loss. Its closest relatives are bidirectional
sync tools like Unison: a base snapshot, conflicts that keep both sides, and temp file + rename. It
is not much like other worktree managers. Every competitor (`.worktreeinclude`, Conductor,
worktrunk, git-env) copies files once, in one direction, when a worktree is created. Continuous
two-way sync against a base, the manifest guard, and push-before-remove are the whole niche.

The recommended build has three crates. `wsx-core` is a pure planner with no fs, git, clock or env
access. The `wsx` binary does all I/O. `wsx-blackbox` is a harness that links neither and runs the
same scenarios against the Python oracle and the Rust binary. Use gix 0.89 for reads, with default
features off, and the `git` binary for `worktree add -b` and `fetch`. Every subprocess runs with
`GIT_DIR`/`GIT_INDEX_FILE`/etc. scrubbed and stdin set to null.

Research found **five new 0.1.0 defects beyond CONCERNS.md**, four of them reproduced. The worst:
an unreadable *directory* makes `push` delete the whole subtree from the source. A
direct Rust port would bring back the absent-vs-unreadable data loss, because `Path::is_file()`/`exists()`
turn permission errors into `false`. The harness, with defect cases that must still fail on the
oracle, is therefore the first phase.

## Key Findings

### Recommended Stack

gix covers most of what wsx needs, as shown by a probe run against git 2.56 on a single fixture
(files backend, SHA-1): discover, git-dir/common-dir, current branch, listing worktrees, reading
repo/global/per-worktree config, writing per-worktree config, the staged set (checked inside a real
pre-commit hook), upstream ranges, commit walks, ignore checks, and worktree removal. Two things
need the `git` binary: `worktree add -b` and `fetch`. gix's `add_worktree` cannot create the
branch; it sets no upstream (which changes the pre-push guard range) and runs no `post-checkout`
hook. gix 0.89 cannot open reftable repos at all.

**Core technologies:**
- gix 0.89 (features `sha1`, `sha256`, `worktree-mutation`, `revision`, `interrupt`; default off): git reads and per-worktree config. Measured at 171 vs 199 crates, 6.2 vs 7.35 MB, 111 vs 160 s clean build.
- `git` binary ≥ 2.31: `worktree add -b`, `fetch`. It is already a hard runtime requirement.
- clap 4.6.7, serde_json 1.0.154, thiserror 2.0.21 (no anyhow/miette).
- sha2 0.11.1 (output matches Python's sha256 hex; needs a small hex encoder), tempfile 3.27.0, walkdir 2.5.0, jiff 0.2.38.
- `std::fs::File::lock` (Rust ≥ 1.89) in place of fs4/fd-lock. Verified with a contention control.
- Tests: nextest 0.9.148, assert_cmd 2.2.2, predicates 3.1.4, snapbox 1.2.2, proptest 1.12.0, cargo-mutants 27.1.0, llvm-cov 0.9.1. One suite picks its binary from `WSX_BIN`.
- `rust-version = "1.89"`, toolchain pinned to 1.99.0. The full set passes `cargo deny`, checked with a negative control. glibc builds for M1; a musl build is likely easy but was not built.

### Expected Features

**Must have (table stakes, M1):**
- Three-way sync that never turns "unreadable" into "deleted"
- Conflicts keep both sides; atomic writes; locking between concurrent runs
- Crash safety mid-apply; dry-run that runs the same planner
- Guard that works for non-ASCII paths, add-then-remove ranges and first push

**Should have (differentiators):**
- Continuous two-way sync. No competitor has it.
- Agent-session integration. FEATURES recommends pushing on every agent turn (`Stop` / `agentStop` / `AfterAgent`), not at session end. Claude Code's SessionEnd gets 1.5 s by default, Codex 1–3 s, Gemini doesn't wait, and Antigravity (agy 1.4.0) has no session-end event at all.
- Git 2.54+ config-based hooks: a guard without husky that gets the real pre-push refs including the first push, plus a `post-checkout` hook that fires in the new worktree on `git worktree add`. Probed on git 2.56 with controls.

**Defer (M2+):**
- Codex / Antigravity / Gemini adapters, guard without husky, maintenance commands, distribution, generalised base-ref defaults.

**Proposed additions not on the PROJECT Active list (need a decision):**
- A backup before every destructive write under any policy.
- A brake on mass deletions (e.g. an agent's `git clean -fdX` would otherwise propagate as source deletes).

### Architecture Approach

A pure `plan()` takes the manifest, file observations and base, and returns typed `Action`s carrying
the expected hashes. Every decision Python makes inside `apply()` moves into it: lww backups,
conflict-marker writes, the poisoned `""` base value, a second base load, and `resolve`'s own logic.
The executor only executes, and it re-hashes (compare-and-swap) before every destructive write. A
stat cache may skip hashing but must never authorise a destructive action.

**Major components:**
1. `wsx-core`: types (`Obs` with `Unreadable`/`Special`), manifest parser compatible with Python's, Base v1/v2, and the pure planner (proptest invariants, cargo-mutants on `-p wsx-core`).
2. `wsx` binary:
   - FS layer: any walk error marks the subtree unreadable; CAS before destructive writes; copy-with-hash; stat cache with a racy-timestamp rule; one flock on `<common-dir>/wsx/lock`, never held across a subprocess that can fire git hooks.
   - Git adapter: gix reads plus a scrubbed-env `git` spawn helper.
   - Commands and hook driver.
3. `wsx-blackbox`: the binary-agnostic harness. It installs the binary under test at a temp `$HOME/.local/bin/wsx`, because the hooks call that absolute path. Known divergences are typed scenarios.

### Critical Pitfalls

1. **The Rust std brings back absent-vs-unreadable data loss.** `Path::is_file()`/`exists()` and `filter_map(Result::ok)` over walkdir both swallow errors. Use an `Obs` enum, treat any walk error as making the subtree unreadable, and never plan a delete from `Unreadable`.
2. **Hook environment leakage.** `GIT_DIR`/`GIT_INDEX_FILE` reach gix and git calls inside hooks. N4 deletes the sibling worktrees' exclude files, and a plain `gix::open` reads the wrong index. Reach other worktrees only through `main_repo()`/`worktrees()`, and scrub the env of every spawn.
3. **The harness doesn't test what it seems to.** Hooks run `~/.local/bin/wsx`, not the build; the host git config and global hooks leak in; permission tests pass trivially as root; oracle tests encode the oracle's bugs. Use a hermetic HOME/XDG/`GIT_CONFIG_GLOBAL`, and give each fixed defect a `fixed:` scenario that must still fail on the oracle.
4. **Hooks fail open, which hides guard failures.** A clap enum would exit 2 on an unknown hook name; `println!` panics on a closed stdout; Ctrl-C returns 130. Keep hook names open-ended, `catch_unwind` in hook mode, and never use `panic = "abort"`. Husky's `h` runner exits before `init.sh` when the team ships no `.husky/pre-push`, so the wsx guard never runs.
5. **gix churn.** Breaking releases roughly monthly; worktree add/remove is 9 days old (0.89.0) and config writing 3 weeks old (0.88.0); gitoxide's `crate-status.md` and Context7 are stale. Keep gix behind one internal module and pin it.

## New 0.1.0 Defects (beyond `.planning/codebase/CONCERNS.md`)

| ID | Defect | Status |
|----|--------|--------|
| N1 | An unreadable local directory makes `push` delete the whole subtree from the source | REPRODUCED |
| N2 | The guard misses non-ASCII/special-character paths (`core.quotePath` quoting) | REPRODUCED |
| N3 | The pre-push guard compares only the two tips; a managed file added and then removed within the range passes | REPRODUCED (PITFALLS and the STACK probe) |
| N4 | `GIT_DIR` exported into hooks makes `prune_orphan_excludes` delete the sibling worktrees' exclude files | REPRODUCED |
| N5 | A non-UTF-8 filename is stored in v1 `base.json` as a lone surrogate, which serde_json rejects | Python side REPRODUCED; serde_json side SOURCE-READ |
| — | Husky `h` exits before `init.sh` without `.husky/pre-push`, so the guard never runs | SOURCE-READ |
| — | Interrupted symlink migration leaves siblings absent, and the next push deletes them from the source (I7) | CODE-READ |
| — | Lost update between two worktrees; base hash taken after the copy; plan and apply load base separately | CODE-READ |
| — | A manifest path that the dev branch tracks is overwritten with personal content | CODE-READ |
| — | Child processes inherit stdin; leftover `*.wsx-tmp` files get synced | CODE-READ |
| — | Hooks resolve the repo from cwd; Claude desktop worktree sessions run hooks in the main checkout, so they sync the wrong worktree | CODE-READ + docs |

## Researcher Conflicts

- **gix config write / worktree remove / staged set.** STACK actually ran these against git with controls, while ARCHITECTURE's "no" rests on the stale `crate-status.md`, so **STACK wins**. PITFALLS still prefers the `git` CLI for these in M1 because the APIs are weeks old. That is a judgment call, not a capability gap.
- **Hook lock wait.** ARCHITECTURE/FEATURES say about 1–2 s, based on Claude's 1.5 s SessionEnd budget. PITFALLS' "30 s" is only the snippet's per-hook timeout, so **1–2 s is right**.
- **Not settled:** gix `interrupt` vs wsx's own SIGINT handling; how the guard checks each commit (diff walk vs tree-contains); gix pin style (caret + `Cargo.lock` vs exact `=`); v2 filename; whether a corrupt base is treated as empty or as an error.

## Open Decisions (not decided; recommendation in brackets)

1. Should the guard fail closed on internal errors? [guard only fails closed; sync stays fail-open] (FEATURES, ARCH, PITFALLS)
2. How should an `lww` mtime tie resolve? [no recommendation] (ARCH, FEATURES)
3. base.json v2: one stat record per path, or one per side? [per side, plus one agreed hash] (ARCH)
4. v2 filename, path encoding, corrupt-base behaviour [new file; paths as bytes; corrupt is an error] (PITFALLS)
5. Non-UTF-8 filenames: support or reject loudly? (PITFALLS)
6. Backup before every destructive write, plus a mass-deletion brake [accept; backup retention then moves into M1] (FEATURES)
7. Pull the hook-stdin `cwd` fix into M1? [yes] (FEATURES)
8. Antigravity CLI before Gemini CLI [Antigravity first; M2] (FEATURES)
9. `wsx new` via the `git` binary [yes]. Also the CLI for `worktree remove` / `config --worktree` in M1? (STACK, PITFALLS)
10. Reftable repos [refuse with a clear message in M1] (STACK, PITFALLS)
11. Per-commit pre-push guard as a deliberate divergence from the oracle [yes] (STACK, PITFALLS)
12. Push on every agent turn vs session end only [turn-level] (FEATURES)
13. Stdout wording is not part of the parity contract; exit codes, filesystem effects and status classification are [agree] (ARCH, STACK, PITFALLS)
14. Symlink policy beyond migration (PITFALLS)
15. A manifest path that the dev branch tracks: skip with a warning, or a hard error? (PITFALLS)
16. Repos with reduced gix trust (another user's repo) [abort] (PITFALLS)

## Implications for Roadmap

Suggested 9 phases. Phases 3–4 can run in parallel with phase 5.

### Phase 1: Black-box harness vs Python oracle
**Rationale:** the harness is the spec, and everything else is measured against it.
**Delivers:** a hermetic, binary-agnostic scenario suite run from `cargo test` (so mutants and coverage see it). Each fixed defect has a `fixed:` case that must still fail on the oracle (the negative control). The binary under test is installed at a temp `$HOME/.local/bin/wsx`.
**Avoids:** pitfall 3.

### Phase 2: Workspace skeleton + core types
**Delivers:** three crates; `Obs` with `Unreadable`/`Special`; a manifest parser compatible with Python's; Base v1 read / v2 schema.
**Uses:** clap, serde_json, thiserror, sha2.

### Phase 3: Pure planner
**Delivers:** a `plan()` holding every decision that used to live in `apply()`; typed actions with expected hashes; proptest invariants; zero surviving mutants.
**Implements:** `wsx-core`.

### Phase 4: FS layer
**Delivers:** an error-aware walk (unreadable subtrees), CAS re-hash before destructive writes, copy-with-hash, a stat cache with the racy-timestamp rule, one repo-wide flock, fault injection.
**Avoids:** pitfall 1, N1, I7.

### Phase 5: Git adapter
**Delivers:** `discover_with_environment_overrides`; siblings reached only via `main_repo()`/`worktrees()`; a scrubbed-env spawn helper; path canonicalisation (`/home` → `/var/home`); reftable and trust checks.
**Avoids:** pitfall 2, N4, pitfall 5.

### Phase 6: Core commands
**Delivers:** `setup`, `pull`, `push`, `sync`, `status`, `resolve` at parity, with the known defects fixed.

### Phase 7: Ignore rules, guard, hook driver
**Delivers:** fixes for N2, N3 and N4 and for the first-push gap; open-ended hook names; `catch_unwind`; Ctrl-C handling in hook mode.
**Avoids:** pitfall 4.

### Phase 8: Lifecycle
**Delivers:** `new`, `rm`, `list`, `doctor`. The lock is released around `git worktree add` and `npm install`.

### Phase 9: Cutover
**Delivers:** the v1→v2 upgrade on real state, including round-tripping surrogate keys; the Rust binary installed; the Python `bin/wsx` deleted.

### Phase Ordering Rationale

- The harness comes first because it is the parity spec and the negative control for every fix.
- The pure core comes before I/O, so the planner can be property- and mutation-tested without a repo.
- The FS layer and git adapter are independent, so they can run in parallel. Commands depend on both.
- The guard and hooks come after the core commands, because they reuse sync and ignore installation.
- Lifecycle commands come last, because they are where locking and subprocesses that fire hooks interact.

### Research Flags

Phases likely needing deeper research during planning:
- **Phase 4:** stat-cache racy rule on kernel 7.2; SQLite WAL under `lww`; reproducing I7.
- **Phase 5:** unborn/detached HEAD, relative worktrees, non-UTF-8 paths.
- **Phase 7:** gix interrupt handling; guard design (per-commit check); llvm-cov through `env_clear`.
- **Phase 9:** round-tripping surrogate keys.

Phases with standard patterns: 2, 3, 6.

## Confidence Assessment

| Area | Confidence | Notes |
|------|------------|-------|
| Stack | HIGH | Probe executed, but on one fixture only (files backend, SHA-1) |
| Features | HIGH / MEDIUM | Agent hooks verified against docs; Gemini CLI findings are docs-only (not installed) |
| Architecture | MEDIUM | Current-behaviour claims HIGH (cited lines read); design recommendations MEDIUM; gix 0.89 method names LOW |
| Pitfalls | MEDIUM-HIGH | Four new defects reproduced with controls; others code-read or source-read |

**Overall confidence:** MEDIUM-HIGH

### Gaps to Address

- Whether Claude Code counts gitignored files as work before auto-removing a worktree. If not, unpushed wsx files are lost. Research in the M2 agent-integration phase.
- Whether Claude's own worktree creation runs `post-checkout`, and whether a detached push survives each runtime's exit.
- Gemini CLI's stdin `cwd` field and enable flag; Antigravity IDE hook firing on Linux (docs only).
- gix 0.89 method names (`tree_index_status` etc.) conflict with an unchecked crate-status item. Spike at the start of phase 5.
- The I7 interrupted-migration data loss is code-read only. Reproduce it in phase 4.

## Sources

See the Sources sections of STACK.md, FEATURES.md, ARCHITECTURE.md and PITFALLS.md: docs.rs/gix 0.89.0, released gix 0.89.0 source, official agent-runtime hook docs, local probes on git 2.56.0 / rustc 1.99.0 / Python 3.15.

---
*Research completed: 2026-10-17*
*Ready for roadmap: yes*
