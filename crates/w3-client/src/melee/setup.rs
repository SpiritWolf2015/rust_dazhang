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
    /// 农民与主城的坐标全部为编译期整数常量；主城相距 192 世界单位，处于
    /// 合成士兵的攻城射程内，使一局可在合理帧数内分出胜负。
    pub fn new() -> Self {
        let mut b = Builder::new();

        // 本地玩家（player 1）：ID 1..=6。
        let local_hall = b.spawn(LOCAL_PLAYER, -96, 0, 120, 0, 0, 0, EntityKind::TownHall);
        let local_peasants = vec![
            b.spawn(LOCAL_PLAYER, -80, -8, 60, 3, 4, 4, EntityKind::Peasant),
            b.spawn(LOCAL_PLAYER, -80, 8, 60, 3, 4, 4, EntityKind::Peasant),
            b.spawn(LOCAL_PLAYER, -72, 0, 60, 3, 4, 4, EntityKind::Peasant),
        ];
        let local_mine = b.spawn(LOCAL_PLAYER, -112, -24, 500, 0, 0, 0, EntityKind::GoldMine);
        b.spawn(LOCAL_PLAYER, -112, 24, 200, 0, 0, 0, EntityKind::Tree);

        // AI 对手（player 2）：ID 7..=12。
        let ai_hall = b.spawn(AI_PLAYER, 96, 0, 120, 0, 0, 0, EntityKind::TownHall);
        let ai_peasants = vec![
            b.spawn(AI_PLAYER, 80, -8, 60, 3, 4, 4, EntityKind::Peasant),
            b.spawn(AI_PLAYER, 80, 8, 60, 3, 4, 4, EntityKind::Peasant),
            b.spawn(AI_PLAYER, 72, 0, 60, 3, 4, 4, EntityKind::Peasant),
        ];
        let ai_mine = b.spawn(AI_PLAYER, 112, -24, 500, 0, 0, 0, EntityKind::GoldMine);
        b.spawn(AI_PLAYER, 112, 24, 200, 0, 0, 0, EntityKind::Tree);

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
}
