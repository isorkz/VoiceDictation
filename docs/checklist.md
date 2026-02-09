# VoiceDictation — 实现状态与验收 Checklist

> 本文件用于跟踪 V1 的已完成项 / 剩余项 / 验收步骤。

## 已完成（V1）

### 基础工程
- Tauri v2 + React + TypeScript 脚手架与构建链路（`npm run tauri dev` / `npm run build` / `npm run lint`）。
- 项目基础文档：`docs/env.md`、`docs/permissions.md`、`docs/macos-globe.md`、`plan.md`。

### 配置与 UI
- `config.json`（不含 key）读写与默认值（Rust：`src-tauri/src/config.rs`）。
- Settings UI：编辑 `endpoint/deployment/apiVersion`、Windows hotkey 字符串、hold/doubleClick 阈值、maxSeconds、restoreClipboard、autostart 开关（TS：`src/App.tsx`）。
- API key 仅从环境变量读取：`AZURE_OPENAI_API_KEY`（UI 显示 detected/not detected）。

### 录音 → 转写 → 写入
- 录音：跨平台采集麦克风 → WAV（mono）并重采样到 16k（Rust：`src-tauri/src/audio.rs`）。
- 转写：调用 Azure OpenAI `/audio/transcriptions`，模型部署名由 `azure.deployment` 提供（Rust：`src-tauri/src/azure_transcribe.rs`）。
- 写入：Clipboard + Paste + Restore（mac 模拟 `Cmd+V`，win 模拟 `Ctrl+V`）（Rust：`src-tauri/src/insert.rs`）。
- 状态机：`Idle/Recording/Transcribing/Inserting` + 错误复位（Rust：`src-tauri/src/lib.rs` / `src-tauri/src/app_state.rs`）。
- 安全阈值：录音超过 `maxSeconds` 自动停止并进入转写流程。

### 托盘与自启动
- Tray menu：Start/Stop、Settings、Launch at login、Quit（Rust：`src-tauri/src/tray.rs`）。
- 启动时主窗口默认不显示；从托盘菜单 “Settings” 打开；关闭窗口会隐藏到托盘继续运行。
- Autostart：集成 `tauri-plugin-autostart`，支持托盘与 UI 开关。

### 全局触发
- Windows：`WH_KEYBOARD_LL` 监听（默认 `Win+Shift+D`；支持按住触发、双击 toggle）（Rust：`src-tauri/src/key_listener/windows.rs`）。
- macOS：方案 A（`CGEventTap` best-effort 监听 Globe/Fn 的 hold/double-click）（Rust：`src-tauri/src/key_listener/macos.rs`）。

## 剩余（未完成 / 已知缺口）

### Hotkey 与配置细节
- Windows hotkey 解析目前只支持 `Win/Ctrl/Alt/Shift` + **单个字母键**（例如 `Win+Shift+D`）；不支持 `Space`/`F1`/`RightAlt` 等。
- Windows/macOS 的 key listener 目前只在启动时读取一次 `config.json`；修改配置后需要重启应用才会影响全局触发。

### 权限与可用性
- 缺少“权限引导 UI/跳转系统设置”的一键按钮（目前只靠文档 + 控制台提示）。
- 未实现系统级通知（Notification）；目前错误主要通过 UI 文本与控制台输出体现。

### 需求扩展（明确不在 V1）
- Direct insertion（不走剪贴板的直接写入）。
- Realtime streaming transcription。
- 更强的 macOS Globe/Fn 事件吞掉策略（目前仅 best-effort，仍可能触发系统输入法切换/emoji 等）。

## 验收 Checklist（手工）

### 0) 环境准备
- 设置 `AZURE_OPENAI_API_KEY`（mac/Windows 的 GUI/自启动环境变量注意事项见 `docs/env.md`）。
- 启动开发版：`npm run tauri dev`。

### 1) Settings UI 基本可用
- 能打开主窗口并看到 Status（Idle/Recording/…）。
- 能保存并重载 `endpoint/deployment/apiVersion`（重启后仍保留）。
- `AZURE_OPENAI_API_KEY` 状态显示正确（detected / not detected）。
- Autostart 开关可切换（允许的话，重启系统/注销后验证是否自启动）。

### 2) Test transcription（端到端）
- 点击 “Test transcription”：
  - 录音约 1.2 秒
  - 返回转写文本并显示在页面 “Transcript”
  - 失败时能显示可读错误（401/404/空配置/缺 key）

### 3) Start/Stop（UI 控制）
- 点击 “Start” → Status 变为 Recording。
- 点击 “Stop” → Status 依次为 Transcribing → Inserting → Idle。
- 在光标所在编辑区（TextEdit/VS Code/Notepad）确认最终文字被粘贴进去。
- `restoreClipboard=true` 时：粘贴后剪贴板内容会恢复到转写前（仅文本类型可恢复）。

### 4) 全局触发（macOS）
- 已授予权限：Microphone + Input Monitoring + Accessibility（见 `docs/permissions.md`）。
- Hold Globe/Fn：
  - 按住超过 holdMs：开始 Recording
  - 松开：停止 → 转写 → 粘贴到当前光标位置
- Double-click Globe/Fn：
  - 双击：进入 Recording（toggle）
  - 再次双击：停止 → 转写 → 粘贴

### 5) 全局触发（Windows）
- 默认热键：`Win+Shift+D`（或在 Settings 修改后重启应用）。
- Hold：
  - 按住超过 holdMs：开始 Recording
  - 松开：停止 → 转写 → 粘贴
- Double-click：
  - 双击：toggle 开始/停止录音
