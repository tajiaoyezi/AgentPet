## ADDED Requirements

### Requirement: 解析应用数据根目录
应用 SHALL 通过平台无关的目录解析得到 AgentPet 数据根目录。Windows 上 MUST 解析为 `%APPDATA%\AgentPet`。路径解析失败时 MUST 记录错误而非崩溃。

#### Scenario: Windows 解析根目录
- **WHEN** 在 Windows 上启动应用
- **THEN** 数据根目录解析为 `%APPDATA%\AgentPet`

### Requirement: 初始化数据目录树
启动时应用 MUST 确保以下子目录存在：`logs/`、`hooks/`、`pets/`、`sounds/`、`bin/`。创建过程 MUST 幂等——目录已存在时不报错、不覆盖已有内容。

#### Scenario: 首次启动创建缺失目录
- **WHEN** 数据根目录为空且应用启动
- **THEN** `logs/`、`hooks/`、`pets/`、`sounds/`、`bin/` 全部被创建

#### Scenario: 重复启动幂等
- **WHEN** 数据目录树已存在且应用再次启动
- **THEN** 不抛出错误
- **AND** 既有目录与文件内容保持不变

### Requirement: settings.json 占位生成
当数据根目录下不存在 `settings.json` 时，应用 MUST 写入一个带 `schema` 字段的最小占位文件；已存在时 MUST 保留不覆盖。

#### Scenario: 首次创建占位
- **WHEN** 数据根目录不含 `settings.json` 且应用启动
- **THEN** 生成包含 `"schema": "agentpet.settings/v1"` 的占位 `settings.json`

#### Scenario: 已存在则保留
- **WHEN** `settings.json` 已存在且应用启动
- **THEN** 文件内容保持不变
