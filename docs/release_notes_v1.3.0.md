## What's changed

**Added**
- **Linux support (x86_64)**: Ubuntu 24.04+ and Debian 13+ `.deb` packages, a Fedora 44 `.rpm`, and an Arch executable archive with a `PKGBUILD` build recipe.
- **Wayland desktop integration**: GlobalShortcuts and RemoteDesktop portals for dictation hotkeys and text insertion, with permission status, the actual system-assigned shortcut, and a shortcut configuration button.
- **Linux desktop features**: X11 hotkeys and insertion, clipboard restoration for text/PNG, audio ducking through PulseAudio/PipeWire, and a non-focusable layer-shell overlay where supported.
- **Caret positioning**: AT-SPI caret tracking with a fallback overlay position; an optional, separately built KWin bridge for the exact installed KWin version.

**Fixed**
- Hotkey capture now saves the current key combination reliably.
- Linux paste failures surface in the UI; insertion is blocked while a portal permission request is pending or the visible settings window has focus.
- Wayland startup works around WebKitGTK DMABUF protocol errors.

**Changed**
- Live typing is disabled on Wayland; the completed transcription is inserted into the currently focused field.
- Linux updates use distribution packages; the existing Windows updater remains available.

**Testing / known limitations**
- **Application testing for this release is limited to Fedora Linux 44 (KDE Plasma Desktop Edition), KDE Plasma 6.7.5 / KWin 6.7.5, Wayland.** Other distribution builds and Windows builds are not confirmation of runtime testing on those systems.
- Wayland requires a desktop portal backend implementing GlobalShortcuts and RemoteDesktop. Modifier-only shortcuts and separate left/right modifiers are unavailable through these portals.
- Overlay rounding during processing/results and visual caret placement still require confirmation. Caret tracking depends on the target application's accessibility/text-input support; the optional native KWin bridge is not included in the packages.
- See the [Linux guide](https://github.com/V2P-Dev/Vispeak/blob/v1.3.0/docs/LINUX.md) for dependencies and desktop limitations. The Arch archive requires those dependencies; its `PKGBUILD` builds from source and is not an AUR publication.

---
📥 Download the installer from the Assets below.
ℹ️ First time here? See the [README](https://github.com/V2P-Dev/Vispeak#readme) for features, privacy, and installation.
