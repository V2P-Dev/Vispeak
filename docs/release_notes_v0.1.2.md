# Vispeak v0.1.2

Vispeak is a local, offline voice-to-text dictation app for Windows: press a
global hotkey, speak, and the transcribed text is pasted into the active window.
Everything runs locally — no audio or text ever leaves your device.

## What's new in v0.1.2
**Fixed**
- App icon: removed the dark square background and outer glow, leaving only the transparent speech bubble.
- UI: fixed layout glitches where tooltip texts could overlap in Settings.

## Key features
- Global hotkey (Ctrl+Space by default; toggle and push-to-talk modes)
- Fully local processing — no telemetry, no cloud
- Multiple recognition models (Whisper, Parakeet, Canary, GigaAM for Russian,
  Nemotron, Qwen), downloaded on demand from Hugging Face
- Three overlay skins, dictation history, five paste methods, auto-update

## Privacy
All speech recognition runs locally on your device. Audio and transcribed text
are never sent to any server. No telemetry or analytics.

## Installation
Download the installer from Assets below and run it.
Windows SmartScreen may warn about an unrecognized app (the build is not
code-signed with a paid certificate) — click "More info" → "Run anyway".

## Requirements
- Windows 10/11 (64-bit)
