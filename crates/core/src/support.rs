//! support — native 可测的 seam stub 实现(T6:纯逻辑 native 测,不拉浏览器)。
//!
//! 这些实现是确定性的(等宽几何、无墙钟),既给单测/重放用,也给 Phase C 的合成
//! 体验做基线。生产 wasm 路径用 pretext / wgpu 真实实现替换。

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use unicode_segmentation::UnicodeSegmentation;

use crate::content::{StyledSpan, TableRegion};
use crate::frame::FrameData;
use crate::seam::{
    Connection, LayoutEngine, LayoutResult, MeasuredSize, PlacedGlyph, RawEvent, RenderSink,
};

/// 共享事件队列(Plan 22 P0 / 0031 §3):host(TS transport)往里塞原始事件,引擎 `poll` 取走。
/// 单线程(wasm/native 测)→ `Rc<RefCell<..>>`;**事件入口 = 录像入口**,transport 移 TS 不破重放。
pub type EventQueue = Rc<RefCell<VecDeque<RawEvent>>>;

/// TS 喂队列式连接(Plan 22 P0):取代 Rust 内 SSE。host 经 `push` 塞事件(transport.ts 做真 SSE +
/// 重连/心跳/僵尸,韧性在 TS;0031 §3.1),引擎照常 `poll`。`queue()` 给 host 留把手。
pub struct QueueConnection {
    q: EventQueue,
}

impl Default for QueueConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl QueueConnection {
    #[must_use]
    pub fn new() -> Self {
        Self {
            q: Rc::new(RefCell::new(VecDeque::new())),
        }
    }

    /// 队列把手(host 持一份 clone 用 `push_event` 塞事件)。
    #[must_use]
    pub fn queue(&self) -> EventQueue {
        self.q.clone()
    }

    /// 塞一条原始事件(测试用;host 走 [`push_event`] 持把手塞)。
    pub fn push(&self, raw: impl Into<String>) {
        self.q.borrow_mut().push_back(RawEvent::new(raw));
    }
}

impl Connection for QueueConnection {
    fn poll(&mut self) -> Vec<RawEvent> {
        self.q.borrow_mut().drain(..).collect()
    }
}

/// 经共享把手塞事件(host 侧持 [`EventQueue`] 即可,无需持 `QueueConnection`)。
pub fn push_event(q: &EventQueue, raw: impl Into<String>) {
    q.borrow_mut().push_back(RawEvent::new(raw));
}

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

    /// 廉价 measure(Plan 13 §4.2):只数 cell 折行,**不产 glyph**——与 [`Self::layout`] 的
    /// (最右墨边, 块高)同源,但省去 glyph 分配(Taffy 每叶子调一次,走热路径)。
    fn measure(&mut self, spans: &[StyledSpan], avail_w: f32) -> MeasuredSize {
        let cols = (avail_w / self.cell_w).floor().max(1.0) as usize;
        let (mut col, mut row, mut max_col) = (0usize, 0usize, 0usize);
        for span in spans {
            for cluster in graphemes(span.text()) {
                if cluster == "\n" {
                    row += 1;
                    col = 0;
                    continue;
                }
                if col >= cols {
                    row += 1;
                    col = 0;
                }
                col += 1;
                max_col = max_col.max(col);
            }
        }
        MeasuredSize {
            w: max_col as f32 * self.cell_w,
            h: (row as f32 + 1.0) * self.line_h,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::plain;

    #[test]
    fn measure_matches_layout_derived_size() {
        // Plan 13④:measure 趟必须与 layout 趟同源——(最右墨边, 块高)。等宽 10×18,宽松可用宽。
        let mut eng = MonospaceLayout::default();
        let spans = plain("abc\nde"); // 两行:首行 3 字(宽 30),共 2 行(高 36)
        let m = eng.measure(&spans, 1000.0);
        assert!((m.w - 30.0).abs() < 0.01, "宽应 = 最长行墨边 30: {}", m.w);
        assert!((m.h - 36.0).abs() < 0.01, "高应 = 2 行 ×18 = 36: {}", m.h);
        // 与 layout 派生一致(default trait 路径的不变量)。
        let r = eng.layout(&spans, &[], 1000.0);
        assert!(
            (m.h - r.block_height).abs() < 0.01,
            "measure.h == layout.block_height"
        );
    }

    #[test]
    fn measure_wraps_at_avail_width() {
        // 可用宽只容 2 cell(20px)→ "abcd" 折成 2 行,宽夹到 20、高 36。
        let mut eng = MonospaceLayout::default();
        let m = eng.measure(&plain("abcd"), 20.0);
        assert!(
            (m.w - 20.0).abs() < 0.01,
            "折行后墨边 = 2 cell = 20: {}",
            m.w
        );
        assert!((m.h - 36.0).abs() < 0.01, "折成 2 行 = 36: {}", m.h);
    }
}
