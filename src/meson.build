if not build_env_only
    # Compile Blueprint resources
    blp_files = []
    ui_files = []
    ui_files_xml = ''

    # These files are already added manually in the resources file because they
    # need custom handling.
    blp_gresource_exceptions = [
        'shortcuts-dialog.blp',  # Must use the root prefix for auto-detection, it must not be under `/ui`.
    ]

    # These files use widgets from libraries that are not available on Android.
    # The widgets that they describe are replaced by fallback widgets that are
    # constructed in Rust, so their template is not needed there.
    blp_android_exceptions = [
        'components/media/location_viewer.blp',  # libshumate
        'components/media/video_player.blp',  # GStreamer
    ]

    foreach file : fs.read('ui-blueprint-resources.in').splitlines()
        file = file.strip()

        if file.startswith('#') or file == ''
            continue
        endif

        if android and blp_android_exceptions.contains(file)
            continue
        endif

        blp_files += file

        # We need to put all output files in the same directory
        ui_alias = file.replace('.blp', '.ui')
        ui_file = ui_alias.replace('/', '-')
        ui_files += ui_file

        if not blp_gresource_exceptions.contains(file)
            ui_files_xml += '\n    <file compressed="true" preprocess="xml-stripblanks" alias="' + ui_alias + '">' + ui_file + '</file>'
        endif
    endforeach

    blueprints = custom_target(
        'blueprints',
        input: blp_files,
        output: ui_files,
        command: [
            '../build-aux/compile-blueprints.sh',
            blueprint_compiler.full_path(),
            '@OUTDIR@',
            '@CURRENT_SOURCE_DIR@',
            '@INPUT@',
        ],
        env: {
            # GtkSourceView is not available on Android, its widgets are
            # replaced by their GTK equivalent.
            'FRACTAL_BLUEPRINT_NO_SOURCEVIEW': android ? '1' : '0',
        },
    )

    # Populate the GResource file for the compiled Blueprint files dynamically.
    ui_resources_xml = configure_file(
        input: 'ui-resources.gresource.xml.in',
        output: 'ui-resources.gresource.xml',
        configuration: {'UI_FILES': ui_files_xml},
    )

    # Compile the UI files.
    ui_resources = gnome.compile_resources(
        'ui-resources',
        ui_resources_xml,
        gresource_bundle: true,
        install: true,
        install_dir: pkgdatadir,
        dependencies: [blueprints, ui_resources_xml],
    )
endif

# Generate config.rs
global_conf = configuration_data()
global_conf.set_quoted('APP_ID', application_id)
global_conf.set(
    'DISABLE_GLYCIN_SANDBOX',
    get_option('disable-glycin-sandbox').to_string(),
)
global_conf.set_quoted('GETTEXT_PACKAGE', gettext_package)
global_conf.set_quoted('LOCALEDIR', localedir)
global_conf.set_quoted('PKGDATADIR', pkgdatadir)
global_conf.set('PROFILE', profile)
global_conf.set_quoted('VERSION', full_version)
global_conf.set_quoted('BUILD_DIR', meson.project_build_root())
config = configure_file(
    input: 'config.rs.in',
    output: 'config.rs',
    configuration: global_conf,
)
run_command(
    'cp',
    config,
    meson.project_source_root() / 'src' / 'config.rs',
    check: true,
)

# Cargo settings
cargo_options = ['--manifest-path', meson.project_source_root() / 'Cargo.toml']
cargo_build_options = cargo_options

if profile == 'Devel'
    rust_target = 'debug'
    message('Building in debug mode')
else
    cargo_build_options += ['--release']
    rust_target = 'release'
    message('Building in release mode')
endif

if android and not build_env_only
    # Build the Android library with cargo, and link the native launcher
    # against it. The launcher is what the Android runtime starts; all the
    # application code lives in the library.
    cpu_family = host_machine.cpu_family()
    if cpu_family == 'aarch64'
        rust_android_triple = 'aarch64-linux-android'
    elif cpu_family == 'arm'
        rust_android_triple = 'armv7-linux-androideabi'
    elif cpu_family == 'x86_64'
        rust_android_triple = 'x86_64-linux-android'
    else
        error('Unsupported Android ABI: ' + cpu_family)
    endif

    android_lib_name = 'libfractal_android.so'
    android_cargo_options = [
        '--manifest-path', meson.project_source_root() / 'android' / 'native' / 'Cargo.toml',
        '--target', rust_android_triple,
    ]
    if profile != 'Devel'
        android_cargo_options += ['--release']
    endif

    # Cargo does not read the Meson cross file, so the NDK toolchain that Meson
    # found is forwarded to it, as well as the `pkg-config` files that Meson
    # generated for the cross-compiled GTK stack. This is scoped to this target:
    # the desktop build never sees these variables.
    android_cc = meson.get_compiler('c').cmd_array()[0]
    android_ndk_bindir = fs.parent(android_cc)
    rust_android_env_triple = rust_android_triple.underscorify().to_upper()

    android_cargo_env = cargo_env + [
        'CARGO_TARGET_' + rust_android_env_triple + '_LINKER=' + android_cc,
        'CC_' + rust_android_triple.underscorify() + '=' + android_cc,
        'AR_' + rust_android_triple.underscorify() + '=' + android_ndk_bindir / 'llvm-ar',
        # The GTK stack is built by Meson as subprojects, so it is only
        # available through the uninstalled `pkg-config` files of the build
        # directory. `PKG_CONFIG_LIBDIR` replaces the default search path, so
        # the `.pc` files of the host are never picked up.
        'PKG_CONFIG_ALLOW_CROSS=1',
        'PKG_CONFIG_LIBDIR=' + meson.project_build_root() / 'meson-uninstalled',
        # The GResource bundles are compiled into the library: an Android
        # package has no installation prefix to load them from at runtime.
        'FRACTAL_RESOURCES_FILE=' + resources.full_path(),
        'FRACTAL_UI_RESOURCES_FILE=' + ui_resources.full_path(),
    ]

    android_lib = custom_target(
        'cargo-build-android',
        build_by_default: true,
        build_always_stale: true,
        output: android_lib_name,
        console: true,
        install: true,
        install_dir: get_option('libdir'),
        depends: [resources, ui_resources],
        env: android_cargo_env,
        command: [
            cargo,
            'build',
            android_cargo_options,
            '&&',
            'cp',
            cargo_target_dir / rust_android_triple / rust_target / android_lib_name,
            '@OUTPUT@',
        ],
    )

    executable(
        meson.project_name(),
        meson.project_source_root() / 'android' / 'shim' / 'main.c',
        link_args: ['-L' + meson.current_build_dir(), '-lfractal_android'],
        link_depends: android_lib,
        install_rpath: '$ORIGIN',
        android_exe_type: 'application',
        install: true,
        install_dir: bindir,
    )
elif not build_env_only
    # Build binary with cargo
    custom_target(
        'cargo-build',
        build_by_default: true,
        build_always_stale: true,
        output: meson.project_name(),
        console: true,
        install: true,
        install_dir: bindir,
        depends: [resources, ui_resources],
        env: cargo_env,
        command: [
            cargo,
            'build',
            cargo_build_options,
            '&&',
            'cp',
            cargo_target_dir / rust_target / meson.project_name(),
            '@OUTPUT@',
        ],
    )

    # Lint Rust code with clippy
    run_target(
        'cargo-clippy',
        env: cargo_env,
        command: [
            cargo,
            'clippy',
            cargo_options,
        ],
    )

    # Run Rust tests with cargo-nextest
    custom_target(
        'cargo-test',
        build_always_stale: true,
        output: 'junit.xml',
        console: true,
        depends: [resources, ui_resources],
        command: [
            'env',
            cargo_env,
            'cargo-nextest',
            'nextest',
            'run',
            cargo_options,
            '--config-file',
            nextest_config.full_path(),
            '&&',
            'cp',
            nextest_store_dir / 'default/junit.xml',
            '@OUTPUT@',
        ],
    )
endif

# Build docs with rustdoc
rustdoc_flags = ' '.join(
    '-Zunstable-options',
    '--enable-index-page',
    # The gtk-rs project docs on docs.rs don't build or don't have comments.
    '--extern-html-root-url=gdk4=https://gtk-rs.org/gtk4-rs/stable/latest/docs/',
    '--extern-html-root-url=gio=https://gtk-rs.org/gtk-rs-core/stable/latest/docs/',
    '--extern-html-root-url=glib=https://gtk-rs.org/gtk-rs-core/stable/latest/docs/',
    '--extern-html-root-url=gsk4=https://gtk-rs.org/gtk4-rs/stable/latest/docs/',
    '--extern-html-root-url=gtk4=https://gtk-rs.org/gtk4-rs/stable/latest/docs/',
    '--extern-html-root-url=libadwaita=https://world.pages.gitlab.gnome.org/Rust/libadwaita-rs/stable/latest/docs/',
    # We are usually on a commit rather than on a stable release, so we are more likely to have up-to-date comments on nightly docs.
    '--extern-html-root-url=matrix-sdk=https://matrix-org.github.io/matrix-rust-sdk/',
    '--extern-html-root-url=matrix-sdk-base=https://matrix-org.github.io/matrix-rust-sdk/',
    '--extern-html-root-url=matrix-sdk-common=https://matrix-org.github.io/matrix-rust-sdk/',
    '--extern-html-root-url=matrix-sdk-crypto=https://matrix-org.github.io/matrix-rust-sdk/',
    '--extern-html-root-url=matrix-sdk-qrcode=https://matrix-org.github.io/matrix-rust-sdk/',
    '--extern-html-root-url=matrix-sdk-sqlite=https://matrix-org.github.io/matrix-rust-sdk/',
    '--extern-html-root-url=matrix-sdk-store-encryption=https://matrix-org.github.io/matrix-rust-sdk/',
    '--extern-html-root-url=matrix-sdk-ui=https://matrix-org.github.io/matrix-rust-sdk/',
    '--extern-html-root-url=matrix-sdk-ui=https://matrix-org.github.io/matrix-rust-sdk/',
    '--extern-html-root-url=ruma=https://docs.ruma.dev/',
    '--extern-html-root-url=ruma-client-api=https://docs.ruma.dev/',
    '--extern-html-root-url=ruma-common=https://docs.ruma.dev/',
    '--extern-html-root-url=ruma-events=https://docs.ruma.dev/',
    '--extern-html-root-url=ruma-html=https://docs.ruma.dev/',
    '--extern-html-root-url=ruma-signatures=https://docs.ruma.dev/',
    '--cfg=docsrs',
    '-Dwarnings',
)
cargo_doc_env = ['RUSTDOCFLAGS=' + rustdoc_flags]
cargo_doc_options = ['--package=fractal', '--no-deps', '-Zrustdoc-map']

custom_target(
    'cargo-doc',
    build_always_stale: true,
    output: 'doc',
    console: true,
    env: cargo_env + cargo_doc_env,
    command: [
        cargo,
        'doc',
        cargo_options + cargo_doc_options,
        '&&',
        'cp',
        '-r',
        cargo_target_dir / 'doc',
        '@OUTPUT@',
    ],
)
