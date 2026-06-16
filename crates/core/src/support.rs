//! support — native 可测的 seam stub 实现(T6:纯逻辑 native 测,不拉浏览器)。
//!
//! 这些实现是确定性的(等宽几何、无墙钟),既给单测/重放用,也给 Phase C 的合成
//! 体验做基线。生产 wasm 路径用 pretext / wgpu 真实实现替换。

use unicode_segmentation::UnicodeSegmentation;

use crate::content::{StyledSpan, TableRegion};
use crate::frame::FrameData;
use crate::seam::{LayoutEngine, LayoutResult, PlacedGlyph, RenderSink};

/// 按 grapheme cluster 切分(AR7 的统一入口)。
pub(crate) fn graphemes(text: &str) -> Vec<&str> {
    text.graphemes(true).collect()
}

/// 等宽排版 stub:每个 grapheme 占一个固定 cell,按 `max_width` 折行。
/// 顺序与输入 grapheme 顺序严格一致(app 据此回填 spawn_time)。
pub struct MonospaceLayout {
    cell_w: f32,
    line_h: f32,
}

impl MonospaceLayout {
    pub fn new(cell_w: f32, line_h: f32) -> Self {
        Self { cell_w, line_h }
    }
}

impl Default for MonospaceLayout {
    fn default() -> Self {
        Self::new(10.0, 18.0)
    }
}

impl LayoutEngine for MonospaceLayout {
    fn layout(
        &mut self,
        spans: &[StyledSpan],
        _tables: &[TableRegion],
        max_width: f32,
    ) -> LayoutResult {
        let cols = (max_width / self.cell_w).floor().max(1.0) as usize;
        let mut glyphs = Vec::new();
        let mut col = 0usize;
        let mut row = 0usize;
        // 严格一字形对一 grapheme(含换行的零宽占位),保证 app 端 spawn_time 1:1 对齐。
        for span in spans {
            for cluster in graphemes(span.text()) {
                if cluster == "\n" {
                    glyphs.push(PlacedGlyph {
                        pos: [col as f32 * self.cell_w, row as f32 * self.line_h],
                        size: [0.0, self.line_h],
                    });
                    row += 1;
                    col = 0;
                    continue;
                }
                if col >= cols {
                    row += 1;
                    col = 0;
                }
                glyphs.push(PlacedGlyph {
                    pos: [col as f32 * self.cell_w, row as f32 * self.line_h],
                    size: [self.cell_w, self.line_h],
                });
                col += 1;
            }
        }
        let block_height = (row as f32 + 1.0) * self.line_h;
        LayoutResult {
            glyphs,
            block_height,
            table_panels: Vec::new(),
        }
    }
}

/// 丢弃帧的渲染汇(只想驱动逻辑、不关心像素时用)。
#[derive(Default)]
pub struct NullSink;

impl RenderSink for NullSink {
    fn submit(&mut self, _frame: &FrameData) {}
}

/// 收集最后一帧的渲染汇,供测试断言可见内容。
#[derive(Default)]
pub struct CollectSink {
    last: Option<FrameData>,
}

impl CollectSink {
    /// 最近一帧。
    pub fn last(&self) -> Option<&FrameData> {
        self.last.as_ref()
    }

    /// 最近一帧拼起来的可见文本(按 glyph 顺序)。
    pub fn visible_text(&self) -> String {
        self.last
            .as_ref()
            .map(|f| f.glyphs.iter().map(|g| g.cluster.as_str()).collect())
            .unwrap_or_default()
    }
}

impl RenderSink for CollectSink {
    fn submit(&mut self, frame: &FrameData) {
        self.last = Some(frame.clone());
    }
}
