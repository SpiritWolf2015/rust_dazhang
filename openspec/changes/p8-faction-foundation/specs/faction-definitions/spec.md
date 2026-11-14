# Spec Delta

## Purpose

为两族完整化提供定义层地基：冻结定义的 `DefId` 按种族分段、开局按种族装配，且种族选择不引入新的运行时权威状态，守住 ADR 0002 确定性契约。

## ADDED Requirements

### Requirement: 冻结定义按种族分段

`FrozenDefinitions` 中 **`units` 类别**的 `DefId` SHALL 按种族分段（人族段、兽族段、中立 / 资源段），同段内唯一、跨段不冲突；分段只约束 `units` 类别，`abilities` / `upgrades` / `items` 三个类别的既有类别内 ID（`10` / `20` / `30`）沿用各自类别内唯一规则、不纳入种族分段。每段定义仍是编译期整数常量，内容哈希 MUST 进入快照头部（ADR 0002）。分段 MUST NOT 引入 `f64`、MUST NOT 读取 `config.toml` 或任何导出资产（ADR 0001 / ADR 0004）。

#### Scenario: 分段唯一且可复现

- **WHEN** 构造含人族段与兽族段的 `FrozenDefinitions`
- **THEN** `units` 类别各 `DefId` 全局唯一、构造可重复、`defs_hash` 固定，且与既有合成定义的 6 个 `UnitDef` 的 DefId 不冲突；三个非 units 类别的 ID 仍各自类别内唯一

#### Scenario: 零资产构造

- **WHEN** 在不含 `config.toml` 与导出目录的干净环境构造分段定义
- **THEN** 构造成功且不做任何文件 IO

### Requirement: 开局按种族装配且不新增权威状态

`MeleeSetup` SHALL 依据种族参数生成对应 `DefId` 段的 `Spawn` 开局命令；玩家选择的种族 MUST NOT 以新的运行时权威状态字段表达，MUST NOT 改变 `SNAPSHOT_VERSION`（当前 3）、状态哈希字段集合或快照头部布局（主 `openspec/specs/melee-match/spec.md:127-129`）。未指定种族时 SHALL 回落人族段，既有入口行为不变。

#### Scenario: 两族分别装配

- **WHEN** 分别以人族与兽族参数构造开局
- **THEN** 双方 `Spawn` 命令的 `DefId` 落在各自段内，资源与人口初值按既有冻结常量确定

#### Scenario: 内核零 diff

- **WHEN** 校验本 capability 的全部改动
- **THEN** `git diff -- crates/w3-core/` 为空、`SNAPSHOT_VERSION` 仍为 3、`cargo test -p w3-core --test determinism` 26 项全绿

#### Scenario: 回落兼容

- **WHEN** 既有 `playable` 合成入口不传种族参数
- **THEN** 行为与改动前一致，一局仍能装配并打完
