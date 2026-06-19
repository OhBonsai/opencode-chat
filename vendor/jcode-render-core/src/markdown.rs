//! Markdown -> backend-neutral [`Document`].
//!
//! This mirrors the *semantics* of the TUI markdown renderer
//! (`jcode-tui-markdown`) but emits the neutral [`crate::model`] types instead
//! of `ratatui` spans. Front-ends adapt the document to their backend and may
//! wrap it with [`crate::wrap`].
//!
//! Scope note: this is the shared foundation. It currently covers headings,
//! paragraphs, inline emphasis/strong/strike/code, fenced & indented code
//! blocks, blockquotes, ordered/unordered (incl. nested) lists, thematic
//! breaks, links, and raw HTML passthrough. Tables and math are tracked as
//! follow-ups; the TUI renderer remains authoritative until parity is proven.

use pulldown_cmark::{CodeBlockKind, Event, Options, Parser, Tag, TagEnd};

use crate::model::{
    Alignment, Block, BlockKind, Document, FillRole, StyleRole, StyledLine, StyledSpan, TextAttrs,
};

#[derive(Clone, Copy, Default)]
struct InlineStyle {
    bold: bool,
    italic: bool,
    strike: bool,
}

impl InlineStyle {
    fn attrs(self) -> TextAttrs {
        TextAttrs {
            bold: self.bold,
            italic: self.italic,
            strikethrough: self.strike,
            underline: false,
        }
    }

    fn role(self) -> StyleRole {
        if self.bold {
            StyleRole::Strong
        } else {
            StyleRole::Text
        }
    }
}

/// pulldown 的列对齐 → 中立模型对齐(`None`/`Left` → Left)。
fn map_alignment(a: &pulldown_cmark::Alignment) -> Alignment {
    match a {
        pulldown_cmark::Alignment::Right => Alignment::Right,
        pulldown_cmark::Alignment::Center => Alignment::Center,
        _ => Alignment::Left,
    }
}

/// 表格单元格 span 序列去首尾空白(首 span trim_start、尾 span trim_end),并丢空 span。
fn trim_spans(mut spans: Vec<StyledSpan>) -> Vec<StyledSpan> {
    if let Some(first) = spans.first_mut() {
        first.text = first.text.trim_start().to_string();
    }
    if let Some(last) = spans.last_mut() {
        last.text = last.text.trim_end().to_string();
    }
    spans.retain(|s| !s.text.is_empty());
    spans
}

struct ListFrame {
    ordered: bool,
    next_number: u64,
    /// Block index in `doc.blocks` where this list's content begins, used to
    /// right-align ordered markers once the list's width is known.
    start_block: usize,
    /// Nesting depth of this list (0 = outermost).
    depth: usize,
}

/// Right-align ordered-list markers within a single list level, mirroring the
/// legacy renderer: when the list has multi-digit item numbers, shorter markers
/// are padded with leading spaces so the `.` separators line up and wrapped
/// continuation lines indent consistently.
///
/// Only markers at exactly `depth` indentation are touched, so nested lists
/// (which carry deeper indentation) are aligned by their own `End(List)`.
fn align_ordered_markers(doc: &mut Document, start_block: usize, depth: usize) {
    let indent = "  ".repeat(depth);

    // First pass: find the widest digit run among this level's markers.
    let mut max_digits = 0usize;
    for block in doc.blocks.iter().skip(start_block) {
        if let Some(d) = ordered_marker_digits(block, &indent) {
            max_digits = max_digits.max(d);
        }
    }
    if max_digits <= 1 {
        return;
    }

    // Second pass: pad shorter markers.
    for block in doc.blocks.iter_mut().skip(start_block) {
        let Some(d) = ordered_marker_digits(block, &indent) else {
            continue;
        };
        let extra = max_digits - d;
        if extra == 0 {
            continue;
        }
        if let Some(first_span) = block
            .lines
            .first_mut()
            .and_then(|line| line.spans.first_mut())
        {
            // Insert padding right after the indent, before the digits.
            let rest = &first_span.text[indent.len()..];
            first_span.text = format!("{indent}{}{rest}", " ".repeat(extra));
        }
    }
}

/// If `block`'s first span is an ordered-list marker at exactly `indent`
/// (i.e. `{indent}{digits}. `), return the digit count.
fn ordered_marker_digits(block: &Block, indent: &str) -> Option<usize> {
    let text = &block.lines.first()?.spans.first()?.text;
    let rest = text.strip_prefix(indent)?;
    // Reject deeper-nested markers (extra leading spaces) and non-markers.
    if rest.starts_with(' ') {
        return None;
    }
    let digits = rest.chars().take_while(|c| c.is_ascii_digit()).count();
    if digits == 0 {
        return None;
    }
    if !rest[digits..].starts_with(". ") {
        return None;
    }
    Some(digits)
}

/// The block kind that inline content flushed in the current context belongs
/// to, based on enclosing blockquote/list nesting.
fn current_kind(blockquote_depth: usize, list_stack: &[ListFrame]) -> BlockKind {
    if blockquote_depth > 0 {
        BlockKind::BlockQuote
    } else if let Some(frame) = list_stack.last() {
        BlockKind::ListItem {
            ordered: frame.ordered,
            depth: list_stack.len().saturating_sub(1),
        }
    } else {
        BlockKind::Paragraph
    }
}

/// Parse markdown into a backend-neutral [`Document`].
pub fn parse_markdown(text: &str) -> Document {
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_MATH);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);
    options.insert(Options::ENABLE_GFM);
    options.insert(Options::ENABLE_SMART_PUNCTUATION);
    options.insert(Options::ENABLE_DEFINITION_LIST);

    let escaped = crate::preprocess::escape_currency_dollars(text);
    let parser = Parser::new_ext(&escaped, options);

    let mut doc = Document::default();

    // Inline accumulation for the block currently being built.
    let mut spans: Vec<StyledSpan> = Vec::new();
    let mut style = InlineStyle::default();

    // Block context.
    let mut heading_level: Option<u8> = None;
    let mut blockquote_depth = 0usize;
    let mut list_stack: Vec<ListFrame> = Vec::new();

    // Code block accumulation.
    let mut in_code_block = false;
    let mut code_lang: Option<String> = None;
    let mut code_buf = String::new();

    // Pending list-item marker prefix to emit when the item's first inline
    // text arrives.
    let mut pending_item_marker: Option<String> = None;

    // Table accumulation. While `in_table`, inline text is collected into the
    // current cell (as raw text) rather than styled spans, mirroring the legacy
    // renderer which lays tables out by width.
    let mut in_table = false;
    let mut table_rows: Vec<Vec<String>> = Vec::new();
    let mut table_row: Vec<String> = Vec::new();
    let mut current_cell = String::new();
    // Rich (styled) parallel of the flat strings above: preserves inline
    // emphasis/code/link per cell + per-column alignment (so front-ends can
    // style cells and honor `:--`/`:-:`/`--:`).
    let mut table_rows_spans: Vec<Vec<Vec<StyledSpan>>> = Vec::new();
    let mut table_row_spans: Vec<Vec<StyledSpan>> = Vec::new();
    let mut current_cell_spans: Vec<StyledSpan> = Vec::new();
    let mut table_align: Vec<Alignment> = Vec::new();

    // Blockquote line accumulation. Legacy emits one rendered line per source
    // line inside a quote (soft breaks split lines), so we buffer the lines and
    // emit a single BlockQuote block when the outermost quote closes.
    let mut bq_lines: Vec<StyledLine> = Vec::new();

    // Link destinations (stack for nesting); appended as a dim ` (url)` suffix
    // after the link text, mirroring the legacy renderer.
    let mut link_targets: Vec<String> = Vec::new();
    // Image alt-text accumulation.
    let mut in_image = false;
    let mut image_url: Option<String> = None;
    // Definition-list term in progress → its inline text rendered Strong (bold term, no bullet).
    let mut in_def_title = false;
    let mut image_alt = String::new();

    let push_block = |doc: &mut Document, kind: BlockKind, lines: Vec<StyledLine>| {
        if !lines.is_empty() {
            doc.blocks.push(Block::new(kind, lines));
        }
    };

    let flush_paragraph = |doc: &mut Document,
                           spans: &mut Vec<StyledSpan>,
                           kind: BlockKind,
                           alignment: Alignment,
                           blockquote_depth: usize,
                           bq_lines: &mut Vec<StyledLine>| {
        if spans.is_empty() {
            return;
        }
        if blockquote_depth > 0 {
            // Inside a quote: accumulate as a gutter-prefixed line. The block is
            // emitted when the outermost quote closes.
            let mut line = std::mem::take(spans);
            line.insert(
                0,
                StyledSpan::new("│ ".repeat(blockquote_depth), StyleRole::Dim),
            );
            bq_lines.push(StyledLine::from_spans(line));
            return;
        }
        let line = StyledLine::aligned(std::mem::take(spans), alignment);
        push_block(doc, kind, vec![line]);
    };

    for event in parser {
        match event {
            // ---- block starts ----
            Event::Start(Tag::Heading { level, .. }) => {
                flush_paragraph(
                    &mut doc,
                    &mut spans,
                    BlockKind::Paragraph,
                    Alignment::Left,
                    blockquote_depth,
                    &mut bq_lines,
                );
                heading_level = Some(level as u8);
            }
            Event::Start(Tag::Paragraph) => {
                // marker (if any) is emitted lazily on first text
            }
            Event::Start(Tag::BlockQuote(_)) => {
                flush_paragraph(
                    &mut doc,
                    &mut spans,
                    current_kind(blockquote_depth, &list_stack),
                    Alignment::Left,
                    blockquote_depth,
                    &mut bq_lines,
                );
                blockquote_depth += 1;
            }
            Event::Start(Tag::List(first)) => {
                flush_paragraph(
                    &mut doc,
                    &mut spans,
                    current_kind(blockquote_depth, &list_stack),
                    Alignment::Left,
                    blockquote_depth,
                    &mut bq_lines,
                );
                let depth = list_stack.len();
                list_stack.push(ListFrame {
                    ordered: first.is_some(),
                    next_number: first.unwrap_or(1),
                    start_block: doc.blocks.len(),
                    depth,
                });
            }
            Event::Start(Tag::Item) => {
                let depth = list_stack.len().saturating_sub(1);
                let indent = "  ".repeat(depth);
                let marker = if let Some(frame) = list_stack.last_mut() {
                    if frame.ordered {
                        let n = frame.next_number;
                        frame.next_number += 1;
                        format!("{indent}{n}. ")
                    } else {
                        format!("{indent}• ")
                    }
                } else {
                    String::new()
                };
                pending_item_marker = Some(marker);
            }
            Event::Start(Tag::CodeBlock(kind)) => {
                flush_paragraph(
                    &mut doc,
                    &mut spans,
                    BlockKind::Paragraph,
                    Alignment::Left,
                    blockquote_depth,
                    &mut bq_lines,
                );
                in_code_block = true;
                code_buf.clear();
                code_lang = match kind {
                    CodeBlockKind::Fenced(lang) if !lang.is_empty() => Some(lang.to_string()),
                    _ => None,
                };
            }
            Event::Start(Tag::Emphasis) => style.italic = true,
            Event::Start(Tag::Strong) => style.bold = true,
            Event::Start(Tag::Strikethrough) => style.strike = true,
            Event::Start(Tag::Link { dest_url, .. }) => {
                link_targets.push(dest_url.to_string());
            }
            Event::Start(Tag::Image { dest_url, .. }) => {
                in_image = true;
                image_url = Some(dest_url.to_string());
                image_alt.clear();
            }

            // ---- footnote definitions ----
            Event::Start(Tag::FootnoteDefinition(label)) => {
                flush_paragraph(
                    &mut doc,
                    &mut spans,
                    BlockKind::Paragraph,
                    Alignment::Left,
                    blockquote_depth,
                    &mut bq_lines,
                );
                spans.push(StyledSpan::new(format!("[^{label}]: "), StyleRole::Dim));
            }
            Event::End(TagEnd::FootnoteDefinition) => {
                flush_paragraph(
                    &mut doc,
                    &mut spans,
                    BlockKind::Paragraph,
                    Alignment::Left,
                    blockquote_depth,
                    &mut bq_lines,
                );
            }

            // ---- definition lists ----
            Event::Start(Tag::DefinitionListTitle) => {
                flush_paragraph(
                    &mut doc,
                    &mut spans,
                    BlockKind::Paragraph,
                    Alignment::Left,
                    blockquote_depth,
                    &mut bq_lines,
                );
                // 术语:不发 bullet,内联文字置 Strong(粗体)→ 术语自成一行、加粗区分。
                in_def_title = true;
            }
            Event::End(TagEnd::DefinitionListTitle) => {
                in_def_title = false;
                flush_paragraph(
                    &mut doc,
                    &mut spans,
                    BlockKind::Paragraph,
                    Alignment::Left,
                    blockquote_depth,
                    &mut bq_lines,
                );
            }
            Event::Start(Tag::DefinitionListDefinition) => {
                flush_paragraph(
                    &mut doc,
                    &mut spans,
                    BlockKind::Paragraph,
                    Alignment::Left,
                    blockquote_depth,
                    &mut bq_lines,
                );
                // 定义:缩进对齐(去掉 `-> ` 箭头),正文随后正常排。
                spans.push(StyledSpan::new("    ".to_string(), StyleRole::Dim));
            }
            Event::End(TagEnd::DefinitionListDefinition) => {
                flush_paragraph(
                    &mut doc,
                    &mut spans,
                    BlockKind::Paragraph,
                    Alignment::Left,
                    blockquote_depth,
                    &mut bq_lines,
                );
            }

            // ---- tables ----
            Event::Start(Tag::Table(aligns)) => {
                flush_paragraph(
                    &mut doc,
                    &mut spans,
                    current_kind(blockquote_depth, &list_stack),
                    Alignment::Left,
                    blockquote_depth,
                    &mut bq_lines,
                );
                in_table = true;
                table_rows.clear();
                table_rows_spans.clear();
                table_align = aligns.iter().map(map_alignment).collect();
            }
            Event::Start(Tag::TableHead) | Event::Start(Tag::TableRow) => {
                table_row.clear();
                table_row_spans.clear();
            }
            Event::Start(Tag::TableCell) => {
                current_cell.clear();
                current_cell_spans.clear();
            }
            Event::End(TagEnd::TableCell) => {
                table_row.push(current_cell.trim().to_string());
                table_row_spans.push(trim_spans(std::mem::take(&mut current_cell_spans)));
                current_cell.clear();
            }
            Event::End(TagEnd::TableHead) | Event::End(TagEnd::TableRow) => {
                if !table_row.is_empty() {
                    table_rows.push(std::mem::take(&mut table_row));
                    table_rows_spans.push(std::mem::take(&mut table_row_spans));
                }
            }
            Event::End(TagEnd::Table) => {
                in_table = false;
                if !table_rows.is_empty() {
                    doc.blocks.push(Block::table_rich(
                        std::mem::take(&mut table_rows),
                        std::mem::take(&mut table_rows_spans),
                        std::mem::take(&mut table_align),
                    ));
                }
            }

            // ---- inline content ----
            Event::Text(t) => {
                if in_image {
                    image_alt.push_str(&t);
                } else if in_table {
                    current_cell.push_str(&t);
                    // 富文本:链接内文字取 Link 角色(URL 由 End(Link) 丢弃,不泄漏),否则取当前内联样式。
                    let role = if link_targets.is_empty() {
                        style.role()
                    } else {
                        StyleRole::Link
                    };
                    current_cell_spans.push(StyledSpan {
                        text: t.to_string(),
                        role,
                        fill: FillRole::None,
                        attrs: style.attrs(),
                    });
                } else if in_code_block {
                    code_buf.push_str(&t);
                } else {
                    if let Some(marker) = pending_item_marker.take() {
                        spans.push(StyledSpan::new(marker, StyleRole::Dim));
                    }
                    // 定义列表术语:内联文字置 Strong(粗体术语,见 DefinitionListTitle)。
                    let role = if in_def_title {
                        StyleRole::Strong
                    } else {
                        style.role()
                    };
                    spans.push(StyledSpan {
                        text: t.to_string(),
                        role,
                        fill: FillRole::None,
                        attrs: style.attrs(),
                    });
                }
            }
            Event::Code(t) => {
                if in_table {
                    current_cell.push_str(&t);
                    current_cell_spans.push(StyledSpan {
                        text: t.to_string(),
                        role: StyleRole::Code,
                        fill: FillRole::Code,
                        attrs: TextAttrs::none(),
                    });
                } else {
                    if let Some(marker) = pending_item_marker.take() {
                        spans.push(StyledSpan::new(marker, StyleRole::Dim));
                    }
                    spans.push(StyledSpan {
                        text: t.to_string(),
                        role: StyleRole::Code,
                        fill: FillRole::Code,
                        attrs: TextAttrs::none(),
                    });
                }
            }
            Event::InlineMath(math) => {
                if in_table {
                    current_cell.push('$');
                    current_cell.push_str(&math);
                    current_cell.push('$');
                } else {
                    if let Some(marker) = pending_item_marker.take() {
                        spans.push(StyledSpan::new(marker, StyleRole::Dim));
                    }
                    spans.push(StyledSpan {
                        text: format!("${math}$"),
                        role: StyleRole::Math,
                        fill: FillRole::None,
                        attrs: TextAttrs::none(),
                    });
                }
            }
            Event::DisplayMath(math) => {
                if in_table {
                    current_cell.push_str("$$");
                    current_cell.push_str(&math);
                    current_cell.push_str("$$");
                } else {
                    flush_paragraph(
                        &mut doc,
                        &mut spans,
                        current_kind(blockquote_depth, &list_stack),
                        Alignment::Left,
                        blockquote_depth,
                        &mut bq_lines,
                    );
                    let mut lines: Vec<StyledLine> = math
                        .lines()
                        .map(|l| {
                            StyledLine::from_spans(vec![StyledSpan::new(
                                l.to_string(),
                                StyleRole::Math,
                            )])
                        })
                        .collect();
                    if lines.is_empty() {
                        lines.push(StyledLine::from_spans(vec![StyledSpan::new(
                            String::new(),
                            StyleRole::Math,
                        )]));
                    }
                    push_block(&mut doc, BlockKind::MathDisplay, lines);
                }
            }
            Event::FootnoteReference(label) => {
                let text = format!("[^{label}]");
                if in_image {
                    image_alt.push_str(&text);
                } else if in_table {
                    current_cell.push_str(&text);
                } else {
                    spans.push(StyledSpan::new(text, StyleRole::Dim));
                }
            }
            Event::TaskListMarker(checked) => {
                let marker = if checked { "[x] " } else { "[ ] " };
                if in_table {
                    current_cell.push_str(marker);
                } else {
                    if let Some(m) = pending_item_marker.take() {
                        spans.push(StyledSpan::new(m, StyleRole::Dim));
                    }
                    spans.push(StyledSpan::new(marker.to_string(), StyleRole::Dim));
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                if in_table {
                    current_cell.push(' ');
                } else if blockquote_depth > 0 {
                    // Inside a quote, a soft/hard break starts a new quoted line.
                    flush_paragraph(
                        &mut doc,
                        &mut spans,
                        current_kind(blockquote_depth, &list_stack),
                        Alignment::Left,
                        blockquote_depth,
                        &mut bq_lines,
                    );
                } else {
                    spans.push(StyledSpan::plain(" "));
                }
            }
            Event::Html(raw) | Event::InlineHtml(raw) => {
                if in_table {
                    current_cell.push_str(&raw);
                } else {
                    spans.push(StyledSpan {
                        text: raw.to_string(),
                        role: StyleRole::Html,
                        fill: FillRole::None,
                        attrs: TextAttrs {
                            italic: true,
                            ..TextAttrs::none()
                        },
                    });
                }
            }
            Event::Rule => {
                flush_paragraph(
                    &mut doc,
                    &mut spans,
                    BlockKind::Paragraph,
                    Alignment::Left,
                    blockquote_depth,
                    &mut bq_lines,
                );
                push_block(
                    &mut doc,
                    BlockKind::ThematicBreak,
                    vec![StyledLine::from_spans(vec![StyledSpan::new(
                        "─".repeat(3),
                        StyleRole::Dim,
                    )])],
                );
            }

            // ---- block ends ----
            Event::End(TagEnd::Heading(_)) => {
                let level = heading_level.take().unwrap_or(1);
                // Headings render with strong role + bold across the line.
                for s in spans.iter_mut() {
                    s.role = StyleRole::Strong;
                    s.attrs.bold = true;
                }
                flush_paragraph(
                    &mut doc,
                    &mut spans,
                    BlockKind::Heading { level },
                    Alignment::Left,
                    blockquote_depth,
                    &mut bq_lines,
                );
            }
            Event::End(TagEnd::Paragraph) => {
                let kind = current_kind(blockquote_depth, &list_stack);
                flush_paragraph(
                    &mut doc,
                    &mut spans,
                    kind,
                    Alignment::Left,
                    blockquote_depth,
                    &mut bq_lines,
                );
            }
            Event::End(TagEnd::Item) => {
                // Item with no paragraph child (tight list): flush inline buffer.
                if !spans.is_empty() {
                    let kind = current_kind(blockquote_depth, &list_stack);
                    flush_paragraph(
                        &mut doc,
                        &mut spans,
                        kind,
                        Alignment::Left,
                        blockquote_depth,
                        &mut bq_lines,
                    );
                }
                pending_item_marker = None;
            }
            Event::End(TagEnd::List(_)) => {
                if let Some(frame) = list_stack.pop() {
                    if frame.ordered {
                        align_ordered_markers(&mut doc, frame.start_block, frame.depth);
                    }
                }
            }
            Event::End(TagEnd::BlockQuote(_)) => {
                blockquote_depth = blockquote_depth.saturating_sub(1);
                if blockquote_depth == 0 && !bq_lines.is_empty() {
                    push_block(
                        &mut doc,
                        BlockKind::BlockQuote,
                        std::mem::take(&mut bq_lines),
                    );
                }
            }
            Event::End(TagEnd::CodeBlock) => {
                let lines: Vec<StyledLine> = code_buf
                    .trim_end_matches('\n')
                    .split('\n')
                    .map(|l| {
                        StyledLine::from_spans(vec![StyledSpan {
                            text: l.to_string(),
                            role: StyleRole::Code,
                            fill: FillRole::Code,
                            attrs: TextAttrs::none(),
                        }])
                    })
                    .collect();
                push_block(
                    &mut doc,
                    BlockKind::CodeBlock {
                        language: code_lang.take(),
                    },
                    lines,
                );
                in_code_block = false;
                code_buf.clear();
            }
            Event::End(TagEnd::Emphasis) => style.italic = false,
            Event::End(TagEnd::Strong) => style.bold = false,
            Event::End(TagEnd::Strikethrough) => style.strike = false,
            Event::End(TagEnd::Link) => {
                if let Some(url) = link_targets.pop() {
                    // 表格内不追加 ` (url)`:否则 URL 漏进段落缓冲、表后另起一行(bug)。
                    // 格内链接只显文字(已带 Link 角色)。
                    if !url.is_empty() && !in_table {
                        spans.push(StyledSpan::new(format!(" ({url})"), StyleRole::Dim));
                    }
                }
            }
            Event::End(TagEnd::Image) => {
                let alt = if image_alt.trim().is_empty() {
                    "image".to_string()
                } else {
                    image_alt.trim().to_string()
                };
                let url = image_url.take().unwrap_or_default();
                in_image = false;
                image_alt.clear();
                if in_table {
                    // 表格内不做嵌入(几何复杂);退回可读文本。
                    current_cell.push_str(&format!("[image: {alt}]"));
                } else {
                    // 桌面端 = 纹理 quad(Plan 14);TUI = alt。span 文本打包 `url\u{1f}alt`,
                    // role=Image 让前端识别并重建 embed(无 url 字段,故借文本带,见 model.rs)。
                    spans.push(StyledSpan::new(
                        format!("{url}\u{1f}{alt}"),
                        StyleRole::Image,
                    ));
                }
            }

            _ => {}
        }
    }

    // Trailing inline buffer.
    flush_paragraph(
        &mut doc,
        &mut spans,
        BlockKind::Paragraph,
        Alignment::Left,
        blockquote_depth,
        &mut bq_lines,
    );

    doc
}
