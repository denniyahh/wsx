---
last_mapped_commit: 5e4217fb6a940bf5ca5f53685fd2245097150676
last_mapped_at: 2026-10-17
---
# External Integrations

**Analysis Date:** 2026-10-17

## APIs & External Services

No network APIs or remote services. All integrations are local: git, hook runners, agent CLIs, and the fish shell.

**Git (core integration):**
- `git` CLI via `subprocess` - `git()`, `git_ok()`, `list_worktrees()` in `bin/wsx`
  - Reads `git worktree list --porcelain` to find the worktree on `wsx.sourceBranch`
  - Writes only per-worktree config: `extensions.worktreeConfig`, `git config --worktree core.excludesFile <generated file>`; refuses on bare repos (`bin/wsx` ~lines 560-567)
  - Never writes tracked files (`.gitignore`, `.husky/`) - see `CLAUDE.md` landmines

**Git hooks via husky:**
- `integrations/husky-init.sh` installed to `${XDG_CONFIG_HOME:-~/.config}/husky/init.sh` (symlinked, or appended with `.pre-wsx.bak` backup if one exists)
  - Husky sources it before every hook with `$n` = hook name; it runs `~/.local/bin/wsx hook "$n" </dev/null || exit 1`
  - `</dev/null` is load-bearing: pre-push reads refs from stdin
- Hook dispatch in `cmd_hook` (`bin/wsx` ~lines 1039-1063):
  - `pre-commit` - guard managed paths, then sync out
  - `pre-push` - guard across outgoing range, then sync out
  - `post-merge`, `post-checkout`, `post-rewrite` - pull into worktree
  - Fails open on everything except the guard; `cmd_hook` must not raise
- Husky arming detected by presence of `.husky/_/pre-commit`; `wsx new` runs `npm install` to arm it

**Claude Code session hooks:**
- `integrations/claude/settings-snippet.json` - `SessionStart` -> `wsx hook session-start`, `SessionEnd` -> `wsx hook session-end`; 30s timeout, output discarded, `; true` so it never fails the session
- `install.sh` merges it into `~/.claude/settings.local.json` with inline Python, skipping events that already mention `wsx`, and skipping invalid JSON. Uninstall leaves this file in place.
- `session-start` also arms the commit guard if the worktree lacks husky

**GitHub Copilot CLI session hooks:**
- `integrations/copilot/wsx-session.json` installed to `~/.copilot/hooks/wsx-session.json` (only if `~/.copilot` exists)
  - Schema `version: 1`, `sessionStart`/`sessionEnd`, `bash` command, `timeoutSec: 30`, prints `{}`
  - `sessionStart` also runs `$HOME/.config/agents/render-agent-rules` - an external, user-specific script not in this repo

**Fish shell:**
- `shell/fish/functions/git.fish` - wraps `git`; intercepts only `git worktree remove` in repos with a `.wsx.conf` on the source branch and routes it to `wsx rm` (push first), mapping `-f` to `--force`. Interactive only; scripts call real git.
- `shell/fish/completions/wsx.fish` - completions
- Installed to `${XDG_CONFIG_HOME:-~/.config}/fish/` only if that dir exists

**npm:**
- `npm install --silent --no-audit --no-fund` in `wsx new` (`bin/wsx` ~lines 826-849); restores `package-lock.json` bytes afterwards

## Data Storage

**Databases:**
- None. JSON snapshot `<git-dir>/wsx/base.json` holds the three-way sync base.

**File Storage:**
- Local filesystem only: copies between source and dev worktrees; backups under `<git-dir>/wsx/backups/`; conflict sidecars `<file>.wsx-remote`.

**Caching:**
- None

## Authentication & Identity

**Auth Provider:**
- None. Relies on the user's existing git setup.

## Monitoring & Observability

**Error Tracking:**
- None

**Logs:**
- Stdout/stderr text with ANSI colour; `wsx doctor` reports configuration health. Hook output is discarded by the agent integrations.

## CI/CD & Deployment

**Hosting:**
- Not applicable; local install via `install.sh` (`--copy`, `--uninstall`, `--dry-run`).

**CI Pipeline:**
- None. No `.github/` workflows; tests run manually with `./tests/sandbox.sh`.

## Environment Configuration

**Required env vars:**
- None required. Optional: `WSX_SKIP=1`, `XDG_CONFIG_HOME`, `HOME`.

**Secrets location:**
- Not applicable; no secrets used.

## Webhooks & Callbacks

**Incoming:**
- Git hooks (via husky) and agent session hooks (Claude Code, Copilot) invoking `wsx hook <name>`.

**Outgoing:**
- None

---

*Integration audit: 2026-10-17*
