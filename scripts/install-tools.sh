#!/usr/bin/env bash
# Install or verify the tools and scanners the gate runs: cargo-nextest
# (`scripts/check.sh test`), cargo-deny (`scripts/check.sh deps`) and gitleaks
# (the pre-commit secret scan).
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
#   verify TOOL...   the tool on PATH must report the pinned version
#
# Exit 2 on a usage error, 1 on any failure.
set -euo pipefail

CARGO_NEXTEST_VERSION=0.9.148
CARGO_NEXTEST_SHA256=223ba936714cc861a9cdedad5a8eb664dfdd69d6fbdf6cea6aa8184f523290dc
CARGO_DENY_VERSION=0.20.2
CARGO_DENY_SHA256=9f12ed4c49936e09b48bf862b595cde2fe64fcbd9d74dfacac6131ca824c8d5f
GITLEAKS_VERSION=8.30.1
GITLEAKS_SHA256=551f6fc83ea457d62a0d98237cbad105af8d557003051f41f3e7ca7b3f2470eb

usage() {
    cat >&2 <<'EOF'
usage: scripts/install-tools.sh install TOOL...
       scripts/install-tools.sh verify TOOL...

  TOOL is one of: cargo-nextest, cargo-deny, gitleaks

  install  download the pinned release tarball, check its sha256, and install
           the binary into $WSX_TOOLS_DIR (default $CARGO_HOME/bin, else
           ~/.cargo/bin); x86_64 Linux only
  verify   fail unless the tool on PATH reports the pinned version
EOF
    exit 2
}

# Set the pin, download URL, archive member, version argument and version
# pattern for one tool. An unknown name is a usage error.
tool_info() {
    case "$1" in
        cargo-nextest)
            version="$CARGO_NEXTEST_VERSION"
            sha256="$CARGO_NEXTEST_SHA256"
            url="https://github.com/nextest-rs/nextest/releases/download/cargo-nextest-${version}/cargo-nextest-${version}-x86_64-unknown-linux-gnu.tar.gz"
            member="cargo-nextest"
            version_arg="--version"
            # Release builds append " (hash date)"; the brew build does not.
            pattern="^cargo-nextest ${version//./\\.}( |\$)"
            ;;
        cargo-deny)
            version="$CARGO_DENY_VERSION"
            sha256="$CARGO_DENY_SHA256"
            url="https://github.com/EmbarkStudios/cargo-deny/releases/download/${version}/cargo-deny-${version}-x86_64-unknown-linux-musl.tar.gz"
            member="cargo-deny-${version}-x86_64-unknown-linux-musl/cargo-deny"
            version_arg="--version"
            pattern="^cargo-deny ${version//./\\.}\$"
            ;;
        gitleaks)
            version="$GITLEAKS_VERSION"
            sha256="$GITLEAKS_SHA256"
            url="https://github.com/gitleaks/gitleaks/releases/download/v${version}/gitleaks_${version}_linux_x64.tar.gz"
            member="gitleaks"
            version_arg="version"
            pattern="^v?${version//./\\.}\$"
            ;;
        *)
            echo "error: unknown tool '$1'" >&2
            usage
            ;;
    esac
}

# $1 = tool, $2 = path to its binary. Exit 1 unless the first line of its
# version output matches the pin.
check_version() {
    local output first
    tool_info "$1"
    if ! output="$("$2" "$version_arg")"; then
        echo "error: $1 at $2 failed to report its version; run scripts/install-tools.sh install $1" >&2
        exit 1
    fi
    first="${output%%$'\n'*}"
    if [[ ! "$first" =~ $pattern ]]; then
        echo "error: $1 at $2 reports '$first', expected $version; run scripts/install-tools.sh install $1" >&2
        exit 1
    fi
}

cmd_verify() {
    local tool bin
    for tool in "$@"; do
        if ! bin="$(command -v "$tool")"; then
            echo "error: $tool is not on PATH; run scripts/install-tools.sh install $tool" >&2
            exit 1
        fi
        check_version "$tool" "$bin"
        echo "==> $tool $version OK"
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
    curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
        --output "$archive" "$url"
    printf '%s  %s\n' "$sha256" "$archive" >"$archive.sha256"
    if ! sha256sum --check --status "$archive.sha256"; then
        echo "error: checksum mismatch for $tool $version: refusing to install" >&2
        exit 1
    fi
    mkdir "$extract_dir"
    tar -xzf "$archive" -C "$extract_dir" "$member"
    install -m 0755 "$extract_dir/$member" "$DEST/$tool"
    check_version "$tool" "$DEST/$tool"
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
    *)
        echo "error: unknown mode '$MODE'" >&2
        usage
        ;;
esac
