# Android build container

`Containerfile` provides the complete **Android** build environment through
Podman. It contains the pinned Android SDK/NDK, JDK 17, Rust 1.93, Meson 1.9,
Blueprint compiler, and the pinned Pixiewood (`gtk-android-builder`) revision.
Nothing from this toolchain is installed on the host.

Host requirements are deliberately limited to:

- Podman on an x86_64 Linux host (or an x86_64 Podman VM);
- `adb` only when installing or inspecting an APK on a device.

The image does not replace the existing desktop Flatpak/Meson development
environment. It is Android-only and is never invoked by the desktop build.

## Commands

Run all commands from the repository root:

```sh
# Build the local image. This is the only download/install step.
build-aux/android/podman.sh image

# Confirm the pinned builder is available inside the container.
build-aux/android/podman.sh exec -- pixiewood --version

# Open a complete Android build shell, without installing host packages.
build-aux/android/podman.sh shell
```

Once the Android adaptation introduces its Pixiewood manifest, the build flow
is:

```sh
build-aux/android/podman.sh prepare build-aux/android/pixiewood.xml
build-aux/android/podman.sh generate
build-aux/android/podman.sh build
build-aux/android/podman.sh install
build-aux/android/podman.sh logcat
```

`prepare` creates `.pixiewood/` and may create `subprojects/` wrappers; both
are generated and ignored. `install` and `logcat` intentionally execute host
`adb`. No USB device, ADB server, SDK, NDK, Gradle, Rust or Java is exposed to
the container command line from the host.

The source checkout is mounted read/write so generated APKs are available on
the host. On SELinux hosts the default `:Z` mount relabels the checkout for the
container. If the environment does not support this option, run commands with
`FRACTAL_PODMAN_VOLUME_SUFFIX=`.

## Reproducibility and current limitation

The command-line-tools archive is checksum-verified. The Pixiewood commit is
pinned in the `Containerfile`; verify it in an image with:

```sh
build-aux/android/podman.sh exec -- cat /usr/local/opt/gtk-android-builder/REVISION
```

Pixiewood's SVG icon converter requires a full Android Studio installation.
To keep the build image compact and based solely on command-line tools, the
future Fractal Pixiewood manifest must use a pre-generated Android icon rather
than that converter. This does not affect GTK/libadwaita or the APK runtime.
