project(
    'fractal-android-smoke',
    'c',
    version: '0.1.0',
    license: 'GPL-3.0-or-later',
    # `android_exe_type` requires Meson 1.9.
    meson_version: '>= 1.9',
)

gnome = import('gnome')

gtk_dep = dependency('gtk4')
adw_dep = dependency('libadwaita-1')

metainfo = configure_file(
    input: 'data/org.gnome.Fractal.AndroidSmoke.metainfo.xml',
    output: 'org.gnome.Fractal.AndroidSmoke.metainfo.xml',
    copy: true,
)

executable(
    'fractal-android-smoke',
    'src/main.c',
    dependencies: [gtk_dep, adw_dep],
    android_exe_type: 'application',
    install: true,
)
