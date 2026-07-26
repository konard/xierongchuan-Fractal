#!/bin/sh
# Minimal stand-in for llvm-readelf, used by test.sh.
#
# The "shared libraries" of the test are text files that describe themselves,
# one property per line:
#
#   NEEDED libglib-2.0.so
#   FUNC main
#
# This prints them back in the format that `verify-apk.sh` parses.
set -eu

mode=${1:?a mode is required}
file=${2:?a file is required}

case "$mode" in
    -d)
        echo "Dynamic section at offset 0x1000 contains 2 entries:"
        while read -r kind value; do
            [ "$kind" = "NEEDED" ] || continue
            printf ' 0x0000000000000001 (NEEDED)             Shared library: [%s]\n' "$value"
        done < "$file"
        ;;
    --dyn-syms)
        echo "Symbol table '.dynsym' contains 2 entries:"
        echo "   Num:    Value          Size Type    Bind   Vis       Ndx Name"
        num=0
        while read -r kind value; do
            [ "$kind" = "FUNC" ] || continue
            num=$((num + 1))
            printf '%6d: 00000000000012a0    24 FUNC    GLOBAL DEFAULT    13 %s\n' "$num" "$value"
        done < "$file"
        ;;
    *)
        echo "fake-readelf: unsupported mode: $mode" >&2
        exit 2
        ;;
esac
