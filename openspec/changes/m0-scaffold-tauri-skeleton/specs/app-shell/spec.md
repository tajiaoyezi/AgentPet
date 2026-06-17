## ADDED Requirements

### Requirement: 应用启动创建三个窗口
应用启动后 SHALL 创建三个独立的 webview 窗口：`pet-overlay`、`status-panel`、`settings`。三个窗口共享同一 Tauri 进程，但各自是独立 webview，前端 store 不跨窗口共享。

#### Scenario: 启动后三个窗口存在
- **WHEN** 应用进程完成启动
- **THEN** 存在 label 为 `pet-overlay`、`status-panel`、`settings` 的三个窗口
- **AND** `pet-overlay` 默认可见

### Requirement: pet-overlay 窗口属性
`pet-overlay` 窗口 MUST 配置为透明（transparent）、无边框（decorations=false）、置顶（alwaysOnTop）、不进任务栏（skipTaskbar）、不可调整尺寸（resizable=false）、可见（visible），并支持鼠标拖动移动位置。

#### Scenario: overlay 窗口属性正确
- **WHEN** 读取 `pet-overlay` 的窗口配置
- **THEN** transparent / decorations=false / alwaysOnTop / skipTaskbar / resizable=false 均符合预期

#### Scenario: overlay 可拖动
- **WHEN** 用户在 overlay 窗口空白区域按下并拖动
- **THEN** 窗口随指针移动到新位置

### Requirement: status-panel 与 settings 为普通窗口
`status-panel` 与 `settings` MUST 为带边框、可调整尺寸的普通窗口，且默认不出现在任务栏置顶层级（普通窗口行为）。本里程碑下两者内容为空占位页。

#### Scenario: 普通窗口可显示与关闭
- **WHEN** `status-panel` 或 `settings` 被显示
- **THEN** 窗口带标准边框并可被用户关闭

### Requirement: 系统托盘入口
应用 SHALL 提供系统托盘图标，其菜单至少包含"打开状态面板"、"打开设置"、"退出"三项。

#### Scenario: 托盘菜单包含三项
- **WHEN** 用户右键点击托盘图标
- **THEN** 菜单显示"打开状态面板"、"打开设置"、"退出"

#### Scenario: 从托盘打开窗口
- **WHEN** 用户点击托盘菜单的"打开设置"
- **THEN** `settings` 窗口被显示并获得前台

### Requirement: 关闭到托盘与托盘退出
关闭任一窗口 MUST 仅隐藏该窗口（应用继续在托盘常驻），只有托盘菜单的"退出"才真正结束进程。

#### Scenario: 关闭窗口不退出应用
- **WHEN** 用户关闭 `settings` 窗口
- **THEN** 窗口隐藏
- **AND** 应用进程仍在运行且托盘图标仍存在

#### Scenario: 托盘退出结束应用
- **WHEN** 用户点击托盘菜单的"退出"
- **THEN** 所有窗口关闭且应用进程结束
