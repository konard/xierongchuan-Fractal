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
resolvable `DT_NEEDED` entries). The checks that are about Fractal itself
rather than the toolchain are turned off for it with
`FRACTAL_ANDROID_APP_CHECKS=0`, because the smoke application has no settings
schema of its own, no translations and no reason to reach the network. Any
built APK can be re-checked with:

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

Pixiewood provides wraps for GLib, fontconfig, cairo, gdk-pixbuf, GTK,
HarfBuzz, libadwaita and rsvg, but not for GStreamer, GtkSourceView, glycin,
libshumate or SQLite. Those are Cargo features of Fractal that the Android
build turns off, each with a fallback implementation behind the same API; see
`docs/android-porting-plan.md` for the current state of that work.

### Install tags

Pixiewood assembles the package from `meson install --tags runtime`, and Meson
skips every installed file whose tag is not in that list. It guesses `runtime`
only for what lands in `bindir` or looks like a shared library, so anything the
application needs at runtime from `datadir` has to say so explicitly. Two files
of Fractal did not:

- the GSettings schema, which is now installed with `install_tag: 'runtime'`,
  the same way GTK installs its own schemas. Without it `g_settings_new()`
  aborts the process before a window appears;
- the translations, which `i18n.gettext()` always tags `i18n`. The Android
  build installs a second, `runtime`-tagged copy of them through
  `install-translations.sh`, so that the package is not English-only.

`verify-apk.sh` fails when either is missing from the package.

### Host resources

The end-to-end build compiles the whole GTK stack and every Rust dependency of
Fractal for `aarch64`. It leaves about **8 GiB** in the checkout
(`.pixiewood/`, `.android-container/` and `subprojects/`) next to the **5 GiB**
of the toolchain image; `podman.sh` warns when less than 20 GiB is free. On six
cores it takes a little over half an hour.

Cargo builds one crate per job, and the largest ones need roughly 2 GiB each,
so `podman.sh` caps the number of parallel Rust jobs at whichever is smaller,
the CPU count or one job per 2 GiB of RAM, and prints the value it chose. A
machine that still runs out of memory -- the symptom is the build dying without
an error, or `Killed`, somewhere in the middle of `Compiling ...` -- should be
given a lower value explicitly:

```sh
CARGO_BUILD_JOBS=1 build-aux/android/podman.sh app
```

The build resumes where it stopped, so a retry does not repeat the work that
already succeeded.

### Building on CI

A machine that does not have those 20 GiB to spare does not have to build the
package at all: `.github/workflows/android.yml` runs the very same commands on
a GitHub Actions runner and uploads the result as the
`fractal-android-debug-apk` artifact. It runs `podman.sh image` and
`podman.sh app` with Docker as the engine -- the script accepts it as a
fallback, and Docker is what the runners ship. A hosted runner has the room:
the last full run started with 88 GiB free, used 13 GiB of it and took
23 minutes from an empty cache, of which the application build was 17.

Two things are kept between runs, both keyed by the file that defines them
rather than by a date or a floating tag:

- the toolchain image, pushed to `ghcr.io/<owner>/fractal-android-builder`
  under a tag derived from the checksum of the `Containerfile`, so an unchanged
  recipe is pulled instead of downloading the Android SDK and the NDK again;
- `.android-container/`, the Cargo and Gradle home of the build, cached under
  the checksum of `Cargo.lock`.

A pull request from a fork gets a read-only token: it builds the image in the
job and skips the push, which costs time but never fails the build.

The workflow can also be started by hand, for a branch that has not opened a
pull request yet:

```sh
gh workflow run android.yml --ref BRANCH
```

The desktop pipeline is unaffected. It stays on GitLab (`.gitlab-ci.yml`), and
this workflow only runs for changes that can end up in the APK.

### Package size

The APK is a development build, but it is not a debugging one: the Rust
workspace is compiled with line tables instead of full DWARF, and
`gradle-init.gradle` points the Android Gradle Plugin at the pinned NDK so that
it strips the native libraries it packages. Without both, the symbol tables are
five times the size of the code and the package grows from about 150 MiB to
about 770 MiB. Unstripped copies of every library stay in `.pixiewood/` for
`llvm-symbolizer`.

`prepare` creates `.pixiewood/` and may create `subprojects/` wrappers; both
are generated and ignored. `install` and `logcat` intentionally execute host
`adb`. No USB device, ADB server, SDK, NDK, Gradle, Rust or Java is exposed to
the container command line from the host.

The source checkout is mounted read/write so generated APKs are available on
the host, and `app` prints the absolute path of the package it produced.

On an SELinux host with Podman, the mount is relabelled with `:z`, which gives
the checkout the shared `container_file_t` label. It is deliberately not `:Z`:
that assigns an MCS category private to a single container, and everything else
SELinux confines -- another container, a Flatpak application, the file manager
-- is then denied access to your own source tree. Podman on a host without
SELinux gets no suffix at all. Both can be overridden, including with an empty
value to skip relabelling:

```sh
FRACTAL_PODMAN_VOLUME_SUFFIX=:Z build-aux/android/podman.sh app
```

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
