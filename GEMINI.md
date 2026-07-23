# Agent rules for the Vispeak project

## About the project
Vispeak is a Windows desktop application for local, offline voice transcription triggered by a global hotkey. The user presses Ctrl+Space, dictates, and the transcribed text is pasted into the active text field. Stack: Tauri 2, Rust (backend), React + TypeScript + Tailwind (frontend), whisper-rs, cpal, enigo, tauri-plugin-global-shortcut.

## Core rules
1. **Windows-only.** Do not write code, cfg branches, or dependencies for macOS/Linux. Do not add cross-platform abstractions "for the future".
2. **MVP scope only.** Do not add features that are not part of the current task: no transcription history, cloud APIs, telemetry, auto-updates, or text post-processing. If you believe a feature would be useful, suggest it in one line at the end of your reply — but do not implement it.
3. **Do not break working code.** After every significant change, verify: `cargo check` for Rust, `npm run tauri dev` for a full run. Do not refactor working code unless the current task directly requires it.
4. **docs/PLAN.md is the source of truth.** Read it at the start of every session. At the end of every task, update it: what was done, what remains, known issues, and build-problem solutions (Troubleshooting section).
5. **Be honest about status.** Never say "done" if you have not run and verified it. If you cannot verify something yourself (requires a microphone / dictation), state explicitly what the user must test manually.

## Critical invariants (violating any of these = broken app)
- **The overlay must never receive input focus.** Any change to the overlay window must be verified with this test: focus in Notepad -> show overlay -> the cursor stays blinking in Notepad. Window flags (focus: false, alwaysOnTop, skipTaskbar, decorations: false, transparent) must not be changed without an explicit request.
- **Text is pasted into the focused window.** Before simulating Ctrl+V, make sure focus is not in any Vispeak window. If the main window is active — do not paste.
- **The user's clipboard is restored** after pasting (~300 ms).
- **Audio for Whisper: strictly mono, 16 kHz, f32.** When touching the audio pipeline, verify resampling (speech in the debug WAV must not be sped up or slowed down).
- **Transcription never blocks the UI** — always in a separate thread. The model is loaded into memory once and reused.
- **The global hotkey works while the app is minimized.** Esc is registered only for the duration of an active recording/processing cycle and unregistered afterwards.

## Design system (do not deviate)
- Window background #0d0d0d, surfaces #1c1c1e, borders #2a2a2e, overlay card #141414.
- Text: primary #f5f5f7, secondary #8e8e93.
- Accent #ff5533 (active states, recording), success #7ed491, processing #4dd8e6 (overlay only), corner radii 12-16px, chips are pill-shaped.
- Forbidden: glassmorphism, blur, gradients, any other accent colors, heavy animation libraries (CSS transitions / canvas only).
- All colors/radii/shadows go through Tailwind tokens only — no hardcoded values in components.
- All UI strings come from the localization file (ru/en) only — no string literals in JSX.

## Code style
- Rust: no unwrap()/expect() outside tests and initialization — errors via Result and a typed error enum; human-readable error messages are forwarded to the frontend as events.
- Rust: modules by responsibility (audio, transcribe, models, paste, hotkeys, overlay); do not pile everything into main.rs/lib.rs.
- TypeScript: strict mode, no `any`; Rust->frontend event types are described in one shared file.
- Events between Rust and the frontend are named in kebab-case ("recording-started", "audio-level", "target-app") and listed in docs/PLAN.md.
- Settings are a single JSON file in %APPDATA%/Vispeak, read/written through one settings module, with defaults when the file is missing or corrupted.
- Code comments in English, commit messages in English, communication with the user in Russian.

## Workflow
- One task = one user prompt. Do not run ahead into future stages.
- Before a large change, briefly outline a plan (3-5 bullet points), then execute; do not wait for confirmation unless the user asked for it.
- On a build error: read the full error message first, check dependency versions and environment variables (LIBCLANG_PATH for whisper-rs); do not downgrade dependencies blindly.
- Add new crates/npm packages only when genuinely needed; justify each new one in a single line in your reply.
- At the end of every task, provide a short manual verification checklist for the user (what to press, what should happen).

## Git & Release — command-gated (CRITICAL)

The agent MUST NEVER perform any of the following on its own, without a separate
explicit command from the user:
- git commit
- git push
- git tag / deleting tags
- creating, editing, publishing, or deleting releases (gh release ...)
- git push --force, git reset, or any history-rewriting operation

This rule always applies and is never overridden by context. Even if the task
looks finished, even if changes are obviously ready to commit, even if the user
sends only a version string (e.g. "v0.1.5") — that is NOT a command to commit,
push, or release; it only starts the preparation. Each of the actions above is
performed ONLY after a direct, unambiguous instruction to do that specific action
("commit", "push", "create the tag", "publish the release").

What to do instead of acting on your own:
- Edit files (code, configs, docs) — allowed, this is not a git operation.
- Prepare everything for the release/commit and STOP with a report of what is
  ready and which git commands are proposed.
- Show the proposed commands/texts (commit message, release notes) for approval,
  but do not execute them.
- Wait for an explicit command.

If unsure whether a message is a command to perform a git action, assume it is
NOT, and ask.

### Release invariants (once a release command is given)
- Consistency with the first release v0.1.1: release-notes structure, latest.json
  format, artifact set (NSIS .exe + .sig + latest.json).
- NEVER touch plugins.updater.pubkey in tauri.conf.json.
- NEVER commit or log the signing private key, its password, or any secret.
- Version must be in sync across tauri.conf.json and package.json.
- Show release notes BEFORE tagging; verify latest.json AFTER the build
  (version, signature, url pointing to the NSIS .exe, endpoint reachable).

### Release notes rules
- Release notes are a CONCISE CHANGELOG only ("What's changed"), never a full app
  description.
- NEVER add a "# Vispeak vX.Y.Z" title inside the notes body — GitHub already
  renders the version as the release title from the tag; repeating it duplicates
  the heading.
- Do NOT repeat app description / key features / privacy / installation in each
  release; those live in the README.
- The first release (v0.1.1) keeps its full "showcase" description; from v0.1.2
  onward, notes are changelog-only.
- The GitHub Release Title must strictly be the version string (e.g. `v0.1.5`).
  NEVER include the word "Vispeak" in the release title (DO NOT use `Vispeak v0.1.5`).
- Use docs/RELEASE_NOTES_TEMPLATE.md as the base; fill only non-empty categories.
