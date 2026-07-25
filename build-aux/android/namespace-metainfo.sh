#!/bin/sh
# Copy a metainfo file, adding the XML namespace that Pixiewood expects.
#
# The desktop metainfo file has no namespace, which is the usual AppStream
# style, but Pixiewood only reads elements in the metainfo namespace when it
# generates the Android manifest. Rather than changing the file the desktop
# packages install, the Android build generates a namespaced copy.
#
# The language tags are converted at the same time. Pixiewood derives the
# Android resource directories from them, and Android only accepts BCP 47, so
# the script modifiers that gettext uses have to become script subtags.
set -eu

if [ "$#" -ne 2 ]; then
    echo "Usage: $0 INPUT OUTPUT" >&2
    exit 2
fi

sed -e 's|<component |<component xmlns="https://specifications.freedesktop.org/metainfo/1.0" |' \
    -e 's|\(xml:lang="[^"]*\)@latin"|\1-Latn"|g' \
    -e 's|\(xml:lang="[^"]*\)@cyrillic"|\1-Cyrl"|g' \
    "$1" > "$2"

remaining_modifier=$(grep -o 'xml:lang="[^"]*@[^"]*"' "$2" || true)
if [ -n "$remaining_modifier" ]; then
    echo "$0: unhandled locale modifier in $1: $remaining_modifier" >&2
    exit 1
fi

grep -q 'specifications.freedesktop.org/metainfo/1.0' "$2" || {
    echo "$0: no <component> element found in $1" >&2
    exit 1
}
