# KWin caret bridge

This optional module reads the global caret rectangle that KWin already receives
through Wayland text-input. It does not replace the input method, grab keys,
modify a target application's launch arguments, or read surrounding text.
Vispeak tries this geometry first and AT-SPI second.

The bridge must be built for the **exact running KWin version**. It checks the
runtime version and active text-input window before calling the compositor API.
Only finite, narrow caret rectangles inside the active client are accepted by
Vispeak. Applications without text-input geometry still need AT-SPI support.

Build with the matching KWin, Qt 6 and KConfig development packages:

```sh
cmake -S packaging/linux/kwin-caret -B src-tauri/target/kwin-caret
cmake --build src-tauri/target/kwin-caret
ctest --test-dir src-tauri/target/kwin-caret --output-on-failure
```

The isolated smoke test verifies plugin loading and rejection of non-KWin
processes. It does **not** load code into the live compositor.

After explicit approval for a live compositor test:

```sh
python3 scripts/probe-kwin-caret.py src-tauri/target/kwin-caret/Vispeak/CaretBridge
```

The probe loads a temporary QML script, prints only PID, caret geometry and failure reason, and
unloads the script. Native code is loaded inside KWin: a module failure could
end the graphical session. No persistent compositor configuration is changed.

Development builds automatically discover the module built at
`src-tauri/target/kwin-caret/Vispeak/CaretBridge` in this checkout. Production
builds discover a bundled module in the app resources. Fedora CI builds and
bundles it using the distribution's KWin headers. For local package builds,
install `kwin-devel qt6-qtbase-devel qt6-qtdeclarative-devel kf6-kconfig-devel`
and run `VISPEAK_BUILD_KWIN_CARET=1 npm run build:linux -- --bundles rpm`.
The bundle records its compile-time KWin version and Vispeak checks it against
the running compositor before loading native code. After a KWin update, a
matching rebuild is needed; otherwise Vispeak falls back to AT-SPI.
`VISPEAK_KWIN_CARET_MODULE` overrides discovery with an absolute module directory.
Without a module, Vispeak uses AT-SPI and the
ordinary JavaScript window-geometry probe. A failed native probe falls back to
the ordinary probe.

On KDE 6.7.5, a live probe successfully returned a 1×20 pixel caret from
Wayland text-input. The Vispeak recording path then received caret geometry in
both Chrome and Codex (including a 0×16 rectangle in Codex). The Rust live
probe passed. Visual overlay placement still awaits user confirmation. Do not overwrite a shared library already loaded
inside KWin; build updates in a new directory and restart KWin before switching
to the updated module.
