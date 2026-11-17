#!/usr/bin/env bash
# Spike 001: are the batched git-binary reads git's own answer for every review shape?
# Each case prints: reference command result, batched read result, and a control
# that changes the reference answer and must change the batched one.
set -euo pipefail
SCRATCH="$1"; [ -n "$SCRATCH" ] || { echo "usage: parity.sh <scratch-dir>" >&2; exit 2; }
mkdir -p "$SCRATCH"
NODE="$(command -v "$NODE" || true)"; [ -x "$NODE" ] || { echo "set NODE to a real node binary (a mise shim fails once HOME moves)" >&2; exit 2; }
export HOME="$SCRATCH/home" XDG_CONFIG_HOME="$SCRATCH/home/.config"
export GIT_CONFIG_NOSYSTEM=1 GIT_CONFIG_GLOBAL="$SCRATCH/home/gitconfig"
mkdir -p "$HOME"; : >"$GIT_CONFIG_GLOBAL"
export GIT_AUTHOR_NAME=s GIT_AUTHOR_EMAIL=s@x GIT_COMMITTER_NAME=s GIT_COMMITTER_EMAIL=s@x
export GIT_AUTHOR_DATE='2026-01-01T00:00:00Z' GIT_COMMITTER_DATE='2026-01-01T00:00:00Z'
unset GIT_DIR GIT_WORK_TREE GIT_INDEX_FILE GIT_COMMON_DIR GIT_PREFIX GIT_NO_REPLACE_OBJECTS || true

PASS=0; FAIL=0
ok()  { PASS=$((PASS+1)); printf 'PASS  %s\n' "$*"; }
bad() { FAIL=$((FAIL+1)); printf 'FAIL  %s\n' "$*"; }
same() { # name ref batch
  if [ "$2" = "$3" ]; then ok "$1: [$2]"; else bad "$1: ref=[$2] batch=[$3]"; fi; }
differs() { # name before after  (control: the answer must change)
  if [ "$2" != "$3" ]; then ok "$1 control moved: [$2] -> [$3]"; else bad "$1 control did not move: [$2]"; fi; }
orno() { if [ -n "$1" ]; then printf '%s' "$1"; else printf none; fi; }

# --- the batched reads (what resolve would issue, one child each) ---
b_config() { git -C "$1" config -z --show-scope --show-origin --get-regexp '^(wsx|remote)\.' 2>/dev/null | tr '\0\n' '|~' || true; }
b_refs() { local d="$1"; shift
  git -C "$d" for-each-ref --format='%(refname) %(objectname) %(objecttype) %(symref)' "$@" 2>/dev/null || true; }
b_blob() { printf '%s:.wsx.conf\n' "$2" | git -C "$1" cat-file --batch 2>/dev/null | tr '\n' '~' || true; }
# wsx key values from local scope, own file only (D-01): origin must be the common dir's config
b_wsx_local() { local d="$1" key="$2" cfg
  cfg="$(cd "$d" && realpath "$(git rev-parse --git-common-dir)")/config"
  git -C "$d" config -z --show-scope --show-origin --get-regexp '^wsx\.' 2>/dev/null |
    "$NODE" -e '
      const [key,cfg,dir]=process.argv.slice(1); const fs=require("fs"),path=require("path");
      const raw=fs.readFileSync(0,"latin1").split("\0"); const out=[];
      for(let i=0;i+2<raw.length;i+=3){const scope=raw[i],origin=raw[i+1],kv=raw[i+2];
        const nl=kv.indexOf("\n"); const k=nl<0?kv:kv.slice(0,nl); const v=nl<0?"<bare>":kv.slice(nl+1);
        const abs=path.resolve(dir,origin.replace(/^file:/,""));
        if(scope==="local"&&abs===cfg&&k===key.toLowerCase()) out.push(v);}
      process.stdout.write(out.join(","));' "$key" "$cfg" "$d"; }

fixture() { # dir -> repo with A (only .planning) on workspace/dev, B (only b) on other
  local r="$1"; git init -q -b main "$r"
  printf 'only .planning\n' >"$r/.wsx.conf"; git -C "$r" add .wsx.conf; git -C "$r" commit -qm A
  git -C "$r" branch workspace/dev
  git -C "$r" checkout -q -b other; printf 'only b\n' >"$r/.wsx.conf"; git -C "$r" commit -qam B
  git -C "$r" checkout -q main
  A=$(git -C "$r" rev-parse workspace/dev); B=$(git -C "$r" rev-parse other); }

echo "## S1 replace ref: cat-file applies replacements exactly as git show does"
R="$SCRATCH/s1"; fixture "$R"
same S1-plain "$(git -C "$R" show "$A:.wsx.conf")" "$(b_blob "$R" "$A" | cut -d~ -f2)"
git -C "$R" replace "$A" "$B"
REF=$(git -C "$R" show "$A:.wsx.conf"); BAT=$(b_blob "$R" "$A" | cut -d~ -f2)
same S1-replaced "$REF" "$BAT"; differs S1-replace "only .planning" "$BAT"
REFN=$(GIT_NO_REPLACE_OBJECTS=1 git -C "$R" show "$A:.wsx.conf"); BATN=$(GIT_NO_REPLACE_OBJECTS=1 b_blob "$R" "$A" | cut -d~ -f2)
same S1-no-replace-env "$REFN" "$BATN"
REFC=$(git -C "$R" -c core.useReplaceRefs=false show "$A:.wsx.conf"); BATC=$(printf '%s:.wsx.conf\n' "$A" | git -C "$R" -c core.useReplaceRefs=false cat-file --batch | sed -n 2p)
same S1-useReplaceRefs-false "$REFC" "$BATC"

echo "## S1b replace ref in a linked worktree's private dir (CR-01): does git apply it?"
R="$SCRATCH/s1b"; fixture "$R"; git -C "$R" worktree add -q "$SCRATCH/s1b-wt" other
WTD="$R/.git/worktrees/s1b-wt"; mkdir -p "$WTD/refs/replace"; printf '%s\n' "$B" >"$WTD/refs/replace/$A"
REF=$(git -C "$SCRATCH/s1b-wt" show "$A:.wsx.conf"); BAT=$(b_blob "$SCRATCH/s1b-wt" "$A" | cut -d~ -f2)
same S1b-private-replace "$REF" "$BAT"
echo "INFO  S1b git in linked worktree reads: [$REF] (only .planning = ignored, only b = applied)"
echo "INFO  S1b today's detection scans only $R/.git/refs/replace: $(ls -A "$R/.git/refs/replace" 2>/dev/null | wc -l) entries there"
echo "INFO  S1b for-each-ref refs/replace from wt: [$(b_refs "$SCRATCH/s1b-wt" refs/replace/)]"
mkdir -p "$R/.git/refs/replace"; printf '%s\n' "$B" >"$R/.git/refs/replace/$A"
differs S1b-common-replace "$REF" "$(b_blob "$SCRATCH/s1b-wt" "$A" | cut -d~ -f2)"

echo "## S2 worktree-private store: a branch only in .git/worktrees/X/refs/heads"
R="$SCRATCH/s2"; fixture "$R"; git -C "$R" worktree add -q "$SCRATCH/s2-wt" other
mkdir -p "$R/.git/worktrees/s2-wt/refs/heads/workspace"; printf '%s\n' "$B" >"$R/.git/worktrees/s2-wt/refs/heads/workspace/priv"
REF=$(git -C "$SCRATCH/s2-wt" branch --list 'workspace/*' --format='%(refname)' | tr '\n' ' ')
BAT=$(b_refs "$SCRATCH/s2-wt" refs/heads/workspace/ | cut -d' ' -f1 | tr '\n' ' ')
same S2-listing "$REF" "$BAT"
git -C "$R" branch workspace/shared other
differs S2-shared-branch "$BAT" "$(b_refs "$SCRATCH/s2-wt" refs/heads/workspace/ | cut -d' ' -f1 | tr '\n' ' ')"

echo "## S3 symlinked loose ref (round 1 codex 1/2)"
R="$SCRATCH/s3"; fixture "$R"; rm "$R/.git/refs/heads/workspace/dev"
ln -s ../other "$R/.git/refs/heads/workspace/dev"   # relative symlink, does not start with refs/
REF=$(orno "$(git -C "$R" rev-parse -q --verify refs/heads/workspace/dev)")
BAT=$(orno "$(b_refs "$R" refs/heads/workspace/dev | cut -d' ' -f2)")
same S3-relative-symlink-value "$REF" "$BAT"
REF=$(git -C "$R" branch --list 'workspace/*' --format='%(refname)' | tr '\n' ' ')
BAT=$(b_refs "$R" refs/heads/workspace/ | cut -d' ' -f1 | tr '\n' ' ')
same S3-listing "$REF" "$BAT"
rm "$R/.git/refs/heads/workspace/dev"; ln -s refs/heads/other "$R/.git/refs/heads/workspace/dev"  # git-style symlink ref
REF2=$(orno "$(git -C "$R" rev-parse -q --verify refs/heads/workspace/dev)")
BAT2=$(b_refs "$R" refs/heads/workspace/dev | cut -d' ' -f2,4)
echo "INFO  S3 refs/-symlink: rev-parse [$REF2] for-each-ref [value symref]=[$BAT2] symbolic-ref [$(orno "$(git -C "$R" symbolic-ref -q refs/heads/workspace/dev)")]"
# git splits on this shape: rev-parse and show-ref --verify resolve it, every listing hides it.
# The batch sides with the listings, which reads as "branch missing" (fails closed).
same S3-refs-symlink-hidden-like-branch-list "$(orno "$(git -C "$R" branch --list 'workspace/dev' --format='%(objectname)')")" "$(orno "$(echo "$BAT2" | cut -d' ' -f1)")"
differs S3-split-rev-parse-vs-listing "$REF2" "$(orno "$(echo "$BAT2" | cut -d' ' -f1)")"
echo "INFO  S3 refs/-symlink: branch --list [$(git -C "$R" branch --list 'workspace/*' --format='%(refname)' | tr '
' ' ')] for-each-ref listing [$(b_refs "$R" refs/heads/workspace/ | cut -d' ' -f1 | tr '
' ' ')] show-ref [$(git -C "$R" show-ref refs/heads/workspace/dev | tr '
' ' ')]"
echo "INFO  S3 show-ref --verify: [$(git -C "$R" show-ref --verify refs/heads/workspace/dev 2>&1 | tr '\n' ' ')]"
echo "INFO  S3 for-each-ref stderr: [$(git -C "$R" for-each-ref refs/heads/workspace/dev 2>&1 >/dev/null | tr '
' ' ')]"

echo "## S4 main-worktree/ alias (round 2 codex 1), read from a linked worktree"
R="$SCRATCH/s4"; fixture "$R"; git -C "$R" worktree add -q "$SCRATCH/s4-wt" -b wtb main
git -C "$R" update-ref refs/heads/target "$A"; git -C "$R" update-ref refs/heads/refs/heads/target "$B"
git -C "$R" -c core.preferSymlinkRefs=true symbolic-ref refs/heads/keep refs/heads/target
git -C "$R" symbolic-ref refs/heads/workspace/dev main-worktree/refs/heads/keep
W="$SCRATCH/s4-wt"
REF=$(orno "$(git -C "$W" rev-parse -q --verify 'refs/heads/workspace/dev^{commit}' 2>/dev/null)")
BAT=$(orno "$(b_refs "$W" refs/heads/workspace/dev | cut -d' ' -f2)")
same S4-alias-tip "$REF" "$BAT"
git -C "$R" symbolic-ref refs/heads/workspace/dev refs/heads/keep
differs S4-plain-symref "$BAT" "$(orno "$(b_refs "$W" refs/heads/workspace/dev | cut -d' ' -f2)")"
same S4-plain-symref-agrees "$(git -C "$W" rev-parse 'refs/heads/workspace/dev^{commit}')" "$(b_refs "$W" refs/heads/workspace/dev | cut -d' ' -f2)"

echo "## S5 symlinked HEAD + includeIf.onbranch (round 2 codex 2)"
R="$SCRATCH/s5"; fixture "$R"
printf '[remote "origin"]\n\turl = /nowhere\n' >"$R/.git/onbranch.inc"
git -C "$R" config 'includeIf.onbranch:workspace/dev.path' onbranch.inc
git -C "$R" checkout -q workspace/dev
rm "$R/.git/HEAD"; ln -s refs/heads/workspace/dev "$R/.git/HEAD"
remotes_of() { b_config "$1" | tr '|' '\n' | sed -n 's/^remote\.\([^~]*\)\.url~.*/\1/p' | tr '\n' ' '; }
REF=$(git -C "$R" remote | tr '\n' ' ')
BAT=$(remotes_of "$R")
same S5-remotes-symlinked-HEAD "$REF" "$BAT"
rm "$R/.git/HEAD"; printf 'ref: refs/heads/main\n' >"$R/.git/HEAD"
differs S5-HEAD-elsewhere "$BAT" "$(remotes_of "$R")"
same S5-HEAD-elsewhere-agrees "$(git -C "$R" remote | tr '\n' ' ')" "$(remotes_of "$R")"

echo "## S6 dangling text symref under refs/heads/workspace/ (round 2 codex 3)"
R="$SCRATCH/s6"; fixture "$R"
git -C "$R" symbolic-ref refs/heads/workspace/ghost refs/heads/nope
REF=$(git -C "$R" branch --list 'workspace/*' --format='%(refname)' 2>/dev/null | tr '\n' ' ')
BAT=$(b_refs "$R" refs/heads/workspace/ | cut -d' ' -f1 | tr '\n' ' ')
same S6-listing "$REF" "$BAT"
git -C "$R" branch nope other
differs S6-target-born "$BAT" "$(b_refs "$R" refs/heads/workspace/ | cut -d' ' -f1 | tr '\n' ' ')"

echo "## S7 dangling symbolic integration rung (refs/remotes/origin/develop -> nowhere)"
R="$SCRATCH/s7"; fixture "$R"; git -C "$R" remote add origin /nowhere
git -C "$R" symbolic-ref refs/remotes/origin/develop refs/remotes/origin/nope
REF=$(orno "$(git -C "$R" rev-parse -q --verify refs/remotes/origin/develop)")
BAT=$(orno "$(b_refs "$R" refs/remotes/origin/develop)")
same S7-dangling-rung "$REF" "$BAT"
echo "INFO  S7 symbolic-ref sees: [$(git -C "$R" symbolic-ref -q refs/remotes/origin/develop)]"
git -C "$R" update-ref refs/remotes/origin/nope "$A"
differs S7-target-born "$BAT" "$(orno "$(b_refs "$R" refs/remotes/origin/develop)")"

echo "## S8 fifo loose ref: hang, bounded"
R="$SCRATCH/s8"; fixture "$R"; mkfifo "$R/.git/refs/heads/workspace/fifo"
set +e
timeout 5 git -C "$R" for-each-ref refs/heads/workspace/ >/dev/null 2>&1; echo "INFO  S8 for-each-ref exit $? (124 = hung, killed at 5 s)"
timeout 5 git -C "$R" rev-parse -q --verify refs/heads/workspace/dev >/dev/null 2>&1; echo "INFO  S8 rev-parse of a sibling ref exit $?"
timeout 5 git -C "$R" rev-parse -q --verify refs/heads/workspace/fifo >/dev/null 2>&1; echo "INFO  S8 rev-parse of the fifo ref itself exit $?"
timeout 5 git -C "$R" for-each-ref refs/heads/workspace/fifo >/dev/null 2>&1; echo "INFO  S8 for-each-ref naming the fifo exit $?"
printf 'refs/heads/workspace/fifo
' | timeout 5 git -C "$R" cat-file --batch-check >/dev/null 2>&1; echo "INFO  S8 cat-file --batch-check of the fifo ref exit $?"
git init -q --bare "$SCRATCH/s8-remote.git"; git -C "$R" remote add origin "$SCRATCH/s8-remote.git"
timeout 10 git -C "$R" push -q origin main >/dev/null 2>&1; echo "INFO  S8 push, no hook: exit $?"
mkdir -p "$R/.git/hooks"; printf '#!/bin/sh\nexec git for-each-ref refs/heads/workspace/ >/dev/null\n' >"$R/.git/hooks/pre-push"; chmod +x "$R/.git/hooks/pre-push"
timeout 10 git -C "$R" push -q origin other >/dev/null 2>&1; echo "INFO  S8 push, pre-push hook listing candidates: exit $?"
rm "$R/.git/refs/heads/workspace/fifo"
timeout 5 git -C "$R" for-each-ref refs/heads/workspace/ >/dev/null 2>&1; echo "INFO  S8 control, fifo removed: for-each-ref exit $?"
set -e

echo "## C1 D-01: git config --local vs today's read (Source::Local, level 0)"
R="$SCRATCH/c1"; fixture "$R"
printf '[wsx]\n\tsourceBranch = from-include\n' >"$R/.git/inc.cfg"; git -C "$R" config include.path inc.cfg
git -C "$R" config extensions.worktreeConfig true
git -C "$R" config --worktree wsx.sourceBranch from-worktree
echo "INFO  C1 plain git config --get-all: [$(git -C "$R" config --get-all wsx.sourceBranch | tr '\n' ',')]"
REF=$(git -C "$R" config --local --get-all wsx.sourceBranch 2>/dev/null | tr '\n' ',' | sed 's/,$//' || true)
same C1-local-skips-include-and-worktree "$REF" "$(b_wsx_local "$R" wsx.sourceBranch)"
same C1-matches-today "" "$REF"
git -C "$R" config --add wsx.sourceBranch direct
differs C1-direct-key "$REF" "$(b_wsx_local "$R" wsx.sourceBranch)"
same C1-direct-agrees "$(git -C "$R" config --local --get-all wsx.sourceBranch)" "$(b_wsx_local "$R" wsx.sourceBranch)"
same C1-command-line-key-ignored "$(GIT_CONFIG_PARAMETERS="'wsx.sourceBranch'='cli'" git -C "$R" config --local --get-all wsx.sourceBranch)" "$(GIT_CONFIG_PARAMETERS="'wsx.sourceBranch'='cli'" b_wsx_local "$R" wsx.sourceBranch)"
echo "INFO  C1 control: with GIT_CONFIG_PARAMETERS, plain --get-all sees [$(GIT_CONFIG_PARAMETERS="'wsx.sourceBranch'='cli'" git -C "$R" config --get-all wsx.sourceBranch | tr '\n' ',')]"
git -C "$R" worktree add -q "$SCRATCH/c1-wt" other
same C1-from-linked-worktree "$(git -C "$SCRATCH/c1-wt" config --local --get-all wsx.sourceBranch | tr '\n' ',' | sed 's/,$//')" "$(b_wsx_local "$SCRATCH/c1-wt" wsx.sourceBranch)"

echo "## C2 D-18: repeated, empty and bare keys stay distinguishable in -z output"
R="$SCRATCH/c2"; fixture "$R"
printf '[wsx]\n\tbaseRef\n\tbaseRemote =\n\tpushRemote = a\n\tpushRemote = b\n' >>"$R/.git/config"
same C2-bare "<bare>" "$(b_wsx_local "$R" wsx.baseRef)"
same C2-empty "" "$(b_wsx_local "$R" wsx.baseRemote)"
same C2-repeated "a,b" "$(b_wsx_local "$R" wsx.pushRemote)"
differs C2-bare-vs-empty "$(b_wsx_local "$R" wsx.baseRef)" "$(b_wsx_local "$R" wsx.baseRemote)"

echo "## C3 remote_configured: global-only section, empty local section, included section"
R="$SCRATCH/c3"; fixture "$R"
printf '[remote "glob"]\n\turl = /g\n' >>"$GIT_CONFIG_GLOBAL"
printf '[remote "empty"]\n' >>"$R/.git/config"
printf '[remote "inc"]\n\turl = /i\n' >"$R/.git/r.inc"; git -C "$R" config include.path r.inc
REM=$(git -C "$R" config -z --show-scope --get-regexp '^remote\.' | "$NODE" -e '
  const raw=require("fs").readFileSync(0,"latin1").split("\0"); const s=new Set();
  for(let i=0;i+1<raw.length;i+=2){const sc=raw[i],kv=raw[i+1]; const k=kv.split("\n")[0];
    if(sc==="local"||sc==="worktree") s.add(k.slice(7,k.lastIndexOf(".")));}
  process.stdout.write([...s].sort().join(" "));')
for n in glob empty inc; do
  if git -C "$R" remote add "$n" /x >/dev/null 2>&1; then ga=free; git -C "$R" remote remove "$n" >/dev/null 2>&1 || true; else ga=defined; fi
  case " $REM " in *" $n "*) bt=defined;; *) bt=free;; esac
  same "C3-remote-$n (git remote add says $ga)" "$ga" "$bt"
done
: >"$GIT_CONFIG_GLOBAL"
same C3-control-set-is-not-empty "inc" "$REM"

echo
echo "TOTAL pass=$PASS fail=$FAIL"
