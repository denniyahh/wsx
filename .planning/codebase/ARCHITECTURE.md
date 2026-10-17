---
last_mapped_commit: 5e4217fb6a940bf5ca5f53685fd2245097150676
last_mapped_at: 2026-10-17
---
<!-- refreshed: 2026-10-17 -->

# Architecture

**Analysis Date:** 2026-10-17

## System Overview

```text
┌──────────────────────────────────────────────────────────────────────┐
│                         Invokers (outside wsx)                        │
├───────────────────┬──────────────────────┬───────────────────────────┤
│ Interactive shell │ Git hooks via Husky  │ Agent session hooks       │
│ `wsx <cmd>`       │ `integrations/       │ `integrations/claude/...` │
│ fish `git` wrapper│  husky-init.sh`      │ `integrations/copilot/...`│
│ `shell/fish/...`  │ -> `wsx hook <name>` │ -> `wsx hook session-*`   │
└─────────┬─────────┴──────────┬───────────┴─────────────┬─────────────┘
          ▼                    ▼                         ▼
┌──────────────────────────────────────────────────────────────────────┐
│  CLI: build_parser() / main()  `bin/wsx:1100-1181`                    │
│  argparse subcommands -> cmd_* (`bin/wsx:614-1092`)                   │
└─────────┬────────────────────────────────────────────────────────────┘
          ▼
┌──────────────────────────────────────────────────────────────────────┐
│  Context `bin/wsx:122`  (git roots, source worktree, manifest, base)  │
├──────────────────────────────┬───────────────────────────────────────┤
│ Planner plan() `bin/wsx:356` │ Ignore + guard `bin/wsx:496-606`      │
│ Executor apply() `:419`      │ (per-worktree core.excludesFile)      │
└──────────────┬───────────────┴──────────────────┬────────────────────┘
               ▼                                  ▼
┌──────────────────────────────────────────────────────────────────────┐
│ Filesystem: source worktree files, dev worktree files,               │
│ `<git-dir>/wsx/base.json`, `<git-dir>/wsx/backups/`,                  │
│ `<common-dir>/info/exclude.wsx-<id>`, `*.wsx-remote` conflict markers │
│ git: `git` subprocess via git()/git_ok() `bin/wsx:77-114`            │
└──────────────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| git helpers | Run `git` subprocesses; parse `git worktree list` | `bin/wsx:77-114` (`git`, `git_ok`, `list_worktrees`) |
| `Context` | Resolve root/git-dir/common-dir, branch, source branch (`wsx.sourceBranch`, default `personal-workspace`), source worktree; load/validate `.wsx.conf`; load/save `base.json` atomically | `bin/wsx:122-229` |
| file plumbing | Hashing, tree walk, longest-match policy lookup, symlink detection, parent materialisation, copy/remove, backups | `bin/wsx:237-338` |
| planner | Turn manifest + live files + base snapshot into `Action`s | `bin/wsx:346-416` (`Action`, `plan`) |
| executor | Execute `Action`s filtered by direction; write conflict markers; update base | `bin/wsx:419-488` (`apply`) |
| ignore + guard | Generate per-worktree exclude file, inline global ignore, prune orphans, self-heal | `bin/wsx:496-606` |
| commands | One `cmd_*` per subcommand | `bin/wsx:614-1092` |
| CLI | argparse wiring, exit-code and fail-open policy | `bin/wsx:1100-1181` |
| installer | Symlink/copy bin, fish files, husky init; merge Claude hooks JSON | `install.sh` |
| test suite | End-to-end bash sandbox against throwaway repo | `tests/sandbox.sh` |

## Pattern Overview

**Overall:** Single-file, stdlib-only Python CLI with a plan/apply split (pure-ish planner, side-effecting executor) and three-way sync against a persisted base snapshot.

**Key Characteristics:**
- All sync decisions live in `plan()`; `apply()` only executes, so `--dry-run` runs the same planner and skips writes (`bin/wsx:463`).
- Three-way comparison: source hash vs local hash vs `base.json` hash decides pull / push / delete / conflict (`bin/wsx:405-414`).
- Fail-open in hook mode: errors are reported and exit 0, except the guard, which returns 1.
- Never writes tracked files: ignore rules go through `extensions.worktreeConfig` + `git config --worktree core.excludesFile` pointing at `<common-dir>/info/exclude.wsx-<id>` (`bin/wsx:559-570`).

## Layers

**Invocation layer:**
- Purpose: Trigger wsx at the right lifecycle moments
- Location: `integrations/`, `shell/fish/`, `install.sh`
- Contains: sh hook extension, agent hook JSON, fish wrapper for `git worktree remove`, fish completions
- Depends on: installed `~/.local/bin/wsx`
- Used by: Husky, Claude Code, Copilot, interactive fish

**CLI/commands layer:**
- Purpose: Parse args, enforce preconditions (`require_configured`, `require_dev_worktree`), print reports
- Location: `bin/wsx:614-1181`
- Depends on: Context, planner/executor, ignore+guard
- Used by: invocation layer

**Core layer:**
- Purpose: Sync semantics and ignore installation
- Location: `bin/wsx:122-606`
- Depends on: git helpers, filesystem

## Data Flow

### Primary Request Path (`wsx sync`)

1. `main()` builds parser, constructs `Context(Path.cwd())` (`bin/wsx:1159-1162`)
2. `cmd_sync` checks dev worktree, calls `plan(ctx)` (`bin/wsx:649`)
3. `plan()` loads manifest + base, unions `walk()` of both trees plus base keys, classifies each path by policy (`bin/wsx:356-416`)
4. `apply(ctx, actions, {"in","out"}, dry_run)` copies/removes, writes `*.wsx-remote` for conflicts, updates base map (`bin/wsx:419-488`)
5. `ctx.save_base()` writes `base.json` via tmp + `replace` (`bin/wsx:217-229`); return value = conflict count

### Hook Path (`wsx hook pre-commit`)

1. Husky sources `~/.config/husky/init.sh`, which runs `wsx hook "$n" </dev/null || exit 1` unless `WSX_SKIP=1` (`integrations/husky-init.sh:28-30`)
2. `cmd_hook` no-ops if unconfigured or source; runs `ensure_installed()` before the guard (`bin/wsx:1022-1035`)
3. `cmd_guard` checks staged paths (or `@{upstream}..HEAD` for pre-push) against `managed_paths()`; returns 1 to block (`bin/wsx:992-1019`)
4. `quiet_sync(ctx, {"out"}, ...)` best-effort push; catches `Problem`, always returns 0 (`bin/wsx:1069-1092`)
5. `post-merge`/`post-checkout`/`post-rewrite`/`session-start` sync inbound; `session-start` also arms husky via `arm_husky()` (`bin/wsx:1050-1058`)

### Worktree lifecycle

1. `wsx new` creates worktree from `base_ref()` (`wsx.baseRef`, else `origin/devtest`), arms husky, runs setup (`bin/wsx:803-891`)
2. `wsx rm` pushes personal state back, then removes the worktree; refuses on conflicts unless `--force` (`bin/wsx:906-951`)
3. fish `git` wrapper reroutes `git worktree remove` to `wsx rm` (`shell/fish/functions/git.fish`)

**State Management:**
- Per-worktree: `<git-dir>/wsx/base.json` (version 1, `entries` path->sha), `<git-dir>/wsx/backups/` for lww overwrites
- Per-repo shared: `<common-dir>/info/exclude.wsx-<worktree-id>`
- Config: `git config wsx.sourceBranch`, `wsx.baseRef`
- Manifest: `.wsx.conf` at the source worktree root (template: `examples/wsx.conf`)

## Key Abstractions

**`Context`:**
- Purpose: Everything resolved once per invocation about where we are
- Examples: `bin/wsx:122`
- Pattern: Plain class constructed in `main()`, passed to every `cmd_*`

**`Action`:**
- Purpose: One planned operation; `kind` in `pull | push | delete-local | delete-remote | conflict | converged`
- Examples: `bin/wsx:346-353`
- Pattern: `__slots__` value object; direction derived from kind (`pull`/`delete-local` = "in")

**Policy:**
- Purpose: `sync | pull | lww | skip` per manifest entry; longest matching entry wins (`policy_for`, `bin/wsx:274`)

**`Problem`:**
- Purpose: Expected, user-facing error; `main()` prints it and exits 2 (0 in hook mode) (`bin/wsx:68`)

## Entry Points

**`bin/wsx` (`main`):**
- Location: `bin/wsx:1159`, `__main__` guard at end of file
- Triggers: shell, hooks, agent sessions
- Responsibilities: dispatch subcommands `setup pull push sync status list doctor new rm resolve guard hook`

**`install.sh`:**
- Triggers: manual; modes default symlink, `--copy`, `--uninstall`, `--dry-run`
- Responsibilities: place bin/fish/husky files, back up existing husky init to `*.pre-wsx.bak`, merge Claude hook snippet via embedded Python

**`tests/sandbox.sh`:**
- Triggers: manual; shims PATH so hooks use this checkout's `bin/wsx`

## Architectural Constraints

- **Threading:** Single-threaded, synchronous; each command is one short-lived process.
- **Global state:** Module constants only (`MANIFEST_NAME`, `DEFAULT_SOURCE_BRANCH`, `CONFLICT_SUFFIX`, `POLICIES`, `IGNORED_BASENAMES`, color codes) at `bin/wsx:55-66`. No mutable singletons.
- **Circular imports:** Not applicable (one file, stdlib only).
- **Dependencies:** stdlib only; no third-party packages allowed (`CLAUDE.md`).
- **Non-interactive:** Must never prompt or read stdin; runs inside hooks.
- **Fail-open location:** The catch-all is in `main()` (`bin/wsx:1163-1178`, keyed on `args.command == "hook"`), not inside `cmd_hook`. `cmd_hook` itself only catches `Problem` from `ensure_installed`; `quiet_sync` catches `Problem`. `CLAUDE.md`'s "cmd_hook catches everything" is true only of the hook path as a whole.
- **Single core.excludesFile:** git allows one; wsx inlines the global ignore into the generated file (`render_exclude`, `bin/wsx:512`).

## Anti-Patterns

### Decision logic in apply()

**What happens:** Tempting to branch on file state inside `apply()`.
**Why it's wrong:** `--dry-run` would no longer reflect what runs. (`apply()` already does one policy-aware thing: lww backup before overwrite, `bin/wsx:467-473`.)
**Do this instead:** Add new classification to `plan()` (`bin/wsx:356`) and emit a distinct `Action.kind`/`detail`.

### Trusting `Path.is_symlink()` alone

**What happens:** File under a symlinked directory hashes identical to source and reads as converged.
**Do this instead:** Use `via_symlink()` (`bin/wsx:285`) and call `materialise_parents()` (`bin/wsx:301`) before any pull write.

### Writing base.json partially

**What happens:** Skipping base updates for an applied action.
**Why it's wrong:** Next run cannot tell who changed what; produces spurious conflicts or wrong-direction copies.
**Do this instead:** Update `base[rel]` for every applied action, as in `bin/wsx:465-484`.

## Error Handling

**Strategy:** Raise `Problem` for expected failures; `main()` maps to exit 2 (red) or 0 in hook mode (yellow). Unexpected exceptions re-raise outside hook mode, swallowed with exit 0 inside it. `KeyboardInterrupt` -> 130.

**Patterns:**
- `git(..., check=False)` returns "" for optional config lookups instead of raising.
- Commands return the conflict count as a non-zero exit (`cmd_setup` returns 1 on conflicts).
- Conflicts never overwrite; source copy goes to `<file>.wsx-remote`.

## Cross-Cutting Concerns

**Logging:** `print` to stdout for progress, stderr for warnings/errors, ANSI colors via module constants.
**Validation:** Manifest parsing in `Context.load_manifest()` rejects unknown policies, absolute paths, and `..` (`bin/wsx:184-207`).
**Authentication:** Not applicable.

---

*Architecture analysis: 2026-10-17*
