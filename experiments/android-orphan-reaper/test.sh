#!/bin/sh
# Why `build-aux/android/podman.sh` runs the container with an init process.
#
# `pixiewood build` waits for its `ninja` children with `waitpid(-1, 0)` and
# aborts with `Unexpected child process N died` as soon as that call returns a
# PID it did not launch itself (`ParallelRunner::wait`, see
# `parallel-runner.pl`). When Pixiewood is PID 1 of the container, the kernel
# reparents every orphan of the build tree to it, so any process that outlives
# its parent -- a `rustc` left behind by a `cargo` that the OOM killer
# terminated, for example -- is reaped by that loop and kills the build. The
# real error is lost too: PID 1 exiting tears the container down before `ninja`
# and `cargo` can report anything.
#
# Running the container with `--init` puts a reaper at PID 1, so orphans never
# reach Pixiewood and the build fails -- or succeeds -- on its own terms.
#
# The test needs a container engine, because the behaviour under test is "which
# process is PID 1 of the PID namespace". It skips itself when there is none.
set -eu

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
project_dir=$(CDPATH= cd -- "$script_dir/../.." && pwd)
image=${FRACTAL_ORPHAN_TEST_IMAGE:-docker.io/library/perl:5-slim}
failures=0

check() {
    name=$1
    expected=$2
    actual=$3
    if [ "$actual" = "$expected" ]; then
        echo "ok: $name"
    else
        echo "FAIL: $name: expected '$expected', got '$actual'" >&2
        failures=$((failures + 1))
    fi
}

# First, that `podman.sh` asks for the option at all. This part needs no
# container engine: the fake one of the sibling experiment records the command
# line it was called with.
work_dir=$(mktemp -d)
trap 'rm -rf "$work_dir"' EXIT
FAKE_ENGINE_STORE="$work_dir/image"
FAKE_ENGINE_LOG="$work_dir/log"
export FAKE_ENGINE_STORE FAKE_ENGINE_LOG
FRACTAL_CONTAINER_ENGINE="$script_dir/../android-image-staleness/fake-engine.sh" \
    "$project_dir/build-aux/android/podman.sh" exec -- true > /dev/null
if grep -q '^run --rm --init ' "$FAKE_ENGINE_LOG"; then
    passed=yes
else
    passed="no: $(grep '^run ' "$FAKE_ENGINE_LOG" | head -n 1)"
fi
check "podman.sh runs the container with --init" yes "$passed"

engine=${FRACTAL_CONTAINER_ENGINE:-}
if [ -z "$engine" ]; then
    if command -v podman > /dev/null 2>&1; then
        engine=podman
    elif command -v docker > /dev/null 2>&1; then
        engine=docker
    else
        echo "skip: the remaining checks need podman or docker" >&2
        [ "$failures" -eq 0 ] || exit 1
        exit 0
    fi
fi

run_case() {
    "$engine" run --rm "$@" \
        --volume "$script_dir:/runner:ro" \
        "$image" \
        perl /runner/parallel-runner.pl sh /runner/orphan-workload.sh 2>&1
}

echo "Using container engine: $engine"

output=$(run_case || true)
case "$output" in
    *"Unexpected child process"*) verdict=died ;;
    *"build succeeded"*) verdict=succeeded ;;
    *) verdict="other: $output" ;;
esac
check "without an init process, an orphan kills the build" died "$verdict"

output=$(run_case --init || true)
case "$output" in
    *"Unexpected child process"*) verdict=died ;;
    *"build succeeded"*) verdict=succeeded ;;
    *) verdict="other: $output" ;;
esac
check "with an init process, the orphan is reaped by it" succeeded "$verdict"

if [ "$failures" -ne 0 ]; then
    echo "$failures check(s) failed." >&2
    exit 1
fi
echo "All checks passed."
