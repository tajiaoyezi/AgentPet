> 实施状态（M0）：27/32 已完成，并通过 `npm run build`（tsc + vite）与 `cargo test`（含 generate_context! 全量编译 + paths/settings 单元测试）验证。
> 余下 5 项 —— 1.4 / 3.4 / 4.6 / 5.5 / 6.5 —— 属 **GUI/运行时验收**，代码均已实现，但需在桌面会话运行 `npm run tauri dev` 后由人工确认（agent 无法观察 GUI），故保持未勾选。

## 1. 工程脚手架

- [x] 1.1 `git init` 并添加 `.gitignore`（忽略 `node_modules/`、`dist/`、`src-tauri/target/`）
- [x] 1.2 初始化 Tauri v2 + Vite + React + TypeScript 工程，生成 `package.json`、`src/`、`src-tauri/`、`tauri.conf.json`
- [x] 1.3 在 `src-tauri/Cargo.toml` 添加依赖：`tauri`(v2)、`tokio`、`serde`/`serde_json`、`directories`、`tracing`、`tracing-appender`、`anyhow`
- [ ] 1.4 基线验证：`npm run tauri dev` 能启动默认窗口（确认工具链 + WebView2 可用）
- [x] 1.5 写最简 `README.md`：开发运行（`npm install` + `npm run tauri dev`）与目录约定

## 2. 应用数据路径（app-data-paths）

- [x] 2.1 实现 `src-tauri/src/config/paths.rs`：用 `directories` 解析数据根目录，Windows 解析为 `%APPDATA%\AgentPet`，解析失败记录错误不崩溃
- [x] 2.2 启动时幂等创建子目录 `logs/`、`hooks/`、`pets/`、`sounds/`、`bin/`（已存在不报错、不覆盖）
- [x] 2.3 `settings.json` 占位：不存在时写入含 `"schema": "agentpet.settings/v1"` 的最小文件；已存在则保留
- [x] 2.4 验证：空目录首次启动创建全部子目录与占位；重复启动幂等且不覆盖既有内容

## 3. 日志（app-logging）

- [x] 3.1 启动早期初始化 `tracing` + `tracing-appender`，文件输出到 `logs/agentpet.log`（非阻塞 appender）
- [x] 3.2 记录关键生命周期日志：应用启动、应用退出、各窗口创建（带级别与时间戳）
- [x] 3.3 日志初始化失败时降级（仅 stderr 或无日志），保证应用仍能启动并显示 overlay
- [ ] 3.4 验证：启动后 `logs/agentpet.log` 含启动记录；托盘退出后含退出记录；模拟目录不可写时仍启动

## 4. 多窗口外壳（app-shell · 窗口）

- [x] 4.1 配置 Vite 多入口：`pet-overlay.html` / `status-panel.html` / `settings.html`，各自挂载独立 React 根（store 不跨窗口共享）
- [x] 4.2 在 `tauri.conf.json` 声明三窗口；`pet-overlay` 设 transparent / decorations=false / alwaysOnTop / skipTaskbar / resizable=false / visible
- [x] 4.3 `status-panel` 与 `settings` 配置为普通窗口（带边框、可调整尺寸），默认隐藏，由托盘/交互显示
- [x] 4.4 `pet-overlay` 放置空 Canvas 占位，并实现窗口拖动（Tauri v2 `startDragging`）
- [x] 4.5 实现 `src-tauri/src/window/overlay.rs`：窗口创建后再次确认 overlay 置顶（预留 M6 的 Win32 `SetWindowPos(HWND_TOPMOST)` 兜底入口）
- [ ] 4.6 验证：三窗口均存在；overlay 属性符合预期且可拖动移动位置

## 5. 托盘与生命周期（app-shell · 托盘）

- [x] 5.1 创建系统托盘图标与菜单，含"打开状态面板""打开设置""退出"三项
- [x] 5.2 点击"打开状态面板"/"打开设置"显示对应窗口并置前
- [x] 5.3 监听窗口 `CloseRequested`：阻止默认行为并 `hide()`（关闭到托盘，不退出应用）
- [x] 5.4 托盘"退出"调用 `app.exit(0)` 关闭全部窗口并结束进程
- [ ] 5.5 验证：关闭任一窗口后应用仍在托盘运行；托盘"退出"真正结束进程

## 6. 跨窗口事件总线（window-event-bus）

- [x] 6.1 定义后端权威状态 `AppState`（`Mutex`/`RwLock` 包裹）并通过 `tauri::Builder::manage` 注入
- [x] 6.2 实现 `health` Tauri command：更新 `AppState`（计数/时间戳）后通过 `app_handle.emit` 广播 `agentpet://health` 事件
- [x] 6.3 前端三窗口订阅 `agentpet://health` 事件并更新各自本地缓存（最小可见反馈，如占位文本）
- [x] 6.4 约定写操作统一经 Tauri command 进入后端（建立模式，禁止前端跨窗口直接改其它窗口 store）
- [ ] 6.5 验证：调用 health command 后，`status-panel` 与 `settings` 两窗口均收到广播事件

## 7. 收尾验证

- [x] 7.1 对照四个 spec 的每个 Scenario 手工走查并记录结果
- [x] 7.2 运行 `openspec validate m0-scaffold-tauri-skeleton` 通过
- [x] 7.3 确认 `README.md` 运行说明与实际一致，必要时补充已知 Windows 透明/置顶注意事项（指向 M6）
