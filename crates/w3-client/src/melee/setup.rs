//! 开局布置（design D1 / D2）：静态世界、双方实体的 `Spawn` 脚本与表现层类型标签。
//!
//! 全部坐标由 `SimFixed::from_int` 从整数常量冻结；实体 ID 按
//! `ExecuteFrame → PlayerId → Sequence → Arrival` 稳定顺序分配，因此脚本
//! 可以在装配期确定每个实体的 `EntityId`，供 AI 命令源与呈现层引用。

use std::collections::BTreeMap;

use w3_core::{
    Command, CommandKind, EntityId, Frame, GameplayCommand, GameplayCommandKind, SimFixed,
    StaticWorld,
};

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
    next_id: u64,
    next_sequence: [u32; 3],
}

impl Builder {
    fn new() -> Self {
        Self {
            commands: Vec::new(),
            kinds: BTreeMap::new(),
            next_id: 1,
            next_sequence: [0; 3],
        }
    }

    /// 追加一条 `Spawn`；ID 按构造顺序分配，与内核执行顺序一致。
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
        EntityId::new(id).expect("melee setup: reason=invalid_entity_id")
    }
}

impl MeleeSetup {
    /// 构造对称开局：双方各一座主城、三名农民、一处金矿与一片树林。
    ///
    /// 主城落在真实 Echo Isles 的两个 sloc（见 [`LOCAL_START`] / [`AI_START`]），
    /// 农民朝地图中心方向聚在主城前方、金矿与树林在侧后方，全部落在各自陆块
    /// 的地面上。主城相距约 155 世界单位，处于合成士兵经移动后的攻城射程内，
    /// 使一局可在合理帧数内分出胜负。坐标全为编译期整数常量。
    pub fn new() -> Self {
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
        );
        b.spawn(AI_PLAYER, ai[5].0, ai[5].1, 200, 0, 0, 0, EntityKind::Tree);

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
            local_hall,
            ai_hall,
            local_peasants,
            ai_peasants,
            local_mine,
            ai_mine,
        }
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
}
