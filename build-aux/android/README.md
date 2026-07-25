# Android build container

`Containerfile` provides the complete **Android** build environment through
Podman. It contains the pinned Android SDK/NDK, JDK 17, Rust 1.93, Meson
1.11.2, Blueprint compiler, and the pinned Pixiewood (`gtk-android-builder`)
revision. Nothing from this toolchain is installed on the host.

Host requirements are deliberately limited to:

- Podman on an x86_64 Linux host (or an x86_64 Podman VM). Docker is accepted
  as a fallback and is selected automatically when Podman is absent; set
  `FRACTAL_CONTAINER_ENGINE=podman|docker` to force one of them;
- `adb` only when installing or inspecting an APK on a device.

The image does not replace the existing desktop Flatpak/Meson development
environment. It is Android-only and is never invoked by the desktop build.

## Commands

Run all commands from the repository root:

```sh
# Build the local image. This is the only download/install step.
build-aux/android/podman.sh image

# Every other command builds it automatically, and rebuilds it when the
# Containerfile in the checkout differs from the one the image was built from.

# Confirm the pinned builder is available inside the container.
build-aux/android/podman.sh exec -- pixiewood --version

# Open a complete Android build shell, without installing host packages.
build-aux/android/podman.sh shell
```

## Toolchain smoke test

Before Fractal itself can be packaged, the toolchain is validated end to end
with a minimal GTK4/libadwaita application in
`experiments/android-gtk-smoke/`:

```sh
build-aux/android/podman.sh smoke
```

This runs `pixiewood prepare`/`generate`/`build` for the smoke application and
then checks the resulting APK with `verify-apk.sh` (minSdkVersion, launcher
activity, single `arm64-v8a` ABI, packaged GTK/libadwaita runtime libraries and
resolvable `DT_NEEDED` entries). Any built APK can be re-checked with:

```sh
build-aux/android/podman.sh verify [APK]
```

## Building Fractal

The Pixiewood manifest of the application is `android/pixiewood.xml`. It builds
the Meson target of the Android launcher, which is only configured when
`-Dandroid=true` is passed, so a desktop build never sees it:

```sh
build-aux/android/podman.sh app
```

That command is the end-to-end equivalent of:

```sh
build-aux/android/podman.sh prepare android/pixiewood.xml
build-aux/android/podman.sh generate
build-aux/android/podman.sh build
build-aux/android/podman.sh verify
build-aux/android/podman.sh install
build-aux/android/podman.sh logcat
```

The manifest uses the launcher icon in `android/data/`, and the metainfo copy
that the Android build generates in `data/android-metainfo.xml`, because
Pixiewood only reads metainfo elements that are in the metainfo XML namespace.

Fractal's full dependency set is not yet buildable this way: Pixiewood provides
wraps for GLib, fontconfig, cairo, gdk-pixbuf, GTK, HarfBuzz, libadwaita and
rsvg, but not for GStreamer, GtkSourceView, glycin, libwebp, libshumate or
SQLite, which `meson.build` requires. See `docs/android-porting-plan.md` for
the current state of that work.

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

`pixiewood.lock` records both the pinned toolchain versions and the revisions
of the GTK stack that Pixiewood's wraps resolved to for the last successful
build, so drift in the runtime dependencies is visible in review. It is
documentation, not an input to the build; the `Containerfile` arguments remain
authoritative for the toolchain.

Pixiewood's SVG icon converter requires a full Android Studio installation.
To keep the build image compact and based solely on command-line tools, the
future Fractal Pixiewood manifest must use a pre-generated Android icon rather
than that converter. This does not affect GTK/libadwaita or the APK runtime.
