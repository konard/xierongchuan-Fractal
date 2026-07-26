#!/bin/sh
# Install the compiled translations with the `runtime` install tag.
#
# Meson's `i18n.gettext()` hard-codes the `i18n` tag for every catalogue it
# installs. Pixiewood assembles the Android package from
# `meson install --tags runtime`, so those catalogues never reach it. This
# script is registered by `po/meson.build` as an install script tagged
# `runtime`, and copies the catalogues that Meson already built.
#
#   install-translations.sh <po build dir> <localedir> <gettext package>
#
# `i18n.gettext()` builds each catalogue as `<po build dir>/<lang>/LC_MESSAGES/
# <package>.mo`, which is the layout `bindtextdomain()` expects, so the tree is
# copied as it is.
set -eu

po_build_dir=$1
localedir=$2
gettext_package=$3

# Meson exports the staging directory; `MESON_INSTALL_DESTDIR_PREFIX` already
# has DESTDIR and the prefix in it, and `localedir` is relative to the prefix
# unless it was given as an absolute path.
destdir=${MESON_INSTALL_DESTDIR_PREFIX:?not run from meson install}
case "$localedir" in
    /*) target=$destdir$localedir ;;
    *) target=$destdir/$localedir ;;
esac

installed=0
for catalogue in "$po_build_dir"/*/LC_MESSAGES/"$gettext_package.mo"; do
    [ -f "$catalogue" ] || continue

    lang_dir=$(dirname "$(dirname "$catalogue")")
    dest="$target/$(basename "$lang_dir")/LC_MESSAGES"
    mkdir -p "$dest"
    cp -p "$catalogue" "$dest/$gettext_package.mo"
    installed=$((installed + 1))
done

if [ "$installed" -eq 0 ]; then
    echo "install-translations.sh: no catalogue found in $po_build_dir" >&2
    exit 1
fi

if [ "${MESON_INSTALL_QUIET:-0}" != "1" ]; then
    echo "Installing $installed translations to $target"
fi
