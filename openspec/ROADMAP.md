# AgentPet 里程碑路线图 (Roadmap)

本文件是 **M0–M6 跨 change 的前向规划与状态跟踪**，用于在不同会话 / 不同 AI 之间无歧义接力。

职责划分：
- **OpenSpec 管"已建 / 已归档"**：`openspec list`（进行中）+ `openspec/changes/archive/`（已完成）。
- **本文件管"还没建的未来计划 + 命名约定 + 进度总览"**。
- **权威需求来源（single source of truth）**：`docs/AgentPet_技术方案_v0.2.md`（尤其 §25 开发里程碑、§28 落地顺序）。

---

## 命名约定

change 目录名 = `m<里程碑号>-<kebab-case 描述>`，**必须小写**（OpenSpec 强制要求 change 名为小写 kebab-case，大写会被 `openspec instructions` 拒绝）。与技术文档 §25 的里程碑 **M0–M6 一一对应**（change 号 == 里程碑号）。

例：`m0-scaffold-tauri-skeleton`（对应文档里程碑 M0）。

---

## 里程碑 → change 映射与状态

状态图例：📋 planned（未建） · 🚧 in-progress（已提案/实施中） · 📦 archived（已归档）

| 前缀 | 里程碑（交付重点） | change 名 | 主要文档章节 | 状态 |
|---|---|---|---|---|
| `m0` | 项目骨架（Tauri 三窗口/托盘/数据目录/事件总线/日志） | `m0-scaffold-tauri-skeleton` | §25 M0, §4–§5, §17.1, §22 | 🚧 已提案 (0/32 tasks) |
| `m1` | Pet Runtime + Codex Importer | `m1-add-pet-runtime-importer` | §13, §14, §25 M1 | 📋 |
| `m2` | Event Server + Session Registry | `m2-add-event-server-session-registry` | §6, §7, §8, §17.2, §25 M2 | 📋 |
| `m3` | Reaction Engine + 通知（声音/Toast/勿扰） | `m3-add-reaction-engine-notifications` | §8, §13.4–§13.6, §15, §25 M3 | 📋 |
| `m4` | 事件 Sidecar + Claude Adapter | `m4-add-event-sidecar-claude-adapter` | §3.4, §9, §10, §25 M4 | 📋 |
| `m5` | Codex Adapter（hooks 主 / notify 可选） | `m5-add-codex-adapter` | §11, §25 M5 | 📋 |
| `m6` | Windows 稳定化 + 发布 | `m6-stabilize-windows-release` | §20, §24, §25 M6 | 📋 |

---

## 依赖顺序

```text
m0 → m1 → m2 → m3 → m4 → m5 → m6
```

与 §28 落地顺序一致：m4 依赖 m2/m3 的事件与 reaction 链路；m5 复用 m4 的 sidecar；m6 在前述全部之上做稳定化与打包。

---

## 如何在新会话继续（接力步骤）

```text
1. openspec list                       # 看进行中的 change（恢复“做到哪了”）
2. 看 openspec/changes/archive/         # 看已完成/归档的里程碑
3. 读本 ROADMAP，找到下一个 📋 的里程碑及其“主要文档章节”
4. 运行 /opsx-propose，或对 agent 说：“按 ROADMAP 生成 <下一个里程碑> 的 change”
   → agent 执行：openspec new change "m<n>-..."，再生成 proposal / specs / design / tasks
5. /opsx-apply 实现该 change → 完成后 /opsx-archive 归档
6. 回到本 ROADMAP，更新对应行的状态列
```

> 命名约定也已写入 `openspec/config.yaml` 的 `context`，会注入到每次 `openspec instructions` 的提示中，因此新会话即使不读本文件也会沿用 `m<n>-` 命名。

---

## 状态维护约定

- 新建某里程碑的 change 时：把该行状态从 📋 → 🚧。
- 归档后：把状态改为 📦，可在状态列追加 archive 日期 / 路径。
- 若中途调整里程碑拆分或命名，**先改本文件再建 change**，保持单一事实来源。
