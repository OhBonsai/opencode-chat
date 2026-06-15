//! 平台缝(CR2):core 不直接依赖 EventSource / pretext / wgpu / 时钟,由组装方注入。
//!
//! 这些 trait 让 core 在 native 下可用 stub 实现跑测试(见 [`support`](crate::support)),
//! 在 wasm 下接真实平台能力(见 `crates/wasm`)。

use crate::content::{StyledSpan, TableRegion};
use crate::frame::FrameData;

/// 事件来源(M1)。Player 重放也实现它,故同一套编排既能跑直播也能跑录像。
pub trait Connection {
    /// 取自上次以来到达的原始事件(非阻塞)。
    fn poll(&mut self) -> Vec<RawEvent>;
}

/// 让 `Box<dyn Connection>` 也是 `Connection`,供组装方在运行时切换合成/SSE 事件源。
impl Connection for Box<dyn Connection> {
    fn poll(&mut self) -> Vec<RawEvent> {
        (**self).poll()
    }
}

/// 排版器(M7)。把样式 run 排成带位置的字形;Plan1 纯文本。
pub trait LayoutEngine {
    /// 返回每个 grapheme 的位置 + 该块总高度。glyph 顺序必须与 span 文本的
    /// grapheme 顺序一致(app 据此回填 spawn_time)。`tables` = 该批 span 里的表格结构
    /// (0014 B,run 区间 + 列对齐),供像素两趟对齐 + 格内折行;无表格传空切片。
    fn layout(
        &mut self,
        spans: &[StyledSpan],
        tables: &[TableRegion],
        max_width: f32,
    ) -> LayoutResult;
}

/// 时钟缝(R8)。core 自身用注入的 `dt_ms` 累加时间;Clock 供组装方(wasm 帧循环、
/// 录制器)取墙钟,不在 core 热路径里调用。
pub trait Clock {
    fn now_ms(&self) -> f64;
}

/// 渲染汇(M13→M10)。
pub trait RenderSink {
    fn submit(&mut self, frame: &FrameData);
}

/// SSE `data` 原文,不在 JS 侧解析(BR1)。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RawEvent {
    raw: String,
}

impl RawEvent {
    pub fn new(raw: impl Into<String>) -> Self {
        Self { raw: raw.into() }
    }

    /// 原始 JSON 文本(只读,R4)。
    pub fn raw(&self) -> &str {
        &self.raw
    }
}

/// 一个排好位置的 grapheme(layout→app),**纯位置无文本**:cluster 文本 app 侧已有
/// (来自 smoother 的 reveal),不必跨界重传(CR4 零拷贝)。glyph 顺序须与输入 span 的
/// grapheme 顺序严格 1:1(含换行的零宽占位),app 据此回填 cluster + spawn_time。
#[derive(Clone, Debug, PartialEq)]
pub struct PlacedGlyph {
    pub pos: [f32; 2],
    pub size: [f32; 2],
}

/// 排版结果(layout→app),平铺位置 + 块高度。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayoutResult {
    pub glyphs: Vec<PlacedGlyph>,
    pub block_height: f32,
}
