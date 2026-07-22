## What's changed
**Fixed**
- **Overlay Positioning on First Dictation After Model Unload**: Fixed an issue where the "At Caret" (mini) overlay skin appeared at the fallback position (bottom center) on the first dictation after auto-unloading the model from memory. Caret position is now captured immediately upon pressing the hotkey before any model loading or audio thread initialization.

**Improved**
- **Theme-Aware Overlay**: All overlay skins (Full, Compact, and At Caret) now dynamically respect the chosen application theme (Light/Dark/System) rather than being fixed to a dark theme.
- **Pulsing Glow Animation**: Added a soft, pulsing breathing animation to the overlay's accent glow during active dictation.
- **System Tray Tooltip**: The system tray icon now dynamically displays the active model name and current status (e.g. `GigaAM v3 E2E — Ready` / `GigaAM v3 E2E — Standby`) matching the status indicator state.
- **Settings UI**: Renamed the memory management setting to "Unload model from memory" ("Выгружать модель из памяти") for better readability.

---
📥 Download the installer from the Assets below.
ℹ️ First time here? See the [README](https://github.com/V2P-Dev/Vispeak#readme)
for features, privacy, and installation.
