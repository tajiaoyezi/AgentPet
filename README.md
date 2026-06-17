# AgentPet

跨 Agent 的本地注意力中心：用 Tauri v2 桌宠统一 Claude Code / Codex 等 AI Agent 的
完成 / 等待输入 / 权限确认 / 错误 / 卡住等状态提醒。

> 当前进度：**M0 项目骨架**（三窗口 + 托盘 + 数据目录 + 跨窗口事件总线 + 日志）。
> 业务能力（事件服务、宠物、通知、Adapter）见 `openspec/ROADMAP.md` 的 M1–M6。

## 技术栈

Tauri v2 · Rust · Vite · React · TypeScript · Canvas 2D（v0.1 Windows-first）

## 开发运行

前置：Node.js + npm、Rust（cargo）；Windows 需 WebView2 运行时。

```bash
npm install
npm run tauri dev
```

`npm run tauri dev` 会启动托盘常驻进程与三个窗口：`pet-overlay`（透明、无边框、置顶、可拖动）、
`status-panel`、`settings`（普通窗口，默认隐藏，由托盘菜单打开）。关闭窗口隐藏到托盘，托盘“退出”才结束进程。

构建 / 检查：

```bash
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
```

## 目录约定

- `src/` 前端：`windows/`（三个窗口组件）、`main-*.tsx`（各窗口入口）、`health.ts`（事件订阅）。
- `src-tauri/src/` 后端：`lib.rs`（窗口/托盘/生命周期）、`app_state.rs`、`config/`（paths / settings）、
  `window/overlay.rs`、`commands.rs`；`event_bus/`、`session/`、`adapters/`、`pet/`、`notify/`、`store/`
  为后续里程碑预留占位。
- 运行时数据：`%APPDATA%\AgentPet\`（`logs/`、`hooks/`、`pets/`、`sounds/`、`bin/`、`settings.json`）。
- 规格与计划：`openspec/`（`ROADMAP.md` + `changes/`）。

## 已知事项（Windows）

- 透明 / always-on-top 受 WebView2 / GPU 影响，M0 仅做基础启用；完整稳定化（含 Win32 置顶兜底、
  DPI、多显示器）见 M6。
- 系统通知（Toast）、安装包注册等留待 M3 / M6。
