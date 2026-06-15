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
    /// 块序号(append-only 稳定):render 侧 morph Scene 的身份高位(0016 §4.1 / 0017 §6)。
    pub block_seq: u32,
    /// 块内字块序号(append-only 稳定):morph Scene 身份低位。
    pub glyph_idx: u32,
}

/// 一个矩形/圆角图元(Plan 4B:装饰底/边/条 + 4C3 调试几何)。世界坐标,与文字 quad 同
/// 相机/裁剪/实例化;在文字**之前**绘制(作背景)。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameRect {
    /// 左上角世界坐标。
    pub pos: [f32; 2],
    /// 宽高。
    pub size: [f32; 2],
    /// 颜色 RGBA(预乘前;`a<1` 半透明叠底)。
    pub color: [f32; 4],
    /// 圆角半径(px);0 = 直角。
    pub radius: f32,
    /// 描边宽度(px);0 = 实心填充,>0 = 仅边框(调试框用)。
    pub stroke: f32,
}

/// 一帧交给 [`RenderSink`](crate::RenderSink) 的全部内容。
///
/// 字形 `pos` 为**世界坐标**(Plan 3 L);相机变换在着色器里做,故本帧携带相机 `cam_pan`/
/// `cam_zoom`(viewport 在 render 后端侧)。`rects` 作背景先于 `glyphs` 绘制(4B)。
#[derive(Clone, Debug, PartialEq)]
pub struct FrameData {
    /// 背景/装饰/调试矩形(先绘制)。
    pub rects: Vec<FrameRect>,
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
            rects: Vec::new(),
            glyphs: Vec::new(),
            time_ms: 0.0,
            cam_pan: [0.0, 0.0],
            cam_zoom: 1.0,
        }
    }
}
