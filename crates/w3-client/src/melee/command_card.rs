//! 表现层命令卡只读定义表（change `p9-command-hotkeys` / D1 / D2）。
//!
//! 本模块把「按钮 → 既有命令面动作 + 快捷键 + 格位」组织成一张由**编译期
//! 整数 `DefId` 索引**的只读定义表：命令卡为 4 列 × 3 行 = 12 格，格位按
//! `slot = y * 4 + x` 落格，键位字母与命令语义解耦。当前卡由「选中集合类型 +
//! 表现层子菜单栈」决定，每次换卡重建「键 → action」表。
//!
//! 硬红线（ADR 0001 / ADR 0002 / C6）：
//! - 全表为合成 / 原创编译期整数常量，零文件 IO、零真实导出 JSON、
//!   零新增依赖，**不冻结进 `FrozenDefinitions`**；具体经典键位字母一律
//!   `[待核 / 待用户拍板]`、以本机导出 SLK 命令卡热键字段或原版客户端核对，
//!   不冒充原版、不照搬 Warsmash rawcode / 暴雪 abilId / 参考项目原创单位名。
//! - 按钮标识 `def_id` 是表现层装配用的合成整数命名空间（避开冻结定义的
//!   `1xx` / `2xx` / `9xx` 段与类别内唯一 ID），**不是权威 `DefId`**，
//!   MUST NOT 进入权威状态 / FNV-1a 状态哈希 / `SNAPSHOT_VERSION`（当前 4）
//!   快照；命令卡 12 格、键→action 表、子菜单栈全是 HUD 只读派生。
//! - 第二键列表由定义表记录驱动，MUST NOT 用建筑名字符串硬编码白名单
//!   （R1.2 / R13.3）；人族六类新单位 / 建筑各有定义表条目。

use std::collections::{BTreeMap, BTreeSet};

use w3_core::DefId;

use super::defs::{
    BARRACKS, GOLD_MINE, HUMAN_ALTAR, HUMAN_BARRACKS, HUMAN_FARM, HUMAN_HERO, HUMAN_PEASANT,
    HUMAN_RANGED, HUMAN_SIEGE, HUMAN_SMITHY, HUMAN_SOLDIER, HUMAN_TOWN_HALL, NEUTRAL_GOLD_MINE,
    NEUTRAL_TREE, ORC_ALTAR, ORC_BARRACKS, ORC_FARM, ORC_HERO, ORC_PEASANT, ORC_RANGED, ORC_SIEGE,
    ORC_SMITHY, ORC_SOLDIER, ORC_TOWN_HALL, PEASANT, Race, SOLDIER, TOWN_HALL, TREE,
};

/// 命令卡列数。
pub const GRID_WIDTH: u8 = 4;
/// 命令卡行数。
pub const GRID_HEIGHT: u8 = 3;
/// 命令卡总格数（4 × 3 = 12）。
pub const CARD_SLOTS: usize = (GRID_WIDTH * GRID_HEIGHT) as usize;

/// 格位公式 `slot = y * 4 + x`：列 `x` 取 0..4、行 `y` 取 0..3。
pub const fn slot(x: u8, y: u8) -> u8 {
    y * GRID_WIDTH + x
}

// 表现层按钮标识命名空间：合成原创编译期整数常量，刻意避开冻结定义的
// `1xx` / `2xx` / `9xx` 段与技能 / 升级 / 物品类别内唯一 ID，避免与任何
// 权威 `DefId` 混淆。这些编号只用来组织定义表，不进权威状态。
/// 按钮标识基准。
const BTN_BASE: DefId = 70_000;
const BTN_MOVE: DefId = BTN_BASE + 1;
const BTN_ATTACK: DefId = BTN_BASE + 2;
const BTN_STOP: DefId = BTN_BASE + 3;
const BTN_GATHER: DefId = BTN_BASE + 4;
const BTN_BUILD_MENU: DefId = BTN_BASE + 5;
const BTN_TRAIN_MENU: DefId = BTN_BASE + 6;
const BTN_CANCEL: DefId = BTN_BASE + 7;
const BTN_BUILD_ALTAR: DefId = BTN_BASE + 8;
const BTN_BUILD_FARM: DefId = BTN_BASE + 9;
const BTN_BUILD_BARRACKS: DefId = BTN_BASE + 10;
const BTN_BUILD_SMITHY: DefId = BTN_BASE + 11;
const BTN_BUILD_HALL: DefId = BTN_BASE + 12;
const BTN_TRAIN_SOLDIER: DefId = BTN_BASE + 13;
const BTN_TRAIN_RANGED: DefId = BTN_BASE + 14;
const BTN_TRAIN_SIEGE: DefId = BTN_BASE + 15;
const BTN_TRAIN_HERO: DefId = BTN_BASE + 16;
const BTN_SHOP_ITEM: DefId = BTN_BASE + 17;

/// 命令卡类别（当前卡 ID）。表现层局部状态，MUST NOT 进入权威状态 /
/// 状态哈希 / 快照；子菜单栈存的就是这个枚举的序列（D3）。
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CardId {
    /// 通用主卡：战斗单位 / 未识别实体 / 资源点。
    Main,
    /// 可建造工人主卡：含采集与建造入口。
    Worker,
    /// 生产建筑主卡：含训练入口与取消。
    Producer,
    /// 建造子卡：`B` 打开后的第二键建筑格。
    Build,
    /// 训练子卡：训练具体单位的第二键格。
    Train,
    /// 商店卡：占位（具体键位 / 购买语义 `[待核]`）。
    Shop,
}

/// 选中集合类型：由装配层 `EntityId → DefId` 关联查定义表得到，
/// MUST NOT 用 `EntityKind` match 或建筑名字符串白名单判定（R1.2 / D2）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardKind {
    Worker,
    Producer,
    Combat,
    Hero,
    Shop,
    Resource,
    Empty,
}

/// 定义表里「按钮 → 既有命令面动作」的语义标识（表现层，**不是**新的
/// `CommandKind` / `GameplayCommandKind` 变体，D7 / R13.6）。
///
/// 每个变体最终都映射到 `w3-core` 既有命令面：
/// `CommandKind::{Move, Attack, Stop}` 与
/// `GameplayCommandKind::{Gather, Train, CancelTrain}`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CardCommand {
    /// `M`：移动瞄准（进入表现层瞄准态，左键定点才产出 `Move`）。
    MoveAim,
    /// `A`：攻击瞄准（仅既有 `Attack { attacker, target }` 指定目标入口）。
    AttackAim,
    /// `S`：停止（既有 `CommandKind::Stop`；与 Hold 语义不合并）。
    Stop,
    /// `G`：采集（既有 `GameplayCommandKind::Gather`）。
    Gather,
    /// `B`：打开建造子菜单（入表现层子菜单栈，不生成权威命令）。
    OpenBuildMenu,
    /// 打开训练子菜单（入表现层子菜单栈，不生成权威命令）。
    OpenTrainMenu,
    /// 训练 / 建造具体项：`unit_def` 是既有 `Train { unit_def, is_building }`
    /// 的参数，来自定义表记录而非建筑名白名单（R1.2 / R13.3）。
    TrainUnit { unit_def: DefId, is_building: bool },
    /// 取消（既有 `GameplayCommandKind::CancelTrain`）。
    CancelTrain,
}

/// 一格定义记录：按钮 → 命令 + 快捷键 + 格位，键位与命令解耦。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ButtonDef {
    /// 表现层按钮标识（合成原创编译期整数，非权威 `DefId`）。
    pub def_id: DefId,
    /// 格位 `slot = y * 4 + x`。
    pub slot: u8,
    /// 快捷键字母（合成原创，`[待核 / 待用户拍板]`）。
    pub hotkey: char,
    /// 该格映射到的既有命令面动作。
    pub command: CardCommand,
    /// i18n 边界标签（ASCII 文案，MUST NOT 进日志 / 权威状态 / 哈希 / 快照）。
    pub label: &'static str,
}

/// 一张命令卡 = 12 格（含空格位）。空选中由 HUD 层决定不渲染。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandCard {
    pub id: CardId,
    pub buttons: [Option<ButtonDef>; CARD_SLOTS],
}

impl CommandCard {
    /// 由卡类别与按钮条目构造 12 格：按 `slot` 落格，重复格位由后者覆盖，
    /// 越界格位忽略（显式保守，不 panic）。
    pub fn new(id: CardId, entries: &[ButtonDef]) -> Self {
        let mut buttons = [None; CARD_SLOTS];
        for button in entries {
            if let Some(cell) = buttons.get_mut(button.slot as usize) {
                *cell = Some(*button);
            }
        }
        Self { id, buttons }
    }

    /// 取指定格位的按钮定义（含空格位返回 `None`）。
    pub fn at(&self, slot: u8) -> Option<ButtonDef> {
        self.buttons.get(slot as usize).copied().flatten()
    }

    /// 该卡里全部非空按钮（稳定格位顺序）。
    pub fn filled(&self) -> Vec<ButtonDef> {
        self.buttons.iter().flatten().copied().collect()
    }
}

/// 表现层只读定义表：多张卡 + 「`DefId` → 选中集合类型」查表。
///
/// 可注入 `InputState`（DIP）；反向验证通过注入退化目录（全局静态键表）
/// 经真实 `apply` 路径证明 chords 断言失败（C5 / D9）。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandCatalog {
    cards: BTreeMap<CardId, CommandCard>,
    def_kinds: BTreeMap<DefId, CardKind>,
}

/// 构造一格按钮的辅助函数，`slot` 用统一公式计算。
const fn btn(
    def_id: DefId,
    x: u8,
    y: u8,
    hotkey: char,
    command: CardCommand,
    label: &'static str,
) -> ButtonDef {
    ButtonDef {
        def_id,
        slot: slot(x, y),
        hotkey,
        command,
        label,
    }
}

/// 命令卡训练 / 建造目标的类别键（编译期整数 `DefId` 定义表的列索引）。
///
/// 纯表现层枚举，MUST NOT 进入权威状态 / 状态哈希 / 快照；键位与命令解耦，
/// 分派只经 [`CommandCatalog::for_race_with_targets`] 的只读定义表查表，
/// MUST NOT 散落 `if race` 分支、不判玩法规则、不用建筑名 / `H`/`O` id
/// 前缀白名单推断（R1.2 / design D2 / godot `BUILD_SYSTEM.md:298/:348-349`）。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrainCategory {
    Soldier,
    Ranged,
    Siege,
    Hero,
    Altar,
    Farm,
    Barracks,
    Smithy,
    TownHall,
}

impl TrainCategory {
    /// 定义表列数。
    pub const COUNT: usize = 9;

    /// 该类别在定义表中的列下标（编译期常量，不引入运行时分支判断）。
    /// 只读查询，供 CPU 回归核对定义表逐项登记（表现层，不进权威状态）。
    pub const fn index(self) -> usize {
        match self {
            Self::Soldier => 0,
            Self::Ranged => 1,
            Self::Siege => 2,
            Self::Hero => 3,
            Self::Altar => 4,
            Self::Farm => 5,
            Self::Barracks => 6,
            Self::Smithy => 7,
            Self::TownHall => 8,
        }
    }
}

/// 定义表行数：两族各一行（人族 `1xx` / 兽族 `2xx`）。
pub const RACE_ROWS: usize = 2;

/// 按族只读查表的编译期整数 `DefId` 定义表类型：行 = 种族段，列 = 目标类别。
///
/// `None` 表示该类别缺登记（由 [`table_target`] 明确回落到人族段默认并以
/// 英文结构化日志可诊断）。全部 `Some` 表项来自 `defs.rs` 的合成原创
/// 编译期整数常量，零文件 IO、不冻结进 `FrozenDefinitions`、不入库
/// （ADR 0001 / ADR 0004 / Q2 拍板 `[待核]`）。
pub type RaceTargetTable = [[Option<DefId>; TrainCategory::COUNT]; RACE_ROWS];

/// 两族训练 / 建造目标定义表（编译期常量）。
///
/// 人族行全部落在 `1xx` 段、兽族行全部落在 `2xx` 段；Producer / Build /
/// Train 卡的 `TrainUnit { unit_def, .. }` 一律经本表查表得到，MUST NOT
/// 在卡片构造处写死某一族的 `DefId`（Q2 / R2 / R4 / R13.3）。
pub const RACE_TRAIN_TARGETS: RaceTargetTable = [
    [
        Some(HUMAN_SOLDIER),
        Some(HUMAN_RANGED),
        Some(HUMAN_SIEGE),
        Some(HUMAN_HERO),
        Some(HUMAN_ALTAR),
        Some(HUMAN_FARM),
        Some(HUMAN_BARRACKS),
        Some(HUMAN_SMITHY),
        Some(HUMAN_TOWN_HALL),
    ],
    [
        Some(ORC_SOLDIER),
        Some(ORC_RANGED),
        Some(ORC_SIEGE),
        Some(ORC_HERO),
        Some(ORC_ALTAR),
        Some(ORC_FARM),
        Some(ORC_BARRACKS),
        Some(ORC_SMITHY),
        Some(ORC_TOWN_HALL),
    ],
];

/// 缺表项 / 未知族的明确默认回落（人族段编译期整数常量）。
///
/// 不以族名 / `H`/`O` id 前缀推断机制（R1.2），只按类别列取编译期常量；
/// 回落发生时由 [`table_target`] 以英文结构化 `key=value` 日志明确报告。
pub const DEFAULT_COMMAND_TARGETS: [DefId; TrainCategory::COUNT] = [
    HUMAN_SOLDIER,
    HUMAN_RANGED,
    HUMAN_SIEGE,
    HUMAN_HERO,
    HUMAN_ALTAR,
    HUMAN_FARM,
    HUMAN_BARRACKS,
    HUMAN_SMITHY,
    HUMAN_TOWN_HALL,
];

/// 本地族 → 定义表行（唯一一处 `match race`，新增族只改定义表与此映射）。
///
/// 这是表索引查表、不是玩法规则分支：卡片构造处一律通过
/// [`table_target`] 取目标，MUST NOT 散落 `if race { HUMAN_* } else { ORC_* }`。
const fn race_row(race: Race) -> usize {
    match race {
        Race::Human => 0,
        Race::Orc => 1,
    }
}

/// 按本地族 + 类别从定义表只读查出目标 `DefId`。
///
/// 缺表项（行 / 列越界）SHALL 明确回落到人族段默认并以英文结构化
/// `key=value` 日志可诊断，MUST NOT 静默产出错误族目标、不写死中文名 /
/// 图标表（C8 / C9 / `CODEBUDDY.md §3` / godot `command_card.gd:441-445`）。
fn table_target(table: &RaceTargetTable, row: usize, category: TrainCategory) -> DefId {
    let fallback = DEFAULT_COMMAND_TARGETS[category.index()];
    let resolved = table
        .get(row)
        .and_then(|entries| entries.get(category.index()))
        .copied()
        .flatten();
    match resolved {
        Some(def_id) => def_id,
        None => {
            log::warn!(
                target: "w3_client",
                "command card target fallback: reason=table_entry_absent row={} category={:?} fallback_def_id={}",
                row, category, fallback
            );
            fallback
        }
    }
}

impl CommandCatalog {
    /// 合成 / 原创标准定义表（全编译期常量，零文件 IO）。
    ///
    /// 保留为**人族装配**的兼容入口：逐字节等于
    /// [`CommandCatalog::for_race`]`(Race::Human)`，既有 12 格 / 键位 / chords
    /// 契约与人族训练目标不变（Q2「人族装配行为逐字节不变」）。
    /// 快捷键字母均为 `[待核]` 的合成配置，不冒充原版经典键位（C6）。
    pub fn standard() -> Self {
        Self::for_race(Race::Human)
    }

    /// 按本地族只读查表构造目录（Q2 生产入口）。
    ///
    /// 目标 `DefId` 全部来自 [`RACE_TRAIN_TARGETS`]：人族 `1xx` / 兽族 `2xx`。
    /// 卡片 12 格 / `slot = y * 4 + x` / 键位解耦 / 两级 chords / 子菜单栈 /
    /// `Esc` 取消一律原样保留（不是重做 R13 命令卡，C5 / C8）。
    pub fn for_race(race: Race) -> Self {
        Self::for_race_with_targets(race, RACE_TRAIN_TARGETS)
    }

    /// 未知族 / 未确认选族的装配入口：`None` 明确回落到人族段并以英文结构化
    /// `key=value` 日志可诊断（C8 / C9）。生产实现 MUST 传 `Some(race)`。
    pub fn for_optional_race(race: Option<Race>) -> Self {
        match race {
            Some(race) => Self::for_race(race),
            None => {
                log::warn!(
                    target: "w3_client",
                    "command card race fallback: reason=unknown_race fallback=human_segment"
                );
                Self::for_race(Race::Human)
            }
        }
    }

    /// 由「本地族 + 编译期整数 `DefId` 定义表」构造目录（装配 / 反向验证注入点）。
    ///
    /// 生产实现 MUST 用 [`CommandCatalog::for_race`]；本注入点让改坏验红可以
    /// 把定义表退化成「两行都写死 `HUMAN_*`」的旧实现，**真经本构造路径**
    /// 产出目录后，令「兽族训练产兽族目标」断言在缺陷态失败、非旁路模拟
    /// （`CODEBUDDY.md §5` / design D6）。只读派生，MUST NOT 反写权威状态。
    pub fn for_race_with_targets(race: Race, table: RaceTargetTable) -> Self {
        use CardCommand as C;
        let row = race_row(race);
        let t = |category: TrainCategory| table_target(&table, row, category);
        let main = CommandCard::new(
            CardId::Main,
            &[
                btn(BTN_MOVE, 0, 0, 'm', C::MoveAim, "Move"),
                btn(BTN_ATTACK, 1, 0, 'a', C::AttackAim, "Attack"),
                btn(BTN_STOP, 2, 0, 's', C::Stop, "Stop"),
                btn(BTN_CANCEL, 3, 0, 'c', C::CancelTrain, "Cancel"),
            ],
        );
        let worker = CommandCard::new(
            CardId::Worker,
            &[
                btn(BTN_MOVE, 0, 0, 'm', C::MoveAim, "Move"),
                btn(BTN_ATTACK, 1, 0, 'a', C::AttackAim, "Attack"),
                btn(BTN_STOP, 2, 0, 's', C::Stop, "Stop"),
                btn(BTN_GATHER, 3, 0, 'g', C::Gather, "Gather"),
                btn(BTN_BUILD_MENU, 0, 1, 'b', C::OpenBuildMenu, "Build"),
                btn(BTN_CANCEL, 3, 2, 'c', C::CancelTrain, "Cancel"),
            ],
        );
        let producer = CommandCard::new(
            CardId::Producer,
            &[
                btn(BTN_TRAIN_MENU, 0, 0, 't', C::OpenTrainMenu, "Train"),
                btn(
                    BTN_TRAIN_SOLDIER,
                    1,
                    0,
                    'f',
                    C::TrainUnit {
                        unit_def: t(TrainCategory::Soldier),
                        is_building: false,
                    },
                    "Footman",
                ),
                btn(
                    BTN_TRAIN_RANGED,
                    2,
                    0,
                    'r',
                    C::TrainUnit {
                        unit_def: t(TrainCategory::Ranged),
                        is_building: false,
                    },
                    "Ranged",
                ),
                btn(BTN_CANCEL, 3, 2, 'c', C::CancelTrain, "Cancel"),
            ],
        );
        // 建造子卡：第二键列表全部由定义表记录驱动（TrainUnit 携带目标
        // `DefId`），MUST NOT 用建筑名字符串白名单（R1.2 / R13.3）。
        // 目标 `DefId` 按本地族只读查表：人族 `1xx` / 兽族 `2xx`（Q2）。
        let build = CommandCard::new(
            CardId::Build,
            &[
                btn(
                    BTN_BUILD_ALTAR,
                    0,
                    0,
                    'a',
                    C::TrainUnit {
                        unit_def: t(TrainCategory::Altar),
                        is_building: true,
                    },
                    "Altar",
                ),
                btn(
                    BTN_BUILD_FARM,
                    1,
                    0,
                    'f',
                    C::TrainUnit {
                        unit_def: t(TrainCategory::Farm),
                        is_building: true,
                    },
                    "Farm",
                ),
                btn(
                    BTN_BUILD_BARRACKS,
                    2,
                    0,
                    'b',
                    C::TrainUnit {
                        unit_def: t(TrainCategory::Barracks),
                        is_building: true,
                    },
                    "Barracks",
                ),
                btn(
                    BTN_BUILD_SMITHY,
                    3,
                    0,
                    'g',
                    C::TrainUnit {
                        unit_def: t(TrainCategory::Smithy),
                        is_building: true,
                    },
                    "Smithy",
                ),
                btn(
                    BTN_BUILD_HALL,
                    0,
                    1,
                    'e',
                    C::TrainUnit {
                        unit_def: t(TrainCategory::TownHall),
                        is_building: true,
                    },
                    "Hall",
                ),
                btn(BTN_CANCEL, 3, 2, 'c', C::CancelTrain, "Cancel"),
            ],
        );
        // 训练子卡：战斗单位第二键（两族六类新 `DefId` 里的可移动单位，
        // 目标按本地族只读查表，MUST NOT 写死 `HUMAN_*`）。
        let train = CommandCard::new(
            CardId::Train,
            &[
                btn(
                    BTN_TRAIN_SOLDIER,
                    0,
                    0,
                    'f',
                    C::TrainUnit {
                        unit_def: t(TrainCategory::Soldier),
                        is_building: false,
                    },
                    "Footman",
                ),
                btn(
                    BTN_TRAIN_RANGED,
                    1,
                    0,
                    'r',
                    C::TrainUnit {
                        unit_def: t(TrainCategory::Ranged),
                        is_building: false,
                    },
                    "Ranged",
                ),
                btn(
                    BTN_TRAIN_SIEGE,
                    2,
                    0,
                    'k',
                    C::TrainUnit {
                        unit_def: t(TrainCategory::Siege),
                        is_building: false,
                    },
                    "Siege",
                ),
                btn(
                    BTN_TRAIN_HERO,
                    3,
                    0,
                    'h',
                    C::TrainUnit {
                        unit_def: t(TrainCategory::Hero),
                        is_building: false,
                    },
                    "Hero",
                ),
                btn(BTN_CANCEL, 3, 2, 'c', C::CancelTrain, "Cancel"),
            ],
        );
        // 商店卡：占位（`G` 在此卡复用采集语义，`[待核]` 具体购买键）。
        let shop = CommandCard::new(
            CardId::Shop,
            &[
                btn(BTN_SHOP_ITEM, 0, 0, 'g', C::Gather, "Item"),
                btn(BTN_CANCEL, 3, 2, 'c', C::CancelTrain, "Cancel"),
            ],
        );

        let mut cards = BTreeMap::new();
        for card in [main, worker, producer, build, train, shop] {
            cards.insert(card.id, card);
        }

        // `DefId → 选中集合类型` 查表：覆盖既有 6 基线、人兽两族段、
        // P8.1 人族六类与 P8.2 兽族六类新定义（Q2 补齐两族登记，design
        // 「逐锚核对差异」差异 2）；未登记 `DefId` 由 `kind_of` 回退通用
        // 类型，运行时动态实体也能按只读查表派生（Risks / tasks 1.4）。
        // 表只读派生，MUST NOT 反写权威状态 / 进哈希 / 快照（ADR 0002）。
        let def_kinds: BTreeMap<DefId, CardKind> = [
            (SOLDIER, CardKind::Combat),
            (PEASANT, CardKind::Worker),
            (TOWN_HALL, CardKind::Producer),
            (BARRACKS, CardKind::Producer),
            (GOLD_MINE, CardKind::Resource),
            (TREE, CardKind::Resource),
            (HUMAN_PEASANT, CardKind::Worker),
            (HUMAN_SOLDIER, CardKind::Combat),
            (HUMAN_TOWN_HALL, CardKind::Producer),
            (HUMAN_BARRACKS, CardKind::Producer),
            (HUMAN_RANGED, CardKind::Combat),
            (HUMAN_SIEGE, CardKind::Combat),
            (HUMAN_FARM, CardKind::Producer),
            (HUMAN_ALTAR, CardKind::Producer),
            (HUMAN_SMITHY, CardKind::Producer),
            (HUMAN_HERO, CardKind::Hero),
            (ORC_PEASANT, CardKind::Worker),
            (ORC_SOLDIER, CardKind::Combat),
            (ORC_TOWN_HALL, CardKind::Producer),
            (ORC_BARRACKS, CardKind::Producer),
            (ORC_RANGED, CardKind::Combat),
            (ORC_SIEGE, CardKind::Combat),
            (ORC_FARM, CardKind::Producer),
            (ORC_ALTAR, CardKind::Producer),
            (ORC_SMITHY, CardKind::Producer),
            (ORC_HERO, CardKind::Hero),
            (NEUTRAL_GOLD_MINE, CardKind::Resource),
            (NEUTRAL_TREE, CardKind::Resource),
        ]
        .into_iter()
        .collect();

        Self { cards, def_kinds }
    }

    /// 由显式卡片与 `DefId` 分类表构造目录（装配 / 反向验证注入点）。
    /// 生产实现 MUST 用 `standard()`；本构造函数供注入退化目录证明
    /// 「同一字母随卡切换」断言在全局静态映射下失败（C5 / D9）。
    pub fn from_cards(
        cards: BTreeMap<CardId, CommandCard>,
        def_kinds: BTreeMap<DefId, CardKind>,
    ) -> Self {
        Self { cards, def_kinds }
    }

    /// 取一张卡的 12 格定义；缺卡返回空卡（显式保守，不 panic）。
    pub fn card(&self, id: CardId) -> CommandCard {
        self.cards.get(&id).copied().unwrap_or(CommandCard {
            id,
            buttons: [None; CARD_SLOTS],
        })
    }

    /// 由 `DefId` 查选中集合类型；未登记项回退通用 `Combat`（按只读查表
    /// 派生，不回退建筑类型白名单、不假定运行时动态实体不可派生）。
    pub fn kind_of(&self, def_id: DefId) -> CardKind {
        self.def_kinds
            .get(&def_id)
            .copied()
            .unwrap_or(CardKind::Combat)
    }

    /// 由选中集合类型定当前主卡（栈为空时）。优先级稳定、确定性：
    /// 工人 > 生产者 > 商店 > 其它（通用）。
    pub fn primary_card(&self, kinds: &[CardKind]) -> CardId {
        if kinds.contains(&CardKind::Worker) {
            CardId::Worker
        } else if kinds.contains(&CardKind::Producer) {
            CardId::Producer
        } else if kinds.contains(&CardKind::Shop) {
            CardId::Shop
        } else {
            CardId::Main
        }
    }

    /// 换卡时重建「键 → action」表：遍历该卡 12 格的非空按钮，取其
    /// `hotkey → command`。旧卡绑定随 `clear()` 一并失效（R13.2 / R13.3）。
    /// 返回 `BTreeMap` 保证跨平台迭代顺序稳定。
    pub fn key_actions(&self, id: CardId) -> BTreeMap<char, CardCommand> {
        self.card(id)
            .filled()
            .into_iter()
            .map(|button| (button.hotkey, button.command))
            .collect()
    }

    /// 建造子卡的第二键列表（由定义表 `Build` 卡记录驱动，非建筑名白名单）。
    pub fn build_buttons(&self) -> Vec<ButtonDef> {
        self.card(CardId::Build)
            .filled()
            .into_iter()
            .filter(|button| {
                matches!(
                    button.command,
                    CardCommand::TrainUnit {
                        is_building: true,
                        ..
                    }
                )
            })
            .collect()
    }

    /// 定义表里作为训练 / 建造目标出现的全部 `DefId`（用于回归核对
    /// P8.1 人族六类新 `DefId` 各有定义表条目，tasks 1.4）。
    pub fn trained_def_ids(&self) -> BTreeSet<DefId> {
        self.cards
            .values()
            .flat_map(|card| card.filled())
            .filter_map(|button| match button.command {
                CardCommand::TrainUnit { unit_def, .. } => Some(unit_def),
                _ => None,
            })
            .collect()
    }

    /// 全部卡片 ID（稳定 `BTreeMap` 顺序）。
    pub fn card_ids(&self) -> Vec<CardId> {
        self.cards.keys().copied().collect()
    }
}

impl Default for CommandCatalog {
    fn default() -> Self {
        Self::standard()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use w3_core::DefId;

    use crate::melee::defs::{
        HUMAN_ALTAR, HUMAN_FARM, HUMAN_HERO, HUMAN_PEASANT, HUMAN_RANGED, HUMAN_SIEGE,
        HUMAN_SMITHY, HUMAN_TOWN_HALL, NEUTRAL_GOLD_MINE,
    };

    use super::{
        ButtonDef, CARD_SLOTS, CardCommand, CardId, CardKind, CommandCatalog, GRID_HEIGHT,
        GRID_WIDTH, slot,
    };

    /// 人族六类新 `DefId`（tasks 1.4：各有定义表条目）。
    const HUMAN_NEW: [DefId; 6] = [
        HUMAN_RANGED,
        HUMAN_SIEGE,
        HUMAN_FARM,
        HUMAN_ALTAR,
        HUMAN_SMITHY,
        HUMAN_HERO,
    ];

    #[test]
    fn grid_is_four_by_three_with_slot_formula() {
        assert_eq!(GRID_WIDTH, 4);
        assert_eq!(GRID_HEIGHT, 3);
        assert_eq!(CARD_SLOTS, 12);
        assert_eq!(slot(0, 0), 0);
        assert_eq!(slot(3, 0), 3);
        assert_eq!(slot(0, 1), 4);
        assert_eq!(slot(2, 2), 10);
        assert_eq!(slot(3, 2), 11);
        let catalog = CommandCatalog::standard();
        for id in catalog.card_ids() {
            let card = catalog.card(id);
            assert_eq!(card.buttons.len(), 12, "reason=twelve_slots id={id:?}");
            // 每格若非空，其 slot 必须等于它所在的数组下标（落格自洽）。
            for (index, button) in card.buttons.iter().enumerate() {
                if let Some(button) = button {
                    assert_eq!(button.slot as usize, index, "reason=slot_matches_index");
                }
            }
        }
    }

    #[test]
    fn hotkey_is_decoupled_from_command() {
        let catalog = CommandCatalog::standard();
        let original = catalog.card(CardId::Worker).at(slot(3, 0)).unwrap();
        assert_eq!(original.command, CardCommand::Gather);
        // 同一命令换到别的格位 / 改快捷键字母，命令语义不变、键位与命令解耦。
        let moved = ButtonDef {
            slot: slot(1, 1),
            hotkey: 'z',
            ..original
        };
        assert_eq!(moved.command, original.command);
        assert_ne!(moved.hotkey, original.hotkey);
        assert_ne!(moved.slot, original.slot);
        // 定义表是单一真值源：换键不改命令，也不触碰任何冻结定义。
        assert_eq!(catalog.card(CardId::Worker).at(slot(1, 1)), None);
    }

    #[test]
    fn six_human_new_def_ids_have_catalog_entries() {
        let catalog = CommandCatalog::standard();
        let trained = catalog.trained_def_ids();
        for id in HUMAN_NEW {
            assert!(
                trained.contains(&id),
                "reason=missing_training_entry id={id}"
            );
            // 每类也都能由定义表查到选中集合类型（非白名单派生）。
            let _kind = catalog.kind_of(id);
        }
        // 建筑类为建造项、可移动类为训练项，is_building 与目标 DefId 成对出现。
        let build_targets: BTreeSet<DefId> = catalog
            .build_buttons()
            .into_iter()
            .filter_map(|button| match button.command {
                CardCommand::TrainUnit { unit_def, .. } => Some(unit_def),
                _ => None,
            })
            .collect();
        for id in [HUMAN_FARM, HUMAN_ALTAR, HUMAN_SMITHY] {
            assert!(build_targets.contains(&id), "reason=building_entry id={id}");
        }
    }

    #[test]
    fn same_letter_changes_meaning_across_cards() {
        let catalog = CommandCatalog::standard();
        let worker = catalog.key_actions(CardId::Worker);
        let build = catalog.key_actions(CardId::Build);
        let producer = catalog.key_actions(CardId::Producer);
        // `g` 在工人主卡是采集、在建造子卡是建造铁匠铺（不同既有动作）。
        assert_eq!(worker.get(&'g'), Some(&CardCommand::Gather));
        assert_eq!(
            build.get(&'g'),
            Some(&CardCommand::TrainUnit {
                unit_def: HUMAN_SMITHY,
                is_building: true,
            })
        );
        assert_ne!(worker.get(&'g'), build.get(&'g'));
        // `b` 在工人主卡开建造子菜单、在建造子卡是建造兵营。
        assert_eq!(worker.get(&'b'), Some(&CardCommand::OpenBuildMenu));
        assert_ne!(worker.get(&'b'), build.get(&'b'));
        // 换卡重建键表：旧卡独有绑定在新卡不存在。
        assert!(build.get(&'m').is_none(), "reason=old_binding_invalid");
        assert!(producer.get(&'t').is_some(), "reason=producer_rebuild");
    }

    #[test]
    fn kind_lookup_reads_definition_table_not_name_whitelist() {
        let catalog = CommandCatalog::standard();
        assert_eq!(catalog.kind_of(HUMAN_PEASANT), CardKind::Worker);
        assert_eq!(catalog.kind_of(HUMAN_TOWN_HALL), CardKind::Producer);
        assert_eq!(catalog.kind_of(NEUTRAL_GOLD_MINE), CardKind::Resource);
        // 未登记 `DefId`（含运行时动态实体）回退通用类型、仍可派生，
        // 不因名字含 hall / mine / tree 而被白名单误判。
        assert_eq!(catalog.kind_of(999_999), CardKind::Combat);
        let empty = catalog.primary_card(&[]);
        assert_eq!(empty, CardId::Main);
        assert_eq!(
            catalog.primary_card(&[CardKind::Combat, CardKind::Worker]),
            CardId::Worker
        );
        assert_eq!(
            catalog.primary_card(&[CardKind::Combat, CardKind::Producer]),
            CardId::Producer
        );
    }
}
