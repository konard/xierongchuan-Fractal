#!/bin/sh
# Regression test for the application library of the Android package.
#
# The GTK Android runtime does not start an executable. `RuntimeApplication`
# loads the library named by the `gtk.android.lib_name` meta-data of the
# manifest -- Pixiewood sets it to the name of the Meson target it builds --
# and calls the `main` symbol it exports.
#
# Fractal installed that target into `bindir`, as a desktop binary would be,
# but Pixiewood only packages `libdir` (it becomes the `jniLibs` directory of
# the generated Android project) and explicitly skips `bindir`. The package
# therefore built, installed and passed verification without containing the
# application at all, and every launch died immediately with:
#
#   java.lang.UnsatisfiedLinkError: Unable to open library "libfractal.so":
#   dlopen failed: library "libfractal.so" not found
#
# The target now installs into `libdir`, and `verify-apk.sh` fails when the
# library named by the manifest is missing from the package or does not export
# `main`. That guard is what this test exercises, with fake SDK tools and
# hand-made packages, so it needs neither the Android SDK nor a build.
#
# The same test covers the guard for the other half of the installation: the
# compiled GSettings schemas, which are shipped as an asset. Without the schema
# of the application, `g_settings_new()` aborts the process just as silently.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_dir=$(CDPATH= cd -- "$script_dir/../.." && pwd)
verify_apk="$project_dir/build-aux/android/verify-apk.sh"

work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT

abi=arm64-v8a

# A fake SDK holding the two tools that `verify-apk.sh` looks up.
sdk="$work_dir/sdk"
mkdir -p "$sdk/build-tools/36.0.0" "$sdk/ndk/bin"
cp "$script_dir/fake-aapt2.sh" "$sdk/build-tools/36.0.0/aapt2"
cp "$script_dir/fake-readelf.sh" "$sdk/ndk/bin/llvm-readelf"
chmod +x "$sdk/build-tools/36.0.0/aapt2" "$sdk/ndk/bin/llvm-readelf"
ANDROID_SDK_ROOT="$sdk"
ANDROID_NDK_HOME="$sdk/ndk"
export ANDROID_SDK_ROOT ANDROID_NDK_HOME

badging="$work_dir/badging.txt"
badging_without_internet="$work_dir/badging-without-internet.txt"
cat > "$badging" <<'EOF'
package: name='org.gnome.Fractal' versionCode='140100000' versionName='14.1'
minSdkVersion:'31'
targetSdkVersion:'36'
uses-permission: name='android.permission.INTERNET'
launchable-activity: name='org.gtk.android.ToplevelActivity'  label='' icon=''
EOF
grep -v 'android.permission.INTERNET' "$badging" > "$badging_without_internet"

# The shape of `aapt2 dump xmltree --file AndroidManifest.xml`, cut down to the
# meta-data element that Pixiewood generates from the Meson target name.
xmltree="$work_dir/xmltree.txt"
xmltree_without_lib_name="$work_dir/xmltree-without-lib-name.txt"
cat > "$xmltree" <<'EOF'
N: android=http://schemas.android.com/apk/res/android
  E: manifest (line=2)
    A: android:versionCode(0x0101021b)=(type 0x10)0x85a4d20
    E: application (line=3)
      A: android:label(0x01010001)=@0x7f0d0000
      E: meta-data (line=4)
        A: android:name(0x01010003)="gtk.android.lib_name" (Raw: "gtk.android.lib_name")
        A: android:value(0x01010024)="fractal" (Raw: "fractal")
EOF
sed '/meta-data/,$d' "$xmltree" > "$xmltree_without_lib_name"

failures=0
check() {
    if [ "$2" = "$3" ]; then
        echo "ok: $1"
    else
        echo "FAIL: $1: expected '$3', got '$2'" >&2
        failures=$((failures + 1))
    fi
}

check_output() {
    if grep -qF "$2" "$log"; then
        echo "ok: $1"
    else
        echo "FAIL: $1: '$2' is not in the output" >&2
        failures=$((failures + 1))
    fi
}

# The libraries that `verify-apk.sh` expects to find in any Fractal package.
runtime_libs='libgtk-4.so libadwaita-1.so libgio-2.0.so libglib-2.0.so
libpango-1.0.so libcairo.so libgdk_pixbuf-2.0.so libfractal_android.so'

# Build a package. The first argument describes the application library:
# `packaged` (installed into `libdir`), `missing` (installed into `bindir`, so
# dropped by Pixiewood) or `no-main` (packaged, but not an application). The
# second describes the compiled GSettings schemas asset: `present`, `foreign`
# (compiled, but without the schema of the application) or `missing`. The third
# describes the translation catalogues: `present` or `missing`.
make_apk() {
    stage="$work_dir/stage"
    apk="$work_dir/app.apk"
    rm -rf "$stage" "$apk"
    mkdir -p "$stage/lib/$abi" "$stage/assets/share/glib-2.0/schemas"

    for lib in $runtime_libs; do
        printf 'NEEDED libc.so\n' > "$stage/lib/$abi/$lib"
    done

    case "$1" in
        packaged)
            printf 'NEEDED libfractal_android.so\nFUNC main\n' \
                > "$stage/lib/$abi/libfractal.so"
            ;;
        no-main)
            printf 'NEEDED libfractal_android.so\n' \
                > "$stage/lib/$abi/libfractal.so"
            ;;
        missing) ;;
        *) echo "unknown library case: $1" >&2; exit 2 ;;
    esac

    case "$3" in
        present)
            for lang in de ru; do
                mkdir -p "$stage/assets/share/locale/$lang/LC_MESSAGES"
                printf 'catalogue\n' \
                    > "$stage/assets/share/locale/$lang/LC_MESSAGES/fractal.mo"
            done
            ;;
        missing) ;;
        *) echo "unknown translations case: $3" >&2; exit 2 ;;
    esac

    schemas="$stage/assets/share/glib-2.0/schemas/gschemas.compiled"
    case "$2" in
        present)
            printf 'GVariant\0org.gtk.Settings.FileChooser\0org.gnome.Fractal\0' \
                > "$schemas"
            ;;
        foreign)
            printf 'GVariant\0org.gtk.Settings.FileChooser\0' > "$schemas"
            ;;
        missing) rm -f "$schemas" ;;
        *) echo "unknown schemas case: $2" >&2; exit 2 ;;
    esac

    (cd "$stage" && zip -q -r "$apk" lib assets)
}

log="$work_dir/verify.log"
# `$app_checks` is what `podman.sh` sets for the toolchain smoke test, whose
# package is not Fractal; the individual cases below override it.
app_checks=1
verify() {
    make_apk "$1" "${3:-present}" "${4:-present}"
    FAKE_AAPT2_XMLTREE="${2:-$xmltree}" FAKE_AAPT2_BADGING="${5:-$badging}" \
        FRACTAL_ANDROID_APP_CHECKS="$app_checks" \
        sh "$verify_apk" "$apk" > "$log" 2>&1 && echo 0 || echo $?
}

# 1. The application library is packaged next to the runtime libraries.
check "a complete package passes" "$(verify packaged)" 0
check_output "the application library is reported" \
    "ok: application library libfractal.so is packaged"
check_output "the entry point is reported" "ok: libfractal.so exports main"

# 2. The application library was installed into `bindir`: it never reaches the
#    package, and the application dies on startup.
check "a package without the application library fails" "$(verify missing)" 1
check_output "the missing application library is reported" \
    "FAIL: application library libfractal.so is missing from the package"

# 3. The library is packaged, but the runtime cannot find its entry point.
check "a package without the main symbol fails" "$(verify no-main)" 1
check_output "the missing entry point is reported" \
    "FAIL: libfractal.so does not export the main symbol"

# 4. Without the meta-data, the runtime does not know what to load at all.
check "a manifest without the meta-data fails" \
    "$(verify packaged "$xmltree_without_lib_name")" 1
check_output "the missing meta-data is reported" \
    "FAIL: the manifest declares no gtk.android.lib_name meta-data"

# 5. The complete package also reports its schemas.
check "a complete package still passes" "$(verify packaged "$xmltree" present)" 0
check_output "the schemas are reported" \
    "ok: compiled GSettings schemas are packaged"
check_output "the schema of the application is reported" \
    "ok: the schema of org.gnome.Fractal is compiled in"

# 6. The schemas are compiled, but not the one the application reads: GLib
#    aborts on the first `g_settings_new()`.
check "a package without the schema of the application fails" \
    "$(verify packaged "$xmltree" foreign)" 1
check_output "the missing schema is reported" \
    "FAIL: the schemas contain no schema for org.gnome.Fractal"

# 7. The schemas are not packaged at all.
check "a package without compiled schemas fails" \
    "$(verify packaged "$xmltree" missing)" 1
check_output "the missing schemas asset is reported" \
    "FAIL: assets/share/glib-2.0/schemas/gschemas.compiled is missing from the package"

# 8. The translations were dropped by the same install tag filter as the
#    schema, and an English-only package is not a working translated one.
check "a package without translations fails" \
    "$(verify packaged "$xmltree" present missing)" 1
check_output "the missing translations are reported" \
    "FAIL: no translation of fractal is packaged"

# 9. Pixiewood derives the permissions from the metainfo file. A Matrix client
#    without the INTERNET permission starts and then cannot do anything.
check "a package without the INTERNET permission fails" \
    "$(verify packaged "$xmltree" present present "$badging_without_internet")" 1
check_output "the missing permission is reported" \
    "FAIL: the package does not request the INTERNET permission"

# 10. The same script checks the package of the toolchain smoke test, which is
#     a minimal GTK application: no schema of its own, no translations, no
#     network. Those three checks are turned off for it, and everything about
#     the toolchain -- including the schemas that GTK itself installs -- is
#     still checked.
app_checks=0
check "the smoke test package passes without the Fractal properties" \
    "$(verify packaged "$xmltree" foreign missing "$badging_without_internet")" 0
check_output "the schemas of the toolchain are still checked" \
    "ok: compiled GSettings schemas are packaged"
check "the smoke test package still needs its application library" \
    "$(verify missing "$xmltree" foreign missing "$badging_without_internet")" 1
check_output "the missing application library is still reported" \
    "FAIL: application library libfractal.so is missing from the package"
check "the smoke test package still needs compiled schemas" \
    "$(verify packaged "$xmltree" missing missing "$badging_without_internet")" 1
app_checks=1

if [ "$failures" -eq 0 ]; then
    echo "All checks passed."
else
    echo "$failures check(s) failed." >&2
    exit 1
fi
