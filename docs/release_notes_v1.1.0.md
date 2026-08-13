## What's changed

**Added**
- **Qwen3-ASR 0.6B**: Added Qwen3-ASR 0.6B multilingual model (~850 MB, Q8_0) to the model registry via `transcribe-cpp` (GGML batch mode, 30 languages).
- **Streaming Live Text Insertion**: Support for instant, real-time typing into the target window for "Mini (at cursor)" skin using Nemotron 3.5 Streaming.

**Improved**
- **Perceptual Brightness Normalization**: Calibrated glow luminance across White accent, Orange accent, Black accent, and status signals (Processing turquoise, Success green) for visual harmony in both Dark and Light themes.
- **Instant Win32 Batch Typing**: Replaced inter-character delay typing with direct Win32 `SendInput` batch event execution, eliminating typewriter animation and thread queuing delays.
- **Streaming Full Overlay**: Vertical auto-expansion and smooth word-by-word streaming text rendering for Nemotron 3.5 ASR Streaming.
- **Controls Page Settings**: Relocated "Live Text / Streaming Input" setting toggle to the Controls page with non-streaming model warnings.

---
📥 Download the installer from the Assets below.
ℹ️ First time here? See the [README](https://github.com/V2P-Dev/Vispeak#readme) for features, privacy, and installation.
