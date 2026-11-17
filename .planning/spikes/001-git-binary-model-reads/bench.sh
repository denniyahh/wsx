#!/usr/bin/env bash
# Spike 001 latency: one resolve as batched git reads, timed on real repos (read-only).
# Children: git --version, config, for-each-ref, cat-file --batch  (budget D-26: <= 4).
set -euo pipefail
RUNS="$1"; shift; [ -n "$RUNS" ] && [ "$#" -gt 0 ] || { echo "usage: bench.sh <runs> <repo>..." >&2; exit 2; }
LOG="$(mktemp)"; WRAP="$(mktemp -d)"; trap 'rm -rf "$LOG" "$WRAP"' EXIT
REALGIT="$(command -v git)"
printf '#!/bin/sh\necho "$*" >>"%s"\nexec "%s" "$@"\n' "$LOG" "$REALGIT" >"$WRAP/git"; chmod +x "$WRAP/git"

resolve_batched() { # repo -> prints "<state> <children-by-own-count>"
  local r="$1" n=0 cfg refs base=origin branch="" cands tip line
  git --version >/dev/null; n=$((n+1))
  cfg=$(git -C "$r" config -z --show-scope --show-origin --get-regexp '^(wsx|remote)\.' | tr '\0' '\n' || true); n=$((n+1))
  branch=$(printf '%s\n' "$cfg" | sed -n 's/^wsx\.sourcebranch$//p;' | head -1)  # unset on these repos
  refs=$(git -C "$r" for-each-ref --format='%(refname) %(objectname) %(objecttype) %(symref)' \
    refs/heads/workspace/ refs/heads/personal/ "refs/remotes/$base/develop" "refs/remotes/$base/next" "refs/remotes/$base/HEAD"); n=$((n+1))
  cands=$(printf '%s\n' "$refs" | awk '$1 ~ /^refs\/heads\/(workspace|personal)\/[^\/]+$/ {print $1" "$2}')
  if [ "$(printf '%s\n' "$cands" | sed '/^$/d' | wc -l)" -ne 1 ]; then echo "candidates!=1 $n"; return; fi
  tip=$(printf '%s\n' "$cands" | cut -d' ' -f2)
  line=$(printf '%s:.wsx.conf\n' "$tip" | git -C "$r" cat-file --batch | head -1); n=$((n+1))
  echo "manifest:[$(echo "$line" | cut -d' ' -f2)] $n"
}

resolve_per_ref() { # negative control: one child per ref, as a naive port would
  local r="$1" n=0
  git --version >/dev/null; n=$((n+1))
  for k in wsx.sourceBranch wsx.baseRemote wsx.pushRemote wsx.baseRef wsx.worktreeDir; do git -C "$r" config --local --get-all "$k" >/dev/null || true; n=$((n+1)); done
  git -C "$r" remote >/dev/null; n=$((n+1))
  for ref in $(git -C "$r" for-each-ref --format='%(refname)' refs/heads/workspace/ refs/heads/personal/); do git -C "$r" rev-parse -q --verify "$ref" >/dev/null; n=$((n+1)); done; n=$((n+1))
  for ref in develop next HEAD; do git -C "$r" rev-parse -q --verify "refs/remotes/origin/$ref" >/dev/null || true; n=$((n+1)); done
  echo "per-ref $n"
}

git_version_only() { git --version >/dev/null; }
p95() { sort -n | awk '{a[NR]=$1} END {i=int(NR*0.95+0.999); if(i>NR)i=NR; m=int((NR+1)/2); printf "p50=%.1fms p95=%.1fms max=%.1fms n=%d", a[m]/1e6, a[i]/1e6, a[NR]/1e6, NR}'; }

time_it() { # fn repo -> nanoseconds per run, one per line
  local fn="$1" r="$2" i s e
  for i in $(seq "$RUNS"); do s=$(date +%s%N); "$fn" "$r" >/dev/null; e=$(date +%s%N); echo $((e-s)); done; }

for r in "$@"; do
  : >"$LOG"; own=$(PATH="$WRAP:$PATH" resolve_batched "$r"); wrapped=$(wc -l <"$LOG")
  : >"$LOG"; ctl=$(PATH="$WRAP:$PATH" resolve_per_ref "$r"); ctlw=$(wc -l <"$LOG")
  echo "== $(basename "$r")  batched: $own (wrapper counted $wrapped)  per-ref control: $ctl (wrapper counted $ctlw)"
  echo "   batched  $(time_it resolve_batched "$r" | p95)"
  echo "   per-ref  $(time_it resolve_per_ref "$r" | p95)"
  echo "   1 child  $(time_it git_version_only "$r" | p95)"
done
