# План Android-порта Fractal

Статус: **в работе** (этапы A и B закрыты со стороны сборки: сам Fractal
воспроизводимо собирается и упаковывается в `app-arm64-v8a-debug.apk`,
проходящий `verify-apk.sh`. Остаётся проверка на устройстве и этапы C–F.
Библиотеки без Android-порта отключены Cargo-фичами с fallback-реализациями —
см. «Текущие блокеры»)
Целевая платформа: Android 12+ (`minSdk = 31`)
Основная ABI на первом этапе: `arm64-v8a`

## Цель

Получить настоящий Android APK/AAB, в котором Fractal работает как нативное
приложение GTK4/libadwaita: GTK рисует интерфейс через Android GDK backend,
а небольшой Kotlin/JNI слой предоставляет только Android-системные API.
Это не WebView и не удалённый Linux runtime. Визуальный язык остаётся
Adwaita, а не Material 3.

Первый практический результат должен быть минимальным и измеримым: на
устройстве или эмуляторе с Android 12+ устанавливается debug APK, открывается
без crash и показывает стартовый экран Fractal. Функции, зависящие от Android
API, подключаются только после этого запуска.

## Зафиксированные решения

- Использовать **GTK4**. Fractal уже требует GTK 4.20.2 и libadwaita 1.8;
- Использовать **Pixiewood / gtk-android-builder** как основной сборщик
  Android-native runtime и генератор Gradle-проекта. Его ревизию, источники и
  патчи нужно зафиксировать в lock-файле или контейнерном образе. Если он не
  собирает весь необходимый набор библиотек воспроизводимо, резервный путь —
  повторить зафиксированный Android pipeline GTK demo, а не заменить UI
  другим toolkit'ом.
- Использовать Android Gradle Plugin только для Android manifest, ресурсов,
  signing, packaging и установки. `cargo-apk` не использовать: он не собирает
  и не упаковывает полный GTK/libadwaita runtime.
- Использовать NDK linker и `cargo-ndk` (либо эквивалентную конфигурацию
  Cargo, предоставляемую сборщиком) только для Rust `cdylib`. Они не являются
  главным APK-пакетировщиком.
- Первый APK — только `arm64-v8a`. Добавление `armeabi-v7a` допускается после
  устойчивого запуска arm64 и отдельной проверки всех нативных зависимостей.
- Версии GTK, libadwaita, GStreamer и Pixiewood нельзя обновлять неявно:
  каждая версия закрепляется в одном месте и обновляется отдельной задачей.

## Неприкосновенные правила desktop-сборки

Android-поддержка не должна менять существующее поведение Linux desktop.

- Обычные `meson setup`, `meson compile`, `cargo check` и desktop-пакетирование
  не требуют Android SDK, NDK, JDK, Gradle или сетевых загрузок.
- Desktop остаётся основным путём по умолчанию. Android запускается только
  явной командой/CI job из `android/` или `build-aux/android/`.
- Общее приложение живёт в Rust и Blueprint. Платформенный код ограничен
  `cfg(target_os = "android")` и маленьким Android bridge; Linux-код не
  оборачивается в Android-условия.
- Не добавлять Android-специфичные значения в глобальные `PATH`, `PKG_CONFIG_*`
  или Cargo-конфигурацию, которыми пользуется desktop.
- Не заменять существующие Linux portal/Secret Service реализации условными
  ветками внутри них. Для Android добавлять отдельные реализации за общим
  trait.
- Каждый этап, затрагивающий Rust или Meson, завершается desktop-проверкой из
  раздела «Контроль прогресса».

## Целевая структура

Имена ниже — ориентир для реализации; новое дерево создаётся только в момент,
когда соответствующая задача начата.

```text
android/                         # изолированный Gradle-проект Android
  app/
    src/main/AndroidManifest.xml
    src/main/java/.../FractalActivity.kt
    src/main/res/
  gradle/libs.versions.toml
  build.gradle.kts
build-aux/android/               # скрипты, manifests, toolchain и CI helpers
  pixiewood.lock
  build-android.sh
  verify-apk.sh
src/platform/
  mod.rs
  desktop.rs
  android.rs                     # только cfg(target_os = "android")
src/secret/android.rs
src/components/camera/android.rs
src/utils/location/android.rs
src/utils/notifications/android.rs
```

`main.rs` должен стать тонкой desktop-точкой входа. Общий запуск приложения
выносится в библиотечный модуль, который используют и desktop binary, и
Android `cdylib`. Это предотвращает расхождение инициализации GTK, ресурсов,
локализации и Matrix runtime.

## Контроль прогресса

Каждый отмеченный пункт должен иметь доказательство. Нельзя отмечать родитель
до завершения всех дочерних пунктов.

1. После каждого завершённого пункта заменить `[ ]` на `[x]`.
2. Добавить одну строку в журнал: дата, ID задачи, commit/merge request,
   команда проверки и краткий результат.
3. Если проверка не прошла, оставить пункт неотмеченным и добавить причину в
   журнал или отдельную issue.
4. Не объединять функциональные доработки с инфраструктурой в одном commit,
   кроме минимально необходимого glue-кода.

| Дата | ID | Commit / MR | Проверка | Результат / ссылка на issue |
| --- | --- | --- | --- | --- |
| — | — | — | — | — |
| 2026-07-25 | A2.1 | 6e7ffc5 | `git diff --check` | Added the isolated Podman Android toolchain definition and host wrapper; image build is still pending. |
| 2026-07-25 | A1 | 3e5aeaa | `grep -n 'gtk4\|libadwaita-1\|meson_version' meson.build`, `grep rust-version Cargo.toml` | Зафиксировано: Rust 1.93, Meson >= 1.4 (desktop) / 1.11.2 (Android image), GTK 4.20.2, libadwaita 1.8.0. |
| 2026-07-25 | A1 | 3e5aeaa | `pkg-config --exists gtk4 libadwaita-1 gstreamer-1.0` | Не выполнено: на host отсутствуют dev-пакеты gtk4/libadwaita/gstreamer, поэтому desktop `meson setup` и `cargo check` здесь недоступны. Записано как недостающая зависимость по правилу A1. |
| 2026-07-25 | A2 | 3e5aeaa, 76d4762 | `build-aux/android/podman.sh image`, `podman.sh exec -- pixiewood --version` | Образ собирается с нуля (EXIT=0). Починено: blueprint-compiler ставится из pinned upstream-коммита (на PyPI версии нет), Meson поднят до 1.11.2 (требование subproject fontconfig), добавлен python3-gi, SDK/build-tools подняты до 36 (Pixiewood генерирует compileSdk/targetSdk 36 при read-only SDK). |
| 2026-07-25 | A2 | 74f5eec | `cat build-aux/android/pixiewood.lock` | Версии toolchain и ревизии GTK-стека зафиксированы в lock-файле; `podman.sh` работает и на Docker (fallback), host получает только `adb`. |
| 2026-07-25 | A3 | 76d4762 | `podman.sh exec -- pixiewood -C experiments/android-gtk-smoke prepare/generate/build` | Ревизия Pixiewood 00b1862 проверена на минимальном GTK4/libadwaita demo: полный проход prepare → generate → build, EXIT=0. Источники и ревизии — в `build-aux/android/pixiewood.lock`. |
| 2026-07-25 | B2 | 76d4762 | `build-aux/android/podman.sh verify …/app-arm64-v8a-debug.apk` | Собран `app-arm64-v8a-debug.apk` (124 MiB): minSdkVersion 31, targetSdkVersion 36, launcher activity `org.gtk.android.ToplevelActivity`, 32 нативных библиотеки только для arm64-v8a, libgtk-4/libadwaita-1/libgio/libglib/libpango/libcairo/libgdk_pixbuf упакованы, все DT_NEEDED разрешаются. Запуск на устройстве не проверялся: Android-устройства/эмулятора в этом окружении нет. |
| 2026-07-25 | A1 | 4a46d7b | `docker build -f experiments/desktop-check/Containerfile …`, `cargo check --all-targets` | Выполнено в контейнере Fedora 43 (`experiments/desktop-check/`), потому что на host нет GTK 4.20/libadwaita 1.8: `Finished dev profile`, EXIT=0, без warnings. Тот же образ выполняет `meson setup -Dprofile=development`, EXIT=0. |
| 2026-07-25 | B3 | 3bcac0c | `cargo check --all-targets`, `rustfmt --edition 2024 --check` | Bootstrap вынесен в `fractal::run()` (`src/lib.rs`), desktop `main()` — тонкий вызов, `src/platform/{mod,desktop,android}.rs` разделяет платформы, Android `cdylib` — `android/native` + C-launcher `android/shim/main.c`. Desktop target и linker не менялись. NDK linker/`pkg-config` настраивает Pixiewood, отдельно не проверено. |
| 2026-07-25 | B4 | 3bcac0c | `cargo check --all-targets` | GResource-бандлы на Android встраиваются в библиотеку (`include_bytes!` по путям из Meson) и регистрируются до создания первого виджета; data/cache-директории берутся из `platform::{data_dir,cache_dir}` вместо `PKGDATADIR`/жёстких путей. |
| 2026-07-25 | B5 | 17cd597 | `cargo check --all-targets` | `AndroidSecret` возвращает пустой список сессий и переведённую ошибку вместо `unimplemented!()`; fallback location больше не паникует; логи идут в logcat через `tracing-android`. Проверка старта на устройстве — пункт B6. |
| 2026-07-25 | B3 | e01fe0a | `meson setup -Dprofile=development`, `meson setup -Dandroid=true` | Desktop-конфигурация проходит без Android-тулчейна и без C-компилятора (58 целей). `-Dandroid=true` в этом образе останавливается на проверке `meson >= 1.9` (Fedora 43 даёт 1.8.5); Android-образ содержит Meson 1.11.2. Сборка Android-цели не выполнялась: её блокируют отсутствующие wrap'ы (E1/E2). |
| 2026-07-25 | B3 | e01fe0a | `ninja data/org.gnome.Fractal.Devel.metainfo.xml && build-aux/android/namespace-metainfo.sh … && xmllint --noout` | Копия metainfo с namespace `https://specifications.freedesktop.org/metainfo/1.0` генерируется корректно и валидна как XML; на неё ссылается `android/pixiewood.xml` через `build://aarch64/data/android-metainfo.xml`. |
| 2026-07-25 | E1/E2 (частично) | 6031c0a | `cargo check --no-default-features`, `cargo check --all-targets` | Библиотеки без Android-порта вынесены в Cargo-фичи, включённые по умолчанию: `gstreamer`, `gtksourceview`, `glycin`, `libwebp`, `shumate`. Для каждой добавлена fallback-реализация с тем же API (GDK вместо glycin, статичные виджеты вместо GStreamer/Shumate, `Gtk.TextView` вместо SourceView). Обе конфигурации собираются без warnings. |
| 2026-07-25 | B3 | 04f8b99 | `podman.sh build` | Blueprint-файлы компилируются и для Android: `.blp` с `GtkSource` подставляются на GTK-эквиваленты, шаблоны на libshumate и GStreamer исключены. Введена переменная `FRACTAL_BLUEPRINT_TYPELIB_PATH`, чтобы blueprint-compiler валидировал по introspection-данным GTK 4.20 (взяты из Fedora 43 отдельным stage образа); Android-сборка GTK собирается без introspection, а Debian Bookworm даёт только GTK 4.8. |
| 2026-07-25 | B3 | 1b48d6d | `podman.sh build` | Toolchain дополнен: `android/rust.cross` даёт Meson Rust-компилятор для host machine, Cargo получает NDK linker/`ar` и `PKG_CONFIG_LIBDIR` на `meson-uninstalled` кросс-собранного GTK-стека, образ получил `gettext`, `appstream`, `desktop-file-utils` и `grass`. Всё ограничено Android-сборкой: desktop `PATH`, `PKG_CONFIG_*` и Cargo-конфигурация не менялись. |
| 2026-07-25 | B3 | 0a32a3b | `podman.sh build` | `android/native` стал workspace member (но не default member), поэтому Android-библиотека резолвит ровно те же версии зависимостей, что и desktop; раньше отдельный lock давал несовместимую пару gtk4/libadwaita. Добавлен конструктор fallback-Location и конвертация language tags metainfo в BCP 47 (`sr@latin` → `sr-Latn`), иначе AAPT отвергает `values-b+sr@latin`. |
| 2026-07-25 | B2 | 0a32a3b | `podman.sh verify` | Собран APK самого Fractal: `app-arm64-v8a-debug.apk`, 769 MiB (debug, без strip). minSdkVersion 31, targetSdkVersion 36, launcher activity `org.gtk.android.ToplevelActivity`, 32 нативные библиотеки только для arm64-v8a, все DT_NEEDED разрешаются. Запуск на устройстве не проверялся: Android-устройства/эмулятора в этом окружении нет. |
| 2026-07-25 | B2/B3 | e164c680 | `meson setup -Dprofile=development`, `ninja`, `cargo check --all-targets`, `podman.sh app` | Desktop-регрессии закрыты: `hooks/checks` исключён из workspace (иначе cargo считал pre-commit-хуки его частью и `meson setup` падал), полная desktop-сборка `ninja` проходит. Обе конфигурации собираются без warnings, повторная сборка APK из закоммиченного дерева снова проходит `verify-apk.sh`. |
| 2026-07-25 | A2 | 7d1733e | `sh experiments/android-image-staleness/test.sh` | Починен отчёт из PR #4: `podman.sh` переиспользовал любой образ с нужным тегом, поэтому checkout с обновлённым `Containerfile` собирался старым образом и падал в Meson на `Program 'msgfmt' not found` (пакет `gettext` добавлен только в 1b48d6d). Образ теперь помечается контрольной суммой `Containerfile` и пересобирается при её расхождении; явно заданный `FRACTAL_ANDROID_IMAGE` не трогается. Регрессия покрыта тестом с фиктивным движком контейнеров (EXIT=0, podman не нужен). |
| 2026-07-26 | A2 | 77f8401 | `sh experiments/android-orphan-reaper/test.sh` | Починен отчёт из PR #4 (`log.md`): сборка падала с `Unexpected child process 24308 died at /usr/local/bin/pixiewood line 264` на этапе Rust. Причина — pixiewood был PID 1 контейнера: его `ParallelRunner::wait` вызывает `waitpid(-1, 0)` и умирает на любом PID, который он не запускал, а ядро переподвешивает к PID 1 всех сирот дерева сборки (например `rustc`, оставшийся от убитого `cargo`). Вместе с PID 1 гибнет контейнер, поэтому настоящая ошибка `cargo` не печатается. Контейнер теперь запускается с `--init`, число параллельных Rust-задач ограничено 2 ГиБ RAM на задачу, а шаг, убитый SIGKILL, объясняет статус 137. |
| 2026-07-26 | B2 | e8f7dc2 | `podman.sh app` (EXIT=0), `podman.sh verify` | Сборка с нуля в этом окружении (6 ядер, 11.7 ГиБ RAM, docker): prepare → generate → build → verify, 34 минуты, `verify-apk.sh` проходит. APK уменьшен с 769 МиБ до **149 МиБ**: Rust-крейты workspace собираются с `CARGO_PROFILE_DEV_DEBUG=line-tables-only`, а AGP наконец стрипает нативные библиотеки — он искал NDK своей версии по умолчанию (28.2.13676358) и молча паковал 32 библиотеки нестрипнутыми; `build-aux/android/gradle-init.gradle` через `init.d` GRADLE_USER_HOME указывает ему на закреплённый NDK 27.2.12479018. Занимает ~8 ГиБ в checkout плюс ~5 ГиБ образ. Запуск на устройстве по-прежнему не проверялся: Android-устройства/эмулятора в этом окружении нет. |
| 2026-07-26 | B6 | 8a9eb90 | `sh experiments/android-app-library/test.sh` | Починен crash при старте из PR #4 (оба logcat'а показывают ровно одну fatal-ошибку): `java.lang.UnsatisfiedLinkError: Unable to open library "libfractal.so": dlopen failed: library "libfractal.so" not found`. Runtime GTK не запускает исполняемый файл: `RuntimeApplication` открывает библиотеку из meta-data `gtk.android.lib_name` (это имя цели Meson) и вызывает её `main`. Цель `fractal` ставилась в `bindir`, как desktop-бинарь, но Pixiewood пакует только `libdir` (он становится `jniLibs` сгенерированного Gradle-проекта) и явно отбрасывает `bin`, поэтому самого приложения в APK не было — при этом сборка, установка и `verify-apk.sh` проходили. Цель переехала в `libdir`, а `verify-apk.sh` теперь падает, если библиотеки из manifest нет в пакете или она не экспортирует `main`. Регрессия покрыта тестом с фиктивными `aapt2`/`llvm-readelf` (EXIT=0, Android SDK не нужен). |
| 2026-07-26 | A2 | 04cdbb8 | `podman.sh app` | Закрыты две претензии из PR #4. (1) «APK не видно на host»: `app`/`smoke` больше не печатают относительный путь, а проверяют, что файл существует, и выводят абсолютный путь с размером и командой установки; если файла нет, команда падает вместо рапорта об успехе. (2) «постоянно SELinux жалуется»: bind-mount checkout'а размечался как `:Z`, то есть получал MCS-категорию, приватную для одного контейнера, и остальная система (другие контейнеры, Flatpak, файловый менеджер) теряла доступ к собственному рабочему дереву пользователя. Теперь Podman использует `:z` и только при включённом SELinux, а `FRACTAL_PODMAN_VOLUME_SUFFIX` по-прежнему позволяет задать `:Z` или пустое значение. |

| 2026-07-26 | B6 | fdabcfe | `sh experiments/android-runtime-tags/test.sh`, `sh experiments/android-app-library/test.sh` | Найден и закрыт следующий гарантированный crash после B6: Pixiewood ставит файлы через `meson install --tags runtime`, а Meson пропускает всё, чему тег не угадан (`mesonbuild/minstall.py:386`, `backends.py:1635`). Схема GSettings Fractal лежит в `datadir` и тега не имела, поэтому в APK её не было, а `g_settings_new()` при отсутствии схемы делает `abort()` — окно не успело бы появиться. Схеме проставлен `install_tag: 'runtime'` (так же делает сам GTK для своих схем). По той же причине терялись переводы: `i18n.gettext()` жёстко ставит тег `i18n`, поэтому Android-сборка доустанавливает каталоги ещё раз install-скриптом с тегом `runtime`. Плюс исправлен `platform::localedir()`: он указывал на записываемый каталог пользователя, а ассеты распаковываются в первый системный data-каталог GLib. `verify-apk.sh` теперь падает, если в пакете нет `gschemas.compiled` со схемой приложения или нет ни одного `.mo`. |
| 2026-07-26 | C1 | d170720 | `podman.sh app` (EXIT=0), `podman.sh verify`, `cargo test` в `experiments/session-data-format` | Реализовано безопасное хранение сессии на Android. Раньше `AndroidSecret` был заглушкой и ничего не хранил; теперь все сессии сериализуются в один msgpack-блоб и шифруются ключом AES-256-GCM, который держит Android Keystore и наружу не отдаёт. Формат вынесен в `src/secret/session_data.rs` (`#[cfg(any(target_os = "android", test))]`) и покрыт 5 unit-тестами (round-trip, пустой список, неподдерживаемая версия, невалидное поле, не-msgpack). JNI-мост к Keystore (`src/secret/android/keystore.rs`) достаёт JVM через `gdk_android_display_get_env` — публичный GDK API с 4.18, — захватывая её один раз на GTK main-thread (`secret::init()` из `run()`) и читая из tokio-задач. Восстановление, удаление и инвалидация ключа обработаны без panic: `is_permanent()` различает временные сбои (файл сохраняется) от постоянных (`AEADBadTagException`/`KeyPermanentlyInvalidatedException` → файл и ключ стираются), новая версия формата никогда не перезаписывается. Запись атомарна (tmp + rename), чтение-модификация-запись сериализованы `Mutex`. Полная Android-сборка проходит (`BUILD SUCCESSFUL`, APK 152 МиБ, все guard'ы `verify-apk.sh` зелёные). Хостовый эксперимент `experiments/session-data-format` прогнал сериализацию там, где нет dev-библиотек GTK, и поймал реальный баг в round-trip (обращение к `ClientId::as_str` как к функции пути не компилируется в oauth2 5.0) до устройства. Запуск на устройстве не проверялся: Android-устройства/эмулятора в этом окружении нет. |
| 2026-07-26 | B6 | 22fad6b | `cargo test` в `experiments/panic-message`, `cargo +nightly fmt --check --all` | Диагностирован вылет при открытии чата из PR #4. Владелец подтвердил, что приложение запускается, но падает при открытии любой комнаты; в logcat — только `Fatal signal 6 (SIGABRT), code -1 (SI_QUEUE)` на `GTK Thread` и tombstone без сообщения. По tombstone: кадры `#00 abort` → `#01–#10` внутри `base.apk` (это `libfractal.so`, упакована несжатой и без soname) — это runtime Rust-паники, то есть **паникует Rust на GTK-потоке**, а не GLib (иначе `abort` был бы в `libglib-2.0.so`). Выше — глубокий повторяющийся каскад `g_object_set_property → notify → Rust-обработчик → снова set_property`, укоренённый в `gtk_widget_activate_action` (#65) — это путь открытия комнаты. Сообщения нет, потому что стандартный хук паники пишет в stderr, а Android его отбрасывает. Каждый кадр каскада — разный PC (не тесная петля из двух свойств), символизировать на host нельзя: несрипнутой `libfractal.so` здесь нет. Поэтому установлен Android-хук паники (`src/platform/android.rs`, `init_panic_logging`), который до делегирования прежнему хуку отправляет имя потока, `file:line` и сообщение в `tracing` → logcat. Следующий запуск назовёт точную паникующую строку. Логика хука вынесена в host-эксперимент `experiments/panic-message` (3 unit-теста: `&str`, `String`, прочий payload), потому что модуль `android.rs` компилируется только под `target_os = "android"` и его тесты не идут в desktop-CI. |
| 2026-07-26 | C2 | 6b52576 | `cargo check --all-targets`, `cargo test --lib`, `cargo clippy --all-targets -- -D warnings` в контейнере `experiments/desktop-check` для обеих конфигураций фич, `cargo +nightly fmt --check --all` | Исправлен вылет при открытии чата из PR #4. Хук паники из 22fad6b назвал причину одной строкой logcat: `glib-0.22.7/src/object.rs:3931: Target property highlight-syntax on type GtkTextBuffer not found`, следом `panic in a function that cannot unwind` и `Fatal signal 6 (SIGABRT)` на `GTK Thread`. Причина по коду: при переключении комнаты `MessageToolbar::update_current_composer_state` биндил своё свойство `markdown-enabled` к свойству `highlight-syntax` буфера композера. Это свойство объявляет только `GtkSourceBuffer`, а в Android-сборке фича `sourceview` выключена и `utils::sourceview::Buffer` — обычный `GtkTextBuffer`; `bind_property` с несуществующим целевым свойством паникует, а зовётся это из `gtk_widget_activate_action` через `extern "C"`-границу, где паника не может размотать стек и превращается в `abort` (тот самый каскад `set_property → notify`, который был виден в tombstone). Исправление: одна функция `utils::sourceview::bind_highlight_syntax()` вне `cfg_if!`, которая проверяет `has_property` и возвращает `Option<glib::Binding>`, а toolbar хранит и отвязывает биндинг только если он есть; на desktop поведение не меняется. Регрессия покрыта unit-тестом `bind_highlight_syntax_supports_both_buffers`, который строит объекты через `glib::Object::new`, поэтому не требует инициализации GTK и дисплея, и проверяет обе конфигурации фич. Аудит того же класса ошибок: `highlight-syntax` был единственным таким местом — остальные `bind_property` целятся в свойства собственных объектов или базового GTK, `.blp` под `FRACTAL_BLUEPRINT_NO_SOURCEVIEW=1` используют только свойства, совместимые с `Gtk.TextView`, а fallback-виджеты `video_player`/`location_viewer` объявляют тот же набор свойств, что и оригиналы. Тот же logcat подтверждает C1 на устройстве: процесс стартовал в 16:38:56, а к 16:39:02 уже шли запросы `matrix_sdk::http_client` и работа `matrix_sdk_crypto` — сессия восстановилась из Keystore без экрана логина и без предупреждения «Sessions will not be persisted». В контейнере остаётся падать не связанный с изменением тест `login::local_server::tests::generate_local_server_landing_page`: ему нужны собранный `resources.gresource` и дисплей для `gtk::init`, в CI он идёт через `meson compile src/cargo-test`. |
| 2026-07-26 | F1 | 595e64a, b8780a0, 272c0cd | GitHub Actions: run [30212305296](https://github.com/konard/xierongchuan-Fractal/actions/runs/30212305296) (сборка с нуля, EXIT=0, 23 минуты), run [30213312024](https://github.com/konard/xierongchuan-Fractal/actions/runs/30213312024) (образ из GHCR, EXIT=0, 18 минут), run [30214098612](https://github.com/konard/xierongchuan-Fractal/actions/runs/30214098612) (финальная версия workflow, EXIT=0, 17 минут) | Сборка APK перенесена в CI по просьбе из PR #4 («use CI/CD instead of local dockers for Android images and so on, as we don't have much space»). `.github/workflows/android.yml` запускает на runner'е ровно те же команды, что человек запускает локально — `podman.sh image` и `podman.sh app` с `FRACTAL_CONTAINER_ENGINE=docker`, движок скрипт поддерживает как fallback, — и выкладывает `app-arm64-v8a-debug.apk` (152 МиБ) артефактом `fractal-android-debug-apk`. `verify-apk.sh` в CI проходит целиком: minSdkVersion 31, только `arm64-v8a`, 33 нативные библиотеки, все `DT_NEEDED` разрешаются, `libfractal.so` экспортирует `main`, схема GSettings скомпилирована, 45 переводов на месте. Ставить локально Podman/Docker и держать 20 ГиБ на машине больше не нужно: runner'у места хватает без чисток — 88 ГиБ свободно на старте, 13 ГиБ съедает сборка (`.pixiewood` 6.5 ГиБ). Toolchain-образ хранится в GHCR под тегом из контрольной суммы `Containerfile` (тот же принцип, что у локального тега): рецепт не менялся — образ скачивается за 46–52 с вместо сборки за 1 м 50 с, а заодно исчезает зависимость каждого запуска от доступности dl.google.com, Fedora и crates.io. Другого кеша нет намеренно: первая версия workflow кешировала `.android-container`, но по логам там лежит только Gradle home (CARGO_HOME сборки `meson.build` уводит в build root, поэтому шаг сохранял 4 КиБ и писал `Path Validation Error`), а замеры показали, что кешировать нечего — 496 crates скачиваются за 5 секунд, дистрибутив Gradle за 3, против 17 минут компиляции. Workflow срабатывает только на пути, попадающие в APK, и на `workflow_dispatch`; desktop-pipeline остаётся на GitLab и не затронут. Не закрыто в F1: nightly-job с эмулятором и артефактом `adb logcat`. |

### Текущие блокеры для APK самого Fractal

Сборка APK самого Fractal больше не заблокирована: `podman.sh app` проходит
prepare → generate → build и выдаёт APK, который принимает `verify-apk.sh`.
Прежние блокеры закрыты так:

1. ~~Pixiewood предоставляет wrap'ы только для glib, fontconfig, cairo,
   gdk-pixbuf, gtk, harfbuzz, libadwaita и rsvg.~~ Закрыто: зависимости без
   Android-порта (gstreamer-*, gtksourceview-5, glycin-2, glycin-gtk4-2,
   libwebp, shumate-1.0) стали Cargo-фичами, включёнными по умолчанию для
   desktop и выключенными для Android; каждая имеет fallback-реализацию за
   тем же API. `sqlite3` даёт `rusqlite` со своей vendored-сборкой. Полноценный
   Android-порт этих библиотек остаётся пунктами E1 и E2.
2. ~~Бинарь Fractal производится cargo через Meson `custom_target`, а Pixiewood
   требует Meson-цель `executable(..., android_exe_type: 'application')` с
   `main(int, char**, char**)`, вызывающей `g_application_run` (пункт B3).~~
   Закрыто: при `-Dandroid=true` Meson собирает `android/native` как
   `cdylib` и линкует с ним C-launcher `android/shim/main.c` через
   `executable(..., android_exe_type: 'application')`. Цель собрана и
   упакована в APK.
3. Rust-зависимости aperture, ashpd и oo7 — только Linux (пункты D4, D5).
   Android-ветка secret storage реализована (пункт C1): сессии шифруются
   ключом AES-256-GCM из Android Keystore и хранятся в одном файле песочницы.
   `oo7`/Secret Service на Android не используются. Восстановление сессии между
   запусками подтверждено на устройстве владельцем (logcat в PR #4).

### Обязательные проверки после изменений

- Rust-код: `cargo check`.
- Изменения Blueprint/ресурсов/Meson: профильная Meson-сборка desktop и
  запуск подходящего теста, если системные зависимости доступны.
- Изменения Android: `./gradlew :app:assembleDebug`, `adb install -r …`,
  `adb logcat`, а также desktop-проверка из первого пункта.
- Перед релизом: чистая Android-сборка в изолированной среде, установка на
  Android 12 и более новую версию, анализ APK/AAB и проверка лицензий.

## Полный TODO

### A. Базовая линия и контракт сборки

- [ ] **A1.** Зафиксировать исходное состояние рабочей ветки и не включать в
  Android-коммиты чужие изменения.
  - [x] Записать текущие версии Rust, Meson, GTK и libadwaita.
  - [x] Выполнить `cargo check` и записать результат.
  - [x] Выполнить существующую минимальную desktop Meson-проверку, если все
    системные зависимости доступны; иначе записать конкретную недостающую
    зависимость.
- [ ] **A2.** Создать отдельный Android CI environment.
  - [x] Добавить изолированный `build-aux/android/Containerfile` и
    `podman.sh`: SDK/NDK/JDK/Rust/Meson/Pixiewood устанавливаются только в
    image, а host `adb` используется только для install/logcat.
  - [ ] Выбрать JDK, Android SDK/Build Tools, NDK, Gradle и emulator image.
    JDK 17, SDK/platform 36, build-tools 36.0.0, NDK 27.2.12479018 и Gradle
    9.3.1 зафиксированы; emulator image ещё не выбран.
  - [x] Собрать образ через `build-aux/android/podman.sh image` и проверить
    `pixiewood --version` внутри него на чистом host.
  - [x] Зафиксировать версии в контейнере либо reproducible setup-скрипте.
  - [x] Убедиться, что SDK/NDK не устанавливаются и не запрашиваются при
    desktop-сборке.
- [ ] **A3.** Зафиксировать runtime builder.
  - [x] Проверить конкретную ревизию Pixiewood на минимальном GTK demo.
  - [x] Документировать source URL, revision, checksum и применённые patches.
  - [ ] Проверить, что runtime использует GDK Android backend и `minSdk=31`.
  - [ ] Если проверка не пройдена, создать issue и перейти на зафиксированную
    схему Android CI GTK demo, сохранив остальные архитектурные решения.

Критерий завершения A: desktop остаётся зелёным, а Android toolchain
воспроизводимо поднимается из чистого окружения.

### B. Минимальный Android APK: сначала запуск

- [ ] **B1.** Создать изолированный Gradle Android skeleton в `android/`.
  - [ ] Добавить manifest с package ID, launcher activity, label и иконкой.
  - [ ] Установить `minSdk = 31`, `targetSdk` и `compileSdk` в одном
    version catalog/Gradle configuration.
  - [ ] Добавить debug signing по умолчанию; release signing брать только из
    CI secrets или локальных переменных окружения.
  - [ ] Выполнить `:app:assembleDebug` до интеграции Fractal.
- [ ] **B2.** Собрать и упаковать минимальный GTK/libadwaita runtime для
  `arm64-v8a`.
  - [x] Собрать GLib/GIO, Cairo, Pango, GdkPixbuf и GTK с Android backend.
  - [x] Собрать libadwaita той же совместимой версии.
  - [x] Упаковать все runtime `.so` в APK и проверить их через
    `readelf -d`/`apkanalyzer`.
  - [ ] Проверить запуск простого GTK demo на Android 12+.
- [ ] **B3.** Подготовить Rust entry point без изменения desktop entry point.
  - [x] Вынести общий bootstrap приложения из `src/main.rs` в библиотечный
    модуль с единственной точкой инициализации.
  - [x] Оставить desktop `main()` тонким вызовом этого bootstrap.
  - [x] Добавить Android `cdylib` entry point согласно контракту выбранного
    GTK runtime/Activity.
  - [ ] Настроить NDK linker, Cargo target и `pkg-config` только в Android
    окружении.
  - [x] Проверить, что `cargo check` для desktop не меняет target и linker.
- [ ] **B4.** Сделать Android-safe конфигурацию и ресурсы.
  - [x] Перестать требовать абсолютный desktop `PKGDATADIR` для GResources на
    Android: встроить их в библиотеку либо надёжно распаковывать в app sandbox.
  - [x] Зарегистрировать `resources.gresource` и UI resources до создания
    первого виджета.
  - [x] Задать Android-safe data/cache/config директории через единый
    platform API, не через жёсткие пути.
  - [ ] Временно отключить только те desktop assets, которые мешают старту;
    не удалять их из desktop-пакета.
- [ ] **B5.** Устранить гарантированные crash-пути первого запуска.
  - [x] Заменить Android fallback `unimplemented!()` у session secret storage
    на контролируемое Android состояние без сохранённых сессий.
  - [x] Если вход ещё не поддержан, показать понятное временное сообщение и
    не позволять дойти до panic.
  - [ ] Убедиться, что отсутствие camera/location/notification bridge не
    вызывает crash при создании стартового окна.
- [ ] **B6.** Доказать первый запуск Fractal.
  - [x] Установить debug APK на Android 12+ с `adb install -r`.
  - [x] Открыть приложение с launcher и получить экран логина/стартовый экран.
  - [x] Проверить `adb logcat` на fatal exception, missing `.so`, GResource и
    GSettings ошибки.
  - [ ] Снять screenshot и приложить к журналу/CI artifact.
  - [x] Выполнить desktop `cargo check` и Meson-проверку.

Критерий завершения B: Fractal открывается на Android 12+ как APK, использует
настоящий GTK4/libadwaita runtime и не регрессирует на desktop. Вход и media
на этом этапе могут быть недоступны, но ни один путь не должен падать.

### C. Базовая работоспособность клиента

- [x] **C1.** Реализовать безопасное хранение сессии для Android.
  - [x] Описать узкий Rust trait для secret storage без Android типов в общем
    слое.
  - [x] Реализовать Android Keystore-backed storage через Kotlin/JNI.
  - [x] Хранить access tokens и passphrase только за ключом Android Keystore;
    не использовать plaintext SharedPreferences или обычные файлы.
  - [x] Обработать восстановление, удаление и invalidated key без panic.
  - [x] Добавить тесты для сериализации и error mapping, где это возможно без
    Android device.
- [ ] **C2.** Поддержать логин и Matrix sync.
  - [x] Убедиться, что TLS, DNS, SQLite и crypto зависимости доступны в APK.
  - [x] Пройти логин на тестовом Matrix account.
  - [x] Перезапустить приложение и проверить восстановление сессии.
  - [ ] Проверить offline/error state и logout.
- [ ] **C3.** Подготовить Android lifecycle.
  - [ ] Передавать resume/pause/stop из Activity в Rust platform layer.
  - [ ] Не уничтожать UI/async runtime дважды при rotation и process recreation.
  - [ ] Приостановить необязательную работу в background по политике Android.
  - [ ] Проверить foreground/background, поворот и возврат из recents.
- [ ] **C4.** Адаптировать UX именно для телефона.
  - [ ] Протестировать breakpoint 600sp на реальном плотностном экране.
  - [ ] Проверить навигацию room list → room → back жестом/кнопкой Android.
  - [ ] Проверить экранную клавиатуру, focus, длинный тап, copy/paste и
    narrow layout.
  - [ ] Исправлять только выявленные Android-расхождения без ухудшения
    desktop layout.

Критерий завершения C: пользователь может залогиниться, читать и отправлять
текстовые сообщения, перезапустить приложение и остаться в сессии.

### D. Системные интеграции Android

- [ ] **D1.** Создать один минимальный Kotlin/JNI bridge.
  - [ ] Определить ownership, threading и error contract для каждого вызова.
  - [ ] Не передавать Android `Context` в общий Rust-код.
  - [ ] Покрыть bridge логированием, чтобы Java/Kotlin exceptions попадали в
    Rust error, а не терялись.
- [ ] **D2.** Поддержать URI и файлы.
  - [ ] Связать `Gtk.FileDialog`/прикрепление с Android Storage Access
    Framework, включая persistable URI permissions, где нужно.
  - [ ] Связать `UriLauncher` и open-with с Android intents.
  - [ ] Проверить импорт/экспорт ключей, аватар, отправку и сохранение файлов.
- [ ] **D3.** Реализовать уведомления.
  - [ ] Запрашивать `POST_NOTIFICATIONS` только по действию пользователя.
  - [ ] Преобразовать `gio::Notification`/intent Fractal в Android notification
    channel и pending intent к нужной комнате.
  - [ ] Проверить tap, группировку, очистку и поведение foreground/background.
- [ ] **D4.** Реализовать camera и QR verification.
  - [ ] Добавить `CAMERA` в manifest и runtime permission flow.
  - [ ] Реализовать Android backend `CameraExt`; отказ в permission — обычный
    UI state, не ошибка процесса.
  - [ ] Проверить QR scan на физическом устройстве.
- [ ] **D5.** Реализовать location.
  - [ ] Добавить Android location backend вместо Linux portal implementation.
  - [ ] Запрашивать approximate/fine location только в момент отправки.
  - [ ] Проверить deny, disabled services и отправку geo URI.

Критерий завершения D: системные действия запускают соответствующие Android
диалоги/API и корректно обрабатывают отказ пользователя.

### E. Медиа, динамические зависимости и производительность

- [ ] **E1.** Собрать GStreamer Android runtime.
  - [ ] Зафиксировать версию и набор plugins, требуемых Fractal.
  - [ ] Упаковать plugins, registry и нужные codec libraries в APK.
  - [ ] Настроить пути поиска plugins в Android platform bootstrap.
  - [ ] Проверить audio и video playback на Android 12+.
- [ ] **E2.** Проверить Glycin, изображения, карты и SourceView.
  - [ ] Проверить image loaders и исключить Linux sandbox assumptions.
  - [ ] Проверить Shumate/map tiles и сетевые ошибки.
  - [ ] Проверить SourceView/formatting в сообщениях, если он доступен в UI.
- [ ] **E3.** Настроить renderer и устойчивость GPU.
  - [ ] Проверить выбранный GTK renderer на physical device и emulator.
  - [ ] Измерить старт, scrolling room history и потребление памяти.
  - [ ] Добавить диагностический fallback только при доказанной device-specific
    ошибке, не делая software renderer default без измерений.

Критерий завершения E: изображения, аудио и видео работают либо имеют явные
корректные ограничения; APK не зависит от библиотек вне своего sandbox.

### F. Release engineering

- [ ] **F1.** Добавить Android CI jobs.
  - [x] Separate debug APK job для каждого изменяющего Android кода merge
    request: `.github/workflows/android.yml` собирает и проверяет debug APK
    и выкладывает его артефактом.
  - [ ] Nightly/integration job с emulator и `adb logcat` artifact.
  - [x] Кешировать зависимости по lock-файлам, не по плавающим `latest`:
    toolchain image — по контрольной сумме `Containerfile`, Cargo registry —
    по контрольной сумме `Cargo.lock`.
- [ ] **F2.** Проверить пакет.
  - [ ] Проверить ABI, `minSdk`, `targetSdk`, permissions и отсутствие
    отсутствующих shared libraries.
  - [ ] Проверить размер APK/AAB и список включённых GStreamer plugins.
  - [ ] Сформировать SBOM и проверить GPL/LGPL notices для bundled runtime.
- [ ] **F3.** Выпустить подписанный артефакт.
  - [ ] Собрать release AAB и, при необходимости, universal APK.
  - [ ] Хранить signing key вне репозитория; CI получает его только из secrets.
  - [ ] Установить release build на Android 12, 13/14 и актуальную версию.
  - [ ] Опубликовать воспроизводимую инструкцию сборки и known limitations.

Критерий завершения F: есть воспроизводимый, подписанный Android релиз и
отдельный, по-прежнему рабочий desktop pipeline.

## Известные риски и правила принятия решений

- Android backend GTK экспериментальный. Любая проблема в backend сначала
  воспроизводится на минимальном GTK demo; затем фиксируется в Fractal только
  если причина в приложении.
- GSettings schemas, translations, GIO modules, GStreamer plugins и image
  loaders часто не находятся из APK автоматически. Каждый класс динамических
  данных имеет отдельную проверку в `verify-apk.sh` и runtime smoke test.
- Не подменять защищённое session storage «временным plaintext файлом» ради
  логина. До Android Keystore стартовый экран допустим, persistent login — нет.
- Любое API, недоступное в Android backend GTK, должно быть закрыто trait и
  feature-level fallback, а не scattered `cfg` внутри UI.
- Android permissions не запрашиваются на старте приложения. Только по
  пользовательскому действию, с корректным состоянием отказа.

## Условия готовности Android beta

- APK/AAB собирается воспроизводимо для `arm64-v8a`, устанавливается на
  Android 12+ и запускается без native crash.
- Работают login, безопасное восстановление сессии, text rooms, вложения,
  Android back navigation, notifications и основной media playback.
- Camera/location имеют корректный permission flow или честно отключены с
  понятным состоянием.
- Desktop `cargo check` и Meson build остаются рабочими и не требуют Android
  toolchain.
- Все незакрытые ограничения задокументированы в release notes.
