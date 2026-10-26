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
CDPATH="" cd -- "$(dirname "${BASH_SOURCE[0]}")/.."

usage() {
    cat >&2 <<'EOF'
usage: scripts/check.sh [all|fmt|clippy|test [filters...]|deps|coverage [filters...]]

  all     fmt + clippy + test   (default)
  fmt     cargo fmt --check, plus rustfmt --check on every tracked .rs file
  clippy  cargo clippy --locked --workspace --all-targets -- -D warnings
  test    cargo nextest run --locked --workspace --no-fail-fast --no-tests=fail
          [-E <filterset> | test-name filter], at most one; no other
          nextest option
  deps    cargo deny --locked check (advisories, licenses, bans, sources);
          not part of all
  coverage
          CARGO_LLVM_COV_SETUP=no cargo llvm-cov nextest --locked --workspace
          --no-fail-fast --no-tests=fail, with the filters test takes; not
          part of all, and no CI job runs it

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

if [ "$TARGET" != "test" ] && [ "$TARGET" != "coverage" ] && [ "$#" -gt 0 ]; then
    echo "error: check.sh $TARGET takes no arguments" >&2
    usage
fi

# Only test selection passes through to nextest: a filterset (-E/--filterset)
# or a test-name filter. Any other option could stop nextest from running tests
# while this script still prints OK (`--help`, `--no-run`), or change what a
# green run means (`--no-tests`, `--run-ignored`), so it is refused.
# At most one selector: nextest ORs several together, so a second one that
# matches nothing still passes on the first one's tests. coverage runs nextest
# too, so it takes the same selection under the same rules.
if [ "$TARGET" = "test" ] || [ "$TARGET" = "coverage" ]; then
    args=("$@")
    i=0
    selectors=0
    while [ "$i" -lt "${#args[@]}" ]; do
        arg="${args[$i]}"
        case "$arg" in
            -E | --filterset)
                if [ $((i + 1)) -ge "${#args[@]}" ]; then
                    echo "error: $arg needs a filterset expression" >&2
                    exit 2
                fi
                i=$((i + 2))
                selectors=$((selectors + 1))
                ;;
            --filterset=*)
                i=$((i + 1))
                selectors=$((selectors + 1))
                ;;
            --no-tests | --no-tests=*)
                echo "error: --no-tests is fixed to fail by check.sh and cannot be overridden" >&2
                exit 2
                ;;
            -*)
                echo "error: check.sh $TARGET passes through only -E/--filterset <expr> and test-name filters, not '$arg'" >&2
                exit 2
                ;;
            *)
                i=$((i + 1))
                selectors=$((selectors + 1))
                ;;
        esac
    done
    if [ "$selectors" -gt 1 ]; then
        echo "error: check.sh $TARGET takes at most one -E/--filterset expression or test name; run one test per command, because a second selector that matches nothing still passes" >&2
        exit 2
    fi
fi

run_fmt() {
    require_tool cargo "install rustup from https://rustup.rs"
    show_toolchain
    require_tool cargo-fmt "rustup component add rustfmt"
    echo "==> cargo fmt --check"
    cargo fmt --check
    # cargo fmt only follows `mod` items it can see, and never one declared
    # inside a macro (the harness's scenario modules are), so check every
    # tracked .rs file directly as well.
    echo "==> rustfmt --check on every tracked .rs file"
    git ls-files -z -- '*.rs' | xargs -0 rustfmt --edition 2024 --check
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
    # The pin is checked through cargo, as in run_deps: cargo puts
    # $CARGO_HOME/bin first only when it is absent from PATH and otherwise
    # follows PATH order, so only `cargo nextest --version` is guaranteed to
    # run the binary `cargo nextest run` runs below.
    echo "==> scripts/install-tools.sh verify-cargo cargo-nextest"
    scripts/install-tools.sh verify-cargo cargo-nextest
    echo "==> cargo nextest run --locked --workspace --no-fail-fast --no-tests=fail $*"
    cargo nextest run --locked --workspace --no-fail-fast --no-tests=fail "$@"
}

run_deps() {
    # Not part of `all`: the RustSec advisory database changes under unrelated
    # work, so a new advisory would turn every unrelated change red. CI runs
    # this in its own advisory job instead.
    # A missing tool is a hard error here, never a skip — a skipped check prints
    # nothing a log reader can tell apart from a passing one.
    require_tool cargo "install rustup from https://rustup.rs"
    require_tool cargo-deny "run scripts/install-tools.sh install cargo-deny"
    # Check the pin before scanning: a package-manager upgrade must not
    # silently replace the scanner this gate was validated against. The
    # version is asked of `cargo deny` itself, because cargo's subcommand
    # lookup (PATH order, with $CARGO_HOME/bin first only when it is absent
    # from PATH) decides which binary the scan runs.
    echo "==> scripts/install-tools.sh verify-cargo cargo-deny"
    scripts/install-tools.sh verify-cargo cargo-deny
    echo "==> cargo deny --locked check"
    cargo deny --locked check
}

run_coverage() {
    # Not part of `all`, and no CI job runs it yet (deferred until real
    # commands exist): a threshold set now would be meaningless. Like deps,
    # it is a hard failure without its pinned tool.
    #
    # CARGO_LLVM_COV_SETUP=no is set here, on the command itself, so a
    # missing llvm-tools component fails the run instead of being installed
    # silently; rust-toolchain.toml lists the component for every environment.
    require_tool cargo "install rustup from https://rustup.rs"
    show_toolchain
    require_tool cargo-llvm-cov "run scripts/install-tools.sh install cargo-llvm-cov"
    require_tool cargo-nextest "run scripts/install-tools.sh install cargo-nextest"
    echo "==> scripts/install-tools.sh verify-cargo cargo-llvm-cov cargo-nextest"
    scripts/install-tools.sh verify-cargo cargo-llvm-cov cargo-nextest
    echo "==> CARGO_LLVM_COV_SETUP=no cargo llvm-cov nextest --locked --workspace --no-fail-fast --no-tests=fail $*"
    CARGO_LLVM_COV_SETUP=no cargo llvm-cov nextest --locked --workspace --no-fail-fast \
        --no-tests=fail "$@"
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
    deps) run_deps ;;
    coverage) run_coverage "$@" ;;
    -h | --help | help) usage ;;
    *)
        echo "error: unknown target '$TARGET'" >&2
        usage
        ;;
esac

echo "==> check.sh: $TARGET OK"
