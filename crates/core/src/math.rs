//! math(M6 / Plan 12 / 0013 §8)— LaTeX 数学排版:**RaTeX 在 core 算绝对坐标字形**,映射到
//! [`MathLayout`](块内相对 em 坐标),下游 build_frame 乘 `math_px` 摆成 `FrameGlyph`/`FrameRect`。
//!
//! 数学字形 = 和正文同级的 **SDF 一等公民**(进 atlas、缩放锐利、逐字可控/可动画),**不烤纹理**。
//! 排版纯 Rust(RaTeX:`parse → layout → to_display_list`)、wasm 安全(CR1,无 fs/平台依赖);
//! JS 只按 `(KaTeX 字族, char_code, size)` 栅化字形外观(复用现有 glyph atlas 管线)。
//!
//! 坐标系(RaTeX `to_display`):x 右增、**y 下增**(屏幕坐标),原点 = 包围盒左上,baseline 在
//! `y=height`,单位 = KaTeX em。`GlyphPath.y` = 该字基线 y,`scale` = 该字 em 字号倍率。
//! `char_code` 已是该字族 cmap 内码点(RaTeX 已解析数学字母映射)→ `char::from_u32` 直接可栅化。

use crate::content::StyleRole;
use crate::frame::{FrameGlyph, FrameRect};
use ratex_layout::{layout, to_display_list, LayoutOptions};
use ratex_parser::parse;
use ratex_types::display_item::DisplayItem;
use ratex_types::math_style::MathStyle;

/// 一个数学字形(块内相对 em 坐标;build_frame 乘 `math_px` → world px)。
#[derive(Clone, Debug, PartialEq)]
pub struct MathGlyph {
    /// 字符(`char::from_u32(char_code)`;= 该 KaTeX 字族 cmap 内码点)。
    pub ch: char,
    /// 字族角色(决定栅化字体 + atlas 分桶;web `fontForRole` 据此选 KaTeX 字体)。
    pub role: StyleRole,
    /// em 字号倍率(`GlyphPath.scale`)。
    pub size: f32,
    /// 块内 x(em,左边缘)。
    pub dx: f32,
    /// 块内基线 y(em,y 下增)。
    pub dy: f32,
}

/// 一条数学线/底(块内相对 em):分数线 / 上划线(`Line`)或实心底(`Rect`,`\colorbox`)。
/// 映射到 `FrameRect`(细线 / 实心)。
#[derive(Clone, Debug, PartialEq)]
pub struct MathRule {
    pub dx: f32,
    pub dy: f32,
    pub w: f32,
    pub h: f32,
}

/// 一个公式的排版结果(块内相对 em 坐标 + 包围盒)。`ok=false` = RaTeX 解析/排版失败 → 上游兜底
/// (相位⑦:退 MathJax/占位;v1 退原文 TeX 文本)。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MathLayout {
    /// 包围盒宽(em)。
    pub width: f32,
    /// baseline 以上高(em)。
    pub height: f32,
    /// baseline 以下深(em);行内对齐用。
    pub depth: f32,
    pub glyphs: Vec<MathGlyph>,
    pub rules: Vec<MathRule>,
    /// 排版成功。
    pub ok: bool,
}

impl MathLayout {
    /// 总高(em)= height + depth。
    pub fn total_height(&self) -> f32 {
        self.height + self.depth
    }
}

/// RaTeX `GlyphPath.font` 字符串 → 我们的数学字形角色(StyleRole)。未知字族退 [`StyleRole::MathMain`]。
#[allow(clippy::match_same_arms)] // reason: 显式列举 Main-Regular 与兜底同为 MathMain,保映射表清晰
pub fn font_role(font: &str) -> StyleRole {
    match font {
        "Main-Regular" => StyleRole::MathMain,
        "Main-Bold" => StyleRole::MathBold,
        "Main-Italic" => StyleRole::MathItalic,
        "Main-BoldItalic" | "Math-BoldItalic" => StyleRole::MathBoldItalic,
        "Math-Italic" => StyleRole::MathVar,
        "AMS-Regular" => StyleRole::MathAms,
        "Size1-Regular" => StyleRole::MathSize1,
        "Size2-Regular" => StyleRole::MathSize2,
        "Size3-Regular" => StyleRole::MathSize3,
        "Size4-Regular" => StyleRole::MathSize4,
        "Caligraphic-Regular" => StyleRole::MathCal,
        "Fraktur-Regular" | "Fraktur-Bold" => StyleRole::MathFrak,
        "SansSerif-Regular" | "SansSerif-Bold" | "SansSerif-Italic" => StyleRole::MathSans,
        "Script-Regular" => StyleRole::MathScript,
        "Typewriter-Regular" => StyleRole::MathTt,
        _ => StyleRole::MathMain, // CJK/Emoji 兜底等
    }
}

/// 数学角色 → 该用的 KaTeX 字族文件基名(`KaTeX_<base>`;web 加载/栅化用)。非数学角色返回 None。
pub fn katex_font_base(role: StyleRole) -> Option<&'static str> {
    Some(match role {
        StyleRole::MathMain => "Main-Regular",
        StyleRole::MathBold => "Main-Bold",
        StyleRole::MathItalic => "Main-Italic",
        StyleRole::MathBoldItalic => "Main-BoldItalic",
        StyleRole::MathVar => "Math-Italic",
        StyleRole::MathAms => "AMS-Regular",
        StyleRole::MathSize1 => "Size1-Regular",
        StyleRole::MathSize2 => "Size2-Regular",
        StyleRole::MathSize3 => "Size3-Regular",
        StyleRole::MathSize4 => "Size4-Regular",
        StyleRole::MathCal => "Caligraphic-Regular",
        StyleRole::MathFrak => "Fraktur-Regular",
        StyleRole::MathSans => "SansSerif-Regular",
        StyleRole::MathScript => "Script-Regular",
        StyleRole::MathTt => "Typewriter-Regular",
        _ => return None,
    })
}

/// 排版一段 LaTeX → [`MathLayout`](块内相对 em)。`display` = `$$…$$`(`Display` 样式)否则 `$…$`
/// (`Text` 行内样式)。RaTeX 三步 `parse → layout → to_display_list`,映射 DisplayList 各 item:
/// `GlyphPath`→[`MathGlyph`]、`Line`/`Rect`→[`MathRule`]。`Path`(根号/大定界符矢量轮廓)v1 暂略
/// (多数定界符走 `Size1–4` 字形已 atlas 化;极大者 Path = 相位④)。解析失败 → `ok=false`。
pub fn layout_math(tex: &str, display: bool) -> MathLayout {
    let Ok(nodes) = parse(tex) else {
        return MathLayout::default(); // ok=false → 上游兜底
    };
    let opts = LayoutOptions {
        style: if display {
            MathStyle::Display
        } else {
            MathStyle::Text
        },
        ..Default::default()
    };
    let lbox = layout(&nodes, &opts);
    let dl = to_display_list(&lbox);

    let mut glyphs = Vec::new();
    let mut rules = Vec::new();
    for item in &dl.items {
        match item {
            DisplayItem::GlyphPath {
                x,
                y,
                scale,
                font,
                char_code,
                ..
            } => {
                if let Some(ch) = char::from_u32(*char_code) {
                    glyphs.push(MathGlyph {
                        ch,
                        role: font_role(font),
                        size: *scale as f32,
                        dx: *x as f32,
                        dy: *y as f32,
                    });
                }
            }
            // 分数线/上划线:细水平线。映射成一条 `MathRule`(厚度 = 线高)。
            DisplayItem::Line {
                x,
                y,
                width,
                thickness,
                ..
            } => rules.push(MathRule {
                dx: *x as f32,
                dy: *y as f32 - *thickness as f32 * 0.5, // y = 线中心 → 取顶
                w: *width as f32,
                h: *thickness as f32,
            }),
            // \colorbox 实心底。
            DisplayItem::Rect {
                x,
                y,
                width,
                height,
                ..
            } => rules.push(MathRule {
                dx: *x as f32,
                dy: *y as f32,
                w: *width as f32,
                h: *height as f32,
            }),
            // Path(根号 surd / 大定界符矢量轮廓):相位④;v1 暂不产(多数走 Size 字形)。
            DisplayItem::Path { .. } => {}
        }
    }
    MathLayout {
        width: dl.width as f32,
        height: dl.height as f32,
        depth: dl.depth as f32,
        glyphs,
        rules,
        ok: true,
    }
}

/// 把 [`MathLayout`](块内相对 em)摆成 **world 坐标的 `FrameGlyph`/`FrameRect`**(em × `math_px`)。
/// `origin` = 公式包围盒**左上角** world 坐标;`spawn` = 上屏时刻(同块揭示);`color` = 规则线/字 RGBA。
///
/// 坐标:RaTeX y 下增、baseline 在 `dy`。`FrameGlyph.pos` = quad 左上角 → 取 `dy - size`(≈ baseline
/// 上推一个 em 的字盒顶;web 栅化按字体 ascent 精对齐,此为 SDF tile 的近似盒)。`glyph_idx` = 公式内
/// 序号(morph 身份低位,0016)。规则线高至少 1px(细分数线不丢)。
pub fn math_to_frame(
    m: &MathLayout,
    origin: [f32; 2],
    math_px: f32,
    block_seq: u32,
    spawn: f32,
    color: [f32; 4],
) -> (Vec<FrameGlyph>, Vec<FrameRect>) {
    let mut glyphs = Vec::with_capacity(m.glyphs.len());
    for (i, g) in m.glyphs.iter().enumerate() {
        let px = g.size * math_px; // 该字 em 字号 → px
        glyphs.push(FrameGlyph {
            cluster: g.ch.to_string(),
            pos: [
                origin[0] + g.dx * math_px,
                origin[1] + (g.dy - g.size) * math_px,
            ],
            size: [px, px],
            spawn_time: spawn,
            style: g.role.as_u32(),
            block_seq,
            glyph_idx: i as u32,
            anim: 0,
        });
    }
    let rects = m
        .rules
        .iter()
        .map(|r| FrameRect {
            pos: [origin[0] + r.dx * math_px, origin[1] + r.dy * math_px],
            // 线高:TeX 规则厚(~0.04em)在显示尺寸下仅 ~0.8 CSS px → 高 DPI 被 AA 抹没。给一个**随
            // 字号缩放的可见下限**(em 的 5%,且 ≥1.5px),分数线/根线在大公式下清晰可见。
            size: [r.w * math_px, (r.h * math_px).max(math_px * 0.05).max(1.5)],
            color,
            radius: 0.0,
            stroke: 0.0,
        })
        .collect();
    (glyphs, rects)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chars(m: &MathLayout) -> String {
        m.glyphs.iter().map(|g| g.ch).collect()
    }

    #[test]
    fn emc2_lays_out_in_reading_order() {
        let m = layout_math("E=mc^2", true);
        assert!(m.ok, "应排版成功");
        assert!(m.width > 0.0 && m.height > 0.0);
        let s = chars(&m);
        // 字形含 E = m c 2(上标 2 也在);顺序按 dx 递增(从左到右)。
        for c in ['E', '=', 'm', 'c', '2'] {
            assert!(s.contains(c), "缺字形 {c}: {s:?}");
        }
        let mut xs: Vec<f32> = m.glyphs.iter().map(|g| g.dx).collect();
        let sorted = {
            let mut v = xs.clone();
            v.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
            v
        };
        xs.sort_by(|a, b| a.partial_cmp(b).expect("finite"));
        assert_eq!(xs, sorted);
        // 上标 '2' 应高于基线('c' 的 dy):y 下增 → 2 的 dy < c 的 dy。
        let dy = |c: char| m.glyphs.iter().find(|g| g.ch == c).map(|g| g.dy);
        if let (Some(c2), Some(cc)) = (dy('2'), dy('c')) {
            assert!(c2 < cc, "上标 2 应在基线之上: {c2} < {cc}");
        }
    }

    #[test]
    fn fraction_has_rule_line() {
        let m = layout_math(r"\frac{a}{b}", true);
        assert!(m.ok);
        assert!(!m.rules.is_empty(), "分数应有分数线(Line→MathRule)");
        let s = chars(&m);
        assert!(s.contains('a') && s.contains('b'), "分子分母字形: {s:?}");
    }

    #[test]
    fn sum_and_sqrt_parse() {
        let sum = layout_math(r"\sum_{i=0}^n i", true);
        assert!(sum.ok && !sum.glyphs.is_empty(), "求和应排版");
        let sq = layout_math(r"\sqrt{x}", true);
        assert!(sq.ok, "根号应排版(surd 走 Size 字形或 Path)");
        assert!(sq.glyphs.iter().any(|g| g.ch == 'x'), "根号下 x");
    }

    #[test]
    fn inline_vs_display_style() {
        // 行内(Text)与显示(Display)样式都应成功;显示样式上下标更舒展(高度通常 ≥ 行内)。
        let disp = layout_math(r"\sum_{i=0}^n", true);
        let inl = layout_math(r"\sum_{i=0}^n", false);
        assert!(disp.ok && inl.ok);
        assert!(disp.total_height() >= inl.total_height());
    }

    #[test]
    fn invalid_latex_is_not_ok() {
        let m = layout_math(r"\frac{a", true); // 未闭合
        assert!(!m.ok, "解析失败应 ok=false(上游兜底)");
        assert!(m.glyphs.is_empty());
    }

    #[test]
    fn math_to_frame_places_glyphs_in_world() {
        let m = layout_math("a+b", true);
        let (glyphs, _rects) =
            math_to_frame(&m, [100.0, 50.0], 20.0, 7, 1234.0, [1.0, 1.0, 1.0, 1.0]);
        assert!(!glyphs.is_empty());
        // 全部带块身份 + spawn;world x 单调递增(从 origin.x 起,左到右)。
        assert!(glyphs
            .iter()
            .all(|g| g.block_seq == 7 && (g.spawn_time - 1234.0).abs() < 1e-3));
        let xs: Vec<f32> = glyphs.iter().map(|g| g.pos[0]).collect();
        assert!(
            xs.windows(2).all(|w| w[1] >= w[0]),
            "world x 应递增: {xs:?}"
        );
        assert!(xs[0] >= 100.0, "首字应在 origin.x 右侧");
        // glyph_idx 连续(morph 身份)。
        for (i, g) in glyphs.iter().enumerate() {
            assert_eq!(g.glyph_idx, i as u32);
        }
    }

    #[test]
    fn math_to_frame_fraction_rule_world() {
        let m = layout_math(r"\frac{a}{b}", true);
        let (_g, rects) = math_to_frame(&m, [0.0, 0.0], 16.0, 0, 0.0, [1.0, 1.0, 1.0, 1.0]);
        assert!(!rects.is_empty(), "分数线应映射成 FrameRect");
        assert!(rects.iter().all(|r| r.size[1] >= 1.0), "线高至少 1px");
    }

    #[test]
    fn font_role_maps_katex_families() {
        assert_eq!(font_role("Math-Italic"), StyleRole::MathVar);
        assert_eq!(font_role("Main-Regular"), StyleRole::MathMain);
        assert_eq!(font_role("Size1-Regular"), StyleRole::MathSize1);
        assert_eq!(font_role("???"), StyleRole::MathMain); // 兜底
        assert_eq!(katex_font_base(StyleRole::MathVar), Some("Math-Italic"));
        assert_eq!(katex_font_base(StyleRole::Normal), None);
    }
}
