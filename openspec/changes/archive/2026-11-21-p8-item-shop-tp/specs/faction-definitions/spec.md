# Spec Delta

## MODIFIED Requirements

### Requirement: 开局按种族装配且不新增权威状态

`MeleeSetup` SHALL 依据种族参数生成对应 `DefId` 段的 `Spawn` 开局命令；玩家选族 MUST NOT 以新的运行时权威状态字段表达，未指定种族时 SHALL 回落人族段（R2 / R12.7 / ADR 0002）。选族装配本身不新增权威状态；P8.5a 复活 / P8.5b 主城升级 / P8.5c 野怪营地与掉落已各自按用户拍板方向 A 新增权威状态并同步进 FNV-1a 状态哈希、由 `SNAPSHOT_VERSION` 定义的快照与 `first_difference()`。本 change P8.5d 物品 / 商店 / 回城卷轴按推荐方向 A（待规格门 Q1 拍板）继续按 ADR 0002 全流程改 `w3-core`：新增英雄六格背包与物品实例（owner / slot / 充能 / 冷却 / `EffectDef`）、拾取 / 购买 / 使用 / 丢弃 / 移交 / 售卖事实、商店库存与刷新、回城传送（施法帧 / 目标 / 中断）、确定性随机码消费序列等权威状态（R6.2 / R7.5 / R7.6 / R9.3），MUST 同步进 FNV-1a 状态哈希、快照与 `first_difference()`；`SNAPSHOT_VERSION` / `SIM_VERSION` 按 Q4 推荐（待规格门拍板）bump 6→7（连同快照头部、单测硬断言、golden hash 基线、ADR 0002 复核记录同步更新；旧 v6 快照 / 回放按 `UnsupportedVersion` / `SimVersionMismatch` 明确拒绝）；按 Q5 推荐（条件项、待规格门拍板）新增 `GameplayCommandKind` / `EventKind` 变体、proto 只追加 tag 37 起、`PROTOCOL_VERSION` 6→7；determinism 26 项 +「改坏验红」全绿；`git diff -- crates/w3-core/` 非空属预期；MUST NOT 沿用「零 diff / 仍为 3、4、5 或 6 / 逐字节不变 / 变体集合不变 / MUST NOT 新增权威状态」旧断言（R12.4 / R7.5 / ADR 0002 §6-8 / `CODEBUDDY.md §5`）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题作 OpenSpec 归档合并身份键、字面不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 两族分别装配

- **WHEN** 分别以人族与兽族参数构造开局
- **THEN** 双方 `Spawn` 命令的 `DefId` 落在各自段内，资源与人口初值按既有冻结常量确定，选族装配不引入 `race` 权威状态字段（R2）；P8.5c 野怪 / 营地按中立归属装配、地面物品为无主（R8.1 / R1.1），P8.5d 六格背包 / 商店货架按定义层常量装配（R6.2 / R7.3），实例独立充能冷却（R7.5）

#### Scenario: 内核零 diff

- **WHEN** 校验本 capability 的选族装配改动，以及 P8.5d 物品 / 商店 / 回城卷轴的 `w3-core` 改动
- **THEN** 选族装配不引入 `race` 权威状态字段；P8.5d 的 `w3-core` 改动 MUST 把新增六格背包 / 物品实例 owner 与 slot / 商店库存 / 回城传送 / 确定性随机码消费序列等权威状态同步进状态哈希 / 快照 / `first_difference()`，`cargo test -p w3-core --test determinism`（26 项）全绿、含「改坏验红」、golden hash 基线按新实现如实更新，`SNAPSHOT_VERSION` 按 Q4 推荐 bump 6→7（连同 `SIM_VERSION`、快照头部、单测硬断言、ADR 0002 复核记录同步更新；旧 v6 快照 / 回放明确拒绝），按 Q5 推荐新增命令 / 事件变体、`git diff -- crates/w3-core/` 非空属预期，MUST NOT 沿用「`git diff -- crates/w3-core/` 为空 / 仍为 3、4、5 或 6 / 逐字节不变 / 变体集合不变 / MUST NOT 新增权威状态」旧断言（R12.4 / R7.5 / R9.3 / ADR 0002 / `CODEBUDDY.md §5`）

#### Scenario: 回落兼容

- **WHEN** 既有 `playable` 合成入口不传种族参数
- **THEN** 行为与改动前一致，一局仍能装配并打完，且主城升级 / 野怪营地 / 清野 / 掉落与 P8.5d 拾取 / 购买 / 使用 / 售卖 / 回城在 `w3-core` 确定性命令面可用、逐帧哈希与 P8.5d 新实现基线一致（R12.7 / R12.4 / R4.1 / R8.1 / R8.6 / R7.5 / R9.3）
