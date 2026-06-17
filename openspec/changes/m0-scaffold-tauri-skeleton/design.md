## Context

AgentPet v0.2 技术方案的 Milestone 0 要求建立可运行的桌面骨架，作为 M1–M6 的承载层。当前仓库仅有 `docs/` 与 `openspec/`，没有任何工程代码，且尚未初始化为 git 仓库。

约束（来自技术方案）：
- 技术栈固定为 Tauri v2 + Rust + Vite + React + TypeScript（§3.1–§3.2）。
- 三窗口为独立 webview，前端 store 不跨窗口共享，后端为唯一权威状态（§4.2、§4.3、A5）。
- Windows 透明/置顶有已知坑：WebView2 + GPU 可能影响透明，后台进程置顶可能被覆盖，需创建后再确认（§20.1）。
- 数据目录结构在 §17.1 已定义，需为 v0.2 macOS 预留路径解析。

本设计聚焦"如何把骨架立起来"，不涉及任何业务事件/渲染/通知逻辑。

## Goals / Non-Goals

**Goals:**
- 一条 `tauri dev` 命令即可启动：托盘常驻 + 三个窗口（overlay 透明可拖动、panel/settings 普通空页）。
- 后端持有权威 AppState，提供一条 health command + event 往返证明跨窗口链路可用。
- 启动时幂等初始化 `%APPDATA%\AgentPet\` 目录树与 `settings.json` 占位。
- `tracing` 日志落盘到 `logs/agentpet.log`，失败可降级不崩溃。
- 目录/模块结构与 §5.1/§5.2 对齐，给后续里程碑留好放置点（空模块或占位）。

**Non-Goals:**
- 不做事件 HTTP 服务、token、SQLite、宠物导入/渲染、reaction、声音、Toast、Adapter、sidecar、安装包（M1–M6）。
- 不做 always-on-top / 透明 / DPI / 多显示器的完整稳定化（M6）。本期只搭好"创建后 reapply 置顶"的钩子，不追求覆盖所有边界。
- 不验证 macOS（v0.2）；仅保证路径解析是跨平台 API、不硬编码。

## Decisions

### D1：三窗口在 `tauri.conf.json` 静态声明，overlay 置顶用 Rust 钩子兜底
在配置中声明 `pet-overlay`（transparent / decorations=false / alwaysOnTop / skipTaskbar / resizable=false / 可见）、`status-panel`、`settings`（普通、默认隐藏，由托盘/交互显示）。窗口创建后在 Rust 侧 `setup` 中对 overlay 再次确认置顶（为 M6 的 Win32 `SetWindowPos(HWND_TOPMOST)` 兜底预留入口）。
- 备选：全部用 `WebviewWindowBuilder` 运行时创建——更灵活但 M0 无动态窗口需求，静态声明生命周期更清晰。

### D2：前端用 Vite 多入口（multi-page）承载三个窗口
每个窗口一个 HTML 入口（`pet-overlay.html` / `status-panel.html` / `settings.html`），各自挂载独立 React 根。
- 备选：单页 + 路由/查询参数区分窗口——会让"store 不跨窗口共享"的边界更模糊；多入口物理隔离更贴合 §4.3。

### D3：后端唯一权威状态 = `AppState` + command 写 + event 广播
用 `tauri::State`（`Mutex`/`RwLock` 包裹的 `AppState`）持有跨窗口状态。写操作经 Tauri command 进入后端；后端变更后用 `app_handle.emit`（全局广播）通知所有窗口。本期实现一条 `health` command：被调用→更新一个计数/时间戳→`emit("agentpet://health", payload)`，前端各窗口订阅验证。
- 备选：前端共享 store——webview 隔离下不可行（A5 已否决）。

### D4：应用数据路径用 `directories` crate 解析，不硬编码 `%APPDATA%`
用 `directories::BaseDirs`/`ProjectDirs` 得到根目录（Windows→`%APPDATA%\AgentPet`），集中在 `config/paths.rs`。
- 理由：跨平台、为 v0.2 macOS（`~/Library/Application Support/AgentPet`）预留；且该逻辑未来可被独立的 `agentpet-event` sidecar crate（无 Tauri 运行时，§3.4）复用，避免与 Tauri path API 形成双源。

### D5：日志用 `tracing` + `tracing-appender`，best-effort 降级
启动早期初始化 `tracing` subscriber，文件输出到 `logs/agentpet.log`（非阻塞 appender）。初始化失败时降级为仅 stderr 或无日志，绝不阻塞/崩溃应用启动（满足 app-logging 的失败降级要求）。

### D6：关闭到托盘 + 托盘退出
监听窗口 `CloseRequested`：阻止默认行为并 `hide()` 该窗口；只有托盘菜单"退出"调用 `app.exit(0)` 真正结束进程。托盘菜单三项："打开状态面板""打开设置""退出"。

### D7：Rust 模块按 §5.1 预铺占位
本期落地 `main.rs`、`app_state.rs`、`config/paths.rs`、`config/settings.rs`、`window/overlay.rs`（含置顶兜底入口）；其余模块（event_bus / session / adapters / pet / notify / store）以空目录或 `mod` 占位预留，不实现逻辑——降低后续里程碑的接入摩擦。

### D8：包管理器用 npm
用户环境已具备 npm（`openspec` 经 npm 安装）。无需引入 pnpm/yarn 以减少环境假设。

## Risks / Trade-offs

- [Windows 透明 WebView 受 GPU/WebView2 影响，可能出现黑底或不透明] → M0 仅验证基础透明；若失败，记录并将完整修复（含 Win32 兜底）留到 M6，不阻塞骨架交付。
- [always-on-top 可能被部分全屏/置顶窗口覆盖] → 仅预留创建后 reapply 钩子；周期性/事件触发的完整 reapply 属 M6（§26.7）。
- [Vite 多入口增加构建配置复杂度] → 接受，换取三窗口职责与 store 边界清晰。
- [仓库尚未 git init，后续 adapter 备份/diff 的心智依赖版本控制] → 在 tasks 中加入 `git init` 作为脚手架步骤；与功能解耦，失败不影响运行。
- [`directories` 与 Tauri path API 双源风险] → 统一只用 `directories`，Tauri 侧也经 `config/paths.rs` 取路径，避免不一致。

## Migration Plan

绿地新建，无存量数据或回滚需求。交付即"可 `npm run tauri dev` 启动"；验证方式见 specs 的各 Scenario（窗口存在、托盘三项、目录树创建、health 往返、日志落盘）。后续里程碑在本骨架上增量叠加，无需迁移。

## Open Questions

- 是否本期就引入最简前端样式系统（仅占位页可暂不需要）？倾向暂不引入，settings/status-panel 用最小占位。
- overlay 拖动用 Tauri 的 `startDragging` 还是自实现指针事件？倾向 `startDragging`（系统级拖动更顺滑），具体在实现时确认 v2 API 名称。
