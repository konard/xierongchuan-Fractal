#!/bin/sh
# Copy a metainfo file, adding the XML namespace that Pixiewood expects.
#
# The desktop metainfo file has no namespace, which is the usual AppStream
# style, but Pixiewood only reads elements in the metainfo namespace when it
# generates the Android manifest. Rather than changing the file the desktop
# packages install, the Android build generates a namespaced copy.
set -eu

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 INPUT OUTPUT" >&2
    exit 2
fi

sed 's|<component |<component xmlns="https://specifications.freedesktop.org/metainfo/1.0" |' \
    "$1" > "$2"

grep -q 'specifications.freedesktop.org/metainfo/1.0' "$2" || {
    echo "$0: no <component> element found in $1" >&2
    exit 1
}
