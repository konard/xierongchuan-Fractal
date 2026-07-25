#!/bin/sh
# Run the Android toolchain entirely in a container. The only Android command
# that runs on the host is `adb` for installing/debugging an already built APK.
#
# Podman is the supported engine; Docker is accepted as a fallback so the
# toolchain can also be used on hosts that only have Docker available. Set
# FRACTAL_CONTAINER_ENGINE to force one of them.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_dir=$(CDPATH= cd -- "$script_dir/../.." && pwd)
image=${FRACTAL_ANDROID_IMAGE:-localhost/fractal-android-builder:dev}
state_dir="$project_dir/.android-container"
# Pixiewood splits the debug package per ABI, so the default APK is the
# arm64-v8a one rather than a plain app-debug.apk.
apk_relative_path=${FRACTAL_ANDROID_APK:-.pixiewood/android/app/build/outputs/apk/debug/app-arm64-v8a-debug.apk}

engine=${FRACTAL_CONTAINER_ENGINE:-}
if [ -z "$engine" ]; then
    if command -v podman > /dev/null 2>&1; then
        engine=podman
    elif command -v docker > /dev/null 2>&1; then
        engine=docker
    else
        echo "Neither podman nor docker was found on the host." >&2
        exit 127
    fi
fi
command -v "$engine" > /dev/null 2>&1 || {
    echo "Container engine not found: $engine" >&2
    exit 127
}

# Only Podman relabels volumes for SELinux, and only Podman maps the host user
# into the container namespace with --userns=keep-id.
case "$engine" in
    *podman*)
        volume_suffix=${FRACTAL_PODMAN_VOLUME_SUFFIX:-:Z}
        userns_args="--userns=keep-id"
        ;;
    *)
        volume_suffix=${FRACTAL_PODMAN_VOLUME_SUFFIX:-}
        userns_args=""
        ;;
esac

usage() {
    cat <<'EOF'
Usage: build-aux/android/podman.sh <command> [arguments]

Commands:
  image                 Build the local Android toolchain image.
  shell                 Open a shell in the isolated build environment.
  exec -- COMMAND ...   Run an arbitrary command in the build environment.
  prepare MANIFEST      Run `pixiewood prepare` for an Android manifest.
  generate              Run `pixiewood generate` after prepare.
  build [--release]     Run `pixiewood build` after generate.
  app                   Build the Fractal APK end to end (android/pixiewood.xml).
  smoke                 Build the GTK/libadwaita smoke-test APK end to end.
  verify [APK]          Run static checks on a built APK.
  install [APK]         Install the debug APK with the host's adb.
  logcat [ARGS ...]     Run the host's adb logcat.

The image is built automatically for commands that need it. Generated files
and caches stay in .pixiewood/ and .android-container/ in the checkout.
EOF
}

prepare_state() {
    mkdir -p "$state_dir/cargo" "$state_dir/gradle" "$state_dir/home"
}

ensure_image() {
    if ! "$engine" image inspect "$image" > /dev/null 2>&1; then
        "$script_dir/podman.sh" image
    fi
}

run_container() {
    prepare_state
    # shellcheck disable=SC2086 # userns_args is intentionally word-split.
    "$engine" run --rm \
        $userns_args \
        --user "$(id -u):$(id -g)" \
        --env HOME=/workspace/.android-container/home \
        --env CARGO_HOME=/workspace/.android-container/cargo \
        --env GRADLE_USER_HOME=/workspace/.android-container/gradle \
        --volume "$project_dir:/workspace${volume_suffix}" \
        --workdir /workspace \
        "$image" "$@"
}

command=${1:-}
case "$command" in
    image)
        "$engine" build \
            --file "$script_dir/Containerfile" \
            --tag "$image" \
            "$project_dir"
        ;;
    shell)
        ensure_image
        run_container bash
        ;;
    exec)
        shift
        if [ "${1:-}" = "--" ]; then
            shift
        fi
        if [ "$#" -eq 0 ]; then
            usage >&2
            exit 2
        fi
        ensure_image
        run_container "$@"
        ;;
    prepare)
        shift
        if [ "$#" -ne 1 ]; then
            usage >&2
            exit 2
        fi
        ensure_image
        run_container pixiewood -C /workspace prepare "$1"
        ;;
    generate)
        ensure_image
        run_container pixiewood -C /workspace generate
        ;;
    build)
        shift
        ensure_image
        if [ "${1:-}" = "--release" ]; then
            # The release/debug mode is selected during `prepare`, because
            # Pixiewood writes it into .pixiewood/pixiewood.ini.
            echo "Run 'prepare --release MANIFEST' before 'build'; --release is not valid here." >&2
            exit 2
        fi
        if [ "$#" -ne 0 ]; then
            usage >&2
            exit 2
        fi
        run_container pixiewood -C /workspace build
        ;;
    app)
        shift
        if [ "$#" -ne 0 ]; then
            usage >&2
            exit 2
        fi
        ensure_image
        run_container pixiewood -C /workspace prepare android/pixiewood.xml
        run_container pixiewood -C /workspace generate
        run_container pixiewood -C /workspace build
        run_container build-aux/android/verify-apk.sh "$apk_relative_path"
        echo "Fractal APK: $apk_relative_path"
        ;;
    smoke)
        shift
        if [ "$#" -ne 0 ]; then
            usage >&2
            exit 2
        fi
        ensure_image
        smoke_dir=experiments/android-gtk-smoke
        run_container pixiewood -C "/workspace/$smoke_dir" prepare \
            "$smoke_dir/pixiewood.xml"
        run_container pixiewood -C "/workspace/$smoke_dir" generate
        run_container pixiewood -C "/workspace/$smoke_dir" build
        run_container build-aux/android/verify-apk.sh \
            "$smoke_dir/$apk_relative_path"
        echo "Smoke test APK: $smoke_dir/$apk_relative_path"
        ;;
    verify)
        shift
        apk=${1:-"$project_dir/$apk_relative_path"}
        if [ "$#" -gt 1 ] || [ ! -f "$apk" ]; then
            echo "APK not found: $apk" >&2
            exit 2
        fi
        ensure_image
        run_container build-aux/android/verify-apk.sh \
            "${apk#"$project_dir/"}"
        ;;
    install)
        shift
        apk=${1:-"$project_dir/$apk_relative_path"}
        if [ "$#" -gt 1 ] || [ ! -f "$apk" ]; then
            echo "APK not found: $apk" >&2
            exit 2
        fi
        command -v adb > /dev/null || {
            echo "adb must be installed on the host to install the APK." >&2
            exit 127
        }
        adb install -r "$apk"
        ;;
    logcat)
        shift
        command -v adb > /dev/null || {
            echo "adb must be installed on the host to read device logs." >&2
            exit 127
        }
        adb logcat "$@"
        ;;
    -h|--help|help|'')
        usage
        ;;
    *)
        usage >&2
        exit 2
        ;;
esac
