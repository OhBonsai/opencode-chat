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
    /// 样式角色([`StyleRole`](crate::StyleRole) 的数值):决定 atlas 分桶与着色器上色。
    pub style: u32,
}

/// 一帧交给 [`RenderSink`](crate::RenderSink) 的全部内容。
///
/// 字形 `pos` 为**世界坐标**(Plan 3 L);相机变换在着色器里做,故本帧携带相机 `cam_pan`/
/// `cam_zoom`(viewport 在 render 后端侧)。
#[derive(Clone, Debug, PartialEq)]
pub struct FrameData {
    /// 本帧可见字形(世界坐标 + spawn_time)。
    pub glyphs: Vec<FrameGlyph>,
    /// 当前帧时间(ms),作为着色器淡入的 `time` uniform。
    pub time_ms: f32,
    /// 相机:屏幕左上角对应的世界坐标。
    pub cam_pan: [f32; 2],
    /// 相机缩放。
    pub cam_zoom: f32,
}

impl Default for FrameData {
    fn default() -> Self {
        Self {
            glyphs: Vec::new(),
            time_ms: 0.0,
            cam_pan: [0.0, 0.0],
            cam_zoom: 1.0,
        }
    }
}
