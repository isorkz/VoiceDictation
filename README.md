# VoiceDictation

Cross-platform (macOS/Windows) speech-to-text tray tool built with Tauri + React (TypeScript) and Azure OpenAI.

## Quickstart

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

## Azure OpenAI API key

Set the API key in Settings. It is saved to the app config file (`config.json`) on disk.

See:
- `docs/env.md`

## Docs

- `docs/env.md`
- `docs/permissions.md`
