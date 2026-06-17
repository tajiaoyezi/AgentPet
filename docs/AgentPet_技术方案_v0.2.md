# AgentPet 技术方案文档

文档版本：v0.2（技术方案修订版）  
目标平台：v0.1 Windows-first，v0.2 macOS  
技术栈：Tauri v2 + Rust + TypeScript + Canvas 2D（+ 一个编译型事件 sidecar，见 §3.4）  
核心目标：统一多个 AI Agent 的完成、等待输入、权限确认、错误、卡住等状态提醒，并通过可自定义桌宠、声音、Toast、托盘和状态面板进行反馈。

> 本版相对 v0.1 的修订均已对照官方文档核实（Claude Code Hooks、Codex Hooks / Config Reference），详见 [0. 修订说明](#0-修订说明) 与 [29. 参考资料](#29-参考资料)。

---

## 目录

- [0. 修订说明](#0-修订说明)
- [1. 项目定位](#1-项目定位)
- [2. 版本边界](#2-版本边界)
- [3. 技术选型](#3-技术选型)
- [4. 总体架构](#4-总体架构)
- [5. 核心模块设计](#5-核心模块设计)
- [6. 本地事件系统](#6-本地事件系统)
- [7. Session Registry](#7-session-registry)
- [8. 状态聚合规则](#8-状态聚合规则)
- [9. Adapter 设计](#9-adapter-设计)
- [10. Claude Code Adapter](#10-claude-code-adapter)
- [11. Codex Adapter](#11-codex-adapter)
- [12. Generic CLI Adapter](#12-generic-cli-adapter)
- [13. Pet Runtime 设计](#13-pet-runtime-设计)
- [14. Codex Pet Importer](#14-codex-pet-importer)
- [15. 通知系统](#15-通知系统)
- [16. 状态面板](#16-状态面板)
- [17. 数据存储](#17-数据存储)
- [18. 设置文件](#18-设置文件)
- [19. UI 设计](#19-ui-设计)
- [20. Windows 平台实现细节](#20-windows-平台实现细节)
- [21. 安全与隐私](#21-安全与隐私)
- [22. 日志与诊断](#22-日志与诊断)
- [23. 测试方案](#23-测试方案)
- [24. v0.1 验收标准](#24-v01-验收标准)
- [25. 开发里程碑](#25-开发里程碑)
- [26. 风险与应对](#26-风险与应对)
- [27. 后续 Hatch Studio 预留](#27-后续-hatch-studio-预留)
- [28. 最终推荐落地顺序](#28-最终推荐落地顺序)
- [29. 参考资料](#29-参考资料)

---

## 0. 修订说明

本节汇总 v0.1 → v0.2 的关键修订。每条都标注了原因，便于团队理解 delta。涉及外部工具行为的判断均已核对官方文档。

### 0.1 必须改的硬问题（已修正）

**[C1] Codex 的外部 `notify` 命令只在 `agent-turn-complete` 触发，不能传 permission / error。**  
原 §11 把 `notify` 当成能按 payload 区分 completed / needs_input / permission_request / error 的通道，这是错误的。官方说明 `notify` 当前仅支持 `agent-turn-complete` 事件；权限请求（approval-requested）属于内置 TUI 通知，不会经由外部 `notify` 命令；子任务生命周期事件只能走 hooks 引擎。  
**修正**：Codex 的**主路径改为 hooks 引擎**（覆盖 Stop / PermissionRequest / UserPromptSubmit 等全部状态），`notify` 降级为可选的“仅完成提醒”冗余信号。详见 §11。

**[C2] `notify` 与 hooks 同时安装会导致“完成”事件 double-fire。**  
Codex 完成一个 turn 时，`notify`（agent-turn-complete）与 `Stop` hook 会同时触发。  
**修正**：默认**只安装 hooks**；若用户主动开启 notify，normalizer 按内容（cwd + turn-id + type）做去重。详见 §6.6、§11.1。

**[C3] Session 主键改用 hook 的 `session_id`，弃用脆弱的 `projectPath + recent` 兜底。**  
Claude Code 每个 hook 事件的 stdin payload 都带 `session_id` / `transcript_path` / `cwd`。原 §7.3 的 `projectPath + source + recent session` 兜底会把**同一 repo 的两个 Claude 终端 tab 合并成一个 session**。  
**修正**：`session_id` 作为第一优先级主键（Claude 始终可用）；Codex 仅能做到 cwd 级别识别，文档显式说明该限制。详见 §7.3。

**[C4] v0.1 最小 hook 集补 `UserPromptSubmit`，否则 `running`（工作中）动画永不出现。**  
原最小集 = Notification / PermissionRequest / Stop / StopFailure，没有任何事件产生 `running`，与验收标准“Agent 工作时桌宠进入 running 动画”冲突。  
**修正**：最小集加入 `UserPromptSubmit`（turn 级、开销极小）→ `running`。`PreToolUse` 仍默认关闭（每次工具调用都触发，会造成进程风暴）。详见 §10.2、§11.4。

**[C5] 同步 hook + PowerShell 冷启动会给 agent 增加可感知延迟，引入编译型事件 sidecar。**  
Codex hook 是**同步**的（Codex 阻塞等待 hook，从 stdout 读响应）；每个被 hook 的事件都会把 PowerShell 冷启动（数百 ms）叠加到 agent 的 turn 延迟上。  
**修正**：热路径改用一个 ~2MB 的编译型静态二进制 `agentpet-event(.exe)`（只做“读 stdin/argv → fire-and-forget POST 127.0.0.1”），延迟从数百 ms 降到个位数 ms，并绕开 ExecutionPolicy / 杀软对 `.ps1` 的拦截；`.ps1` 作为未部署 sidecar 时的 fallback。Claude 侧额外采用异步（非阻塞）hook。详见 §3.4、§10.3、§11.2。

### 0.2 应该改 / 需注意（已调整）

**[A1] “聚焦窗口”是过度承诺。** hook 子进程拿不到启动它的终端模拟器与窗口标题，且 Windows 限制后台进程抢焦点（通常只能 `FlashWindowEx` 闪任务栏）。已将可靠聚焦限定在 Generic CLI wrapper 启动的会话，并把验收项改为“尝试聚焦，失败则闪烁 + 提示”。详见 §16.2、§24。

**[A2] Claude 的 `Notification` 有子类型，不能全映射成 needs_input。** matcher 含 idleprompt / permissionprompt / authsuccess / elicitationdialog；permissionprompt 与单独的 `PermissionRequest` 事件重叠。normalizer 按 matcher 路由并去重。详见 §6.6。

**[A3] Codex hooks 的 `commandWindows` 用数组形式，删除多余的 `[features] hooks = true`。** 官方示例中 `command` / `commandWindows` 为数组；hooks 默认即启用。详见 §11.3。

**[A4] `runtime.json` 生命周期。** 退出时删除，避免 AgentPet 未运行时 hook 仍 POST 死端口吃超时；处理端口被复用的脏读。详见 §6.1。

**[A5] 三个 Tauri 窗口不共享前端 store，以 Rust 后端为唯一 source of truth，窗口通过 Tauri events 订阅。** 详见 §4.3、§5.2。

**[A6] WebP 解码先验证。** Rust `image` crate 对 WebP（尤其 lossless / animated）支持历史上不全，M1 用真实 spritesheet 验证，必要时换 libwebp 后端的 `webp` crate；atlas 规格对照当前 hatch-pet skill 复核。详见 §14.4、§25。

**[A7] 勿扰 × sticky 重复提醒的交互规则补齐。** 详见 §15.3。

### 0.3 保持不变（已确认正确）

- 整体架构：单进程 Tauri v2、loopback HTTP + token、adapter 模式、带 raw 保留的 normalizer、session registry、reaction engine。
- Adapter 安装流程：检测 → diff → 确认 → 备份 → 写入 → 测试 → 恢复（幂等 + 可卸载）。
- 安全模型：只绑 127.0.0.1、token、不执行 payload 命令、不远程拉脚本、宠物包拒绝可执行文件、禁 `../` 与绝对路径。
- 传参机制：hooks 走 stdin（Claude 与 Codex hooks）、Codex `notify` 走 argv——区分正确。
- Claude hook 事件名 Notification / PermissionRequest / Stop / StopFailure 均真实存在且使用正确。
- 里程碑 M0–M6 与“看到 → 收到 → 提醒 → 接 Agent → 发布”的顺序。

---

## 1. 项目定位

AgentPet 不是单纯的桌宠，也不是单纯的通知工具，而是一个 **跨 Agent 的本地注意力中心**。

用户在 Windows 上同时使用 Claude Code、Codex、Cursor、ZCode、Windows Terminal、Warp、GUI Agent 等工具时，经常会出现：

```text
Agent 已经完成，但用户没有看到。
Agent 卡在权限确认，但用户不知道。
Agent 报错或停止，但终端 / GUI 没有明显提醒。
多个 Agent 并行运行时，用户不知道哪个任务最需要处理。
```

AgentPet 的目标是把这些状态统一收敛到本地：

```text
Claude / Codex / Cursor / ZCode / Generic CLI
        ↓
Agent Adapter
        ↓
Local Event Bus
        ↓
Session Registry
        ↓
Reaction Engine
        ↓
桌宠动作 / 声音 / Toast / 托盘 / 状态面板
```

v0.1 的核心目标是：**Windows 上优先支持 Claude Code、Codex、Codex 宠物导入、桌宠动作响应、声音提醒、系统通知和状态面板。**

---

## 2. 版本边界

### 2.1 v0.1 范围

v0.1 聚焦可用性和稳定性，不做宠物制作器。

```text
v0.1 Windows-first MVP
├─ Tauri v2 + Rust 桌面应用
├─ 透明桌宠悬浮窗口
├─ 托盘入口
├─ 本地事件接收服务
├─ 编译型事件 sidecar（agentpet-event.exe）+ PowerShell fallback
├─ Claude Code Adapter（hooks，含 UserPromptSubmit）
├─ Codex Adapter（hooks 为主，notify 可选）
├─ Generic CLI Adapter 基础能力（窗口 / PID / 标题最可靠的路径）
├─ Codex 宠物导入
├─ AgentPet Pet Runtime
├─ 状态动作映射
├─ 声音提醒
├─ Windows Toast / 原生通知
├─ 多 Agent Session 状态面板
├─ 配置备份 / 恢复
└─ 本地安全 token
```

接入机制要点（已核对官方文档）：

- **Claude Code hooks**：在生命周期事件匹配时触发，command hook 的输入从 **stdin** 传入，且每个事件的 payload 都包含 `session_id`、`transcript_path`、`cwd`、`hook_event_name` 等公共字段。与 AgentPet v0.1 直接相关的事件：`UserPromptSubmit`、`Notification`、`PermissionRequest`、`Stop`、`StopFailure`。command hook 可异步（非阻塞）执行，适合通知用途。
- **Codex hooks 引擎**：已稳定，默认启用，可在 `config.toml` 内联配置；事件包含 `SessionStart`、`UserPromptSubmit`、`PreToolUse`、`PostToolUse`、`PermissionRequest`、`SubagentStart`、`SubagentStop`、`Stop` 等，payload 经 **stdin** 同步传入。这是 Codex 的**主接入路径**。
- **Codex `notify`**：根级配置，是一个命令数组，payload 经 **argv** 传入；**当前仅在 `agent-turn-complete` 触发**（只能用于完成提醒，无法获取权限 / 报错 / 子任务事件）。因此在 AgentPet 中为可选冗余信号，默认不启用。

详见 [10. Claude Code Adapter](#10-claude-code-adapter)、[11. Codex Adapter](#11-codex-adapter)、[29. 参考资料](#29-参考资料)。

### 2.2 v0.1 不做

```text
v0.1 不包含：
├─ 不做 Hatch Pet / AI 宠物制作器
├─ 不做在线宠物市场
├─ 不做逐帧动画编辑器
├─ 不做复杂物理交互
├─ 不做 Cursor / ZCode 深度完整接入
├─ 不做云同步
├─ 不做插件市场
├─ 不做移动端提醒
└─ 不做多机器状态同步
```

### 2.3 v0.2 规划

```text
v0.2 macOS 适配
├─ macOS 透明悬浮窗适配
├─ macOS 通知权限引导
├─ macOS 菜单栏入口
├─ Retina 缩放处理
├─ 多显示器位置恢复
├─ macOS Codex pet 路径扫描
├─ Claude / Codex macOS hook 脚本适配（sidecar 跨平台编译）
└─ Cursor Background Agent Webhook 接入
```

### 2.4 v0.3+ 规划

```text
v0.3+
├─ AgentPet-native Pet Pack
├─ Zip / .agentpet 宠物包导入
├─ 宠物制作 Hatch Studio
├─ 参考图生成宠物
├─ 动作行生成
├─ 自动切帧 / 校验 / 合成 spritesheet
├─ 自定义声音包
├─ Cursor / ZCode 深度适配
└─ 插件化 Adapter SDK
```

---

## 3. 技术选型

### 3.1 桌面框架

采用：

```text
Tauri v2
Rust backend
TypeScript frontend
Canvas 2D pet renderer
SQLite local store
JSON / TOML config files
```

选择 Tauri v2 的原因：

```text
1. 跨平台能力优先，v0.1 Windows，v0.2 macOS。
2. Rust 侧适合做本地事件服务、文件校验、配置 patch、进程和窗口管理。
3. Web 前端适合做设置页、状态面板、宠物预览。
4. 比 Electron 更轻量。
5. 支持透明窗口、托盘、通知、sidecar 等桌面能力。
```

### 3.2 前端技术

推荐：

```text
Vite
React + TypeScript
Canvas 2D
Zustand / Jotai 状态管理（仅作窗口内本地缓存，状态以 Rust 后端为准，见 §4.3）
```

桌宠动画部分建议使用 Canvas 2D，而不是 GIF 或 CSS sprite。原因：

```text
1. 更容易控制 DPI 缩放。
2. 更容易支持 pixelated / smooth 两种渲染。
3. 更容易做 frame crop。
4. 更容易叠加气泡、badge、提醒状态。
5. 后续支持多层动画和 hit-test 更方便。
```

### 3.3 Rust 后端

推荐核心依赖方向：

```text
tauri
tokio
axum / hyper
serde / serde_json
toml_edit
rusqlite 或 sqlx
image（WebP 解码需验证，必要时换 webp / libwebp 后端，见 §14.4）
directories
uuid
sha2
tracing
```

v0.1 先把本地事件服务放在 Tauri 主进程内。后续若要拆成独立 `agentpetd`，可以使用 Tauri sidecar 机制把外部二进制随应用打包。

### 3.4 编译型事件 sidecar（新增）

hook 是热路径，且 Codex hook 同步执行，因此**不用 `powershell.exe` 跑热路径**，而是用一个极小的编译型二进制：

```text
名称：agentpet-event（Windows 下 agentpet-event.exe）
语言：Rust 或 Go（静态链接，~2MB，无运行时依赖）
职责（唯一）：
  1. 读取事件来源：优先 stdin（hooks），无 stdin 数据时回退到最后一个 argv（Codex notify）。
  2. 读取 %APPDATA%\AgentPet\runtime.json 与 agentpet.token。
  3. 解析最小字段（source / adapter / event / session_id / cwd）并组装统一事件。
  4. fire-and-forget POST 到 127.0.0.1:<port>/api/v1/events（超时 ≤ 300ms），不等待响应体，立即退出。
安全约束：
  - 只 POST 到 127.0.0.1。
  - 绝不执行 payload 中的任何命令，不访问远程地址，不读写项目文件或 agent 配置。
```

CLI 契约：

```text
agentpet-event --source <claude-code|codex|generic> --adapter <hook|notify|wrapper> --event <EventName>
```

`.ps1` 桥接脚本（§10.3、§11.2）保留作为未部署 sidecar 时的 fallback，行为一致。

---

## 4. 总体架构

### 4.1 进程结构

v0.1 推荐单进程架构：

```text
AgentPet.exe
├─ Tauri Runtime
├─ Rust Core
│  ├─ Local Event HTTP Server
│  ├─ Adapter Installer
│  ├─ Session Registry
│  ├─ Event Store
│  ├─ Pet Package Manager
│  ├─ Reaction Engine
│  ├─ Notification Service
│  ├─ Sound Service
│  └─ Window Controller
└─ Webview Frontend
   ├─ pet-overlay
   ├─ status-panel
   └─ settings
```

> 注意：`agentpet-event(.exe)` sidecar 是一个**独立的、由 hook 短暂拉起的小二进制**，与上面的常驻服务不是同一进程；它只负责把事件 POST 回常驻服务，不是 daemon。

后续可演进为：

```text
AgentPet Desktop
└─ agentpetd sidecar
   ├─ event receiver
   ├─ session registry
   └─ notification broker
```

v0.1 不建议一开始拆 daemon，理由是安装、权限、升级、日志、退出生命周期都会复杂化。Tauri 主进程常驻托盘即可满足第一版需求。

### 4.2 窗口结构

```text
Window 1: pet-overlay
├─ 透明
├─ 无边框
├─ always-on-top
├─ skip-taskbar
├─ 不可调整尺寸
├─ 可拖动
└─ Canvas 渲染桌宠

Window 2: status-panel
├─ 普通窗口
├─ 展示所有 Agent Session
├─ 展示最近事件
├─ 支持标记已处理
└─ 支持尝试聚焦窗口（best-effort，见 §16.2）

Window 3: settings
├─ Adapter 管理
├─ Codex 宠物导入
├─ 宠物选择
├─ 声音配置
├─ 通知规则
├─ 安全 token 状态
└─ 日志 / 诊断
```

### 4.3 跨窗口状态一致性（新增）

三个窗口是**独立的 webview**，前端 store（Zustand/Jotai）**不跨窗口共享**。因此：

```text
1. Rust 后端是唯一 source of truth（session / 事件 / 设置）。
2. 后端状态变化通过 Tauri events 广播。
3. 各窗口订阅 Tauri events 并更新本地缓存 store。
4. 写操作（标记已处理、切换宠物、改设置）走 Tauri command → 后端 → 再广播回各窗口。
5. 前端 store 仅作渲染缓存，不作权威状态。
```

---

## 5. 核心模块设计

### 5.1 Rust 模块结构

```text
src-tauri/src/
├─ main.rs
├─ app_state.rs
├─ config/
│  ├─ settings.rs
│  ├─ paths.rs
│  └─ migration.rs
├─ event_bus/
│  ├─ server.rs
│  ├─ event.rs
│  ├─ normalizer.rs
│  └─ auth.rs
├─ session/
│  ├─ registry.rs
│  ├─ session.rs
│  └─ stale_detector.rs
├─ adapters/
│  ├─ mod.rs
│  ├─ claude.rs
│  ├─ codex.rs
│  ├─ generic_cli.rs
│  └─ installer.rs
├─ pet/
│  ├─ manifest.rs
│  ├─ package.rs
│  ├─ codex_importer.rs
│  ├─ validator.rs
│  ├─ reaction_engine.rs
│  └─ animation_state.rs
├─ notify/
│  ├─ toast.rs
│  ├─ sound.rs
│  └─ tray.rs
├─ store/
│  ├─ db.rs
│  ├─ events.rs
│  └─ sessions.rs
└─ window/
   ├─ overlay.rs
   ├─ focus.rs
   └─ platform_windows.rs
```

> 另有一个独立的 sidecar crate（`agentpet-event`），随应用打包，见 §3.4。

### 5.2 前端模块结构

```text
src/
├─ windows/
│  ├─ PetOverlay.tsx
│  ├─ StatusPanel.tsx
│  └─ Settings.tsx
├─ pet/
│  ├─ CanvasPetRenderer.ts
│  ├─ AnimationPlayer.ts
│  ├─ BubbleLayer.tsx
│  └─ PetPreview.tsx
├─ stores/                # 仅窗口内渲染缓存，权威状态在 Rust 后端（见 §4.3）
│  ├─ sessions.ts
│  ├─ settings.ts
│  ├─ pets.ts
│  └─ notifications.ts
├─ components/
│  ├─ PetLibrary.tsx
│  ├─ AdapterManager.tsx
│  ├─ SoundRuleEditor.tsx
│  ├─ EventLog.tsx
│  └─ Diagnostics.tsx
└─ api/
   ├─ tauriCommands.ts     # 写操作走 command
   └─ tauriEvents.ts       # 订阅后端广播
```

---

## 6. 本地事件系统

### 6.1 事件入口

v0.1 使用本地 HTTP Server：

```text
POST http://127.0.0.1:<port>/api/v1/events
GET  http://127.0.0.1:<port>/api/v1/health
```

默认端口：

```text
38388
```

端口冲突处理：

```text
1. 尝试 38388。
2. 冲突则选择随机可用端口。
3. 写入 runtime state 文件。
4. Sidecar / Hook 脚本从 runtime state 读取当前端口。
```

runtime state：

```json
{
  "port": 38388,
  "tokenPath": "C:\\Users\\<user>\\AppData\\Roaming\\AgentPet\\agentpet.token",
  "startedAt": "2026-06-16T10:00:00Z"
}
```

runtime.json 生命周期（修订 A4）：

```text
1. 启动时写入当前 port / pid / startedAt。
2. 优雅退出时删除 runtime.json（避免 sidecar 向死端口 POST 吃超时）。
3. 启动时若发现已有 runtime.json：
   - 校验其中的 pid 是否仍为 AgentPet 进程；不是则视为脏文件并覆盖。
4. Sidecar POST 失败（connection refused / 超时）一律静默退出码 0，绝不阻塞 agent。
5. Sidecar 超时上限 ≤ 300ms，防止端口被其他程序复用时长时间挂起同步的 Codex hook。
```

### 6.2 安全认证

每次启动生成或读取本地 token：

```text
%APPDATA%\AgentPet\agentpet.token
```

请求必须带：

```http
X-AgentPet-Token: <token>
```

安全规则：

```text
1. 只监听 127.0.0.1。
2. 不监听 0.0.0.0。
3. token 至少 32 bytes random。
4. token 文件仅当前用户可读。
5. payload 只当 JSON 数据。
6. 不执行 payload 中的任何命令。
7. 不允许 adapter 从远程 URL 自动安装脚本。
8. 事件 raw 字段只用于诊断和显示。
```

> 补充：浏览器跨源向 127.0.0.1 发带自定义头 `X-AgentPet-Token` 的 POST 会触发 CORS 预检，服务端不返回 CORS 头即被浏览器拦截；无 token 的请求返回 401。双重防护，无需额外处理。

### 6.3 统一事件 Schema

```json
{
  "schema": "agentpet.event/v1",
  "eventId": "evt_01HX...",
  "source": "claude-code",
  "adapter": "hook",
  "agentName": "Claude",
  "event": "Stop",
  "state": "completed",
  "severity": "normal",
  "sessionId": "claude-code:7f3a1c2e-...",
  "projectPath": "D:\\work\\repo-a",
  "terminal": null,
  "windowTitle": null,
  "processId": null,
  "message": "Claude completed a response.",
  "timestamp": "2026-06-16T10:30:00.000Z",
  "raw": {}
}
```

> 字段说明：`sessionId` 优先取自 hook payload 的 `session_id`（见 §7.3）。`terminal` / `windowTitle` / `processId` 在 hook 路径通常为 `null`（hook 子进程拿不到），仅 Generic CLI wrapper 路径能可靠填充。

### 6.4 状态枚举

```ts
export type AgentUnifiedState =
  | "idle"
  | "running"
  | "thinking"
  | "tool_running"
  | "permission_request"
  | "needs_input"
  | "completed"
  | "error"
  | "stale";
```

> v0.1 实际可产生的状态：`running`（UserPromptSubmit）、`permission_request`、`needs_input`、`completed`、`error`、`stale`、`idle`。`thinking` / `tool_running` 需要 `PreToolUse`（默认关闭），v0.1 不产生。

### 6.5 严重级别

```ts
export type EventSeverity =
  | "debug"
  | "info"
  | "normal"
  | "warning"
  | "critical";
```

### 6.6 事件归一化（修订 C1 / C2 / A2 / C4）

不同工具的原始事件需要转成统一状态。

```text
# Claude Code（hooks，payload via stdin）
UserPromptSubmit                         -> running
Notification (matcher=idleprompt)        -> needs_input
Notification (matcher=permissionprompt)  -> 交给 PermissionRequest 处理（避免与下条重复）
Notification (matcher=authsuccess/其他)  -> info，保留 raw
PermissionRequest                        -> permission_request
Stop                                     -> completed
StopFailure                              -> error

# Codex（hooks，payload via stdin —— 主路径）
UserPromptSubmit                         -> running
PermissionRequest                        -> permission_request
Stop                                     -> completed
PostToolUse(Failure)                     -> error（可选，默认不开）
SubagentStop                             -> completed / running，按 payload 判断

# Codex（notify，payload via argv —— 可选冗余，默认不启用）
type = agent-turn-complete               -> completed
未知 notify payload                      -> completed，raw 保留

# Generic CLI（wrapper）
process exit 0                           -> completed
process exit != 0                        -> error

# 全局
No event for N minutes                   -> stale
```

去重原则（修订 C2 / A2）：

```text
1. 若同时启用 Codex notify 与 hooks，对同一完成事件去重：
   按 (source=codex, cwd, turn-id, 30s 时间窗) 合并，保留先到的，忽略后到的同义事件。
2. Claude 的 AskUserQuestion 会同时触发 PreToolUse 与 PermissionRequest；
   v0.1 PreToolUse 默认关闭，仅 PermissionRequest 生效，不会重复。
   （若以后开启 PreToolUse，需对同一 tool_call_id 去重。）
3. permissionprompt 的 Notification 不单独成事件，由 PermissionRequest 表达。
```

normalizer 容错原则：

```text
能明确判断则归一化。
不能明确判断则保留 raw，state 设为 running 或 info。
不要因为未知字段导致事件丢失。
```

---

## 7. Session Registry

### 7.1 Session 目标

Session Registry 解决多 Agent 并发问题。

```text
Claude / repo-a / session 7f3a... / completed
Codex  / repo-b / session ...     / needs_input
Cursor / repo-c / Background Agent / running
ZCode  / repo-d / GUI             / unknown
```

### 7.2 Session Schema

```json
{
  "sessionId": "claude-code:7f3a1c2e-...",
  "source": "claude-code",
  "agentName": "Claude",
  "projectPath": "D:\\work\\repo-a",
  "terminal": null,
  "windowTitle": null,
  "processId": null,
  "state": "completed",
  "severity": "normal",
  "createdAt": "2026-06-16T10:00:00Z",
  "updatedAt": "2026-06-16T10:30:00Z",
  "lastMessage": "Claude completed a response.",
  "unread": true,
  "acknowledged": false,
  "rawContext": {}
}
```

### 7.3 Session ID 生成策略（修订 C3）

优先级：

```text
1. source + payload.session_id          ← 主键
2. source + projectPath + processId      ← 仅 Generic CLI wrapper 路径可用
3. source + cwd + nearestTimeWindow      ← Codex notify 兜底（cwd 级别）
4. source + unknown + generated UUID
```

按工具的实际可用性：

```text
Claude Code：
  hook 每个事件的 stdin payload 都带 session_id / cwd / transcript_path。
  → 始终走优先级 1（session_id），同一 repo 的两个终端 tab 也能正确区分为两个 session。

Codex（hooks）：
  hook payload 经 stdin 同步传入，使用其中的会话标识 + cwd 作为主键。
  → 若 payload 提供稳定会话 ID 则走优先级 1；否则退到 cwd 级别。

Codex（notify，可选）：
  payload 带 cwd / turn-id / input-messages / last-assistant-message / type。
  turn-id 是“每 turn”而非“每 session”，不稳定。
  → 只能做到 cwd 级别识别（优先级 3）。
  ⚠ 限制：同一 cwd 下并行的两个 Codex 会话在 notify-only 模式下可能无法区分；
     需要区分时请使用 hooks 路径或 Generic CLI wrapper。
```

### 7.4 Session 生命周期

```text
created
running
attention
completed_unread
acknowledged
expired
```

默认保留策略：

```text
completed_unread 保留 30 分钟
acknowledged completed 保留 10 分钟
error / needs_input 保留到用户处理
running 超过 30 分钟无事件转 stale
stale 超过 2 小时无更新转 expired
```

---

## 8. 状态聚合规则

桌宠不直接显示某一个 Agent，而是显示全局聚合状态。

### 8.1 全局优先级

```text
permission_request  100
needs_input         100
error                90
completed_unread     70
stale                50
running              30
thinking             20
idle                  0
```

### 8.2 聚合输出

```ts
export type PetMood =
  | "idle"
  | "working"
  | "attention"
  | "happy"
  | "error"
  | "stale";
```

映射：

```text
permission_request -> attention
needs_input        -> attention
error              -> error
completed_unread   -> happy
stale              -> stale
running            -> working
thinking           -> working
idle               -> idle
```

> 因 §10.2 最小 hook 集已加入 `UserPromptSubmit`，`running -> working` 在 v0.1 可正常出现。

### 8.3 多事件冲突规则

例如：

```text
Claude completed
Codex needs_input
ZCode error
```

桌宠表现：

```text
PetMood: attention
Bubble: 2 个 Agent 需要注意
Panel 排序:
1. Codex needs_input
2. ZCode error
3. Claude completed
```

采用该规则的原因是：`needs_input` 和 `permission_request` 代表用户现在可以解除阻塞，产品价值最高；`error` 同样重要，但可以在同一个 attention 聚合中展示。

---

## 9. Adapter 设计

### 9.1 Adapter 能力接口

```ts
export interface AdapterCapability {
  id: string;
  displayName: string;
  platform: Array<"windows" | "macos" | "linux">;
  supportedEvents: Array<
    | "completed"
    | "needs_input"
    | "permission_request"
    | "error"
    | "running"
    | "stale"
  >;
  installModes: Array<"auto_patch" | "manual" | "wrapper">;
}
```

各 adapter 的真实能力（修订后）：

| Adapter | running | completed | needs_input | permission_request | error | 会话识别 | 窗口聚焦 |
|---|:--:|:--:|:--:|:--:|:--:|---|---|
| Claude Code (hooks) | ✓ | ✓ | ✓ | ✓ | ✓ | session_id（强） | 弱（仅闪烁） |
| Codex (hooks) | ✓ | ✓ | ✓(注1) | ✓ | ✓(可选) | cwd / 会话标识 | 弱（仅闪烁） |
| Codex (notify, 可选) | ✗ | ✓ | ✗ | ✗ | ✗ | cwd（弱） | 弱（仅闪烁） |
| Generic CLI (wrapper) | 启停 | ✓ | ✗ | ✗ | ✓ | PID（强） | 强（wrapper 掌握 PID/窗口） |

注1：Codex 的 needs_input 主要由 `agent-turn-complete` 表达“turn 结束等待输入”；细分的“正在向用户提问”取决于具体事件载荷。

### 9.2 Adapter 安装原则

所有 adapter 安装必须：

```text
1. 自动检测配置文件。
2. 读取当前配置。
3. 生成 patch。
4. 展示 patch diff。
5. 用户确认。
6. 备份原文件。
7. 写入配置。
8. 执行测试事件。
9. 支持一键恢复。
```

备份命名：

```text
settings.json.agentpet-backup-20260616-103000
config.toml.agentpet-backup-20260616-103000
```

---

## 10. Claude Code Adapter

### 10.1 接入策略

v0.1 使用 Claude Code hooks。

Claude Code 的 hooks 会在生命周期事件匹配时触发，command hook 的输入从 **stdin** 传入；每个事件的 payload 都包含 `session_id`、`transcript_path`、`cwd`、`hook_event_name`。

- `UserPromptSubmit` 表示用户提交了一个 prompt（turn 开始）。
- `Notification` 表示 Claude Code 发出通知（matcher 含 idleprompt / permissionprompt / authsuccess / elicitationdialog）。
- `PermissionRequest` 表示权限对话框出现。
- `Stop` 表示一次响应停止。
- `StopFailure` 表示 turn 因 API error 结束（matcher 含 rate_limit / overloaded / billing_error）。

执行方式（修订 C5）：

```text
1. 通知类 hook 采用异步（非阻塞）执行，绝不延迟 Claude。
2. 热路径调用编译型 sidecar agentpet-event.exe（快速 spawn + fire-and-forget POST）。
3. sidecar 不可用时回退到 PowerShell 桥接脚本（§10.3）。
4. sidecar / 脚本始终 exit 0，即使 POST 失败，避免 Stop hook 阻塞 Claude 收尾。
```

### 10.2 状态映射（修订 C4）

```text
SessionStart       -> running（可选）
UserPromptSubmit   -> running        ← v0.1 必需，提供“工作中”动画
PreToolUse         -> tool_running   ← 默认关闭（每次工具调用都触发，开销大）
PermissionRequest  -> permission_request
Notification       -> needs_input（按 matcher 路由，见 §6.6）
Stop               -> completed
StopFailure        -> error
```

v0.1 最小必需：

```text
UserPromptSubmit   ← 新增
Notification
PermissionRequest
Stop
StopFailure
```

### 10.3 事件桥接（sidecar 优先，PowerShell fallback）

**优先：编译型 sidecar**（见 §3.4），hook command 直接调用：

```text
"%APPDATA%\AgentPet\bin\agentpet-event.exe" --source claude-code --adapter hook --event Stop
```

sidecar 读取 stdin 的 hook JSON，从中解析 `session_id` / `cwd`，组装事件后 fire-and-forget POST。

**Fallback：PowerShell 桥接脚本**（未部署 sidecar 时）：

```powershell
# %APPDATA%\AgentPet\hooks\agentpet-event.ps1

param(
  [string]$Source,
  [string]$Adapter,
  [string]$Event
)

$ErrorActionPreference = "Stop"

$runtimePath = Join-Path $env:APPDATA "AgentPet\runtime.json"
$tokenPath = Join-Path $env:APPDATA "AgentPet\agentpet.token"

if (!(Test-Path $runtimePath) -or !(Test-Path $tokenPath)) {
  exit 0
}

$runtime = Get-Content $runtimePath -Raw | ConvertFrom-Json
$token = Get-Content $tokenPath -Raw

$stdin = [Console]::In.ReadToEnd()

try {
  $rawJson = $stdin | ConvertFrom-Json
} catch {
  $rawJson = @{
    parseError = $true
    raw = $stdin
  }
}

$body = @{
  schema = "agentpet.event/v1"
  source = $Source
  adapter = $Adapter
  event = $Event
  timestamp = (Get-Date).ToUniversalTime().ToString("o")
  raw = $rawJson
} | ConvertTo-Json -Depth 50

try {
  Invoke-RestMethod `
    -Method Post `
    -Uri "http://127.0.0.1:$($runtime.port)/api/v1/events" `
    -Headers @{ "X-AgentPet-Token" = $token.Trim() } `
    -Body $body `
    -ContentType "application/json" `
    -TimeoutSec 1 | Out-Null
} catch {
  exit 0
}
```

> 服务端从 `raw` 中提取 `session_id` / `cwd` 完成归一化与会话定位；sidecar 可在客户端预解析这两个字段以减少服务端工作量。

### 10.4 Claude settings patch 示例（含 UserPromptSubmit）

下例使用 sidecar；若用 PowerShell fallback，把 `command` 换成 `powershell.exe -NoProfile -ExecutionPolicy Bypass -File "...agentpet-event.ps1" -Source ... -Adapter hook -Event ...` 即可。

```json
{
  "hooks": {
    "UserPromptSubmit": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"%APPDATA%\\AgentPet\\bin\\agentpet-event.exe\" --source claude-code --adapter hook --event UserPromptSubmit"
          }
        ]
      }
    ],
    "Notification": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"%APPDATA%\\AgentPet\\bin\\agentpet-event.exe\" --source claude-code --adapter hook --event Notification"
          }
        ]
      }
    ],
    "PermissionRequest": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"%APPDATA%\\AgentPet\\bin\\agentpet-event.exe\" --source claude-code --adapter hook --event PermissionRequest"
          }
        ]
      }
    ],
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"%APPDATA%\\AgentPet\\bin\\agentpet-event.exe\" --source claude-code --adapter hook --event Stop"
          }
        ]
      }
    ],
    "StopFailure": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "\"%APPDATA%\\AgentPet\\bin\\agentpet-event.exe\" --source claude-code --adapter hook --event StopFailure"
          }
        ]
      }
    ]
  }
}
```

---

## 11. Codex Adapter

### 11.1 接入策略（修订 C1 / C2）

**v0.1 Codex 以 hooks 引擎为主路径，notify 为可选冗余。**

```text
P0（默认）：Codex hooks 引擎
  - 覆盖 completed / permission_request / running / error 全部所需状态。
  - payload 经 stdin 同步传入，复用与 Claude 相同的 sidecar / 桥接脚本。

P1（可选，默认关闭）：notify
  - 仅在 agent-turn-complete 触发，只能用于“完成提醒”。
  - 无法获取权限、报错、子任务事件。
  - 若用户主动开启，与 hooks 的 Stop 完成事件按 §6.6 去重。
```

> 关键事实（已核对官方文档）：外部 `notify` 命令当前**仅支持 `agent-turn-complete`**；权限请求属于内置 TUI 通知，不经由 `notify`。因此**不能用 `notify` 实现权限 / 报错提醒**，必须走 hooks。

### 11.2 Codex notify 配置（可选）

`notify` 是命令数组，Codex 把 JSON payload 作为**最后一个 argv** 传入。

```toml
notify = [
  "C:\\Users\\<user>\\AppData\\Roaming\\AgentPet\\bin\\agentpet-event.exe",
  "--source", "codex",
  "--adapter", "notify",
  "--event", "notify"
]
```

sidecar 检测：stdin 无数据时，读取最后一个 argv 作为 payload（agent-turn-complete JSON），POST 后退出。

**Fallback：`codex-notify.ps1`（直接 POST，不再二次 spawn，修订 C5）**

```powershell
param(
  [string]$Payload
)

# Codex 通过最后一个 argv 传入 JSON；兼容 stdin。
$stdin = [Console]::In.ReadToEnd()
if (-not [string]::IsNullOrWhiteSpace($Payload)) {
  $raw = $Payload
} elseif (-not [string]::IsNullOrWhiteSpace($stdin)) {
  $raw = $stdin
} else {
  $raw = "{}"
}

$runtimePath = Join-Path $env:APPDATA "AgentPet\runtime.json"
$tokenPath = Join-Path $env:APPDATA "AgentPet\agentpet.token"
if (!(Test-Path $runtimePath) -or !(Test-Path $tokenPath)) { exit 0 }
$runtime = Get-Content $runtimePath -Raw | ConvertFrom-Json
$token = Get-Content $tokenPath -Raw

try { $rawJson = $raw | ConvertFrom-Json } catch { $rawJson = @{ parseError = $true; raw = $raw } }

$body = @{
  schema = "agentpet.event/v1"
  source = "codex"
  adapter = "notify"
  event = "notify"
  timestamp = (Get-Date).ToUniversalTime().ToString("o")
  raw = $rawJson
} | ConvertTo-Json -Depth 50

try {
  Invoke-RestMethod -Method Post `
    -Uri "http://127.0.0.1:$($runtime.port)/api/v1/events" `
    -Headers @{ "X-AgentPet-Token" = $token.Trim() } `
    -Body $body -ContentType "application/json" -TimeoutSec 1 | Out-Null
} catch { exit 0 }
```

> 与 v0.1 不同：原方案让 notify 脚本再 pipe 拉起 bridge 脚本（两次 powershell spawn）。此处直接 POST，少一次进程启动。

### 11.3 Codex hooks 配置示例（修订 A3：commandWindows 用数组，去掉 [features]）

hooks 默认即启用，无需 `[features] hooks = true`。`command` / `commandWindows` 用**数组**形式。

```toml
[[hooks.UserPromptSubmit]]
matcher = "*"
[[hooks.UserPromptSubmit.hooks]]
type = "command"
command = ["sh", "-c", "true"]   # 占位；Windows 用下方 commandWindows
commandWindows = ["C:\\Users\\<user>\\AppData\\Roaming\\AgentPet\\bin\\agentpet-event.exe", "--source", "codex", "--adapter", "hook", "--event", "UserPromptSubmit"]

[[hooks.Stop]]
matcher = "*"
[[hooks.Stop.hooks]]
type = "command"
command = ["sh", "-c", "true"]
commandWindows = ["C:\\Users\\<user>\\AppData\\Roaming\\AgentPet\\bin\\agentpet-event.exe", "--source", "codex", "--adapter", "hook", "--event", "Stop"]

[[hooks.PermissionRequest]]
matcher = "*"
[[hooks.PermissionRequest.hooks]]
type = "command"
command = ["sh", "-c", "true"]
commandWindows = ["C:\\Users\\<user>\\AppData\\Roaming\\AgentPet\\bin\\agentpet-event.exe", "--source", "codex", "--adapter", "hook", "--event", "PermissionRequest"]
```

> 说明：`command`（非 Windows）与 `commandWindows`（Windows 覆盖）；TOML 别名 `command_windows` 同样被接受。Windows-first 下 `commandWindows` 生效，`command` 占位。Codex hook 同步执行并从 stdout 读响应，故 sidecar/脚本必须快速返回且 exit 0。

### 11.4 Codex 状态映射

```text
# hooks（主路径）
UserPromptSubmit          -> running
Stop                      -> completed
PermissionRequest         -> permission_request
PostToolUse(Failure)      -> error（可选，默认不开）
SubagentStop              -> completed / running，按 payload 判断

# notify（可选冗余，仅此一种）
type = agent-turn-complete -> completed
未知 notify payload        -> completed，raw 保留
```

去重：若 notify 与 hooks 同时开启，完成事件按 (cwd, turn-id, 30s 窗) 去重，见 §6.6。

### 11.5 Windows / WSL 注意事项

Codex Windows 和 WSL 的配置路径可能不同。v0.1 Windows-first 时优先处理：

```text
Windows native:
%USERPROFILE%\.codex\config.toml
%USERPROFILE%\.codex\pets

WSL:
$HOME/.codex/config.toml
$HOME/.codex/pets
```

如果用户希望 WSL 使用 Windows 侧 Codex home，可以通过：

```bash
export CODEX_HOME=/mnt/c/Users/<windows-user>/.codex
```

AgentPet 只负责检测并提示，不应静默修改 WSL shell profile。

> WSL 场景下，sidecar（Windows .exe）无法直接被 WSL 内的 Codex 调用；WSL 内应使用 Linux 版 sidecar 或脚本，并 POST 到 Windows 宿主的 127.0.0.1（经由 WSL2 的 localhost 转发）。v0.1 优先保证 Windows native；WSL 作为已知限制记录。

---

## 12. Generic CLI Adapter

v0.1 提供基础 wrapper：

```powershell
agentpet run --source generic --name aider -- aider
agentpet run --source claude-code -- claude
agentpet run --source codex -- codex
```

功能：

```text
1. 记录进程启动。
2. 记录 cwd。
3. 记录 processId。
4. 记录 terminal/windowTitle（wrapper 路径可可靠获取）。
5. 进程退出码 0 -> completed。
6. 进程退出码非 0 -> error。
7. 超时无输出 -> stale。
```

> 这不是 Claude / Codex 的主路径，但有两个独特价值：
> （a）对未知 CLI agent 的兜底；
> （b）**唯一能可靠拿到 PID / 终端 / 窗口标题的路径**，因此“聚焦窗口”功能在 wrapper 启动的会话上成功率最高（见 §16.2）。

---

## 13. Pet Runtime 设计

### 13.1 设计目标

Pet Runtime 要支持两类资源：

```text
1. Codex-compatible Pet
   读取 Codex 的 pet.json + spritesheet.webp

2. AgentPet-native Pet
   读取 agentpet.pet.json，支持更丰富的动画、声音、reaction
```

v0.1 先实现：

```text
Codex Pet Importer
Codex spritesheet 播放
AgentPet manifest 自动生成
基础 reaction 映射
```

不实现：

```text
AI 生成宠物
逐帧编辑
动作自定义编辑器
复杂宠物脚本
```

### 13.2 Codex 宠物规格

Codex-compatible pet 的核心规格：

```text
atlas: 8 列 × 9 行
cell: 192 × 208 px
spritesheet: 1536 × 1872 px   （8×192=1536，9×208=1872，自洽）
格式: WebP 或 PNG
未使用 cell: 全透明
```

> ⚠ 该规格请在 M1 对照当前 hatch-pet skill 的 animation-rows 参考再次复核（行序 / 时长 / cell 尺寸可能随 Codex 版本演进）。xiaoling 样本作为标准回归 fixture。

Codex 当前 9 行动作：

| Row | State | Used Columns | Duration |
|---:|---|---:|---|
| 0 | `idle` | 0-5 | 280, 110, 110, 140, 140, 320 ms |
| 1 | `running-right` | 0-7 | 120 ms each, final 220 ms |
| 2 | `running-left` | 0-7 | 120 ms each, final 220 ms |
| 3 | `waving` | 0-3 | 140 ms each, final 280 ms |
| 4 | `jumping` | 0-4 | 140 ms each, final 280 ms |
| 5 | `failed` | 0-7 | 140 ms each, final 240 ms |
| 6 | `waiting` | 0-5 | 150 ms each, final 260 ms |
| 7 | `running` | 0-5 | 120 ms each, final 220 ms |
| 8 | `review` | 0-5 | 150 ms each, final 280 ms |

### 13.3 用户当前样本

用户当前样本的 `pet.json`：

```json
{
  "id": "xiaoling",
  "displayName": "小玲",
  "description": "Q版蓝紫色短发女仆小宠物，参考原版与Q版形象。。",
  "spritesheetPath": "spritesheet.webp"
}
```

该样本可以作为 v0.1 Codex Pet Importer 的标准测试样本：

```text
1. 读取 pet.json。
2. 定位同目录 spritesheet.webp。
3. 校验 1536 × 1872 atlas。
4. 校验 8 × 9 grid。
5. 生成 agentpet.pet.json。
6. 导入到 AgentPet 宠物库。
7. 播放 idle / running / waiting / failed / jumping。
```

### 13.4 动画状态机

```ts
interface AnimationState {
  id: string;
  row: number;
  cols: number[];
  durationsMs: number[];
  loop: boolean;
  next?: string;
  priority: number;
  interruptible: boolean;
}
```

播放规则：

```text
1. ReactionEngine 收到 PetMood。
2. 找到匹配 reaction。
3. 比较 priority。
4. 高优先级打断低优先级。
5. 相同优先级按 cooldown 处理。
6. loop=false 的动画播放完进入 next。
7. loop=true 的动画持续播放到状态变化。
```

> 渲染性能：idle 等静态/低频动画应降帧或在帧不变时暂停 requestAnimationFrame，避免常驻透明窗口持续吃 GPU/CPU（笔记本电量）。

### 13.5 默认动作优先级

```text
waiting / permission_request 100
failed                       90
jumping / completed          60
waving                       40
review                       30
running                      10
idle                          0
```

### 13.6 Codex 动作映射

```text
Codex row 0 idle            -> idle
Codex row 1 running-right   -> dragging_right
Codex row 2 running-left    -> dragging_left
Codex row 3 waving          -> light_notice / greeting
Codex row 4 jumping         -> completed
Codex row 5 failed          -> error
Codex row 6 waiting         -> needs_input / permission_request
Codex row 7 running         -> running
Codex row 8 review          -> thinking / stale
```

### 13.7 用户交互

```text
单击桌宠：
打开状态面板

双击桌宠：
聚焦最近一个 needs_input / error / completed_unread session（best-effort，见 §16.2）

右键桌宠：
打开菜单

拖动桌宠：
移动位置，播放 running-left / running-right

悬停桌宠：
显示当前聚合状态气泡
```

---

## 14. Codex Pet Importer

### 14.1 支持输入

```text
输入方式：
├─ 选择单个 Codex pet 文件夹
├─ 扫描 Codex pets 根目录
├─ 拖入文件夹
└─ v0.2+ 支持 zip
```

默认扫描路径：

```text
Windows:
%USERPROFILE%\.codex\pets

macOS:
$HOME/.codex/pets

Linux:
$HOME/.codex/pets
```

### 14.2 文件结构

```text
xiaoling/
├─ pet.json
└─ spritesheet.webp
```

### 14.3 Codex manifest 类型

```rust
#[derive(Debug, Deserialize)]
pub struct CodexPetManifest {
    pub id: String,

    #[serde(rename = "displayName")]
    pub display_name: String,

    pub description: Option<String>,

    #[serde(rename = "spritesheetPath")]
    pub spritesheet_path: String,
}
```

### 14.4 导入校验（含 WebP 解码鲁棒性，修订 A6）

```text
pet.json 校验：
├─ 合法 JSON
├─ id 必填
├─ displayName 必填
├─ spritesheetPath 必填
├─ spritesheetPath 必须是相对路径
├─ 禁止 ../
├─ 禁止绝对路径
└─ 禁止远程 URL

spritesheet 校验：
├─ 文件存在
├─ 可被真实解码
├─ 格式为 WebP 或 PNG
├─ v0.1 优先支持 WebP
├─ 尺寸必须为 1536 × 1872
├─ grid 必须为 8 × 9
├─ cell 必须为 192 × 208
├─ 必须支持透明
├─ used cells 非空
└─ unused cells 应为空或透明
```

WebP 解码注意：

```text
1. Rust image crate 的 WebP（尤其 lossless / animated）支持历史上不完整。
2. M1 用 xiaoling 真实 spritesheet 验证解码是否成功。
3. 解码失败或不支持时，回退到 libwebp 后端（webp crate）。
4. 解码失败给出明确错误，不静默导入空图。
```

### 14.5 导入落盘

Windows：

```text
%APPDATA%\AgentPet\pets\xiaoling\
├─ agentpet.pet.json
├─ spritesheet.webp
└─ original\
   ├─ pet.json
   └─ spritesheet.webp
```

macOS：

```text
~/Library/Application Support/AgentPet/pets/xiaoling/
├─ agentpet.pet.json
├─ spritesheet.webp
└─ original/
   ├─ pet.json
   └─ spritesheet.webp
```

不直接引用 Codex 原目录，原因：

```text
1. Codex 删除或更新资源不影响 AgentPet。
2. AgentPet 可以生成自己的 manifest。
3. 可以记录导入版本。
4. 可以做 SHA256 去重。
5. 可以做迁移和回滚。
```

### 14.6 AgentPet manifest 自动生成

```json
{
  "schema": "agentpet.pet/v1",
  "sourceFormat": "codex.pet",
  "id": "xiaoling",
  "displayName": "小玲",
  "description": "Q版蓝紫色短发女仆小宠物，参考原版与Q版形象。。",
  "assets": {
    "spritesheet": "spritesheet.webp"
  },
  "renderer": {
    "type": "spritesheet",
    "frameWidth": 192,
    "frameHeight": 208,
    "columns": 8,
    "rows": 9,
    "rendering": "pixelated",
    "defaultScale": 1.0,
    "minScale": 0.5,
    "maxScale": 3.0
  },
  "animations": {
    "idle": {
      "row": 0,
      "cols": [0, 1, 2, 3, 4, 5],
      "durationsMs": [280, 110, 110, 140, 140, 320],
      "loop": true,
      "priority": 0
    },
    "running_right": {
      "row": 1,
      "cols": [0, 1, 2, 3, 4, 5, 6, 7],
      "durationsMs": [120, 120, 120, 120, 120, 120, 120, 220],
      "loop": true,
      "priority": 20
    },
    "running_left": {
      "row": 2,
      "cols": [0, 1, 2, 3, 4, 5, 6, 7],
      "durationsMs": [120, 120, 120, 120, 120, 120, 120, 220],
      "loop": true,
      "priority": 20
    },
    "waving": {
      "row": 3,
      "cols": [0, 1, 2, 3],
      "durationsMs": [140, 140, 140, 280],
      "loop": false,
      "next": "idle",
      "priority": 40
    },
    "jumping": {
      "row": 4,
      "cols": [0, 1, 2, 3, 4],
      "durationsMs": [140, 140, 140, 140, 280],
      "loop": false,
      "next": "idle",
      "priority": 60
    },
    "failed": {
      "row": 5,
      "cols": [0, 1, 2, 3, 4, 5, 6, 7],
      "durationsMs": [140, 140, 140, 140, 140, 140, 140, 240],
      "loop": true,
      "priority": 90
    },
    "waiting": {
      "row": 6,
      "cols": [0, 1, 2, 3, 4, 5],
      "durationsMs": [150, 150, 150, 150, 150, 260],
      "loop": true,
      "priority": 100
    },
    "running": {
      "row": 7,
      "cols": [0, 1, 2, 3, 4, 5],
      "durationsMs": [120, 120, 120, 120, 120, 220],
      "loop": true,
      "priority": 10
    },
    "review": {
      "row": 8,
      "cols": [0, 1, 2, 3, 4, 5],
      "durationsMs": [150, 150, 150, 150, 150, 280],
      "loop": true,
      "priority": 30
    }
  },
  "reactions": [
    {
      "when": { "agentState": "idle" },
      "play": "idle"
    },
    {
      "when": { "agentState": "running" },
      "play": "running",
      "bubble": "{agentName} 正在工作"
    },
    {
      "when": { "agentState": "thinking" },
      "play": "review",
      "bubble": "{agentName} 正在思考"
    },
    {
      "when": { "agentState": "completed" },
      "play": "jumping",
      "bubble": "{agentName} 完成了"
    },
    {
      "when": { "agentState": "needs_input" },
      "play": "waiting",
      "bubble": "{agentName} 需要你确认"
    },
    {
      "when": { "agentState": "permission_request" },
      "play": "waiting",
      "bubble": "{agentName} 请求权限"
    },
    {
      "when": { "agentState": "error" },
      "play": "failed",
      "bubble": "{agentName} 出错了"
    },
    {
      "when": { "agentState": "stale" },
      "play": "review",
      "bubble": "{agentName} 好像卡住了"
    }
  ],
  "codexCompatibility": {
    "compatible": true,
    "atlas": "8x9-192x208",
    "originalManifest": "original/pet.json"
  }
}
```

---

## 15. 通知系统

### 15.1 通知输出

```text
Reaction Output
├─ Pet animation
├─ Bubble
├─ Sound
├─ Toast
├─ Tray badge
├─ Repeat reminder
└─ Status panel update
```

### 15.2 默认通知规则

```json
{
  "rules": [
    {
      "name": "任意 Agent 完成",
      "match": { "state": "completed" },
      "actions": {
        "animation": "jumping",
        "sound": "completed.wav",
        "toast": true,
        "bubble": "{agentName} 完成了"
      }
    },
    {
      "name": "任意 Agent 等待输入",
      "match": { "state": "needs_input" },
      "actions": {
        "animation": "waiting",
        "sound": "needs-input.wav",
        "toast": true,
        "sticky": true,
        "repeatEveryMs": 60000,
        "maxRepeat": 5,
        "bubble": "{agentName} 需要你确认"
      }
    },
    {
      "name": "任意 Agent 请求权限",
      "match": { "state": "permission_request" },
      "actions": {
        "animation": "waiting",
        "sound": "permission.wav",
        "toast": true,
        "sticky": true,
        "repeatEveryMs": 60000,
        "maxRepeat": 5,
        "bubble": "{agentName} 请求权限"
      }
    },
    {
      "name": "任意 Agent 报错",
      "match": { "state": "error" },
      "actions": {
        "animation": "failed",
        "sound": "error.wav",
        "toast": true,
        "bubble": "{agentName} 出错了"
      }
    }
  ]
}
```

### 15.3 声音策略（含勿扰 × sticky 规则，修订 A7）

```text
全局声音开关
状态级声音配置
Agent 级声音覆盖
时间段静音
勿扰模式
重复提醒控制
测试播放
```

默认：

```text
completed          播放一次
needs_input        立即播放，60 秒重复，最多 5 次
permission_request 立即播放，60 秒重复，最多 5 次
error              播放一次，可选重复
stale              只播放轻提示
```

勿扰 / 静音时段与 sticky 重复的交互：

```text
1. 勿扰时段内：
   - 声音静音（包括 sticky 的重复声音）。
   - Toast 与桌宠动作仍正常（视觉提醒不打扰他人）。
   - Tray badge / 状态面板照常更新。
2. 退出勿扰时段时：
   - 不补播积压的重复声音；只按当前未读状态播放一次汇总轻提示。
3. needs_input / permission_request 的 sticky 计数在勿扰期间暂停，
   退出勿扰后从剩余次数继续。
```

---

## 16. 状态面板

### 16.1 面板信息

```text
AgentPet 状态面板
├─ Agent 名称
├─ 来源工具
├─ 项目路径
├─ 当前状态
├─ 最后更新时间
├─ 最近消息
├─ 是否未读
├─ 是否已处理
├─ 终端 / 窗口标题（hook 路径通常为空）
├─ raw event 查看
└─ 操作按钮
```

示例：

```text
🟡 Codex    needs input     D:\repo-a     1 min ago
   请求执行命令，需要确认
   [尝试聚焦] [标记已处理] [查看 raw]

🔴 Claude   error           D:\repo-b     4 min ago
   StopFailure
   [尝试聚焦] [标记已处理] [查看 raw]

✅ Claude   completed       D:\repo-c     8 min ago
   任务完成
   [尝试聚焦] [清除]
```

### 16.2 聚焦窗口（best-effort，修订 A1）

聚焦能力分层，**v0.1 不承诺总是成功**：

```text
可靠（wrapper 启动的会话）：
  Generic CLI wrapper 掌握 PID / 窗口句柄，
  可较可靠地定位并尝试激活窗口。

弱（hook 路径的 Claude / Codex 会话）：
  hook 子进程拿不到启动它的终端模拟器与窗口标题，
  无 PID / windowTitle 时无法定位窗口。

Windows 限制：
  后台进程通常无法直接 SetForegroundWindow 抢焦点；
  实际多为 FlashWindowEx 闪烁任务栏提示。
```

v0.1 聚焦逻辑：

```text
1. 若 session 有 processId（wrapper 路径），用 Win32 枚举找 hwnd 并尝试激活；
   被前台锁限制时退化为 FlashWindowEx。
2. 若仅有 windowTitle，尝试模糊匹配；同样可能只能闪烁。
3. 若都没有（典型 hook 路径），显示“无法定位窗口，请手动切回”。
```

要更高聚焦成功率，请用 wrapper 启动 agent：

```powershell
agentpet run --source claude-code -- claude
agentpet run --source codex -- codex
```

---

## 17. 数据存储

### 17.1 目录结构

Windows：

```text
%APPDATA%\AgentPet\
├─ settings.json
├─ runtime.json            （退出时删除，见 §6.1）
├─ agentpet.token
├─ agentpet.db
├─ bin\
│  └─ agentpet-event.exe   （编译型事件 sidecar）
├─ logs\
│  └─ agentpet.log
├─ hooks\
│  ├─ agentpet-event.ps1   （fallback 桥接）
│  ├─ codex-notify.ps1
│  └─ generic-event.ps1
├─ pets\
│  └─ xiaoling\
│     ├─ agentpet.pet.json
│     ├─ spritesheet.webp
│     └─ original\
│        ├─ pet.json
│        └─ spritesheet.webp
└─ sounds\
   ├─ completed.wav
   ├─ needs-input.wav
   ├─ permission.wav
   └─ error.wav
```

macOS v0.2：

```text
~/Library/Application Support/AgentPet/
├─ settings.json
├─ runtime.json
├─ agentpet.token
├─ agentpet.db
├─ bin/
│  └─ agentpet-event
├─ logs/
├─ hooks/
├─ pets/
└─ sounds/
```

### 17.2 SQLite 表

```sql
CREATE TABLE events (
  id TEXT PRIMARY KEY,
  session_id TEXT,
  source TEXT NOT NULL,
  adapter TEXT NOT NULL,
  event TEXT NOT NULL,
  state TEXT NOT NULL,
  severity TEXT NOT NULL,
  project_path TEXT,
  message TEXT,
  raw_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE TABLE sessions (
  id TEXT PRIMARY KEY,
  source TEXT NOT NULL,
  agent_name TEXT,
  project_path TEXT,
  terminal TEXT,
  window_title TEXT,
  process_id INTEGER,
  state TEXT NOT NULL,
  severity TEXT NOT NULL,
  last_message TEXT,
  unread INTEGER NOT NULL DEFAULT 1,
  acknowledged INTEGER NOT NULL DEFAULT 0,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE pets (
  id TEXT PRIMARY KEY,
  display_name TEXT NOT NULL,
  source_format TEXT NOT NULL,
  install_dir TEXT NOT NULL,
  manifest_path TEXT NOT NULL,
  spritesheet_path TEXT NOT NULL,
  sha256 TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
```

---

## 18. 设置文件

```json
{
  "schema": "agentpet.settings/v1",
  "currentPetId": "xiaoling",
  "pet": {
    "scale": 1.0,
    "alwaysOnTop": true,
    "rememberPosition": true,
    "position": {
      "displayId": "primary",
      "x": 1200,
      "y": 780
    }
  },
  "eventServer": {
    "preferredPort": 38388,
    "bindHost": "127.0.0.1"
  },
  "eventBridge": {
    "mode": "sidecar",
    "sidecarPath": "%APPDATA%\\AgentPet\\bin\\agentpet-event.exe"
  },
  "notifications": {
    "toastEnabled": true,
    "soundEnabled": true,
    "quietHoursEnabled": false,
    "quietHours": {
      "start": "23:00",
      "end": "08:00"
    }
  },
  "adapters": {
    "claude-code": {
      "enabled": true,
      "installMode": "auto_patch"
    },
    "codex": {
      "enabled": true,
      "installMode": "auto_patch",
      "useHooks": true,
      "useNotify": false
    }
  }
}
```

---

## 19. UI 设计

### 19.1 设置页结构

```text
设置
├─ 总览
│  ├─ 当前桌宠
│  ├─ 事件服务状态
│  ├─ 已启用 Adapter
│  └─ 最近事件
│
├─ 宠物
│  ├─ 当前宠物预览
│  ├─ 导入 Codex 宠物
│  ├─ 扫描 Codex pets 目录
│  ├─ 动作预览
│  ├─ 缩放比例
│  └─ 删除 / 重新导入
│
├─ Agent Adapter
│  ├─ Claude Code
│  ├─ Codex（hooks / notify 开关）
│  ├─ Generic CLI
│  └─ 诊断
│
├─ 通知
│  ├─ Toast 开关
│  ├─ 声音开关
│  ├─ 每状态音效
│  ├─ 重复提醒
│  └─ 勿扰时间
│
├─ 安全
│  ├─ 本地服务地址
│  ├─ token 状态
│  ├─ 配置修改历史
│  └─ 恢复备份
│
└─ 日志
   ├─ 事件日志
   ├─ Adapter 日志
   ├─ 导入日志
   └─ 导出诊断包
```

### 19.2 宠物导入 UI

```text
宠物 > 导入 Codex 宠物

[扫描 Codex pets 目录]
[选择文件夹]

发现：
┌──────────┬──────────┬──────────────┬────────┐
│ 预览     │ 名称     │ ID           │ 状态   │
├──────────┼──────────┼──────────────┼────────┤
│ 小图     │ 小玲     │ xiaoling     │ 可导入 │
└──────────┴──────────┴──────────────┴────────┘

[导入]
```

导入结果：

```text
导入成功：小玲

格式：Codex-compatible
spritesheet：spritesheet.webp
尺寸：1536 × 1872
Grid：8 × 9
Cell：192 × 208
透明背景：通过
AgentPet manifest：已生成

[设为当前宠物] [预览动作] [打开目录]
```

---

## 20. Windows 平台实现细节

### 20.1 透明桌宠窗口

配置方向：

```json
{
  "label": "pet-overlay",
  "transparent": true,
  "decorations": false,
  "alwaysOnTop": true,
  "skipTaskbar": true,
  "resizable": false,
  "visible": true
}
```

实现注意：

```text
1. Windows 上透明 WebView 可能受 GPU / WebView2 影响。
2. always-on-top 需要在窗口创建后再次确认。
3. 必要时 Rust 侧通过 Win32 API 设置 HWND_TOPMOST。
4. 桌宠窗口尺寸根据 pet frame + scale 动态计算。
5. 鼠标穿透 v0.1 默认关闭。
```

### 20.2 窗口尺寸计算

```text
windowWidth  = frameWidth  * scale + safePadding * 2 + bubbleExtraWidth
windowHeight = frameHeight * scale + safePadding * 2 + bubbleExtraHeight
```

默认：

```text
frameWidth = 192
frameHeight = 208
safePadding = 16
bubbleExtraHeight = 48
```

### 20.3 DPI

```text
Canvas CSS size:
192 × 208 × scale

Canvas backing store:
cssWidth × devicePixelRatio
cssHeight × devicePixelRatio
```

渲染：

```ts
ctx.imageSmoothingEnabled = rendering === "smooth";
```

pixel pet：

```ts
ctx.imageSmoothingEnabled = false;
```

non-pixel pet：

```ts
ctx.imageSmoothingEnabled = true;
```

### 20.4 Toast 通知（打包要求）

```text
1. Windows 安装态 Toast 依赖已注册的 AppUserModelID（由安装器注册开始菜单快捷方式）。
2. 开发模式与安装模式表现不同，验收必须在 installer 安装态下测试 Toast。
3. Toast 失败时，声音与桌宠动作仍需生效（needs_input 不能只依赖 Toast）。
```

---

## 21. 安全与隐私

### 21.1 安全原则

```text
1. 默认纯本地。
2. 不上传事件内容。
3. 不上传 projectPath。
4. 不上传 raw payload。
5. 本地 HTTP 只绑定 127.0.0.1。
6. 请求必须带 token。
7. token 文件限制当前用户可读。
8. adapter 安装必须显示 diff。
9. 配置写入必须备份。
10. 宠物包不允许执行代码。
11. 事件 sidecar 只 POST 127.0.0.1，绝不执行 payload 命令、不访问远程、不读写项目文件或 agent 配置。
```

### 21.2 宠物包安全

v0.1 Codex pet importer 只允许：

```text
pet.json
spritesheet.webp
spritesheet.png
license.txt，可选
preview.png，可选
```

禁止：

```text
.exe
.dll
.ps1
.bat
.cmd
.vbs
.js
.mjs
.wasm
.url
.lnk
```

路径限制：

```text
禁止绝对路径
禁止 ../
禁止远程 URL
禁止 symlink 指向外部
```

### 21.3 Hook 安全

Hook 脚本 / sidecar 只做：

```text
1. 读取 stdin（hooks）或 argv（Codex notify）。
2. 读取 runtime.json。
3. 读取 token。
4. POST 到 127.0.0.1。
```

禁止：

```text
1. 执行 payload 中的命令。
2. 调用远程地址。
3. 修改项目文件。
4. 修改 agent 配置。
5. 读取敏感文件。
```

---

## 22. 日志与诊断

### 22.1 日志类别

```text
agentpet.log
├─ app lifecycle
├─ event server
├─ adapter installer
├─ pet importer
├─ notification service
├─ sound service
└─ errors
```

### 22.2 诊断包

用户可以导出：

```text
diagnostics-20260616.zip
├─ settings.redacted.json
├─ runtime.redacted.json
├─ events-last-100.redacted.json
├─ sessions.redacted.json
├─ adapter-status.json
├─ pet-import-report.json
└─ logs.redacted.txt
```

脱敏规则：

```text
projectPath 可选脱敏
token 必须脱敏
raw payload 默认脱敏
用户 home 路径默认替换为 <HOME>
```

---

## 23. 测试方案

### 23.1 Codex Pet Importer 测试

```text
正常：
├─ 导入 xiaoling
├─ pet.json 正确解析
├─ spritesheet.webp 正确定位与解码（含 libwebp fallback 验证）
├─ atlas 尺寸 1536 × 1872
├─ cell 尺寸 192 × 208
├─ agentpet.pet.json 生成
├─ idle preview 正常
├─ waiting preview 正常
├─ failed preview 正常
└─ 设置当前宠物成功

异常：
├─ pet.json 缺少 id
├─ pet.json 缺少 displayName
├─ pet.json 缺少 spritesheetPath
├─ spritesheetPath 不存在
├─ spritesheetPath 使用 ../
├─ spritesheetPath 是绝对路径
├─ spritesheet.webp 损坏 / 解码失败
├─ spritesheet 尺寸错误
├─ 重复导入
└─ 删除后重新导入
```

### 23.2 Pet Runtime 测试

```text
idle 循环
running 循环（由 UserPromptSubmit 触发）
completed 播放 jumping 后回 idle
needs_input 播放 waiting 并循环
permission_request 播放 waiting 并循环
error 播放 failed 并循环
completed 不打断 needs_input
error 可以打断 running
needs_input 可以打断 completed
拖动向右播放 running_right
拖动向左播放 running_left
```

### 23.3 Event Server 测试

```text
无 token 请求 -> 401
错误 token -> 401
正确 token -> 200
非法 JSON -> 400 或记录 parseError
Claude UserPromptSubmit -> running
Claude Notification(idleprompt) -> needs_input
Claude PermissionRequest -> permission_request
Claude Stop -> completed
Claude StopFailure -> error
Codex hooks UserPromptSubmit -> running
Codex hooks Stop -> completed
Codex hooks PermissionRequest -> permission_request
Codex notify agent-turn-complete -> completed
notify + hooks 同时开启 -> 完成事件去重，无重复 session
同一 repo 两个 Claude tab -> 两个独立 session（session_id 区分）
多事件并发 -> session registry 正确
stale timer -> 正确转 stale
```

### 23.4 Adapter Installer 测试

```text
Claude settings 不存在 -> 创建或提示
Claude settings 已存在 -> patch merge（含 UserPromptSubmit）
Claude settings JSON 非法 -> 拒绝修改
Codex config 不存在 -> 创建或提示
Codex config 已存在 -> 插入 hooks（commandWindows 数组）
Codex notify 默认不写入；开启后才写入
重复安装 -> 幂等
卸载 adapter -> 恢复或删除 patch
备份文件生成
测试通知成功
```

### 23.5 事件桥接测试（新增）

```text
sidecar：stdin（hooks）路径正确 POST
sidecar：argv（Codex notify）路径正确 POST
sidecar：POST 失败（服务未启动）-> 静默 exit 0，不阻塞
sidecar：超时 ≤ 300ms 生效
PowerShell fallback：与 sidecar 行为一致
runtime.json 已删除时 -> sidecar 立即 exit 0
runtime.json 端口为脏（被他人占用）-> 不长时间挂起
codex-notify.ps1 不再二次 spawn powershell
```

### 23.6 Windows 桌面测试

```text
透明窗口正常
不会出现在任务栏
托盘菜单正常
关闭窗口不退出应用
退出托盘菜单能退出应用
always-on-top 正常
多显示器位置保存
DPI 100% / 125% / 150% 正常
声音播放正常
Toast 在安装包模式正常
聚焦：wrapper 会话可定位；hook 会话退化为闪烁 / 提示
退出后 runtime.json 删除、事件服务停止
```

---

## 24. v0.1 验收标准

```text
1. 用户安装并启动 AgentPet 后，可以看到透明桌宠。
2. 托盘入口可打开设置页和状态面板。
3. 用户可以从 Codex pets 目录导入“小玲”。
4. 导入后可以预览 idle / running / completed / needs_input / error。
5. 用户可以把“小玲”设为当前桌宠。
6. Claude Code 用户提交 prompt 后桌宠进入 running 动画；完成响应后产生 completed 事件、声音、Toast、桌宠动作和状态面板记录。
7. Claude Code 出现权限或输入提示时，AgentPet 进入 waiting 动画并重复提醒。
8. Codex（hooks）用户提交后进入 running；完成后产生 completed 事件、声音、Toast、桌宠动作和状态面板记录。
9. Codex（hooks）权限请求时进入 waiting 动画并重复提醒（注：notify-only 模式不具备此能力）。
10. 多个 Agent 同时运行时，状态面板能显示多个 session；同一 repo 的两个 Claude 会话显示为两个独立 session。
11. needs_input / permission_request 优先于 completed 显示。
12. 用户可以静音声音；勿扰时段内 sticky 重复声音被抑制、视觉提醒保留。
13. 用户可以关闭重复提醒。
14. 用户可以卸载 Claude / Codex adapter patch 并恢复原配置。
15. 双击桌宠尝试聚焦最近需处理会话：wrapper 会话可定位，hook 会话退化为闪烁或提示（不作为硬性成功项）。
16. AgentPet 退出后，runtime.json 删除、本地事件服务停止。
17. 所有配置修改均有备份。
```

---

## 25. 开发里程碑

### Milestone 0：项目骨架

```text
目标：
建立 Tauri v2 + Rust + React + TS 基础项目。

交付：
├─ pet-overlay 空窗口
├─ status-panel 空窗口
├─ settings 空窗口
├─ 托盘菜单
├─ app data path 初始化
├─ 跨窗口事件总线（Tauri events）骨架
└─ 日志系统
```

### Milestone 1：Pet Runtime + Codex Importer

```text
目标：
能导入并播放 Codex 宠物。

交付：
├─ Codex pet.json parser
├─ spritesheet.webp 解码（含 image / libwebp fallback 验证）
├─ atlas 校验（对照当前 hatch-pet skill 复核规格）
├─ agentpet.pet.json 生成
├─ Pet Library
├─ CanvasPetRenderer（含 idle 降帧）
├─ 动画状态机
├─ 动作预览
└─ 当前宠物切换
```

### Milestone 2：Event Server + Session Registry

```text
目标：
能接收本地事件并更新 session。

交付：
├─ 127.0.0.1 HTTP server
├─ token auth
├─ event schema
├─ event normalizer（含 §6.6 路由与去重）
├─ session_id 优先的 session 主键策略
├─ SQLite event store
├─ session registry
├─ stale detector
├─ runtime.json 生命周期（退出删除 / 脏读处理）
└─ status-panel 显示 session（经 Tauri events 同步）
```

### Milestone 3：Reaction Engine + 通知

```text
目标：
事件能驱动桌宠、声音、Toast、托盘状态。

交付：
├─ reaction rules
├─ PetMood 聚合
├─ 动画切换
├─ 声音播放
├─ Toast
├─ repeat reminder
├─ unread badge
└─ quiet hours（含 sticky 交互规则）
```

### Milestone 4：事件 Sidecar + Claude Adapter

```text
目标：
Claude Code 完成、工作中、等待输入、权限确认、错误能触发 AgentPet。

交付：
├─ 编译型 sidecar agentpet-event（stdin/argv，fire-and-forget POST）
├─ PowerShell fallback 桥接
├─ Claude config 检测
├─ Claude settings patch（含 UserPromptSubmit，异步 hook）
├─ UserPromptSubmit -> running
├─ Notification(idleprompt) -> needs_input
├─ PermissionRequest -> permission_request
├─ Stop -> completed
├─ StopFailure -> error
├─ 测试事件
└─ 卸载 / 恢复
```

### Milestone 5：Codex Adapter

```text
目标：
Codex hooks（主）/ notify（可选）能触发 AgentPet。

交付：
├─ Codex config 检测
├─ hooks patch（commandWindows 数组，UserPromptSubmit / Stop / PermissionRequest）
├─ notify patch（可选，默认关闭，直接 POST）
├─ UserPromptSubmit -> running
├─ Stop -> completed
├─ PermissionRequest -> permission_request
├─ notify + hooks 去重
├─ 测试事件
└─ 卸载 / 恢复
```

### Milestone 6：Windows 稳定化

```text
目标：
打磨 Windows-first 体验。

交付：
├─ always-on-top 稳定性
├─ DPI 修复
├─ 多显示器位置恢复
├─ 安装包（注册 AppUserModelID）
├─ Toast 安装包验证
├─ 聚焦 best-effort / 闪烁退化
├─ 日志导出
├─ 错误提示文案
└─ v0.1 验收测试
```

---

## 26. 风险与应对

### 26.1 Hook 配置格式演进

风险：Claude / Codex 的 hook 配置格式可能演进。  
应对：

```text
1. Adapter 版本化。
2. 配置 patch 幂等。
3. 保留手动安装说明。
4. Adapter 安装前做配置检测。
5. 失败时不阻断主程序。
```

### 26.2 Codex notify / hooks payload 不稳定

风险：payload 字段可能变化；notify 仅 agent-turn-complete。  
应对：

```text
1. raw 永久保留。
2. normalizer 容错。
3. 未知 payload 仍记录为 info/completed。
4. 权限 / 报错只走 hooks，不依赖 notify。
5. 日志中显示 payload schema。
```

### 26.3 notify 与 hooks 完成事件重复

风险：同时启用时完成事件 double-fire。  
应对：

```text
1. 默认只启用 hooks。
2. 同时启用时按 (cwd, turn-id, 30s 窗) 去重。
3. UI 明确提示同时启用会去重。
```

### 26.4 同步 hook 延迟

风险：同步 hook（尤其 Codex）+ PowerShell 冷启动拖慢 agent。  
应对：

```text
1. 热路径用编译型 sidecar（快速 spawn）。
2. POST fire-and-forget，超时 ≤ 300ms。
3. Claude 用异步 hook。
4. sidecar 始终 exit 0，不阻塞收尾。
```

### 26.5 Windows Toast 不稳定

风险：开发模式和安装模式表现不同。  
应对：

```text
1. v0.1 必须用 installer 测试通知。
2. 注册 AppUserModelID。
3. Toast 失败时仍有声音和桌宠动作。
4. needs_input 不能只依赖 Toast。
```

### 26.6 窗口聚焦受限

风险：hook 路径拿不到窗口；Windows 限制抢焦点。  
应对：

```text
1. 可靠聚焦限定 wrapper 会话。
2. 抢焦点失败退化为 FlashWindowEx。
3. 无窗口信息时提示用户手动切回。
4. 不把聚焦成功作为硬性验收。
```

### 26.7 always-on-top 被覆盖

风险：桌宠可能被部分窗口盖住。  
应对：

```text
1. 定期或事件触发 reapply topmost。
2. display change / resume 后重设窗口层级。
3. Rust 侧 Win32 API 兜底。
```

### 26.8 宠物素材不符合规格 / WebP 解码

风险：导入失败或动画错位；WebP 解码不支持。  
应对：

```text
1. 严格校验。
2. WebP 解码 image / libwebp fallback。
3. 给出明确错误。
4. 保留预览页。
5. v0.1 只支持 Codex 标准 atlas。
```

---

## 27. 后续 Hatch Studio 预留

虽然 v0.1 不做宠物制作，但 v0.1 的 importer、validator、manifest generator 会成为 Hatch Studio 的最终验收链路。

未来流程：

```text
用户输入描述 / 参考图
        ↓
生成 base
        ↓
生成 9 个动作行
        ↓
本地切帧 / 去背景 / 合成
        ↓
生成 spritesheet.webp
        ↓
调用 CodexPetImporter.validate()
        ↓
生成 pet.json
        ↓
生成 agentpet.pet.json
        ↓
安装到 AgentPet
```

v0.1 需要提前留好的接口：

```rust
pub trait PetPackageImporter {
    fn detect(&self, path: &Path) -> anyhow::Result<DetectionResult>;
    fn validate(&self, path: &Path) -> anyhow::Result<ValidationReport>;
    fn import(&self, path: &Path) -> anyhow::Result<ImportResult>;
}
```

这样后续可以扩展：

```text
CodexPetImporter
AgentPetPackImporter
ZipPetImporter
HatchOutputImporter
```

---

## 28. 最终推荐落地顺序

第一阶段先做“能看到”：

```text
1. Tauri 窗口 + 托盘 + 跨窗口事件总线
2. Codex pet importer（含 WebP 解码验证）
3. Canvas 播放小玲 idle
4. 手动切换 running / waiting / failed / jumping
```

第二阶段做“能收到”：

```text
5. Event server
6. token auth
7. session_id 优先的 session registry
8. status panel + fake event tester
```

第三阶段做“能提醒”：

```text
9. reaction engine
10. sound（含勿扰规则）
11. Toast
12. repeat reminder
```

第四阶段做“能接 Agent”：

```text
13. 编译型 sidecar
14. Claude adapter（含 UserPromptSubmit，异步）
15. Codex adapter（hooks 为主）
16. adapter install / uninstall + backup / restore
```

第五阶段做“能发布”：

```text
17. installer（注册 AppUserModelID）
18. Windows notification installed-mode test
19. 多显示器 / DPI / 聚焦退化
20. v0.1 验收
```

v0.1 发布时，用户体验应该是：

```text
1. 用户安装 AgentPet。
2. AgentPet 自动启动托盘和桌宠。
3. 用户进入设置页，点击“扫描 Codex 宠物”。
4. AgentPet 找到 ~/.codex/pets 或 %USERPROFILE%\.codex\pets 下的小玲。
5. 用户点击导入并设为当前宠物。
6. 用户点击“安装 Claude Adapter”和“安装 Codex Adapter”。
7. AgentPet 展示配置 diff，用户确认（Codex 默认装 hooks）。
8. 用户在 Windows Terminal / Warp 里运行 Claude 或 Codex。
9. 用户提交任务时，小玲进入 running/review 动画。
10. Agent 完成时，小玲跳起，播放完成音，显示 Toast。
11. Agent 等待权限或输入时，小玲进入 waiting 动画，重复提醒。
12. 用户点开小玲，看到所有 Agent session 的状态。
```

---

## 29. 参考资料

- Claude Code Hooks 文档：<https://code.claude.com/docs/en/hooks>
  - 用于核实：事件 `UserPromptSubmit` / `Notification` / `PermissionRequest` / `Stop` / `StopFailure` 存在；command hook 输入经 stdin；公共字段含 `session_id` / `transcript_path` / `cwd`；支持异步 hook。
- Codex 配置参考：<https://developers.openai.com/codex/config-reference>
  - 用于核实：`[hooks]` 内联配置；`commandWindows`（别名 `command_windows`）为 command hook 的 Windows 覆盖。
- Codex 高级配置：<https://developers.openai.com/codex/config-advanced>
  - 用于核实：外部 `notify` 当前仅支持 `agent-turn-complete`。
- Codex Hooks 文档：<https://developers.openai.com/codex/hooks>
  - 用于核实：hooks 默认启用、`hooks` 为规范 feature key（`codex_hooks` 为废弃别名）；事件在 turn scope 运行。
- Codex Windows 文档：<https://developers.openai.com/codex/app/windows>
- Tauri v2 Window Customization：<https://v2.tauri.app/learn/window-customization/>
- Tauri v2 Notification Plugin：<https://v2.tauri.app/plugin/notification/>
- Tauri v2 Sidecar：<https://v2.tauri.app/develop/sidecar/>
- Codex hatch-pet skill：<https://github.com/openai/skills/blob/main/skills/.curated/hatch-pet/SKILL.md>
- Codex animation rows：<https://raw.githubusercontent.com/openai/skills/main/skills/.curated/hatch-pet/references/animation-rows.md>
  - 用于核实：spritesheet atlas 规格与 9 行动作时长（M1 复核）。
