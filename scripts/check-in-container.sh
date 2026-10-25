#!/usr/bin/env bash
# Run scripts/check.sh inside the pinned devcontainer image (the same image CI
# uses), so a green local run and a green CI run mean the same thing.
#
# The host toolchain, libc and OS are not CI's. A change that passes on a
# Fedora host can fail on a Debian runner minutes later, and finding out why
# costs far more than running the gate in the right place. Verification
# belongs in the pinned image; editing, git and agent tooling stay on the host
# where your credentials are.
#
# To verify on the host instead, accepting that a green result does NOT imply
# CI will be green, run scripts/check.sh directly.
#
# The image is read from the FROM line of .devcontainer/Dockerfile, so there is
# exactly one place to bump it. Requires a reachable Docker daemon.
#
# The checkout and its git directories are mounted read-only: verification
# must not be able to change the source. Build output, including nextest's
# store, goes to a named volume mounted over /workspace/target.
#
#   scripts/check-in-container.sh [TARGET] [ARGS...]
#
# TARGET defaults to `all`; TARGET and ARGS pass through to scripts/check.sh,
# which enforces its own argument rules. Exit 1 if the image cannot be read or
# Docker is unreachable, otherwise check.sh's exit code.
set -euo pipefail

# git is legitimately required here: the gitdir mounts below need it. The
# repository is the one holding this script, never the caller's directory:
# run from another checkout (the main checkout holds the linked worktrees
# under .worktrees/), the caller's tree would be gated under this name.
SCRIPT_DIR="$(CDPATH="" cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git -C "$SCRIPT_DIR" rev-parse --show-toplevel)"
cd "$REPO_ROOT"

TARGET="${1:-all}"
shift || true

# Single source of truth for the image: the first FROM line of the Dockerfile.
# `sed -n` exits 0 whether or not it printed anything, so the empty check
# below is what catches a missing or malformed FROM line.
if ! IMAGE="$(sed -n '/^FROM[[:space:]]/{s/^FROM[[:space:]]\{1,\}\([^[:space:]]\{1,\}\).*/\1/p;q;}' \
    .devcontainer/Dockerfile)"; then
    IMAGE=""
fi
if [ -z "$IMAGE" ]; then
    echo "error: could not read the FROM image from .devcontainer/Dockerfile" >&2
    exit 1
fi

DOCKER=(docker)

if ! "${DOCKER[@]}" info >/dev/null 2>&1; then
    cat >&2 <<'EOF'
error: Docker daemon is not reachable.

Verification runs in the pinned devcontainer image so that local results
match CI. Start Docker and retry.

To deliberately verify on the host instead — accepting that a green result
does NOT imply CI will be green — run scripts/check.sh directly.
EOF
    exit 1
fi

# Persist the container's target/ and cargo registry across runs so repeat
# invocations are incremental. Named volumes, not bind mounts: the container
# toolchain's artifacts must never collide with the host's target/, which is
# built against a different libc.
#
# HYG-02 (devflow 41-02 review, reproduced there): the volume names are derived
# PER CHECKOUT. A single shared volume, mounted at the same container path
# `/workspace` for the main checkout and every worktree, makes cargo's
# path-keyed fingerprints alias across checkouts: a worktree run then reuses a
# stale rmeta built from another checkout's sources (false RED), and the same
# aliasing yields a false GREEN in the other direction. Suffix the volume with
# a hash of REPO_ROOT so each checkout gets its own cache.
CACHE_SUFFIX="$(printf '%s' "$REPO_ROOT" | sha256sum | cut -c1-12)"
TARGET_VOLUME="wsx-ci-target-${CACHE_SUFFIX}"
REGISTRY_VOLUME="wsx-ci-registry-${CACHE_SUFFIX}"
"${DOCKER[@]}" volume create "$TARGET_VOLUME" >/dev/null
"${DOCKER[@]}" volume create "$REGISTRY_VOLUME" >/dev/null

# HYG-02 (devflow 41-02, re-derived from real container runs): a git
# WORKTREE's `.git` is a FILE (`gitdir: <main>/.git/worktrees/<N>`) pointing
# at the main repo's gitdir, which lives OUTSIDE the worktree and therefore
# outside the single `/workspace` mount below. Inside the container git
# resolves that absolute gitdir path to nothing and every git-dependent check
# dies with `fatal: not a git repository`. Two paths must come through: the
# worktree gitdir itself AND the COMMON gitdir it shares (`commondir`, the
# main repo's `.git`, where refs and objects live), each at its own host path.
# The MAIN checkout is unaffected: its gitdir is inside REPO_ROOT. The fix is
# the mount, not skipping git-dependent checks in containers. Both are bound
# read-only, like the checkout.
GITDIR="$(git rev-parse --absolute-git-dir)"
COMMONDIR="$(git rev-parse --path-format=absolute --git-common-dir)"
GITDIR_MOUNT=()
_mounted=""
for dir in "$GITDIR" "$COMMONDIR"; do
    if [ "${dir#"$REPO_ROOT"/}" = "$dir" ]; then
        # Dedup against the PATHS already mounted (not the `-v dir:dir` array
        # entries): a --separate-git-dir clone has gitdir == commondir, and a
        # naive grep for the bare path inside the `-v` array can never match.
        case " $_mounted " in
            *" $dir "*) ;;
            *)
                echo "==> worktree detected: gitdir component $dir is outside the mount; binding it through"
                GITDIR_MOUNT+=(-v "$dir:$dir:ro")
                _mounted="$_mounted $dir"
                ;;
        esac
    fi
done

# The target volume is mounted at /workspace/target rather than elsewhere
# through CARGO_TARGET_DIR: nextest writes its store (the JUnit report that
# .config/nextest.toml enables) under <workspace root>/target/nextest whatever
# CARGO_TARGET_DIR says, so with the checkout read-only that path must be a
# writable mount. Docker cannot create a mount point inside a read-only bind,
# so make sure the git-ignored target/ exists on the host first; the host gate
# creates the same directory. A volume last used with a different mount path
# keeps stale CARGO_BIN_EXE paths in its fingerprints and fails the tests;
# delete the wsx-ci-target-* volumes after changing this mount.
mkdir -p "$REPO_ROOT/target"

# Allocate a TTY only when stdout is one: with `-t` and no terminal, the
# output gets CRLF line endings, which pollutes captured and CI logs.
TTY=()
if [ -t 1 ]; then
    TTY=(-t)
fi

echo "==> image:  $IMAGE"
echo "==> target: $TARGET"

# cargo-nextest and cargo-deny are not in the image; install the pinned,
# checksum-verified builds into the throwaway container before the gate runs.
exec "${DOCKER[@]}" run --rm \
    "${TTY[@]}" \
    "${GITDIR_MOUNT[@]}" \
    -v "$REPO_ROOT":/workspace:ro \
    -v "$TARGET_VOLUME":/workspace/target \
    -v "$REGISTRY_VOLUME":/usr/local/cargo/registry \
    -w /workspace \
    "$IMAGE" \
    bash -c 'scripts/install-tools.sh install cargo-nextest cargo-deny >/dev/null && exec scripts/check.sh "$@"' \
    _ "$TARGET" "$@"
