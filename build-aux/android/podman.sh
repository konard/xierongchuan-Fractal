#!/bin/sh
# Run the Android toolchain entirely in Podman. The only Android command that
# runs on the host is `adb` for installing/debugging an already built APK.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_dir=$(CDPATH= cd -- "$script_dir/../.." && pwd)
image=${FRACTAL_ANDROID_IMAGE:-localhost/fractal-android-builder:dev}
state_dir="$project_dir/.android-container"
volume_suffix=${FRACTAL_PODMAN_VOLUME_SUFFIX:-:Z}

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
    if ! podman image exists "$image"; then
        "$script_dir/podman.sh" image
    fi
}

run_container() {
    prepare_state
    podman run --rm \
        --userns=keep-id \
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
        podman build \
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
    install)
        shift
        apk=${1:-"$project_dir/.pixiewood/android/app/build/outputs/apk/debug/app-debug.apk"}
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
