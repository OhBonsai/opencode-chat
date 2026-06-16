//! reveal(M6 / 0019)— 揭示门控 × 编排:把"何时够格揭示"(gate)与"按何序/节奏揭示"
//! (style/stage)从渲染机制里剥出来,二者皆数据驱动的 policy(0016 机制不动)。
//!
//! 四层模型(0019 §4):① gate 就绪谓词 → ② plan(`RevealStyle` 纯数据)→ ③ sched 调度器
//! (读门、单调激活 stage、产 spawn_time)→ ④ morph(0016 插值,完全不知 gate/plan)。
//!
//! 本模块是 ①②③ 的 core 实现(平台无关,native 可测,CR1);④ 在 render crate。Selector
//! 在 **[0020 节点树](crate::nodes)** 上按 kind/区间寻址(不按 glyph 下标写死)。
//!
//! 北极星(thinking §1/§3 / `md-reveal-cadence-north-star`):**阅读体验 > 实时性**——
//! 结构块绝不闪 raw、骨架先行(框→填字)、揭示节奏与 token 解耦(可限速 / 刻意放慢)。

use crate::nodes::{NodeKind, NodeTree};
use crate::seam::TablePanel;

/// 揭示就绪粒度(0019 §4.1):某块结构上已完成到哪一级。**单调只增**(append-only,
/// 0017 §6)——调度器据此只前进、无回滚。`Row(n)` 携已完成行数(表格风格 2 用)。
///
/// v1 取 `Glyph/Line/Row/Block` 四级(0019 §4.1 的 `Subtree` 嵌套根整体 = 后续)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RevealUnit {
    /// 逐字(段落/行内,风格 1 = 实时逐字)。
    Glyph,
    /// 整行/整逻辑行(列表项闭合、未闭合围栏的已到行)。
    Line,
    /// 表格第 `n` 行完成(风格 2:每行框→填字)。
    Row(u32),
    /// 整块完成(整表/整代码块/整列表,风格 3:整表→网格→表头→cell)。
    Block,
}

impl RevealUnit {
    /// 单调比较用的级别(`Glyph<Line<Row<Block`);`Row(n)` 的 n 不进 rank,另比。
    pub fn rank(self) -> u32 {
        match self {
            RevealUnit::Glyph => 0,
            RevealUnit::Line => 1,
            RevealUnit::Row(_) => 2,
            RevealUnit::Block => 3,
        }
    }

    /// 已完成行数(仅 `Row(n)` 非零)。
    pub fn rows_done(self) -> u32 {
        match self {
            RevealUnit::Row(n) => n,
            _ => 0,
        }
    }

    /// 本就绪级是否已达 `at`(单调阈值判定):rank 更高即达;同为 `Row` 比行数。
    pub fn reached(self, at: RevealUnit) -> bool {
        match (self, at) {
            (RevealUnit::Row(a), RevealUnit::Row(b)) => a >= b,
            (s, a) => s.rank() >= a.rank(),
        }
    }
}

/// 布局就绪门(0019 §5,双门之二):几何稳定到哪一级 = **框/网格能否精确画**。与内容门正交
/// (化解"行框需全表列宽"的依赖错配):列宽来自整表(0014 B 像素两趟),故 `table_panels` 一旦
/// 回传(列已定)→ `Block` 级布局就绪,可精确画整表骨架;否则 `Glyph`(无几何,只能逐字)。
pub fn layout_gate(table_panels: &[TablePanel]) -> RevealUnit {
    // 任一表面板带列线 = 列宽已定(几何稳定),整表骨架可精确入场。
    if table_panels.iter().any(|p| !p.cols.is_empty() || p.w > 0.0) {
        RevealUnit::Block
    } else {
        RevealUnit::Glyph
    }
}

/// 某 kind 是否"结构块"(骨架先行 / raw 抑制的对象;纯文本/行内不在内)。
pub fn is_structural(kind: NodeKind) -> bool {
    matches!(
        kind,
        NodeKind::Table
            | NodeKind::TableRow
            | NodeKind::TableCell
            | NodeKind::CodeBlock
            | NodeKind::List
            | NodeKind::ListItem
            | NodeKind::Quote
            | NodeKind::Embed
    )
}

/// 块的整体结构 kind(树根下第一层容器;无则 Paragraph):供调度器选默认风格。
pub fn block_kind(tree: &NodeTree) -> NodeKind {
    // 根 Doc 的首个非 Run/Glyph 子 = 该块结构;表格块根下含 Table。
    for kind in [
        NodeKind::Table,
        NodeKind::CodeBlock,
        NodeKind::List,
        NodeKind::Quote,
        NodeKind::Heading,
    ] {
        if tree.nodes_of_kind(kind).next().is_some() {
            return kind;
        }
    }
    NodeKind::Paragraph
}

// ───────────────────────── 8B:揭示风格 = 纯数据(0019 §4.2)─────────────────────────

/// 缓动标识(policy 引用;0016 morph 自带缓动,这里仅作数据携带)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EaseId {
    Linear,
    CubicOut,
}

/// 选择子(0019 §4.2):在 [0020 节点树](crate::nodes)上按 kind/区间解析为**glyph 区间集**或
/// **骨架**。`Cell(r,c)` 的 `u32::MAX` = 通配(任意行/列)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selector {
    /// 按节点 kind 选其全部 glyph(通用形态)。
    ByKind(NodeKind),
    /// 整表外框骨架(面板;无 glyph)。
    Frame,
    /// 网格线骨架(面板;无 glyph)。
    Grid,
    /// 表头行各 cell 的字。
    Header,
    /// 第 r 行 c 列 cell 的字(`u32::MAX` = 通配)。
    Cell(u32, u32),
    /// 第 n 行的字(`u32::MAX` = 所有数据行)。
    RowGlyphs(u32),
    /// 块内所有字。
    Glyphs,
}

/// stage 依赖边(0019 §4.2):相对哪个就绪点起算。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StageEdge {
    Start,
    End,
}

/// stage 的依赖(0019 §4.2 / §5 双门):内容门 / 布局门 / 兄弟 stage 边 / 立即。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dep {
    /// 内容完整到某级(揭哪些字)。
    ContentGate(RevealUnit),
    /// 几何稳定到某级(列宽定 = 可精确画框)。
    LayoutGate(RevealUnit),
    /// 兄弟 stage(下标)的起/止边。
    Stage(usize, StageEdge),
    /// 立即(无门)。
    Now,
}

/// 一个揭示 stage(0019 §4.2):选谁、依赖谁、延迟多少、时长、缓动。纯数据。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stage {
    pub select: Selector,
    pub after: Dep,
    pub offset_ms: f32,
    pub dur_ms: f32,
    pub ease: EaseId,
}

/// 一个揭示风格(0019 §4.2)= 就绪门 + 一组 stage(纯数据)。加风格 = 加一条,管线零改。
#[derive(Clone, Debug, PartialEq)]
pub struct RevealStyle {
    pub gate: RevealUnit,
    pub stages: Vec<Stage>,
}

/// 用户可切的表格揭示风格(0019 §2 配置表三行;调试面板下拉)。默认 [`TableStyleKind::Full`]
/// (作者最想要的"整表→网格→表头→cell")。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TableStyleKind {
    /// 风格 1:逐行 raw 跟随(逐字,无骨架)。
    Raw,
    /// 风格 2:每行框 → 填字。
    RowFrame,
    /// 风格 3(默认):整表 → 网格 → 表头 → 各 cell 并行填字。
    #[default]
    Full,
}

impl TableStyleKind {
    /// 数值 ↔ JS 下拉(0=Raw,1=RowFrame,2=Full)。
    pub fn from_u32(v: u32) -> Self {
        match v {
            0 => TableStyleKind::Raw,
            1 => TableStyleKind::RowFrame,
            _ => TableStyleKind::Full,
        }
    }
}

const D: f32 = 200.0; // 默认 stage 时长(ms)
const HEADER_OFFSET: f32 = 60.0; // 网格→表头延迟(0019 §4.2 风格 3 示意)
const SKELETON_OFFSET: f32 = 120.0; // 容器骨架→字延迟(骨架先行可见)

/// 三种内置表格风格 = 0019 §4.2 配置表三行(纯数据)。
pub fn table_style(kind: TableStyleKind) -> RevealStyle {
    match kind {
        // 风格 1:gate Glyph + 单 stage 逐字,无骨架(等价 0017 现状)。
        TableStyleKind::Raw => RevealStyle {
            gate: RevealUnit::Glyph,
            stages: vec![Stage {
                select: Selector::Glyphs,
                after: Dep::ContentGate(RevealUnit::Glyph),
                offset_ms: 0.0,
                dur_ms: D,
                ease: EaseId::CubicOut,
            }],
        },
        // 风格 2:gate Row;每行框(布局门)→ 该行字。
        TableStyleKind::RowFrame => RevealStyle {
            gate: RevealUnit::Row(0),
            stages: vec![
                Stage {
                    select: Selector::Frame,
                    after: Dep::LayoutGate(RevealUnit::Row(0)),
                    offset_ms: 0.0,
                    dur_ms: D,
                    ease: EaseId::CubicOut,
                },
                Stage {
                    select: Selector::RowGlyphs(u32::MAX),
                    after: Dep::Stage(0, StageEdge::End),
                    offset_ms: 0.0,
                    dur_ms: D,
                    ease: EaseId::CubicOut,
                },
            ],
        },
        // 风格 3:gate Block;网格 → 表头 → 各 cell(并行)。
        TableStyleKind::Full => RevealStyle {
            gate: RevealUnit::Block,
            stages: vec![
                Stage {
                    select: Selector::Grid,
                    after: Dep::LayoutGate(RevealUnit::Block),
                    offset_ms: 0.0,
                    dur_ms: D,
                    ease: EaseId::CubicOut,
                },
                Stage {
                    select: Selector::Header,
                    after: Dep::Stage(0, StageEdge::End),
                    offset_ms: HEADER_OFFSET,
                    dur_ms: D,
                    ease: EaseId::CubicOut,
                },
                Stage {
                    select: Selector::Cell(u32::MAX, u32::MAX),
                    after: Dep::Stage(1, StageEdge::End),
                    offset_ms: 0.0,
                    dur_ms: D,
                    ease: EaseId::CubicOut,
                },
            ],
        },
    }
}

/// 纯文本/标题/行内默认风格:逐字 fade(实时感,速度可调;等价现状,DoD #5)。
pub fn text_style() -> RevealStyle {
    RevealStyle {
        gate: RevealUnit::Glyph,
        stages: vec![Stage {
            select: Selector::Glyphs,
            after: Dep::ContentGate(RevealUnit::Glyph),
            offset_ms: 0.0,
            dur_ms: D,
            ease: EaseId::CubicOut,
        }],
    }
}

/// 非表格结构块(代码/列表/引用)默认风格:**骨架先行**——容器(底/条)先入场,字延后。
pub fn skeleton_style() -> RevealStyle {
    RevealStyle {
        gate: RevealUnit::Line,
        stages: vec![
            Stage {
                select: Selector::Frame,
                after: Dep::LayoutGate(RevealUnit::Glyph),
                offset_ms: 0.0,
                dur_ms: D,
                ease: EaseId::CubicOut,
            },
            Stage {
                select: Selector::Glyphs,
                after: Dep::Stage(0, StageEdge::End),
                offset_ms: SKELETON_OFFSET,
                dur_ms: D,
                ease: EaseId::CubicOut,
            },
        ],
    }
}

/// 选定块的揭示风格(0019 §4.2):表格用用户选的 3 风格之一;代码/列表/引用骨架先行;
/// 段落/标题/行内逐字。Selector 由 [`resolve`] 在节点树上落地。
pub fn style_for_block(tree: &NodeTree, table: TableStyleKind) -> RevealStyle {
    match block_kind(tree) {
        NodeKind::Table => table_style(table),
        NodeKind::CodeBlock | NodeKind::List | NodeKind::Quote => skeleton_style(),
        _ => text_style(),
    }
}

/// 风格在节点树上落地的结果(0019 §4.3 sched 的输入):每块内 glyph 的揭示层级 + 偏移。
/// `tier[g]`:揭示序(越小越早;`u32::MAX` = 未选中 = hold/不揭示)。`offset_ms[g]`:相对该
/// tier 起点的额外延迟(stagger)。`skeleton_first`:是否有骨架(Frame/Grid)stage 先于字。
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphPlan {
    pub tier: Vec<u32>,
    pub offset_ms: Vec<f32>,
    pub skeleton_first: bool,
}

impl GlyphPlan {
    /// 该 glyph 是否在本规划里被揭示(非 hold)。
    pub fn revealed(&self, g: usize) -> bool {
        self.tier.get(g).is_some_and(|&t| t != u32::MAX)
    }
}

/// 把一个 [`RevealStyle`] 在节点树上解析成逐 glyph 的揭示层级 + 偏移(0019 §4.2 Selector → 端点)。
/// stage 在 `style.stages` 里**已按依赖序**排列(built-in 风格保证),故 tier = stage 下标;后写
/// 覆盖前写(Header 先标表头字,Cell 通配再标其余 body 字,互不重叠)。表格行/cell 由 TableRow/
/// TableCell 节点的文档序定位。
pub fn resolve(style: &RevealStyle, tree: &NodeTree) -> GlyphPlan {
    let total = tree.root().map_or(0, |r| r.range.1) as usize;
    let mut tier = vec![u32::MAX; total];
    let mut offset_ms = vec![0.0f32; total];
    let mut skeleton_first = false;

    // 表格行(文档序)→ 各行的 cell 节点。
    let rows: Vec<u32> = tree
        .nodes_of_kind(NodeKind::TableRow)
        .map(|(i, _)| i)
        .collect();
    let cells_of = |row_idx: u32| -> Vec<(u32, u32)> {
        tree.children(row_idx)
            .filter(|&c| tree.nodes()[c as usize].kind == NodeKind::TableCell)
            .map(|c| tree.nodes()[c as usize].range)
            .collect()
    };
    let mark = |tier: &mut [u32], offset_ms: &mut [f32], range: (u32, u32), t: u32, off: f32| {
        for g in range.0..range.1 {
            if let (Some(tt), Some(oo)) = (tier.get_mut(g as usize), offset_ms.get_mut(g as usize))
            {
                *tt = t;
                *oo = off;
            }
        }
    };

    for (si, stage) in style.stages.iter().enumerate() {
        let t = si as u32;
        let off = stage.offset_ms;
        match stage.select {
            Selector::Frame | Selector::Grid => skeleton_first = true,
            Selector::Glyphs => {
                for g in 0..total {
                    tier[g] = t;
                    offset_ms[g] = off;
                }
            }
            Selector::ByKind(kind) => {
                for (_, n) in tree.nodes_of_kind(kind) {
                    mark(&mut tier, &mut offset_ms, n.range, t, off);
                }
            }
            Selector::Header => {
                if let Some(&r0) = rows.first() {
                    for cr in cells_of(r0) {
                        mark(&mut tier, &mut offset_ms, cr, t, off);
                    }
                }
            }
            Selector::Cell(rsel, csel) => {
                for (ri, &row) in rows.iter().enumerate() {
                    if rsel != u32::MAX && rsel != ri as u32 {
                        continue;
                    }
                    // 通配 body cell 时跳过表头行(已由 Header stage 标)。
                    if rsel == u32::MAX && ri == 0 {
                        continue;
                    }
                    for (ci, cr) in cells_of(row).into_iter().enumerate() {
                        if csel != u32::MAX && csel != ci as u32 {
                            continue;
                        }
                        mark(&mut tier, &mut offset_ms, cr, t, off);
                    }
                }
            }
            Selector::RowGlyphs(nsel) => {
                for (ri, &row) in rows.iter().enumerate() {
                    if nsel != u32::MAX && nsel != ri as u32 {
                        continue;
                    }
                    let rng = tree.nodes()[row as usize].range;
                    mark(&mut tier, &mut offset_ms, rng, t, off);
                }
            }
        }
    }
    GlyphPlan {
        tier,
        offset_ms,
        skeleton_first,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seam::TablePanel;

    fn table_tree() -> NodeTree {
        crate::content::parse_markdown_nodes("| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |", 0).2
    }

    #[test]
    fn reveal_unit_monotone_order() {
        assert!(RevealUnit::Block.rank() > RevealUnit::Row(9).rank());
        assert!(RevealUnit::Row(0).rank() > RevealUnit::Line.rank());
        assert!(RevealUnit::Line.rank() > RevealUnit::Glyph.rank());
        // Row 同级比行数。
        assert!(RevealUnit::Row(3).reached(RevealUnit::Row(2)));
        assert!(!RevealUnit::Row(1).reached(RevealUnit::Row(2)));
        // 跨级:Block 达到任意低门;Glyph 不达 Block。
        assert!(RevealUnit::Block.reached(RevealUnit::Row(5)));
        assert!(!RevealUnit::Glyph.reached(RevealUnit::Block));
    }

    #[test]
    fn layout_gate_needs_columns() {
        assert_eq!(layout_gate(&[]), RevealUnit::Glyph);
        let p = TablePanel {
            cols: vec![10.0, 20.0],
            w: 100.0,
            ..Default::default()
        };
        assert_eq!(layout_gate(&[p]), RevealUnit::Block);
    }

    #[test]
    fn structural_kinds_classified() {
        assert!(is_structural(NodeKind::Table));
        assert!(is_structural(NodeKind::CodeBlock));
        assert!(is_structural(NodeKind::List));
        assert!(!is_structural(NodeKind::Paragraph));
        assert!(!is_structural(NodeKind::Run));
    }

    #[test]
    fn three_table_styles_have_expected_stage_order() {
        // 风格 1:单 stage 逐字,无骨架。
        let raw = table_style(TableStyleKind::Raw);
        assert_eq!(raw.stages.len(), 1);
        assert_eq!(raw.stages[0].select, Selector::Glyphs);
        // 风格 2:框 → 行字。
        let rf = table_style(TableStyleKind::RowFrame);
        assert_eq!(rf.stages.len(), 2);
        assert_eq!(rf.stages[0].select, Selector::Frame);
        assert!(matches!(rf.stages[1].select, Selector::RowGlyphs(_)));
        assert_eq!(rf.stages[1].after, Dep::Stage(0, StageEdge::End));
        // 风格 3:网格 → 表头 → cell(链式依赖)。
        let full = table_style(TableStyleKind::Full);
        assert_eq!(full.stages.len(), 3);
        assert_eq!(full.stages[0].select, Selector::Grid);
        assert_eq!(full.stages[1].select, Selector::Header);
        assert!(matches!(full.stages[2].select, Selector::Cell(_, _)));
        assert_eq!(full.stages[2].after, Dep::Stage(1, StageEdge::End));
    }

    #[test]
    fn style_for_block_picks_by_kind() {
        let table = table_tree();
        assert_eq!(
            style_for_block(&table, TableStyleKind::Full).gate,
            RevealUnit::Block
        );
        let code = crate::content::parse_markdown_nodes("```\nx\n```", 0).2;
        assert_eq!(
            style_for_block(&code, TableStyleKind::Full),
            skeleton_style()
        );
        let para = crate::content::parse_markdown_nodes("just text", 0).2;
        assert_eq!(style_for_block(&para, TableStyleKind::Full), text_style());
    }

    #[test]
    fn resolve_full_table_header_before_body() {
        // 风格 3:Grid(骨架先行)→ Header(tier 1)→ body cell(tier 2)。
        let tree = table_tree();
        let plan = resolve(&table_style(TableStyleKind::Full), &tree);
        assert!(plan.skeleton_first, "整表风格应骨架先行");
        // 表头字 'A'/'B' 在 tier 1;数据字 '1'/'3' 在 tier 2(晚于表头)。
        // glyph 0 = 第一格表头 'A'。
        assert_eq!(plan.tier[0], 1, "表头 tier=1");
        // 找一个数据 cell glyph(tier 2)。
        assert!(plan.tier.contains(&2), "应有 body cell 在 tier 2(晚于表头)");
        // 表头偏移 = HEADER_OFFSET(>0,网格后)。
        assert!(plan.offset_ms[0] > 0.0, "表头有 offset(网格后)");
    }

    #[test]
    fn resolve_raw_table_all_tier_zero_no_skeleton() {
        let tree = table_tree();
        let plan = resolve(&table_style(TableStyleKind::Raw), &tree);
        assert!(!plan.skeleton_first, "raw 风格无骨架");
        assert!(plan.tier.iter().all(|&t| t == 0), "raw 全 tier 0 逐字");
    }

    #[test]
    fn resolve_text_style_all_revealed_tier_zero() {
        let tree = crate::content::parse_markdown_nodes("hello world", 0).2;
        let plan = resolve(&text_style(), &tree);
        assert!(!plan.skeleton_first);
        let total = tree.root().map_or(0, |r| r.range.1) as usize;
        assert!((0..total).all(|g| plan.revealed(g)), "纯文本全部揭示");
        assert!(plan.tier.iter().all(|&t| t == 0));
    }

    #[test]
    fn resolve_skeleton_glyphs_after_frame() {
        // 代码块骨架先行:Frame stage(tier 0,骨架)→ Glyphs(tier 1,带 offset)。
        let tree = crate::content::parse_markdown_nodes("```\nlet x=1;\n```", 0).2;
        let plan = resolve(&skeleton_style(), &tree);
        assert!(plan.skeleton_first, "代码块骨架先行");
        // 所有字在 tier 1(Frame stage 不占 glyph),带骨架偏移。
        let total = tree.root().map_or(0, |r| r.range.1) as usize;
        assert!(
            (0..total).all(|g| plan.tier[g] == 1),
            "字在骨架之后(tier 1)"
        );
        assert!(plan.offset_ms.iter().any(|&o| o > 0.0), "字有骨架偏移");
    }
}
