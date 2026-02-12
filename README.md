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

This app reads the API key from the environment variable `AZURE_OPENAI_API_KEY` only (it is never saved to disk).

See:
- `docs/env.md`

## Docs

- `docs/env.md`
- `docs/permissions.md`

