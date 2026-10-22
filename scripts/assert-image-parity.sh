#!/usr/bin/env bash
# Fail unless every CI job runs in the image the devcontainer pins.
#
# Local checks predict CI only while both run in the same image. The pin lives
# in one place, the first FROM line of .devcontainer/Dockerfile, but GitHub
# Actions cannot read a job's container image from a file, so every workflow
# job repeats it as `container: image:`. This script compares the Dockerfile
# pin with every `image:` in .github/workflows/*.yml and *.yaml, so the copies
# cannot drift silently. A Dependabot PR that bumps only the Dockerfile fails
# here until the workflows in the same PR match.
#
# It reads files only: no argument, and no copy of the image anywhere else to
# keep in step (devflow passed the image as an argument, which repeated it a
# second time beside `container.image`, so bumping one copy passed the check
# while CI ran another image). The root comes from the script's own location,
# never from git.
#
# CI's Test job runs this inside the pinned image, which has bash, coreutils,
# grep, sed, mawk and find but no rg. Parsing is done with bash builtins, so
# the only external command is `dirname`, called directly where `set -e`
# stops on its failure.
#
# Exit 2 on any argument; exit 1 on a missing or non-exact pin, no workflow
# files, a job without a container image, zero images, or any mismatch.
set -euo pipefail

if [ "$#" -ne 0 ]; then
    echo "usage: scripts/assert-image-parity.sh   (takes no arguments)" >&2
    exit 2
fi

cd "$(dirname "${BASH_SOURCE[0]}")/.."

DOCKERFILE=.devcontainer/Dockerfile
# An exact devcontainer tag such as rust:2.1.3-1-trixie. Never a floating one:
# Dependabot's docker updater has rewritten such tags to less specific ones
# (dependabot-core#15530), and this check would otherwise accept that.
EXACT_TAG='^[^[:space:]:@]+:[0-9]+\.[0-9]+\.[0-9]+-[0-9]+-[a-z]+$'

if [ ! -f "$DOCKERFILE" ]; then
    echo "error: $DOCKERFILE not found" >&2
    exit 1
fi

# The first token after FROM on the first FROM line, as
# scripts/check-in-container.sh reads it.
pin=""
while IFS= read -r line || [ -n "$line" ]; do
    if [[ $line =~ ^FROM[[:space:]]+([^[:space:]]+) ]]; then
        pin="${BASH_REMATCH[1]}"
        break
    fi
done < "$DOCKERFILE"

if [ -z "$pin" ]; then
    echo "error: could not read the FROM image from $DOCKERFILE" >&2
    exit 1
fi
if ! [[ $pin =~ $EXACT_TAG ]]; then
    echo "error: $DOCKERFILE pins '$pin', which is not an exact version tag" >&2
    exit 1
fi

shopt -s nullglob
workflows=(.github/workflows/*.yml .github/workflows/*.yaml)
shopt -u nullglob
if [ "${#workflows[@]}" -eq 0 ]; then
    echo "error: no workflow files found in .github/workflows; nothing to compare with $pin" >&2
    exit 1
fi

# Strip the key, a trailing ` #` comment, surrounding whitespace and quotes.
image_value() {
    local value="$1"
    value="${value%%[[:space:]]#*}"
    value="${value#"${value%%[![:space:]]*}"}"
    value="${value%"${value##*[![:space:]]}"}"
    if [[ $value =~ ^\"(.*)\"$ || $value =~ ^\'(.*)\'$ ]]; then
        value="${BASH_REMATCH[1]}"
    fi
    printf '%s' "$value"
}

total_images=0
mismatches=()
for file in "${workflows[@]}"; do
    jobs=0
    images=0
    # Only lines whose first non-blank text is the key match, so comments may
    # mention either word.
    while IFS= read -r line || [ -n "$line" ]; do
        if [[ $line =~ ^[[:space:]]*runs-on: ]]; then
            jobs=$((jobs + 1))
        elif [[ $line =~ ^[[:space:]]*image:(.*)$ ]]; then
            images=$((images + 1))
            image="$(image_value "${BASH_REMATCH[1]}")"
            if [ "$image" != "$pin" ]; then
                mismatches+=("$file: $image")
            fi
        fi
    done < "$file"
    if [ "$images" -lt "$jobs" ]; then
        echo "error: $file has $jobs job(s) but only $images container image(s); every CI job must run in the pinned image" >&2
        exit 1
    fi
    total_images=$((total_images + images))
done

if [ "$total_images" -eq 0 ]; then
    echo "error: no container image found in any workflow file; every CI job must run in $pin" >&2
    exit 1
fi

if [ "${#mismatches[@]}" -ne 0 ]; then
    {
        echo "error: CI image does not match the devcontainer pin."
        echo
        echo "  $DOCKERFILE (the pin): $pin"
        for entry in "${mismatches[@]}"; do
            echo "  $entry"
        done
        echo
        echo "These must be identical or local checks stop predicting CI. Update every copy in the same change."
    } >&2
    exit 1
fi

echo "image parity OK: $pin ($total_images workflow image(s))"
