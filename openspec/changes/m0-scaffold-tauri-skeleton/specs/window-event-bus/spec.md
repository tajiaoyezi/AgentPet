## ADDED Requirements

### Requirement: 后端为唯一权威状态
跨窗口共享的状态 MUST 以 Rust 后端为唯一 source of truth。前端各窗口的 store 仅作渲染缓存，MUST NOT 跨窗口直接共享状态。

#### Scenario: 写操作经 command 进入后端
- **WHEN** 任一窗口发起一次写操作
- **THEN** 该写操作经 Tauri command 提交到后端处理
- **AND** 不通过窗口间直接通信修改其它窗口的本地 store

### Requirement: 状态变化广播到所有窗口
后端状态发生变化时 SHALL 通过 Tauri event 向所有已打开窗口广播，使各窗口能订阅并更新本地缓存。

#### Scenario: 广播被多个窗口接收
- **WHEN** 后端发出一个状态广播事件
- **AND** `status-panel` 与 `settings` 均已打开并订阅该事件
- **THEN** 两个窗口都接收到该事件

### Requirement: health 往返验证
本里程碑 MUST 提供一个可验证链路打通的 health 往返：前端调用一个 health Tauri command，后端处理后广播一个 health event，订阅方收到即证明 command→后端→event 链路可用。

#### Scenario: health command 触发广播
- **WHEN** 前端调用 health command
- **THEN** 后端返回成功结果
- **AND** 后端随后广播一个 health event 被订阅窗口收到
