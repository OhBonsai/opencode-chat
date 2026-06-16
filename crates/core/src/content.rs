//! content(M6)— markdown 语义化(Plan2 H,plan2 H1)。
//!
//! 解析交给 vendored 的 **jcode-render-core**(后端中立文档模型:`parse_markdown -> Document`,
//! 含标题/粗斜/行内与块代码/列表/引用/**表格**/数学等);本文件把它的 `Document` 适配成我们
//! 渲染管线的 [`StyledSpan`] + [`StyleRole`](决定字体/上色,render 侧按 `as_u32` 分桶取色)。
//!
//! 流式不闪烁:[`remend`] 在解析前对尾部"主动补全"未闭合的 `**`/`` ` ``/```` ``` ````
//! (0004 §5.1),避免半截语法字符瞬间字面显形(AR9 同族)。

use jcode_render_core::{
    parse_markdown as jcode_parse, Alignment, Block, BlockKind, StyleRole as JRole,
    StyledSpan as JSpan, TextAttrs,
};

/// 样式角色(content→layout/render 契约,architecture §五.3)。role 决定字体(粗/斜/等宽)
/// 与上色;render 侧 atlas 按 (role, cluster) 分桶,glyph.wgsl 按 role 取色。数值稳定。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum StyleRole {
    #[default]
    Normal,
    Bold,
    Italic,
    BoldItalic,
    /// 行内代码。
    Code,
    /// 代码块内文本。
    CodeBlock,
    /// 标题(H1–H6 合并为一类,Plan2 不分级)。
    Heading,
    /// 链接文字。
    Link,
    /// 引用块 / 弱化文本(reasoning)。
    Quote,
    /// 列表/表格标记(dim)。
    ListMarker,
    /// 标题 H2–H6 分级(H1 = [`StyleRole::Heading`]);逐级字号在 layout/raster 侧按角色取。
    Heading2,
    Heading3,
    Heading4,
    Heading5,
    Heading6,
    /// 分隔线(`---`)锚点:发一个零墨空格,render 侧据此画整宽细线 rect(4B1)。
    Rule,
    /// GitHub Alert 标签行(`[!NOTE]` 等);承载告警类型,render 侧据此上色左条(4B1)。
    AlertLabel,
    /// 表格单元格(等宽对齐,0014 A);layout 用等宽字体 → 列对齐。
    TableCell,
    /// 表格表头单元格(等宽 + 表头色;render 侧据此画表头底 + 分隔线)。
    TableHeader,
    /// 表格体内强调(粗/链接文字):**等宽加粗**保列对齐 + 区分(0014 A,5E.1 #2)。
    TableStrong,
    /// 表格体内斜体:**等宽斜体**保列对齐(与 TableStrong 区分,5E.1 #2)。
    TableEm,
    /// 表格列分隔符(`│`):render 侧据其 x 画**全表高竖直网格线**(5E.1 #5);等宽 + 弱化色。
    TableSep,
}

impl StyleRole {
    /// 给 render/atlas 用的稳定数值。
    pub fn as_u32(self) -> u32 {
        self as u32
    }

    /// markdown 标题级别(1–6)→ 角色。H1 复用 `Heading`,H2–H6 分级(4A3)。
    pub fn heading(level: u8) -> StyleRole {
        match level {
            0 | 1 => StyleRole::Heading,
            2 => StyleRole::Heading2,
            3 => StyleRole::Heading3,
            4 => StyleRole::Heading4,
            5 => StyleRole::Heading5,
            _ => StyleRole::Heading6,
        }
    }

    /// 是否标题角色(任意级别)。
    pub fn is_heading(self) -> bool {
        matches!(
            self,
            StyleRole::Heading
                | StyleRole::Heading2
                | StyleRole::Heading3
                | StyleRole::Heading4
                | StyleRole::Heading5
                | StyleRole::Heading6
        )
    }
}

/// 一段带样式角色的文本 run(content→layout)。`strike` = 删除线(与 role 正交:粗/斜可叠删除线;
/// 删除线是**装饰**不是字体,render 侧在字中线画一条 → 不入 layout,故 layout 忽略此字段)。
#[derive(Clone, Debug, PartialEq)]
pub struct StyledSpan {
    text: String,
    role: StyleRole,
    strike: bool,
}

impl StyledSpan {
    pub fn new(text: impl Into<String>, role: StyleRole) -> Self {
        Self {
            text: text.into(),
            role,
            strike: false,
        }
    }

    /// 带删除线的 run(`~~…~~`)。
    pub fn styled(text: impl Into<String>, role: StyleRole, strike: bool) -> Self {
        Self {
            text: text.into(),
            role,
            strike,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn role(&self) -> StyleRole {
        self.role
    }

    /// 是否删除线(render 在字中线画线;`~~…~~`)。
    pub fn is_struck(&self) -> bool {
        self.strike
    }
}

/// 纯文本直通:整段文本 → 单个 Normal span(给非 markdown 路径/测试用)。
pub fn plain(text: &str) -> Vec<StyledSpan> {
    if text.is_empty() {
        Vec::new()
    } else {
        vec![StyledSpan::new(text, StyleRole::Normal)]
    }
}

/// 末块(活动块)是否「正在成形的表格」:还是 Paragraph,但每个非空行 trim 后以 `|` 开头
/// —— 即表头/数据行已到、分隔行 `|---|` 未到,pulldown 尚未确认成 `Table`。
/// 用于 reveal 抑制:这种态**别显示 raw `| a | b |`**(0017 §10 / thinking §3,避免 raw→snap 跳变)。
fn is_pending_table(block: &Block) -> bool {
    if !matches!(block.kind, BlockKind::Paragraph) {
        return false;
    }
    let mut any = false;
    for line in &block.lines {
        let t = line.plain_text();
        let t = t.trim();
        if t.is_empty() {
            continue;
        }
        any = true;
        if !t.starts_with('|') {
            return false;
        }
    }
    any
}

/// 末块是否「正在成形的结构块」—— 不止表格(0019 §4.2 / 0017 §10:把 [`is_pending_table`]
/// 泛化到更多会闪 raw 的结构)。命中即 reveal 抑制(hold 整块,不发 raw),待结构确认再揭示:
/// - **表格**:表头/数据行已到、分隔行 `|---|` 未到([`is_pending_table`])。
/// - **显示公式** `$$…$$`:开了未闭(奇数个 `$$`)→ 别闪半截 `$$E=mc^2`。
///
/// 保守(0019 风险"raw 抑制误伤"):列表 `- ` / 围栏 ``` ``` ``` 由 marker / [`remend`] 闭合已不闪 raw,
/// 不纳入抑制(否则正常 `- 文本`/代码会被误 hold)。
fn is_pending_structure(block: &Block) -> bool {
    if is_pending_table(block) {
        return true;
    }
    // 显示公式半截:段落且整块文本含奇数个 `$$`(开了未闭)。
    if matches!(block.kind, BlockKind::Paragraph) {
        let dollars: usize = block
            .lines
            .iter()
            .map(|l| l.plain_text().matches("$$").count())
            .sum();
        if !dollars.is_multiple_of(2) {
            return true;
        }
    }
    false
}

/// 活动块(末块)内容就绪门(0019 §4.1):该块结构上已完成到哪一级,驱动调度器何时可揭示。
/// 单调只增(append-only);**泛化** [`is_pending_table`](只答是否成形)为"成形到第几级":
/// - 段落/标题:逐字 → [`RevealUnit::Glyph`]。
/// - 列表项:闭合到行 → [`RevealUnit::Line`]。
/// - 围栏代码:源 ``` ``` ``` 配平(闭合)→ [`RevealUnit::Block`];未闭 → [`RevealUnit::Line`](已到行可逐行)。
/// - 表格:确认成 `Table` → [`RevealUnit::Row`]`(行数)`;成形中(未确认)由 [`is_pending_structure`] 抑制。
pub fn content_gate(src: &str) -> crate::reveal::RevealUnit {
    use crate::reveal::RevealUnit;
    let patched = remend(&mark_alerts(src));
    let doc = jcode_parse(&patched);
    match doc.blocks.last() {
        None => RevealUnit::Block,
        Some(b) => match &b.kind {
            BlockKind::Table => RevealUnit::Row(b.table.len() as u32),
            // 源里 ``` 配平 = 围栏闭合(remend 只在奇数时补;故偶数=已闭)。
            BlockKind::CodeBlock { .. } => {
                if src.matches("```").count().is_multiple_of(2) {
                    RevealUnit::Block
                } else {
                    RevealUnit::Line
                }
            }
            BlockKind::ListItem { .. } => RevealUnit::Line,
            _ => RevealUnit::Glyph,
        },
    }
}

/// 表格结构(0014 B / plan5 §5F):`rows[r][c]` = 该格在 spans 数组里的 run 区间 `[start, end)`;
/// `aligns[c]` = 列对齐(0=Left / 1=Center / 2=Right,与 JS 一致)。供 JS 像素两趟布局/格内折行。
#[derive(Clone, Debug, PartialEq)]
pub struct TableRegion {
    pub rows: Vec<Vec<(u32, u32)>>,
    pub aligns: Vec<u8>,
}

/// 解析 markdown 源 → 带角色的 span 序列。块/行间以 `\n` 分隔(零宽换行由 layout 处理)。
pub fn parse_markdown(src: &str) -> Vec<StyledSpan> {
    emit_doc(src).0
}

/// 解析 markdown → `(spans, 表格结构, 内容节点树)`(0014 B / 0020 / Plan 7)。**单源**:表格结构
/// (`TableRegion`,plan5 §5F:cell run 区间 + 列对齐,喂 layout 像素两趟)与**内容节点树**(身份
/// 地基)一并产出——不再留 spans-only-with-tables 的并行 API(0→1 单源准则)。`block_seq` = 该 part
/// 稳定序号(打进节点 key 高 32);节点 `range` 是块内 glyph(grapheme)下标。
pub fn parse_markdown_nodes(
    src: &str,
    block_seq: u32,
) -> (Vec<StyledSpan>, Vec<TableRegion>, crate::nodes::NodeTree) {
    let (spans, tables, specs) = emit_doc(src);
    // span k → 首 grapheme 下标的前缀和(与 app `ensure_layouts` 的 grapheme 展开同序)。
    let mut prefix = Vec::with_capacity(spans.len() + 1);
    let mut acc = 0u32;
    prefix.push(0);
    for s in &spans {
        acc += crate::support::graphemes(s.text()).len() as u32;
        prefix.push(acc);
    }
    let tree = crate::nodes::build(block_seq, &prefix, &specs);
    (spans, tables, tree)
}

/// `jcode` 块类型 → 节点 kind(0020)。
fn block_node_kind(block: &Block) -> crate::nodes::NodeKind {
    use crate::nodes::NodeKind;
    match &block.kind {
        BlockKind::Heading { .. } => NodeKind::Heading,
        BlockKind::CodeBlock { .. } => NodeKind::CodeBlock,
        BlockKind::BlockQuote => NodeKind::Quote,
        BlockKind::ListItem { .. } => NodeKind::ListItem,
        BlockKind::Table => NodeKind::Table,
        _ => NodeKind::Paragraph, // Paragraph / ThematicBreak / MathDisplay / Html
    }
}

/// `ListItem` 嵌套深度(其余块 = 0)。
fn block_depth(block: &Block) -> u32 {
    match &block.kind {
        BlockKind::ListItem { depth, .. } => *depth as u32,
        _ => 0,
    }
}

#[allow(clippy::type_complexity)] // reason: spans + 表格 + 块规格三件套,拆 struct 反而绕
fn emit_doc(
    src: &str,
) -> (
    Vec<StyledSpan>,
    Vec<TableRegion>,
    Vec<crate::nodes::BlockSpec>,
) {
    let patched = remend(&mark_alerts(src));
    let doc = jcode_parse(&patched);
    let mut out: Vec<StyledSpan> = Vec::new();
    let mut tables: Vec<TableRegion> = Vec::new();
    let mut specs: Vec<crate::nodes::BlockSpec> = Vec::new();
    let last = doc.blocks.len().wrapping_sub(1);
    for (i, block) in doc.blocks.iter().enumerate() {
        // reveal 抑制(0017 §10 / 0019 §4.2):末块若是"正在成形的结构块"(表格未确认 /
        // 半截显示公式)则 hold——不发 raw,待结构确认后再揭示。避免 `| a | b |`/`$$x` 闪现后 snap。
        if i == last && is_pending_structure(block) {
            continue;
        }
        // 0020:块区间从**前导块间 `\n` 之前**起 → 块连续无空洞,分隔符不成 Doc 孤儿(保不变式)。
        let span_start = out.len() as u32;
        if !out.is_empty() {
            out.push(StyledSpan::new("\n", StyleRole::Normal)); // 块间换行
        }
        let table_before = tables.len();
        emit_block(&mut out, block, &mut tables);
        let table = tables.get(table_before).map(|r| r.rows.clone());
        specs.push(crate::nodes::BlockSpec {
            kind: block_node_kind(block),
            depth: block_depth(block),
            spans: (span_start, out.len() as u32),
            table,
        });
    }
    (out, tables, specs)
}

/// GitHub Alert 标记(`[!NOTE]` 等,大小写不敏感)→ 显示标签;非告警返回 None。
///
/// 注:pulldown-cmark 把 `[!TYPE]` 当链接解析,方括号被吞,首行 `plain_text` 实为 `!TYPE`;
/// 故这里宽松剥掉两端 `[` `]` 再匹配,兼容两种形态。
fn alert_kind(line: &str) -> Option<&'static str> {
    let t = line.trim().trim_start_matches('[').trim_end_matches(']');
    match t.to_ascii_uppercase().as_str() {
        "!NOTE" => Some("NOTE"),
        "!TIP" => Some("TIP"),
        "!IMPORTANT" => Some("IMPORTANT"),
        "!WARNING" => Some("WARNING"),
        "!CAUTION" => Some("CAUTION"),
        _ => None,
    }
}

/// 私用区哨兵:`mark_alerts` 把 `[!TYPE]` 标记换成 `\u{E000}TYPE`,使其穿过 jcode(其底层
/// pulldown 无 GFM alert 扩展、会吞掉 `[!TYPE]`)后仍以纯文本存活,供 `emit_blockquote` 识别。
const ALERT_SENTINEL: char = '\u{E000}';

/// 预处理原文:把独占一行的 `> [!TYPE]` 的标记替换为 `\u{E000}TYPE` 哨兵(见 [`ALERT_SENTINEL`])。
fn mark_alerts(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    for (i, line) in src.split('\n').enumerate() {
        if i > 0 {
            out.push('\n');
        }
        if let Some(gt) = line.find('>') {
            if line[..gt].trim().is_empty() {
                if let Some(label) = alert_kind(line[gt + 1..].trim()) {
                    out.push_str(&line[..=gt]); // 保留前导缩进与 '>'
                    out.push(' ');
                    out.push(ALERT_SENTINEL);
                    out.push_str(label);
                    continue;
                }
            }
        }
        out.push_str(line);
    }
    out
}

/// jcode 给每个引用行前置 `"│ "`(Dim)栏线;我们改画矩形左条,故剔除这些字面栏线。
fn is_gutter(span: &JSpan) -> bool {
    matches!(span.role, JRole::Dim) && span.text.contains('│')
}

/// 引用块展平:GitHub Alert(哨兵首行)→ 标签行 + 引用体;普通引用 → 全 Quote 角色;
/// 两者都剔除 jcode 的 `│ ` 栏线(改由 render 画矩形左条)。
fn emit_blockquote(out: &mut Vec<StyledSpan>, block: &Block) {
    // 首行若含告警哨兵 → 取类型作 AlertLabel,余下行作引用体。
    let label = block.lines.first().and_then(|l| {
        l.spans
            .iter()
            .find_map(|s| s.text.strip_prefix(ALERT_SENTINEL))
            .map(str::to_string)
    });
    if let Some(label) = label {
        out.push(StyledSpan::new(label, StyleRole::AlertLabel));
        for line in block.lines.iter().skip(1) {
            out.push(StyledSpan::new("\n", StyleRole::Normal));
            for span in line.spans.iter().filter(|s| !is_gutter(s)) {
                push_text(out, &span.text, StyleRole::Quote, span.attrs.strikethrough);
            }
        }
        return;
    }
    for (i, line) in block.lines.iter().enumerate() {
        if i > 0 {
            out.push(StyledSpan::new("\n", StyleRole::Normal));
        }
        for span in line.spans.iter().filter(|s| !is_gutter(s)) {
            push_text(out, &span.text, StyleRole::Quote, span.attrs.strikethrough);
        }
    }
}

/// 表格体内单元格 span 角色映射(0014 A 等宽 → 全 MONO 保列对齐,5E.1 #2):行内码 → `Code`
/// (等宽绿),粗/斜/链接文字 → `TableStrong`(等宽加粗),其余 → `TableCell`。表头统一 `TableHeader`
/// (保 render 的表头检测;表头极少带内联格式)。
fn cell_role(span: &JSpan, is_header: bool) -> StyleRole {
    if is_header {
        return StyleRole::TableHeader;
    }
    match span.role {
        JRole::Code | JRole::Math => StyleRole::Code,
        JRole::Strong | JRole::Link => StyleRole::TableStrong,
        _ if span.attrs.bold => StyleRole::TableStrong,
        _ if span.attrs.italic => StyleRole::TableEm,
        _ => StyleRole::TableCell,
    }
}

/// 表格 → 单元格 run 序列(0014 B / plan5 §5F):每格按内容发 run(行内码/强调/链接分角色),
/// **不补白、不发 `│` 分隔**(对齐/竖线/折行交 JS 像素两趟 + render rect);行间 `\n`。
/// 返回 [`TableRegion`](每格在 `out` 里的 run 区间 `[start,end)` + 每列对齐 0/1/2),供 JS 定位。
fn emit_table(
    out: &mut Vec<StyledSpan>,
    table: &[Vec<String>],
    spans: &[Vec<Vec<JSpan>>],
    align: &[Alignment],
) -> TableRegion {
    let cols = table.iter().map(Vec::len).max().unwrap_or(0);
    let mut rows: Vec<Vec<(u32, u32)>> = Vec::with_capacity(table.len());
    for (r, row) in table.iter().enumerate() {
        if r > 0 {
            out.push(StyledSpan::new("\n", StyleRole::Normal));
        }
        let is_header = r == 0;
        let mut cells: Vec<(u32, u32)> = Vec::with_capacity(cols);
        for c in 0..cols {
            let start = out.len() as u32;
            // 单元格内容:有富 span 用富 span(分角色),否则回退纯字符串(整格一个 run)。
            match spans.get(r).and_then(|row| row.get(c)) {
                Some(cell_spans) if !cell_spans.is_empty() => {
                    for s in cell_spans {
                        out.push(StyledSpan::styled(
                            s.text.clone(),
                            cell_role(s, is_header),
                            s.attrs.strikethrough,
                        ));
                    }
                }
                _ => {
                    let cell = row.get(c).map_or("", |s| s.trim());
                    if !cell.is_empty() {
                        let role = if is_header {
                            StyleRole::TableHeader
                        } else {
                            StyleRole::TableCell
                        };
                        out.push(StyledSpan::new(cell.to_owned(), role));
                    }
                }
            }
            cells.push((start, out.len() as u32)); // 空格 → 空区间 (start==end)
        }
        rows.push(cells);
    }
    let aligns = (0..cols)
        .map(|c| match align.get(c).copied().unwrap_or(Alignment::Left) {
            Alignment::Right => 2u8,
            Alignment::Center => 1u8,
            Alignment::Left => 0u8,
        })
        .collect();
    TableRegion { rows, aligns }
}

/// 把一个 jcode Block 展平成我们的 span 序列(行间插 `\n`);表格额外产出 [`TableRegion`] 入 `tables`。
fn emit_block(out: &mut Vec<StyledSpan>, block: &Block, tables: &mut Vec<TableRegion>) {
    match &block.kind {
        // 分隔线:发一个零墨空格作锚点,render 侧画整宽细线(4B1)。
        BlockKind::ThematicBreak => {
            out.push(StyledSpan::new(" ", StyleRole::Rule));
            return;
        }
        BlockKind::BlockQuote => {
            emit_blockquote(out, block);
            return;
        }
        _ => {}
    }
    if matches!(block.kind, BlockKind::Table) && !block.table.is_empty() {
        let region = emit_table(out, &block.table, &block.table_spans, &block.table_align);
        tables.push(region);
        return;
    }
    for (i, line) in block.lines.iter().enumerate() {
        if i > 0 {
            out.push(StyledSpan::new("\n", StyleRole::Normal));
        }
        for span in &line.spans {
            let role = map_role(span, &block.kind);
            push_text(out, &span.text, role, span.attrs.strikethrough);
        }
    }
}

/// jcode 的 (StyleRole + attrs + 块类型) → 我们的渲染角色。
fn map_role(span: &JSpan, kind: &BlockKind) -> StyleRole {
    // 块级覆盖优先。
    match kind {
        BlockKind::Heading { level } => return StyleRole::heading(*level),
        BlockKind::CodeBlock { .. } => return StyleRole::CodeBlock,
        _ => {}
    }
    match span.role {
        JRole::Code | JRole::Math => StyleRole::Code,
        JRole::Link => StyleRole::Link,
        JRole::Dim => StyleRole::ListMarker,
        JRole::Reasoning => StyleRole::Quote,
        JRole::Html => StyleRole::Normal,
        JRole::Strong | JRole::Text => {
            let bold = span.attrs.bold || matches!(span.role, JRole::Strong);
            emphasis(bold, span.attrs)
        }
    }
}

fn emphasis(bold: bool, attrs: TextAttrs) -> StyleRole {
    match (bold, attrs.italic) {
        (true, true) => StyleRole::BoldItalic,
        (true, false) => StyleRole::Bold,
        (false, true) => StyleRole::Italic,
        (false, false) => StyleRole::Normal,
    }
}

/// 把内部换行拆成 span 边界的 `\n`(零宽,layout 处理);其余直接成 span。`strike` = 删除线(A)。
fn push_text(out: &mut Vec<StyledSpan>, text: &str, role: StyleRole, strike: bool) {
    let mut first = true;
    for line in text.split('\n') {
        if !first {
            out.push(StyledSpan::new("\n", StyleRole::Normal));
        }
        first = false;
        if !line.is_empty() {
            out.push(StyledSpan::styled(line, role, strike));
        }
    }
}

/// 尾部"主动补全"未闭合的行内/块语法,消除流式半截标记闪烁(0004 §5.1,AR9)。
/// 只补全成可解析的形态,不改变已闭合内容。
fn remend(src: &str) -> String {
    let fence_count = src.matches("```").count();
    let mut patched = src.to_string();
    if fence_count % 2 == 1 {
        if !patched.ends_with('\n') {
            patched.push('\n');
        }
        patched.push_str("```");
        return patched; // 围栏内不再处理行内标记
    }
    let last_line = patched.rsplit('\n').next().unwrap_or("");
    let mut suffix = String::new();
    if last_line.matches('`').count() % 2 == 1 {
        suffix.push('`');
    }
    let strong = last_line.matches("**").count();
    if strong % 2 == 1 {
        suffix.push_str("**");
    }
    let without_strong = last_line.replace("**", "");
    if without_strong.matches('*').count() % 2 == 1 {
        suffix.push('*');
    }
    patched.push_str(&suffix);
    patched
}

#[cfg(test)]
mod node_tests {
    use super::*;
    use crate::nodes::{check_invariants, glyph_key, Node, NodeKind, NodeTree};

    /// 各结构样例 markdown(同时是 7E `?debug` 节点框 case 的输入,Plan 7 测试表)。
    const SAMPLES: &[(&str, &str)] = &[
        ("n01-headings", "# H1\n\n## H2\n\npara text\n\n### H3"),
        ("n02-inline", "plain **bold** and *em* and `code` and [lnk](u) and ~~del~~"),
        ("n03-list-flat", "- a\n- b\n\n1. one\n2. two"),
        ("n04-list-nested", "- a\n  - a1\n  - a2\n- b"),
        ("n05-quote-nested", "> q1\n> q2\n\n> outer\n> > inner"),
        ("n06-codeblock", "text\n\n```rust\nfn main() {}\n```\n\nafter"),
        ("n07-table", "| A | B |\n|:--|--:|\n| 1 | 2 |\n| 3 | 4 |"),
        (
            "n08-mixed",
            "# Title\n\npara\n\n- item\n\n> quote\n\n```\ncode\n```\n\n| x | y |\n|---|---|\n| 1 | 2 |",
        ),
        ("n09-edge", "-\n\n> code:\n> ```\n> x\n> ```\n\n| a |\n|---|"),
    ];

    fn tree(src: &str) -> NodeTree {
        parse_markdown_nodes(src, 0).2
    }

    #[test]
    fn all_samples_satisfy_tree_invariants() {
        for (name, src) in SAMPLES {
            let t = tree(src);
            let inv = check_invariants(&t);
            assert!(inv.is_ok(), "{name}: 不变式失败 {inv:?}");
            assert!(t.root().is_some(), "{name}: 应有根");
            assert_eq!(t.root().map(|n| n.kind), Some(NodeKind::Doc));
        }
    }

    #[test]
    fn headings_and_paragraphs_have_nodes() {
        let t = tree("# H1\n\npara\n\n## H2");
        assert_eq!(t.nodes_of_kind(NodeKind::Heading).count(), 2);
        assert!(t.nodes_of_kind(NodeKind::Paragraph).count() >= 1);
    }

    #[test]
    fn list_nesting_parent_chain() {
        let t = tree("- a\n  - a1\n- b");
        let lists = t.nodes_of_kind(NodeKind::List).count();
        assert!(lists >= 2, "应有外层 + 嵌套 List: {lists}");
        let items: Vec<u32> = t
            .nodes_of_kind(NodeKind::ListItem)
            .map(|(i, _)| i)
            .collect();
        assert_eq!(items.len(), 3, "三个 ListItem");
        // 嵌套项(a1)的祖先链含两个 List。
        let nested = items
            .iter()
            .find(|&&i| {
                t.ancestors(i)
                    .iter()
                    .filter(|&&a| t.nodes()[a as usize].kind == NodeKind::List)
                    .count()
                    == 2
            })
            .copied();
        assert!(nested.is_some(), "应有一项处于双层 List 下");
    }

    #[test]
    fn table_tree_and_cell_ranges_match_region() {
        let src = "| A | B |\n|---|---|\n| 1 | 2 |";
        let (_spans, tables, t) = parse_markdown_nodes(src, 0);
        assert_eq!(t.nodes_of_kind(NodeKind::Table).count(), 1);
        assert_eq!(t.nodes_of_kind(NodeKind::TableRow).count(), 2);
        let cells: Vec<&Node> = t
            .nodes_of_kind(NodeKind::TableCell)
            .map(|(_, n)| n)
            .collect();
        assert_eq!(cells.len(), 4);
        // 身份折并:每个 TableCell 节点非空、被某 TableRow 包含(range 来自 TableRegion span 区间)。
        assert_eq!(tables.len(), 1);
        for c in &cells {
            assert!(!c.is_empty(), "cell 区间非空");
        }
    }

    #[test]
    fn node_at_finds_innermost() {
        let t = tree("# Hi\n\nword");
        // glyph 0 = 标题首字 'H' → 最内层应在 Heading 子树(Run),其祖先含 Heading。
        let inner = t.node_at(0).expect("命中");
        let anc = t.ancestors(inner);
        assert!(
            anc.iter()
                .any(|&a| t.nodes()[a as usize].kind == NodeKind::Heading)
                || t.nodes()[inner as usize].kind == NodeKind::Heading,
            "glyph 0 应属标题"
        );
    }

    #[test]
    fn append_only_keeps_prefix_identity() {
        // 0017 §6:同前缀追加 → 已有节点 key + range.start 不变。
        let a = tree("alpha\n\nbeta");
        let b = tree("alpha\n\nbeta\n\ngamma");
        // 第一个 Paragraph(alpha)节点。
        let pa = a.nodes_of_kind(NodeKind::Paragraph).next().expect("a para");
        let pb = b.nodes_of_kind(NodeKind::Paragraph).next().expect("b para");
        assert_eq!(pa.1.key, pb.1.key, "前缀节点 key 稳定");
        assert_eq!(pa.1.range.0, pb.1.range.0, "前缀节点 range.start 稳定");
    }

    #[test]
    fn glyph_key_packs_block_seq_and_idx() {
        // 0016/0020:glyph 虚拟身份 = (block_seq<<32)|idx。
        assert_eq!(glyph_key(0, 5), 5);
        assert_eq!(glyph_key(3, 7), (3u64 << 32) | 7);
        assert_ne!(glyph_key(1, 0), glyph_key(0, 1));
    }

    #[test]
    fn block_seq_in_key_high_bits() {
        let t = tree("para");
        let root = t.root().expect("root");
        assert_eq!(root.key >> 32, 0, "block_seq 0");
        let t2 = parse_markdown_nodes("para", 9).2;
        assert_eq!(t2.root().expect("root").key >> 32, 9, "block_seq 9 进高位");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render(spans: &[StyledSpan]) -> String {
        spans.iter().map(StyledSpan::text).collect()
    }

    fn role_of(spans: &[StyledSpan], needle: &str) -> Option<StyleRole> {
        spans
            .iter()
            .find(|s| s.text().contains(needle))
            .map(StyledSpan::role)
    }

    #[test]
    fn bold_italic_code_roles() {
        let spans = parse_markdown("a **b** c *d* `e`");
        assert_eq!(role_of(&spans, "b"), Some(StyleRole::Bold));
        assert_eq!(role_of(&spans, "d"), Some(StyleRole::Italic));
        assert_eq!(role_of(&spans, "e"), Some(StyleRole::Code));
        assert!(!render(&spans).contains('*'));
        assert!(!render(&spans).contains('`'));
    }

    #[test]
    fn currency_dollar_not_math() {
        // 借鉴 jcode escape_currency_dollars(vendored parse_markdown 内置,0013):
        // `$5`/`$5x` 是货币不是行内数学 → 原样显示、非 Code/Math 角色。
        let spans = parse_markdown("一共 $5 和 $5x 两笔");
        let r = render(&spans);
        assert!(r.contains("$5"), "货币 $ 应原样显示: {r}");
        assert!(r.contains("$5x"), "货币 $5x 应原样显示: {r}");
        assert_eq!(
            role_of(&spans, "$5"),
            Some(StyleRole::Normal),
            "货币不应是代码/数学角色"
        );
    }

    #[test]
    fn heading_role_and_break() {
        let spans = parse_markdown("# Title\n\nbody");
        assert_eq!(role_of(&spans, "Title"), Some(StyleRole::Heading));
        assert!(render(&spans).contains('\n'), "标题与正文之间应有换行");
        assert!(!render(&spans).contains('#'));
    }

    #[test]
    fn heading_levels_distinct() {
        let spans = parse_markdown("# One\n\n## Two\n\n### Three");
        assert_eq!(role_of(&spans, "One"), Some(StyleRole::Heading)); // H1
        assert_eq!(role_of(&spans, "Two"), Some(StyleRole::Heading2));
        assert_eq!(role_of(&spans, "Three"), Some(StyleRole::Heading3));
        assert!(StyleRole::Heading3.is_heading());
        assert!(!StyleRole::Bold.is_heading());
    }

    #[test]
    fn code_block_role() {
        let spans = parse_markdown("```\nlet x = 1;\n```");
        assert_eq!(role_of(&spans, "let x"), Some(StyleRole::CodeBlock));
    }

    #[test]
    fn table_emits_cells_and_region() {
        // 0014 B:表格发单元格 run(**无补白、无 │**)+ TableRegion(run 区间 + 列对齐),
        // 像素对齐/竖线/折行交 JS(plan5 §5F)。
        let md = "| Name | Score |\n|:--|--:|\n| Al | 3 |\n| Catherine | 1000 |";
        let (spans, tables, _) = parse_markdown_nodes(md, 0);
        let r = render(&spans);
        assert!(
            !r.contains('|') && !r.contains('│'),
            "无 raw/装饰竖线: {r:?}"
        );
        assert!(!r.contains("---"), "分隔行不该显形: {r:?}");
        for cell in ["Name", "Score", "Al", "Catherine", "1000"] {
            assert!(r.contains(cell), "缺单元格 {cell}: {r:?}");
        }
        assert_eq!(role_of(&spans, "Name"), Some(StyleRole::TableHeader));
        assert_eq!(role_of(&spans, "Catherine"), Some(StyleRole::TableCell));
        // 结构:1 个表、3 行(表头 + 2 数据)、2 列、对齐 [Left, Right]。
        assert_eq!(tables.len(), 1, "应有 1 个表格区");
        let t = &tables[0];
        assert_eq!(t.rows.len(), 3, "3 行");
        assert_eq!(t.aligns, vec![0u8, 2u8], "列对齐 L/R");
        for row in &t.rows {
            assert_eq!(row.len(), 2, "每行 2 格");
            for &(s, e) in row {
                assert!(s <= e && (e as usize) <= spans.len(), "run 区间合法");
            }
        }
    }

    #[test]
    fn table_inline_format_styled_not_plain() {
        // 5E.1 #2:单元格内 `**粗**` / `` `码` `` 保留样式(等宽角色),非纯文本。
        let md = "| H |\n|---|\n| **b** and `c` |";
        let spans = parse_markdown(md);
        assert_eq!(
            role_of(&spans, "b"),
            Some(StyleRole::TableStrong),
            "粗 → TableStrong"
        );
        assert_eq!(role_of(&spans, "c"), Some(StyleRole::Code), "行内码 → Code");
        assert!(!render(&spans).contains('*'), "raw ** 不应显形");
        assert!(!render(&spans).contains('`'), "raw ` 不应显形");
    }

    #[test]
    fn table_link_text_only_no_url_leak() {
        // 5E.1 #3:格内链接只显文字(TableStrong),URL 不泄漏到表后(不另起一行)。
        let md = "| H |\n|---|\n| [docs](https://example.com/very/long/path) |";
        let spans = parse_markdown(md);
        let r = render(&spans);
        assert!(r.contains("docs"), "链接文字应显示: {r}");
        assert!(!r.contains("example.com"), "URL 不应泄漏: {r}");
        assert!(!r.contains('('), "( 不应出现(无 (url) 后缀): {r}");
        assert_eq!(role_of(&spans, "docs"), Some(StyleRole::TableStrong));
    }

    #[test]
    fn strikethrough_flags_struck_run() {
        // A:`~~…~~` 经 attrs.strikethrough 标到 StyledSpan.is_struck()(正文 + 表格通用)。
        let spans = parse_markdown("a ~~b~~ c");
        let struck = spans
            .iter()
            .find(|s| s.text().contains('b'))
            .map(StyledSpan::is_struck);
        assert_eq!(struck, Some(true), "~~b~~ 应标记删除线");
        let plain = spans
            .iter()
            .find(|s| s.text().contains('a'))
            .is_some_and(StyledSpan::is_struck);
        assert!(!plain, "普通文本不应有删除线");
        // 表格内删除线同样标记。
        let (tspans, _, _) = parse_markdown_nodes("| H |\n|---|\n| ~~x~~ |", 0);
        let tstruck = tspans
            .iter()
            .find(|s| s.text().contains('x'))
            .map(StyledSpan::is_struck);
        assert_eq!(tstruck, Some(true), "表格内 ~~x~~ 应标记删除线");
    }

    #[test]
    fn table_alignment_in_region() {
        // 5E.1 #1:对齐从 jcode 带出到 TableRegion.aligns(L/C/R = 0/1/2),布局由 JS 像素两趟用。
        let md = "| L | C | R |\n|:--|:-:|--:|\n| a | b | c |";
        let (_spans, tables, _) = parse_markdown_nodes(md, 0);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].aligns, vec![0u8, 1u8, 2u8]);
    }

    #[test]
    fn pending_table_suppressed_until_delimiter() {
        // 表头行已到、分隔行未到 → 末块"正在成形的表格" → 抑制,不闪 raw `| a | b |`(#4)。
        let r1 = render(&parse_markdown("| Name | Score |"));
        assert!(r1.is_empty(), "成形中的表格应抑制不显示,实得: {r1:?}");
        // 多行(表头 + 半截分隔)仍未确认 → 继续抑制。
        let r1b = render(&parse_markdown("| Name | Score |\n|--"));
        assert!(r1b.is_empty(), "分隔行未完仍应抑制,实得: {r1b:?}");
        // 分隔行到齐 → pulldown 确认成 Table → 揭示表头,且无 raw 竖线。
        let r2 = render(&parse_markdown("| Name | Score |\n|---|---|"));
        assert!(
            r2.contains("Name") && r2.contains("Score"),
            "确认后应显示表头: {r2}"
        );
        assert!(!r2.contains('|'), "应无 raw 竖线(用 │): {r2}");
        // 前面有正常段落时,只抑制末尾的成形表格,前文照常显示。
        let r3 = render(&parse_markdown("hello\n\n| a | b |"));
        assert!(r3.contains("hello"), "前文段落应显示: {r3}");
        assert!(!r3.contains("a | b"), "末尾成形表格应抑制: {r3}");
    }

    #[test]
    fn table_empty_cell_is_empty_run_range() {
        // 残缺格 → 空 run 区间(start==end),不 panic;CJK 对齐由 JS 像素量(不再靠字符数)。
        let md = "| A | B |\n|---|---|\n| 1 |  |";
        let (_spans, tables, _) = parse_markdown_nodes(md, 0);
        let (s, e) = tables[0].rows[1][1]; // 第 2 行第 2 格 = 空
        assert_eq!(s, e, "空格应是空 run 区间");
    }

    #[test]
    fn content_gate_levels_per_block_kind() {
        use crate::reveal::RevealUnit;
        // 段落逐字。
        assert_eq!(content_gate("hello world"), RevealUnit::Glyph);
        // 围栏:未闭 = Line(已到行逐行),闭合 = Block。
        assert_eq!(content_gate("```\nlet x = 1;"), RevealUnit::Line);
        assert_eq!(content_gate("```\nlet x = 1;\n```"), RevealUnit::Block);
        // 列表项闭合 = Line。
        assert_eq!(content_gate("- one\n- two"), RevealUnit::Line);
        // 表格确认成 Table → Row(行数:表头 + 2 数据)。
        assert_eq!(
            content_gate("| A | B |\n|---|---|\n| 1 | 2 |\n| 3 | 4 |"),
            RevealUnit::Row(3)
        );
    }

    #[test]
    fn pending_structure_suppresses_table_and_math() {
        // 表格成形中(无分隔行)→ 抑制(沿用 is_pending_table)。
        assert!(render(&parse_markdown("| a | b |")).is_empty());
        // 半截显示公式 `$$…`(开了未闭)→ 抑制,不闪 raw `$$`。
        let r = render(&parse_markdown("$$E = mc^2"));
        assert!(!r.contains("$$"), "半截公式不应闪 raw $$: {r:?}");
        // 闭合后正常显示(不抑制)。
        let r2 = render(&parse_markdown("$$E = mc^2$$"));
        assert!(r2.contains("E = mc"), "闭合公式应显示: {r2}");
    }

    #[test]
    fn blockquote_maps_to_quote_role() {
        let spans = parse_markdown("> quoted text");
        assert_eq!(role_of(&spans, "quoted text"), Some(StyleRole::Quote));
    }

    #[test]
    fn github_alert_emits_label_and_quote_body() {
        let spans = parse_markdown("> [!WARNING]\n> be careful here");
        let r = render(&spans);
        assert!(!r.contains("[!"), "告警标记不该显形: {r}");
        assert_eq!(
            role_of(&spans, "WARNING"),
            Some(StyleRole::AlertLabel),
            "首行应是 AlertLabel"
        );
        assert_eq!(role_of(&spans, "careful"), Some(StyleRole::Quote));
    }

    #[test]
    fn thematic_break_emits_rule_anchor() {
        let spans = parse_markdown("a\n\n---\n\nb");
        assert!(
            spans.iter().any(|s| s.role() == StyleRole::Rule),
            "应发 Rule 锚点"
        );
        // 锚点零墨(空格),不显形为 ─
        assert!(!render(&spans).contains('─'));
    }

    #[test]
    fn remend_hides_unclosed_bold_no_flicker() {
        let spans = parse_markdown("**bol");
        assert!(
            !render(&spans).contains('*'),
            "未闭合 ** 不该显形: {:?}",
            render(&spans)
        );
        assert_eq!(role_of(&spans, "bol"), Some(StyleRole::Bold));
    }

    #[test]
    fn remend_closes_unbalanced_fence() {
        let spans = parse_markdown("```\ncode");
        assert_eq!(role_of(&spans, "code"), Some(StyleRole::CodeBlock));
        assert!(!render(&spans).contains('`'));
    }

    #[test]
    fn plain_passthrough_still_works() {
        let spans = plain("纯文本 🚀");
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].role(), StyleRole::Normal);
        assert_eq!(spans[0].text(), "纯文本 🚀");
    }

    #[test]
    fn comprehensive_markdown_syntax_coverage() {
        let md = r#"# Heading 1
## Heading 2
### Heading 3
#### Heading 4
##### Heading 5
###### Heading 6

This is a **bold** word and an *italic* word and ***bold italic*** and ~~strikethrough~~ text.

Inline `code` span, a [link](https://example.com), an ![image](img.png), and some $math$ inline.

> A blockquote line.
> Multiple blockquote lines.

- Unordered item
- Another item

1. First ordered
2. Second ordered

- Top level
  - Nested item

- [ ] Pending task
- [x] Done task

```
fn main() {
    let x = 42;
}
```

```rust
fn typed() -> &'static str {
    "hello"
}
```

| Name | Age |
|------|-----|
| Alice | 30 |
| Bob | 25 |

---

Footnote: some text[^1].

[^1]: This is a footnote definition.

Term
: Definition text.

Display math:
$$E = mc^2$$
"#;

        let spans = parse_markdown(md);
        let r = render(&spans);

        // Headings — H1 = Heading, H2–H6 分级(4A3)
        assert_eq!(role_of(&spans, "Heading 1"), Some(StyleRole::Heading));
        assert_eq!(role_of(&spans, "Heading 6"), Some(StyleRole::Heading6));
        assert!(!r.contains('#'), "raw # 不应出现在渲染文本中");

        // Inline styles
        assert_eq!(role_of(&spans, "bold"), Some(StyleRole::Bold));
        assert_eq!(role_of(&spans, "italic"), Some(StyleRole::Italic));
        assert_eq!(role_of(&spans, "bold italic"), Some(StyleRole::BoldItalic));
        assert!(!r.contains("~~"), "raw ~~ 不应出现");

        // Code
        assert_eq!(role_of(&spans, "code"), Some(StyleRole::Code));
        assert!(!r.contains('`'), "raw backtick 不应出现");

        // Link and image
        assert!(r.contains("link"), "链接文本应保留: {r}");
        assert!(r.contains("https://example.com"), "链接 URL 应保留");
        assert!(
            r.contains("[image:") || r.contains("img.png"),
            "图片应渲染为占位符: {r}"
        );

        // Math
        assert!(r.contains("math"), "行内数学应保留文本");

        // Blockquote — content preserved, mapped to Quote role (4B1 左条)
        assert!(r.contains("blockquote line"), "引用块文本应保留: {r}");
        assert_eq!(role_of(&spans, "blockquote line"), Some(StyleRole::Quote));

        // Lists
        assert!(r.contains("Unordered"), "无序列表项应保留");
        assert!(r.contains("First"), "有序列表项应保留");
        assert!(r.contains("Nested"), "嵌套列表项应保留");

        // Task lists
        assert!(r.contains("Pending"), "待办项应保留");
        assert!(r.contains("Done"), "完成项应保留");

        // Code blocks
        assert_eq!(role_of(&spans, "fn main() {"), Some(StyleRole::CodeBlock));
        assert_eq!(role_of(&spans, "fn typed()"), Some(StyleRole::CodeBlock));

        // Table — 0014 B:无 │ 分隔(列由 JS 像素两趟定位);单元格内容在,raw |/│ 不显形
        assert!(
            r.contains("Name") && r.contains("Alice"),
            "表格单元格应显示: {r}"
        );
        assert!(
            !r.contains('|') && !r.contains('│'),
            "raw 竖线不应显形: {r}"
        );

        // Thematic break — 4B1:不再吐 ─ 字符,改发 Rule 锚点(render 画细线 rect)
        assert!(
            spans.iter().any(|s| s.role() == StyleRole::Rule),
            "分隔线应发 Rule 锚点"
        );

        // Footnote
        assert!(r.to_lowercase().contains("footnote"), "脚注定义应保留: {r}");

        // Definition list
        assert!(r.contains("Definition"), "定义列表项应保留");

        // Display math
        assert!(r.contains("E = mc"), "显示数学应保留文本");
    }
}
