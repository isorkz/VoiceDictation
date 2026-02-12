# VoiceDictation（macOS/Windows）语音转文字工具（Tauri + React(TypeScript) + Azure OpenAI）

> 说明：本项目计划以 `docs/plan.md` 为准；根目录 `plan.md` 仅作为历史草稿参考。

## Summary（目标与成功标准）
- 做一个常驻托盘的小工具：按住（push-to-talk）或双击（toggle）触发录音 → 调用 Azure OpenAI `gpt-4o-mini-transcribe` 转写 → 将文本写入“当前光标所在编辑区”。
- **macOS**：Language(Globe/Fn) 键支持「按住说话、松开转写并粘贴」+「双击开/关持续录音」。
- **Windows**：默认热键 `Win+Shift+D`，支持「按住/双击」同样行为，并提供可配置热键。
- V1 插入文本方式：**Clipboard + Paste**（转写结果写入剪贴板 → 模拟 Cmd+V/Ctrl+V），并**默认恢复原剪贴板**。
- V2（后续）再加：Direct insertion（辅助功能/UIA 直接写入，不动剪贴板）。

## 当前仓库现状
- V1 功能基本可用（配置、托盘、自启动、录音→转写→粘贴、状态机、基础测试等），详见 `docs/checklist.md`。
- 当前改动目标：优化 Settings 页面 UI（布局与视觉层级），不改变现有功能行为。

## Tech Stack（已定）
- App：Tauri v2（Rust 后端）+ React（前端设置页）
- 音频采集：Rust `cpal`（跨平台）+ `hound`（写 WAV）
- HTTP：Rust `reqwest` + `multipart`
- 配置存储：**本地json配置文件**（放到系统 app config 目录，并在文档中明确风险）
- 全局键盘监听/热键：
  - macOS：CGEventTap（需要 Input Monitoring 权限）
  - Windows：`SetWindowsHookEx(WH_KEYBOARD_LL)`（`windows` crate）
- 文本注入（粘贴）：
  - macOS：CGEvent 模拟 `Cmd+V`（需要 Accessibility 权限）
  - Windows：`SendInput` 模拟 `Ctrl+V`
- 自启动：Tauri autostart 插件 + UI 开关

## Public Interfaces / Commands（对前端暴露的后端接口）
定义 Tauri commands（Rust → JS）：
- `get_status() -> { state, last_error? }`
- `get_config() -> Config`
- `set_config(Config) -> Result<()>`
- `test_connection() -> Result<()>`（上传一段极短静音/或仅做配置校验与握手请求）
- `toggle_recording()`（给 UI/托盘按钮用）
- 事件（emit 给前端）：`status_changed`, `error`, `transcript_ready`

Config（存储为 JSON）字段（全部在 UI 可编辑）：
- `azure.endpoint`（例：`https://<resource>.openai.azure.com`）
- `azure.deployment`（部署名，指向 `gpt-4o-mini-transcribe`）
- `azure.apiVersion`（默认：`2025-03-01-preview`；必要时可在 UI 改）
- `hotkey.windows`（默认：`Win+Shift+D`）
- `thresholds.holdMs`（默认：180ms）
- `thresholds.doubleClickMs`（默认：300ms）
- `recording.maxSeconds`（默认：120s；toggle 模式同样限制）
- `insert.restoreClipboard`（默认：true）
- `insert.postfix`（默认：无额外换行/空格）

## Core Data Flow（状态机）
- States：`Idle` → `Recording` → `Transcribing` → `Inserting` → `Idle`（或 `Error`）
- Push-to-talk（按住）：
  1) 监听到热键 down：开始录音（或先进入准备态，立即打开流以降低延迟）
  2) 热键 up：停止录音 → 写 WAV → 调用转写 → 得到文本 → clipboard+paste → 恢复剪贴板 → 回到 Idle
- Toggle（双击）：
  - 双击热键：`Idle -> Recording(toggle)`；再次双击：停止并进入转写/插入流程
- Busy 策略（明确，不留决策）：
  - `Transcribing/Inserting` 期间再次触发：忽略并弹通知 “Busy”
  - `Recording(toggle)` 期间按住不做额外动作（只响应“停止”触发）

## Hotkey / Language Key 细节与已选策略（关键风险点说明）
- **“单击 Language 键由 macOS 自己处理”**。
- 实现策略（满足需求且尽量不干扰）：
  - 我们监听 Globe/Fn 的 down/up 与时间间隔。
  - 对于 hold/double-click 触发的那次会尽力吞掉相关事件（至少吞 key up），以减少触发系统切换输入法的概率。
  - 但由于 macOS 对 Globe 键的系统行为可能在按键流程中提前触发，**文档里会明确：若出现输入法切换/弹窗干扰，需要用户在系统里把 Globe 行为改成 Do Nothing/Emoji 等**（并给出具体路径）。
- Windows：默认 `Win+Shift+D`，并在设置中可改。

## Azure OpenAI Transcription（REST after stop）
- 在 key up / toggle stop 时生成音频文件（WAV，mono，16k 或设备原采样率后重采样到 16k）。
- 请求形态（实现上用 `reqwest::multipart`）：
  - `POST {endpoint}/openai/deployments/{deployment}/audio/transcriptions?api-version={apiVersion}`
  - Header：`api-key: {apiKey}`
  - Form：`file=@recording.wav`（必要时加入 `response_format=json`）
- 解析结果：取 `text` 字段作为最终插入文本。
- 语言：默认不传 language（Auto-detect）。

## Insert（V1：Clipboard + Paste + Restore）
- 步骤：
  1) 保存当前剪贴板文本（仅 text；若原剪贴板不是文本则记录“无法恢复”为 warning）
  2) 写入转写文本到剪贴板
  3) 模拟粘贴（mac：`Cmd+V`；win：`Ctrl+V`）
  4) 延迟 150ms 后恢复剪贴板
- 失败处理：如果粘贴注入失败（缺权限/被系统拒绝）→ 弹通知提示开启权限，并提供“复制到剪贴板但不粘贴”的降级动作（仅作为错误处理路径，不额外做兼容分支）。

## Permissions / Onboarding（必须落地的用户引导）
- macOS：
  - Microphone
  - Input Monitoring（键盘监听）
  - Accessibility（模拟粘贴）
- Windows：
  - Microphone
  - 可能需要允许后台应用控制输入（通常 SendInput 无需管理员）
- 首次启动流程：
  - 托盘提示缺少的权限项 + “Open Settings” 按钮（分别打开对应系统设置页）
  - 配置页要求填完 Azure 三项（endpoint/apiKey/deployment），并提供 “Test connection”

## UI（React）
- Tray menu：
  - Start/Stop（toggle）
  - Open Settings
  - Launch at login（开关）
  - Quit
- Settings UI（layout refresh, Tailwind）：
  - 目标：卡片化分区 + 响应式栅格 + 清晰的状态/错误展示 + 统一按钮层级（现代简洁风）。
  - 边界：仅做 UI/布局/交互观感优化，不改变保存/调用后端等业务行为与字段含义。
  - 约束：继续保持表单的可访问性（`label`/`htmlFor`/`aria-*`），并同步更新现有前端测试。
- Settings page：
  - Azure：endpoint / apiKey / deployment / apiVersion / Test
  - Hotkeys：Windows 默认热键可编辑；macOS 显示 Language(Globe/Fn) 键说明（hold/double-click）
  - Thresholds：holdMs / doubleClickMs / maxSeconds
  - Insert：restoreClipboard（开关）+ postfix（先固定 No extra）
  - Test：按钮上用括号显示固定测试时长（当前为 3s），让用户知道每次测试会录音多久

## Implementation Steps（按小步提交；脚手架一次性生成除外）
> 说明：初始化脚手架会新增大量文件；后续每个任务尽量控制为小范围（若预计 >3 个文件改动就拆分任务）。

1) Scaffold & repo setup
- `git init`
- `npm create tauri-app@latest`（Tauri v2 + React + TS）
- 加 `.gitignore`/`.ignore`：忽略 `.env`、构建产物
- Commit：`chore: scaffold tauri react typescript app`

2) Config + UI skeleton
- Rust：Config 结构体 + 读写 JSON（app config dir）
- React：Settings 页表单 + 校验 + 保存/读取
- Commit：`feat: add config storage and settings UI`

3) Azure transcription client
- Rust：`transcribe(file_path) -> text`（reqwest multipart）
- React：Test connection 触发后端并展示结果/错误
- Commit：`feat: add azure openai transcription client`

4) Audio recording module
- Rust：`Recorder`（cpal capture → WAV）
- 单元测试：状态机/文件输出基本校验（不依赖真实麦克风的部分做 mock）
- Commit：`feat: add cross-platform audio recorder`

5) Insert (clipboard+paste) + clipboard restore
- Rust：ClipboardGuard + mac paste injection + win paste injection（按平台拆两次提交）
- Commit A：`feat: paste insertion on macOS`
- Commit B：`feat: paste insertion on Windows`

6) Global hotkey / key listener
- macOS：CGEventTap 监听 Globe/Fn；实现 hold/double-click 识别与录音状态机驱动
- Windows：WH_KEYBOARD_LL 监听 `Win+Shift+D` 并可配置
- Commit A：`feat: language key trigger on macOS`
- Commit B：`feat: hotkey trigger on Windows`

7) Tray + autostart + notifications
- Tray 菜单与状态展示（Recording/Transcribing）
- autostart 开关接入（并持久化）
- Commit：`feat: tray, autostart, and notifications`

8) Docs + smoke tests
- `docs/`：
  - 安装/运行（dev/build）
  - macOS 权限开启与 Globe 键系统设置建议
  - Azure OpenAI 配置说明（deployment 必须是 `gpt-4o-mini-transcribe`）
- Smoke test checklist（macOS：TextEdit/VS Code；Windows：Notepad/VS Code）
- Commit：`docs: add setup and permissions guide`

9) V2 预留（不在 V1 实现）
- Direct insertion（mac AX / win UIA）作为可选插入方式
- Realtime streaming transcription（WebSocket）作为可选模式

## Test Cases（必须覆盖）
- Hotkey 逻辑：
  - hold：down→(>=holdMs)→up 触发一次转写
  - double-click：两次 tap within doubleClickMs 切换 Recording(toggle)
  - busy：Transcribing 时触发会提示 Busy 且不打断
- Config：
  - 保存/读取一致性；缺字段默认值生效
- Azure client：
  - 请求构造（URL、header、multipart）；解析 `text`
  - 超时/401/404（deployment 错）错误信息可读
- Insert：
  - restoreClipboard=true 时最终剪贴板恢复；失败时保留转写文本并提示

## Assumptions / Defaults（明确记录）
- V1 插入采用 Clipboard+Paste，且默认恢复剪贴板。
- 转写使用 REST `/audio/transcriptions`，在停止录音后一次性上传。
- macOS 单击 Globe 键由系统处理；如与双击/按住冲突，用户需在系统设置调整 Globe 行为。
- 默认阈值：hold 180ms；double-click 300ms；max recording 120s。
- Windows 默认热键：`Win+Shift+D`。
- Azure key 通过环境变量读取；文档提示风险与建议。
