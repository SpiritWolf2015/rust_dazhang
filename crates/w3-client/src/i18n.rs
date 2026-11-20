//! UI 文案（i18n）。文案只存在于表现层。
//!
//! 硬红线：本模块所有文案 MUST NOT 进入任何 `log` 记录、权威状态或状态哈希
//! （CODEBUDDY.md §3、`playable-client` spec）。日志一律英文结构化 `key=value`，
//! 不引用这些常量。字体使用 egui 内置开源默认字体（`default_fonts`），
//! 不加载任何资产文件（ADR 0001）；内置字体不含 CJK 字形，故第一版文案
//! 使用 ASCII 保证可见，中文上屏留后续独立 change。

/// HUD、窗口标题与输入提示文案集合；纯展示数据，不参与确定性计算。
#[derive(Clone, Copy, Debug)]
pub struct Texts {
    /// 操作系统窗口标题（由宿主创建窗口时读取，只存在于表现层）。
    pub window_title: &'static str,
    pub hud_title: &'static str,
    pub selection: &'static str,
    pub resources: &'static str,
    pub gold: &'static str,
    pub lumber: &'static str,
    pub supply: &'static str,
    pub production: &'static str,
    pub hp: &'static str,
    pub tick: &'static str,
    pub no_selection: &'static str,
    pub empty_queue: &'static str,
    pub font_unavailable: &'static str,
    pub action_hint: &'static str,
    /// 命令卡标题。
    pub commands: &'static str,
    /// 选择面板标题。
    pub selection_panel: &'static str,
    /// 小地图标题。
    pub minimap: &'static str,
    /// 胜者标签。
    pub winner: &'static str,
    /// 胜负结算标题。
    pub victory: &'static str,
    /// 开局前选族 overlay 标题（Q1）。
    pub faction_title: &'static str,
    /// 本地族标签。
    pub faction_local: &'static str,
    /// AI 族标签。
    pub faction_ai: &'static str,
    /// 人族选项标签。
    pub faction_human: &'static str,
    /// 兽族选项标签。
    pub faction_orc: &'static str,
    /// 确认按钮标签。
    pub faction_confirm: &'static str,
    /// 已选族只读状态提示。
    pub faction_status: &'static str,
    /// 祭坛复活队列标题（P8.5a / tasks 6.2）。
    pub revive_queue: &'static str,
    /// 复活队列条目主体标签。
    pub revive_hero: &'static str,
    /// 复活剩余 / 总帧标签。
    pub revive_remaining: &'static str,
    /// 复活费用标签。
    pub revive_cost: &'static str,
    /// 空复活队列提示。
    pub revive_empty: &'static str,
    /// 死亡登记条数标签。
    pub revive_deaths: &'static str,
    /// 冻结扣费登记条数标签。
    pub revive_charges: &'static str,
    /// 英雄阵亡事实事件标签。
    pub hero_died: &'static str,
    /// 英雄复活完成事实事件标签。
    pub hero_revived: &'static str,
    /// 主城升级队列标题（P8.5b / tasks 6.2）。
    pub town_hall_queue: &'static str,
    /// 主城升级条目主体标签。
    pub town_hall: &'static str,
    /// 主城等级标签。
    pub town_hall_tier: &'static str,
    /// 主城升级剩余 / 总帧标签。
    pub town_hall_remaining: &'static str,
    /// 主城升级费用标签。
    pub town_hall_cost: &'static str,
    /// 空升级队列提示。
    pub town_hall_empty: &'static str,
    /// 升级冻结扣费登记条数标签。
    pub town_hall_charges: &'static str,
    /// 升级开始事实事件标签。
    pub town_hall_started: &'static str,
    /// 升级完成事实事件标签。
    pub town_hall_completed: &'static str,
    /// 升级取消 / 中止事实事件标签。
    pub town_hall_cancelled: &'static str,
    /// 已登记主城等级条数标签。
    pub town_hall_tier_count: &'static str,
    /// 野怪营地 HUD 标题 / 计数标签（P8.5c / tasks 7.2）。
    pub creep_camps: &'static str,
    /// 无野怪营地数据提示。
    pub creep_empty: &'static str,
    /// 已全清营地标签。
    pub creep_cleared: &'static str,
    /// 存活野怪标签。
    pub creep_alive: &'static str,
    /// 野怪血条标签。
    pub creep_hp: &'static str,
    /// leash 阶段标签。
    pub creep_phase: &'static str,
    /// 掉落标记标签。
    pub creep_drops: &'static str,
    /// 击杀收益标签。
    pub creep_rewards: &'static str,
    /// 经验收益标签。
    pub creep_xp: &'static str,
    /// 金币收益标签。
    pub creep_gold: &'static str,
    /// 事实事件计数标签。
    pub creep_events: &'static str,
    /// 营地装配事实事件标签。
    pub creep_seeded: &'static str,
    /// 野怪索敌事实事件标签。
    pub creep_engaged: &'static str,
    /// 野怪归巢开始事实事件标签。
    pub creep_leash_returning: &'static str,
    /// 野怪归巢完成事实事件标签。
    pub creep_leash_returned: &'static str,
    /// 野怪死亡事实事件标签。
    pub creep_died: &'static str,
    /// 道具掉落事实事件标签。
    pub item_dropped: &'static str,
}

/// 第一版本地化：ASCII 文案，保证内置开源字体可显示。
/// 常量名沿用 i18n 层既有导出，避免改动既有公开签名。
pub const ZH: Texts = Texts {
    window_title: "w3-client",
    hud_title: "Status",
    selection: "Selected",
    resources: "Resources",
    gold: "Gold",
    lumber: "Lumber",
    supply: "Supply",
    production: "Production",
    hp: "HP",
    tick: "Tick",
    no_selection: "none",
    empty_queue: "queue empty",
    font_unavailable: "font unavailable, text hidden",
    action_hint: "LMB drag: select | RMB: move/attack | G: gather | B: build | T: train | C: cancel | H: stop | Q: cast | F1..F9: group | 1..9: select group | WASD: pan | Wheel: zoom | Z/X: rotate",
    commands: "Commands",
    selection_panel: "Selection",
    minimap: "Minimap",
    winner: "Winner",
    victory: "VICTORY",
    faction_title: "Faction (pre-match)",
    faction_local: "Local",
    faction_ai: "AI",
    faction_human: "Human",
    faction_orc: "Orc",
    faction_confirm: "Confirm",
    faction_status: "Selected",
    revive_queue: "Revive Queue",
    revive_hero: "Hero",
    revive_remaining: "Frames",
    revive_cost: "Cost",
    revive_empty: "queue empty",
    revive_deaths: "Fallen",
    revive_charges: "Charges",
    hero_died: "Hero fell",
    hero_revived: "Hero revived",
    town_hall_queue: "Town Hall Upgrade",
    town_hall: "Hall",
    town_hall_tier: "Tier",
    town_hall_remaining: "Frames",
    town_hall_cost: "Cost",
    town_hall_empty: "no upgrade in progress",
    town_hall_charges: "Charges",
    town_hall_started: "Started",
    town_hall_completed: "Completed",
    town_hall_cancelled: "Cancelled",
    town_hall_tier_count: "Tiers",
    creep_camps: "Creep Camps",
    creep_empty: "no camp data",
    creep_cleared: "Cleared",
    creep_alive: "Alive",
    creep_hp: "Creep",
    creep_phase: "Phase",
    creep_drops: "Drops",
    creep_rewards: "Rewards",
    creep_xp: "XP",
    creep_gold: "Gold",
    creep_events: "Draws",
    creep_seeded: "Seeded",
    creep_engaged: "Engaged",
    creep_leash_returning: "Returning",
    creep_leash_returned: "Returned",
    creep_died: "Died",
    item_dropped: "Dropped",
};
