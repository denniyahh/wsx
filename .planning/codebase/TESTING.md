---
last_mapped_commit: 5e4217fb6a940bf5ca5f53685fd2245097150676
last_mapped_at: 2026-10-17
---
# Testing Patterns

**Analysis Date:** 2026-10-17

## Test Framework

**Runner:**
- Custom Bash end-to-end harness: `tests/sandbox.sh` (160 lines). No pytest/unittest, no test config files.
- Verified 2026-10-17: 28 `check` assertions in the file (counted by `rg -c '^\s*check "'`), and a run reported `passed: 28   failed: 0`, exit 0. That shows the suite is green on this host; it does not cover the untested areas listed under Coverage.

**Assertion Library:**
- A single shell function, `check <description> <expected> <actual>`, doing exact string equality and incrementing `PASS`/`FAIL`.

**Run Commands:**

```bash
./tests/sandbox.sh                 # Run all tests (exit 0 iff FAIL == 0)
python3 -m py_compile bin/wsx      # Fast syntax check (creates bin/__pycache__/, gitignored)
```

No watch mode or coverage tooling.

## Test File Organization

**Location:**
- Separate `tests/` directory; one file.

**Naming:**
- `tests/<name>.sh`.

**Structure:**

```
tests/
└── sandbox.sh    # numbered scenario sections "== n. topic ==", sequential shared state
```

## Test Structure

**Suite Organization:**

```bash
check() { # check <description> <expected> <actual>
	if [[ "$2" == "$3" ]]; then
		echo "  PASS  $1"; PASS=$((PASS + 1))
	else
		echo "  FAIL  $1"; echo "          expected: $2"; echo "          actual:   $3"
		FAIL=$((FAIL + 1))
	fi
}

echo "== 4. divergent edit is a conflict =="
echo "plan LOCAL" >.planning/PLAN.md
echo "plan SOURCE" >"$SRC/.planning/PLAN.md"
wsx sync >/dev/null 2>&1
check "conflict reported" "1" "$(wsx status 2>&1 | grep -c 'conflict')"
check "local copy untouched" "plan LOCAL" "$(cat .planning/PLAN.md)"
```

**Patterns:**
- Setup: `mktemp -d` sandbox; `git init -q -b main repo`, commit, create `personal-workspace` branch as source worktree `../source`, write `.wsx.conf` with `sync`, `pull`, `lww` entries, then a dev worktree `../dev` on branch `feature`.
- PATH shim: symlinks `bin/wsx` into a temp dir prepended to `PATH`, so the working-tree copy (not an installed one) is tested, including calls made by hooks.
- Teardown: `trap 'rm -rf "$SANDBOX" "$SHIM"' EXIT`.
- Sections are ordered and build on prior state (CONTRIBUTING: "keep them ordered so later cases can build on earlier state"). New cases go at the end as `echo "== 12. topic =="` unless they need earlier state.
- Command output is discarded (`>/dev/null 2>&1`); assertions inspect file contents, `git status --porcelain`, `grep -o`/`grep -c` on `wsx status`, or exit codes.
- Exit-code assertion idiom:
  ```bash
  GUARD_RC=0
  wsx guard >/dev/null 2>&1 || GUARD_RC=$?
  check "guard blocks managed path on feature branch" "1" "$GUARD_RC"
  ```
- Existence idiom: `"$([[ -e path ]] && echo yes || echo no)"`.
- `sleep 1` before lww edits so mtime ordering is deterministic.

## Mocking

**Framework:** None. Tests run real `git` against real temporary repositories and worktrees.

**What to Mock:**
- Nothing; isolate via temp dirs and PATH shim instead.

**What NOT to Mock:**
- git and the filesystem — the sync logic depends on real worktree, config and symlink behaviour.

**Hermeticity gaps:** The sandbox does not scrub `GIT_DIR`/`GIT_WORK_TREE`/etc., and does not redirect `HOME`/`XDG_CONFIG_HOME`; it sets repo-local `user.email`/`user.name` only. Global git config of the host still applies.

## Fixtures and Factories

**Test Data:**

```bash
echo "plan v1" >"$SRC/.planning/PLAN.md"
printf 'binary-v1' >"$SRC/.state/db.bin"
cat >"$SRC/.wsx.conf" <<'EOF'
sync  .planning
sync  AGENTS.md
pull  .planning/ref.txt
lww   .state
EOF
```

**Location:**
- Inline in `tests/sandbox.sh`. Reference manifest for users: `examples/wsx.conf`.

## Coverage

**Requirements:** None enforced; no coverage tool.

**Covered (sections 1-11):** `setup`, `push`, `pull`, `sync`, `status` (in-sync / conflict), `resolve --mine|--theirs` and `.wsx-remote` cleanup, deletions both directions, `pull` policy, `lww` with backup, `guard` (block / pass / ignore ordinary files), refusal in source worktree (exit 2).

**Not covered by the suite (from grep of commands invoked):**
- `wsx doctor`, `wsx new`, `wsx rm`, `wsx list`, `wsx hook <name>` (no husky in sandbox, so hook paths and fail-open behaviour are untested), `--dry-run`, `skip` policy, `wsx guard` range mode (pre-push), migration from symlinked layouts (`via_symlink()`, `materialise_parents()`), orphan exclude pruning, `install.sh`.
- CLAUDE.md requires a manual pass for ignore/guard/hook changes: `wsx doctor`, `wsx new test/scratch`, `wsx rm test/scratch` in a real repo.

## Test Types

**Unit Tests:** Not used.

**Integration Tests:** `tests/sandbox.sh` — end-to-end CLI against temp git repos.

**E2E Tests:** Same as above; plus documented manual verification. No CI pipeline in the repo.

## Common Patterns

**Async Testing:** Not applicable.

**Error Testing:**

```bash
cd "$SRC"
SRC_RC=0
wsx push >/dev/null 2>&1 || SRC_RC=$?
check "wsx refuses to run against the source" "2" "$SRC_RC"
```

**Adding a regression test:** reproduce the bug state with file writes in `$SRC`/`$DEV`, run the `wsx` command with output silenced, then `check` both the side that should change and the side that must stay untouched (the "never lose an edit" constraint). Pair positive and negative cases as section 10 does.

---

*Testing analysis: 2026-10-17*
