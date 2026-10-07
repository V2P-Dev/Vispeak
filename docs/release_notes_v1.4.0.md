## What's changed

**Fixed**
- Window title-bar buttons respond immediately on Linux Wayland without first maximizing or resizing the window.
- Settings use an opaque background independently of the transparent dictation overlays. Desktop compositor opacity effects can still apply.
- Audio ducking and volume restoration now work with PipeWire stream IDs on Fedora.
- The Particles visualizer stays inside compact and mini overlay capsules; the outer glow is preserved.
- Fedora packages now include the KWin caret bridge and discover it automatically, restoring placement above the text caret when the application exposes it.

**Updates**
- Installed Fedora RPM and Ubuntu/Debian DEB builds can check for new releases from settings, display release notes, download a signed package, and install it after a system administrator prompt.
- Linux update packages are signed with the existing updater key and included alongside Windows in `latest.json`. Distribution-specific targets prevent Ubuntu and Debian packages from being mixed.
- Windows updates retain the existing signed NSIS installer and update endpoint.
- **The original Linux v1.3.0 package requires one manual upgrade to enable in-app updates**, because that version disabled update checks. Arch and source builds still update through the package manager.

**Compatibility / verification**
- Window controls, dictation audio ducking, caret placement, and particle clipping were confirmed by the user on Fedora 44 KDE Plasma / KWin 6.7.5, Wayland. Other distribution and Windows builds do not imply runtime testing.
- The KWin bridge is used only when its build version exactly matches the running compositor. After a KWin update, a matching Vispeak build is needed; otherwise AT-SPI and the configured fallback position are used.

---
📥 Download the installer from the Assets below, or check for updates in a supported installed version.
ℹ️ See the [README](https://github.com/V2P-Dev/Vispeak#readme) and [Linux guide](https://github.com/V2P-Dev/Vispeak/blob/v1.4.0/docs/LINUX.md) for installation and desktop limitations.
