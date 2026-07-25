#!/bin/sh
# Regression test for the stale Android build image.
#
# `build-aux/android/podman.sh` used to reuse any image that happened to carry
# the expected tag. A checkout that added packages to the Containerfile (such
# as `gettext`, which provides `msgfmt`) therefore kept building against an
# image made before that change, and the build only failed much later inside
# Meson with "Program 'msgfmt' not found".
#
# The image is now labelled with the checksum of the Containerfile it was built
# from and rebuilt when the checksum no longer matches. Both cases are checked
# here with a fake container engine, so the test needs neither Podman nor
# network access.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_dir=$(CDPATH= cd -- "$script_dir/../.." && pwd)
podman_sh="$project_dir/build-aux/android/podman.sh"

work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT

FAKE_ENGINE_STORE="$work_dir/image"
FAKE_ENGINE_LOG="$work_dir/log"
export FAKE_ENGINE_STORE FAKE_ENGINE_LOG
export FRACTAL_CONTAINER_ENGINE="$script_dir/fake-engine.sh"

failures=0
check() {
    if [ "$2" = "$3" ]; then
        echo "ok: $1"
    else
        echo "FAIL: $1: expected '$3', got '$2'" >&2
        failures=$((failures + 1))
    fi
}

builds() {
    grep --count '^build ' "$FAKE_ENGINE_LOG" 2> /dev/null || true
}

# 1. No image yet: it must be built.
: > "$FAKE_ENGINE_LOG"
"$podman_sh" exec -- true > /dev/null
check "missing image is built" "$(builds)" 1

# 2. Image built from the current Containerfile: no rebuild.
: > "$FAKE_ENGINE_LOG"
"$podman_sh" exec -- true > /dev/null
check "up-to-date image is reused" "$(builds)" 0

# 3. Image built from an older Containerfile: rebuild.
: > "$FAKE_ENGINE_LOG"
printf 'stale-digest\n' > "$FAKE_ENGINE_STORE"
"$podman_sh" exec -- true > /dev/null 2>&1
check "stale image is rebuilt" "$(builds)" 1

# 4. An image the user pointed at explicitly is never rebuilt behind its tag.
: > "$FAKE_ENGINE_LOG"
printf 'stale-digest\n' > "$FAKE_ENGINE_STORE"
FRACTAL_ANDROID_IMAGE=example.invalid/custom:tag "$podman_sh" exec -- true \
    > /dev/null
check "explicit image is left alone" "$(builds)" 0

[ "$failures" -eq 0 ] || exit 1
echo "All checks passed."
