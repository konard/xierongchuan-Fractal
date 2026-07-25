# План Android-порта Fractal

Статус: **в работе** (этапы A/B2 закрыты: Android toolchain воспроизводимо
собирает и упаковывает GTK4/libadwaita APK; B3–B5 закрыты со стороны кода:
entry point, ресурсы, директории и crash-пути готовы к Android, Meson-цель и
Pixiewood-манифест Fractal добавлены. Сама сборка APK Fractal блокируется
отсутствующими Android-wrap'ами зависимостей — см. «Текущие блокеры»)
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
| 2026-07-25 | A2.1 | working tree | `git diff --check` | Added the isolated Podman Android toolchain definition and host wrapper; image build is still pending. |
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

### Текущие блокеры для APK самого Fractal

Toolchain доказан на минимальном GTK4/libadwaita приложении
(`experiments/android-gtk-smoke`). Прежде чем через него пройдёт сам Fractal,
нужно закрыть три конкретных блокера:

1. Pixiewood предоставляет wrap'ы только для glib, fontconfig, cairo,
   gdk-pixbuf, gtk, harfbuzz, libadwaita и rsvg. Обязательные зависимости
   `meson.build` Fractal — gstreamer-*, gtksourceview-5, glycin-2,
   glycin-gtk4-2, libwebp, shumate-1.0, sqlite3 — Android-сборки не имеют
   (пункты E1 и E2).
2. ~~Бинарь Fractal производится cargo через Meson `custom_target`, а Pixiewood
   требует Meson-цель `executable(..., android_exe_type: 'application')` с
   `main(int, char**, char**)`, вызывающей `g_application_run` (пункт B3).~~
   Закрыто: при `-Dandroid=true` Meson собирает `android/native` как
   `cdylib` и линкует с ним C-launcher `android/shim/main.c` через
   `executable(..., android_exe_type: 'application')`. Сама сборка этой цели
   ещё не выполнялась, потому что её блокирует пункт 1.
3. Rust-зависимости aperture, ashpd и oo7 — только Linux (пункты D4, D5).
   Android-ветка secret storage больше не `unimplemented!()`: она возвращает
   состояние «нет сохранённых сессий» и понятную ошибку при попытке сохранить
   сессию, пока не подключён Android Keystore (пункты B5, C1).

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
  - [ ] Установить debug APK на Android 12+ с `adb install -r`.
  - [ ] Открыть приложение с launcher и получить экран логина/стартовый экран.
  - [ ] Проверить `adb logcat` на fatal exception, missing `.so`, GResource и
    GSettings ошибки.
  - [ ] Снять screenshot и приложить к журналу/CI artifact.
  - [ ] Выполнить desktop `cargo check` и Meson-проверку.

Критерий завершения B: Fractal открывается на Android 12+ как APK, использует
настоящий GTK4/libadwaita runtime и не регрессирует на desktop. Вход и media
на этом этапе могут быть недоступны, но ни один путь не должен падать.

### C. Базовая работоспособность клиента

- [ ] **C1.** Реализовать безопасное хранение сессии для Android.
  - [ ] Описать узкий Rust trait для secret storage без Android типов в общем
    слое.
  - [ ] Реализовать Android Keystore-backed storage через Kotlin/JNI.
  - [ ] Хранить access tokens и passphrase только за ключом Android Keystore;
    не использовать plaintext SharedPreferences или обычные файлы.
  - [ ] Обработать восстановление, удаление и invalidated key без panic.
  - [ ] Добавить тесты для сериализации и error mapping, где это возможно без
    Android device.
- [ ] **C2.** Поддержать логин и Matrix sync.
  - [ ] Убедиться, что TLS, DNS, SQLite и crypto зависимости доступны в APK.
  - [ ] Пройти логин на тестовом Matrix account.
  - [ ] Перезапустить приложение и проверить восстановление сессии.
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
  - [ ] Separate debug APK job для каждого изменяющего Android кода merge
    request.
  - [ ] Nightly/integration job с emulator и `adb logcat` artifact.
  - [ ] Кешировать зависимости по lock-файлам, не по плавающим `latest`.
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
