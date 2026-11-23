//! 开局布置（design D1 / D2）：静态世界、双方实体的 `Spawn` 脚本与表现层类型标签。
//!
//! 全部坐标由 `SimFixed::from_int` 从整数常量冻结；实体 ID 按
//! `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定顺序分配，因此脚本
//! 可以在装配期确定每个实体的 `EntityId`，供 AI 命令源与呈现层引用。
//!
//! P8.3 双族对抗（design D1）：本地 / AI 两个席位各自带独立的种族参数，
//! 经 `with_races` 装配；`with_race` 保留为 `with_races(race,
//! race.opponent())` 兼容别名。种族只停在本层 `EntityId → DefId` 关联与
//! 新类别绑定，MUST NOT 写入 `w3-core` 的 `Player` / `RtsMatch`
//! （不新增权威状态字段），也 MUST NOT 改动任何 `Spawn` 命令字节。

use std::collections::BTreeMap;

use w3_core::{
    Command, CommandKind, DefId, EntityId, Frame, FrozenDefinitions, GameplayCommand,
    GameplayCommandKind, SimFixed, StaticWorld,
};

use crate::input::{MovabilityMap, derive_movability};

use super::defs::Race;

/// 合成世界格数。
pub const WORLD_WIDTH: u32 = 32;
pub const WORLD_HEIGHT: u32 = 32;
/// 合成世界与地形的统一格距。
pub const CELL_SIZE: i64 = 8;
/// 本地玩家编号。
pub const LOCAL_PLAYER: u8 = 1;
/// AI 对手编号。
pub const AI_PLAYER: u8 = 2;

/// 开局出生点对齐真实 Echo Isles 的两个 sloc（`units.json` 缩放后世界坐标）。
///
/// 真实导出 sloc：本地 `(-5184, 2944)`、AI `(4672, 2944)`，乘 `WORLD_SCALE
/// = 2/128` 得 `(-81, 46)`、`(73, 46)`。玩法世界（`synthetic_world`，覆盖
/// `[-128,128]`）与缩放后真实地形是同一坐标系，故把主城落在此处，动态单位
/// 会经 `terrain.height_at_world` 贴到真实陆地、农民环绕主城、金矿 / 树林在
/// 侧后方——对齐参考项目 `MeleeBootstrap.spawn_at_sloc` 的「正常 War3 开局」。
///
/// 这些是**编译期整数常量**（合成 / 原创冻结定义的一部分），CPU 闭环测试零
/// 资产依赖不变；坐标进 `Spawn` 命令（权威状态），但 `w3-core` 的 golden hash
/// 用独立 fixture、闭环测试比对重放哈希 / 终态相等而非固定坐标，故确定性契约
/// 不受影响。`w3-core` 零改动。
///
/// 本地主城在 sloc 起始区的**天然无树空地** `(-74, 38)`（距 sloc 约 11、
/// 半径 14 内 0 棵静态树）：真实 EI 出生点被密林环绕，若把主城放字面 sloc
/// `(-81,46)`，拉近相机后视野会被放大后的树冠糊满、看不见主基地。此空地经
/// 离线量化主城 / 农民 / 金矿 / 树 6 个落点全为陆地且主城周围无树冠遮挡，
/// 对齐参考项目「起始区无树、主城居中、农民环绕」的开局观感。AI 侧陆块较
/// 小且多树，玩家开局不可见（相距 155），不影响本地可玩性。
///
/// 农民簇朝地图中心方向（本地 +x / AI -x）在主城前方一字排开；金矿与树林在
/// 背离中心一侧。动态单位模型经表现层放大 4×（`MODEL_DISPLAY_SCALE ×
/// ENTITY_DISPLAY_SCALE`），故农民簇需相应拉开，站在主城前方开阔空地、彼此
/// 分开、清晰可框选，而非挤在主城墙根。全部落点落在各自陆块（ground>water），
/// 不泡水、不压地图边缘、主城周围无树冠遮挡。
const LOCAL_START: (i64, i64) = (-74, 38);
const AI_START: (i64, i64) = (74, 48);
/// 本地 / AI 农民簇疏密：本地陆块开阔用 10，AI 陆块窄用 4（玩家开局不可见，
/// 且该值经离线量化保证 AI 侧 6 个落点全为陆地、不泡水）。
const LOCAL_SPREAD: i64 = 10;
const AI_SPREAD: i64 = 4;

/// 以出生点 `start`、朝向 `facing`（本地 +1 / AI -1）与簇疏密 `spread` 生成
/// 开局布局。返回 `(主城, [农民3], 金矿, 树)`，与参考项目农民朝金矿方向聚簇
/// 同语义；农民在主城前方 `spread` 处一字排开、纵向间距 `spread`，金矿 / 树在
/// 主城背离中心一侧。
fn opening_layout(start: (i64, i64), facing: i64, spread: i64) -> [(i64, i64); 6] {
    let (hx, hy) = start;
    let front = hx + facing * spread;
    let back = hx - facing * (spread + 2);
    [
        (hx, hy),             // 主城：sloc
        (front, hy + spread), // 农民 1
        (front, hy),          // 农民 2
        (front, hy - spread), // 农民 3
        (back, hy + spread),  // 金矿
        (back, hy - spread),  // 树
    ]
}

/// 实体类型标签（表现层专属，MUST NOT 进入权威状态 / 哈希 / 快照）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntityKind {
    Peasant,
    Soldier,
    TownHall,
    Barracks,
    GoldMine,
    Tree,
}

impl EntityKind {
    /// 呈现层模型键；`ModelLibrary::bind_entity` 用它按类型选择几何。
    pub fn model_key(self) -> &'static str {
        match self {
            Self::Peasant => "peasant",
            Self::Soldier => "soldier",
            Self::TownHall => "town-hall",
            Self::Barracks => "barracks",
            Self::GoldMine => "gold-mine",
            Self::Tree => "tree",
        }
    }

    /// 是否可作为生产者（命令卡显示训练动作）。
    pub fn is_producer(self) -> bool {
        matches!(self, Self::TownHall | Self::Barracks)
    }

    /// 是否为资源点。
    pub fn is_resource(self) -> bool {
        matches!(self, Self::GoldMine | Self::Tree)
    }

    /// 命令卡标签（ASCII，走 i18n 边界，不进日志 / 权威状态 / 哈希）。
    pub fn label(self) -> &'static str {
        match self {
            Self::Peasant => "Peasant",
            Self::Soldier => "Soldier",
            Self::TownHall => "Town Hall",
            Self::Barracks => "Barracks",
            Self::GoldMine => "Gold Mine",
            Self::Tree => "Tree",
        }
    }
}

/// 由整数字面量构造 Q47.16 定点值；显式处理错误，不静默吞错。
fn fixed(value: i64) -> SimFixed {
    SimFixed::from_int(value).expect("melee setup: reason=fixed_conversion_failed")
}

/// 合成世界：以世界原点为中心的开阔地形，不读取任何地图数据。
pub fn synthetic_world() -> StaticWorld {
    let half = fixed(-(WORLD_WIDTH as i64) * CELL_SIZE / 2);
    StaticWorld::new(
        WORLD_WIDTH,
        WORLD_HEIGHT,
        fixed(CELL_SIZE),
        (half, half),
        Vec::new(),
        Vec::new(),
    )
}

/// 一局的开局布置：命令脚本、玩法命令与表现层类型映射。
#[derive(Clone, Debug)]
pub struct MeleeSetup {
    /// 普通命令（全部 `Spawn`，执行帧 0）。
    pub commands: Vec<Command>,
    /// 玩法命令（创建双方玩家经济的 `Gather`，执行帧 0）。
    pub gameplay_commands: Vec<GameplayCommand>,
    /// 表现层类型映射（实体 ID → 类型），不进权威状态。
    pub kinds: BTreeMap<u64, EntityKind>,
    /// 表现层定义的关联（实体 ID → 分段 `DefId`），只用于从冻结定义派生
    /// 可移动性判据（design D1 / D2）。`CommandKind::Spawn` 只携带冻结初值、
    /// 不携带 `DefId`，故种族维度只停在这一层，MUST NOT 进入权威状态 /
    /// 状态哈希 / 快照；两族装配的开局命令字节完全一致。
    pub def_ids: BTreeMap<u64, DefId>,
    /// **本地席位**的新类别装配绑定（槽位 → 分段 `DefId`）。
    ///
    /// P8.3 起 `category_def_ids` 字段与 [`MeleeSetup::category_def_ids`]
    /// 访问器保留为**本地份别名**（兼容既有验收与退化注入点 ≥10 处），
    /// 由 `local_race.category_def_ids()` 派生；AI 席位另见
    /// [`MeleeSetup::ai_category_def_ids`]。本表与 AI 份 MUST NOT 串用。
    /// `CommandKind::Spawn` 不含 `DefId`，frame-0 静态集合只有
    /// 主城 / 农民 / 资源点，故新增六类不进 `def_ids`、不改任何 `Spawn`
    /// 命令字节；本地验收 / `category_movability` 用本份，AI 脚本用 AI 份。
    /// 这是表现层专属数据，MUST NOT 进入权威状态 / 哈希 / 快照；
    /// 运行时 `Train` / `Build` 产生的新实体不在此表（P8.0 已知限制 B1，
    /// 本 change 不擅自闭合、不改 `unwrap_or` 语义、不扩展视图）。
    pub category_def_ids: BTreeMap<u64, DefId>,
    /// **AI 席位**的新类别装配绑定（槽位 → 分段 `DefId`，design D2）。
    ///
    /// 由 `ai_race.category_def_ids()` 派生，供 `AiScript::new` 取训练 /
    /// 施法 / 研发目标，使 AI 命令的 `DefId` 落在 AI 自身种族段（C3）。
    /// 异族组合两份互不相交、同族组合两份段位一致；同为表现层专属，
    /// MUST NOT 进入权威状态 / 哈希 / 快照。
    pub ai_category_def_ids: BTreeMap<u64, DefId>,
    /// 本局本地玩家所选种族（定义层参数，回落人族段；本地族别名）。
    ///
    /// P8.3 保留字段名与含义不变（既有 `.race` 断言不破坏），仍是
    /// `with_races` 的 `local_race` 入参，MUST NOT 与 AI 族混淆。
    pub race: Race,
    /// 本局 AI 对手所选种族（**装配层只读**字段，design D1）。
    ///
    /// 与 `race`（本地族）解耦为两个独立参数，使 P9 / Q1' 4×4 十六组合
    /// （含同族组合）可装配。
    /// 只用于选择 AI 席位 `Spawn` 关联的 `DefId` 段与 AI 脚本查表，
    /// MUST NOT 写入 `w3-core` 的 `Player` / `RtsMatch`、不进状态哈希 /
    /// 快照（不新增权威状态字段）；`with_race` 别名回落 `race.opponent()`。
    pub ai_race: Race,
    pub local_hall: EntityId,
    pub ai_hall: EntityId,
    pub local_peasants: Vec<EntityId>,
    pub ai_peasants: Vec<EntityId>,
    pub local_mine: EntityId,
    pub ai_mine: EntityId,
}

/// 开局脚本构造器：按玩家与序列稳定推进，同时记录实体 ID 与类型。
struct Builder {
    commands: Vec<Command>,
    kinds: BTreeMap<u64, EntityKind>,
    def_ids: BTreeMap<u64, DefId>,
    next_id: u64,
    next_sequence: [u32; 3],
}

impl Builder {
    fn new() -> Self {
        Self {
            commands: Vec::new(),
            kinds: BTreeMap::new(),
            def_ids: BTreeMap::new(),
            next_id: 1,
            next_sequence: [0; 3],
        }
    }

    /// 追加一条 `Spawn`；ID 按构造顺序分配，与内核执行顺序一致。
    /// `def_id` 是该出生实体在 client 侧冻结定义中的分段编号（表现层关联，
    /// 不写入 `CommandKind::Spawn` 的权威初值）。
    fn spawn(
        &mut self,
        player: u8,
        x: i64,
        y: i64,
        hp: i64,
        speed: i64,
        damage: i64,
        range: i64,
        kind: EntityKind,
        def_id: DefId,
    ) -> EntityId {
        let slot = &mut self.next_sequence[player as usize];
        *slot += 1;
        let id = self.next_id;
        self.next_id += 1;
        self.commands.push(Command::new(
            Frame::new(0),
            player,
            *slot,
            CommandKind::Spawn {
                x: fixed(x),
                y: fixed(y),
                hp,
                // 内核 `Command::submit` 要求 `speed.raw() > 0`；建筑 / 资源点
                // 不下达移动命令，但仍以最小正值合法构造，不绕过既有校验。
                speed: fixed(speed.max(1)),
                attack_damage: damage,
                attack_range: fixed(range),
            },
        ));
        self.kinds.insert(id, kind);
        self.def_ids.insert(id, def_id);
        EntityId::new(id).expect("melee setup: reason=invalid_entity_id")
    }
}

impl MeleeSetup {
    /// 构造默认开局：回落人族段（本地人族、AI 兽族），既有入口行为不变。
    ///
    /// 装配层 `def_ids` 的段位差异不影响任何 `Spawn` 命令字节，故权威状态
    /// 哈希、快照与改动前一致（design D2 / ADR 0002）。P8.3 起等价于
    /// `with_races(Race::Human, Race::Orc)`，逐字节行为不变。
    pub fn new() -> Self {
        Self::with_race(Race::Human)
    }

    /// 兼容别名：`with_race(race)` 严格等于 `with_races(race,
    /// race.opponent())`（design D1）。既有 `MeleeSetup::new()` /
    /// `with_race` 入口、两族基线 / 回落测试逐字节不变；删除本别名或
    /// 改写回落语义会令既有回落基线测试失败（tasks 6.3 / C5）。
    pub fn with_race(race: Race) -> Self {
        Self::with_races(race, race.opponent())
    }

    /// 双族独立参数装配：双方各一座主城、三名农民、一处金矿与一片树林。
    /// 本地席位用 `local_race` 段、AI 席位用 `ai_race` 段，中立资源恒
    /// `9xx` 段（design D1 / D2 定义层种族分段，R2 四族不对称）。
    ///
    /// 两个参数彼此独立，故 P9 / Q1' 4×4 十六组合（含同族组合）都能
    /// 装配（人 / 兽 / 不死 / 暗夜四族各段）；`race` 字段继续作本地族
    /// 别名、`ai_race` 字段记录 AI 族。主城落在真实 Echo Isles 的两个 sloc（见
    /// [`LOCAL_START`] / [`AI_START`]），农民朝地图中心方向聚在主城前方、
    /// 金矿与树林在侧后方，全部落在各自陆块的地面上。主城相距约 155
    /// 世界单位，处于合成士兵经移动后的攻城射程内，使一局可在合理帧数
    /// 内分出胜负。坐标全为编译期整数常量；种族参数只写入装配层
    /// `def_ids` 与两份新类别绑定，MUST NOT 新增权威状态字段或改动
    /// `Spawn` 初值（四组合装配的 `Spawn` 字节逐字节一致）。
    pub fn with_races(local_race: Race, ai_race: Race) -> Self {
        let mut b = Builder::new();
        let local = opening_layout(LOCAL_START, 1, LOCAL_SPREAD);
        let ai = opening_layout(AI_START, -1, AI_SPREAD);

        // 本地玩家（player 1）：ID 1..=6。
        let local_hall = b.spawn(
            LOCAL_PLAYER,
            local[0].0,
            local[0].1,
            120,
            0,
            0,
            0,
            EntityKind::TownHall,
            local_race.town_hall(),
        );
        let local_peasants = vec![
            b.spawn(
                LOCAL_PLAYER,
                local[1].0,
                local[1].1,
                60,
                3,
                4,
                4,
                EntityKind::Peasant,
                local_race.peasant(),
            ),
            b.spawn(
                LOCAL_PLAYER,
                local[2].0,
                local[2].1,
                60,
                3,
                4,
                4,
                EntityKind::Peasant,
                local_race.peasant(),
            ),
            b.spawn(
                LOCAL_PLAYER,
                local[3].0,
                local[3].1,
                60,
                3,
                4,
                4,
                EntityKind::Peasant,
                local_race.peasant(),
            ),
        ];
        let local_mine = b.spawn(
            LOCAL_PLAYER,
            local[4].0,
            local[4].1,
            500,
            0,
            0,
            0,
            EntityKind::GoldMine,
            local_race.gold_mine(),
        );
        b.spawn(
            LOCAL_PLAYER,
            local[5].0,
            local[5].1,
            200,
            0,
            0,
            0,
            EntityKind::Tree,
            local_race.tree(),
        );

        // AI 对手（player 2）：ID 7..=12。
        let ai_hall = b.spawn(
            AI_PLAYER,
            ai[0].0,
            ai[0].1,
            120,
            0,
            0,
            0,
            EntityKind::TownHall,
            ai_race.town_hall(),
        );
        let ai_peasants = vec![
            b.spawn(
                AI_PLAYER,
                ai[1].0,
                ai[1].1,
                60,
                3,
                4,
                4,
                EntityKind::Peasant,
                ai_race.peasant(),
            ),
            b.spawn(
                AI_PLAYER,
                ai[2].0,
                ai[2].1,
                60,
                3,
                4,
                4,
                EntityKind::Peasant,
                ai_race.peasant(),
            ),
            b.spawn(
                AI_PLAYER,
                ai[3].0,
                ai[3].1,
                60,
                3,
                4,
                4,
                EntityKind::Peasant,
                ai_race.peasant(),
            ),
        ];
        let ai_mine = b.spawn(
            AI_PLAYER,
            ai[4].0,
            ai[4].1,
            500,
            0,
            0,
            0,
            EntityKind::GoldMine,
            ai_race.gold_mine(),
        );
        b.spawn(
            AI_PLAYER,
            ai[5].0,
            ai[5].1,
            200,
            0,
            0,
            0,
            EntityKind::Tree,
            ai_race.tree(),
        );

        // 双方各提交一条开局 `Gather`：既让农民开始采集，也确保两个玩家的
        // 经济实体在内核中登记（`resolve_victory` 要求至少两个 active 玩家）。
        // 命令带固定 `ExecuteFrame` / `PlayerId` / `Sequence`，是外部命令，
        // 重放注入同一脚本即可复现。
        let gameplay_commands = vec![
            GameplayCommand::new(
                Frame::new(0),
                LOCAL_PLAYER,
                1,
                GameplayCommandKind::Gather {
                    unit: local_peasants[0],
                },
            ),
            GameplayCommand::new(
                Frame::new(0),
                AI_PLAYER,
                1,
                GameplayCommandKind::Gather {
                    unit: ai_peasants[0],
                },
            ),
        ];

        Self {
            commands: b.commands,
            gameplay_commands,
            kinds: b.kinds,
            def_ids: b.def_ids,
            // 本地份：由 local_race 派生，作 `category_def_ids()` 别名。
            category_def_ids: local_race.category_def_ids(),
            // AI 份：由 ai_race 派生，仅供 `AiScript::new` 查表（C3）。
            ai_category_def_ids: ai_race.category_def_ids(),
            race: local_race,
            ai_race,
            local_hall,
            ai_hall,
            local_peasants,
            ai_peasants,
            local_mine,
            ai_mine,
        }
    }

    /// 装配期从 client 侧冻结定义派生 `EntityId → 可移动性` 只读映射
    /// （design D1）：判据是 `FrozenDefinitions::units()` 的
    /// `UnitDef.speed_raw`，MUST NOT 用运行时 `speed` 或建筑类型白名单。
    /// 该映射是表现层只读数据，MUST NOT 进入权威状态 / 状态哈希 / 快照。
    pub fn movability(&self, defs: &FrozenDefinitions) -> MovabilityMap {
        derive_movability(defs, &self.def_ids)
    }

    /// 装配期从冻结定义为新增类别派生 `槽位 → 可移动性` 只读映射
    /// （design D1 / D7）：判据仍是 `UnitDef.speed_raw > 0`，MUST NOT
    /// 用运行时 `speed` 或建筑类型白名单。该映射是表现层只读数据，
    /// MUST NOT 进入权威状态 / 状态哈希 / 快照；只覆盖 frame-0 静态
    /// 装配的新类别绑定，不擅自闭合 P8.0 运行时动态实体限制 B1。
    pub fn category_movability(&self, defs: &FrozenDefinitions) -> MovabilityMap {
        derive_movability(defs, &self.category_def_ids)
    }

    /// 只读**本地份**新增类别装配绑定（槽位 → `DefId`），作兼容别名。
    ///
    /// 供本地验收（段位 / 唯一性 / 退化注入）与 `category_movability`
    /// 使用；P8.3 起 `AiScript::new` 改用 [`MeleeSetup::ai_category_def_ids`]，
    /// 两份 MUST NOT 串用。
    pub fn category_def_ids(&self) -> &BTreeMap<u64, DefId> {
        &self.category_def_ids
    }

    /// 只读**AI 份**新增类别装配绑定（槽位 → `DefId`，design D2）。
    ///
    /// `AiScript::new` 经此查表取训练 / 施法 / 研发目标，DefId 落在
    /// `ai_race` 段；异族组合与本地份互不相交、同族组合段位一致。
    pub fn ai_category_def_ids(&self) -> &BTreeMap<u64, DefId> {
        &self.ai_category_def_ids
    }

    /// 按种族只读查表：返回该种族对应的新类别绑定。
    ///
    /// 本地族（`race`）返回本地份、AI 族（`ai_race`）返回 AI 份。同族
    /// 组合两份内容一致，此时按 `race` 优先返回本地份；异族组合按入参
    /// 精确命中对应席位。装配层只读数据，MUST NOT 进权威状态 / 哈希 /
    /// 快照。
    pub fn category_def_ids_for(&self, race: Race) -> &BTreeMap<u64, DefId> {
        if race == self.race {
            &self.category_def_ids
        } else {
            &self.ai_category_def_ids
        }
    }

    /// 只读分段定义关联（实体 ID → `DefId`），供验收核对段位。
    pub fn def_ids(&self) -> &BTreeMap<u64, DefId> {
        &self.def_ids
    }

    /// 表现层模型键映射，供 `ModelLibrary::bind_entity` 注入。
    pub fn model_keys(&self) -> BTreeMap<u64, &'static str> {
        self.kinds
            .iter()
            .map(|(&id, &kind)| (id, kind.model_key()))
            .collect()
    }
}

impl Default for MeleeSetup {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use w3_core::SCALE;

    #[test]
    fn setup_has_stable_ids_and_kinds() {
        let setup = MeleeSetup::new();
        assert_eq!(setup.commands.len(), 12);
        assert_eq!(setup.gameplay_commands.len(), 2);
        assert_eq!(setup.kinds.len(), 12);
        assert_eq!(setup.local_hall.value(), 1);
        assert_eq!(setup.ai_hall.value(), 7);
        assert_eq!(setup.local_peasants.len(), 3);
        assert_eq!(setup.ai_peasants.len(), 3);
        assert_eq!(setup.local_mine.value(), 5);
        assert_eq!(setup.ai_mine.value(), 11);
        assert_eq!(setup.kinds.get(&1), Some(&EntityKind::TownHall));
        assert_eq!(setup.kinds.get(&7), Some(&EntityKind::TownHall));
        assert_eq!(setup.kinds.get(&5), Some(&EntityKind::GoldMine));
        assert_eq!(setup.model_keys().len(), 12);
    }

    #[test]
    fn spawn_commands_carry_frozen_frame_zero_coordinates() {
        let setup = MeleeSetup::new();
        for command in &setup.commands {
            assert_eq!(command.execute_frame, Frame::new(0));
            assert!(matches!(command.kind, CommandKind::Spawn { .. }));
        }
        // 同一玩家内序列唯一。
        let mut seen = std::collections::BTreeSet::new();
        for command in &setup.commands {
            assert!(seen.insert((command.player_id, command.sequence)));
        }
    }

    #[test]
    fn model_keys_distinguish_all_types() {
        assert_ne!(
            EntityKind::Peasant.model_key(),
            EntityKind::Soldier.model_key()
        );
        assert_ne!(
            EntityKind::TownHall.model_key(),
            EntityKind::Barracks.model_key()
        );
        assert!(EntityKind::TownHall.is_producer());
        assert!(EntityKind::GoldMine.is_resource());
        assert!(!EntityKind::Peasant.is_producer());
    }

    /// 从 frame-0 `Spawn` 命令取出 `(player, x, y)`（Q47.16 → 世界单位整数）。
    fn spawn_positions(setup: &MeleeSetup) -> Vec<(u8, i64, i64)> {
        setup
            .commands
            .iter()
            .map(|command| match &command.kind {
                CommandKind::Spawn { x, y, .. } => {
                    (command.player_id, x.raw() / SCALE, y.raw() / SCALE)
                }
                other => panic!("expected Spawn, got {other:?}"),
            })
            .collect()
    }

    #[test]
    fn opening_halls_align_to_real_echo_isles_start_clearing() {
        // 主城落在真实 EI sloc 起始区的无树空地，而非旧的合成浅滩 (-96,0)。
        let setup = MeleeSetup::new();
        let positions = spawn_positions(&setup);
        let local: Vec<_> = positions
            .iter()
            .filter(|(p, _, _)| *p == LOCAL_PLAYER)
            .map(|(_, x, y)| (*x, *y))
            .collect();
        // 写死字面量（非引用 LOCAL_START）：守护「主城确实落在真实 EI 起始
        // 区无树空地」这一事实，常量被改成别的错误坐标时此处会失败。
        assert_eq!(
            local[0],
            (-74, 38),
            "本地主城必须落在真实 EI sloc 起始区的无树空地 (-74,38)"
        );
        assert!(
            local[0].0 > -100 && local[0].1 > 20,
            "主城不得落在岛南浅滩/海上（旧合成坐标 -96,0 在真实地图是水）"
        );
        // 全部 12 个落点都在玩法世界 `[-128,128]` 内（`synthetic_world` 边界）。
        for (_, x, y) in &positions {
            assert!(
                (-128..=128).contains(x) && (-128..=128).contains(y),
                "开局坐标越界: ({x},{y})"
            );
        }
    }

    #[test]
    fn opening_peasants_are_spread_in_front_of_hall() {
        // 农民簇在主城前方、彼此分开（放大 4× 后仍可框选），不挤在主城墙根。
        let setup = MeleeSetup::new();
        let positions = spawn_positions(&setup);
        let local: Vec<_> = positions
            .iter()
            .filter(|(p, _, _)| *p == LOCAL_PLAYER)
            .map(|(_, x, y)| (*x, *y))
            .collect();
        let hall = local[0];
        let peasants = &local[1..4];
        // 都在主城朝向地图中心一侧（+x）。
        for p in peasants {
            assert!(p.0 > hall.0, "农民应在主城前方，实际 {p:?} vs {hall:?}");
        }
        // 纵向 y 互不相同、且拉开足够间距（>= LOCAL_SPREAD）。
        let mut ys: Vec<i64> = peasants.iter().map(|p| p.1).collect();
        ys.sort();
        assert_eq!(
            ys.windows(2)
                .filter(|w| w[1] - w[0] >= LOCAL_SPREAD)
                .count(),
            2,
            "三个农民纵向应各相距至少一个 spread，实际 {ys:?}"
        );
    }

    #[test]
    fn opening_layout_is_deterministic_and_halls_far_apart() {
        // 两侧布局确定性（重放同一脚本坐标一致）；注意两侧 start/spread 本就
        // 不同（AI 陆块窄），故不是「对称」布局，此处只断言确定性与主城分处
        // 东西两片陆块、相距足够远以便一局分出胜负。
        let a = MeleeSetup::new();
        let b = MeleeSetup::new();
        assert_eq!(spawn_positions(&a), spawn_positions(&b), "开局布局确定性");
        let positions = spawn_positions(&a);
        let local_hall = positions
            .iter()
            .find(|(p, _, _)| *p == LOCAL_PLAYER)
            .map(|(_, x, y)| (*x, *y))
            .unwrap();
        let ai_hall = positions
            .iter()
            .find(|(p, _, _)| *p == AI_PLAYER)
            .map(|(_, x, y)| (*x, *y))
            .unwrap();
        assert_eq!(local_hall, (-74, 38));
        assert_eq!(ai_hall, (74, 48));
        assert!(
            ai_hall.0 - local_hall.0 > 100,
            "两主城应分处东西两片陆块、相距足够远以便一局分出胜负"
        );
    }

    /// 两族装配：双方各取对应段的 `DefId`，且一局仍是 12 个 `Spawn`。
    #[test]
    fn two_races_assemble_with_segmented_def_ids() {
        use crate::melee::defs::{
            HUMAN_BARRACKS, HUMAN_PEASANT, HUMAN_SOLDIER, HUMAN_TOWN_HALL, NEUTRAL_GOLD_MINE,
            NEUTRAL_TREE, ORC_BARRACKS, ORC_PEASANT, ORC_SOLDIER, ORC_TOWN_HALL,
        };

        let human = MeleeSetup::with_race(Race::Human);
        let orc = MeleeSetup::with_race(Race::Orc);
        assert_eq!(human.race, Race::Human);
        assert_eq!(orc.race, Race::Orc);

        for setup in [&human, &orc] {
            assert_eq!(setup.commands.len(), 12, "reason=opening_spawn_count");
            assert_eq!(
                setup.gameplay_commands.len(),
                2,
                "reason=opening_gather_count"
            );
            assert_eq!(setup.kinds.len(), 12, "reason=all_entities_tagged");
            assert_eq!(setup.def_ids.len(), 12, "reason=all_entities_have_def_id");
        }

        // 人族本地单位落在 1xx 段、AI 对手落在 2xx 段。
        let human_local: Vec<DefId> = [1, 2, 3, 4]
            .iter()
            .map(|id| *human.def_ids.get(id).unwrap())
            .collect();
        assert!(
            human_local
                .iter()
                .all(|id| (100..200).contains(id) || (900..1000).contains(id)),
            "reason=human_local_units_in_1xx_or_neutral"
        );
        assert_eq!(human.def_ids.get(&1), Some(&HUMAN_TOWN_HALL));
        assert_eq!(human.def_ids.get(&2), Some(&HUMAN_PEASANT));
        assert_eq!(human.def_ids.get(&5), Some(&NEUTRAL_GOLD_MINE));
        assert_eq!(human.def_ids.get(&6), Some(&NEUTRAL_TREE));
        assert_eq!(human.def_ids.get(&7), Some(&ORC_TOWN_HALL));
        assert_eq!(human.def_ids.get(&8), Some(&ORC_PEASANT));
        assert_eq!(human.def_ids.get(&11), Some(&NEUTRAL_GOLD_MINE));

        // 兽族参数翻转：本地 2xx、AI 1xx，中立资源恒为 9xx。
        assert_eq!(orc.def_ids.get(&1), Some(&ORC_TOWN_HALL));
        assert_eq!(orc.def_ids.get(&2), Some(&ORC_PEASANT));
        assert_eq!(orc.def_ids.get(&5), Some(&NEUTRAL_GOLD_MINE));
        assert_eq!(orc.def_ids.get(&6), Some(&NEUTRAL_TREE));
        assert_eq!(orc.def_ids.get(&7), Some(&HUMAN_TOWN_HALL));
        assert_eq!(orc.def_ids.get(&8), Some(&HUMAN_PEASANT));
        assert_eq!(orc.def_ids.get(&11), Some(&NEUTRAL_GOLD_MINE));

        // 分段常量确实落在各自段内（两族士兵 / 兵营段位）。
        assert!((100..200).contains(&HUMAN_SOLDIER));
        assert!((100..200).contains(&HUMAN_BARRACKS));
        assert!((200..300).contains(&ORC_SOLDIER));
        assert!((200..300).contains(&ORC_BARRACKS));
    }

    /// 回落兼容：未传种族参数回落人族段，既有入口行为完全不变。
    #[test]
    fn race_fallback_matches_human_segment_exactly() {
        let fallback = MeleeSetup::new();
        let human = MeleeSetup::with_race(Race::Human);
        assert_eq!(fallback.race, Race::Human);
        assert_eq!(fallback.def_ids, human.def_ids);
        assert_eq!(fallback.commands, human.commands);
        assert_eq!(fallback.gameplay_commands, human.gameplay_commands);
        assert_eq!(fallback.kinds, human.kinds);
        assert_eq!(spawn_positions(&fallback), spawn_positions(&human));
    }

    /// 种族参数只改表现层 `def_ids`，`Spawn` 命令字节完全一致
    /// （design D2：不新增权威状态、逐帧哈希与基线一致）。
    #[test]
    fn race_does_not_change_authoritative_spawn_commands() {
        let human = MeleeSetup::with_race(Race::Human);
        let orc = MeleeSetup::with_race(Race::Orc);
        assert_eq!(
            human.commands, orc.commands,
            "reason=spawn_bytes_race_independent"
        );
        assert_eq!(
            human.gameplay_commands, orc.gameplay_commands,
            "reason=gameplay_bytes_race_independent"
        );
        assert_eq!(human.kinds, orc.kinds, "reason=kinds_race_independent");
        assert_ne!(
            human.def_ids, orc.def_ids,
            "reason=def_ids_are_race_specific"
        );
    }

    /// 新类别装配绑定（P8.1 design D4 / P8.2 design D4）：人族 `1xx`、
    /// 兽族 `2xx` 真实新常量（不再是 `ORC_SOLDIER` / `ORC_BARRACKS` 占位），
    /// 六类各占独立槽位、两族不重名不相同；`Spawn` / `gameplay_commands`
    /// 字节逐字节不受影响（C2 / design D4）。
    #[test]
    fn category_bindings_are_segmented_without_touching_spawn_bytes() {
        use crate::melee::defs::{
            CATEGORY_ALTAR, CATEGORY_FARM, CATEGORY_HERO, CATEGORY_RANGED, CATEGORY_SIEGE,
            CATEGORY_SMITHY, ORC_ALTAR, ORC_BARRACKS, ORC_FARM, ORC_HERO, ORC_RANGED, ORC_SIEGE,
            ORC_SMITHY, ORC_SOLDIER,
        };

        let human = MeleeSetup::with_race(Race::Human);
        let orc = MeleeSetup::with_race(Race::Orc);
        // P9 / Q1'：不死 / 暗夜装配同样登记六类新类别绑定。
        let undead = MeleeSetup::with_race(Race::Undead);
        let night_elf = MeleeSetup::with_race(Race::NightElf);
        let slots = [
            CATEGORY_RANGED,
            CATEGORY_SIEGE,
            CATEGORY_FARM,
            CATEGORY_ALTAR,
            CATEGORY_SMITHY,
            CATEGORY_HERO,
        ];
        for setup in [&human, &orc, &undead, &night_elf] {
            assert_eq!(setup.category_def_ids.len(), 6, "reason=six_new_categories");
            let (lo, hi) = match setup.race {
                Race::Human => (100, 200),
                Race::Orc => (200, 300),
                Race::Undead => (300, 400),
                Race::NightElf => (400, 500),
            };
            for slot in slots {
                let def_id = *setup
                    .category_def_ids
                    .get(&slot)
                    .expect("melee setup: reason=missing_category_slot");
                assert!(
                    (lo..hi).contains(&def_id),
                    "reason=category_def_id_in_race_segment slot={slot} id={def_id}"
                );
            }
        }
        // P8.2：兽族六类已替换占位、指向真实 `2xx` 新常量，且互不相同。
        let orc_expected = [
            (CATEGORY_RANGED, ORC_RANGED),
            (CATEGORY_SIEGE, ORC_SIEGE),
            (CATEGORY_FARM, ORC_FARM),
            (CATEGORY_ALTAR, ORC_ALTAR),
            (CATEGORY_SMITHY, ORC_SMITHY),
            (CATEGORY_HERO, ORC_HERO),
        ];
        for (slot, def_id) in orc_expected {
            assert_eq!(
                orc.category_def_ids.get(&slot),
                Some(&def_id),
                "reason=orc_category_binding_must_be_real_2xx slot={slot}"
            );
            assert_ne!(def_id, ORC_SOLDIER, "reason=not_soldier_placeholder");
            assert_ne!(def_id, ORC_BARRACKS, "reason=not_barracks_placeholder");
        }
        let orc_values: Vec<_> = orc.category_def_ids.values().copied().collect();
        let distinct: std::collections::BTreeSet<_> = orc_values.iter().copied().collect();
        assert_eq!(distinct.len(), 6, "reason=orc_bindings_not_duplicated");
        // 新类别绑定与 frame-0 `def_ids` 命名空间隔离，不新增实体。
        for slot in slots {
            assert!(
                !human.def_ids.contains_key(&slot),
                "reason=category_slot_must_not_be_an_entity_id"
            );
        }
        // 两族绑定都落在各自段、不重名（互不相交）、不相同。
        let human_values: std::collections::BTreeSet<_> =
            human.category_def_ids.values().copied().collect();
        let orc_values_set: std::collections::BTreeSet<_> =
            orc.category_def_ids.values().copied().collect();
        assert!(
            human_values.is_disjoint(&orc_values_set),
            "reason=human_orc_categories_do_not_overlap"
        );
        assert_ne!(
            human.category_def_ids, orc.category_def_ids,
            "reason=category_binding_is_race_specific"
        );
        assert_eq!(human.commands, orc.commands, "reason=spawn_bytes_unchanged");
        assert_eq!(
            human.gameplay_commands, orc.gameplay_commands,
            "reason=opening_gameplay_bytes_unchanged"
        );
    }

    /// P8.1 可移动性（design D7 / tasks 2.3）：新增建筑不可移动、
    /// 远程 / 反重甲单位与英雄可移动；判据来自冻结定义 `speed_raw`，
    /// 不来自运行时 `speed` 或建筑类型白名单。
    #[test]
    fn category_movability_is_derived_from_definition_speed_raw() {
        use crate::melee::defs::{
            CATEGORY_ALTAR, CATEGORY_FARM, CATEGORY_HERO, CATEGORY_RANGED, CATEGORY_SIEGE,
            CATEGORY_SMITHY,
        };

        let defs = crate::melee::defs::synthetic_definitions();
        let setup = MeleeSetup::new();
        let map = setup.category_movability(&defs);
        assert_eq!(map.len(), 6, "reason=one_entry_per_new_category");
        for slot in [CATEGORY_RANGED, CATEGORY_SIEGE, CATEGORY_HERO] {
            assert_eq!(map.get(&slot), Some(&true), "reason=movable slot={slot}");
        }
        for slot in [CATEGORY_FARM, CATEGORY_ALTAR, CATEGORY_SMITHY] {
            assert_eq!(map.get(&slot), Some(&false), "reason=immovable slot={slot}");
        }
    }

    /// P8.2 兽族新类别可移动性（design D7 / tasks 3.5）：远程 / 反重甲 /
    /// 英雄可移动，人口地洞 / 祭坛 / 战争磨坊不可移动；判据来自冻结定义
    /// `speed_raw`（`Race::Orc.category_def_ids()` 已替换占位到真实常量），
    /// 不来自运行时 `speed` 或类型白名单。`Spawn` 字节不变（C2 / D4）。
    #[test]
    fn orc_category_movability_is_derived_from_speed_raw() {
        use crate::melee::defs::{
            CATEGORY_ALTAR, CATEGORY_FARM, CATEGORY_HERO, CATEGORY_RANGED, CATEGORY_SIEGE,
            CATEGORY_SMITHY,
        };

        let defs = crate::melee::defs::synthetic_definitions();
        let setup = MeleeSetup::with_race(Race::Orc);
        let map = setup.category_movability(&defs);
        assert_eq!(map.len(), 6, "reason=one_entry_per_orc_new_category");
        for slot in [CATEGORY_RANGED, CATEGORY_SIEGE, CATEGORY_HERO] {
            assert_eq!(
                map.get(&slot),
                Some(&true),
                "reason=movable_orc_slot slot={slot}"
            );
        }
        for slot in [CATEGORY_FARM, CATEGORY_ALTAR, CATEGORY_SMITHY] {
            assert_eq!(
                map.get(&slot),
                Some(&false),
                "reason=immovable_orc_building slot={slot}"
            );
        }
        // 装配层种族差异只停在只读绑定，不改变任何 `Spawn` 命令字节。
        let human = MeleeSetup::with_race(Race::Human);
        assert_eq!(
            setup.commands, human.commands,
            "reason=orc_spawn_bytes_unchanged"
        );
    }

    /// 可移动性判据由冻结定义 `speed_raw` 派生：农民可移动、建筑 / 资源点
    /// 不可移动；MUST NOT 用运行时 `speed`（`Builder::spawn` 强制 `max(1)`）。
    #[test]
    fn movability_is_derived_from_definition_speed_raw() {
        let defs = crate::melee::defs::synthetic_definitions();
        let setup = MeleeSetup::new();
        let map = setup.movability(&defs);
        assert_eq!(map.len(), 12, "reason=one_entry_per_opening_entity");
        // 本地 3 农民（ID 2,3,4）与 AI 3 农民（ID 8,9,10）可移动。
        for id in [2, 3, 4, 8, 9, 10] {
            assert_eq!(map.get(&id), Some(&true), "reason=peasant_movable id={id}");
        }
        // 双方主城（1,7）与资源点（5,6,11,12）不可移动。
        for id in [1, 5, 6, 7, 11, 12] {
            assert_eq!(map.get(&id), Some(&false), "reason=immovable id={id}");
        }
        // 运行时 Spawn 的 speed 被抬到 max(1)（> 0），但判据仍判主城不可移动，
        // 证明没有回退到运行时 `speed`。
        for command in &setup.commands {
            if let CommandKind::Spawn { speed, .. } = command.kind {
                assert!(speed.raw() > 0, "reason=runtime_speed_is_always_positive");
            }
        }
    }

    // -----------------------------------------------------------------------
    // P8.3 双族对抗（design D1 / D2 / tasks 1.x / 2.2 / 6.2 / 6.3）
    // -----------------------------------------------------------------------

    /// P9 / Q1' 十六组合（本地族, AI 族）：四族 4×4 笛卡尔积，含同族组合。
    const COMBINATIONS: [(Race, Race); 16] = [
        (Race::Human, Race::Human),
        (Race::Human, Race::Orc),
        (Race::Human, Race::Undead),
        (Race::Human, Race::NightElf),
        (Race::Orc, Race::Human),
        (Race::Orc, Race::Orc),
        (Race::Orc, Race::Undead),
        (Race::Orc, Race::NightElf),
        (Race::Undead, Race::Human),
        (Race::Undead, Race::Orc),
        (Race::Undead, Race::Undead),
        (Race::Undead, Race::NightElf),
        (Race::NightElf, Race::Human),
        (Race::NightElf, Race::Orc),
        (Race::NightElf, Race::Undead),
        (Race::NightElf, Race::NightElf),
    ];

    /// `DefId` 是否落在给定种族的 `units` 段（P9 五段中的四族段：
    /// 人族 1xx / 兽族 2xx / 不死 3xx / 暗夜 4xx）。
    fn in_race_segment(id: DefId, race: Race) -> bool {
        match race {
            Race::Human => (100..200).contains(&id),
            Race::Orc => (200..300).contains(&id),
            Race::Undead => (300..400).contains(&id),
            Race::NightElf => (400..500).contains(&id),
        }
    }

    /// 逐字段比对两份装配层数据是否逐字节一致（装配层只读，无权威状态）。
    fn setup_assembly_eq(a: &MeleeSetup, b: &MeleeSetup) -> bool {
        a.commands == b.commands
            && a.gameplay_commands == b.gameplay_commands
            && a.kinds == b.kinds
            && a.def_ids == b.def_ids
            && a.category_def_ids == b.category_def_ids
            && a.ai_category_def_ids == b.ai_category_def_ids
            && a.race == b.race
            && a.ai_race == b.ai_race
            && a.local_hall == b.local_hall
            && a.ai_hall == b.ai_hall
            && a.local_peasants == b.local_peasants
            && a.ai_peasants == b.ai_peasants
            && a.local_mine == b.local_mine
            && a.ai_mine == b.ai_mine
    }

    /// P9 / Q1' 十六组合装配：本地 / AI 席位段位正确（四族各段）、中立
    /// 资源恒 9xx、同族组合双方段位一致、十六组合 `Spawn` /
    /// `gameplay_commands` / `kinds` 字节一致（design D1 / C2 /
    /// `CommandKind::Spawn` 不含 DefId，字节逐字节不变）。
    #[test]
    fn with_races_assembles_sixteen_combinations_with_segmented_def_ids() {
        let baseline = MeleeSetup::new();
        for (local, ai) in COMBINATIONS {
            let setup = MeleeSetup::with_races(local, ai);
            assert_eq!(setup.race, local, "reason=local_race_alias local={local:?}");
            assert_eq!(setup.ai_race, ai, "reason=ai_race_field ai={ai:?}");
            assert_eq!(setup.commands.len(), 12, "reason=opening_spawn_count");
            assert_eq!(setup.def_ids.len(), 12, "reason=all_entities_have_def_id");

            // 本地席位主城（1）与农民（2,3,4）落本地族段。
            for id in [1u64, 2, 3, 4] {
                let def_id = *setup.def_ids.get(&id).expect("missing local def id");
                assert!(
                    in_race_segment(def_id, local),
                    "reason=local_seat_in_local_segment id={id} def={def_id} local={local:?}"
                );
            }
            // AI 席位主城（7）与农民（8,9,10）落 AI 族段。
            for id in [7u64, 8, 9, 10] {
                let def_id = *setup.def_ids.get(&id).expect("missing ai def id");
                assert!(
                    in_race_segment(def_id, ai),
                    "reason=ai_seat_in_ai_segment id={id} def={def_id} ai={ai:?}"
                );
            }
            // 中立资源（5,6,11,12）恒 9xx，与四族段都不冲突。
            for id in [5u64, 6, 11, 12] {
                let def_id = *setup.def_ids.get(&id).expect("missing neutral def id");
                assert!(
                    (900..1000).contains(&def_id),
                    "reason=neutral_resource_in_9xx id={id} def={def_id}"
                );
            }
            // 同族组合双方段位一致（本地 / AI 席位 def_ids 相等）。
            if local == ai {
                for (local_id, ai_id) in [(1u64, 7u64), (2, 8), (3, 9), (4, 10)] {
                    assert_eq!(
                        setup.def_ids.get(&local_id),
                        setup.def_ids.get(&ai_id),
                        "reason=same_race_seats_share_segment local={local:?}"
                    );
                }
            }
            // 十六组合权威命令字节与基线逐字节一致（种族只停装配层）。
            assert_eq!(
                setup.commands, baseline.commands,
                "reason=spawn_bytes_identical"
            );
            assert_eq!(
                setup.gameplay_commands, baseline.gameplay_commands,
                "reason=opening_gameplay_bytes_identical"
            );
            assert_eq!(setup.kinds, baseline.kinds, "reason=kinds_bytes_identical");
        }
    }

    /// `with_race(race)` 兼容别名严格等于 `with_races(race,
    /// race.opponent())`（P9 四族对合映射）；`new()` 仍为
    /// 「本地人族 / AI 兽族」逐字节不变（design D1 / C2 / Q1'）。
    #[test]
    fn with_race_alias_matches_dual_entry_and_new_keeps_default() {
        for race in [Race::Human, Race::Orc, Race::Undead, Race::NightElf] {
            let alias = MeleeSetup::with_race(race);
            let dual = MeleeSetup::with_races(race, race.opponent());
            assert!(
                setup_assembly_eq(&alias, &dual),
                "reason=with_race_alias_matches_dual race={race:?}"
            );
            // 回落基线证据：别名不改动 Spawn 字节 / kinds。
            assert_eq!(alias.commands, dual.commands, "reason=alias_spawn_bytes");
            assert_eq!(alias.kinds, dual.kinds, "reason=alias_kinds_bytes");
        }
        let default = MeleeSetup::new();
        assert_eq!(default.race, Race::Human, "reason=default_local_human");
        assert_eq!(default.ai_race, Race::Orc, "reason=default_ai_orc");
        assert!(
            setup_assembly_eq(&default, &MeleeSetup::with_races(Race::Human, Race::Orc)),
            "reason=new_equals_with_races_human_orc"
        );
    }

    /// 本地 / AI 两份 `category_def_ids` 区分：本地份落本地族段、AI 份落
    /// AI 族段；异族组合互不相交、同族组合两份相等；查表与
    /// `category_movability` 都取本地份（design D2 / tasks 2.2）。
    #[test]
    fn category_bindings_are_split_into_local_and_ai_copies() {
        let defs = crate::melee::defs::synthetic_definitions();
        for (local, ai) in COMBINATIONS {
            let setup = MeleeSetup::with_races(local, ai);
            let local_copy = setup.category_def_ids();
            let ai_copy = setup.ai_category_def_ids();
            assert_eq!(local_copy.len(), 6, "reason=six_local_categories");
            assert_eq!(ai_copy.len(), 6, "reason=six_ai_categories");
            assert!(
                local_copy.values().all(|&id| in_race_segment(id, local)),
                "reason=local_copy_in_local_segment local={local:?}"
            );
            assert!(
                ai_copy.values().all(|&id| in_race_segment(id, ai)),
                "reason=ai_copy_in_ai_segment ai={ai:?}"
            );
            // 兼容别名：`category_def_ids()` 仍是本地份。
            assert_eq!(setup.category_def_ids(), local_copy, "reason=local_alias");
            // 按种族查表：本地族命中本地份、AI 族命中 AI 份（异族时）。
            assert_eq!(
                setup.category_def_ids_for(local),
                local_copy,
                "reason=for_local_returns_local_copy"
            );
            if local != ai {
                assert_eq!(
                    setup.category_def_ids_for(ai),
                    ai_copy,
                    "reason=for_ai_returns_ai_copy"
                );
                let local_set: std::collections::BTreeSet<_> =
                    local_copy.values().copied().collect();
                let ai_set: std::collections::BTreeSet<_> = ai_copy.values().copied().collect();
                assert!(
                    local_set.is_disjoint(&ai_set),
                    "reason=distinct_race_copies_disjoint"
                );
            } else {
                assert_eq!(local_copy, ai_copy, "reason=same_race_copies_equal");
            }
            // 可移动性派生仍只读本地份（R1.4 / R13.6 分层判据沿用）。
            let map = setup.category_movability(&defs);
            assert_eq!(map.len(), 6, "reason=one_entry_per_local_category");
        }
    }

    /// 被反向验证的段位断言：AI 席位主城 / 农民的 `def_ids` 全部落在
    /// `ai_race` 段（四组合装配的正确性判据，design D1 / tasks 6.2）。
    fn ai_seat_def_ids_in_segment(setup: &MeleeSetup, ai_race: Race) -> bool {
        [
            setup.def_ids.get(&7),
            setup.def_ids.get(&8),
            setup.def_ids.get(&9),
            setup.def_ids.get(&10),
        ]
        .into_iter()
        .all(|slot| slot.is_some_and(|&id| in_race_segment(id, ai_race)))
    }

    /// 退化装配（缺陷态，tasks 6.2）：把 `with_races(local, ai)` 退化为
    /// 单参数 `opponent()` 语义——忽略 `ai`，恒取 `local.opponent()`。
    /// 仍经真实 [`MeleeSetup::with_races`] 装配路径，不是旁路模拟。
    fn degenerate_opponent_assembly(local: Race, _ai: Race) -> MeleeSetup {
        MeleeSetup::with_races(local, local.opponent())
    }

    /// 反向验证·修复态（tasks 6.2 / Q1'）：正式双参数装配十六组合全通过
    /// 段位断言（四族各段）。
    #[test]
    fn reverse_validation_dual_race_formal_passes_seat_assertion() {
        for (local, ai) in COMBINATIONS {
            let setup = MeleeSetup::with_races(local, ai);
            assert!(
                ai_seat_def_ids_in_segment(&setup, ai),
                "reason=formal_dual_race_passes local={local:?} ai={ai:?}"
            );
        }
    }

    /// 反向验证·缺陷态（tasks 6.2 / Q1'）：退化为单参数 `opponent()` 时，
    /// 四族四个同族组合（人vs人 / 兽vs兽 / 不死vs不死 / 暗夜vs暗夜）的
    /// AI 席位段位断言必失败；正式实现仍通过（元断言防退化）。
    /// 异族组合恰好不暴露该缺陷，正是同族组合判据的价值。
    #[test]
    fn reverse_validation_opponent_degenerate_fails_same_race_seat_assertion() {
        for (local, ai) in [
            (Race::Human, Race::Human),
            (Race::Orc, Race::Orc),
            (Race::Undead, Race::Undead),
            (Race::NightElf, Race::NightElf),
        ] {
            let degenerate = degenerate_opponent_assembly(local, ai);
            assert!(
                !ai_seat_def_ids_in_segment(&degenerate, ai),
                "reason=opponent_degenerate_fails local={local:?} ai={ai:?}"
            );
            let formal = MeleeSetup::with_races(local, ai);
            assert!(
                ai_seat_def_ids_in_segment(&formal, ai),
                "reason=formal_still_passes local={local:?} ai={ai:?}"
            );
        }
    }

    /// 被反向验证的回落基线断言：`with_race(race)` 候选与
    /// `with_races(race, race.opponent())` 的装配数据逐字段一致
    /// （`race_fallback_matches_human_segment_in_full_match` /
    /// `race_does_not_change_authoritative_spawn_commands` 的装配层等价）。
    fn alias_is_consistent(candidate: &MeleeSetup, race: Race) -> bool {
        setup_assembly_eq(candidate, &MeleeSetup::with_races(race, race.opponent()))
    }

    /// 反向验证·修复态（tasks 6.3 / Q1'）：`with_race` 兼容别名与双参数
    /// 入口一致（四族对合映射），既有回落基线 / Spawn 字节不变测试保持成立。
    #[test]
    fn reverse_validation_alias_formal_passes_fallback_assertion() {
        for race in [Race::Human, Race::Orc, Race::Undead, Race::NightElf] {
            assert!(
                alias_is_consistent(&MeleeSetup::with_race(race), race),
                "reason=formal_alias_consistent race={race:?}"
            );
        }
        // 既有回落基线证据：`new()` 与 `with_race(Human)` 逐字节一致。
        assert!(
            setup_assembly_eq(&MeleeSetup::new(), &MeleeSetup::with_race(Race::Human)),
            "reason=fallback_matches_human_segment"
        );
    }

    /// 反向验证·缺陷态（tasks 6.3）：删除别名 / 令别名不再等于
    /// `with_races(race, race.opponent())`（此处退化为 `with_races(race, race)`）
    /// 后回落基线断言必失败；正式别名仍通过（元断言）。退化候选经真实
    /// `with_races` 路径构造，非旁路模拟。
    #[test]
    fn reverse_validation_alias_degenerate_fails_fallback_assertion() {
        for race in [Race::Human, Race::Orc, Race::Undead, Race::NightElf] {
            let degenerate = MeleeSetup::with_races(race, race);
            assert!(
                !alias_is_consistent(&degenerate, race),
                "reason=alias_degenerate_fails race={race:?}"
            );
            assert!(
                alias_is_consistent(&MeleeSetup::with_race(race), race),
                "reason=formal_alias_still_passes race={race:?}"
            );
        }
    }
}
