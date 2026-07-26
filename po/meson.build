i18n.gettext(
    gettext_package,
    args: ['--keyword=gettext_f', '--keyword=ngettext_f:1,2'],
    preset: 'glib',
)

if android
    # `i18n.gettext()` tags every catalogue it installs `i18n`, and the Android
    # package is assembled from `meson install --tags runtime`, so a package
    # built from it would only ever speak English. The catalogues are part of
    # the runtime of an application rather than an optional extra, so they are
    # installed a second time by a script that carries the `runtime` tag.
    # Only the Android build does this: on the desktop the `i18n` tag is what
    # distributions expect, and this would install every `.mo` file twice.
    meson.add_install_script(
        meson.project_source_root() / 'build-aux/android/install-translations.sh',
        meson.current_build_dir(),
        get_option('localedir'),
        gettext_package,
        install_tag: 'runtime',
    )
endif
