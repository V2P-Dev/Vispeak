## What's changed
**Added**
- **Auto Unload Idle Model**: Added a setting on the General page to automatically unload the transcription model from RAM after 1, 5, or 15 minutes of inactivity, freeing memory when idle while keeping instant dictation as default ("Never"). Speech recorded while the model re-loads is fully buffered without loss.
- **Model Status Indicator**: Main window header indicator now dynamically reflects model memory state in real time: Green ("Ready") when loaded in RAM, Gray ("Standby") when idle/unloaded, and Cyan ("Loading...") during model re-loading.

**Fixed**
- **i18n**: Fixed missing English translation key for the "At Caret" (mini) overlay skin setting.

**Improved**
- **Overlay Equalizer**: Made the recording visualizer dynamic and responsive to real-time audio levels at 60 FPS, removing artificial symmetry and adding smooth attack/decay animation across all overlay skins.
- **Model Page UI**: Models are now grouped by status (Active, Downloaded, Available) for better clarity, with a dedicated "Recommended" tag on recommended models.
- **Updater**: Added a link to view the full release notes on GitHub directly from the update notification dialog.

---
📥 Download the installer from the Assets below.
ℹ️ First time here? See the [README](https://github.com/V2P-Dev/Vispeak#readme)
for features, privacy, and installation.
