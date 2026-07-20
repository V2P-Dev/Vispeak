# Vispeak

Vispeak is a desktop application for local offline voice-to-text dictation triggered by a global hotkey.

[Русская версия ниже](#russian-version)

## Features
- 🎙️ **Completely Offline**: Audio is never sent to any servers. Everything is processed locally using Whisper models.
- ⚡ **Global Hotkey**: Press `Ctrl+Space` (default) in any app, speak your text, release the keys — and the text is pasted.
- 🎨 **Modern UI**: Dark interface built with Tauri 2 and React.
- 📦 **Choose your Model**: Use lightweight models for speed or larger ones for accuracy. Models are downloaded on-demand and are not bundled with the app.

## 🤖 Speech Recognition (Models)
The application supports various models for transcription. **These are not bundled with the app and are downloaded automatically from Hugging Face only upon your explicit request in the settings.**

| Model Family | License | Source (Hugging Face) | Description |
|--------------|---------|-----------------------|-------------|
| **Whisper** | MIT | `ggerganov/whisper.cpp` | Base models from OpenAI |
| **Parakeet TDT / Canary v2** | CC-BY-4.0 | `istupakov/...` | High-accuracy models from NVIDIA |
| **GigaAM v3** | MIT | `istupakov/gigaam-v3-onnx` | Best for Russian language (SberDevices) |
| **Nemotron / Qwen** | Various | `handy-computer/...` | Models in GGUF format |

For detailed information about licenses and model sources, see [THIRD_PARTY_LICENSES.md](./THIRD_PARTY_LICENSES.md).

## 🔒 Privacy
- **Local Processing**: All speech recognition is performed entirely locally on your device. Audio and transcribed text are never sent to any external servers.
- **Local History**: Dictation history is stored locally in `%LOCALAPPDATA%/app.vispeak` and is fully managed by the user (you can set storage limits or clear it completely).
- **Network Requests**: The app makes only two types of network requests: checking for and downloading updates (GitHub Releases), and downloading models from Hugging Face (only upon explicit user action). There is absolutely no telemetry or analytics.

## 💖 Support the Project
If you like Vispeak and want to support its development, you can do so here:
- [Boosty](https://boosty.to/v2p/donate)
- [DaLink](https://dalink.to/v2p)

Crypto wallets:
- **BTC**:
  ```text
  12tSjndfTjfttXsckBqQwbrZZADWbEeiLi
  ```
- **USDT (ERC20)**:
  ```text
  0xeff9305f8f48261c3f4b3990306bece26788a04c
  ```
- **USDT (TRC20)**:
  ```text
  TCVzqHNmYq9KZRbH3GcZgWNnQeet1hFckp
  ```

## Requirements (Windows)
- Windows 10/11 (64-bit)
- To build from source, you need Rust, Node.js, LLVM, and MSVC Build Tools installed.

## Usage
1. Launch the app. It will appear in the system tray.
2. In the settings window, go to the **Model** tab and download your preferred model.
3. Click **Select** to make it the active model.
4. Open any text editor or input field (Notepad, browser, messenger).
5. Press and hold `Ctrl+Space` (or your chosen hotkey) to start dictating. A small floating widget will appear.
6. Release the keys. The transcribed text will be automatically pasted into the active window.
7. You can cancel dictation by pressing `Esc` while recording.

## Build from Source

```bash
# Clone the repository
git clone https://github.com/ViPunch/Vispeak.git
cd Vispeak

# Install dependencies
npm install

# Run in dev mode
npm run tauri dev

# Build the release binary
npm run tauri build
```

## License
Vispeak is licensed under the MIT License. See [LICENSE](./LICENSE) for details.
Information about third-party libraries and models is available in [THIRD_PARTY_LICENSES.md](./THIRD_PARTY_LICENSES.md).

---

<a id="russian-version"></a>
# Vispeak

Vispeak — desktop-приложение для локальной офлайн-транскрибации голоса по глобальной горячей клавише.

## Особенности
- 🎙️ **Полностью офлайн**: Аудио не отправляется на сторонние серверы, всё обрабатывается локально на вашем компьютере с помощью моделей Whisper.
- ⚡ **Глобальный хоткей**: Нажмите `Ctrl+Space` (по умолчанию) в любом приложении, произнесите текст, отпустите клавишу — и текст будет вставлен.
- 🎨 **Современный UI**: Темный интерфейс на базе Tauri 2 и React.
- 📦 **Модели на выбор**: Используйте легкие модели для скорости или более крупные для точности распознавания. Все модели загружаются отдельно от приложения по запросу пользователя.

## 🤖 Распознавание речи (Модели)
Приложение поддерживает различные модели для транскрибации. **Они не входят в состав приложения и скачиваются автоматически с Hugging Face только по вашему явному запросу в настройках.**

| Семейство моделей | Лицензия | Источник (Hugging Face) | Описание |
|-------------------|----------|-------------------------|----------|
| **Whisper** | MIT | `ggerganov/whisper.cpp` | Базовые модели от OpenAI (через whisper.cpp) |
| **Parakeet TDT / Canary v2** | CC-BY-4.0 | `istupakov/...` | Модели от NVIDIA с высокой точностью |
| **GigaAM v3** | MIT | `istupakov/gigaam-v3-onnx` | Лучший выбор для русского языка (SberDevices) |
| **Nemotron / Qwen** | Разные | `handy-computer/...` | Модели в формате GGUF |

Подробную информацию о лицензиях и источниках моделей см. в [THIRD_PARTY_LICENSES.md](./THIRD_PARTY_LICENSES.md).

## 🔒 Конфиденциальность (Privacy)
- **Локальная обработка**: Всё распознавание речи выполняется исключительно локально на вашем устройстве. Аудио и распознанный текст никогда не отправляются ни на какие серверы.
- **Локальная история**: История диктовок хранится локально в `%LOCALAPPDATA%/app.vispeak` и полностью управляется пользователем (можно настроить лимит хранения или полностью очистить).
- **Сетевые обращения**: Приложение выполняет только два вида сетевых запросов: проверка и скачивание обновлений (GitHub Releases) и скачивание моделей с Hugging Face (только по явному действию пользователя). Телеметрия и аналитика полностью отсутствуют.

## 💖 Поддержать проект
Если вам нравится Vispeak и вы хотите поддержать разработку, вы можете сделать это по ссылкам ниже:
- [Boosty](https://boosty.to/v2p/donate)
- [DaLink](https://dalink.to/v2p)

Криптокошельки:
- **BTC**:
  ```text
  12tSjndfTjfttXsckBqQwbrZZADWbEeiLi
  ```
- **USDT (ERC20)**:
  ```text
  0xeff9305f8f48261c3f4b3990306bece26788a04c
  ```
- **USDT (TRC20)**:
  ```text
  TCVzqHNmYq9KZRbH3GcZgWNnQeet1hFckp
  ```

## Требования (Windows)
- Windows 10/11 (64-bit)
- Если вы собираете из исходников, потребуется установленный Rust, Node.js, LLVM и MSVC Build Tools.

## Установка и использование
1. Запустите приложение. Оно появится в системном трее.
2. В окне настроек перейдите на вкладку **Модель** и скачайте нужную модель.
3. Нажмите кнопку **Выбрать**, чтобы сделать модель активной.
4. Откройте любой текстовый редактор или поле ввода (Блокнот, браузер, мессенджер).
5. Нажмите и удерживайте `Ctrl+Space` (или выбранную вами горячую клавишу) для начала диктовки. Появится небольшой плавающий виджет.
6. Отпустите клавиши. Распознанный текст будет автоматически вставлен в активное окно.
7. Вы можете отменить диктовку, нажав `Esc` во время записи.

## Сборка из исходников

```bash
# Клонируйте репозиторий
git clone https://github.com/ViPunch/Vispeak.git
cd Vispeak

# Установите зависимости
npm install

# Запустите в режиме разработчика
npm run tauri dev

# Соберите релизный бинарник
npm run tauri build
```

## Лицензия
Vispeak распространяется под лицензией MIT. Подробности в файле [LICENSE](./LICENSE).
Информация о сторонних библиотеках и моделях доступна в [THIRD_PARTY_LICENSES.md](./THIRD_PARTY_LICENSES.md).
