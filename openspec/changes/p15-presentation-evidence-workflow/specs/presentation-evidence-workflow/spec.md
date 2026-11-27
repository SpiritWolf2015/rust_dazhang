# Spec Delta

## Purpose

本 capability 定义呈现层证据流程的行为契约：以 clean-room 反编译证据流程在本机外部工具产出行为规格文档，经仓库外证据目录向主仓库做只读文档交接，再由消费侧视觉 oracle 对照呈现层行为。它是长期目标「像素级复刻 War3」的承接手段（ADR 0003 第二次修订），产出的是视觉 oracle / 行为规格，不是可编译进项目的代码，也不是 ADR 0002 口径的业务逻辑 oracle；零资产、零新增权威状态、零确定性影响，非玩法 change。

## ADDED Requirements

### Requirement: 生产侧 clean-room 行为规格产出

系统 SHALL 以 `craft-anything` 之类本机外部工具（CLI / MCP）从用户合法自备的经典客户端原生二进制产出**行为规格文档**（伪 C / 调用图 / 呈现层行为描述），供呈现层像素对齐取证。系统 MUST NOT 把产出当可编译进项目的代码或 ADR 0002 口径的业务逻辑 oracle；工具 MUST 不入库、不复制上游代码、不新增 crate 依赖、不登记 `NOTICE.md`；产出 MUST 存于仓库外本机证据目录（ADR 0001 / 0003）。

#### Scenario: 产出为行为规格文档而非代码

- **WHEN** 生产侧对本机经典客户端运行反编译 / 抓帧取证，产出伪 C、调用图与呈现层行为描述
- **THEN** 产物以行为规格文档形态落在仓库外本机证据目录，主仓库 `git status --short` 无新增代码 / 资产 / 依赖 / `NOTICE.md` 条目；产物 MUST NOT 被描述为可编译代码或业务逻辑 oracle

#### Scenario: 合法自备与合规红线

- **WHEN** 取证流程启动前逐条核对使用前提
- **THEN** 仅针对用户合法自备的经典客户端，MUST NOT 绕过 EULA / DRM / 登录 / 反作弊；零暴雪资产、零 MPQ 派生数据入库（ADR 0001 复核记录 `0001-license.md:80-82`）；不声称已获暴雪授权

### Requirement: 仓库外证据目录到主仓库的只读文档交接

行为规格与证据记录 SHALL 存于**仓库外本机证据目录**，主仓库 SHALL 只接收**只读文档交接**（生产侧与消费侧只通过只读文档交接）。证据纪律 SHALL 遵循判定须经独立重审：判定与产物同提交、并经受独立重审。主仓库 MUST NOT 接收伪 C / 抓帧二进制 / 证据原始产物或任何游戏资产（ADR 0001 / 0003）。

#### Scenario: 只交只读文档、不交原始产物

- **WHEN** 生产侧把行为规格与证据记录交接给主仓库
- **THEN** 主仓库只新增只读文档交接件，仓库外证据目录原件不入库；`git ls-files` 无 `*.mpq` / `*.blp` / `*.mdx` / `*.slk` / 派生 JSON / PNG / GLB；交接格式 / 字段 / 版本化属规格门待确认项、不臆造

#### Scenario: 判定与产物同提交并独立重审

- **WHEN** 试点产出一条「行为规格 → 视觉对照」判定
- **THEN** 判定与该判定所依据的产物同批提交，并留有独立重审记录（不由产出方自审即标通过）；重审方式与判定粒度属规格门待确认项，不冒充已验收

### Requirement: 消费侧视觉 oracle 与呈现层只读派生边界

消费侧 SHALL 用 `crates/w3-render` 离屏 `render_headless_rgba` / `render_headless_rgba_with_textures` + ViewImage + 真窗口人工验收作**视觉 oracle**，对照呈现层行为（混合模式 / 透明排序 / 绘制顺序 / 粒子 / 动画 / UI 观感）。反编译证据 MUST NOT 进入权威状态 / 状态哈希 / 快照 / golden hash / determinism 验收 / CI 门禁；表现层保持只读派生；WC3 资产与数据表 MUST 仍以 `godot_warcraft3` 导出管线为准（ADR 0001 / 0002 / 0003）。

#### Scenario: 离屏对照不等于原作保真验收

- **WHEN** 用离屏 RGBA + ViewImage 对某个呈现特性做像素 / 观感对照
- **THEN** 判定 MUST 收敛到「相同动作 / 时间点 / 机位的原作参考」与真窗口人工验收（依据 `godot_warcraft3@18b7a70:docs/design/asset-convert/FX_BAKE_FIDELITY.md:48,50,59`）；Headless 检查与截图验证 MUST NOT 被写成「GPU 画面正确」或「原作保真验收已通过」

#### Scenario: 证据不进确定性门禁

- **WHEN** 收尾核对反编译证据与视觉 oracle 的落点
- **THEN** 证据不进 FNV-1a 状态哈希 / `SNAPSHOT_VERSION` 快照 / `first_difference()` / golden hash / `determinism` 验收 / CI 门禁；`git diff -- crates/w3-core/` 为空、三版本号仍 == 9、无 `cargo test -p w3-core --test determinism` 重跑任务（ADR 0002）

#### Scenario: 资产与数据表来源不迁移

- **WHEN** 试点需要 WC3 资产 / 数据表 / 数值
- **THEN** 仍走 `godot_warcraft3` 导出管线（MPQ→SLK / BLP / MDX→地图 JSON 三车道、不入库），外部工具无 MPQ / BLP / MDX / SLK 专用解析器；参考项目 AlphaKey 阈值 / FilterMode 等数值只作 `[待核]` 候选观察字段，WC3 数值唯一来源为本机导出 SLK（`WC3-MELEE-RULES.md:7,126`）

### Requirement: 试点边界与规格门待确认项

试点 SHALL 限**单个呈现特性**、形态 A；具体特性、合规逐条判定、行为规格文档字段 / 粒度 / 版本化、视觉 oracle 验收判据、Deep 模式 + Clean room 角色隔离组合 SHALL 由规格门用户拍板，AI MUST NOT self-approve。系统 MUST NOT 改业务规则、不登记第六个只读参考项目、不改 p14「明确不纳入」边界、不改主 `openspec/specs/**`、不追溯改写 P11 归档件、不接 CI（ADR 0003 第二次修订）。

#### Scenario: 单特性试点由人拍板

- **WHEN** 规格门审核本 change 的试点范围
- **THEN** 试点特性选择（含推荐 + 依据 + 代价 + 备选）与 5 项参考缺口保持「待确认项」、由人拍板，不臆造答案；`## 规格门` 保持 `status: pending`、不填指纹 / `approved_by` / `date`、不写省略说明、AI 未 self-approve

#### Scenario: 参考项目对照与红线不越界

- **WHEN** 收尾核对 `design.md`「## 参考项目对照」与变更边界
- **THEN** 逐条登记 T5 主命中、T1 / T3 / T7 可选交叉（仅红线）、T2 / T4 / T6 不命中，T1 承重锚为 `vera20k@305ea3a75:<path>:<line>` 固定提交双标；五个只读参考项目不变、`craft-anything` 不登记为第六个；`git diff -- openspec/specs/` / `-- docs/` / `-- crates/` / `-- tools/` 为空

### Requirement: 非玩法边界与规则文档同步结论

本 capability SHALL 声明为**非玩法 / 呈现层证据工作流**能力：不实现任何 `R-x.y`、不碰 1.0 范围 S1–S6。`decision-memo.md`「## 规则文档同步」SHALL 按 `docs/references/WC3-MELEE-RULES.md:424` §14 第 5 条给出「无需同步」结论并写明依据，保持非空；MUST NOT 把规则编号引用冒充规则已实现。

#### Scenario: 无需同步结论与豁免

- **WHEN** 收尾运行规则文档同步结构门禁与内容复核
- **THEN** `decision-memo.md`「## 规则文档同步」给出「无需同步」结论（非玩法 / 不承接 `R-x.y` / 不改 §13.7 / §15 / 附录覆盖对照 / 参考项目数量），`git diff -- docs/references/WC3-MELEE-RULES.md` 为空；`rule_doc_sync_check()` 对 `status: pending` / 非玩法 change 豁免（结论语义由 `critic` 清单第 13 项复核）
