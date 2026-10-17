---
last_mapped_commit: 5e4217fb6a940bf5ca5f53685fd2245097150676
last_mapped_at: 2026-10-17
---
# Coding Conventions

**Analysis Date:** 2026-10-17

Authoritative style sources: `CONTRIBUTING.md` (design constraints + Style section) and `CLAUDE.md` (code shape + landmines). Read both before editing `bin/wsx`.

## Naming Patterns

**Files:**
- Main program is an extensionless executable script: `bin/wsx` (Python 3, shebang `#!/usr/bin/env python3`). Do not add a `.py` extension or split into a package.
- Shell scripts use `.sh` and lowercase/hyphenated names: `install.sh`, `tests/sandbox.sh`, `integrations/husky-init.sh`.
- Fish integrations live under `shell/fish/{functions,completions}/<name>.fish`.

**Functions:**
- `snake_case`, short verb or noun phrases: `git()`, `git_ok()`, `list_worktrees()`, `digest()`, `walk()`, `copy_file()`, `materialise_parents()` (British spelling is used; keep it consistent, also `colour` in `cmd_status`).
- Subcommand handlers are `cmd_<subcommand>(ctx: Context, args: argparse.Namespace) -> int` (e.g. `cmd_setup`, `cmd_status`, `cmd_hook` in `bin/wsx`). Register them in `build_parser()` (`bin/wsx` ~line 1100).
- Boolean predicates read as questions: `git_ok`, `is_managed`, `via_symlink`, `ctx.is_source`, `ctx.configured`.

**Variables:**
- `snake_case` locals; short domain names are accepted (`rel` for repo-relative path, `ctx`, `exc`, `h` for a hash object).
- Module constants are `UPPER_SNAKE`: `MANIFEST_NAME`, `DEFAULT_SOURCE_BRANCH`, `CONFLICT_SUFFIX`, `POLICIES`, `IGNORED_BASENAMES` (`bin/wsx` lines 55-59). Add new magic strings here, not inline.
- ANSI colour constants `BOLD, DIM, RED, YELLOW, GREEN, RESET` are empty strings when stdout is not a TTY (`bin/wsx` lines 61-65). Always use these constants; never hard-code escape codes.
- Shell: UPPER_CASE globals (`REPO_ROOT`, `WSX`, `SANDBOX`, `SRC`, `DEV`, `PASS`, `FAIL`).

**Types:**
- Classes are `PascalCase`: `Problem`, `Context`, `Action`.
- Full type hints on every function signature, using modern syntax enabled by `from __future__ import annotations`: `Path | None`, `list[tuple[Path, str]]`, `set[str]`.
- Enumerated values are plain strings, not `Enum`: policies `("sync", "pull", "lww", "skip")`, action kinds `pull | push | delete-local | delete-remote | conflict | converged` (documented in a trailing comment on `Action.kind`).
- `Action` uses `__slots__` with an explicit `__init__` rather than a dataclass. Follow the existing pattern when extending it.

## Code Style

**Formatting:**
- No formatter config committed (no `pyproject.toml`, `ruff.toml`, `.editorconfig`). Style is Black-like: 4-space indent, double quotes, trailing commas in multi-line calls, ~100-char lines tolerated.
- `ruff format --check bin/wsx` (default settings) reports the file would be reformatted. Do not run a formatter over the whole file; it would produce an unrelated diff. Match surrounding style in edits.
- Shell scripts indent with tabs (`tests/sandbox.sh`, `install.sh`).

**Linting:**
- No linter config committed. One inline suppression exists: `# noqa: BLE001 - hooks must fail open` in `main()`. Suppressions carry a reason after the code.
- `ruff check --isolated bin/wsx` (defaults) reports 6 findings: 3x `PLW1510` (subprocess.run without explicit `check`, intentional — return codes are inspected manually in `git()`/`git_ok()`), `RUF023` unsorted `__slots__`, `ISC004`, `RUF100` unused noqa. None are enforced.
- `shellcheck` reports SC2164 (unguarded `cd`) in `tests/sandbox.sh`, SC2209 in `install.sh`, SC2154 in `integrations/husky-init.sh` (`$n` is provided by husky when sourced).

## Import Organization

**Order (in `bin/wsx`):**
1. `from __future__ import annotations`
2. Stdlib `import x` lines, alphabetical: `argparse, hashlib, json, os, re, shutil, subprocess, sys, time`
3. `from pathlib import Path`

**Hard rule:** standard library only (CONTRIBUTING constraint 6). Never add a third-party import.

**Path Aliases:** Not applicable.

## Error Handling

**Patterns:**
- Raise `Problem(message)` (`bin/wsx` line 68) for every user-facing failure. `main()` catches it, prints `wsx: <msg>` in red to stderr and returns exit code `2`.
- `git(*args, check=True)` raises `Problem` with git's stderr on non-zero exit; pass `check=False` when an empty result is an acceptable answer (e.g. reading optional config, `@{upstream}`).
- Use `git_ok(...)` when only success/failure matters.
- Narrow `except` clauses for expected filesystem conditions: `except OSError: return None` in `digest()`, `except (FileNotFoundError, json.JSONDecodeError)` when loading `base.json`.
- Fail-open in hook paths (constraint 2): `cmd_hook`, `quiet_sync` catch `Problem` and print a yellow warning, returning 0. `main()` in hook mode also swallows any `Exception`. The only non-zero hook exit is the guard (`return 1`). Never add a code path in `cmd_hook` that can raise or exit non-zero.
- Exit codes: `0` ok, `1` actionable state (conflicts in `cmd_status`, guard block), `2` `Problem`/refused (e.g. running against the source worktree), `130` KeyboardInterrupt.
- `assert ctx.source is not None` is used for type narrowing after `ctx.require_dev_worktree()`; it is not a runtime check.

## Logging

**Framework:** `print` only — no `logging` module.

**Patterns:**
- Normal output to stdout; warnings/errors to stderr with `file=sys.stderr`.
- Prefix messages with `wsx:` (or `<hook-label>: wsx ...` inside hooks).
- Output is column-aligned (`f"{action.kind:<14}"`), coloured via the TTY-aware constants, and states the next action when reporting a problem (e.g. "run 'wsx status'", "Resolve with wsx resolve <path> --mine").
- Never read stdin anywhere reachable from `wsx hook` (constraint 3).

## Comments

**When to Comment:**
- Comments explain *why*, never *what* (`CONTRIBUTING.md` Style). Every non-obvious constraint names the failure it prevents — model examples: the comment above `ensure_installed` in `cmd_hook` ("without ignore rules ... `git add -A` would sweep them into the index") and `materialise_parents()`.
- Section banners divide the file in dependency order:
  ```python
  # --------------------------------------------------------------------------
  # git helpers
  # --------------------------------------------------------------------------
  ```
  Put new code in the matching section (git helpers, context, file plumbing, planning, ignore + guard, commands, entry point).

**Docstrings:**
- Module docstring at top of `bin/wsx` is the user manual (layout, policies, commands). Update it when adding a command or policy.
- Function docstrings are one or two lines, stating contract and edge cases: `"""sha256 of a regular file; None if absent. Symlinks are not followed."""`. Trivial helpers have none.

## Function Design

**Size:** Mostly under ~40 lines; `plan()`, `apply()` and `cmd_doctor()` are longer.

**Parameters:** First parameter is `ctx: Context` for anything needing repo state. Keyword-only-by-convention flags (`cwd=`, `check=`, `dry_run=`, `announce=`). Paths are `pathlib.Path`; repo-relative paths are `str` named `rel`.

**Return Values:** Commands return `int` exit codes; helpers return `None` for "absent" (`digest`) or empty collections (`walk`). Decision logic returns `list[Action]` from `plan()`; `apply()` only executes — never put decisions in `apply()` (keeps `--dry-run` honest).

## Module Design

**Exports:** Single script; entry is `if __name__ == "__main__": raise SystemExit(main(sys.argv[1:]))`. `main(argv)` takes argv explicitly.

**Barrel Files:** Not applicable.

**Shell scripts:** `install.sh` uses `set -euo pipefail`; `tests/sandbox.sh` uses `set -uo pipefail` (no `-e`, so failing assertions do not abort). Resolve own location via `"$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"`.

**Commits:** Conventional Commits; body explains the failure mode fixed.

---

*Convention analysis: 2026-10-17*
