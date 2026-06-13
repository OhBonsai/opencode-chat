//! 帧数据(M13→render):core 算出的、与 GPU 无关的最终绘制指令。
//!
//! 注意命名:core 这里是**语义字形** `FrameGlyph`,携带 grapheme cluster 文本 +
//! 几何 + `spawn_time`;render crate 才把它配上 atlas UV、压成 `#[repr(C)]` 的 GPU
//! instance。core 不持有 atlas/UV(那是 GPU 关切,CR1/AR1)。

/// 一个待绘制的字形 = 一个 grapheme cluster。
#[derive(Clone, Debug, PartialEq)]
pub struct FrameGlyph {
    /// grapheme cluster 原文(render 侧据此光栅化/查 atlas)。
    pub cluster: String,
    /// 左上角 world 坐标(world unit = CSS px,见 architecture §10)。
    pub pos: [f32; 2],
    /// 字形宽高(px)。
    pub size: [f32; 2],
    /// 上屏时刻(ms),着色器据 `time - spawn_time` 做淡入(0002 §5)。
    pub spawn_time: f32,
}

/// 一帧交给 [`RenderSink`](crate::RenderSink) 的全部内容。
#[derive(Clone, Debug, Default, PartialEq)]
pub struct FrameData {
    /// 本帧可见字形(已带 spawn_time)。
    pub glyphs: Vec<FrameGlyph>,
    /// 当前帧时间(ms),作为着色器淡入的 `time` uniform。
    pub time_ms: f32,
}
