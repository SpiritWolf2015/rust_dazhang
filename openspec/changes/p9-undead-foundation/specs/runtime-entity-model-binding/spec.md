# Spec Delta

## MODIFIED Requirements

### Requirement: 运行时新实体按族段与单位/建筑粒度派生真实两族 glTF 模型键

运行时 `Train` / `Build` 产出的新实体 SHALL 按 `DefId` 族段（人 `1xx` / 兽 `2xx` / **不死族 `3xx`** / **暗夜族 `4xx`** / 中立 `9xx`，Q1' 已拍板双族全量）与单位 / 建筑粒度派生真实 glTF 模型键；`SEGMENT_HUMAN=100` / `SEGMENT_ORC=200` / `SEGMENT_NEUTRAL=900`（`runtime_models.rs:69-71`）SHALL 按表增设 `SEGMENT_UNDEAD=300` 与 `SEGMENT_NIGHT_ELF=400`，`SEGMENT_MODEL_KEYS`（`:89-111`）**5 段**与 `race_segment()`（`:169-172`）按表扩展、`def_id >= SEGMENT_*` 段判定（`:148-156`）同步覆盖 `3xx` / `4xx`，`ENTITY_MODEL_FILES`（`crates/w3-content/src/model_index.rs:58-74`）SHALL 增设不死 / 暗夜**逻辑路径字符串合成键表**（非资产内容、不含真实导出数据，R2 / R4 / ADR 0001 / ADR 0004 / Q1' / Q5'）。段键映射 MUST 保持只读派生、稳定顺序、不反写权威状态 / 状态哈希 / 快照（ADR 0002 §5 / R10.5 / R13.6）；Q2 新增族特色机制顶层权威状态与模型键只读派生严格区分、MUST NOT 把模型键 / fog 可见性带进权威状态（Q2' 已拍板）。

> 身份键说明（随归档并入主 spec）：历史 `### Requirement:` / `#### Scenario:` 标题（含字面「真实两族」「兽族运行时新实体」等历史措辞）作 OpenSpec 归档合并身份键、字面保留不代表本 change 的行为断言；行为口径一律以各 Scenario 的 THEN 与本 requirement 新正文为准（参照主 `openspec/specs/gameplay-systems/spec.md:10` 与本 change `design.md` Context 末「MODIFIED delta 的身份键说明」）。

#### Scenario: 族段 → 段代表真实模型键的映射与烘焙（R2 / R4 / C16–C18 / C21）

- **WHEN** 枚举 4 族（含不死 `3xx` / 暗夜 `4xx`）与中立共 5 段的 `SEGMENT_MODEL_KEYS` 并烘焙运行时新实体模型键
- **THEN** 每段均映射到段代表逻辑模型键、新增 `SEGMENT_UNDEAD=300` / `SEGMENT_NIGHT_ELF=400` 与既有 `1xx` / `2xx` / `8xxx` / `9xxx` 段判定不冲突，`race_segment()` 仍是单一查表处、由数据驱动不散落 `if race` 白名单；按 Q3 bump 8 的 `SNAPSHOT_VERSION` 快照契约下模型键仍只读派生（R2 / R4 / R1.2 / D6 / Q1' / Q3 / Q5' / C16–C18 / C21）

#### Scenario: 大小写归一命中真实大小写磁盘文件（C18，不是待修缺陷）

- **WHEN** 以不死 / 暗夜段键经 `normalize()` 双侧归一查 `ENTITY_MODEL_FILES` 逻辑路径
- **THEN** 全小写归一逻辑路径由既有归一机制命中真实大小写磁盘文件（本机导出目录、gitignored、不入库）；归一命中是既成事实不是待修缺陷，第 3 / 4 族键表沿用同一机制（C18 / ADR 0001 / ADR 0004 / Q1' / Q5'）

#### Scenario: 兽族运行时新实体呈现真实 glTF 网格而非四面体（R2 / R4 / C9 / C21）

- **WHEN** 4 族（人 / 兽 / 不死 / 暗夜）各自运行时 `Train` 产出新实体并离屏渲染
- **THEN** 各族按段代表键呈现真实 glTF 网格而非四面体占位，不死 / 暗夜新实体按 `3xx` / `4xx` 段键呈现且不影响人 / 兽既有网格；无本机资产时按既有显式可诊断回退、不冒充验证真实网格（R2 / R4 / C9 / C21 / Q1' / Q5' / C12）

#### Scenario: 静态摆放链路保持不变（ADR 0004 / C7）

- **WHEN** 校验 frame-0 静态摆放链路与 4 族段表扩展、Q2 / Q3 bump 8 的关系
- **THEN** 静态摆放链路保持不变、4 族扩展只作用于运行时动态实体的段键派生，不改写装配期 `DefId→Def` 冻结边界；`w3-content` 原始 `f64` 坐标不进哈希 / 快照，按 Q3 的 `SNAPSHOT_VERSION` bump 8 与 Q2 新增权威状态段均不改变该边界（ADR 0004 / ADR 0002 / Q2 / Q3 / C7 / Q1'）

#### Scenario: R-1 加宽：八类走段代表键、6 键不可达、reason=None 为已知限制（C21 / C22，用户 2026-11-18 拍板接受）

- **WHEN** 复核运行时实体模型绑定的覆盖判定与已知限制
- **THEN** 八类走段代表键、6 键不可达、`reason=None` 的既有已知限制按用户 2026-11-18 拍板接受的口径保留、不冒充已消除；本期新增不死 / 暗夜段键按同一判定登记、不扩范围修复既有不可达键、不静默降级（C21 / C22 / ADR 0001 / Q1' / Q9' 已拍板登记已知限制）
