# План Android-порта Fractal

Статус: **не начат**
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
  - [ ] Записать текущие версии Rust, Meson, GTK и libadwaita.
  - [ ] Выполнить `cargo check` и записать результат.
  - [ ] Выполнить существующую минимальную desktop Meson-проверку, если все
    системные зависимости доступны; иначе записать конкретную недостающую
    зависимость.
- [ ] **A2.** Создать отдельный Android CI environment.
  - [ ] Выбрать JDK, Android SDK/Build Tools, NDK, Gradle и emulator image.
  - [ ] Зафиксировать версии в контейнере либо reproducible setup-скрипте.
  - [ ] Убедиться, что SDK/NDK не устанавливаются и не запрашиваются при
    desktop-сборке.
- [ ] **A3.** Зафиксировать runtime builder.
  - [ ] Проверить конкретную ревизию Pixiewood на минимальном GTK demo.
  - [ ] Документировать source URL, revision, checksum и применённые patches.
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
  - [ ] Собрать GLib/GIO, Cairo, Pango, GdkPixbuf и GTK с Android backend.
  - [ ] Собрать libadwaita той же совместимой версии.
  - [ ] Упаковать все runtime `.so` в APK и проверить их через
    `readelf -d`/`apkanalyzer`.
  - [ ] Проверить запуск простого GTK demo на Android 12+.
- [ ] **B3.** Подготовить Rust entry point без изменения desktop entry point.
  - [ ] Вынести общий bootstrap приложения из `src/main.rs` в библиотечный
    модуль с единственной точкой инициализации.
  - [ ] Оставить desktop `main()` тонким вызовом этого bootstrap.
  - [ ] Добавить Android `cdylib` entry point согласно контракту выбранного
    GTK runtime/Activity.
  - [ ] Настроить NDK linker, Cargo target и `pkg-config` только в Android
    окружении.
  - [ ] Проверить, что `cargo check` для desktop не меняет target и linker.
- [ ] **B4.** Сделать Android-safe конфигурацию и ресурсы.
  - [ ] Перестать требовать абсолютный desktop `PKGDATADIR` для GResources на
    Android: встроить их в библиотеку либо надёжно распаковывать в app sandbox.
  - [ ] Зарегистрировать `resources.gresource` и UI resources до создания
    первого виджета.
  - [ ] Задать Android-safe data/cache/config директории через единый
    platform API, не через жёсткие пути.
  - [ ] Временно отключить только те desktop assets, которые мешают старту;
    не удалять их из desktop-пакета.
- [ ] **B5.** Устранить гарантированные crash-пути первого запуска.
  - [ ] Заменить Android fallback `unimplemented!()` у session secret storage
    на контролируемое Android состояние без сохранённых сессий.
  - [ ] Если вход ещё не поддержан, показать понятное временное сообщение и
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
