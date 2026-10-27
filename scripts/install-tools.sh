#!/usr/bin/env bash
# Install or verify the tools and scanners the gate runs: cargo-nextest
# (`scripts/check.sh test`), cargo-deny (`scripts/check.sh deps`) and gitleaks
# (the pre-commit secret scan). It also pins cargo-llvm-cov, which only
# `scripts/check.sh coverage` runs (not part of `all`, and no CI job); that
# target fails without it and sets CARGO_LLVM_COV_SETUP=no, so it never
# installs a toolchain component itself.
#
# This is the single place their versions are pinned (SUP-04). CI,
# scripts/check-in-container.sh and contributors all install through it, so a
# version bump is one edit here and nowhere else.
#
# It downloads each tool's release tarball and checks the pinned sha256 before
# extracting anything, instead of compiling with `cargo install`: an install
# takes seconds, and a tampered or swapped artefact is refused rather than run.
# Only x86_64 Linux artefacts are pinned; other platforms are refused.
#
# It also runs inside the pinned devcontainer image, which has bash, coreutils,
# grep, sed, mawk, curl and tar but no rg. Use only those, and call every
# external tool directly (never in a process substitution or behind
# `|| true`), so `set -euo pipefail` stops on a missing or failing tool.
#
#   install TOOL...  download, verify and install into
#                    ${WSX_TOOLS_DIR:-${CARGO_HOME:-$HOME/.cargo}/bin}
#   verify TOOL...   the tool on PATH, run with its subcommand where it needs
#                    one, must report the pinned version
#   verify-cargo TOOL...
#                    `cargo <subcommand> --version` must report the pinned
#                    version, for cargo-* tools only. This asks cargo itself,
#                    so the binary checked is the binary `cargo <subcommand>`
#                    runs: cargo puts $CARGO_HOME/bin first only when it is
#                    absent from PATH, and otherwise follows PATH order
#                    (cargo#11020), which no PATH lookup here can reproduce.
#
# Exit 2 on a usage error, 1 on any failure.
set -euo pipefail

CARGO_NEXTEST_VERSION=0.9.148
CARGO_NEXTEST_SHA256=223ba936714cc861a9cdedad5a8eb664dfdd69d6fbdf6cea6aa8184f523290dc
CARGO_DENY_VERSION=0.20.2
CARGO_DENY_SHA256=9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f
GITLEAKS_VERSION=8.30.1
GITLEAKS_SHA256=551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb
CARGO_LLVM_COV_VERSION=0.9.1
CARGO_LLVM_COV_SHA256=3fca950394a3c49457657c158b1619cec8dfd2647ae5b48746734c0ab969a522

usage() {
    cat >&2 <<'EOF'
usage: scripts/install-tools.sh install TOOL...
       scripts/install-tools.sh verify TOOL...
       scripts/install-tools.sh verify-cargo TOOL...

  TOOL is one of: cargo-nextest, cargo-deny, gitleaks, cargo-llvm-cov

  install  download the pinned release tarball, check its sha256, and install
           the binary into $WSX_TOOLS_DIR (default $CARGO_HOME/bin, else
           ~/.cargo/bin); x86_64 Linux only
  verify   fail unless the tool on PATH, run with its subcommand where it
           needs one, reports the pinned version
  verify-cargo
           fail unless `cargo <subcommand> --version` reports the pinned
           version; cargo-nextest, cargo-deny and cargo-llvm-cov only
EOF
    exit 2
}

# Set the pin, download URL, archive member, version argument and version
# pattern for one tool, and direct_sub: the subcommand word the binary needs
# before version_arg when it is run directly rather than by cargo (empty for
# most tools). An unknown name is a usage error.
tool_info() {
    case "$1" in
        cargo-nextest)
            version="$CARGO_NEXTEST_VERSION"
            sha256="$CARGO_NEXTEST_SHA256"
            url="https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-${version}/cargo-nextest-${version}-x86_64-unknown-linux-gnu.tar.gz"
            member="cargo-nextest"
            version_arg="--version"
            direct_sub=""
            # Release builds append " (hash date)"; the brew build does not.
            pattern="^cargo-nextest ${version//./\\.}( |\$)"
            ;;
        cargo-deny)
            version="$CARGO_DENY_VERSION"
            sha256="$CARGO_DENY_SHA256"
            url="https://github.com/EmbarkStudios/cargo-deny/releases/download/${version}/cargo-deny-${version}-x86_64-unknown-linux-musl.tar.gz"
            member="cargo-deny-${version}-x86_64-unknown-linux-musl/cargo-deny"
            version_arg="--version"
            direct_sub=""
            pattern="^cargo-deny ${version//./\\.}\$"
            ;;
        gitleaks)
            version="$GITLEAKS_VERSION"
            sha256="$GITLEAKS_SHA256"
            url="https://github.com/gitleaks/gitleaks/releases/download/v${version}/gitleaks_${version}_linux_x64.tar.gz"
            member="gitleaks"
            version_arg="version"
            direct_sub=""
            pattern="^v?${version//./\\.}\$"
            ;;
        cargo-llvm-cov)
            version="$CARGO_LLVM_COV_VERSION"
            sha256="$CARGO_LLVM_COV_SHA256"
            url="https://github.com/taiki-e/cargo-llvm-cov/releases/download/v${version}/cargo-llvm-cov-x86_64-unknown-linux-musl.tar.gz"
            member="cargo-llvm-cov"
            version_arg="--version"
            # Run directly, the binary rejects a bare --version ("expected
            # subcommand 'llvm-cov'"); cargo passes that word itself.
            direct_sub="llvm-cov"
            pattern="^cargo-llvm-cov ${version//./\\.}\$"
            ;;
        *)
            echo "error: unknown tool '$1'" >&2
            usage
            ;;
    esac
}

# $1 = tool. Print the command that installs it where it is looked up: with
# WSX_TOOLS_DIR set (check.sh sets it with a checkout's .tools/bin, which goes
# first on PATH), the command names that directory, so following it replaces
# the copy that is found rather than installing one it shadows.
install_hint() {
    if [ -n "${WSX_TOOLS_DIR:-}" ]; then
        printf 'WSX_TOOLS_DIR=%q scripts/install-tools.sh install %s' "$WSX_TOOLS_DIR" "$1"
    else
        printf 'scripts/install-tools.sh install %s' "$1"
    fi
}

# $1 = tool, $2 = where it was found (for messages), then the command that
# runs it. Exit 1 unless the first line of its version output matches the pin.
check_version() {
    local tool="$1" where="$2" output first
    shift 2
    tool_info "$tool"
    if ! output="$("$@" "$version_arg")"; then
        echo "error: $tool $where failed to report its version; run $(install_hint "$tool")" >&2
        exit 1
    fi
    first="${output%%$'\n'*}"
    if [[ ! "$first" =~ $pattern ]]; then
        echo "error: $tool $where reports '$first', expected $version; run $(install_hint "$tool")" >&2
        exit 1
    fi
}

cmd_verify() {
    local tool bin
    for tool in "$@"; do
        if ! bin="$(command -v "$tool")"; then
            echo "error: $tool is not on PATH; run $(install_hint "$tool")" >&2
            exit 1
        fi
        tool_info "$tool"
        check_version "$tool" "at $bin" "$bin" ${direct_sub:+"$direct_sub"}
        echo "==> $tool $version OK"
    done
}

cmd_verify_cargo() {
    local tool sub
    for tool in "$@"; do
        if [ "${tool#cargo-}" = "$tool" ]; then
            echo "error: verify-cargo checks cargo subcommands only, not '$tool'" >&2
            usage
        fi
        sub="${tool#cargo-}"
        check_version "$tool" "run as 'cargo $sub'" cargo "$sub"
        echo "==> cargo $sub: $tool $version OK"
    done
}

# $1 = tool. Download into WORK_DIR, refuse a checksum mismatch before
# extracting, then install the one binary into DEST.
install_one() {
    local tool="$1" archive extract_dir
    tool_info "$tool"
    archive="$WORK_DIR/$tool.tar.gz"
    extract_dir="$WORK_DIR/$tool"
    echo "==> downloading $url"
    # No header or user-agent options: nothing identifying the operator is sent.
    # --disable must come first: it stops curl reading a .curlrc, which could
    # otherwise add headers, a user agent or a proxy to the request.
    # curl's and tar's own exit codes are mapped to 1: tar's fatal-error code
    # is 2, which would read as a usage error.
    if ! curl --disable --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
        --output "$archive" "$url"; then
        echo "error: download failed for $tool $version" >&2
        exit 1
    fi
    printf '%s  %s\n' "$sha256" "$archive" >"$archive.sha256"
    if ! sha256sum --check --status "$archive.sha256"; then
        echo "error: checksum mismatch for $tool $version: refusing to install" >&2
        exit 1
    fi
    mkdir "$extract_dir"
    if ! tar -xzf "$archive" -C "$extract_dir" "$member"; then
        echo "error: could not extract $member from the $tool $version archive" >&2
        exit 1
    fi
    install -m 0755 "$extract_dir/$member" "$DEST/$tool"
    check_version "$tool" "at $DEST/$tool" "$DEST/$tool" ${direct_sub:+"$direct_sub"}
    echo "==> installed $tool $version to $DEST"
}

cmd_install() {
    local os arch tool
    os="$(uname -s)"
    arch="$(uname -m)"
    if [ "$os" != "Linux" ] || [ "$arch" != "x86_64" ]; then
        echo "error: only x86_64 Linux release binaries are pinned (this is $os $arch)" >&2
        exit 1
    fi
    for tool in curl sha256sum tar; do
        if ! command -v "$tool" >/dev/null 2>&1; then
            echo "error: '$tool' is required for 'install-tools.sh install' but is not on PATH" >&2
            exit 1
        fi
    done
    DEST="${WSX_TOOLS_DIR:-${CARGO_HOME:-$HOME/.cargo}/bin}"
    mkdir -p "$DEST"
    # mktemp honours TMPDIR. The trap removes the work dir on every exit,
    # including a refused checksum.
    WORK_DIR="$(mktemp -d)"
    trap 'rm -rf -- "$WORK_DIR"' EXIT
    for tool in "$@"; do
        install_one "$tool"
    done
}

if [ "$#" -lt 2 ]; then
    usage
fi
MODE="$1"
shift

# Check every name before doing any work, so a typo never leaves a partial
# install behind.
for name in "$@"; do
    tool_info "$name"
done

case "$MODE" in
    install) cmd_install "$@" ;;
    verify) cmd_verify "$@" ;;
    verify-cargo) cmd_verify_cargo "$@" ;;
    *)
        echo "error: unknown mode '$MODE'" >&2
        usage
        ;;
esac
