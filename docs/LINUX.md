# Linux

Linux-порт для x86_64 включён в v1.3.0: `.deb` для Ubuntu/Debian, `.rpm` для Fedora и архив для Arch с PKGBUILD. Пользовательское тестирование этой версии ограничено Fedora Linux 44 KDE Plasma Desktop Edition, Plasma 6.7.5 / KWin 6.7.5, Wayland. Сборки CI не означают проверку работы приложения на других системах.

## Целевые системы

| Дистрибутив | Формат | Базовое требование |
| --- | --- | --- |
| Ubuntu | `.deb` | 24.04+ |
| Debian | `.deb` | 13+ |
| Fedora | `.rpm` | 44 (база сборки CI) |
| Arch | архив `.tar.gz`, `PKGBUILD` для сборки пакета | актуальная система после `pacman -Syu` |

Готовый статический ONNX Runtime из `ort-sys 2.0.0-rc.12` требует символы glibc 2.38+. На Ubuntu 22.04 Rust-код проверяется, но линковка падает с `__isoc23_strtol/strtoll/strtoull`. Для старых систем потребуется отдельно собранный совместимый ONNX Runtime. Версии зависимостей распознавания не понижались.

## Сеансы рабочего окружения

- **X11:** глобальные клавиши через `rdev`; вставка и проверка активного окна через `xdotool`; буфер через `xclip`. Различение левых и правых модификаторов сохранено.
- **Wayland:** системные порталы `GlobalShortcuts` и `RemoteDesktop` (только разрешение на клавиатуру, без захвата экрана); буфер через `wl-clipboard`. Нужно подтвердить системные диалоги. Для горячей клавиши нужен обычный ключ в дополнение к модификаторам; левый/правый Ctrl, Alt и Shift объединяются. Рабочее окружение может заменить предпочитаемое сочетание своим.
- На странице «Управление» показано фактическое сочетание, возвращённое порталом. `Ctrl+Space` в поле Vispeak — предпочитаемое сочетание; KDE может сохранить «Нет». Используйте «Настроить горячие клавиши в системе», назначьте сочетание и нажмите «Применить». Кнопка использует [ConfigureShortcuts](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html#org-freedesktop-portal-globalshortcuts-configureshortcuts), доступный в GlobalShortcuts v2. На более старом backend настройте сочетание средствами рабочего окружения. Изменения портала отражаются на странице автоматически.
- Портал отмены регистрируется только на время записи/обработки и закрывается после цикла. Рабочее окружение может показывать диалог подтверждения этой клавиши при регистрации. Регистрация отмены не блокирует события отпускания основного хоткея.
- Нужен backend порталов именно вашего рабочего окружения, например `xdg-desktop-portal-kde` или `xdg-desktop-portal-gnome`, с реализациями обоих интерфейсов. Одного `xdg-desktop-portal-gtk` недостаточно. Наличие Wayland само по себе не гарантирует поддержку порталов.
- При отказе в разрешении ошибка отображается на странице управления. После разрешения управления клавиатурой перезапустите приложение. После исправления глобальных клавиш измените сочетание в настройках для повторной регистрации.
- Перед вставкой проверяется, что окно Vispeak не активно и нет открытого запроса разрешения портала. Текст не вводится в системные диалоги разрешений. В X11 дополнительно проверяется исходное окно; при переключении фокуса текст остаётся в истории, если её хранение включено. Wayland не раскрывает идентификатор чужого активного окна: готовый текст поступает в текущее поле, а потоковая посимвольная вставка отключена.
- В KDE и других Wayland-композиторах с layer-shell оверлей использует отдельную поверхность с запретом клавиатурного фокуса и привязкой к краю экрана. Режим «у курсора» запрашивает координаты текстовой каретки через AT-SPI; если приложение их не предоставляет, мини-оверлей отображается в резервной позиции из настроек. В GNOME без layer-shell позиционирование обычного окна определяет композитор. Фокусируемость оверлея выключена через GTK и Tauri. Прозрачность в текущем Fedora-сеансе ещё проверяется.
- Восстановление буфера сохраняет текст или PNG. Исходный набор нескольких MIME-форматов (например, форматирование HTML) целиком не сохраняется. Неизвестный формат отменяет вставку при политике восстановления. Новое содержимое, скопированное пользователем во время задержки, не перезаписывается.
- Приглушение звука использует `pactl` и PulseAudio либо PipeWire с `pipewire-pulse`; восстанавливаются отдельные потоки, собственный процесс исключён.
- Автозапуск использует существующий Tauri-плагин. Рабочее окружение может запрашивать управление клавиатурой при каждом запуске; тихий старт не подавляет системный диалог разрешений. Обновления Linux устанавливаются пакетным менеджером или новым пакетом со страницы релизов; Windows-манифест автообновления не применяется.
- В GNOME значок трея может потребовать расширение AppIndicator. Автозапуск и закрытие окна следует проверять с доступным треем.

Настройки, модели и история: `~/.local/share/app.vispeak` (или `$XDG_DATA_HOME/app.vispeak`). Данные Windows не перемещаются автоматически.

## Инструменты сборки

Нужны Rust stable (через rustup или пакет дистрибутива) и Node.js 22+ с npm. Системные команды:

Ubuntu / Debian:

```bash
sudo apt update
sudo apt install build-essential cmake clang libclang-dev pkg-config libgtk-3-dev libgtk-layer-shell-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf libssl-dev libasound2-dev libx11-dev libxtst-dev libxi-dev xdotool xclip wl-clipboard pulseaudio-utils
```

Fedora:

```bash
sudo dnf install nodejs npm rust cargo rustfmt gcc-c++ cmake clang clang-devel llvm-devel pkgconf-pkg-config gtk3-devel gtk-layer-shell-devel webkit2gtk4.1-devel libayatana-appindicator-gtk3-devel gtk-layer-shell-devel librsvg2-devel patchelf openssl-devel alsa-lib-devel libX11-devel libXtst-devel libXi-devel xdotool xclip wl-clipboard pulseaudio-utils rpm-build
```

Arch:

```bash
sudo pacman -Syu --needed base-devel rust nodejs npm clang cmake pkgconf gtk3 gtk-layer-shell webkit2gtk-4.1 libappindicator-gtk3 librsvg patchelf openssl alsa-lib libx11 libxtst libxi xdotool xclip wl-clipboard libpulse
```

Также установите `xdg-desktop-portal` и backend вашего окружения.

```bash
npm ci
cargo check --locked --manifest-path src-tauri/Cargo.toml
npm run tauri dev
npm run build:linux -- --bundles deb,rpm -- --locked
```

Для Wayland при запуске из исходников нужен desktop-файл, по которому портал идентифицирует приложение. Установите его один раз для своего пользователя:

```bash
mkdir -p ~/.local/share/applications
sed "s|^Exec=.*|Exec=\"$PWD/src-tauri/target/debug/vispeak\"|; s|^Icon=.*|Icon=$PWD/src-tauri/icons/128x128.png|" packaging/linux/app.vispeak.desktop > ~/.local/share/applications/app.vispeak.desktop
```

Vispeak регистрирует `app.vispeak` через [Registry](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.host.portal.Registry.html) до запросов порталов. Пакетные сборки также включают этот файл; он скрыт из меню, чтобы не дублировать основной launcher.

В desktop-файле dev-версии должен быть абсолютный путь к бинарнику: неустановленный `Exec=vispeak` может привести к `App info not found`. Команду выполняйте из корня проекта. Бинарник появится при первой dev-сборке.

На Fedora 44 KDE запуск с DMABUF-рендерером WebKit выявил `Error 71 (Protocol error)` / `Error flushing display`. В Wayland Vispeak теперь по умолчанию выставляет `WEBKIT_DISABLE_DMABUF_RENDERER=1` до инициализации GTK/WebKit. Обычная команда запуска:

```bash
npm run tauri dev
```

Это обход [ошибки WebKit/Wayland](https://bugs.webkit.org/show_bug.cgi?id=324551), который может уменьшить производительность рендеринга. X11 не меняется. Явно заданная пользователем переменная имеет приоритет; `WEBKIT_DISABLE_DMABUF_RENDERER=0` возвращает стандартный рендерер. Сообщения Vite/esbuild `The service is no longer running` после падения GTK могут быть следствием остановки dev-сервера.

Команда `build:linux` выключает `GGML_NATIVE` и фиксированные AVX/AVX2/FMA/F16C-оптимизации для переносимого базового x86_64-бинарника. Сборка использует отдельный Cargo target-каталог `src-tauri/target/linux`: whisper-rs-sys не отслеживает изменения этих флагов окружения в кеше Cargo. Не используйте обычную `tauri build` для распространяемых Linux-пакетов: по умолчанию C++-движки могут включить `-march=native`.

`tauri.linux.conf.json` автоматически объединяется с основным конфигом только в Linux. Ключ подписания не нужен. Результат: `src-tauri/target/linux/release/bundle/deb/` и `rpm/`.

Arch: после публикации исходников используйте `cd packaging/arch && makepkg -si`. `PKGBUILD` получает исходники из upstream Git, а не незакоммиченные изменения рабочего дерева. Для проверки текущего рабочего дерева на Arch можно скопировать его в `packaging/arch/src/vispeak` без `node_modules` и `src-tauri/target`, затем выполнить `makepkg --noextract`; каталог должен содержать Git-метаданные для `pkgver()`.

## Проверки

CI `.github/workflows/linux.yml` проверяет Rust, тесты, X11-буфер в Xvfb и сборку на Ubuntu, Debian, Fedora и Arch. Workflow загружает артефакты сборки и не публикует релизы.

```bash
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
xvfb-run -a cargo test --locked --manifest-path src-tauri/Cargo.toml --lib paste::tests -- --ignored --test-threads=1
```

Тест X11 требует `xclip`, Xvfb и xauth. Он проверяет Unicode, переносы строк и восстановление PNG в отдельном дисплее. Не запускайте его на личном дисплее: тест меняет буфер.

Ручная проверка на каждом окружении:

1. Установить пакет, запустить Vispeak, подтвердить Wayland-разрешения и выбрать скачанную модель.
2. Свернуть Vispeak, поставить курсор в редактор, проверить toggle и Push-to-Talk, отпускание клавиш и отмену. Оверлей не должен уводить курсор.
3. Надиктовать русский текст, проверить вставку, пробел после текста и автоотправку. В X11 переключить окно во время распознавания — автоматическая вставка должна отмениться.
4. Проверить восстановление обычного текста и изображения в буфере, режим «оставлять» и «только копировать».
5. Проверить микрофон, звуковые сигналы, парное приглушение/восстановление, автозапуск и выход из трея.
6. В Wayland отклонить разрешение, проверить понятную ошибку, затем разрешить и повторить после перезапуска.

Фактические результаты локальных проверок записываются в `docs/PLAN.md`. Поддержку всех окружений нельзя считать подтверждённой одним успешным компилированием.

Справочники: [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/), [GlobalShortcuts](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.GlobalShortcuts.html), [RemoteDesktop](https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.RemoteDesktop.html).


### Серый прямоугольник в KDE с Better Blur DX

Если HTML/WebKit прозрачны, но виден серый шумовой фон всей поверхности, проверьте Better Blur DX → Force Blur. В режиме «Blur all except matching» добавьте отдельной строкой `vispeak` в исключения. GTK может сохранять общий класс приложения даже при namespace layer-shell `vispeak-overlay`; в таком случае исключение namespace не сработает. Vispeak не меняет настройки эффектов KDE автоматически.

Основной системный источник каретки на Linux — AT-SPI. В KDE Wayland подготовлен дополнительный источник: модуль KWin читает геометрию, полученную композитором через Wayland text-input. Модуль требует сборки для точной версии KWin; dev-версия обнаруживает его в `src-tauri/target/kwin-caret/Vispeak/CaretBridge`, production-версия подключает его через `VISPEAK_KWIN_CARET_MODULE`. В KDE 6.7.5 временная проверка получила реальную каретку 1×20 пикселей; путь записи Vispeak затем получил координаты каретки и в Chrome, и в Codex. Пользователь подтвердил размещение по центру каретки; вертикальный отступ скорректирован, чтобы окно не перекрывало её. Подробности и команды сборки: [мост каретки KWin](../packaging/linux/kwin-caret/README.md). В KDE Wayland активное окно и начало его клиентской области запрашиваются у KWin временным скриптом; скрипт удаляется после запроса и не меняет настройки композитора. Для процесса активного окна оконные координаты каретки переводятся в экранные. Для accessibility-мостов другого процесса проверяются активность дерева и принадлежность экранных координат активному окну. Используются только координаты каретки видимого редактируемого поля в активном окне; границы поля не заменяют каретку. Мини-оверлей расположен по центру над кареткой: нижний край всего окна, включая прозрачный отступ для свечения, находится на 8 пикселей выше каретки. Если координаты недоступны или сверху недостаточно места, мини-оверлей остаётся видимым в резервной позиции из настроек. Это резервный режим, а не точное размещение над кареткой.
