//! content(M6)— markdown 语义化(Plan2 H,plan2 H1)。
//!
//! 解析交给 vendored 的 **jcode-render-core**(后端中立文档模型:`parse_markdown -> Document`,
//! 含标题/粗斜/行内与块代码/列表/引用/**表格**/数学等);本文件把它的 `Document` 适配成我们
//! 渲染管线的 [`StyledSpan`] + [`StyleRole`](决定字体/上色,render 侧按 `as_u32` 分桶取色)。
//!
//! 流式不闪烁:[`remend`] 在解析前对尾部"主动补全"未闭合的 `**`/`` ` ``/```` ``` ````
//! (0004 §5.1),避免半截语法字符瞬间字面显形(AR9 同族)。

use jcode_render_core::{
    parse_markdown as jcode_parse, Block, BlockKind, StyleRole as JRole, StyledSpan as JSpan,
    TextAttrs,
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
}

impl StyleRole {
    /// 给 render/atlas 用的稳定数值。
    pub fn as_u32(self) -> u32 {
        self as u32
    }
}

/// 一段带样式角色的文本 run(content→layout)。
#[derive(Clone, Debug, PartialEq)]
pub struct StyledSpan {
    text: String,
    role: StyleRole,
}

impl StyledSpan {
    pub fn new(text: impl Into<String>, role: StyleRole) -> Self {
        Self {
            text: text.into(),
            role,
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn role(&self) -> StyleRole {
        self.role
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

/// 解析 markdown 源 → 带角色的 span 序列。块/行间以 `\n` 分隔(零宽换行由 layout 处理)。
pub fn parse_markdown(src: &str) -> Vec<StyledSpan> {
    let patched = remend(src);
    let doc = jcode_parse(&patched);
    let mut out: Vec<StyledSpan> = Vec::new();
    for block in &doc.blocks {
        if !out.is_empty() {
            out.push(StyledSpan::new("\n", StyleRole::Normal)); // 块间换行
        }
        emit_block(&mut out, block);
    }
    out
}

/// 把一个 jcode Block 展平成我们的 span 序列(行间插 `\n`)。
fn emit_block(out: &mut Vec<StyledSpan>, block: &Block) {
    if matches!(block.kind, BlockKind::Table) && !block.table.is_empty() {
        // 表格:每行一行,单元格用 " │ " 分隔;表头加粗。列对齐(等宽)留后续。
        for (r, row) in block.table.iter().enumerate() {
            if r > 0 {
                out.push(StyledSpan::new("\n", StyleRole::Normal));
            }
            let role = if r == 0 {
                StyleRole::Heading
            } else {
                StyleRole::Normal
            };
            for (c, cell) in row.iter().enumerate() {
                if c > 0 {
                    out.push(StyledSpan::new(" │ ", StyleRole::ListMarker));
                }
                push_text(out, cell, role);
            }
        }
        return;
    }
    for (i, line) in block.lines.iter().enumerate() {
        if i > 0 {
            out.push(StyledSpan::new("\n", StyleRole::Normal));
        }
        for span in &line.spans {
            let role = map_role(span, &block.kind);
            push_text(out, &span.text, role);
        }
    }
}

/// jcode 的 (StyleRole + attrs + 块类型) → 我们的渲染角色。
fn map_role(span: &JSpan, kind: &BlockKind) -> StyleRole {
    // 块级覆盖优先。
    match kind {
        BlockKind::Heading { .. } => return StyleRole::Heading,
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

/// 把内部换行拆成 span 边界的 `\n`(零宽,layout 处理);其余直接成 span。
fn push_text(out: &mut Vec<StyledSpan>, text: &str, role: StyleRole) {
    let mut first = true;
    for line in text.split('\n') {
        if !first {
            out.push(StyledSpan::new("\n", StyleRole::Normal));
        }
        first = false;
        if !line.is_empty() {
            out.push(StyledSpan::new(line, role));
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
    fn heading_role_and_break() {
        let spans = parse_markdown("# Title\n\nbody");
        assert_eq!(role_of(&spans, "Title"), Some(StyleRole::Heading));
        assert!(render(&spans).contains('\n'), "标题与正文之间应有换行");
        assert!(!render(&spans).contains('#'));
    }

    #[test]
    fn code_block_role() {
        let spans = parse_markdown("```\nlet x = 1;\n```");
        assert_eq!(role_of(&spans, "let x"), Some(StyleRole::CodeBlock));
    }

    #[test]
    fn table_renders_rows_not_raw_pipes() {
        let md = "| A | B |\n|---|---|\n| 1 | 2 |";
        let spans = parse_markdown(md);
        let r = render(&spans);
        assert!(!r.contains('|'), "原始竖线不该显形: {r}");
        assert!(!r.contains("---"), "分隔行不该显形: {r}");
        assert!(r.contains('│'), "应有单元格分隔符: {r}");
        for cell in ["A", "B", "1", "2"] {
            assert!(r.contains(cell), "缺单元格 {cell}: {r}");
        }
        assert_eq!(role_of(&spans, "A"), Some(StyleRole::Heading)); // 表头加粗
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
}
