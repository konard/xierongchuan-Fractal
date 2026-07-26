#!/bin/sh
# Minimal stand-in for aapt2, used by test.sh. It serves the two dumps that
# `verify-apk.sh` asks for from fixture files, so the test needs neither the
# Android SDK nor a real package.
set -eu

case "${1:-} ${2:-}" in
    "dump badging")
        cat "${FAKE_AAPT2_BADGING:?FAKE_AAPT2_BADGING must be set}"
        ;;
    "dump xmltree")
        cat "${FAKE_AAPT2_XMLTREE:?FAKE_AAPT2_XMLTREE must be set}"
        ;;
    *)
        echo "fake-aapt2: unsupported command: $*" >&2
        exit 2
        ;;
esac
