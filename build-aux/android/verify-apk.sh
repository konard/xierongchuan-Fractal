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
#   * the package contains native libraries only for the expected ABI;
#   * the GTK/libadwaita runtime libraries are packaged;
#   * no packaged library has a DT_NEEDED entry that is missing from the
#     package and is not provided by the Android system itself.
set -eu

expected_min_sdk=${FRACTAL_ANDROID_MIN_SDK:-31}
expected_abi=${FRACTAL_ANDROID_ABI:-arm64-v8a}
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

aapt2=$(find "$sdk_root/build-tools" -maxdepth 2 -name aapt2 -type f | sort | tail -1)
readelf=$(find "${ANDROID_NDK_HOME:-$sdk_root/ndk}" -maxdepth 5 -name 'llvm-readelf' -type f | sort | tail -1)
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
    | sed -n "s/^sdkVersion:'\([0-9]*\)'.*/\1/p" | head -1)
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

abis=$(unzip -Z1 "$apk" 'lib/*' 2>/dev/null | cut -d/ -f2 | sort -u)
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
