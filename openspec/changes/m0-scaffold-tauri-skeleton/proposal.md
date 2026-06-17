## Why

AgentPet（跨 Agent 的本地注意力中心）的所有后续能力——宠物运行时、事件服务、通知、Adapter——都必须运行在一个稳定的桌面外壳之上。技术方案 v0.2 的 Milestone 0 要求先交付这个骨架：三个窗口、托盘、应用数据目录、跨窗口事件总线与日志。没有它，M1–M6 无处落脚。

本变更只做"能立起来"的最小骨架，不包含任何业务逻辑（不接事件、不渲染宠物、不发通知），以便快速验证 Tauri v2 + Rust + React/TS 这套技术选型在 Windows 上可用。

## What Changes

- 初始化 Tauri v2 + Rust 后端 + Vite + React + TypeScript 前端工程（Windows-first），建立 `src-tauri/` 与 `src/` 目录骨架。
- 创建三个 webview 窗口：`pet-overlay`（透明、无边框、置顶、不进任务栏、可拖动、空 Canvas）、`status-panel`（普通窗口、空）、`settings`（普通窗口、空）。
- 增加系统托盘入口：菜单含"打开状态面板 / 打开设置 / 退出"；关闭窗口默认隐藏到托盘而非退出应用，仅托盘"退出"真正结束进程。
- 初始化应用数据目录树（`%APPDATA%\AgentPet\`：`logs/`、`hooks/`、`pets/`、`sounds/`、`bin/` 及 `settings.json` 占位），通过平台无关的路径解析创建。
- 搭建跨窗口事件总线骨架：Rust 后端作为唯一 source of truth，写操作经 Tauri command 进入后端，状态变化经 Tauri event 广播回各窗口；提供一个 demo/health 往返事件以证明链路打通。
- 接入日志系统：基于 `tracing` 写入 `logs/agentpet.log`，按类别（app lifecycle / window / 其它）记录。

非目标（本变更明确不做）：事件 HTTP 服务、token、宠物导入与渲染、reaction engine、声音、Toast、Adapter、sidecar、安装包。这些属于 M1–M6。

## Capabilities

### New Capabilities
- `app-shell`: Tauri 应用外壳——三个窗口（pet-overlay / status-panel / settings）的创建与窗口属性、系统托盘菜单、关闭到托盘与托盘退出的生命周期。
- `app-data-paths`: 应用数据目录的解析与初始化（`%APPDATA%\AgentPet\` 目录树与 `settings.json` 占位），供后续里程碑的存储、token、宠物、hook 复用。
- `window-event-bus`: 跨窗口状态一致性骨架——后端唯一权威状态、Tauri command 写入、Tauri event 广播订阅，含一个可验证的 health 往返。
- `app-logging`: 诊断日志能力——基于 `tracing` 输出到 `logs/agentpet.log`，区分日志类别，覆盖启动/退出/窗口生命周期。

### Modified Capabilities
<!-- 无：openspec/specs/ 为空，本变更全部为新建能力。 -->

## Impact

- 新增工程脚手架：`src-tauri/`（Rust，依赖 `tauri`、`tokio`、`serde`、`directories`、`tracing` 等）、`src/`（React + TS）、`tauri.conf.json`、`package.json`、`Cargo.toml`。
- 平台范围：v0.1 Windows-first；目录解析需为 macOS（v0.2）预留但本期不验证。
- 对后续里程碑的契约：app-data-paths 定义的目录结构是 M2 事件服务（runtime.json/token/db）、M4/M5 hook 脚本与 sidecar、M1 宠物落盘的共同前提；window-event-bus 是 M2/M3 状态广播的传输层。
- 无外部 API 变更、无对 Claude/Codex 配置的写入（Adapter 属于 M4/M5）。
