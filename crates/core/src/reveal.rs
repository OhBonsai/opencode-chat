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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::seam::TablePanel;

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
}
