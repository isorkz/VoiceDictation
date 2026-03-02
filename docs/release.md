# Release & Update（GitHub Releases）

> 本文档描述 macOS/Windows 的发布与更新流程（仅 GitHub Releases）。

## Overview
- 发行渠道：GitHub Releases（public）。
- 触发方式：`git tag vX.Y.Z && git push --tags`。
- 构建产物：macOS/Windows 安装包。
- 不提供 App 内更新；用户通过 GitHub Releases 手动下载新版本。
- 不做平台代码签名（macOS Gatekeeper / Windows SmartScreen 可能提示风险）。

## One-time Setup

### 1) 创建 GitHub 仓库
- 远程仓库：`https://github.com/isorkz/VoiceDictation.git`
- 本地添加 remote：
  - `git remote add origin https://github.com/isorkz/VoiceDictation.git`

### 2) 配置 GitHub Secrets
- 仅需默认的 `GITHUB_TOKEN`（GitHub Actions 自动提供）。

## Release Workflow（Tag 触发）

### 1) 版本号一致性
发布前确保下列版本号一致：
- `package.json` `version`
- `src-tauri/tauri.conf.json` `version`
- `src-tauri/Cargo.toml` `version`

### 2) 生成并推送 Tag
- `git tag vX.Y.Z`
- `git push --tags`

### 3) GitHub Actions
- 发布 workflow 会先检查 `CHANGELOG.md` 是否包含 `## [X.Y.Z]` 对应版本标题；缺失则发布失败。
- 发布 workflow 会检查 `tag` 版本必须与以下版本完全一致，不一致则发布失败：
  - `package.json` `version`
  - `src-tauri/tauri.conf.json` `version`
  - `src-tauri/Cargo.toml` `version`
- 发布说明（Release body）会从 `CHANGELOG.md` 的 `## [X.Y.Z]` 小节自动提取，不再使用固定文案。
- 自动构建 macOS 产物。
- 对 `.app` 做 ad-hoc 重新签名，避免 Gatekeeper 报 “damaged”。
- 使用本地 `hdiutil` 重新打包 DMG。
- Windows 产物会在上传前按 tag 版本统一命名，避免版本号展示不一致。
- 发布到 GitHub Releases。
  - Workflow 文件：`.github/workflows/release.yml`

### 4) 验证
- 确认 Release 中存在安装包。
- 手动下载并安装验证。

## Troubleshooting
- Release 缺少产物：检查 GitHub Actions 日志。
- macOS 警告：无平台签名证书导致，属预期行为。
- macOS 仍无法打开：让用户移除隔离标记：
  - `xattr -dr com.apple.quarantine /Applications/VoiceDictation.app`
