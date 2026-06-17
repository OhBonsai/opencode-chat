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

// ───────────── 9A/9B:递归揭示 = 容器 ordering + 文档序 tier(Plan 9,替换 0019 §4.2 Selector 阶梯)─────────────
//
// Plan 8 的"全局 tier 阶梯 + 表格专用 Selector(Header/Cell/RowGlyphs)"已被本节的**嵌套集递归**
// 收编(0→1 不留并行旧路):tier = 顶层块**文档序**(块间自上而下、不抢位),块内时序全靠
// **每容器 ordering** 累加的 `delay_ms`,表格 3 风格 = Table/TableRow 的 ordering 预设。

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

// 编排时长常量(ms)。delay_ms 由这些沿递归累加,定 `spawn=释放时刻+delay` 的相对时序。
const SKELETON_LEAD: f32 = 120.0; // 容器骨架(代码底/引用条/公式框)先现 → 字延后
const GRID_LEAD: f32 = 200.0; // 整表网格骨架先现 → 表头/cell 延后
const CELL_GAP: f32 = 8.0; // 整表 cell 间极小间隔(近"并行",方案 B 真并行后续)
const ROW_FRAME_LEAD: f32 = 120.0; // 行框:每行框先现 → 该行字延后
const ROW_GAP: f32 = 80.0; // 行框:行与行之间隔
const ITEM_GAP: f32 = 60.0; // 列表项之间隔(逐项)

/// 容器对其**子节点**的揭示排程(Plan 9 §2;0019 §4.2 风格的泛化)。方案 A:子项按**文档序**逐个。
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Ordering {
    /// 子项按文档序逐个揭示,相邻隔 `gap_ms`(叶 = 逐字,gap 通常 0,靠配额/到达节奏逐字)。
    Sequential { gap_ms: f32 },
    /// 容器**骨架**(框/网格/底/左条)先现(占 `frame_ms` 前导),再按文档序排子项(隔 `gap_ms`)。
    SkeletonThenChildren { frame_ms: f32, gap_ms: f32 },
}

impl Ordering {
    fn gap_ms(self) -> f32 {
        match self {
            Ordering::Sequential { gap_ms } | Ordering::SkeletonThenChildren { gap_ms, .. } => {
                gap_ms
            }
        }
    }
    /// 骨架前导(无骨架 = 0)。
    fn frame_ms(self) -> f32 {
        match self {
            Ordering::SkeletonThenChildren { frame_ms, .. } => frame_ms,
            Ordering::Sequential { .. } => 0.0,
        }
    }
}

/// 每 NodeKind 的默认 ordering(Plan 9 §2 表);表格 3 风格 = Table/TableRow 的 ordering 预设(§3)。
/// `table` 仅影响 Table/TableRow/TableCell 子树。
pub fn ordering_for(kind: NodeKind, table: TableStyleKind) -> Ordering {
    match kind {
        // 根 / 段落 / 标题 / HTML:逐字,无骨架、无间隔(纯文本不回归,DoD #5)。
        NodeKind::Doc | NodeKind::Paragraph | NodeKind::Heading | NodeKind::HtmlBlock => {
            Ordering::Sequential { gap_ms: 0.0 }
        }
        // 列表:逐项(项间 gap);嵌套 List 递归 → 逐层逐项。
        NodeKind::List | NodeKind::ListItem => Ordering::Sequential { gap_ms: ITEM_GAP },
        // 代码 / 引用 / 公式:骨架(底/条/框)先现 → 字。
        NodeKind::CodeBlock | NodeKind::Quote | NodeKind::MathDisplay => {
            Ordering::SkeletonThenChildren {
                frame_ms: SKELETON_LEAD,
                gap_ms: 0.0,
            }
        }
        // 表格:按 3 风格预设(§3)。
        NodeKind::Table => match table {
            TableStyleKind::Raw => Ordering::Sequential { gap_ms: 0.0 },
            TableStyleKind::RowFrame => Ordering::Sequential { gap_ms: ROW_GAP },
            TableStyleKind::Full => Ordering::SkeletonThenChildren {
                frame_ms: GRID_LEAD,
                gap_ms: CELL_GAP,
            },
        },
        NodeKind::TableRow => match table {
            // 行框:每行先画框 → 该行字。整表/原始:行内 cell 顺序(整表近"并行"小 gap)。
            TableStyleKind::RowFrame => Ordering::SkeletonThenChildren {
                frame_ms: ROW_FRAME_LEAD,
                gap_ms: 0.0,
            },
            TableStyleKind::Full => Ordering::Sequential { gap_ms: CELL_GAP },
            TableStyleKind::Raw => Ordering::Sequential { gap_ms: 0.0 },
        },
        // cell / 其余容器:逐字。
        _ => Ordering::Sequential { gap_ms: 0.0 },
    }
}

/// 是否**无字块**(走 NodeSpawn,§2.6):整块/节点级淡入,无逐字 glyph 编排。
/// `ThematicBreak`(单零墨 Rule)与 `Embed`(图片/公式/mermaid 占位)目前归此类。
pub fn is_nodespawn(kind: NodeKind) -> bool {
    matches!(kind, NodeKind::ThematicBreak | NodeKind::Embed)
}

/// 递归揭示落地结果(Plan 9 §2 / 0019 §4.3 sched 输入):逐 glyph 的释放序 + 绝对延迟。
/// - `tier[g]`:**释放序 = 顶层块的文档序下标**(`u32::MAX` = hold/未揭)。块间据此严格自上而下、
///   互不抢位(根治"靠后块跳到靠前块之前");块内细序由 `g`(= 文档序 glyph 下标)兜底。
/// - `delay_ms[g]`:容器 ordering 沿递归累加的**绝对延迟**(骨架前导 + 逐项/逐行 gap);定
///   `spawn = 释放时刻 + delay`,使骨架→表头→body / 逐项 有序淡入(0016 morph 据此)。
/// - `skeleton_first`:子树含 `SkeletonThenChildren`(骨架先行)。
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphPlan {
    pub tier: Vec<u32>,
    pub delay_ms: Vec<f32>,
    pub skeleton_first: bool,
}

impl GlyphPlan {
    /// 该 glyph 是否在本规划里被揭示(非 hold)。
    pub fn revealed(&self, g: usize) -> bool {
        self.tier.get(g).is_some_and(|&t| t != u32::MAX)
    }
}

/// **递归揭示**(Plan 9 §2 替换 Plan 8 全局 tier/Selector):在 [0020 嵌套集节点树](crate::nodes)上
/// 自根 DFS——每容器按其 [`Ordering`] 排子节点(文档序),骨架先行的容器留前导延迟,叶(Run/Glyph)
/// 逐字。**tier = 顶层块文档序**(块间自上而下、不拖累);**delay_ms = 沿途累加的编排时序**。
/// `table` 选表格 3 风格预设。
///
/// **就绪门(9F §2.5,Full 表)**:`open_block` = 当前**仍在流入(未闭合)**的末块节点下标(无则
/// `None`)。整表骨架风格要求"整表闭合 + 列宽全定"才揭(g-table 整表),故当 `open_block` 正是一个
/// **Full 表格块**时,把它整体 hold(留 `tier=MAX`)直到它闭合(不再是 open_block:后续块到达 /
/// turn 收尾)——`spawn=max(门, 编排)` 的内容门下限。行框/原始不受此门(到行/到字即揭)。
/// 调度器([`crate::app::Engine::schedule`])用此函数。
pub fn resolve_tree(tree: &NodeTree, table: TableStyleKind, open_block: Option<u32>) -> GlyphPlan {
    let total = tree.root().map_or(0, |r| r.range.1) as usize;
    let mut tier = vec![u32::MAX; total];
    let mut delay_ms = vec![0.0f32; total];
    let mut skeleton_first = false;
    // 顶层块 = 根 Doc 直接子;每块一个 tier(= 文档序块下标),块内 delay 由递归编排。
    for (bi, b) in tree.children(0).enumerate() {
        // 9F 内容门:整表风格下,正在流入的 Full 表格 hold(等闭合)→ 跳过,glyph 留 MAX。
        if table == TableStyleKind::Full
            && open_block == Some(b)
            && tree.nodes()[b as usize].kind == NodeKind::Table
        {
            continue;
        }
        resolve_node(
            tree,
            b,
            table,
            bi as u32,
            0.0,
            &mut tier,
            &mut delay_ms,
            &mut skeleton_first,
        );
    }
    GlyphPlan {
        tier,
        delay_ms,
        skeleton_first,
    }
}

/// 递归排一个节点的揭示,返回其揭示**结束延迟**(供兄弟接续)。叶(Run/Glyph)按 `start_delay`
/// 标其区间所有 glyph;容器按 [`Ordering`]:骨架先行留前导,子节点按文档序递归、相邻加 gap。
/// 无字块(`is_nodespawn`)不标 glyph(NodeSpawn:面板/装饰按节点淡入,§2.6),仅占骨架前导。
#[allow(clippy::too_many_arguments)] // reason: 递归需树/节点/风格/块tier/游标/双输出/骨架旗,拆 struct 反绕
fn resolve_node(
    tree: &NodeTree,
    idx: u32,
    table: TableStyleKind,
    block_tier: u32,
    start_delay: f32,
    tier: &mut [u32],
    delay_ms: &mut [f32],
    skeleton_first: &mut bool,
) -> f32 {
    let node = tree.nodes()[idx as usize];
    // 叶:逐字标该区间(tier=块文档序,delay=当前游标)。
    if matches!(node.kind, NodeKind::Run | NodeKind::Glyph) {
        for g in node.range.0..node.range.1 {
            if let (Some(tt), Some(dd)) = (tier.get_mut(g as usize), delay_ms.get_mut(g as usize)) {
                *tt = block_tier;
                *dd = start_delay;
            }
        }
        return start_delay;
    }
    // 无字块(分隔线/Embed):NodeSpawn,不逐字;占位返回(装饰/面板按节点淡入)。
    if is_nodespawn(node.kind) {
        return start_delay;
    }
    let ordering = ordering_for(node.kind, table);
    let mut delay = start_delay;
    if ordering.frame_ms() > 0.0 {
        *skeleton_first = true;
        delay += ordering.frame_ms(); // 骨架先行:子项延后
    }
    let gap = ordering.gap_ms();
    // 子节点按文档序(append-only build ⇒ children 已按 range.start);递归。
    let children: Vec<u32> = tree.children(idx).collect();
    for child in children {
        delay = resolve_node(
            tree,
            child,
            table,
            block_tier,
            delay,
            tier,
            delay_ms,
            skeleton_first,
        );
        delay += gap;
    }
    delay
}

// ───────────────────────── 8C:揭示调度器 = 解耦的揭示时钟(0019 §4.3 + 北极星)─────────────────────────

/// 默认揭示速率(display glyph/秒):`INFINITY` = **跟内容到达**(不限速),等价 0017 现状的逐字
/// (DoD #5:纯文本不回归)。调慢到有限值即限速;放慢因子再乘,刻意拉慢让揭示被看见。
pub const DEFAULT_REVEAL_CPS: f32 = f32::INFINITY;

/// `reveal_cps` 不限速(默认)但用户**只调了放慢因子**(`slow < 1`,北极星"刻意放慢")时的
/// 基准速率(display glyph/秒)。否则 `INFINITY * slow = INFINITY` → 放慢无效(速度档点了没反应)。
/// 取一个"看得清揭示过程"的基准;`正常`(slow==1)仍走不限速,纯文本不回归。
const SLOW_BASE_CPS: f32 = 80.0;

/// 揭示调度器(0019 §4.3 sched):**唯一**揭示路径——收编"grapheme 到达即 `spawn_time=now`"的
/// 即时揭示(0017),改由调度器按**自有时钟**(注入 `dt_ms`,可重放)释放 glyph、产 `spawn_time`。
///
/// 与 smoother 分工(plan8 §8C):smoother 管"grapheme 到达 = 内容真值"(整流 token 突发);
/// 调度器管"何时上屏 = 呈现"(限速 / 放慢 / 骨架先行)。二者串联,不重叠。
///
/// 本结构只持**时钟 + 参数**(限速积分);逐 view 的释放进度由调用方(Engine)持有,因释放要按
/// [`resolve`] 的 tier/offset 在节点树上落地(借用 view 缓存)。
#[derive(Clone, Debug)]
pub struct RevealScheduler {
    /// 揭示速率上限(display glyph/秒);`INFINITY` = 跟内容到达。
    reveal_cps: f32,
    /// 放慢因子(1.0 正常,<1 更慢;0019 北极星"刻意放慢")。
    slow: f32,
    /// 当前表格揭示风格(用户可切;非表格块用 text/skeleton 默认)。
    table_style: TableStyleKind,
    /// 限速积分余量(攒够 1 个才释放一个;`INFINITY` 时不用)。
    budget: f32,
}

impl Default for RevealScheduler {
    fn default() -> Self {
        Self {
            reveal_cps: DEFAULT_REVEAL_CPS,
            slow: 1.0,
            table_style: TableStyleKind::default(),
            budget: 0.0,
        }
    }
}

impl RevealScheduler {
    pub fn new() -> Self {
        Self::default()
    }

    /// 设揭示速率上限(glyph/秒);≤0 或非有限 → 不限速(跟内容到达)。
    pub fn set_reveal_cps(&mut self, cps: f32) {
        self.reveal_cps = if cps > 0.0 { cps } else { DEFAULT_REVEAL_CPS };
        self.budget = 0.0;
    }

    /// 设放慢因子(夹到 `[0.01, 1.0]`;越小越慢)。
    pub fn set_slow(&mut self, slow: f32) {
        self.slow = slow.clamp(0.01, 1.0);
        self.budget = 0.0;
    }

    /// 设表格揭示风格(3 风格切换)。
    pub fn set_table_style(&mut self, k: TableStyleKind) {
        self.table_style = k;
    }

    pub fn table_style(&self) -> TableStyleKind {
        self.table_style
    }

    /// 是否限速:`reveal_cps` 有限,**或**仅放慢(`slow < 1`)——后者用 [`SLOW_BASE_CPS`] 作基准,
    /// 否则不限速时放慢因子被 `INFINITY` 吞掉、速度档无效。
    pub fn is_rate_limited(&self) -> bool {
        self.reveal_cps.is_finite() || self.slow < 1.0
    }

    /// 当前生效基准速率(glyph/秒):限速取 `reveal_cps`;仅放慢则取 [`SLOW_BASE_CPS`]。
    fn base_cps(&self) -> f32 {
        if self.reveal_cps.is_finite() {
            self.reveal_cps
        } else {
            SLOW_BASE_CPS
        }
    }

    /// 推进时钟 `dt_ms`,累加限速预算(不限速时空操作)。确定性:同 dt 序列 → 同预算(R8/R9)。
    pub fn advance_clock(&mut self, dt_ms: f64) {
        if self.is_rate_limited() {
            let rate = self.base_cps() * self.slow;
            self.budget += rate * (dt_ms as f32) / 1000.0;
            // 限速突发上限:攒够约 0.25s 即封顶,空转后不一次倾泻(同 smoother 精神)。
            let cap = (rate * 0.25).max(1.0);
            self.budget = self.budget.min(cap);
        }
    }

    /// 本帧可释放的 glyph 配额(整数);不限速 → `usize::MAX`。
    pub fn quota(&self) -> usize {
        if self.is_rate_limited() {
            self.budget.max(0.0) as usize
        } else {
            usize::MAX
        }
    }

    /// 消费 `k` 个释放配额(限速时扣预算)。
    pub fn consume(&mut self, k: usize) {
        if self.is_rate_limited() {
            self.budget = (self.budget - k as f32).max(0.0);
        }
    }

    /// 本帧未释放任何 glyph(无内容可揭)→ 清零预算,避免空转后突发(同 smoother)。
    pub fn idle_reset(&mut self) {
        if self.is_rate_limited() {
            self.budget = 0.0;
        }
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

    // (tree, display clusters) —— 便于按字符定位 glyph 下标查 delay/tier。
    fn tree_and_clusters(src: &str) -> (NodeTree, Vec<String>) {
        let (spans, _t, tree) = crate::content::parse_markdown_nodes(src, 0);
        let mut clusters = Vec::new();
        for s in &spans {
            for g in crate::support::graphemes(s.text()) {
                clusters.push(g.to_owned());
            }
        }
        (tree, clusters)
    }

    #[test]
    fn ordering_defaults_by_kind() {
        // 9B:每 NodeKind 默认 ordering(方案 A)。
        let t = TableStyleKind::Full;
        assert_eq!(
            ordering_for(NodeKind::Paragraph, t),
            Ordering::Sequential { gap_ms: 0.0 }
        );
        assert!(matches!(
            ordering_for(NodeKind::List, t),
            Ordering::Sequential { gap_ms } if gap_ms > 0.0
        ));
        assert!(matches!(
            ordering_for(NodeKind::CodeBlock, t),
            Ordering::SkeletonThenChildren { .. }
        ));
        // 表格 3 预设:Raw=Sequential 无 gap、RowFrame=行 gap、Full=骨架先行。
        assert_eq!(
            ordering_for(NodeKind::Table, TableStyleKind::Raw),
            Ordering::Sequential { gap_ms: 0.0 }
        );
        assert!(matches!(
            ordering_for(NodeKind::Table, TableStyleKind::Full),
            Ordering::SkeletonThenChildren { .. }
        ));
        // 行框:TableRow 骨架先行(每行框);整表:TableRow 顺序。
        assert!(matches!(
            ordering_for(NodeKind::TableRow, TableStyleKind::RowFrame),
            Ordering::SkeletonThenChildren { .. }
        ));
        assert!(matches!(
            ordering_for(NodeKind::TableRow, TableStyleKind::Full),
            Ordering::Sequential { .. }
        ));
        // 无字块走 NodeSpawn。
        assert!(is_nodespawn(NodeKind::ThematicBreak));
        assert!(is_nodespawn(NodeKind::Embed));
        assert!(!is_nodespawn(NodeKind::Paragraph));
    }

    #[test]
    fn resolve_tree_tiers_are_document_order_no_cross_block_jump() {
        // 9A DoD #1:tier = 顶层块文档序 → 靠后块 tier 更大,绝不抢到靠前块之前。
        let (tree, clusters) = tree_and_clusters("# 一\n\npara\n\n## 二");
        let plan = resolve_tree(&tree, TableStyleKind::Full, None);
        let idx = |c: &str| clusters.iter().position(|x| x == c).expect("char");
        // 标题"一"(块0)< 段落"p"(块1)< 标题"二"(块2)。
        let t1 = plan.tier[idx("一")];
        let tp = plan.tier[idx("p")];
        let t2 = plan.tier[idx("二")];
        assert!(t1 < tp && tp < t2, "tier 应严格文档序: {t1} {tp} {t2}");
        // 全部揭示(无 hold)。
        assert!(
            (0..clusters.len()).all(|g| plan.revealed(g) || clusters[g] == "\n"),
            "所有非换行字应揭示(无连坐 hold)"
        );
    }

    #[test]
    fn resolve_tree_full_table_skeleton_header_before_body() {
        // 9B/§3 整表:骨架先行;表头(文档序在前)delay < body。
        let tree = table_tree();
        let plan = resolve_tree(&tree, TableStyleKind::Full, None);
        assert!(plan.skeleton_first, "整表风格应骨架先行");
        // glyph 0 = 表头首格 'A';其 delay = 网格前导(>0)。
        assert!(plan.delay_ms[0] >= GRID_LEAD, "表头在网格骨架之后");
        // body(后续行)delay 严格大于表头。
        let max_delay = plan.delay_ms.iter().copied().fold(0.0_f32, f32::max);
        assert!(max_delay > plan.delay_ms[0], "body cell 晚于表头");
        // 单块表格 → 已揭字 tier 全相同(同一顶层块;换行零墨 hold 不计)。
        assert!(
            plan.tier
                .iter()
                .filter(|&&t| t != u32::MAX)
                .all(|&t| t == 0),
            "表格是单顶层块 → 已揭字 tier 全 0"
        );
    }

    #[test]
    fn resolve_tree_full_table_held_while_open() {
        // 9F 内容门:整表风格下,正在流入(open_block)的 Full 表格整体 hold(等闭合);
        // 闭合后(open_block=None)才揭。行框不受此门。
        let tree = table_tree();
        let table_block = tree.children(0).next().expect("table block idx");
        let open = resolve_tree(&tree, TableStyleKind::Full, Some(table_block));
        assert!(
            (0..open.tier.len()).all(|g| !open.revealed(g)),
            "流入中的整表应整体 hold(等闭合)"
        );
        let closed = resolve_tree(&tree, TableStyleKind::Full, None);
        assert!(
            (0..closed.tier.len()).any(|g| closed.revealed(g)),
            "闭合后整表应揭示"
        );
        let rowframe = resolve_tree(&tree, TableStyleKind::RowFrame, Some(table_block));
        assert!(
            (0..rowframe.tier.len()).any(|g| rowframe.revealed(g)),
            "行框不等整表闭合(到行即揭)"
        );
    }

    #[test]
    fn resolve_tree_raw_table_no_skeleton_zero_delay() {
        let tree = table_tree();
        let plan = resolve_tree(&tree, TableStyleKind::Raw, None);
        assert!(!plan.skeleton_first, "raw 风格无骨架");
        assert!(
            plan.delay_ms.iter().all(|&d| d == 0.0),
            "raw 全零延迟(逐字)"
        );
    }

    #[test]
    fn resolve_tree_text_all_revealed_zero_delay() {
        let tree = crate::content::parse_markdown_nodes("hello world", 0).2;
        let plan = resolve_tree(&tree, TableStyleKind::Full, None);
        assert!(!plan.skeleton_first);
        let total = tree.root().map_or(0, |r| r.range.1) as usize;
        assert!((0..total).all(|g| plan.revealed(g)), "纯文本全部揭示");
        assert!(
            plan.delay_ms.iter().all(|&d| d == 0.0),
            "纯文本零编排延迟(不回归)"
        );
    }

    #[test]
    fn resolve_tree_nested_list_depth_first_doc_order() {
        // 9D:嵌套列表逐项,深度优先文档序;a < a1(嵌套子项)< b。
        let (tree, clusters) = tree_and_clusters("- a\n  - a1\n- b");
        let plan = resolve_tree(&tree, TableStyleKind::Full, None);
        let d = |c: &str| plan.delay_ms[clusters.iter().position(|x| x == c).expect("char")];
        assert!(d("a") < d("b"), "项 a 应早于项 b: {} {}", d("a"), d("b"));
        // 'a1' 的 '1'(嵌套子项)在 a 之后、b 之前(深度优先)。
        let a1 = plan.delay_ms[clusters.iter().position(|x| x == "1").expect("a1")];
        assert!(
            d("a") < a1 && a1 < d("b"),
            "嵌套项应深度优先夹在 a 与 b 之间"
        );
    }

    #[test]
    fn scheduler_unlimited_by_default() {
        let mut s = RevealScheduler::new();
        assert!(!s.is_rate_limited(), "默认不限速(跟内容到达)");
        s.advance_clock(16.0);
        assert_eq!(s.quota(), usize::MAX, "不限速 → 无限配额");
    }

    #[test]
    fn scheduler_rate_limits_and_is_deterministic() {
        // 限速 100 glyph/s → 100ms 约 10 个配额(封顶 0.25s = 25)。
        let run = || {
            let mut s = RevealScheduler::new();
            s.set_reveal_cps(100.0);
            let mut released = 0usize;
            let mut t = 0.0;
            for _ in 0..10 {
                t += 16.0;
                s.advance_clock(16.0);
                let q = s.quota();
                s.consume(q);
                released += q;
            }
            (released, t)
        };
        let (a, _) = run();
        let (b, _) = run();
        assert_eq!(a, b, "同 dt 序列 → 同释放数(确定性 R8/R9)");
        // 160ms * 100cps = 16 个(封顶 25 内),受 reveal_cps 约束,远少于不限速。
        assert!((10..=25).contains(&a), "限速配额受 reveal_cps 约束: {a}");
    }

    #[test]
    fn scheduler_slow_factor_reduces_quota() {
        let count = |slow: f32| {
            let mut s = RevealScheduler::new();
            s.set_reveal_cps(100.0);
            s.set_slow(slow);
            let mut released = 0;
            for _ in 0..20 {
                s.advance_clock(16.0);
                let q = s.quota();
                s.consume(q);
                released += q;
            }
            released
        };
        let fast = count(1.0);
        let slow = count(0.1);
        assert!(
            slow < fast,
            "放慢 0.1× 应显著少于正常: slow={slow} fast={fast}"
        );
    }

    #[test]
    fn slow_alone_engages_rate_limit() {
        // 回归:默认 reveal_cps=∞ 时,只调放慢因子也应限速(否则 ∞×slow=∞,速度档点了没反应)。
        let mut s = RevealScheduler::new();
        assert!(!s.is_rate_limited(), "默认正常速 → 不限速");
        s.set_slow(0.1);
        assert!(
            s.is_rate_limited(),
            "放慢 0.1× → 应启用限速(SLOW_BASE_CPS 基准)"
        );
        s.advance_clock(100.0);
        // SLOW_BASE_CPS(80)×0.1×0.1s = 0.8 → 本帧 0 个(攒着),证明确实被放慢。
        assert!(s.quota() < 5, "极慢档配额应很小: {}", s.quota());
        // 恢复正常 → 回到不限速。
        s.set_slow(1.0);
        assert!(!s.is_rate_limited(), "正常速 → 不限速(纯文本不回归)");
    }

    #[test]
    fn resolve_tree_skeleton_code_chars_delayed() {
        // 代码块骨架先行:底先现(SKELETON_LEAD 前导)→ 码字带延迟。
        let tree = crate::content::parse_markdown_nodes("```\nlet x=1;\n```", 0).2;
        let plan = resolve_tree(&tree, TableStyleKind::Full, None);
        assert!(plan.skeleton_first, "代码块骨架先行");
        // 码字延迟 ≥ 骨架前导(底/框先于字)。
        assert!(
            plan.delay_ms.iter().any(|&d| d >= SKELETON_LEAD),
            "码字应带骨架延迟(底先于字)"
        );
    }
}
