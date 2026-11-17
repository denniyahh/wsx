#!/usr/bin/env bash
# Spike 001 latency, git children only: the four batched reads with arguments fixed in advance,
# so the time is git's, not the shell's parsing. Control: the per-ref command set (12-13 children).
set -euo pipefail
RUNS="$1"; shift; [ -n "$RUNS" ] && [ "$#" -gt 0 ] || { echo "usage: bench-git.sh <runs> <repo>..." >&2; exit 2; }
p() { sort -n | awk '{a[NR]=$1} END {i=int(NR*0.95+0.999); if(i>NR)i=NR; m=int((NR+1)/2); printf "p50=%.1fms p95=%.1fms max=%.1fms", a[m]/1e6, a[i]/1e6, a[NR]/1e6}'; }
for r in "$@"; do
  tip=$(git -C "$r" for-each-ref --format='%(objectname)' refs/heads/workspace/ refs/heads/personal/ | head -1)
  name=$(git -C "$r" for-each-ref --format='%(refname)' refs/heads/workspace/ refs/heads/personal/ | head -1)
  blob=.workspace-divergence; git -C "$r" cat-file -e "$tip:$blob" 2>/dev/null || blob=.wsx.conf
  batched=(); perref=()
  for i in $(seq "$RUNS"); do
    s=$(date +%s%N)
    git --version >/dev/null
    git -C "$r" config -z --show-scope --show-origin --get-regexp '^(wsx|remote)\.' >/dev/null || true
    git -C "$r" for-each-ref --format='%(refname) %(objectname) %(objecttype) %(symref)' refs/heads/workspace/ refs/heads/personal/ refs/remotes/origin/develop refs/remotes/origin/next refs/remotes/origin/HEAD >/dev/null
    printf '%s:%s\n' "$tip" "$blob" | git -C "$r" cat-file --batch >/dev/null
    e=$(date +%s%N); batched+=($((e-s)))
    s=$(date +%s%N)
    git --version >/dev/null
    for k in wsx.sourceBranch wsx.baseRemote wsx.pushRemote wsx.baseRef wsx.worktreeDir; do git -C "$r" config --local --get-all "$k" >/dev/null || true; done
    git -C "$r" remote >/dev/null
    git -C "$r" for-each-ref --format='%(refname)' refs/heads/workspace/ refs/heads/personal/ >/dev/null
    git -C "$r" rev-parse -q --verify "$name" >/dev/null
    for ref in develop next HEAD; do git -C "$r" rev-parse -q --verify "refs/remotes/origin/$ref" >/dev/null || true; done
    git -C "$r" cat-file blob "$tip:$blob" >/dev/null 2>&1 || true
    e=$(date +%s%N); perref+=($((e-s)))
  done
  echo "== $(basename "$r") (blob $blob)"
  echo "   batched 4 children   $(printf '%s\n' "${batched[@]}" | p)"
  echo "   per-ref 13 children  $(printf '%s\n' "${perref[@]}" | p)"
done
