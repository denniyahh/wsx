#!/usr/bin/env bash
# Spike 002: what gix (opened as wsx opens it) reads by object id, against git, per shape.
set -euo pipefail
SCRATCH="$1"; PROBE="$2"; [ -n "$SCRATCH" ] && [ -x "$PROBE" ] || { echo "usage: gix-cases.sh <scratch> <probe-binary>" >&2; exit 2; }
mkdir -p "$SCRATCH"
export HOME="$SCRATCH/home" XDG_CONFIG_HOME="$SCRATCH/home/.config"
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL="$SCRATCH/home/gitconfig"
mkdir -p "$HOME"; : >"$GIT_CONFIG_GLOBAL"
export GIT_AUTHOR_NAME=s GIT_AUTHOR_EMAIL=s@x GIT_COMMITTER_NAME=s GIT_COMMITTER_EMAIL=s@x
export GIT_AUTHOR_DATE='2026-01-01T00:00:00Z' GIT_COMMITTER_DATE='2026-01-01T00:00:00Z'
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_PREFIX GIT_NO_REPLACE_OBJECTS || true
PASS=0; FAIL=0
ok()  { PASS=$((PASS+1)); printf 'PASS  %s\n' "$*"; }
bad() { FAIL=$((FAIL+1)); printf 'FAIL  %s\n' "$*"; }
same() { if [ "$2" = "$3" ]; then ok "$1: git=gix=[$2]"; else bad "$1: git=[$2] gix=[$3]"; fi; }
gixblob() { "$PROBE" blob "$1" "$2" .wsx.conf 2>&1 || true; }
fixture() { local r="$1"; git init -q -b main "$r"
  printf 'only .planning\n' >"$r/.wsx.conf"; git -C "$r" add .wsx.conf; git -C "$r" commit -qm A
  git -C "$r" branch workspace/dev
  git -C "$r" checkout -q -b other; printf 'only b\n' >"$r/.wsx.conf"; git -C "$r" commit -qam B
  git -C "$r" checkout -q main
  A=$(git -C "$r" rev-parse workspace/dev); B=$(git -C "$r" rev-parse other); }

echo "## B1 replace refs on/off"
R="$SCRATCH/b1"; fixture "$R"
same B1-no-replace "$(git -C "$R" show "$A:.wsx.conf")" "$(gixblob "$R" "$A")"
git -C "$R" replace "$A" "$B"
same B1-replace-applied "$(git -C "$R" show "$A:.wsx.conf")" "$(gixblob "$R" "$A")"
same B1-GIT_NO_REPLACE_OBJECTS "$(GIT_NO_REPLACE_OBJECTS=1 git -C "$R" show "$A:.wsx.conf")" "$(GIT_NO_REPLACE_OBJECTS=1 gixblob "$R" "$A")"
git -C "$R" config core.useReplaceRefs false
same B1-core.useReplaceRefs=false "$(git -C "$R" show "$A:.wsx.conf")" "$(gixblob "$R" "$A")"
git -C "$R" config --unset core.useReplaceRefs
printf '[core]\n\tuseReplaceRefs = false\n' >>"$GIT_CONFIG_GLOBAL"
same B1-useReplaceRefs-false-in-global "$(git -C "$R" show "$A:.wsx.conf")" "$(gixblob "$R" "$A")"
: >"$GIT_CONFIG_GLOBAL"
same B1-control-back-on "only b" "$(gixblob "$R" "$A")"

echo "## B2 replace ref only in a linked worktree's private dir (CR-01)"
R="$SCRATCH/b2"; fixture "$R"; git -C "$R" worktree add -q "$SCRATCH/b2-wt" other
mkdir -p "$R/.git/worktrees/b2-wt/refs/replace"; printf '%s\n' "$B" >"$R/.git/worktrees/b2-wt/refs/replace/$A"
same B2-from-linked-wt "$(git -C "$SCRATCH/b2-wt" show "$A:.wsx.conf")" "$(gixblob "$SCRATCH/b2-wt" "$A")"
same B2-from-main-wt "$(git -C "$R" show "$A:.wsx.conf")" "$(gixblob "$R" "$A")"
mkdir -p "$R/.git/refs/replace"; printf '%s\n' "$B" >"$R/.git/refs/replace/$A"
same B2-control-common-replace "$(git -C "$SCRATCH/b2-wt" show "$A:.wsx.conf")" "$(gixblob "$SCRATCH/b2-wt" "$A")"
rm -rf "$R/.git/refs/replace"
echo "## B2b replace ref in packed-refs of the private dir"
printf '# pack-refs with: peeled fully-peeled sorted \n%s refs/replace/%s\n' "$B" "$A" >"$R/.git/worktrees/b2-wt/packed-refs"
rm -rf "$R/.git/worktrees/b2-wt/refs/replace"
same B2b-private-packed "$(git -C "$SCRATCH/b2-wt" show "$A:.wsx.conf")" "$(gixblob "$SCRATCH/b2-wt" "$A")"

echo "## B3 symlinked HEAD + includeIf.onbranch carrying core.useReplaceRefs=false"
R="$SCRATCH/b3"; fixture "$R"; git -C "$R" replace "$A" "$B"
printf '[core]\n\tuseReplaceRefs = false\n' >"$R/.git/onbranch.inc"
git -C "$R" config 'includeIf.onbranch:workspace/dev.path' onbranch.inc
git -C "$R" checkout -q workspace/dev
same B3-text-HEAD "$(git -C "$R" show "$A:.wsx.conf")" "$(gixblob "$R" "$A")"
rm "$R/.git/HEAD"; ln -s refs/heads/workspace/dev "$R/.git/HEAD"
same B3-symlinked-HEAD "$(git -C "$R" show "$A:.wsx.conf")" "$(gixblob "$R" "$A")"
rm "$R/.git/HEAD"; printf 'ref: refs/heads/main\n' >"$R/.git/HEAD"
same B3-control-HEAD-elsewhere "$(git -C "$R" show "$A:.wsx.conf")" "$(gixblob "$R" "$A")"

echo "## B4 info/grafts and merge-base (object work by id, later phases)"
R="$SCRATCH/b4"; fixture "$R"
git -C "$R" checkout -q -b c1 "$A"; echo 1 >"$R/f"; git -C "$R" add f; git -C "$R" commit -qm c1; C1=$(git -C "$R" rev-parse HEAD)
git -C "$R" checkout -q -b c2 "$A"; echo 2 >"$R/g"; git -C "$R" add g; git -C "$R" commit -qm c2; C2=$(git -C "$R" rev-parse HEAD)
same B4-plain "$(git -C "$R" merge-base "$C1" "$C2")" "$("$PROBE" base "$R" "$C1" "$C2")"
printf '%s\n' "$C2" >"$R/.git/info/grafts"
same B4-grafted-root "$(git -C "$R" merge-base "$C1" "$C2" 2>/dev/null || echo none)" "$("$PROBE" base "$R" "$C1" "$C2" 2>&1)"
rm "$R/.git/info/grafts"
echo "## B4b shallow boundary"
printf '%s\n' "$C2" >"$R/.git/shallow"
same B4b-shallow "$(git -C "$R" merge-base "$C1" "$C2" 2>/dev/null || echo none)" "$("$PROBE" base "$R" "$C1" "$C2" 2>&1)"
rm "$R/.git/shallow"
same B4-control-plain-again "$(git -C "$R" merge-base "$C1" "$C2")" "$("$PROBE" base "$R" "$C1" "$C2")"

echo; echo "TOTAL pass=$PASS fail=$FAIL"
