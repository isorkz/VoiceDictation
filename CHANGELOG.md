# Changelog

All notable changes to this project will be documented in this file.

The format is based on Keep a Changelog, and this project adheres to Semantic Versioning.

## [Unreleased]
- TBD

## [1.0.1] - 2026-03-02
- Windows trigger key changed to `Ctrl` by default, with support for double-click toggle and hold-to-talk behavior.
- Added a real `Reset settings` action that resets to defaults while preserving the full `azure` config block.
- Added `sound.enabled` config and Settings UI switch (`Enable sounds`) to disable start/stop prompt sounds.
- Enforced a release gate: tag version must exist in `CHANGELOG.md` before GitHub Release workflow can proceed.
- Release workflow now enforces tag/version alignment across `package.json`, `tauri.conf.json`, and `Cargo.toml`.
- Release notes are now generated from the matching `CHANGELOG.md` section instead of a fixed message.
- Windows release asset filenames are normalized to the tag version before upload.

## [1.0.0] - 2026-02-22 (planned)
- Initial public release.
- GitHub Releases workflow for macOS/Windows builds.
