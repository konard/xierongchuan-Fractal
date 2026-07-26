#!/bin/sh
# Static checks for an Android package built by Pixiewood.
#
# It is meant to run inside the Android toolchain image, where the Android SDK
# build-tools and the NDK are available:
#
#   build-aux/android/podman.sh verify [APK]
#
# Checked properties:
#   * the manifest declares the expected minSdkVersion;
#   * the package requests the INTERNET permission;*
#   * the package contains native libraries only for the expected ABI;
#   * the library that the GTK runtime loads on startup, named by the
#     `gtk.android.lib_name` meta-data of the manifest, is packaged and exports
#     the `main` symbol that the runtime looks up in it;
#   * the GTK/libadwaita runtime libraries are packaged;
#   * the compiled GSettings schemas are among the assets, and contain the
#     schema of the application;* the translations are among them too;*
#   * no packaged library has a DT_NEEDED entry that is missing from the
#     package and is not provided by the Android system itself.
#
# The properties marked with * are about Fractal rather than the toolchain, and
# are skipped with FRACTAL_ANDROID_APP_CHECKS=0, which is how the package of
# the toolchain smoke test is checked.
set -eu

expected_min_sdk=${FRACTAL_ANDROID_MIN_SDK:-31}
expected_abi=${FRACTAL_ANDROID_ABI:-arm64-v8a}
gettext_package=${FRACTAL_GETTEXT_PACKAGE:-fractal}
# The toolchain smoke test packages a minimal GTK application instead of
# Fractal. It has no settings schema of its own, no translations and no reason
# to reach the network, so the checks that are about Fractal itself are turned
# off for it. Everything about the toolchain is still checked.
app_checks=${FRACTAL_ANDROID_APP_CHECKS:-1}
# Libraries that Android itself provides to applications (NDK stable ABI).
system_libs="libc.so libm.so libdl.so liblog.so libz.so libandroid.so \
libEGL.so libGLESv1_CM.so libGLESv2.so libGLESv3.so libOpenSLES.so \
libOpenMAXAL.so libjnigraphics.so libnativewindow.so libvulkan.so \
libaaudio.so libamidi.so libcamera2ndk.so libmediandk.so libneuralnetworks.so \
libc++_shared.so ld-android.so libstdc++.so"

apk=${1:-}
if [ -z "$apk" ] || [ ! -f "$apk" ]; then
    echo "Usage: $0 <apk>" >&2
    exit 2
fi

sdk_root=${ANDROID_SDK_ROOT:-${ANDROID_HOME:-}}
[ -n "$sdk_root" ] || {
    echo "ANDROID_SDK_ROOT is not set; run this inside the Android image." >&2
    exit 127
}

aapt2=$(find "$sdk_root/build-tools" -maxdepth 2 -name aapt2 | sort | tail -1)
readelf=$(find "${ANDROID_NDK_HOME:-$sdk_root/ndk}" -maxdepth 8 -name 'llvm-readelf' | sort | tail -1)
[ -x "$aapt2" ] || { echo "aapt2 not found in $sdk_root/build-tools" >&2; exit 127; }
[ -x "$readelf" ] || { echo "llvm-readelf not found in the NDK" >&2; exit 127; }

workdir=$(mktemp -d)
trap 'rm -rf "$workdir"' EXIT

failures=0
fail() {
    echo "FAIL: $1" >&2
    failures=$((failures + 1))
}
pass() {
    echo "ok: $1"
}

badging=$("$aapt2" dump badging "$apk")

min_sdk=$(printf '%s\n' "$badging" \
    | sed -n "s/^minSdkVersion:'\([0-9]*\)'.*/\1/p" | head -1)
if [ "$min_sdk" = "$expected_min_sdk" ]; then
    pass "minSdkVersion is $min_sdk"
else
    fail "minSdkVersion is '${min_sdk:-unset}', expected $expected_min_sdk"
fi

target_sdk=$(printf '%s\n' "$badging" \
    | sed -n "s/^targetSdkVersion:'\([0-9]*\)'.*/\1/p" | head -1)
echo "info: targetSdkVersion is ${target_sdk:-unset}"

launchable=$(printf '%s\n' "$badging" | sed -n "s/^launchable-activity: name='\([^']*\)'.*/\1/p" | head -1)
if [ -n "$launchable" ]; then
    pass "launcher activity is $launchable"
else
    fail "the package has no launcher activity"
fi

# Pixiewood derives the permissions from the metainfo file, so a Matrix client
# that cannot reach the network is a packaging error rather than a runtime one.
if [ "$app_checks" = 1 ]; then
    if printf '%s\n' "$badging" \
            | grep -q "^uses-permission: name='android.permission.INTERNET'"; then
        pass "the package requests the INTERNET permission"
    else
        fail "the package does not request the INTERNET permission"
    fi
fi

abis=$(unzip -Z1 "$apk" 'lib/*/*.so' 2>/dev/null | cut -d/ -f2 | sort -u)
if [ "$abis" = "$expected_abi" ]; then
    pass "native libraries are limited to $expected_abi"
else
    fail "unexpected ABI set: '$(echo "$abis" | tr '\n' ' ')', expected $expected_abi"
fi

packaged_libs=$(unzip -Z1 "$apk" "lib/$expected_abi/*.so" 2>/dev/null | xargs -r -n1 basename | sort -u)
if [ -z "$packaged_libs" ]; then
    fail "the package contains no native library for $expected_abi"
    exit 1
fi
echo "info: $(printf '%s\n' "$packaged_libs" | wc -l) native libraries packaged"

for lib in libgtk-4.so libadwaita-1.so libgio-2.0.so libglib-2.0.so \
        libpango-1.0.so libcairo.so libgdk_pixbuf-2.0.so; do
    if printf '%s\n' "$packaged_libs" | grep -qx "$lib"; then
        pass "runtime library $lib is packaged"
    else
        fail "runtime library $lib is missing from the package"
    fi
done

unzip -q -o "$apk" "lib/$expected_abi/*.so" -d "$workdir"

# The GTK runtime does not start an executable: on startup it loads the library
# named by the `gtk.android.lib_name` meta-data of the manifest and calls the
# `main` symbol it exports. When that library is missing from the package, or
# does not export `main`, the application dies with an `UnsatisfiedLinkError`
# before showing a window, so both are checked here.
app_lib_name=$("$aapt2" dump xmltree --file AndroidManifest.xml "$apk" | awk '
    /:name\(/ && /"gtk\.android\.lib_name"/ { found = 1; next }
    found && /:value\(/ && match($0, /="[^"]*"/) {
        print substr($0, RSTART + 2, RLENGTH - 3)
        exit
    }
')
if [ -z "$app_lib_name" ]; then
    fail "the manifest declares no gtk.android.lib_name meta-data"
else
    app_lib="lib$app_lib_name.so"
    if printf '%s\n' "$packaged_libs" | grep -qx "$app_lib"; then
        pass "application library $app_lib is packaged"
        if "$readelf" --dyn-syms "$workdir/lib/$expected_abi/$app_lib" \
                | awk '$4 == "FUNC" && $8 == "main" { found = 1 } END { exit !found }'; then
            pass "$app_lib exports main"
        else
            fail "$app_lib does not export the main symbol"
        fi
    else
        fail "application library $app_lib is missing from the package"
    fi
fi

# Everything that is not a native library is shipped as an asset and unpacked
# into the data directory of the application on first start. GSettings schemas
# are the part of it the application cannot start without: `g_settings_new()`
# aborts the process when the schema of the application is not installed, which
# on Android is a `SIGABRT` with no window ever appearing.
schemas_asset=assets/share/glib-2.0/schemas/gschemas.compiled
schemas=$workdir/gschemas.compiled
if unzip -p "$apk" "$schemas_asset" > "$schemas" 2> /dev/null && [ -s "$schemas" ]; then
    pass "compiled GSettings schemas are packaged"
    package_name=$(printf '%s\n' "$badging" \
        | sed -n "s/^package: name='\([^']*\)'.*/\1/p" | head -1)
    if [ "$app_checks" != 1 ]; then
        :
    elif [ -z "$package_name" ]; then
        fail "the manifest declares no package name"
    elif grep -qa -- "$package_name" "$schemas"; then
        pass "the schema of $package_name is compiled in"
    else
        fail "the schemas contain no schema for $package_name"
    fi
else
    fail "$schemas_asset is missing from the package"
fi

# The translations are shipped the same way, and were dropped by the same
# filter: `i18n.gettext()` tags them `i18n`, so `meson install --tags runtime`
# left the package English-only.
if [ "$app_checks" = 1 ]; then
    catalogues=$(unzip -Z1 "$apk" \
        "assets/share/locale/*/LC_MESSAGES/$gettext_package.mo" 2> /dev/null \
        | wc -l)
    if [ "$catalogues" -gt 0 ]; then
        pass "$catalogues translations are packaged"
    else
        fail "no translation of $gettext_package is packaged"
    fi
fi

missing_deps=""
for so in "$workdir/lib/$expected_abi"/*.so; do
    for needed in $("$readelf" -d "$so" \
            | sed -n 's/.*(NEEDED).*\[\(.*\)\]/\1/p'); do
        printf '%s\n' "$packaged_libs" | grep -qx "$needed" && continue
        case " $system_libs " in
            *" $needed "*) continue ;;
        esac
        missing_deps="$missing_deps $(basename "$so")->$needed"
    done
done
if [ -z "$missing_deps" ]; then
    pass "every DT_NEEDED entry is packaged or provided by Android"
else
    fail "unresolved shared libraries:$missing_deps"
fi

if [ "$failures" -eq 0 ]; then
    echo "APK verification passed: $apk"
else
    echo "APK verification failed with $failures error(s): $apk" >&2
    exit 1
fi
