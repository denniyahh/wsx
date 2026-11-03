#!/usr/bin/env bash
set -u
SP="$1"; PB="$SP/probe/target/debug/probe"; ROOT="$SP/probe/repos"; rm -rf "$ROOT"; mkdir -p "$ROOT/home"
export HOME="$ROOT/home" GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL=/dev/null
export GIT_AUTHOR_NAME=t GIT_AUTHOR_EMAIL=t@t GIT_COMMITTER_NAME=t GIT_COMMITTER_EMAIL=t@t
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_REPLACE_REF_BASE GIT_NO_REPLACE_OBJECTS
W=refs/heads/workspace/dev
fresh() { # sets R A B C (blob ids) CA CB (side commits) TIP
  R="$ROOT/$1"; git init -q -b main "$R"; cd "$R"
  git commit -q --allow-empty -m root
  git switch -q -c workspace/dev; echo 'only original' > .wsx.conf; git add .wsx.conf; git commit -q -m tip; TIP=$(git rev-parse HEAD)
  A=$(git rev-parse HEAD:.wsx.conf)
  git switch -q -c side1; echo 'only intermediate' > .wsx.conf; git commit -qam s1; B=$(git rev-parse HEAD:.wsx.conf); CB=$(git rev-parse HEAD)
  git switch -q -c side2; echo 'only far' > .wsx.conf; git commit -qam s2; C=$(git rev-parse HEAD:.wsx.conf); CC=$(git rev-parse HEAD)
  git switch -q main
}
gitread() { git -C "$R" cat-file blob "$W:.wsx.conf" 2>&1 | head -1; echo "   git_rc=${PIPESTATUS[0]}"; }
hdr() { echo; echo "=== $1"; }

hdr "CTRL no replacement"; fresh c0; gitread; "$PB" guard-read "$R" "$W"

hdr "agy-1 single commit replacement: tip id vs show-ref"; fresh a1
git replace "$TIP" "$CB"; echo "show-ref:  $(git show-ref --verify $W | cut -d' ' -f1)"; echo "probe tip: $("$PB" tip "$R" "$W")"; echo "orig=$TIP repl=$CB"; "$PB" read "$R" "$W"; gitread
echo "control after replace -d:"; git replace -d "$TIP" >/dev/null; echo "probe tip: $("$PB" tip "$R" "$W")"; "$PB" read "$R" "$W"

hdr "agy-5 commit replacement cycle: does tip read fail before guard?"; fresh a5
git update-ref "refs/replace/$TIP" "$CB"; git update-ref "refs/replace/$CB" "$TIP"; "$PB" tip "$R" "$W"; echo "tip_rc=$?"; "$PB" guard-read "$R" "$W"; gitread

hdr "agy-3 base without slash, chain A>B>C: GIT_REPLACE_REF_BASE=refs/replace"; fresh a3
git replace "$A" "$B"; git replace "$B" "$C"
echo "-- control, with slash:"; GIT_REPLACE_REF_BASE=refs/replace/ "$PB" guard-read "$R" "$W"
echo "-- no slash:"; GIT_REPLACE_REF_BASE=refs/replace "$PB" guard-read "$R" "$W"
echo "   git: $(GIT_REPLACE_REF_BASE=refs/replace git cat-file blob "$W:.wsx.conf" 2>&1 | head -1)"
echo "-- no variable:"; "$PB" guard-read "$R" "$W"; gitread

hdr "codex-2 duplicate replacement, case differing"; fresh c2
UP=$(printf %s "$A" | tr a-f A-F)
git update-ref "refs/replace/$A" "$B"; git update-ref "refs/replace/$UP" "$C"; "$PB" guard-read "$R" "$W"; gitread
echo "-- control, uppercase removed:"; git update-ref -d "refs/replace/$UP"; "$PB" guard-read "$R" "$W"; gitread

hdr "codex-3 live refs vs map cached at open: delete after open"; fresh c3
git replace "$A" "$B"; git replace "$B" "$C"
echo "-- control, no interleave:"; "$PB" guard-read "$R" "$W"; gitread
AFTER_OPEN="git -C '$R' update-ref -d refs/replace/$A && git -C '$R' update-ref -d refs/replace/$B" "$PB" guard-read "$R" "$W"; gitread
echo "-- fresh open:"; "$PB" guard-read "$R" "$W"

hdr "codex-3b replacement ADDED after open"; fresh c3b
AFTER_OPEN="git -C '$R' replace $A $B" "$PB" guard-read "$R" "$W"; gitread

hdr "codex-4 gix-only key gitoxide.objects.replaceRefBase"; fresh c4
git config --local gitoxide.objects.replaceRefBase refs/other/; git update-ref "refs/other/$A" "$C"; "$PB" guard-read "$R" "$W"; gitread
echo "-- control, key unset:"; git config --local --unset gitoxide.objects.replaceRefBase; "$PB" guard-read "$R" "$W"; gitread

hdr "codex-1 ref swapped between validation and second lookup"; fresh c1
ORIGIN=refs/remotes/origin/develop; git update-ref "$ORIGIN" main
echo "remote=$(git rev-parse main) workspace=$TIP"
echo "-- control, no swap:"; "$PB" ref-commit "$R" "$ORIGIN"
AFTER_OPEN="git -C '$R' symbolic-ref $ORIGIN $W" "$PB" ref-commit "$R" "$ORIGIN"
