# Third-Party Licenses

Vispeak is built using various open-source libraries, frameworks, and resources. We are grateful to the authors and contributors of these projects.
All third-party code and assets belong to their respective owners and are licensed under their respective licenses.

## 1. Rust Dependencies
The backend of Vispeak is written in Rust and relies on the following open-source crates.
All Rust dependencies used in this project are licensed under permissive licenses:
- `MIT License`
- `Apache License 2.0`
- `Zlib License`
- `BSD-2-Clause` / `BSD-3-Clause`
- `BSL-1.0` (Boost Software License)
- `CDLA-Permissive-2.0`
- `ISC License`
- `MPL-2.0`
- `Unicode-3.0`
- `Unlicense`

A full list of Rust dependencies and their exact license texts can be reproduced using `cargo license` or `cargo deny`.

## 2. Node.js & Frontend Dependencies
The frontend of Vispeak is built with React and Tauri. All bundled production npm dependencies are released under permissive licenses:
- `@tauri-apps/api`: Apache-2.0 OR MIT
- `@tauri-apps/plugin-autostart`: MIT OR Apache-2.0
- `@tauri-apps/plugin-global-shortcut`: MIT OR Apache-2.0
- `@tauri-apps/plugin-opener`: MIT OR Apache-2.0
- `lucide-react`: ISC
- `react`, `react-dom`, `scheduler`: MIT
- `tailwindcss` (and related plugins): MIT

## 3. Bundled Binaries and Assets
The following third-party binaries and assets are bundled directly with the application installer:

### ONNX Runtime (`onnxruntime.dll`)
Downloaded via the `ort` Rust crate.
**License**: MIT License
**Source**: https://github.com/microsoft/onnxruntime

### Whisper.cpp (`ggml.dll`, `transcribe.dll`)
Locally compiled libraries for GGML inference.
**License**: MIT License
**Source**: https://github.com/ggerganov/whisper.cpp

### WebView2Loader (`WebView2Loader.dll`)
Required for Tauri webview on Windows.
**License**: BSD-3-Clause / MIT

### Silero VAD (`silero_vad.onnx`)
Pre-trained Voice Activity Detection model bundled inside the application executable.
**License**: MIT License
**Authors**: Silero Team
**Source**: https://github.com/snakers4/silero-vad

### Application Icon (`logo.png`)
**Authorship**: Generated using AI (Nano Banana 2 pro).

## 4. Downloadable AI Models (Not Bundled)
Vispeak does **not** include speech recognition models out-of-the-box. Users must explicitly download a model of their choice from within the app.
These models are downloaded from Hugging Face. Their licenses apply to the models themselves:

| Model Family | Author/Organization | Hugging Face Repository | License |
|--------------|---------------------|--------------------------|---------|
| **Whisper** (all sizes) | OpenAI / ggerganov | `ggerganov/whisper.cpp` | **MIT** |
| **Parakeet TDT 0.6B v3** | NVIDIA / istupakov | `istupakov/parakeet-tdt-0.6b-v3-onnx` | **CC-BY-4.0** |
| **Canary 1B v2** | NVIDIA / istupakov | `istupakov/canary-1b-v2-onnx` | **CC-BY-4.0** |
| **GigaAM v3 E2E** | SberDevices / istupakov | `istupakov/gigaam-v3-onnx` | **MIT** |
| **Nemotron 3.5 ASR** | NVIDIA / handy-computer | `handy-computer/nemotron-3.5-asr-streaming-0.6b-gguf` | **NVIDIA Open Model License** |
| **Qwen3-ASR 0.6B** | Qwen / handy-computer | `handy-computer/Qwen3-ASR-0.6B-gguf` | **Apache 2.0** |

*Note: The Hugging Face repositories are community-maintained mirrors providing quantized / optimized formats (ONNX/GGUF) of the official models.*
