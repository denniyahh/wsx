---
last_mapped_commit: 5e4217fb6a940bf5ca5f53685fd2245097150676
last_mapped_at: 2026-10-17
---
# Codebase Structure

**Analysis Date:** 2026-10-17

## Directory Layout

```
wsx/
├── bin/
│   └── wsx                         # The tool: one extensionless Python 3 file, stdlib only (~1181 lines)
├── examples/
│   └── wsx.conf                    # Annotated manifest template (copy to .wsx.conf on personal-workspace)
├── integrations/
│   ├── husky-init.sh               # Install to ~/.config/husky/init.sh; calls `wsx hook "$n"`
│   ├── claude/settings-snippet.json   # Claude Code SessionStart/SessionEnd hooks (merged by install.sh)
│   └── copilot/wsx-session.json    # Copilot sessionStart/sessionEnd hooks
├── shell/fish/
│   ├── completions/wsx.fish        # fish completions
│   └── functions/git.fish          # `git` wrapper: routes `git worktree remove` to `wsx rm`
├── tests/
│   └── sandbox.sh                  # End-to-end suite against a mktemp repo
├── install.sh                      # Installer (symlink default, --copy, --uninstall, --dry-run)
├── README.md / CONTRIBUTING.md / CLAUDE.md / CHANGELOG.md / LICENSE
└── .gitignore
```

## Directory Purposes

**`bin/`:**
- Purpose: The executable. Everything functional lives in `bin/wsx`.
- Key files: `bin/wsx`

**`bin/wsx` internal sections (in dependency order, delimited by `# ----` banner comments):**
- constants + `Problem` (`:55-70`)
- git helpers (`:77-114`)
- `Context` (`:122-229`)
- file plumbing (`:237-338`)
- planning: `Action`, `plan`, `apply` (`:346-488`)
- ignore + guard (`:496-606`)
- commands `cmd_*` and helpers (`:614-1092`)
- entry point `build_parser`, `main` (`:1100-1181`)

**`integrations/`:**
- Purpose: Glue that makes external systems call `wsx hook <name>`; each must fail open (`; true`, `printf '{}'`).

**`shell/fish/`:**
- Purpose: Interactive-only fish additions, installed by `install.sh`.

**`tests/`:**
- Purpose: Bash sandbox; tests `bin/wsx` from this checkout via a PATH shim.

## Key File Locations

**Entry Points:**
- `bin/wsx`: CLI (`main`, `bin/wsx:1159`)
- `install.sh`: machine install
- `integrations/husky-init.sh`: git hook entry

**Configuration:**
- `examples/wsx.conf`: manifest format reference
- Runtime git config keys: `wsx.sourceBranch`, `wsx.baseRef`
- No project config files for linting/formatting detected

**Core Logic:**
- `bin/wsx` `plan()` / `apply()`: sync semantics
- `bin/wsx` `install_ignore()` / `ensure_installed()`: ignore rules
- `bin/wsx` `cmd_guard()` / `cmd_hook()`: commit guard

**Testing:**
- `tests/sandbox.sh`

## Naming Conventions

**Files:**
- Executables extensionless in `bin/` (`bin/wsx`); shell scripts `.sh`; fish files named after the function/command they define (`git.fish`, `wsx.fish`).

**Functions (in `bin/wsx`):**
- `cmd_<subcommand>` for each argparse subcommand, registered via `add(...)` in `build_parser()`.
- snake_case helpers; `require_*` for precondition checks on `Context`.
- Constants UPPER_SNAKE at top of file.

**Directories:**
- `integrations/<tool>/` per external tool.

## Where to Add New Code

**New subcommand:**
- Implementation: new `cmd_<name>(ctx, args) -> int` in the commands section of `bin/wsx`, registered in `build_parser()` (`bin/wsx:1100`)
- Completions: `shell/fish/completions/wsx.fish`
- Docs: module docstring in `bin/wsx` (it is the `--help` text) and `README.md`
- Tests: new case in `tests/sandbox.sh`

**New sync rule / policy:**
- Decision logic: `plan()` only (`bin/wsx:356`); add policy name to `POLICIES` (`bin/wsx:58`)
- Execution of a new action kind: `apply()` (`bin/wsx:419`), keeping base updates
- Docs: `examples/wsx.conf`, module docstring

**New hook name:**
- `cmd_hook()` (`bin/wsx:1022`); keep it non-raising and stdin-free

**New integration:**
- `integrations/<tool>/`, wired in `install.sh` (install + uninstall branches)

**Utilities:**
- File plumbing section of `bin/wsx`. Do not split into modules or add dependencies.

## Special Directories

**`<git-dir>/wsx/` (in managed repos, not this repo):**
- Purpose: `base.json` snapshot, `backups/`
- Generated: Yes; Committed: No

**`<common-dir>/info/exclude.wsx-<id>` (in managed repos):**
- Purpose: generated per-worktree ignore file
- Generated: Yes; Committed: No

**`.planning/`:**
- Purpose: GSD planning docs
- Generated: by GSD tooling; Committed: not currently tracked (`git ls-files` shows none)

---

*Structure analysis: 2026-10-17*
