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
/// 行内码 chip 底(逐行)。
pub(crate) const CODE_CHIP: [f32; 4] = [0.18, 0.19, 0.26, 0.7];
/// 普通引用左条。
pub(crate) const QUOTE_BAR: [f32; 4] = [0.42, 0.46, 0.56, 0.9];
/// H1/H2 底部细线(GitHub 风)。
pub(crate) const HEAD_RULE: [f32; 4] = [0.24, 0.27, 0.33, 0.9];
/// 分隔线(`---`)。
pub(crate) const HR_RULE: [f32; 4] = [0.20, 0.23, 0.28, 0.9];

/// 表格表头底(淡,0014 A)。
pub(crate) const TABLE_HEADER_BG: [f32; 4] = [0.16, 0.18, 0.24, 0.6];
/// 表格分隔线(表头底线 / 表尾外边线)。
pub(crate) const TABLE_RULE: [f32; 4] = [0.26, 0.29, 0.36, 0.9];

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
