#!/bin/sh
# Minimal stand-in for podman/docker, used by test.sh. The "image store" is a
# single file holding the label that the image was built with; a missing file
# means the image does not exist.
set -eu

store=${FAKE_ENGINE_STORE:?FAKE_ENGINE_STORE must be set}
log=${FAKE_ENGINE_LOG:?FAKE_ENGINE_LOG must be set}

echo "$*" >> "$log"

case "${1:-}" in
    image)
        # `image inspect [--format FMT] IMAGE`
        [ -f "$store" ] || exit 1
        if [ "${3:-}" = "--format" ]; then
            cat "$store"
        fi
        ;;
    build)
        # Only the value of --label is recorded.
        while [ "$#" -gt 0 ]; do
            if [ "$1" = "--label" ]; then
                printf '%s\n' "${2#*=}" > "$store"
            fi
            shift
        done
        ;;
    run)
        ;;
    *)
        echo "fake-engine: unsupported command: $*" >&2
        exit 2
        ;;
esac
