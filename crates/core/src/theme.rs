//! theme(M6/M13)— 固化的 github-theme 设计令牌(Plan 4B3)。
//!
//! **单一出处**:装饰矩形(代码块底 / 行内码 chip / 引用左条 / 标题细线 / Alert 条与底)的
//! 颜色集中在此,取色对标 `github-markdown-css`(深色)。字形**文字色**仍在 `glyph.wgsl`
//! 的 `style_color`(GPU 取色,见那里同名注释),字号比例在 `web/src/layout-bridge.ts` 的
//! `roleScale` —— 三处构成跨语言令牌表(本文件是 Rust/装饰侧的权威)。
//!
//! 颜色一律 `[r,g,b,a]`(0–1,sRGB 直值;`a<1` 半透明叠底)。

/// 代码块底(整宽圆角)。
pub(crate) const CODE_BG: [f32; 4] = [0.10, 0.11, 0.16, 0.75];
/// 代码块行号 gutter 与代码区的分隔细线(Plan 15 ②⑥)。
pub(crate) const CODE_GUTTER_LINE: [f32; 4] = [0.30, 0.33, 0.42, 0.6];
/// 代码块外框描边(Plan 15 ⑥:可见 box 框)。
pub(crate) const CODE_BORDER: [f32; 4] = [0.32, 0.36, 0.46, 0.85];
/// 行内码 chip 底(逐行)。
pub(crate) const CODE_CHIP: [f32; 4] = [0.18, 0.19, 0.26, 0.7];
/// 普通引用左条。
pub(crate) const QUOTE_BAR: [f32; 4] = [0.42, 0.46, 0.56, 0.9];
/// H1/H2 底部细线(GitHub 风)。
pub(crate) const HEAD_RULE: [f32; 4] = [0.24, 0.27, 0.33, 0.9];
/// 分隔线(`---`)。中央色(Plan 11:迁 markdown widget,中间亮两端淡出渐变线 → 取略亮中性,
/// shader 横向淡出到 0,故中央需可见)。
pub(crate) const HR_RULE: [f32; 4] = [0.82, 0.86, 0.94, 1.0];
/// 删除线(`~~…~~`,A):字中线一条细线,中性浅灰偏暖,暗底可读。
pub(crate) const STRIKE: [f32; 4] = [0.80, 0.82, 0.88, 0.85];

/// 任务复选框·未勾:框线中性色(0026/Plan 11,markdown widget;暗底可辨)。
pub(crate) const TASK_BOX: [f32; 4] = [0.55, 0.60, 0.70, 0.95];
/// 任务复选框·已勾:框 + 对勾强调色(GitHub 风绿)。
pub(crate) const TASK_DONE: [f32; 4] = [0.40, 0.80, 0.55, 0.98];

/// 表格表头底(淡,0014 A)。
pub(crate) const TABLE_HEADER_BG: [f32; 4] = [0.16, 0.18, 0.24, 0.6];
/// 表格分隔线(表头底线 / 表尾外边线)。
pub(crate) const TABLE_RULE: [f32; 4] = [0.26, 0.29, 0.36, 0.9];

/// 文本选区高亮(Plan 21 P2 / 0030):画在文字**之下**(rect pass 先于 glyph)→ 文字永全不透明在上,
/// 任意多色(代码/链接/标题)不被洗淡。半透明蓝,暗底可辨;DOM `::selection` 透明,高亮独此一份。
pub(crate) const SELECTION: [f32; 4] = [0.26, 0.45, 0.92, 0.40];

// ── Plan 23 part 渲染装饰(tool 卡 / reasoning / diff;0018 SDF 面板 + 行底 rect)。
/// tool / reasoning / compaction 卡底(SDF 面板,微透叠底 → 与正文区分)。
pub(crate) const CARD_BG: [f32; 4] = [0.14, 0.16, 0.21, 0.55];
/// tool / reasoning 卡描边(细,圆角)。
pub(crate) const CARD_BORDER: [f32; 4] = [0.30, 0.34, 0.44, 0.7];
/// diff 新增行底(绿,半透叠底)。
pub(crate) const DIFF_ADD_BG: [f32; 4] = [0.22, 0.45, 0.27, 0.35];
/// diff 删除行底(红,半透叠底)。
pub(crate) const DIFF_DEL_BG: [f32; 4] = [0.50, 0.22, 0.24, 0.35];

/// 调试:块 AABB 描边。
pub(crate) const DBG_BLOCK: [f32; 4] = [0.40, 0.90, 0.50, 0.7];
/// 调试:视口框描边。
pub(crate) const DBG_VIEW: [f32; 4] = [0.95, 0.80, 0.30, 0.85];

/// GitHub Alert 左条强调色(按类型),对标 github 深色 accent。
fn alert_accent(label: &str) -> [f32; 3] {
    match label {
        "NOTE" => [0.35, 0.65, 1.0],      // 蓝
        "TIP" => [0.30, 0.80, 0.45],      // 绿
        "IMPORTANT" => [0.70, 0.50, 1.0], // 紫
        "WARNING" => [0.95, 0.75, 0.25],  // 琥珀
        "CAUTION" => [0.95, 0.45, 0.45],  // 红
        _ => [0.55, 0.60, 0.70],          // 兜底:中性
    }
}

/// Alert 左条颜色(实心,按类型)。`label` = `NOTE`/`TIP`/`IMPORTANT`/`WARNING`/`CAUTION`。
pub(crate) fn alert_bar(label: &str) -> [f32; 4] {
    let [r, g, b] = alert_accent(label);
    [r, g, b, 0.95]
}

/// Alert 整块淡底(同强调色低 alpha,叠在文字下作 GitHub 风提示底)。
pub(crate) fn alert_bg(label: &str) -> [f32; 4] {
    let [r, g, b] = alert_accent(label);
    [r, g, b, 0.08]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 4], b: [f32; 4]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6)
    }

    #[test]
    fn alert_types_have_distinct_accents() {
        // NOTE(蓝)与 WARNING(琥珀)红通道差异明显。
        assert!((alert_bar("NOTE")[0] - alert_bar("WARNING")[0]).abs() > 0.1);
        // 淡底与左条同色相、低 alpha。
        assert!(alert_bg("NOTE")[3] < alert_bar("NOTE")[3]);
        assert!((alert_bg("NOTE")[0] - alert_bar("NOTE")[0]).abs() < 1e-6);
    }

    #[test]
    fn unknown_alert_falls_back_neutral() {
        assert!(close(alert_bar("???"), alert_bar("NOPE")));
    }
}
