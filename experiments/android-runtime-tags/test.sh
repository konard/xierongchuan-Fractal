#!/bin/sh
# Regression test for `build-aux/android/install-translations.sh`.
#
# Pixiewood assembles the Android package from `meson install --tags runtime`,
# and Meson skips every file that has no tag or a different one. Two things
# Fractal needs at runtime were dropped by that filter:
#
#   * its GSettings schema, because Meson guesses no tag for a file under
#     `datadir` -- fixed by tagging it `runtime` in `data/meson.build`, and
#     checked in the package itself by `verify-apk.sh`;
#   * its translations, because `i18n.gettext()` hard-codes the `i18n` tag --
#     fixed by installing them a second time from an install script that
#     carries the `runtime` tag.
#
# This test covers that script: it needs neither Meson nor a build, only the
# environment Meson sets for an install script.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_dir=$(CDPATH= cd -- "$script_dir/../.." && pwd)
install_translations="$project_dir/build-aux/android/install-translations.sh"

work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT

failures=0
check() {
    if [ "$2" = "$3" ]; then
        echo "ok: $1"
    else
        echo "FAIL: $1: expected '$3', got '$2'" >&2
        failures=$((failures + 1))
    fi
}

# The layout `i18n.gettext()` builds in the `po` directory of the build tree.
po_build_dir="$work_dir/build/po"
for lang in de ru sr@latin; do
    mkdir -p "$po_build_dir/$lang/LC_MESSAGES"
    printf 'catalogue of %s\n' "$lang" > "$po_build_dir/$lang/LC_MESSAGES/fractal.mo"
done
# `msgfmt` leaves the sources next to the catalogues; only the catalogues of
# this package are installed.
touch "$po_build_dir/de/LC_MESSAGES/other.mo" "$po_build_dir/de.po"

run() {
    MESON_INSTALL_DESTDIR_PREFIX="$1" MESON_INSTALL_QUIET=1 \
        sh "$install_translations" "$po_build_dir" "$2" fractal \
        > "$work_dir/install.log" 2>&1 && echo 0 || echo $?
}

# 1. A relative localedir, which is what `get_option('localedir')` gives.
destdir="$work_dir/destdir"
check "installing catalogues succeeds" "$(run "$destdir" share/locale)" 0
for lang in de ru sr@latin; do
    catalogue="$destdir/share/locale/$lang/LC_MESSAGES/fractal.mo"
    check "the $lang catalogue is installed" \
        "$([ -f "$catalogue" ] && echo yes || echo no)" yes
done
check "the catalogue keeps its contents" \
    "$(cat "$destdir/share/locale/ru/LC_MESSAGES/fractal.mo")" \
    "catalogue of ru"
check "no catalogue of another package is installed" \
    "$([ -e "$destdir/share/locale/de/LC_MESSAGES/other.mo" ] && echo yes || echo no)" \
    no

# 2. An absolute localedir must not be joined to the staging directory twice.
destdir=$work_dir/absolute
check "an absolute localedir is accepted" "$(run "$destdir" /usr/share/locale)" 0
check "an absolute localedir is staged once" \
    "$([ -f "$destdir/usr/share/locale/de/LC_MESSAGES/fractal.mo" ] && echo yes || echo no)" \
    yes

# 3. Silently installing nothing is how the previous packages lost their
#    translations, so an empty build directory is an error.
mkdir -p "$work_dir/empty"
po_build_dir="$work_dir/empty"
check "an empty build directory fails" "$(run "$work_dir/none" share/locale)" 1
check "the empty build directory is reported" \
    "$(grep -c 'no catalogue found' "$work_dir/install.log")" 1

if [ "$failures" -eq 0 ]; then
    echo "All checks passed."
else
    echo "$failures check(s) failed." >&2
    exit 1
fi
