## ADDED Requirements

### Requirement: 日志写入文件
应用 SHALL 使用 `tracing` 将日志写入数据目录下的 `logs/agentpet.log`。日志初始化 MUST 在应用启动早期完成，以便记录后续生命周期事件。

#### Scenario: 启动后日志文件存在
- **WHEN** 应用完成启动
- **THEN** `logs/agentpet.log` 存在
- **AND** 文件中包含一条应用启动记录

### Requirement: 记录关键生命周期事件
应用 MUST 至少记录以下生命周期事件：应用启动、应用退出、各窗口的创建。每条记录 SHALL 带级别（如 info）与时间戳。

#### Scenario: 退出时记录退出日志
- **WHEN** 用户通过托盘"退出"结束应用
- **THEN** `logs/agentpet.log` 中追加一条应用退出记录

### Requirement: 日志失败不阻塞应用
当日志目录不可写或日志初始化失败时，应用 MUST 仍能正常启动（best-effort），MUST NOT 因日志故障而崩溃。

#### Scenario: 日志初始化失败时仍启动
- **WHEN** 日志文件无法创建（例如目录不可写）
- **THEN** 应用仍完成启动并显示 `pet-overlay`
