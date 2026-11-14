# rts-command-hud Specification

## Purpose
定义可玩 RTS 的命令界面契约：命令卡、选择面板、小地图、可见的生产队列与编队/群体输入，让玩家能看懂并按得动一局 Melee，且所有呈现只读派生、不触碰权威状态。

> 相关 ADR：ADR 0001（UI 字体 / 图标非 MPQ 派生、不入库）；ADR 0002（HUD / 选中集合 / 小地图点击为表现层状态，不进哈希 / 快照，输入只经公开提交入口）；ADR 0004（屏幕坐标经冻结边界）；ADR 0005（本地呈现管线接缝不变）。

## Requirements

### Requirement: 命令卡与选择面板

HUD SHALL 显示当前选中单位或建筑的可执行命令卡（移动、攻击、停止、采集、训练、建造、取消）与主选择面板（类型、生命、归属、资源），且随选中集合变化刷新；全部数据 MUST 只读派生。

#### Scenario: 选中农民

- **WHEN** 玩家选中一个或多个农民
- **THEN** 命令卡显示采集与建造等可用动作，选择面板显示其类型与生命

#### Scenario: 选中生产者

- **WHEN** 玩家选中主城或兵营
- **THEN** 命令卡显示可训练的单位与建筑动作

#### Scenario: 空选中

- **WHEN** 选中集合为空
- **THEN** 不显示单位命令卡，且不生成任何权威命令

### Requirement: 生产队列可见

HUD SHALL 为每个选中的生产者显示生产队列条目、单位类型与剩余进度；队列条目完成或取消后 MUST 立即刷新，MUST NOT 出现「队列生效但画面不可见」的呈现。

#### Scenario: 训练中队列可见

- **WHEN** 对生产者下达训练命令
- **THEN** HUD 出现队列条目且剩余帧数随逻辑帧递减，进度可观察

#### Scenario: 完成与取消刷新

- **WHEN** 队列条目完成或被取消
- **THEN** 对应条目从 HUD 消失，完成后新实体在对局画面中出现

### Requirement: 小地图

HUD SHALL 呈现只读小地图，显示地形轮廓、己方/敌方单位、建筑、资源点位置与相机视口框；点击小地图 SHALL 只平移表现层相机，MUST NOT 生成权威命令。

#### Scenario: 小地图显示实体

- **WHEN** 一局进行中且存在多个实体
- **THEN** 小地图按队伍色标出实体位置并绘制当前相机视口框

#### Scenario: 点击小地图平移

- **WHEN** 玩家点击小地图某处
- **THEN** 相机平移到该处，逐帧状态哈希与快照字节不变

### Requirement: 编队与群体命令

输入层 SHALL 支持编队（编组与按组选择）与群体命令（多选后单次移动或攻击按稳定实体 ID 分配落点）；选中集合 MUST 保持为表现层状态，MUST NOT 生成权威命令或进入状态哈希。

#### Scenario: 编组选择

- **WHEN** 玩家把当前选中集合编组后再按组号选择
- **THEN** 恢复该组的表现层选中集合，权威状态哈希不变

#### Scenario: 群体移动

- **WHEN** 多选单位后对地面下达一次移动命令
- **THEN** 生成带稳定分组信息的 `Move` 命令，落点按稳定实体 ID 分配

### Requirement: UI 文案 i18n 与可见性

UI 文案 SHALL 走 i18n 层，MUST NOT 进入日志、权威状态或状态哈希；HUD 所有文字层 SHALL 以已启用的开源内置字体正常显示；MUST NOT 使用 MPQ 派生字体或向仓库提交字体文件。中文上屏（CJK 字形）SHALL 留后续独立 change（涉及新增字体依赖与许可证人工拍板）。

#### Scenario: 文字层可见

- **WHEN** 在有显示设备的桌面运行
- **THEN** 命令卡、选择面板、资源与队列文字均可读，无文字层整体缺失的降级

#### Scenario: 文案不进日志与哈希

- **WHEN** HUD 刷新中文或 ASCII 文案
- **THEN** 文案不写入任何 `log` 记录，`compute_state_hash()` 与快照字节不变

### Requirement: HUD 只读派生与不改确定性

HUD 所有数据 SHALL 只从只读 `WorldView`、事实事件与只读 `Snapshot` 派生；MUST NOT 反写模拟状态；`w3-core` MUST 零改动、`SNAPSHOT_VERSION` MUST 仍为 3，`crates/w3-render/tests/isolation.rs` MUST 全绿。

#### Scenario: HUD 刷新不改哈希

- **WHEN** 选中、相机、命令卡或小地图随交互连续变化
- **THEN** 逐帧 `compute_state_hash()` 与捕获的快照字节与无 HUD 实例一致

#### Scenario: 内核零 diff

- **WHEN** 校验本 capability 的全部改动
- **THEN** `git diff -- crates/w3-core/` 为空，既有权威状态字段集合不变
