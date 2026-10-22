#!/usr/bin/env bash
# The single definition of "is this green?" — used by CI, by
# scripts/check-in-container.sh and by humans. If a check is not in here, it
# is not enforced anywhere; if it is, it is enforced identically everywhere.
#
# Deliberately NOT parameterised by environment. The whole point is that the
# same commands run on a developer machine and on a runner, so a green local
# run means the same thing as a green CI run.
#
# A missing tool is a hard error, never a skip: a skipped check prints
# nothing a log reader can tell apart from a passing one.
set -euo pipefail

# Repo root from the script's own location, so the gate never depends on git.
cd "$(dirname "${BASH_SOURCE[0]}")/.."

usage() {
    cat >&2 <<'EOF'
usage: scripts/check.sh [all|fmt|clippy|test [nextest args...]]

  all     fmt + clippy + test   (default)
  fmt     cargo fmt --check
  clippy  cargo clippy --locked --workspace --all-targets -- -D warnings
  test    cargo nextest run --locked --workspace --no-fail-fast --no-tests=fail
          [nextest args...]; --no-tests is fixed and cannot be overridden

Run inside the pinned devcontainer for CI parity:
  scripts/check-in-container.sh [target]
EOF
    exit 2
}

# $1 = binary that must be on PATH, $2 = how to get it.
require_tool() {
    if ! command -v "$1" >/dev/null 2>&1; then
        echo "error: '$1' is required for 'check.sh $TARGET' but is not on PATH; $2" >&2
        exit 1
    fi
}

# Print the compiler once per invocation, so every container and CI log
# records which rustc produced the result (DEV-02). rust-toolchain.toml pins
# it; this line is the evidence that the pin was honoured.
TOOLCHAIN_SHOWN=0
show_toolchain() {
    if [ "$TOOLCHAIN_SHOWN" -eq 1 ]; then
        return
    fi
    require_tool rustc "install rustup from https://rustup.rs"
    # A plain assignment, not `echo "$(rustc --version)"`: only the assignment
    # passes rustc's exit status to `set -e`, so a broken rustc (for example a
    # pinned toolchain rustup cannot install) stops the gate here.
    local version
    version="$(rustc --version)"
    echo "==> $version"
    TOOLCHAIN_SHOWN=1
}

TARGET="${1:-all}"
shift || true

if [ "$TARGET" != "test" ] && [ "$#" -gt 0 ]; then
    echo "error: check.sh $TARGET takes no arguments" >&2
    usage
fi

if [ "$TARGET" = "test" ]; then
    for arg in "$@"; do
        if [ "$arg" = "--no-tests" ] || [[ "$arg" == --no-tests=* ]]; then
            echo "error: --no-tests is fixed to fail by check.sh and cannot be overridden" >&2
            exit 2
        fi
    done
fi

run_fmt() {
    require_tool cargo "install rustup from https://rustup.rs"
    show_toolchain
    require_tool cargo-fmt "rustup component add rustfmt"
    echo "==> cargo fmt --check"
    cargo fmt --check
}

run_clippy() {
    # --all-targets so test code is linted too; the env-mutation ban in
    # clippy.toml exists for test code above all.
    require_tool cargo "install rustup from https://rustup.rs"
    show_toolchain
    require_tool cargo-clippy "rustup component add clippy"
    echo "==> cargo clippy --locked --workspace --all-targets -- -D warnings"
    cargo clippy --locked --workspace --all-targets -- -D warnings
}

run_test() {
    # --no-fail-fast deliberately: without it the run stops at the first
    # failing test, so one failure hides every other one. One run should
    # report everything that is broken, not the first thing.
    #
    # --no-tests=fail comes before the caller's arguments, and callers may not
    # pass their own --no-tests (refused above): a filter that matches nothing
    # must fail, never report green over zero tests.
    require_tool cargo "install rustup from https://rustup.rs"
    show_toolchain
    require_tool cargo-nextest "run scripts/install-tools.sh install cargo-nextest"
    echo "==> cargo nextest run --locked --workspace --no-fail-fast --no-tests=fail $*"
    cargo nextest run --locked --workspace --no-fail-fast --no-tests=fail "$@"
}

case "$TARGET" in
    all)
        # Cheapest first: fail on formatting before paying for a compile.
        run_fmt
        run_clippy
        run_test
        ;;
    fmt) run_fmt ;;
    clippy) run_clippy ;;
    test) run_test "$@" ;;
    -h | --help | help) usage ;;
    *)
        echo "error: unknown target '$TARGET'" >&2
        usage
        ;;
esac

echo "==> check.sh: $TARGET OK"
