## Context

P0 已建立确定性内核（ADR 0002）。P1 需要地图与单位数据，但 ADR 0003 明确第一版不写原生 MPQ / SLK / MDX / W3X 解析器。上游 `godot_warcraft3` 的 Node 工具已把经典客户端导出为 JSON，其 schema 记录在 `docs/architecture/MAP_DATA.md` 与 `tools/{map-parse,slk-export}/src`。实现见 commit `b21f111`。

## Goals / Non-Goals

**Goals:** 用最小成本把本机导出 JSON 变成中立 Rust 结构；在边界校验一致性；保持仓库不含任何资产。

**Non-Goals:** 原生格式解析；Godot 宿主；把坐标直接喂给模拟；提交任何导出产物。

## Decisions

- **只读导出 JSON**：复用上游工具而非重写解析器。否决"先写全套 Rust 原生解析器"（成本高、阻塞核心）。
- **serde 忽略未知字段**：上游 schema 会增长，读取器只声明 P1 需要的字段。否决逐字段硬编码全部字段。
- **f64 仅存在于内容层**：原始坐标是冻结输入，不进权威状态/快照/哈希；进入 `w3-core` 前 MUST 按 ADR 0002 冻结定点转换规则。否决在 P1 就悄悄把浮点混入模拟。
- **合成 fixture 做回归**：测试不依赖真实导出数据。否决把 MPQ 派生 JSON 提交进仓库。

## Risks / Trade-offs

- [依赖上游导出 schema，格式变化会破坏读取] → 在 `docs/references/UPSTREAM.md` 固定上游提交并显式升级。
- [dev-setup 的 extract 会因单个解包失败而退出] → 已记录绕过方式：跳过 extract 直接跑 slk / maps 步骤。
- [f64 到 SimFixed 的舍入未冻结] → 列为 P2 的前置任务。

## Migration Plan

无迁移：新增 crate。真实导出数据留在 `C:/open_rts/godot_warcraft3/assets/`，通过 `config.toml` 路径读取。

## Open Questions

- P2 的坐标单位、缩放与舍入规则尚未冻结。
- 是否需要 `pathing.json`（WPM）作为模拟寻路输入。
