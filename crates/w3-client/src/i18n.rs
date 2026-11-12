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
};
