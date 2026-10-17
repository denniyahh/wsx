---
last_mapped_commit: 5e4217fb6a940bf5ca5f53685fd2245097150676
last_mapped_at: 2026-10-17
---
# Technology Stack

**Analysis Date:** 2026-10-17

## Languages

**Primary:**
- Python 3 (README requires >= 3.9; `from __future__ import annotations`) - the whole tool, `bin/wsx` (single extensionless file, shebang `#!/usr/bin/env python3`)

**Secondary:**
- Bash - installer `install.sh` (`set -euo pipefail`) and test suite `tests/sandbox.sh` (`set -uo pipefail`)
- POSIX sh - husky personal init `integrations/husky-init.sh` (sourced by husky, must not consume stdin)
- Fish - `shell/fish/functions/git.fish` (git wrapper), `shell/fish/completions/wsx.fish`
- JSON - agent hook config snippets `integrations/claude/settings-snippet.json`, `integrations/copilot/wsx-session.json`
- Plain-text manifest format `.wsx.conf` (`<policy> <path>` per line; template `examples/wsx.conf`)

## Runtime

**Environment:**
- CPython 3.9+ (dev host observed: Python 3.15.0)
- git >= 2.31 required (`extensions.worktreeConfig` + `git config --worktree`); dev host observed: git 2.56.0

**Package Manager:**
- None. No `pyproject.toml`, `requirements.txt`, `setup.py`, or `package.json`.
- Lockfile: not applicable
- Installation is via `install.sh` symlinking (default) or copying (`--copy`) into `~/.local/bin` and user config dirs.

## Frameworks

**Core:**
- None. Python standard library only: `argparse`, `hashlib`, `json`, `os`, `re`, `shutil`, `subprocess`, `sys`, `time`, `pathlib` (`bin/wsx` lines 42-53).

**Testing:**
- Custom Bash harness `tests/sandbox.sh` - end-to-end assertions against a throwaway repo under `mktemp -d`; tests `bin/wsx` in-tree, shadowing any installed copy.
- `python3 -m py_compile bin/wsx` as a syntax check (`CONTRIBUTING.md`).

**Build/Dev:**
- No build step. Symlinked install makes edits live immediately.

## Key Dependencies

**Critical:**
- `git` binary - invoked via `subprocess.run(["git", ...])` in `git()`/`git_ok()` (`bin/wsx` ~lines 78-92). All repo state, worktree listing, config, and ignore wiring goes through it.

**Infrastructure (optional external tools):**
- `npm` - `wsx new` runs `npm install --silent --no-audit --no-fund` to arm husky when `package.json` exists, then restores `package-lock.json` (`bin/wsx` ~lines 826-849)
- husky - hook runner that sources `~/.config/husky/init.sh`
- fish shell - interactive git wrapper and completions
- `python3` - also used inline by `install.sh` to merge Claude Code settings JSON

**Constraint:** adding third-party dependencies is prohibited (`CLAUDE.md`, `CONTRIBUTING.md` constraint 6).

## Configuration

**Environment:**
- `WSX_SKIP=1` - bypass hooks and the fish git wrapper (`integrations/husky-init.sh`, `shell/fish/functions/git.fish`)
- `XDG_CONFIG_HOME` - locates global git ignore fallback (`bin/wsx` line 500) and fish/husky install targets (`install.sh`)
- No `.env` files present.

**Git config keys (per repo):**
- `wsx.sourceBranch` - source branch name, default `personal-workspace` (`DEFAULT_SOURCE_BRANCH`, `bin/wsx` line 56)
- `wsx.baseRef` - base for `wsx new`; fallback chain `origin/devtest`, `origin/develop`, `origin/main`, `origin/master` (`bin/wsx` ~line 807)
- `extensions.worktreeConfig=true` and per-worktree `core.excludesFile` written by wsx (`bin/wsx` ~lines 560-567)

**State files:**
- Manifest `.wsx.conf` at source worktree root
- `<git-dir>/wsx/base.json` sync base snapshot; `<git-dir>/wsx/backups/<stamp>/...` LWW/conflict backups

**Build:**
- None. `.gitignore` covers only `__pycache__/`, editor, OS and `*.bak`/`*.orig` files.

## Platform Requirements

**Development:**
- Unix-like OS with bash, git >= 2.31, python3 >= 3.9. Optional fish, husky, npm.

**Production:**
- Personal developer machines; installed into `~/.local/bin/wsx` (must be on `PATH`). Versioned per `CHANGELOG.md` (SemVer, currently 0.1.0).

---

*Stack analysis: 2026-10-17*
