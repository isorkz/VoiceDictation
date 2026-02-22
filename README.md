# VoiceDictation

Cross-platform (macOS/Windows) speech-to-text tray tool built with Tauri + React (TypeScript) and Azure OpenAI.

## What it does

- Records microphone audio
- Transcribes speech to text
- Pastes the result into the currently focused app

VoiceDictation runs as a tray app. Use the tray icon/menu to interact with it.

## Develop / Build from source

If you installed a prebuilt app, you can skip this section.

### Quickstart

Prerequisites:
- Node.js + npm
- Rust toolchain

Install deps:
```bash
npm install
```

Run in dev:
```bash
npm run tauri dev
```

Build for production:
```bash
npm run tauri build
```

## Setup

### 1) Permissions

VoiceDictation needs OS-level permissions to:
- record microphone audio
- listen for global key events (macOS Globe/Fn)
- paste text into the currently focused app

#### macOS

Required:
- **Microphone**
- **Input Monitoring** (global key listening)
- **Accessibility** (simulate paste)

Notes:
- The app runs in tray-only mode (no Dock icon and not shown in Cmd+Tab). Use the tray icon/menu to interact with it.
- The first time you start recording, macOS should show a Microphone permission prompt. If it doesn't, check System Settings → Privacy & Security → Microphone.

If a feature is not working, check:
System Settings → Privacy & Security → (Microphone / Input Monitoring / Accessibility)

#### Windows

Required:
- **Microphone**

Pasting uses Windows input injection APIs. Some security software may block it; allow VoiceDictation if needed.

### 2) Azure OpenAI API key & config

Set the API key in Settings (Azure → API key), then click Save.

VoiceDictation stores its settings in `config.json` under the OS-specific app config directory.
You can also edit the config file manually:
- `azure.apiKey`: Azure OpenAI API key
- `azure.endpoint`: `https://<resource>.openai.azure.com`
- `azure.deployment`: deployment name
- `azure.apiVersion`: API version

Transcription note: the app prompts the model to use Simplified Chinese for Chinese words while keeping English unchanged.

Security note: the API key is stored on disk (plain text). Treat the config file as sensitive data and protect your user account accordingly.

## Usage

### macOS: Globe/Fn (Language key)

VoiceDictation supports:
- hold Globe/Fn to start recording, release to stop + transcribe + paste
- double-click Globe/Fn to toggle recording on/off

#### Known limitations

The Globe/Fn key is handled by macOS for system features (input source switching, emoji picker, etc.).
Depending on your macOS version, keyboard model, and system settings, macOS may still react to the key even when VoiceDictation is listening.

VoiceDictation tries to **only swallow events after a hold/double-click trigger is detected**, so a single tap is still handled by macOS by default.

#### Recommended system settings

If macOS input switching interferes with VoiceDictation triggers, change the Globe key behavior:
System Settings → Keyboard → (Keyboard Shortcuts / Press Globe key to …)

Pick an action that does not conflict with your usage (for example, "Do Nothing" if available).

## Docs

- `docs/checklist.md` (implementation status + manual verification checklist)
- `docs/plan.md` (architecture notes / work plan)
