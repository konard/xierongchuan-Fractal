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
#
# The relabelling is `:z`, not `:Z`. `:Z` gives the whole checkout an MCS
# category private to a single container, so the source tree stops being
# readable for everything else on the host that SELinux confines -- other
# containers, Flatpak applications, the file manager -- until something
# relabels it back. On an enforcing system that is a stream of denials about
# your own working directory. `:z` labels the content `container_file_t`
# without a private category, which is what a shared bind mount of a checkout
# needs. Set FRACTAL_PODMAN_VOLUME_SUFFIX to override, for example to `:Z` for
# an isolated checkout or to an empty value to skip relabelling entirely.
selinux_enabled() {
    if command -v selinuxenabled > /dev/null 2>&1; then
        selinuxenabled
    else
        [ -r /sys/fs/selinux/enforce ]
    fi
}

case "$engine" in
    *podman*)
        if [ -n "${FRACTAL_PODMAN_VOLUME_SUFFIX+set}" ]; then
            volume_suffix=$FRACTAL_PODMAN_VOLUME_SUFFIX
        elif selinux_enabled; then
            volume_suffix=:z
        else
            volume_suffix=''
        fi
        userns_args="--userns=keep-id"
        ;;
    *)
        volume_suffix=${FRACTAL_PODMAN_VOLUME_SUFFIX:-}
        userns_args=""
        ;;
esac

# The image is tagged with the checksum of the recipe it was built from, so a
# checkout that adds a package to the Containerfile does not silently keep
# using an image built before it. Without this, `ensure_image` reuses any
# image that merely has the right tag and the build fails much later with a
# missing tool (for example `Program 'msgfmt' not found`).
containerfile_label=org.gnome.Fractal.containerfile-sha256

containerfile_digest() {
    if command -v sha256sum > /dev/null 2>&1; then
        sha256sum "$script_dir/Containerfile" | cut -d' ' -f1
    elif command -v shasum > /dev/null 2>&1; then
        shasum -a 256 "$script_dir/Containerfile" | cut -d' ' -f1
    else
        echo unknown
    fi
}

image_digest() {
    "$engine" image inspect \
        --format "{{index .Config.Labels \"$containerfile_label\"}}" \
        "$image" 2> /dev/null || true
}

online_cpus() {
    if command -v nproc > /dev/null 2>&1; then
        nproc
    else
        getconf _NPROCESSORS_ONLN 2> /dev/null || echo 1
    fi
}

# Cargo runs one `rustc` per job, and the heaviest crates of Fractal
# (`ruma-*`, `matrix-sdk-*` and Fractal itself, all built with `opt-level = 3`)
# need on the order of two gigabytes each. Cargo's default of one job per CPU
# therefore asks for far more memory than an ordinary machine has as soon as it
# has more cores than it has spare gigabytes, and the out-of-memory killer ends
# the build somewhere in the middle of the Rust compilation.
bytes_per_cargo_job=2147483648
default_cargo_jobs() {
    cpus=$(online_cpus)
    case "$cpus" in
        '' | *[!0-9]*) cpus=1 ;;
    esac
    [ "$cpus" -ge 1 ] || cpus=1

    by_memory=''
    if [ -r /proc/meminfo ]; then
        by_memory=$(
            awk -v per_job="$bytes_per_cargo_job" \
                '/^MemTotal:/ { printf "%d\n", $2 * 1024 / per_job; exit }' \
                /proc/meminfo
        )
    fi
    case "$by_memory" in
        '' | *[!0-9]*)
            echo "$cpus"
            return
            ;;
    esac
    [ "$by_memory" -ge 1 ] || by_memory=1

    if [ "$by_memory" -lt "$cpus" ]; then
        echo "$by_memory"
    else
        echo "$cpus"
    fi
}

cargo_jobs=${CARGO_BUILD_JOBS:-$(default_cargo_jobs)}

disk_available_gib() {
    df -Pk "$1" 2> /dev/null | awk 'NR == 2 { printf "%d\n", $4 / 1048576 }'
}

# Measured on a finished `app` build: about 8 GiB in the checkout (.pixiewood,
# .android-container and the subproject sources) and about 5 GiB for the
# toolchain image, rounded up for the headroom the packaging step needs.
required_disk_gib=20

# Both ways this build fails on a workstation -- out of memory during the Rust
# compilation and out of disk while packaging -- are far easier to recognise
# before the fact than from the wreckage afterwards, and the build is long
# enough that saying so up front is worth the two lines.
report_build_resources() {
    echo "Building with CARGO_BUILD_JOBS=$cargo_jobs on $(online_cpus) CPU(s)." \
        "Set CARGO_BUILD_JOBS to override." >&2
    available=$(disk_available_gib "$project_dir")
    case "$available" in
        '' | *[!0-9]*) return ;;
    esac
    if [ "$available" -lt "$required_disk_gib" ]; then
        echo "Warning: only ${available} GiB are free on the filesystem of" \
            "$project_dir, and a full Android build needs about" \
            "${required_disk_gib} GiB in .pixiewood/ and .android-container/." >&2
    fi
}

# The package is written inside the bind-mounted checkout, so it is on the host
# as soon as the build ends. Reporting where it is, with its size, is how the
# build says so: a relative path alone leaves it unclear whether the file
# really made it out of the container.
report_apk() {
    label=$1
    apk="$project_dir/$2"
    if [ ! -f "$apk" ]; then
        echo "$label was not produced: no file at $apk" >&2
        exit 1
    fi
    size=$(du -h "$apk" 2> /dev/null | cut -f1)
    echo "$label: $apk${size:+ ($size)}"
    echo "Install it on a connected device with:" \
        "build-aux/android/podman.sh install $2"
}

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
    mkdir -p "$state_dir/cargo" "$state_dir/gradle/init.d" "$state_dir/home"
    # Gradle applies every script in this directory to every build it runs with
    # this GRADLE_USER_HOME. It is the only place the setting can go: the
    # Gradle project itself is generated by Pixiewood. See the file.
    cp "$script_dir/gradle-init.gradle" "$state_dir/gradle/init.d/fractal-ndk.gradle"
}

ensure_image() {
    if ! "$engine" image inspect "$image" > /dev/null 2>&1; then
        "$script_dir/podman.sh" image
        return
    fi
    # An explicitly provided image is left alone: it was not necessarily built
    # from this Containerfile and rebuilding it under that tag would be wrong.
    if [ -n "${FRACTAL_ANDROID_IMAGE:-}" ]; then
        return
    fi
    expected=$(containerfile_digest)
    actual=$(image_digest)
    if [ "$expected" != "$actual" ]; then
        echo "Container image $image was built from a different" \
            "Containerfile; rebuilding it." >&2
        "$script_dir/podman.sh" image
    fi
}

# `--init` is not a nicety here. Pixiewood waits for the `ninja` it launched
# with `waitpid(-1, 0)` and aborts with `Unexpected child process N died` for
# any PID it did not launch itself. As PID 1 of the container it is also the
# reaper of every orphan of the build tree, so a single `rustc` left behind by
# a `cargo` that the kernel killed ends the build with that message -- and,
# because the container is torn down as soon as PID 1 exits, the error `cargo`
# was about to print is lost with it. With an init process at PID 1 the orphans
# are reaped there and a failing build reports its own error.
# See experiments/android-orphan-reaper/.
init_args="--init"

# Podman implements `--init` with a separate `catatonit` binary, so the option
# can be present and still not work. Rather than trade the diagnosis of one
# obscure failure for another, the support is established once, on a container
# that does nothing.
ensure_init_support() {
    if [ -n "${init_checked:-}" ]; then
        return
    fi
    init_checked=1
    if engine_run true > /dev/null 2>&1; then
        return
    fi
    saved_init_args=$init_args
    init_args=""
    if engine_run true > /dev/null 2>&1; then
        echo "Warning: $engine cannot run this image with --init. Pixiewood" \
            "runs as PID 1, so a build that fails may report" \
            "'Unexpected child process N died' instead of its own error." >&2
        return
    fi
    # Neither works, so --init is not the problem. Restore it and let the real
    # command report whatever is.
    init_args=$saved_init_args
}

engine_run() {
    prepare_state
    # shellcheck disable=SC2086 # the args are intentionally word-split.
    "$engine" run --rm \
        $init_args \
        $userns_args \
        --user "$(id -u):$(id -g)" \
        --env HOME=/workspace/.android-container/home \
        --env CARGO_HOME=/workspace/.android-container/cargo \
        --env CARGO_BUILD_JOBS="$cargo_jobs" \
        --env GRADLE_USER_HOME=/workspace/.android-container/gradle \
        --volume "$project_dir:/workspace${volume_suffix}" \
        --workdir /workspace \
        "$image" "$@"
}

run_container() {
    ensure_init_support
    engine_run "$@"
}

# Every build step goes through this, so that a step that dies without an
# error message of its own -- which is what running out of memory looks like --
# says so instead of just ending the script.
run_step() {
    if run_container "$@"; then
        return 0
    fi
    status=$?
    echo >&2
    echo "Failed with exit status $status: $*" >&2
    if [ "$status" -eq 137 ]; then
        echo "Exit status 137 means the process was killed with SIGKILL. On a" \
            "build of this size that is almost always the out-of-memory" \
            "killer. Retry with fewer parallel Rust jobs, for example" \
            "CARGO_BUILD_JOBS=1 (currently $cargo_jobs)." >&2
    fi
    exit "$status"
}

command=${1:-}
case "$command" in
    image)
        "$engine" build \
            --file "$script_dir/Containerfile" \
            --label "$containerfile_label=$(containerfile_digest)" \
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
        run_step pixiewood -C /workspace prepare "$1"
        ;;
    generate)
        ensure_image
        run_step pixiewood -C /workspace generate
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
        run_step pixiewood -C /workspace build
        ;;
    app)
        shift
        if [ "$#" -ne 0 ]; then
            usage >&2
            exit 2
        fi
        ensure_image
        report_build_resources
        run_step pixiewood -C /workspace prepare android/pixiewood.xml
        run_step pixiewood -C /workspace generate
        run_step pixiewood -C /workspace build
        run_step build-aux/android/verify-apk.sh "$apk_relative_path"
        report_apk "Fractal APK" "$apk_relative_path"
        ;;
    smoke)
        shift
        if [ "$#" -ne 0 ]; then
            usage >&2
            exit 2
        fi
        ensure_image
        smoke_dir=experiments/android-gtk-smoke
        run_step pixiewood -C "/workspace/$smoke_dir" prepare \
            "$smoke_dir/pixiewood.xml"
        run_step pixiewood -C "/workspace/$smoke_dir" generate
        run_step pixiewood -C "/workspace/$smoke_dir" build
        run_step build-aux/android/verify-apk.sh \
            "$smoke_dir/$apk_relative_path"
        report_apk "Smoke test APK" "$smoke_dir/$apk_relative_path"
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
