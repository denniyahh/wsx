# The single source of truth for the verification image. The dev container
# (devcontainer.json builds from this file), scripts/check-in-container.sh and
# every CI job image all use this FROM line.
#
# An exact tag, never a floating one. Tag and digest verified against the
# registry on 2026-10-22.
#
# Dependabot's `docker` ecosystem bumps this line. scripts/assert-image-parity.sh
# then fails CI until every workflow image matches it, so such a PR must update
# the workflows too.
#
# Nothing else belongs here: rust-toolchain.toml selects the compiler and
# scripts/install-tools.sh installs the pinned tools.
FROM mcr.microsoft.com/devcontainers/rust:2.1.3-1-trixie
