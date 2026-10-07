# Vispeak

Vispeak is a dictation app for Windows and Linux. Press a global hotkey, speak, and the app recognizes speech locally and inserts the result into the active text field.

Read this in: English | [Русский](README.ru.md)

## Features
- 🎙️ **Completely Offline**: Audio is never sent to any servers. Everything is processed locally using Whisper, Parakeet, Canary, GigaAM, Nemotron or Qwen3-ASR.
- ⚡ **Global Hotkey**: Press `Ctrl+Space` (default) in any app, speak your text, release the keys — and the text is pasted.
- 🎨 **Customizable Overlay**: Full, compact and mini widgets, spectrum styles and an accent color.
- 📝 **History and Controls**: Local history, retranscription, push-to-talk or toggle recording, cancellation and audio ducking.
- 🖥️ **Windows and Linux**: X11 and Wayland integration; Wayland hotkeys and insertion require desktop permissions.
- 📦 **Choose your Model**: Use lightweight models for speed or larger ones for accuracy. Models are downloaded on-demand and are not bundled with the app.

## Screenshots

<details>
<summary>Main Window — Model Selection</summary>

![Main Window — Model Selection](docs/screenshots/main-window.jpg)

</details>

<details>
<summary>Overlay Widget — Recording</summary>

![Overlay Widget — Recording](docs/screenshots/full-overlay-speak.jpg)

</details>

## 🤖 Speech Recognition (Models)
The application supports various models for transcription. **These are not bundled with the app and are downloaded automatically from Hugging Face only upon your explicit request in the settings.**

| Model Family | License | Source (Hugging Face) | Description |
|--------------|---------|-----------------------|-------------|
| **Whisper** | MIT | `ggerganov/whisper.cpp` | Base models from OpenAI |
| **Parakeet TDT / Canary v2** | CC-BY-4.0 | `istupakov/...` | High-accuracy models from NVIDIA |
| **GigaAM v3** | MIT | `istupakov/gigaam-v3-onnx` | Russian speech recognition (SberDevices) |
| **Nemotron 3.5 / Qwen3-ASR** | Various | `handy-computer/...` | Models in GGUF format |

For detailed information about licenses and model sources, see [THIRD_PARTY_LICENSES.md](./THIRD_PARTY_LICENSES.md).

## 🔒 Privacy
- **Local Processing**: All speech recognition is performed entirely locally on your device. Audio and transcribed text are never sent to any external servers.
- **Local History**: Dictation history is stored locally in `%LOCALAPPDATA%/app.vispeak` on Windows and `~/.local/share/app.vispeak` on Linux (or `$XDG_DATA_HOME/app.vispeak`) and is fully managed by the user (you can set storage limits or clear it completely).
- **Network Requests**: The app makes only two types of network requests: checking for and downloading updates (GitHub Releases), and downloading models from Hugging Face (only upon explicit user action). There is absolutely no telemetry or analytics.

## 💖 Support the Project
If you like Vispeak and want to support its development, you can do so here:
- [Boosty](https://boosty.to/v2p/donate)
- [DaLink](https://dalink.to/v2p)

## Installation

Download a build from **Assets** in the [latest release](https://github.com/V2P-Dev/Vispeak/releases/latest). All builds target x86_64.

### Windows 10/11

1. Download `Vispeak_<version>_x64-setup.exe` and run the installer. An `.msi` is also available.
2. Open Vispeak from the Start menu; settings are also accessible through its tray icon.
3. Installed Windows versions check for updates on startup and every five hours. To check immediately, use the updates section in settings and confirm installation of the offered version. You do not need to download `.sig` or `latest.json` manually.

### Ubuntu 24.04+ / Debian 13+

Download the `.deb` named for your distribution. In the download directory, run the appropriate command:

```bash
# Ubuntu
sudo apt install ./*ubuntu*.deb
# Debian
sudo apt install ./*debian*.deb
```

### Fedora 44

Download the Fedora `.rpm`, then run in the download directory:

```bash
sudo dnf install ./*fedora*.rpm
```

### Arch Linux

Update the system and install build tools:

```bash
sudo pacman -Syu --needed base-devel git
```

Download `v<version>-arch-PKGBUILD` from Assets, place it in a separate directory, rename it to `PKGBUILD`, review the recipe and run **without sudo**:

```bash
makepkg -si
```

The recipe builds the current `main` sources and installs `vispeak-git`; it is not published to the AUR. A prebuilt Arch `.tar.gz` is also available for manual installation; dependencies and build instructions are in the [Linux guide](docs/LINUX.md).

### First Launch on Linux

Start **Vispeak** from the application menu or run `vispeak`. On Wayland, approve portal requests for hotkeys and keyboard control. Check the actual system-assigned shortcut in Controls; use the system shortcut configuration button if needed.

You need `xdg-desktop-portal` and a desktop backend implementing GlobalShortcuts and RemoteDesktop, such as KDE. X11 uses `xdotool` and `xclip`. GNOME may need an AppIndicator extension for the tray icon. Starting with v1.4.0, installed Fedora RPM and Ubuntu/Debian DEB packages can check for updates in settings and install signed packages after a system administrator prompt. The original Linux v1.3.0 package needs one manual upgrade to enable this. Arch/source builds use the package manager.

**Application testing of v1.4.0 is limited to Fedora Linux 44 KDE Plasma Desktop Edition, Plasma 6.7.5 / KWin 6.7.5, Wayland.** Builds for other systems do not confirm runtime testing. The user confirmed working window buttons, caret placement, audio ducking, and particle clipping on Fedora KDE Wayland. See the [Linux guide](docs/LINUX.md) for further dependencies and limitations.

## Getting Started

1. Open **Model** in settings, download a model and click **Select**. Downloading requires internet; subsequent recognition works offline.
2. Open a text editor, browser or messenger and focus a text field.
3. In push-to-talk mode, hold `Ctrl+Space` (or the system-assigned hotkey), speak and release the keys. In toggle mode, press the shortcut again to stop.
4. The completed text is inserted into the active field. `Esc` cancels recording if assigned; Wayland may request additional portal approval.

Wayland inserts the completed result; live typing is disabled. Shortcuts must include a regular key; portals cannot distinguish left/right Ctrl, Alt or Shift.

## Build from Source

Windows requires Rust stable, Node.js 22+, LLVM and MSVC Build Tools. Linux system dependencies and dev setup are documented in the [Linux guide](docs/LINUX.md).

```bash
git clone https://github.com/V2P-Dev/Vispeak.git
cd Vispeak
npm ci
npm run tauri dev
```

Release build: `npm run tauri build` on Windows, `npm run build:linux -- --bundles deb,rpm -- --locked` on Linux.

## License
Vispeak is licensed under the MIT License. See [LICENSE](./LICENSE) for details.
Information about third-party libraries and models is available in [THIRD_PARTY_LICENSES.md](./THIRD_PARTY_LICENSES.md).
