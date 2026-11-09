//! UI 文案（i18n）。中文标签只存在于表现层。
//!
//! 硬红线：本模块所有文案 MUST NOT 进入任何 `log` 记录、权威状态或状态哈希
//! （CODEBUDDY.md §3、`playable-client` spec）。日志一律英文结构化 `key=value`，
//! 不引用这些常量。字体使用 egui 内置默认字体，不加载任何资产文件（ADR 0001）。

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
}

/// 当前（第一版）本地化：简体中文。
pub const ZH: Texts = Texts {
    window_title: "w3-client 可玩客户端",
    hud_title: "战况",
    selection: "选中单位",
    resources: "资源",
    gold: "金币",
    lumber: "木材",
    supply: "人口",
    production: "生产队列",
    hp: "生命",
    tick: "逻辑帧",
    no_selection: "未选中单位",
    empty_queue: "队列为空",
    font_unavailable: "字体不可用，文字元素已隐藏",
    action_hint: "左键框选 · 右键移动/攻击 · Q 施法 · T 生产 · C 取消 · WASD 平移 · 滚轮缩放",
};
