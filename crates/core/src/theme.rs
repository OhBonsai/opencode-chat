//! theme(M6/M13)— github-theme 设计令牌(Plan 4B3 → Plan 26① 运行时化)。
//!
//! **单一出处**:装饰矩形(代码块底 / 行内码 chip / 引用左条 / 标题细线 / Alert 条与底 /
//! 选区 / 卡片 / diff 行底)的颜色集中在 [`Theme`]。字形**文字色**仍在 `glyph.wgsl` 的
//! `style_color`(GPU 取色),字号比例在 `web/src/layout-bridge.ts` 的 `roleScale` ——
//! 三处构成跨语言令牌表(本文件是 Rust/装饰侧的权威)。
//!
//! Plan 26①:令牌从 `const` 收编为**可运行时替换**的 [`Theme`] —— Engine 持一份,wasm
//! `set_theme(json)` 局部覆盖。关键性质:**颜色不进排版缓存**——缓存存 `StyleRole`/几何,
//! 颜色在 `build_frame` emit 时才解析 → 换主题**不重排,下一帧生效**(与 0029 虚拟化正交)。
//! `Default` = 迁移前的常量值,**默认主题逐字节等于旧观感,零回归**。
//!
//! 颜色一律 `[r,g,b,a]`(0–1,sRGB 直值;`a<1` 半透明叠底);Alert 强调色为 `[r,g,b]`。

use serde::Deserialize;

/// 视觉令牌表(装饰侧)。字段名即 JSON 键(snake_case);`#[serde(default)]` → 缺字段用默认
/// 值补,**局部覆盖友好**(如只改 `{"selection":[...]}`)。CR1:纯数据,native 可测。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default)]
pub struct Theme {
    /// 代码块底(整宽圆角)。
    pub code_bg: [f32; 4],
    /// 代码块行号 gutter 与代码区的分隔细线(Plan 15 ②⑥)。
    pub code_gutter_line: [f32; 4],
    /// 代码块外框描边(Plan 15 ⑥)。
    pub code_border: [f32; 4],
    /// 行内码 chip 底(逐行)。
    pub code_chip: [f32; 4],
    /// 普通引用左条。
    pub quote_bar: [f32; 4],
    /// H1/H2 底部细线(GitHub 风)。
    pub head_rule: [f32; 4],
    /// 分隔线(`---`)中央色(shader 横向淡出到 0,故中央需可见;Plan 11)。
    pub hr_rule: [f32; 4],
    /// 删除线(`~~…~~`):字中线细线,中性浅灰偏暖。
    pub strike: [f32; 4],
    /// 任务复选框·未勾(0026/Plan 11)。
    pub task_box: [f32; 4],
    /// 任务复选框·已勾(GitHub 风绿)。
    pub task_done: [f32; 4],
    /// 表格表头底(淡,0014 A)。
    pub table_header_bg: [f32; 4],
    /// 表格分隔线(表头底线 / 表尾外边线)。
    pub table_rule: [f32; 4],
    /// 文本选区高亮(Plan 21 P2 / 0030:画在文字之下,文字永不被洗淡)。
    pub selection: [f32; 4],
    /// tool / reasoning / compaction 卡底(Plan 23;SDF 面板,微透叠底)。
    pub card_bg: [f32; 4],
    /// tool / reasoning 卡描边(细,圆角)。
    pub card_border: [f32; 4],
    /// diff 新增行底(绿,半透叠底)。
    pub diff_add_bg: [f32; 4],
    /// diff 删除行底(红,半透叠底)。
    pub diff_del_bg: [f32; 4],
    /// 调试:块 AABB 描边。
    pub dbg_block: [f32; 4],
    /// 调试:视口框描边。
    pub dbg_view: [f32; 4],
    /// GitHub Alert 强调色(左条实心 / 整块淡底),按类型;对标 github 深色 accent。
    pub alert_note: [f32; 3],
    pub alert_tip: [f32; 3],
    pub alert_important: [f32; 3],
    pub alert_warning: [f32; 3],
    pub alert_caution: [f32; 3],
    /// 未知 Alert 类型的兜底中性色。
    pub alert_neutral: [f32; 3],
}

impl Default for Theme {
    /// 迁移前的常量值(github-markdown-css 深色)。**改此处 = 改默认观感**,视觉黄金帧会红。
    fn default() -> Self {
        Self {
            code_bg: [0.10, 0.11, 0.16, 0.75],
            code_gutter_line: [0.30, 0.33, 0.42, 0.6],
            code_border: [0.32, 0.36, 0.46, 0.85],
            code_chip: [0.18, 0.19, 0.26, 0.7],
            quote_bar: [0.42, 0.46, 0.56, 0.9],
            head_rule: [0.24, 0.27, 0.33, 0.9],
            hr_rule: [0.82, 0.86, 0.94, 1.0],
            strike: [0.80, 0.82, 0.88, 0.85],
            task_box: [0.55, 0.60, 0.70, 0.95],
            task_done: [0.40, 0.80, 0.55, 0.98],
            table_header_bg: [0.16, 0.18, 0.24, 0.6],
            table_rule: [0.26, 0.29, 0.36, 0.9],
            selection: [0.26, 0.45, 0.92, 0.40],
            card_bg: [0.14, 0.16, 0.21, 0.55],
            card_border: [0.30, 0.34, 0.44, 0.7],
            diff_add_bg: [0.22, 0.45, 0.27, 0.35],
            diff_del_bg: [0.50, 0.22, 0.24, 0.35],
            dbg_block: [0.40, 0.90, 0.50, 0.7],
            dbg_view: [0.95, 0.80, 0.30, 0.85],
            alert_note: [0.35, 0.65, 1.0],      // 蓝
            alert_tip: [0.30, 0.80, 0.45],      // 绿
            alert_important: [0.70, 0.50, 1.0], // 紫
            alert_warning: [0.95, 0.75, 0.25],  // 琥珀
            alert_caution: [0.95, 0.45, 0.45],  // 红
            alert_neutral: [0.55, 0.60, 0.70],
        }
    }
}

impl Theme {
    /// 从 JSON 解析(缺字段用默认补 → 局部覆盖友好)。wasm `set_theme` 用;错误由调用方处理。
    pub fn from_json(json: &str) -> Result<Theme, serde_json::Error> {
        serde_json::from_str(json)
    }

    /// Alert 强调色(按类型;未知 → 中性兜底)。
    fn alert_accent(&self, label: &str) -> [f32; 3] {
        match label {
            "NOTE" => self.alert_note,
            "TIP" => self.alert_tip,
            "IMPORTANT" => self.alert_important,
            "WARNING" => self.alert_warning,
            "CAUTION" => self.alert_caution,
            _ => self.alert_neutral,
        }
    }

    /// Alert 左条颜色(实心,按类型)。`label` = `NOTE`/`TIP`/`IMPORTANT`/`WARNING`/`CAUTION`。
    pub(crate) fn alert_bar(&self, label: &str) -> [f32; 4] {
        let [r, g, b] = self.alert_accent(label);
        [r, g, b, 0.95]
    }

    /// Alert 整块淡底(同强调色低 alpha,叠在文字下作 GitHub 风提示底)。
    pub(crate) fn alert_bg(&self, label: &str) -> [f32; 4] {
        let [r, g, b] = self.alert_accent(label);
        [r, g, b, 0.08]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 4], b: [f32; 4]) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6)
    }

    #[test]
    fn default_matches_legacy_palette() {
        // Plan 26① 迁移护栏:默认主题逐值 = 迁移前常量(零回归;视觉黄金帧同守)。
        let t = Theme::default();
        assert!(close(t.code_bg, [0.10, 0.11, 0.16, 0.75]));
        assert!(close(t.selection, [0.26, 0.45, 0.92, 0.40]));
        assert!(close(t.card_bg, [0.14, 0.16, 0.21, 0.55]));
        assert!(close(t.table_rule, [0.26, 0.29, 0.36, 0.9]));
        assert!(close(t.hr_rule, [0.82, 0.86, 0.94, 1.0]));
    }

    #[test]
    fn partial_json_overrides_only_named_fields() {
        // 局部覆盖:只给 selection → 其余字段保持默认(serde(default))。
        let t: Theme = serde_json::from_str(r#"{"selection":[1.0,0.0,0.0,0.5]}"#).expect("parse");
        assert!(close(t.selection, [1.0, 0.0, 0.0, 0.5]));
        assert!(close(t.code_bg, Theme::default().code_bg));
        let (a, b) = (t.alert_note, Theme::default().alert_note);
        assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-6));
    }

    #[test]
    fn alert_types_have_distinct_accents() {
        let t = Theme::default();
        // NOTE(蓝)与 WARNING(琥珀)红通道差异明显;淡底与左条同色相、低 alpha。
        assert!((t.alert_bar("NOTE")[0] - t.alert_bar("WARNING")[0]).abs() > 0.1);
        assert!(t.alert_bg("NOTE")[3] < t.alert_bar("NOTE")[3]);
        assert!((t.alert_bg("NOTE")[0] - t.alert_bar("NOTE")[0]).abs() < 1e-6);
    }

    #[test]
    fn unknown_alert_falls_back_neutral() {
        let t = Theme::default();
        assert!(close(t.alert_bar("???"), t.alert_bar("NOPE")));
    }
}
