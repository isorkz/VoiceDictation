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
- 自动构建 macOS/Windows 产物。
- 发布到 GitHub Releases。
 - Workflow 文件：`.github/workflows/release.yml`

### 4) 验证
- 确认 Release 中存在安装包。
- 手动下载并安装验证。

## Troubleshooting
- Release 缺少产物：检查 GitHub Actions 日志。
- macOS/Windows 警告：无平台签名证书导致，属预期行为。
