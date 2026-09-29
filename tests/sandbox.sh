#!/usr/bin/env bash
# End-to-end exercise of wsx sync semantics against a throwaway repository.
#
# Builds everything under $(mktemp -d) and removes it on exit, so this is safe
# to run at any time -- it never touches a real worktree.
#
# Tests the copy in this repo (bin/wsx), not whatever is installed on PATH, so
# results reflect the working tree you are editing.
set -uo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WSX="$REPO_ROOT/bin/wsx"

if [[ ! -x "$WSX" ]]; then
	echo "error: $WSX is missing or not executable" >&2
	exit 2
fi

# Shadow any installed wsx so every invocation below exercises this checkout,
# including the ones git hooks make.
SHIM="$(mktemp -d)"
ln -s "$WSX" "$SHIM/wsx"
export PATH="$SHIM:$PATH"

SANDBOX="$(mktemp -d)"
trap 'rm -rf "$SANDBOX" "$SHIM"' EXIT
PASS=0
FAIL=0

check() { # check <description> <expected> <actual>
	if [[ "$2" == "$3" ]]; then
		echo "  PASS  $1"
		PASS=$((PASS + 1))
	else
		echo "  FAIL  $1"
		echo "          expected: $2"
		echo "          actual:   $3"
		FAIL=$((FAIL + 1))
	fi
}

cd "$SANDBOX"
git init -q -b main repo
cd repo
git config user.email t@t.t && git config user.name t
mkdir -p src && echo "shared" >src/app.txt
git add -A && git commit -qm init

git branch personal-workspace
git worktree add -q ../source personal-workspace
SRC="$SANDBOX/source"

mkdir -p "$SRC/.planning" "$SRC/.state"
echo "plan v1" >"$SRC/.planning/PLAN.md"
echo "notes v1" >"$SRC/.planning/NOTES.md"
echo "rules v1" >"$SRC/AGENTS.md"
echo "ref v1" >"$SRC/.planning/ref.txt"
printf 'binary-v1' >"$SRC/.state/db.bin"
cat >"$SRC/.wsx.conf" <<'EOF'
sync  .planning
sync  AGENTS.md
pull  .planning/ref.txt
lww   .state
EOF

git worktree add -q ../dev -b feature main
DEV="$SANDBOX/dev"
cd "$DEV"

echo "== 1. setup =="
wsx setup >/dev/null 2>&1
check "PLAN.md copied in" "plan v1" "$(cat .planning/PLAN.md)"
check "AGENTS.md copied in" "rules v1" "$(cat AGENTS.md)"
check "copy is a real file, not a symlink" "no" "$([[ -L .planning ]] && echo yes || echo no)"
check "ignored by git" "" "$(git status --porcelain -- .planning AGENTS.md)"
check "source still has its copy" "plan v1" "$(cat "$SRC/.planning/PLAN.md")"

echo "== 2. local edit pushes back =="
echo "plan v2" >.planning/PLAN.md
wsx push >/dev/null 2>&1
check "source received local edit" "plan v2" "$(cat "$SRC/.planning/PLAN.md")"
check "clean after push" "in sync" "$(wsx status 2>&1 | grep -o 'in sync')"

echo "== 3. source edit pulls in =="
echo "notes v2" >"$SRC/.planning/NOTES.md"
wsx pull >/dev/null 2>&1
check "worktree received source edit" "notes v2" "$(cat .planning/NOTES.md)"

echo "== 4. divergent edit is a conflict =="
echo "plan LOCAL" >.planning/PLAN.md
echo "plan SOURCE" >"$SRC/.planning/PLAN.md"
wsx sync >/dev/null 2>&1
check "conflict reported" "1" "$(wsx status 2>&1 | grep -c 'conflict')"
check "local copy untouched" "plan LOCAL" "$(cat .planning/PLAN.md)"
check "source copy untouched" "plan SOURCE" "$(cat "$SRC/.planning/PLAN.md")"
check "source version offered alongside" "plan SOURCE" "$(cat .planning/PLAN.md.wsx-remote)"

echo "== 5. resolve --mine =="
wsx resolve .planning/PLAN.md --mine >/dev/null 2>&1
check "source took local version" "plan LOCAL" "$(cat "$SRC/.planning/PLAN.md")"
check "conflict marker removed" "no" "$([[ -e .planning/PLAN.md.wsx-remote ]] && echo yes || echo no)"
check "in sync after resolve" "in sync" "$(wsx status 2>&1 | grep -o 'in sync')"

echo "== 6. resolve --theirs =="
echo "notes LOCAL" >.planning/NOTES.md
echo "notes SOURCE" >"$SRC/.planning/NOTES.md"
wsx sync >/dev/null 2>&1
wsx resolve .planning/NOTES.md --theirs >/dev/null 2>&1
check "local took source version" "notes SOURCE" "$(cat .planning/NOTES.md)"
check "in sync after resolve" "in sync" "$(wsx status 2>&1 | grep -o 'in sync')"

echo "== 7. deletions propagate =="
rm .planning/NOTES.md
wsx push >/dev/null 2>&1
check "delete propagated to source" "no" "$([[ -e "$SRC/.planning/NOTES.md" ]] && echo yes || echo no)"
echo "fresh" >"$SRC/.planning/NEW.md"
wsx pull >/dev/null 2>&1
check "new source file pulled in" "fresh" "$(cat .planning/NEW.md)"
rm "$SRC/.planning/NEW.md"
wsx pull >/dev/null 2>&1
check "source delete propagated locally" "no" "$([[ -e .planning/NEW.md ]] && echo yes || echo no)"

echo "== 8. pull-only policy never pushes back =="
echo "ref LOCAL" >.planning/ref.txt
wsx push >/dev/null 2>&1
check "source ref unchanged by push" "ref v1" "$(cat "$SRC/.planning/ref.txt")"
wsx pull >/dev/null 2>&1
check "local ref overwritten by pull" "ref v1" "$(cat .planning/ref.txt)"

echo "== 9. lww policy backs up the loser =="
sleep 1
printf 'binary-v2-local' >.state/db.bin
wsx push >/dev/null 2>&1
check "newer local copy won" "binary-v2-local" "$(cat "$SRC/.state/db.bin")"
check "overwritten source copy backed up" "1" "$(find .git* "$SANDBOX/repo/.git/worktrees" -name 'db.bin' -path '*backups*' 2>/dev/null | wc -l)"

echo "== 10. commit guard =="
git add -f .planning/PLAN.md 2>/dev/null
GUARD_RC=0
wsx guard >/dev/null 2>&1 || GUARD_RC=$?
check "guard blocks managed path on feature branch" "1" "$GUARD_RC"
git restore --staged .planning/PLAN.md 2>/dev/null
GUARD_RC=0
wsx guard >/dev/null 2>&1 || GUARD_RC=$?
check "guard passes when nothing managed is staged" "0" "$GUARD_RC"
echo "code change" >>src/app.txt
git add src/app.txt
GUARD_RC=0
wsx guard >/dev/null 2>&1 || GUARD_RC=$?
check "guard ignores ordinary source files" "0" "$GUARD_RC"

echo "== 11. source worktree is refused =="
cd "$SRC"
SRC_RC=0
wsx push >/dev/null 2>&1 || SRC_RC=$?
check "wsx refuses to run against the source" "2" "$SRC_RC"

echo "== 12. plain git hooks (no husky) =="
cd "$DEV"
HOOKS="$(git rev-parse --path-format=absolute --git-common-dir)/hooks"
check "setup installed pre-commit shim" "1" "$(grep -c 'wsx-managed git hook shim' "$HOOKS/pre-commit" 2>/dev/null)"
check "shim is executable" "yes" "$([[ -x "$HOOKS/pre-push" ]] && echo yes || echo no)"
check "doctor sees the guard" "0" "$(wsx doctor 2>&1 | grep -c INACTIVE)"
git add -f .planning/PLAN.md
COMMIT_RC=0
git commit -qm "should be blocked" >/dev/null 2>&1 || COMMIT_RC=$?
check "real commit of managed path is blocked" "1" "$COMMIT_RC"
git restore --staged .planning/PLAN.md
echo "custom" >"$HOOKS/post-merge"
rm -f "$HOOKS/pre-commit"
wsx setup >/dev/null 2>&1
check "foreign hook left untouched" "custom" "$(cat "$HOOKS/post-merge")"
check "missing shim re-armed" "1" "$(grep -c 'wsx-managed git hook shim' "$HOOKS/pre-commit" 2>/dev/null)"
rm -f "$HOOKS/post-merge"
git config core.hooksPath .hooks-elsewhere
check "hooksPath repos report inactive" "1" "$(wsx doctor 2>&1 | grep -c INACTIVE)"
git config --unset core.hooksPath

echo
echo "passed: $PASS   failed: $FAIL"
[[ "$FAIL" -eq 0 ]]
