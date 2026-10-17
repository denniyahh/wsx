# Feature Research

**Domain:** CLI that syncs personal, untracked workspace files (planning docs, AI-agent state, local config) across git worktrees, with a commit guard and AI-agent session hooks
**Researched:** 2026-10-17
**Confidence:** HIGH for agent hook mechanisms and git hook mechanics (official docs, plus local binaries and probes); MEDIUM for competitor behaviour (official docs, some third-party); MEDIUM for the milestone split (it is my recommendation).

Evidence tags used below:
- **[DOCS]**: current official documentation, fetched 2026-10-17.
- **[LOCAL]**: checked on this machine against an installed binary or a throwaway-repo probe with a negative control. Versions: Claude Code 2.1.301, Codex CLI 0.162.1, Copilot CLI 1.0.81, Antigravity CLI (`agy`) 1.4.0, git 2.56.0. Gemini CLI is **not installed**, so Gemini findings are docs-only.
- **[CODE-READ]**: follows from reading `bin/wsx` 0.1.0. Not executed.
- **[3P]**: third-party source only. Do not treat as authoritative.
- **UNVERIFIED**: no source settled it. Needs research at the phase that depends on it.

---

## Findings that change the plan

Read these first. Each one changes what a milestone has to contain.

1. **Session-end push is not reliable in any of the four runtimes.** Claude Code gives `SessionEnd` hooks a **1.5 s** default budget. Per-hook `timeout` can raise it to 60 s, but **timeouts on plugin-provided hooks do not raise it** [DOCS]. Codex `SessionEnd` defaults to **1 s, maximum 3 s** [DOCS]. Gemini CLI "will not wait" for `SessionEnd` [DOCS]. Antigravity has **no session-end event at all**. Its hook-arg types are only `SessionStart`, `PreInvocation`, `PostInvocation`, `PreTool`, `PostTool` and `Stop` [DOCS + LOCAL, agy 1.4.0 binary]. So a design where "session end pushes" will lose the last edits whenever the push is slow or the runtime does not wait. The fix is to push at **end of turn** (`Stop` / `agentStop` / `AfterAgent`), which depends on the stat cache and locking from milestone 1, and to treat session end as best-effort.
2. **Hooks in agent runtimes may not run in the worktree.** In Claude Code desktop-app worktree sessions, "Hook commands run in that root" (the main checkout), and `${CLAUDE_PROJECT_DIR}` stays at the launch directory. Only the stdin `cwd` field follows the worktree [DOCS]. wsx 0.1.0's `wsx hook session-start` resolves the repo from the process cwd [CODE-READ], so in that case it syncs the wrong worktree. Every agent adapter must read the worktree path from the hook's stdin JSON: `cwd` for Claude, Copilot and Codex; `workspacePaths` (an array, with no `cwd` field) for Antigravity.
3. **Git ≥ 2.54 config-based hooks solve both "guard without husky" and "hook worktree creation".** [LOCAL, git 2.56, hermetic probe with controls]:
   - `git config --worktree hook.wsxguard.event pre-commit` (plus `pre-push`) runs **only in that worktree**. The control case, a commit in the main worktree, did not fire it.
   - It runs **in addition to** a `core.hooksPath` (husky-style) hook, and runs first.
   - It receives the real `pre-push` stdin refs, including the all-zero remote SHA on a first push.
   - A non-zero exit blocks the commit.
   - A **repo-level** `hook.<name>.event post-checkout` fires inside the *new* worktree on `git worktree add`. The control, `--no-checkout`, did not fire.
   - Nothing tracked is written. The README's "worktree creation can't be hooked cleanly" limitation goes away on git ≥ 2.54.
4. **No competitor does continuous two-way sync.** Claude Code `.worktreeinclude`, Conductor, worktrunk `wt step copy-ignored`, git-wt `--copyignored`, wrktreez and git-env all **copy once at creation, in one direction** [DOCS/3P]. git-worktree-share uses symlinks [3P], the very layout wsx migrates away from. A base-tracked two-way sync, plus a manifest-keyed commit guard, plus push-before-remove, is wsx's actual niche. That keeps the differentiator list short and the table-stakes list about not losing data.
5. **Claude Code may delete a worktree whose only unsaved work is wsx-managed files.** Claude removes an unnamed worktree on exit when it finds no "changed or untracked files", and its periodic sweep removes subagent and background worktrees on the same test [DOCS]. wsx-managed paths are *ignored* through the per-worktree `core.excludesFile`. Whether Claude counts ignored files as work, and whether `SessionEnd` finishes before the removal, are **UNVERIFIED**. If ignored files are not counted, unpushed personal edits in those worktrees are deleted. Turn-level push (finding 1) shrinks that window to the last turn.
6. **Gemini CLI has been replaced by Antigravity CLI for unpaid-tier and Google One users** (2026-06-18) [DOCS]. The "Gemini/Antigravity" integration should target Antigravity CLI (`agy`) first. Gemini CLI can be a cheap second adapter, because it uses a Claude-like schema.

---

## Agent Session Hook Mechanisms (verified 2026-10-17)

| Runtime | Start event | End event | Turn-end event (recommended push point) | User-level config, where wsx should install | Format | Timeout | Worktree path in stdin | Trust / gating | Evidence |
|---|---|---|---|---|---|---|---|---|---|
| **Claude Code** 2.1.301 | `SessionStart`; matcher/`source` = `startup`, `resume`, `clear`, `compact`, `fork` | `SessionEnd`; `reason` = `clear`, `resume`, `logout`, `prompt_input_exit`, `other`. Cannot block. | `Stop` | `~/.claude/settings.json` → `hooks` (also `.claude/settings{,.local}.json`, plugin `hooks/hooks.json`, managed policy) | JSON: `{"hooks":{"SessionStart":[{"matcher":"…","hooks":[{"type":"command","command":"…","timeout":30}]}]}}` | Command hooks 600 s. **SessionEnd: 1.5 s budget**, raised by per-hook `timeout` up to 60 s (not for plugin hooks) or by `CLAUDE_CODE_SESSIONEND_HOOKS_TIMEOUT_MS` | `cwd` (follows the worktree); `CLAUDE_PROJECT_DIR` does **not** follow | None for user settings | [DOCS] hooks + worktrees pages; [LOCAL] version |
| **GitHub Copilot CLI** 1.0.81 | `sessionStart`; `source` = `startup`, `resume`, `new`. Output `additionalContext` is used. | `sessionEnd`; `reason` = `complete`, `error`, `abort`, `timeout`, `user_exit`. Output ignored. | `agentStop` | `~/.copilot/hooks/*.json` (or `$COPILOT_HOME/hooks/`); also `hooks` in `~/.copilot/settings.json` | JSON `{"version":1,"hooks":{"sessionStart":[{"type":"command","bash":"…","timeoutSec":30}]}}`; `exec`+`args` runs without a shell | `timeoutSec` default 30; **timeouts fail open** | `cwd` (camelCase payload; PascalCase event names switch to snake_case) | None for user hooks. Copilot **also reads project `.claude/settings.json`**, so double firing is possible | [DOCS] CLI hooks reference; [LOCAL] version only. The binary is packed, so a string check was inconclusive. |
| **OpenAI Codex CLI** 0.162.1 | `SessionStart`; `source` = `startup`, `resume`, `clear`, `compact`, `fork` | `SessionEnd`: fires on archive/delete of an open conversation, on normal close, or **after 30 min idle**. Synchronous. Not for subagents. | `Stop` | `~/.codex/hooks.json` or inline `[hooks]` in `~/.codex/config.toml` | Claude-like JSON (`matcher`, nested `hooks`, `type:"command"`, `statusMessage`) | Seconds; most 600. **SessionEnd default 1, max 3** | `cwd` (also `session_id`, `transcript_path`, `model`, `permission_mode`) | **On by default** (`[features] hooks = false` disables). **Non-managed hooks are skipped until trusted via `/hooks`, and trust is pinned to the hook's hash**, so editing the command string forces re-trust. Project `.codex/` hooks load only in trusted projects. | [DOCS] learn.chatgpt.com/docs/hooks; [LOCAL] binary contains `SessionStart`/`SessionEnd`/`Interrupt` (control string absent) |
| **Gemini CLI** (not installed) | `SessionStart`; `source` = `startup`, `resume`, `clear`. Never blocks. | `SessionEnd`; `reason` = `exit`, `clear`, `logout`, `prompt_input_exit`, `other`. **Best-effort, not awaited.** | `AfterAgent` | `hooks` in `~/.gemini/settings.json` (also project `.gemini/settings.json`, `/etc/gemini-cli/settings.json`, extensions) | JSON matcher groups; `type:"command"`, `name`, `command`, `timeout` | **Milliseconds**, default 60000 | UNVERIFIED (not in the excerpts fetched) | Project hooks are fingerprinted, and a changed command is untrusted. Whether hooks need an enable flag: UNVERIFIED ([3P] mentions `hooks.enabled`; official pages do not). | [DOCS] geminicli.com hooks + reference |
| **Antigravity CLI `agy`** 1.4.0 (and IDE / App) | `SessionStart`: once per conversation, before the first model call | **None.** `Stop` (execution loop ends; fields `terminationReason`, `fullyIdle`, `executionNum`) is the closest. | `Stop` with `fullyIdle: true` | Global `~/.gemini/config/hooks.json` (all surfaces); CLI also `~/.gemini/antigravity-cli/settings.json`; workspace `.agents/hooks.json` | JSON keyed by a **named group** first: `{"wsx":{"SessionStart":[{"type":"command","command":"…","timeout":30}]}}`. `SessionStart`/`Stop` take a flat handler list (matcher ignored). `enabled` flag per hook. | Seconds, default 30 | **No `cwd`**. `workspacePaths` (array), `conversationId`, `transcriptPath` | Managed via `/hooks` in the TUI; trust model UNVERIFIED | [DOCS] antigravity.google/docs/hooks; [LOCAL] the agy 1.4.0 binary compiles exactly six `*HookArgs` types: SessionStart, PreInvocation, PostInvocation, PreTool, PostTool, Stop |

**Caveats on this table:**
- Antigravity: [3P] claude-mem reports that agy **1.2.1** had no `SessionStart`. 1.4.0 has it [LOCAL], so support depends on the version, and the adapter needs a version check. [3P] A forum report says `Stop`/`PostToolUse` hooks in `.agents/hooks.json` never fired on Antigravity **IDE** 1.107.0 (Windows). The IDE is UNVERIFIED on Linux. Plan for CLI first.
- Codex trust pinning means `wsx install` cannot activate Codex hooks silently. The user must run `/hooks` once, and again after any wsx upgrade that changes the command string. **Keep the installed command string stable across versions**, for example `wsx hook --agent codex session-start`, and keep version logic inside the binary.
- The existing `integrations/copilot/wsx-session.json` calls `$HOME/.config/agents/render-agent-rules`. That is personal to the author and must not ship in milestone 2.
- `SessionStart` stdout is added to Claude's context on exit 0. The 0.1.0 snippet sends it to `/dev/null`, which is correct. Any output must be deliberate (see Differentiators, conflict notice).

---

## Feature Landscape

Milestone key: **M1** = Rust parity + hardening (no release). **M2** = public baseline. **Later** = after the baseline ships. "Have" means present in 0.1.0 and ported for parity.

### Table Stakes (Users Expect These)

#### A. Sync semantics: what users treat as data loss

| # | Feature | Why Expected | Complexity | Milestone | Notes / Depends on |
|---|---|---|---|---|---|
| T1 | Three-way sync against a persistent base snapshot | The standard model: Unison keeps "a record of the contents of each path after each successful synchronization" [DOCS]. Without a base, "they changed it" and "I changed it" cannot be told apart. | LOW (port) | Have → M1 | Black-box harness |
| T2 | A conflict never overwrites either side; both versions are kept | Unison leaves both replicas as they are [DOCS]. Syncthing keeps the loser as `<name>.sync-conflict-<date>-<time>-<by>.<ext>` [DOCS]. wsx's `.wsx-remote` sidecar is the Syncthing model, written next to the file without propagating it. | LOW | Have → M1 | — |
| T3 | Modify-vs-delete is a conflict in both directions, never a silent delete | Syncthing: if a delete wins, the modified file is kept as a conflict copy [DOCS]. 0.1.0 already classifies it as `conflict` when the base differs from both sides [CODE-READ, `plan()` L406-414]. What the conflict sidecar holds when the source is absent is not covered. | LOW | Have → M1 (needs a harness case for each direction) | T2 |
| T4 | Missing or corrupt base degrades to "no history", never to deletions | Unison: missing archives are treated "as though the replicas had both been completely empty" [DOCS]. 0.1.0 does the same: `load_base()` returns `{}` on missing/corrupt input, so a one-sided file is copied and a differing one conflicts [CODE-READ]. The `base.json` v2 upgrade must keep this property. | LOW | Have → M1 (test it, including a v1→v2 upgrade failure) | base.json v2 |
| T5 | Unreadable ≠ absent | A read error (EACCES, EIO, lock) planned as a deletion is the known reproduced data-loss bug. | LOW–MED | **M1 fix** | Stat cache must keep the same three states |
| T6 | `lww` uses the base for existence, backs up before every destructive op, and breaks ties deterministically | Syncthing's newest-wins still keeps the loser as a conflict copy and breaks mtime ties by device ID [DOCS]. Newest-wins that deletes never-synced files, or tie-breaks into a push, is data loss. | LOW | **M1 fix** | T7 |
| T7 | **Back up before every destructive write, whatever the policy and on either side** (overwrite, delete-local, delete-remote, `pull` discards) | Users of sync tools expect a recovery path for anything the tool deletes or overwrites (Syncthing versioning, Unison `backup`/`maxbackups` default 2 [DOCS]). 0.1.0 backs up only `lww` overwrites. | MED | **M1 (recommended)**. Goes beyond the PROJECT Active list, which says "lww local delete" only. | Unique backup stamps; retention (M2) becomes urgent once this lands |
| T8 | **Mass-deletion brake** | `git clean -fdX` (agents run it), a vanished managed root, or a source worktree on the wrong commit turns into "delete everything" on the next hook. Unison has a `mountpoint` preference precisely to guard "against accidental deletions" when a root is missing [DOCS], and a `confirmbigdeletes` preference (MEDIUM: training knowledge, not fetched). Non-interactive version: if a manifest root is wholly absent on one side, or more than a threshold of tracked files would be deleted in one run, hold those deletions, report them like conflicts, and require an explicit flag. | MED | **M1 (recommended)**. Core Value; not on the PROJECT Active list. | T7 (backups make the brake's threshold less critical, but they are not a substitute) |
| T9 | A pre-copied file with no base entry, present on both sides and differing, is a conflict, never a push | Claude Code, Conductor and worktrunk copy ignored files from the **main checkout** at creation [DOCS]. If wsx's paths overlap, the worktree starts with a stale copy and no base. Pushing it would clobber the source. 0.1.0 already returns `conflict` [CODE-READ]. | LOW | Have → M1 (harness case) | — |
| T10 | `--dry-run` uses the same planner | Expected of any destructive sync tool. | LOW | Have → M1 | plan/apply split |
| T11 | Status visibility (`status`, `list`, conflicts listed with a next action) | Unison and Syncthing both make unresolved items visible. | LOW | Have → M1 | — |

#### B. Hardening expected of a sync tool

| # | Feature | Why Expected | Complexity | Milestone | Notes / Depends on |
|---|---|---|---|---|---|
| T12 | Atomic writes: temp file in the destination directory + rename, unique temp names, temp files never synced | Unison and Syncthing both write to a temp file and move it into place, and Syncthing excludes its own `.syncthing.*.tmp` from sync [DOCS]. 0.1.0 uses a fixed `.wsx-tmp` name and `walk()` does not skip it [CODE-READ]. | LOW | **M1** | — |
| T13 | Locking between concurrent runs | Hooks in several worktrees, an agent's turn-end push and a commit all race on the source worktree. Use an OS advisory lock (`flock`, released by the kernel on crash, so it cannot go stale the way Unison's lock files can [DOCS: "may leave the archives … locked"]). Scope: the repo common dir. In hook mode, wait a bounded time and then skip with a message (fail open). The guard is read-only and should not take the lock. | MED | **M1** | Prerequisite for T17, D5, D6 |
| T14 | Crash safety: an interrupted apply never causes loss on the next run | Unison: "safe to interrupt … at any time" [DOCS]. In wsx this holds by construction if "both sides equal → converged" is checked before any classification [CODE-READ L380]. Prove it with crash-injection tests in the harness, not by argument. | MED | **M1** (test); recovery command M2 | T12, harness |
| T15 | Stat cache `(size, mtime_ns, inode, ctime)` → hash, with racy-entry handling | Unison fastcheck uses inode + mtime [DOCS]; Syncthing rehashes on any change to mtime, size or permissions [DOCS]. **Racy-git rule** [DOCS]: an entry whose mtime is ≥ the cache-write time must be rehashed. Record the destination's stat *after* the write. `copy2` preserves the source mtime, so the source's stat must not be reused. | MED | **M1** | base.json v2; T5 |
| T16 | Non-interactive; hooks fail open; the guard decides deliberately | Hooks run inside git and agent runtimes. 0.1.0's guard **fails open on internal errors** [CODE-READ, CONCERNS]. Recommend that only the guard fail closed on internal error. That changes 0.1.0 behaviour, so the requirements step should decide it. | LOW | M1 | — |
| T17 | Interrupted-sync recovery and richer `doctor` (stale temp files, lock holder, base version, hook arming per git hook and per agent runtime, Codex trust state) | Syncthing keeps failed temp files for a day; Unison leaves `DANGER.README` and warns on the next run [DOCS]. | MED | **M2** (per PROJECT) | T12, T13 |
| T18 | Backup retention / prune (keep N per path + max age; dry-run) | Unison `maxbackups` default 2 [DOCS]; Syncthing has staggered versioning. 0.1.0 backups grow without bound [CONCERNS]. | LOW–MED | **M2** (per PROJECT). Pull to M1 if T7 lands, since T7 multiplies the backup volume. | Unique stamps (M1) |

#### C. Commit guard and git hook integration

| # | Feature | Why Expected | Complexity | Milestone | Notes / Depends on |
|---|---|---|---|---|---|
| T19 | Guard on `pre-commit` and `pre-push`, **including a first push** | A guard that skips the most common push (`push -u` of a new branch) is not a guard. | LOW | **M1 fix** | Under husky's sourced init, stdin is shared and must not be read, so fall back to `<base_ref>..HEAD` / `HEAD --not --remotes`. |
| T20 | Guard **without husky**: git ≥ 2.54 per-worktree config hooks, fallback for older git | Most repos have no husky, and a public tool cannot assume npm. [LOCAL] showed per-worktree `hook.<name>.event/command` coexists with `core.hooksPath`, gets real `pre-push` stdin, blocks on non-zero exit, and does not fire in other worktrees. Fallback for git < 2.54: pre-commit framework refuses to install when `core.hooksPath` is set ("Cowardly refusing…") and moves existing hooks to `.legacy` [DOCS, source]; lefthook supports a personal `lefthook-local.yml` that is meant to be globally gitignored [DOCS]. Hooks in `.git/hooks` are shared by every worktree, so the guard would also run in the source worktree. That is harmless (the guard allows the source branch), but the script must chain any existing hook. | MED | **M2** | Requires git ≥ 2.54 detection; with config hooks, reading stdin for exact pre-push refs is safe |
| T21 | Self-heal on any wsx run (re-arm ignore rules and guard) | Worktrees created by other tools (`claude --worktree`, worktrunk, Conductor, bare `git worktree add`) must become correct without a memory step. | LOW | Have → M1 | — |
| T22 | Inherit a custom global `core.excludesFile` | Silently dropping a user's global ignores is a regression users notice. | LOW | **M1 fix** | — |

#### D. Agent integrations and distribution (public baseline)

| # | Feature | Why Expected | Complexity | Milestone | Notes / Depends on |
|---|---|---|---|---|---|
| T23 | Session hooks for Claude Code, Copilot CLI, Codex CLI and Antigravity CLI; Gemini CLI optional | PROJECT names all four as required for the public release. | MED–HIGH | **M2** | T24, T25, D5 |
| T24 | Adapters read the worktree path from hook stdin (`cwd` / `workspacePaths`), never from process cwd or `CLAUDE_PROJECT_DIR` | Finding 2. Without this, desktop-app Claude sessions sync the main checkout. | LOW–MED | **M2** (cheap enough to pull into M1 for the existing Claude/Copilot integrations) | Reading stdin is safe here; agent hooks own their stdin |
| T25 | Idempotent install/uninstall into **user-level** config only, merging without clobbering existing hooks; a stable command string | Existing users have other hooks (the author's machine has GSD and herdr hooks). Codex hash-pins trust, so a changing string forces re-trust. | MED | **M2** | Per-runtime format differences: named-group JSON for Antigravity, `version:1` for Copilot, TOML-or-JSON for Codex, ms timeouts for Gemini |
| T26 | Generalised defaults: base ref from `origin/HEAD` or config, no husky/npm assumption, no `npm install` from hooks | Team-specific `origin/devtest` and `npm install` in hooks are not acceptable in a public tool (security: lifecycle scripts run from whatever branch is checked out) [CONCERNS]. | LOW–MED | **M2** | — |
| T27 | Distribution: static Linux binaries (x86_64/aarch64, musl), `cargo install`, install script with uninstall, shell completions | Baseline for any CLI release. | MED | **M2** | — |
| T28 | Machine-readable output (`status --json`, `list --json`) | Agents and statuslines consume wsx state; scraping prose breaks. | LOW | **M2** | — |

### Differentiators (Competitive Advantage)

All of these serve the Core Value or the headline agent use case. Don't add more.

| # | Feature | Value Proposition | Complexity | Milestone | Notes |
|---|---|---|---|---|---|
| D1 | Continuous two-way, base-tracked sync | Every competitor copies once from the main checkout [DOCS/3P]. wsx keeps edits from N agent worktrees flowing back to one source. | — | Have | Core |
| D2 | Manifest-keyed commit/push guard | No worktree tool blocks personal files from reaching team branches. | — | Have | — |
| D3 | Push-before-remove (`wsx rm`, fish wrapper) | `git worktree remove` and agent auto-cleanup delete unpushed personal edits. | — | Have | See D6, finding 5 |
| D4 | **Setup at worktree creation through a repo-level `post-checkout` config hook** (git ≥ 2.54) | Any tool that runs `git worktree add` (Claude `--worktree`, worktrunk, Conductor, by hand) gets ignores, guard and first pull at creation, with no tracked file and no husky. [LOCAL] verified it fires in the new worktree, with `--no-checkout` as the negative control. | LOW–MED | **M2** | Whether Claude Code's own worktree creation (which neutralises repo filter drivers) runs `post-checkout`: **UNVERIFIED**. |
| D5 | **Turn-level push** on `Stop` / `agentStop` / `AfterAgent` / Antigravity `Stop{fullyIdle}`; session end stays best-effort | Finding 1: the only runtime-independent way to bound loss to one turn. Cheap only with the stat cache. | MED | **M2** | Needs T13 (concurrent pushes from parallel agents) and T15 (hashing on every turn would be too slow) |
| D6 | Detached session-end push (`setsid` + double fork, lock-protected) | Survives Claude's 1.5 s and Codex's 1–3 s budgets and Gemini not waiting, without asking users to raise timeouts. | LOW–MED | M2 | Needs T13. Whether a detached child survives the runtime killing its process group: **UNVERIFIED** per runtime. |
| D7 | Conflict notice injected into the agent's context at session start (`additionalContext` / stdout) | Stops the agent editing a file that is in conflict, or writing over the `.wsx-remote` decision. Supported by Copilot `sessionStart` [DOCS], Claude `SessionStart` stdout, and Codex `additionalContext` (MEDIUM: [DOCS] mentions `additionalContextLimit`; payload shape from [3P]). | LOW | M2 or Later | Output only when there is something to say; otherwise stay silent |
| D8 | `wsx doctor --agents`: per-runtime install, version support and trust check | Codex silently skips untrusted hooks and Antigravity < 1.4 may lack `SessionStart`, so a hook that does not fire is the most likely support issue. | LOW–MED | M2 | T17 |
| D9 | Symlink-layout migration | Existing users of the symlink approach (git-worktree-share style) move without losing data. | — | Have | Untested landmines; M1 harness |
| D10 | Overlap warning with `.worktreeinclude` / `wt step copy-ignored` | Two tools managing one path produce noisy first-sync conflicts (T9). | LOW | M2 (doctor) | — |
| D11 | SQLite-aware `lww` (copy the db together with its `-wal`/`-shm`, or use the backup API; skip while a write lock is held) | Agent state is often SQLite. A byte copy of a live WAL database can be inconsistent. | HIGH | Later | Until then, document that `lww` copies plain bytes. Live-copy corruption risk is [training knowledge], not verified here. |

### Anti-Features (Commonly Requested, Often Problematic)

| Feature | Why Requested | Why Problematic | Alternative |
|---|---|---|---|
| Automatic content merge with conflict markers written into the user's file | git and chezmoi `merge` do three-way merges [DOCS] | Agents read, act on and commit marker-laden files. JSON/YAML state is corrupted. It is irreversible without a backup. | Keep the sidecar and `wsx resolve`. Later, maybe an opt-in `resolve --merge` that runs `git merge-file` into a *separate* output file. |
| Newest-mtime-wins for `sync` paths | "Just make it work" | mtimes are preserved by copies and tie often [CONCERNS]; clocks skew across worktrees on network filesystems. Even Syncthing keeps the loser [DOCS]. | `lww` stays an explicit per-path opt-in, with backups. |
| Interactive prompts (chezmoi `apply` prompts on externally modified targets [DOCS]) | Safety | wsx runs in git hooks and agent hooks with no TTY. | Hold and report (conflict, mass-delete brake), plus explicit flags. |
| Filesystem-watcher daemon (Syncthing style) | "Real-time" sync | A resident process per repo, a missed-event class of bugs (Syncthing itself recommends keeping full rescans [DOCS]), and lifecycle management. | Git hooks + turn-level agent hooks + stat cache. |
| Symlink mode | Zero-copy simplicity | One file with many names: no per-worktree version, so no conflict detection; tools that don't follow links break. The README already rejects it. | Real copies (have). |
| Writing anything shared or tracked: `.husky/`, `.github/hooks/`, `.pre-commit-config.yaml`, project `.claude/settings.json`, repo `.codex/`, `.agents/hooks.json` | Easiest install path | Leaks a personal tool into the team. Copilot also reads project `.claude/settings.json`, so it would double-fire. The Copilot **cloud** agent reads only `.github/hooks/` [DOCS], so supporting it would require a tracked file. | User-level agent config, per-worktree git config, `.git/info/exclude.wsx-*`. Explicitly do not support Copilot cloud agent. |
| Relying on session-end hooks as the only push | Matches the mental model "save on exit" | Finding 1: budgets of 1–3 s, not awaited, or no such event. | D5 + D6, plus git-hook and `wsx rm` pushes. |
| Bypassing Codex hook trust (`--dangerously-bypass-hook-trust`, writing managed `requirements.toml`) | Zero-touch install | Defeats a security control the user relies on, and is surprising for an OSS tool. | Print the one-time `/hooks` trust instruction; `doctor --agents` detects untrusted hooks. |
| Auto-committing on the `personal-workspace` branch | Free version history as backup | Commits on the user's branch without consent; runs that branch's own hooks inside wsx's hooks. | Backups (T7, T18). Later, maybe an opt-in `wsx snapshot`. |
| Server-side or CI enforcement of the guard | "`--no-verify` bypasses it" | Not a personal tool's job; it needs team buy-in and tracked config. | Per-worktree excludes keep paths untracked; doctor reports an unarmed guard. |
| Cross-machine sync | "Sync my notes everywhere" | Network transport, auth and clock skew: a different product. | The source is a git branch, so the user pushes `personal-workspace` to a private remote. |
| Running `npm install` / arming husky from hooks | Convenience in 0.1.0 | Runs lifecycle scripts from whatever branch is checked out [CONCERNS]. | Config-based hooks (T20) remove the need. |

---

## Feature Dependencies

```
Black-box harness (M1) ──required by──> every M1 fix and parity claim
Hermetic test env (HOME/XDG/GIT_CONFIG_GLOBAL) ──required by──> guard, hook, excludesFile tests

base.json v2 ──required by──> Stat cache (T15)
Unreadable≠absent (T5) ──must be preserved by──> Stat cache (T15)
Unique backup stamps ──required by──> Backup-all-destructive (T7) ──makes urgent──> Retention (T18)
Atomic unique temp files (T12) ──required by──> Crash-safety tests (T14) ──enables──> Recovery/doctor (T17)

Locking (T13) ──required by──> Turn-level push (D5)
                └─required by──> Detached session-end push (D6)
                └─required by──> Multi-runtime hooks (T23; Copilot may double-fire with Claude config)
Stat cache (T15) ──required by──> Turn-level push (D5)

git ≥ 2.54 detection ──required by──> Guard without husky (T20)
                     └─required by──> Creation-time setup via post-checkout (D4)
Guard without husky (T20) ──enhances──> First-push guard (T19): exact refs from stdin

Stdin payload parsing (T24) ──required by──> all agent adapters (T23)
                            └─required by──> Conflict notice injection (D7)
Runtime version/trust detection ──required by──> doctor --agents (D8), Antigravity SessionStart gating

Guard fails closed on internal error (T16) ──conflicts with──> "hooks never block" constraint (scope it to the guard only)
Reading pre-push stdin ──conflicts with──> husky sourced init (shared stdin); safe only in config-hook path
.worktreeinclude / copy-ignored overlap ──conflicts with──> clean first sync (T9 keeps it safe, D10 warns)
```

### Dependency Notes

- **Locking and the stat cache are milestone 1, not "hardening polish".** Every milestone 2 agent feature that bounds data loss (D5, D6) multiplies concurrency and invocation frequency. Without them, M2 has to choose between slow hooks and races.
- **T7 (backup-all-destructive) and T8 (mass-deletion brake)** are my recommendations from the Core Value. They are not on the PROJECT Active list. Requirements should accept or reject them explicitly.
- **D4 and T20 share one mechanism** (git config hooks) and should be one phase, with a fallback path for git < 2.54.
- **T24 is cheap and fixes a real bug** in the existing Claude integration (desktop-app sessions run hooks in the main checkout). It can move into M1 without breaking parity, because 0.1.0's own behaviour there is wrong.

---

## MVP Definition

### Milestone 1: Rust parity + hardening (no release)

- [ ] Parity: T1–T4, T9–T11, T21, D1–D3, D9, under the black-box harness against the Python oracle
- [ ] Data-loss fixes: T5, T6, stale `""` base entry after a both-sides delete
- [ ] T7 backup before every destructive write *(recommended addition)*
- [ ] T8 mass-deletion brake *(recommended addition)*
- [ ] T12 atomic unique temp files; T13 `flock` locking with a bounded wait in hook mode
- [ ] T14 crash-injection tests; T15 stat cache with racy-entry rule; base.json v2 upgrade
- [ ] T19 first-push guard; T22 global excludesFile; T16 a decision on the guard's internal-error behaviour
- [ ] Optional pull-in: T24 stdin `cwd` for the existing Claude/Copilot adapters

### Milestone 2: public baseline

- [ ] T20 + D4 config-based git hooks (guard + creation-time setup), with a pre-2.54 fallback
- [ ] T23 + T25 adapters for Claude Code, Copilot CLI, Codex CLI, Antigravity CLI (Gemini CLI: decide; adding it is cheap)
- [ ] D5 turn-level push + D6 detached best-effort session-end push
- [ ] T17 recovery/doctor + D8 `doctor --agents`; T18 retention/prune
- [ ] T26 generalised defaults; T27 distribution; T28 `--json`
- [ ] D10 overlap warning

### Later

- [ ] D7 conflict notice in agent context (could slip into M2 if cheap)
- [ ] D11 SQLite-aware `lww`
- [ ] Opt-in `resolve --merge` into a separate file; opt-in snapshot commits
- [ ] Claude `WorktreeCreate` hook adapter (replaces git creation and disables `.worktreeinclude` [DOCS], so it is risky; only if D4 turns out not to fire for Claude-created worktrees)
- [ ] macOS/Windows

---

## Feature Prioritization Matrix

| Feature | User Value | Implementation Cost | Priority |
|---|---|---|---|
| T5/T6 data-loss fixes | HIGH | LOW | P1 (M1) |
| T8 mass-deletion brake | HIGH | MEDIUM | P1 (M1, recommended) |
| T7 backup-all-destructive | HIGH | MEDIUM | P1 (M1, recommended) |
| T13 locking | HIGH | MEDIUM | P1 (M1) |
| T15 stat cache | MEDIUM now, HIGH for D5 | MEDIUM | P1 (M1) |
| T12 atomic temp / T14 crash tests | HIGH | LOW / MEDIUM | P1 (M1) |
| T19 first-push guard | HIGH | LOW | P1 (M1) |
| T20 + D4 config-based hooks | HIGH | MEDIUM | P1 (M2) |
| T23–T25 four agent adapters | HIGH (headline) | MEDIUM–HIGH | P1 (M2) |
| D5 turn-level push | HIGH | MEDIUM | P1 (M2) |
| D6 detached session-end push | MEDIUM | LOW–MEDIUM | P2 (M2) |
| T17 / D8 doctor | MEDIUM | MEDIUM | P2 (M2) |
| T18 retention | MEDIUM | LOW | P2 (M2) |
| T27 distribution | HIGH for adoption | MEDIUM | P1 (M2) |
| T28 `--json` | MEDIUM | LOW | P2 (M2) |
| D7 conflict notice | MEDIUM | LOW | P3 |
| D11 SQLite-aware lww | MEDIUM | HIGH | P3 |

---

## Competitor Feature Analysis

| Capability | Claude Code `.worktreeinclude` / Conductor | worktrunk (`wt`) | Syncthing | Unison | chezmoi | git-env / git-worktree-share | **wsx** |
|---|---|---|---|---|---|---|---|
| Direction | One-shot copy main → new worktree, gitignored matches only [DOCS] | `wt step copy-ignored` in a `post-start` hook [DOCS]; one-shot (details UNVERIFIED) | Continuous both ways | On-demand both ways | Source → target | git-env: primary → linked, skips differing files unless `--force` [3P]; share: symlinks into `.git/shared` [3P] | Continuous both ways, per-path policy |
| Base / history | None | None | Version vectors + index | Archive per path [DOCS] | Persistent state | None | `base.json` |
| Conflict | n/a | n/a | Older mtime renamed to `.sync-conflict-…` [DOCS] | Skipped, both kept [DOCS] | Prompt on `apply`; `merge` three-way via tool [DOCS] | git-env: skip + warn (safe default) | Sidecar `.wsx-remote` + `resolve` |
| Backups | n/a | n/a | Versioning (trashcan/staggered) | `backup`, `maxbackups` default 2 [DOCS] | n/a | n/a | `lww` only (M1: all destructive ops, recommended) |
| Crash safety | n/a | n/a | Temp + rename, temp kept 1 day [DOCS] | Temp + atomic move; `DANGER.README` [DOCS] | — | — | M1: temp + rename, lock, crash tests |
| Change detection | n/a | n/a | Watcher + hourly rescan; mtime/size/perm → rehash [DOCS] | fastcheck: inode + mtime [DOCS] | — | — | M1: stat cache with racy rule |
| Commit guard | No | No (has `pre-commit` lifecycle hooks of its own) | n/a | n/a | n/a | No | Yes |
| Hook trust model | — | Project hooks need approval, stored in `approvals.toml` [DOCS] | — | — | — | — | User-level only; nothing to approve in the repo |
| Agent session hooks | Claude-native | No | No | No | No | wrktreez uses Claude `WorktreeCreate` [3P] | 4 runtimes (M2) |

---

## Sources

Agent runtimes (official, HIGH):
- Claude Code hooks reference: https://code.claude.com/docs/en/hooks
- Claude Code worktrees (`.worktreeinclude`, cleanup, hooks `cwd` vs `CLAUDE_PROJECT_DIR`, desktop-app hook root): https://code.claude.com/docs/en/worktrees
- GitHub Copilot CLI hooks reference: https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-hooks-reference
- GitHub Copilot hooks concepts: https://docs.github.com/en/copilot/concepts/agents/hooks
- OpenAI Codex hooks (redirected from developers.openai.com/codex/hooks): https://learn.chatgpt.com/docs/hooks
- Gemini CLI hooks overview: https://geminicli.com/docs/hooks/ and reference: https://geminicli.com/docs/hooks/reference/
- Antigravity hooks: https://antigravity.google/docs/hooks

Agent runtimes (third-party, LOW; used only where flagged [3P]):
- claude-mem Antigravity CLI setup (agy 1.2.1 event names): https://docs.claude-mem.ai/antigravity-cli/setup
- Antigravity IDE hooks-not-firing report: https://discuss.ai.google.dev/t/stop-and-posttooluse-hooks-in-agents-hooks-json-never-fire-antigravity-ide-1-107-0-windows/178288

Git hooks (HIGH):
- Git 2.54 highlights (config-based hooks): https://github.blog/open-source/git/highlights-from-git-2-54/
- `git hook` / `hook.*` config: https://www.kernel.org/pub/software/scm/git/docs/git-hook.html
- Racy git: https://github.com/git/git/blob/master/Documentation/technical/racy-git.adoc
- pre-commit `core.hooksPath` refusal and `.legacy` migration (source): https://github.com/pre-commit/pre-commit/blob/main/pre_commit/commands/install_uninstall.py
- lefthook local config: https://lefthook.dev/configuration/
- Local probe: git 2.56.0, hermetic (`GIT_CONFIG_GLOBAL=/dev/null`, `GIT_CONFIG_NOSYSTEM=1`, scratch `HOME`), per-worktree and repo-level config hooks with controls (main-worktree commit; `--no-checkout` add). Scratch repo deleted.

Sync tools (HIGH for official docs):
- Syncthing syncing/conflicts/temp files/scanning: https://docs.syncthing.net/users/syncing.html
- Unison manual (archive, conflicts, backups, fastcheck, interruption, mountpoint): https://github.com/bcpierce00/unison/blob/master/doc/unison-manual.tex
- chezmoi `merge`: https://www.chezmoi.io/reference/commands/merge/ and `apply`: https://www.chezmoi.io/reference/commands/apply/

Worktree tools (MEDIUM):
- worktrunk hooks: https://worktrunk.dev/hook/
- Conductor `.worktreeinclude`: https://www.conductor.build/docs/reference/worktreeinclude
- git-env: https://pypi.org/project/git-env/ ; wrktreez: https://pypi.org/project/wrktreez/ ; git-worktree-share: https://github.com/keithamus/git-worktree-share ; worktree-setup: https://docs.rs/worktree-setup/

Project inputs: `.planning/PROJECT.md`, `README.md`, `.planning/codebase/CONCERNS.md`, `bin/wsx` (`plan()`, `load_base()`), `integrations/claude/settings-snippet.json`, `integrations/copilot/wsx-session.json`.

---
*Feature research for: personal-workspace sync across git worktrees for AI-agent users*
*Researched: 2026-10-17*
