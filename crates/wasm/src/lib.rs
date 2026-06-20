//! infinite-chat-wasm(M12 api / M1 transport / 平台胶水)。
//!
//! 薄 `#[wasm_bindgen]` 层(CR5):业务逻辑全在 core,这里只做平台接缝——从 canvas 建
//! wgpu surface、连 SSE、调 JS 排版/光栅化、跑 requestAnimationFrame 帧循环。
//!
//! 整个 crate 仅 `wasm32` 目标有内容;native `cargo build --workspace` 把它当空 lib
//! 跳过(平台代码无法在 native 链接)。真实编译验证:
//! `cargo build -p infinite-chat-wasm --target wasm32-unknown-unknown`。
#![cfg(target_arch = "wasm32")]

mod clock;
mod glyph_bridge;
mod layout_bridge;
mod msdf;
mod observe;
mod transport;

use std::cell::RefCell;
use std::rc::Rc;

use infinite_chat_core::{
    Clock, Connection, Engine, FrameData, FrameGlyph, Player, Record, RenderSink, TableStyle,
};
use infinite_chat_render::{
    EffectProfile, Geom, NodeId, PanelGeom, PanelScene, RenderBackend, Sample, Scene, WebGpuBackend,
};
use wasm_bindgen::prelude::*;

use crate::clock::WebClock;
use crate::layout_bridge::LayoutBridge;
use crate::transport::{fetch_snapshot, SseConnection};

type AppEngine = Engine<Box<dyn Connection>, LayoutBridge, GpuSink>;
type SharedState = Rc<RefCell<Option<AppState>>>;
type RafHandle = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

/// 周期性对账间隔(ms,Phase J)。
const RESYNC_MS: f64 = 20_000.0;

/// 调试器数据快照(Plan 4C1):每秒由帧循环更新,`ChatCanvas.stats()` 读出给 DOM 面板。
#[derive(Clone, Copy, Default)]
struct StatsSnapshot {
    fps: f64,
    frame_ms_avg: f64,
    frame_ms_max: f64,
    dropped: u32,
    glyphs_visible: usize,
    glyphs_total: usize,
    blocks_visible: usize,
    blocks_total: usize,
    shaderbox_active: usize,
    shaderbox_pixels: u64,
    atlas_used: usize,
    atlas_cap: usize,
    atlas_evict: u64,
    cam_zoom: f32,
    /// 逐源计数 `[bitmap, tinysdf, msdf, rgba]`(0015:MSDF 命中率)。
    src_counts: [u32; 4],
}
type StatsCell = Rc<RefCell<StatsSnapshot>>;
type Flag = Rc<RefCell<bool>>;

/// 字形渲染方案(0015 §2.6 调试器全局切)。数值与 JS 端 `GlyphMode` 一致。
#[derive(Clone, Copy, PartialEq, Eq)]
enum GlyphMode {
    /// MSDF 命中 → MSDF,否则回退 TinySDF(默认)。
    Auto = 0,
    /// 全部走 Canvas 位图覆盖率(轴 A 的另一套方案)。
    Bitmap = 1,
    /// 禁 MSDF,全部 TinySDF(验回退)。
    ForceTinySdf = 2,
    /// 强制 MSDF(看烘集覆盖空洞;未命中仍只能回退 TinySDF)。
    ForceMsdf = 3,
}

impl GlyphMode {
    fn from_u32(v: u32) -> Self {
        match v {
            1 => Self::Bitmap,
            2 => Self::ForceTinySdf,
            3 => Self::ForceMsdf,
            _ => Self::Auto,
        }
    }
}

/// 字形源 kind(并入 atlas key + GpuInstance.kind;与 glyph.wgsl 分支一致)。
const KIND_BITMAP: u32 = 0;
const KIND_TINYSDF: u32 = 1;
const KIND_MSDF: u32 = 2;
const KIND_RGBA: u32 = 3;

/// 快判定彩色 emoji(0015 §7.3):码点区间谓词,O(码点数),不栅格。命中主力 emoji plane +
/// VS16/ZWJ/旗帜;**故意不收** ★(U+2605)/▲(U+25B2)/•(U+2022) 等无默认 emoji 呈现的符号
/// (它们留单色 SDF + 文字色 tint)。v1 缺口:U+2600–26FF 区少数默认彩 emoji 漏判 → 回退单色。
fn is_color_emoji(cluster: &str) -> bool {
    cluster.chars().any(|c| {
        let u = c as u32;
        u == 0xFE0F // VS16:显式 emoji 呈现
            || u == 0x200D // ZWJ:emoji 连写序列
            || (0x1F000..=0x1FAFF).contains(&u) // 象形/表情/符号扩展 plane(主力)
            || (0x1F1E6..=0x1F1FF).contains(&u) // 区域指示符(旗帜)
    })
}

/// 源解析结果(0015 §2.2):决定该字走哪条路。
enum Source {
    /// 位图覆盖率 / TinySDF 距离场:走运行时 R8 图集(携带 kind)。
    Raster(u32),
    /// MSDF 命中:用 baked 静态图集 + BMFont metrics,不走运行时图集。
    Msdf(msdf::MsdfGlyph),
    /// ForceMSDF 下未命中:留空洞(不绘制),用来看烘集覆盖。
    Skip,
}

/// 渲染汇:把 core 的语义字形按需光栅化进图集,再交后端绘制。
struct GpuSink {
    backend: WebGpuBackend,
    rasterize_fn: js_sys::Function,
    profile: EffectProfile,
    /// 字体代:并入 atlas key,换字体时 +1 → 旧字形 key 失配、重栅,老 tile 走 LRU 自然淘汰。
    font_gen: u32,
    /// 字形渲染方案(调试器切)。
    glyph_mode: GlyphMode,
    /// 本帧逐源计数 `[bitmap, tinysdf, msdf, rgba]`(可观测:MSDF 命中率)。
    src_counts: [u32; 4],
    /// 离线烘焙 MSDF 字体(coverage + metrics);None = 未加载(0015 §2.3)。
    msdf_font: Option<msdf::MsdfFont>,
    /// streaming 形变保留态场景(0016):几何 past→current 补间,不跳变。
    scene: Scene,
    /// 表格面板形变保留态(0018 §5 / Plan 6D):框/网格随列变宽补间,与字 `scene` 同 dur(框字同步)。
    panel_scene: PanelScene,
    /// 上一帧的动图嵌入世界矩形(Plan 14 ⑥):供 `frame_embeds()` 转屏幕坐标喂 DOM overlay。
    last_embeds: Vec<infinite_chat_core::FrameEmbed>,
}

impl GpuSink {
    fn resize(&mut self, width: u32, height: u32) {
        self.backend.resize(width, height);
    }

    /// atlas 可观测:(占用, 容量, 累计淘汰)。
    fn atlas_stats(&self) -> (usize, usize, u64) {
        self.backend.atlas_stats()
    }

    /// 换字体代:之后所有字形 key 变化 → 用新字体重新光栅化。
    fn bump_font_gen(&mut self) {
        self.font_gen = self.font_gen.wrapping_add(1);
    }

    /// 切换字形渲染方案(0015 §2.6)。
    fn set_glyph_mode(&mut self, mode: GlyphMode) {
        self.glyph_mode = mode;
    }

    /// 本帧逐源计数 `[bitmap, tinysdf, msdf, rgba]`。
    fn source_counts(&self) -> [u32; 4] {
        self.src_counts
    }

    /// 加载 baked MSDF:解析元数据 + 逐页上传像素到静态图集(0015 §2.3)。
    fn load_msdf(&mut self, meta: &JsValue) -> Result<(), String> {
        let font = msdf::MsdfFont::from_js(meta)?;
        let pages = js_sys::Array::from(
            &js_sys::Reflect::get(meta, &JsValue::from_str("pixels"))
                .map_err(|_| "缺 pixels".to_string())?,
        );
        let count = pages.length();
        self.backend
            .msdf_init(font.atlas_w as u32, font.atlas_h as u32, count.max(1));
        for p in 0..count {
            let bytes = js_sys::Uint8Array::from(pages.get(p)).to_vec();
            self.backend.msdf_upload(p, &bytes);
        }
        tracing::info!(target: "M8", "MSDF 加载:{} 字 / {count} 页", font.len());
        self.msdf_font = Some(font);
        Ok(())
    }

    /// 解析某字形的源(0015 §2.2 源解析器 + 回退链)。`style` = 角色(数学角色另走 KaTeX MSDF)。
    fn resolve(&self, cluster: &str, style: u32) -> Source {
        // 数学字形(Plan 12 ④,角色 26–40):查 **KaTeX MSDF atlas**(合成键 `role*0x110000+codepoint`,
        // 与正文 codepoint key 空间不撞)→ 命中 MSDF(任意缩放锐利);未命中(未加载/缺字)回退 TinySDF
        // (KaTeX woff2 经 canvas)。数学不走 lxgw 的 codepoint MSDF(会与正文字撞)。
        if (26..=40).contains(&style) {
            if self.backend.msdf_loaded() {
                if let Some(font) = &self.msdf_font {
                    let mut it = cluster.chars();
                    if let (Some(c), None) = (it.next(), it.next()) {
                        let syn = style * 0x0011_0000 + c as u32;
                        if let Some(g) = font.glyph(syn) {
                            return Source::Msdf(*g);
                        }
                    }
                }
            }
            return Source::Raster(KIND_TINYSDF);
        }
        // 彩色 emoji → RGBA 动态图集(0015 §7):不进 MSDF/单色路,直采真彩。
        if is_color_emoji(cluster) {
            return Source::Raster(KIND_RGBA);
        }
        // MSDF 仅对单 codepoint 簇命中(BMFont 按码点烘);多码点簇(emoji ZWJ 等)不走 MSDF。
        let want_msdf = matches!(self.glyph_mode, GlyphMode::Auto | GlyphMode::ForceMsdf)
            && self.backend.msdf_loaded();
        if want_msdf {
            if let Some(font) = &self.msdf_font {
                let mut it = cluster.chars();
                if let (Some(c), None) = (it.next(), it.next()) {
                    if let Some(g) = font.glyph(c as u32) {
                        return Source::Msdf(*g);
                    }
                }
            }
            // 未命中:Auto 回退 TinySDF;ForceMSDF 留空洞(看覆盖)。
            if matches!(self.glyph_mode, GlyphMode::ForceMsdf) {
                return Source::Skip;
            }
        }
        match self.glyph_mode {
            GlyphMode::Bitmap => Source::Raster(KIND_BITMAP),
            _ => Source::Raster(KIND_TINYSDF),
        }
    }
}

/// 从方格几何(`g.pos`/`g.size` = TILE 方格世界坐标)+ BMFont metrics 算 MSDF quad 的世界
/// 几何(0015 §2.5)+ 不插值载荷。方格里字形落在 `[SDF_BUFFER, TILE-SDF_BUFFER]`,em = `FONT_PX`
/// 占比;据此反推 pen 原点 / em 盒,再按 BMFont 字号缩放放置。
fn msdf_node(
    g: &FrameGlyph,
    m: &msdf::MsdfGlyph,
    font_size: f32,
    atlas: (f32, f32),
) -> (Geom, Sample) {
    let tile = infinite_chat_render::TILE_PX as f32;
    let buf = infinite_chat_render::SDF_BUFFER as f32;
    let cell = g.size[0]; // 方格世界边长
    let scale = cell / tile; // 世界 px / tile px
    let off = buf * scale; // 方格内字形留白(世界 px)
    let em = (tile - 2.0 * buf) * scale; // em 盒(世界 px)
    let k = em / font_size.max(1.0); // BMFont 单位 → 世界 px
    let pen_x = g.pos[0] + off;
    let top = g.pos[1] + off;
    let (aw, ah) = atlas;
    (
        Geom {
            pos: [pen_x + m.xoff * k, top + m.yoff * k],
            size: [m.w * k, m.h * k],
            alpha: g.alpha, // Plan 15:行窗边缘淡入淡出(非代码块恒 1)
        },
        Sample {
            uv: [m.x / aw, m.y / ah, (m.x + m.w) / aw, (m.y + m.h) / ah],
            style: g.style,
            layer: m.page,
            kind: KIND_MSDF,
            spawn_time: g.spawn_time,
            anim: g.anim,
        },
    )
}

impl RenderSink for GpuSink {
    fn submit(&mut self, frame: &FrameData) {
        self.last_embeds = frame.embeds.clone(); // Plan 14 ⑥:动图世界矩形留给 DOM overlay
        self.backend.atlas_begin_frame();
        self.src_counts = [0; 4];
        let now = frame.time_ms;
        // 1) 解析源 + atlas + 算几何/载荷 → 活跃区布局快照(带稳定 NodeId,0016 §7)。
        let mut snapshot: Vec<(NodeId, Geom, Sample)> = Vec::with_capacity(frame.glyphs.len());
        for g in &frame.glyphs {
            let id = NodeId::new(g.block_seq, g.glyph_idx);
            match self.resolve(&g.cluster, g.style) {
                Source::Msdf(m) => {
                    self.src_counts[KIND_MSDF as usize] += 1;
                    let (aw, ah, size) = self
                        .msdf_font
                        .as_ref()
                        .map_or((1.0, 1.0, 1.0), |f| (f.atlas_w, f.atlas_h, f.size));
                    let (geom, sample) = msdf_node(g, &m, size, (aw, ah));
                    snapshot.push((id, geom, sample));
                }
                Source::Skip => {}
                Source::Raster(kind) => {
                    self.src_counts[kind as usize] += 1;
                    // atlas 按 (font_gen, kind, style, cluster) 分桶;font_gen/kind 变化让 key 失配
                    // 触发重栅(render 与此处同 key)。
                    let key = format!(
                        "{}\u{1}{}\u{1}{}",
                        self.font_gen,
                        kind,
                        infinite_chat_render::glyph_key(g.style, &g.cluster)
                    );
                    self.backend.atlas_pin(&key);
                    let a = self.backend.atlas_alloc(&key);
                    if a.is_new {
                        if let Some(tile) =
                            glyph_bridge::rasterize(&self.rasterize_fn, &g.cluster, g.style, kind)
                        {
                            self.backend.atlas_upload(a.slot, &tile);
                        }
                    }
                    snapshot.push((
                        id,
                        Geom {
                            pos: g.pos,
                            size: g.size,
                            alpha: g.alpha, // Plan 15:行窗边缘淡入淡出(非代码块恒 1)
                        },
                        Sample {
                            uv: a.slot.uv(),
                            style: g.style,
                            layer: a.slot.page,
                            kind,
                            spawn_time: g.spawn_time,
                            anim: g.anim,
                        },
                    ));
                }
            }
        }
        // 2) 提交快照 → join(0016 §4.4);3) 插值发射(几何 past→current 补间,不跳变)。
        self.scene.commit(&snapshot, now);
        let instances = self.scene.instances(now);
        // 装饰/调试矩形(Plan 4B):core 已算好世界坐标,直接平铺为 instance。
        let rects: Vec<infinite_chat_render::RectInstance> = frame
            .rects
            .iter()
            .map(|r| infinite_chat_render::RectInstance {
                pos: r.pos,
                size: r.size,
                color: r.color,
                radius: r.radius,
                stroke: r.stroke,
            })
            .collect();
        // markdown 组件(0026/Plan 11):core 已算好世界坐标 + 参数,直接平铺为 widget instance。
        let widgets: Vec<infinite_chat_render::WidgetInstance> = frame
            .widgets
            .iter()
            .map(|w| infinite_chat_render::WidgetInstance {
                pos: w.pos,
                size: w.size,
                color: w.color,
                params: w.params,
                component: w.component,
            })
            .collect();
        // SDF 面板(Plan 6 / 0018):先把本帧面板**几何**提交 panel_scene join(6D:列随吐字变宽
        // 补间,与字 scene 同 dur → 框字同步),再用**插值后的**几何(box/header/col/row)+ 快照样式
        // (色/AO/线宽,不补间)扁平进共享 params buffer。参数块布局须与 panel.wgsl 一致。
        let incoming: Vec<(u64, PanelGeom)> = frame
            .panels
            .iter()
            .map(|p| {
                (
                    p.id,
                    PanelGeom {
                        pos: p.pos,
                        size: p.size,
                        header_ratio: p.header_ratio,
                        col_ratios: p.col_ratios.clone(),
                        row_ratios: p.row_ratios.clone(),
                    },
                )
            })
            .collect();
        self.panel_scene.commit(&incoming, now);
        let mut params: Vec<f32> = Vec::new();
        let panels: Vec<infinite_chat_render::PanelInstance> = frame
            .panels
            .iter()
            .map(|p| {
                // 插值几何(缺失 → 回退原始几何);样式(色/AO/线宽/圆角/flags)取最新,不补间。
                let g = self.panel_scene.displayed(p.id, now).unwrap_or(PanelGeom {
                    pos: p.pos,
                    size: p.size,
                    header_ratio: p.header_ratio,
                    col_ratios: p.col_ratios.clone(),
                    row_ratios: p.row_ratios.clone(),
                });
                let offset = params.len() as u32;
                params.extend_from_slice(&p.fill);
                params.extend_from_slice(&p.line_color);
                params.extend_from_slice(&p.header_fill);
                params.push(p.line_w);
                params.push(p.ao);
                params.push(g.header_ratio); // 插值
                params.push(g.col_ratios.len() as f32);
                params.push(g.row_ratios.len() as f32);
                params.extend_from_slice(&p.ao_color); // [17..20]
                params.push(p.ao_width); // [20]
                params.push(p.reveal); // [21] 纵向揭示比例(不插值,逐帧;0019 风格化骨架)
                params.extend_from_slice(&g.col_ratios); // [22..22+n_cols] 插值
                params.extend_from_slice(&g.row_ratios); // 插值
                infinite_chat_render::PanelInstance {
                    pos: g.pos,   // 插值
                    size: g.size, // 插值
                    radius: p.radius,
                    param_offset: offset,
                    param_len: params.len() as u32 - offset,
                    flags: p.flags,
                }
            })
            .collect();
        // 图片纹理 quad(Plan 14 ②③):纹理已由 upload_image_rgba 上传(tex_id 存于 FrameImage)。
        let (image_insts, image_tex_ids): (Vec<infinite_chat_render::ImageInstance>, Vec<u32>) =
            frame
                .images
                .iter()
                .map(|im| {
                    (
                        infinite_chat_render::ImageInstance {
                            pos: im.pos,
                            size: im.size,
                            alpha: im.alpha,
                            radius: im.radius,
                        },
                        im.tex_id,
                    )
                })
                .unzip();
        // ShaderBox 画板(Plan 16):shader_id 选 pipeline、params/bg/time 喂 uniform(实例)。
        let (sb_insts, sb_ids): (Vec<infinite_chat_render::ShaderBoxInstance>, Vec<u32>) = frame
            .shaderboxes
            .iter()
            .map(|sb| {
                (
                    infinite_chat_render::ShaderBoxInstance {
                        pos: sb.pos,
                        size: sb.size,
                        params0: [sb.params[0], sb.params[1], sb.params[2], sb.params[3]],
                        params1: [sb.params[4], sb.params[5], sb.params[6], sb.params[7]],
                        bg: sb.bg,
                        time: sb.time,
                        _pad: [0.0; 3],
                    },
                    sb.shader_id,
                )
            })
            .unzip();
        if let Err(e) = self.backend.draw(
            &instances,
            &rects,
            &panels,
            &params,
            &widgets,
            &image_insts,
            &image_tex_ids,
            &sb_insts,
            &sb_ids,
            frame.time_ms,
            self.profile.fade_ms(),
            frame.cam_pan,
            frame.cam_zoom,
        ) {
            tracing::warn!(target: "M10", "draw 失败: {e}");
        }
    }
}

struct AppState {
    engine: AppEngine,
}

/// 宿主面向的画布组件。
#[wasm_bindgen]
pub struct ChatCanvas {
    canvas: web_sys::HtmlCanvasElement,
    layout_fn: js_sys::Function,
    /// 可选 measure 回调(Plan 13 §4.2);缺省时 LayoutBridge 退回 layout 派生尺寸。
    measure_fn: Option<js_sys::Function>,
    rasterize_fn: js_sys::Function,
    server_url: Option<String>,
    session_id: Option<String>,
    /// 重放记录(Plan 5D):传入则用 `Player` 喂预录事件,替代 SSE/合成流(不连服务端)。
    replay: Option<Vec<Record>>,
    state: SharedState,
    raf: RafHandle,
    stats: StatsCell,
    paused: Flag,
    step: Flag,
}

#[wasm_bindgen]
impl ChatCanvas {
    /// `config`:`{ layout, rasterize, serverUrl?, sessionId? }`。
    /// - `layout(text, maxWidth) => Float32Array`([x,y,w,h]*N)
    /// - `rasterize(cluster) => { data: Uint8Array, width, height }`
    /// - 不传 `serverUrl` 时跑合成流(Phase C 体验演示)。
    #[wasm_bindgen(constructor)]
    #[allow(clippy::needless_pass_by_value)] // reason: wasm_bindgen 构造器按值接收 JsValue
    pub fn new(canvas: web_sys::HtmlCanvasElement, config: JsValue) -> Result<ChatCanvas, JsValue> {
        observe::init();
        let layout_fn = get_fn(&config, "layout")?;
        let measure_fn = get_fn(&config, "measure").ok(); // 可选(Plan 13 §4.2);缺省退回 layout 派生
        let rasterize_fn = get_fn(&config, "rasterize")?;
        Ok(Self {
            canvas,
            layout_fn,
            measure_fn,
            rasterize_fn,
            server_url: get_str(&config, "serverUrl"),
            session_id: get_str(&config, "sessionId"),
            replay: get_replay(&config, "replay"),
            state: Rc::new(RefCell::new(None)),
            raf: Rc::new(RefCell::new(None)),
            stats: Rc::new(RefCell::new(StatsSnapshot::default())),
            paused: Rc::new(RefCell::new(false)),
            step: Rc::new(RefCell::new(false)),
        })
    }

    /// 调试器数据通道(Plan 4C1):返回 `{ fps, frameMsAvg, glyphsVisible/Total, atlasUsed/Cap/Evict, … }`。
    pub fn stats(&self) -> JsValue {
        let obj = js_sys::Object::new();
        let set = |k: &str, v: f64| {
            let _ = js_sys::Reflect::set(&obj, &JsValue::from_str(k), &JsValue::from_f64(v));
        };
        let s = *self.stats.borrow();
        set("fps", s.fps);
        set("frameMsAvg", s.frame_ms_avg);
        set("frameMsMax", s.frame_ms_max);
        set("dropped", f64::from(s.dropped));
        set("glyphsVisible", s.glyphs_visible as f64);
        set("glyphsTotal", s.glyphs_total as f64);
        set("blocksVisible", s.blocks_visible as f64);
        set("blocksTotal", s.blocks_total as f64);
        set("shaderboxActive", s.shaderbox_active as f64);
        set("shaderboxPixels", s.shaderbox_pixels as f64);
        set("atlasUsed", s.atlas_used as f64);
        set("atlasCap", s.atlas_cap as f64);
        set("atlasEvict", s.atlas_evict as f64);
        set("camZoom", f64::from(s.cam_zoom));
        set("paused", f64::from(u8::from(*self.paused.borrow())));
        set("srcBitmap", f64::from(s.src_counts[0]));
        set("srcTinySdf", f64::from(s.src_counts[1]));
        set("srcMsdf", f64::from(s.src_counts[2]));
        set("srcRgba", f64::from(s.src_counts[3]));
        obj.into()
    }

    /// 切换字形渲染方案(0015 §2.6):0=Auto / 1=Bitmap / 2=ForceTinySDF / 3=ForceMSDF。
    /// 仅改源(key 含 kind → 重栅),advance 不变故无需重排。
    pub fn set_glyph_mode(&self, mode: u32) {
        if let Some(app) = self.state.borrow_mut().as_mut() {
            app.engine
                .sink_mut()
                .set_glyph_mode(GlyphMode::from_u32(mode));
        }
    }

    /// 平移画布 `dx,dy` 屏幕像素(web 层 wheel/拖拽统一调用,Plan 6)。dy>0 看更新内容,
    /// dx>0 看右侧(宽表溢出)。横向自由、纵向锚底由 core 处理。
    pub fn pan_by(&self, dx: f32, dy: f32) {
        if let Some(app) = self.state.borrow_mut().as_mut() {
            app.engine.pan_by(dx, dy);
        }
    }

    /// 围绕屏幕点 `(sx,sy)`(设备像素)缩放 `factor`(web 层 ctrl+wheel/捏合调用)。factor>1 放大。
    pub fn zoom_at(&self, factor: f32, sx: f32, sy: f32) {
        if let Some(app) = self.state.borrow_mut().as_mut() {
            app.engine.zoom_by(factor, sx, sy);
        }
    }

    /// 领取待解码图片(Plan 14 ③):返回 JSON `[{key,url}]`(key 为字符串避 u64/JS 精度问题),
    /// 并把这些嵌入转 Loading。web `image-loader` 每帧轮询 → 解码/上传 → 回调 `image_ready`/`image_failed`。
    pub fn take_pending_images(&self) -> String {
        let mut guard = self.state.borrow_mut();
        let Some(app) = guard.as_mut() else {
            return "[]".to_string();
        };
        let items: Vec<String> = app
            .engine
            .take_pending_images()
            .iter()
            .map(|(k, url)| format!(r#"{{"key":"{k}","url":{url:?}}}"#))
            .collect();
        format!("[{}]", items.join(","))
    }

    /// 图片解码完成(Plan 14 ③):上传 RGBA 首帧到 GPU 纹理 → 推进该 key 嵌入到 Ready(记 tex_id/
    /// 自然尺寸/动图标志)。`key` = `take_pending_images` 给的字符串;`rgba` = w×h×4 sRGB 字节。
    pub fn upload_image_rgba(&self, key: &str, rgba: &[u8], w: u32, h: u32, animated: bool) {
        let Ok(k) = key.parse::<u64>() else { return };
        if let Some(app) = self.state.borrow_mut().as_mut() {
            let tex_id = app.engine.sink_mut().backend.upload_image(rgba, w, h);
            if tex_id == 0 {
                app.engine.image_failed(k); // 上传失败 → alt 兜底
            } else {
                app.engine
                    .image_ready(k, tex_id, w as f32, h as f32, animated);
            }
        }
    }

    /// 图片解码/网络失败(Plan 14 ③):该 key 嵌入 → Failed(显 alt 兜底)。
    pub fn image_failed(&self, key: &str) {
        let Ok(k) = key.parse::<u64>() else { return };
        if let Some(app) = self.state.borrow_mut().as_mut() {
            app.engine.image_failed(k);
        }
    }

    /// 屏幕点(设备像素)命中哪个代码块行窗(Plan 15 ④)→ 返回 key 字符串(空 = 未命中)。input 层
    /// wheel/drag 据此决定滚块还是滚画布。
    pub fn code_block_at_screen(&self, sx: f32, sy: f32) -> String {
        let guard = self.state.borrow();
        let Some(app) = guard.as_ref() else {
            return String::new();
        };
        let cam = app.engine.camera();
        let pan = cam.pan();
        let zoom = cam.zoom().max(f32::EPSILON);
        // 屏幕(设备像素)→ 世界:world = screen/zoom + pan(visible_world_rect 的逆)。
        let (wx, wy) = (sx / zoom + pan[0], sy / zoom + pan[1]);
        app.engine
            .code_block_at(wx, wy)
            .map(|k| k.to_string())
            .unwrap_or_default()
    }

    /// 块内滚动(Plan 15 ④):`dx` px 横滚、`dy_lines` 行纵滚。`key` = `code_block_at_screen` 给的串。
    pub fn scroll_code_block(&self, key: &str, dx: f32, dy_lines: i32) {
        let Ok(k) = key.parse::<u64>() else { return };
        if let Some(app) = self.state.borrow_mut().as_mut() {
            app.engine.scroll_code_block(k, dx, dy_lines);
        }
    }

    /// 动图嵌入的屏幕矩形(Plan 14 ⑥):JSON `[{key,url,x,y,w,h}]`,坐标 = 设备像素(world→screen
    /// 经相机 pan/zoom)。`embed-overlay` 据此定位 `<img>` 叠在 canvas 上让浏览器自播(除以 DPR 转 CSS)。
    pub fn frame_embeds(&self) -> String {
        let guard = self.state.borrow();
        let Some(app) = guard.as_ref() else {
            return "[]".to_string();
        };
        let cam = app.engine.camera();
        let pan = cam.pan();
        let zoom = cam.zoom();
        let items: Vec<String> = app
            .engine
            .sink()
            .last_embeds
            .iter()
            .map(|e| {
                let x = (e.pos[0] - pan[0]) * zoom;
                let y = (e.pos[1] - pan[1]) * zoom;
                let w = e.size[0] * zoom;
                let h = e.size[1] * zoom;
                format!(
                    r#"{{"key":"{}","url":{:?},"x":{x},"y":{y},"w":{w},"h":{h}}}"#,
                    e.key, e.url
                )
            })
            .collect();
        format!("[{}]", items.join(","))
    }

    /// 设表格面板渲染样式(web 层 style 面板实时调;Plan 6 / 0018)。**无需重排/reload**:
    /// `block_decorations` 每帧读 → 下一帧即生效。`cfg` 为对象,字段缺省则保留默认:
    /// `{ lineColor:[r,g,b,a], headerFill:[r,g,b,a], aoColor:[r,g,b], lineW, ao, aoWidth, radius }`
    /// (颜色分量 0..1)。
    #[allow(clippy::needless_pass_by_value)] // reason: wasm_bindgen 按值接收 JsValue
    pub fn set_table_style(&self, cfg: JsValue) {
        if let Some(app) = self.state.borrow_mut().as_mut() {
            let mut s = TableStyle::default();
            if let Some(c) = get_f32_arr(&cfg, "lineColor", 4) {
                s.line_color = [c[0], c[1], c[2], c[3]];
            }
            if let Some(c) = get_f32_arr(&cfg, "headerFill", 4) {
                s.header_fill = [c[0], c[1], c[2], c[3]];
            }
            if let Some(c) = get_f32_arr(&cfg, "aoColor", 3) {
                s.ao_color = [c[0], c[1], c[2]];
            }
            if let Some(n) = get_f32(&cfg, "lineW") {
                s.line_w = n;
            }
            if let Some(n) = get_f32(&cfg, "ao") {
                s.ao = n;
            }
            if let Some(n) = get_f32(&cfg, "aoWidth") {
                s.ao_width = n;
            }
            if let Some(n) = get_f32(&cfg, "radius") {
                s.radius = n;
            }
            app.engine.set_table_style(s);
        }
    }

    /// 加载离线烘焙 MSDF 字体(0015 §2.3)。`meta`:`{ atlasW, atlasH, fontSize, ids:
    /// Uint32Array, cells: Float32Array(每字 7 个), pixels: Uint8Array[](逐页 RGBA) }`。
    /// JS 侧 fetch BMFont json + 解码 PNG 后调用;成功后 Auto/ForceMSDF 模式即命中烘集。
    #[allow(clippy::needless_pass_by_value)] // reason: wasm_bindgen 按值接收 JsValue
    pub fn load_msdf(&self, meta: JsValue) -> Result<(), JsValue> {
        if let Some(app) = self.state.borrow_mut().as_mut() {
            app.engine
                .sink_mut()
                .load_msdf(&meta)
                .map_err(|e| JsValue::from_str(&e))?;
        }
        Ok(())
    }

    /// 暂停 / 恢复帧推进(渲染冻结在最后一帧,Plan 4C2)。
    pub fn set_paused(&self, paused: bool) {
        *self.paused.borrow_mut() = paused;
    }

    /// 暂停时单步推进一帧。
    pub fn step(&self) {
        *self.step.borrow_mut() = true;
    }

    /// 切换引擎自绘调试几何:块 AABB 描边 + 视口框(Plan 4C3)。
    pub fn set_debug_geometry(&self, on: bool) {
        if let Some(app) = self.state.borrow_mut().as_mut() {
            app.engine.set_debug_geometry(on);
        }
    }

    /// 设揭示速率上限(glyph/秒,Plan 8C / 0019);≤0 = 不限速(跟内容到达,默认)。
    /// 与 token 解耦的揭示时钟;调慢即限速。web 调试面板"reveal 速度"调。
    pub fn set_reveal_cps(&self, cps: f32) {
        if let Some(app) = self.state.borrow_mut().as_mut() {
            app.engine.set_reveal_cps(cps);
        }
    }

    /// 设揭示放慢因子(`[0.01,1.0]`,越小越慢;0019 北极星"刻意放慢")。web"放慢"档调。
    pub fn set_reveal_slow(&self, slow: f32) {
        if let Some(app) = self.state.borrow_mut().as_mut() {
            app.engine.set_reveal_slow(slow);
        }
    }

    /// 设表格揭示风格(Plan 8B / 0019 §2:0=原始逐字 / 1=行框 / 2=整表骨架先行)。web 下拉调。
    pub fn set_table_reveal_style(&self, style: u32) {
        if let Some(app) = self.state.borrow_mut().as_mut() {
            app.engine.set_table_reveal_style(style);
        }
    }

    /// 设数学每 em 的 world px(Plan 12)= 正文字号(含 DPR)。行内数学贴正文、显示数学 ×1.3(H3)。
    /// web 启动按 `FONT_SIZE` 注入,使公式与正文同尺度。
    pub fn set_math_em(&self, px: f32) {
        if let Some(app) = self.state.borrow_mut().as_mut() {
            app.engine.set_math_em(px);
        }
    }

    /// 重放揭示动画(调试):内容已全部上屏(冻结)时,改风格/速度本身没有待揭的字 → 看不到效果。
    /// 调此清空 spawn,使调度器按**当前**风格/速度从头再揭示一遍。web 下拉改完即调,所见即所设。
    pub fn restart_reveal(&self) {
        if let Some(app) = self.state.borrow_mut().as_mut() {
            app.engine.restart_reveal();
        }
    }

    /// 调试播放器:按显式 `dt_ms` 推进一帧(出图),不走墙钟。配 `set_paused(true)` 由 JS 掌钟,
    /// 实现播放/调速(dt×倍率)/单步(传一帧 dt)。
    pub fn tick(&self, dt_ms: f32) {
        if let Some(app) = self.state.borrow_mut().as_mut() {
            app.engine.frame(f64::from(dt_ms.max(0.0)));
        }
    }

    /// 调试播放器:把**揭示**动画跳到时间轴 `target_ms`(拖拽 scrubber 调)。清空 spawn 后确定性
    /// 重跑揭示到该时刻并出一帧(向后跳也对)。内容须已加载;揭示基速用当前 `reveal_cps`/`slow`。
    pub fn seek_reveal(&self, target_ms: f32) {
        if let Some(app) = self.state.borrow_mut().as_mut() {
            app.engine.seek_reveal(f64::from(target_ms.max(0.0)));
        }
    }

    /// 字体切换后刷新(JS 侧已 `setFontPreset`):换 atlas 代让字形用新字体重栅 + 全量重排
    /// (字宽变了,块冻结的脏判据不会自动触发)。Plan 4C 调试器换字体用。
    pub fn refresh_fonts(&self) {
        if let Some(app) = self.state.borrow_mut().as_mut() {
            app.engine.sink_mut().bump_font_gen();
            app.engine.mark_layout_dirty();
        }
    }

    /// 初始化 GPU、连流、起帧循环(异步)。
    pub fn start(&self) {
        let canvas = self.canvas.clone();
        let layout_fn = self.layout_fn.clone();
        let measure_fn = self.measure_fn.clone();
        let rasterize_fn = self.rasterize_fn.clone();
        let server_url = self.server_url.clone();
        let session_id = self.session_id.clone();
        let replay = self.replay.clone();
        let state = self.state.clone();
        let raf = self.raf.clone();
        let stats_cell = self.stats.clone();
        let paused_flag = self.paused.clone();
        let step_flag = self.step.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = init_and_run(
                canvas,
                layout_fn,
                measure_fn,
                rasterize_fn,
                server_url,
                session_id,
                replay,
                state,
                raf,
                stats_cell,
                paused_flag,
                step_flag,
            )
            .await
            {
                tracing::error!(target: "M13", "启动失败: {e}");
            }
        });
    }
}

#[allow(clippy::too_many_arguments)] // reason: 内部启动管线;装一堆共享句柄,拆 struct 反而更绕
async fn init_and_run(
    canvas: web_sys::HtmlCanvasElement,
    layout_fn: js_sys::Function,
    measure_fn: Option<js_sys::Function>,
    rasterize_fn: js_sys::Function,
    server_url: Option<String>,
    session_id: Option<String>,
    replay: Option<Vec<Record>>,
    state: SharedState,
    raf: RafHandle,
    stats_cell: StatsCell,
    paused: Flag,
    step: Flag,
) -> Result<(), String> {
    let width = canvas.width().max(1);
    let height = canvas.height().max(1);

    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::BROWSER_WEBGPU | wgpu::Backends::GL,
        ..Default::default()
    });
    let surface = instance
        .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
        .map_err(|e| format!("create_surface 失败: {e}"))?;
    let backend = WebGpuBackend::new(&instance, surface, width, height)
        .await
        .map_err(|e| e.to_string())?;

    // Phase F:先拉快照(SSE 连接之前取的时间点),再连 SSE。这样 SSE 缓冲的事件都在
    // 快照点之后 → catch-up 与 live 不重叠(避免双重追加)。0003 §4 的 buffer-first 严格
    // 时序留到 Phase J。
    let snapshot_raw = match (&server_url, &session_id) {
        (Some(url), Some(sid)) => fetch_snapshot(url, sid).await,
        _ => None,
    };

    let conn: Box<dyn Connection> = match (replay, &server_url) {
        // 重放优先(Plan 5D):预录事件喂 Player,不连服务端。
        (Some(records), _) => Box::new(Player::new(records, 16.0)),
        (None, Some(url)) => Box::new(SseConnection::connect(url)?),
        (None, None) => Box::new(synthetic()),
    };
    let layout = LayoutBridge::new(layout_fn, measure_fn);
    let sink = GpuSink {
        backend,
        rasterize_fn,
        profile: EffectProfile::Full,
        font_gen: 0,
        glyph_mode: GlyphMode::Auto,
        src_counts: [0; 4],
        msdf_font: None,
        scene: Scene::new(120.0),            // 过渡时长(policy 默认,0016 §8)
        panel_scene: PanelScene::new(120.0), // 同 dur → 框字补间同步(0018 §5 / 6D)
        last_embeds: Vec::new(),
    };
    // 留给周期性 resync(Phase J)用:server+session。
    let resync_server = server_url.clone();
    let resync_session = session_id.clone();

    let mut engine = Engine::new(conn, layout, sink, 200.0, width as f32);
    engine.set_viewport_height(height as f32);
    if let Some(sid) = session_id {
        engine.set_target_session(Some(sid));
    }
    if let Some(raw) = snapshot_raw {
        engine.prime_from_snapshot(&raw);
    }
    *state.borrow_mut() = Some(AppState { engine });

    // requestAnimationFrame 帧循环:dt 由 performance.now 差分得出(R8)。
    let clock = WebClock::new().ok_or("无 performance.now")?;
    let last = Rc::new(RefCell::new(clock.now_ms()));
    let since_resync = Rc::new(RefCell::new(0.0f64));
    let inner = state.clone();
    let next = raf.clone();
    let resync_state = state.clone();
    // 可观测(`?debug`):节流 1s 打一行帧统计(帧耗时 / 发射 glyph / 块 / atlas / 丢帧 / fps)。
    let debug = web_sys::window()
        .and_then(|w| w.location().search().ok())
        .is_some_and(|s| s.contains("debug"));
    let mut perf_frames = 0u32;
    let mut perf_acc_ms = 0.0f64;
    let mut perf_max_ms = 0.0f64;
    let mut perf_dropped = 0u32;
    let mut perf_window = 0.0f64;
    *raf.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let now = clock.now_ms();
        let dt = (now - *last.borrow()).max(0.0);
        *last.borrow_mut() = now;
        // 暂停/单步(Plan 4C2):暂停时冻结(surface 保留上帧),step 只推一帧。
        let do_step = std::mem::replace(&mut *step.borrow_mut(), false);
        let t0 = clock.now_ms();
        if !*paused.borrow() || do_step {
            if let Some(app) = inner.borrow_mut().as_mut() {
                app.engine.frame(dt);
            }
        }
        // 帧统计(Plan 4C1):每秒汇总写快照(`stats()` 读给面板),debug 时同时打日志。
        let frame_ms = clock.now_ms() - t0;
        perf_frames += 1;
        perf_acc_ms += frame_ms;
        perf_max_ms = perf_max_ms.max(frame_ms);
        if dt > 24.0 {
            perf_dropped += 1; // > ~1.5 × 16.67ms 视为丢帧
        }
        perf_window += dt;
        if perf_window >= 1000.0 {
            let fps = f64::from(perf_frames) * 1000.0 / perf_window;
            let avg = perf_acc_ms / f64::from(perf_frames.max(1));
            if let Some(app) = inner.borrow().as_ref() {
                let st = app.engine.frame_stats();
                let (used, cap, evict) = app.engine.sink().atlas_stats();
                *stats_cell.borrow_mut() = StatsSnapshot {
                    fps,
                    frame_ms_avg: avg,
                    frame_ms_max: perf_max_ms,
                    dropped: perf_dropped,
                    glyphs_visible: st.frame_glyphs,
                    glyphs_total: st.total_glyphs,
                    blocks_visible: st.visible_blocks,
                    blocks_total: st.total_blocks,
                    shaderbox_active: st.shaderbox_active,
                    shaderbox_pixels: st.shaderbox_pixels,
                    atlas_used: used,
                    atlas_cap: cap,
                    atlas_evict: evict,
                    cam_zoom: app.engine.camera().zoom(),
                    src_counts: app.engine.sink().source_counts(),
                };
                if debug {
                    tracing::info!(target: "perf",
                        "fps={fps:.0} frame_ms(avg={avg:.1} max={:.1}) dropped={perf_dropped} glyphs={}/{} blocks={}/{} sbox={}({}px) atlas={used}/{cap} evict={evict}",
                        perf_max_ms, st.frame_glyphs, st.total_glyphs, st.visible_blocks, st.total_blocks,
                        st.shaderbox_active, st.shaderbox_pixels);
                }
            }
            perf_frames = 0;
            perf_acc_ms = 0.0;
            perf_max_ms = 0.0;
            perf_dropped = 0;
            perf_window = 0.0;
        }
        // 周期性对账(Phase J):每 RESYNC_MS 拉一次快照补错过的历史(配合 EventSource 自动
        // 重连,覆盖弱网/僵尸连接下的丢失,不动 live 块)。
        *since_resync.borrow_mut() += dt;
        if *since_resync.borrow() >= RESYNC_MS {
            *since_resync.borrow_mut() = 0.0;
            if let (Some(server), Some(sid)) = (resync_server.clone(), resync_session.clone()) {
                let st = resync_state.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    if let Some(raw) = fetch_snapshot(&server, &sid).await {
                        if let Some(app) = st.borrow_mut().as_mut() {
                            app.engine.resync_from_snapshot(&raw);
                        }
                    }
                });
            }
        }
        if let Some(cb) = next.borrow().as_ref() {
            request_animation_frame(cb);
        }
    }) as Box<dyn FnMut()>));
    if let Some(cb) = raf.borrow().as_ref() {
        request_animation_frame(cb);
    }

    // 画布输入(滚轮/触摸板/拖拽)移到 web 层(TS `input.ts`),经 `ChatCanvas.pan_by/zoom_at` 调入,
    // 便于不重编 wasm 调手感(Plan 6)。此处只保留窗口级 resize。

    // 窗口尺寸变化:重设后备缓冲(设备像素)→ 重配 surface + 更新排版宽度。
    let canvas_r = canvas.clone();
    let state_r = state.clone();
    let on_resize = Closure::wrap(Box::new(move || {
        let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
        let w = (f64::from(canvas_r.client_width().max(1)) * dpr).round() as u32;
        let h = (f64::from(canvas_r.client_height().max(1)) * dpr).round() as u32;
        canvas_r.set_width(w);
        canvas_r.set_height(h);
        if let Some(app) = state_r.borrow_mut().as_mut() {
            app.engine.sink_mut().resize(w, h);
            app.engine.set_max_width(w as f32);
            app.engine.set_viewport_height(h as f32);
        }
    }) as Box<dyn FnMut()>);
    if let Some(win) = web_sys::window() {
        win.add_event_listener_with_callback("resize", on_resize.as_ref().unchecked_ref())
            .map_err(|e| format!("注册 resize 监听失败: {e:?}"))?;
    }
    // 让监听器自持(独立于 JS 端 ChatCanvas 句柄的生命周期),避免句柄被 GC 后回调悬空。
    on_resize.forget();
    Ok(())
}

/// 合成事件源:逐块吐预设文本 delta,演示匀速淡入(Phase C,无需服务端)。
fn synthetic() -> Player {
    // 含一张同源测试图片(Plan 14:`web/public/test-image.svg`,vite 同源服务,无 CORS)→ 演示
    // markdown 图片嵌入解码上屏。动图测试改 `/test-animated.svg`(走 DOM overlay)。
    const TEXT: &str =
        "你好!我是 opencode 渲染引擎 🚀 正在逐字淡入 streaming text.\n\n![测试图片](/test-image.svg)";
    let mut records = Vec::new();
    let mut t = 0.0;
    let chars: Vec<char> = TEXT.chars().collect();
    for chunk in chars.chunks(3) {
        let delta: String = chunk.iter().collect();
        let raw = format!(
            r#"{{"type":"message.part.delta","properties":{{"sessionID":"demo","messageID":"m","partID":"p1","field":"text","delta":{delta:?}}}}}"#
        );
        records.push(Record { t, raw });
        t += 60.0;
    }
    Player::new(records, 16.0)
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    if let Some(win) = web_sys::window() {
        let _ = win.request_animation_frame(f.as_ref().unchecked_ref());
    }
}

fn get_fn(config: &JsValue, key: &str) -> Result<js_sys::Function, JsValue> {
    let v = js_sys::Reflect::get(config, &JsValue::from_str(key))?;
    v.dyn_into::<js_sys::Function>()
        .map_err(|_| JsValue::from_str(&format!("config.{key} 必须是函数")))
}

fn get_str(config: &JsValue, key: &str) -> Option<String> {
    js_sys::Reflect::get(config, &JsValue::from_str(key))
        .ok()?
        .as_string()
}

/// 读对象字段为 f32(数值);缺省/非数值 → None(set_table_style 用)。
fn get_f32(obj: &JsValue, key: &str) -> Option<f32> {
    js_sys::Reflect::get(obj, &JsValue::from_str(key))
        .ok()?
        .as_f64()
        .map(|n| n as f32)
}

/// 读对象字段为定长 f32 数组(颜色分量);长度不足/非数组 → None(set_table_style 用)。
fn get_f32_arr(obj: &JsValue, key: &str, n: usize) -> Option<Vec<f32>> {
    let v = js_sys::Reflect::get(obj, &JsValue::from_str(key)).ok()?;
    let arr = v.dyn_into::<js_sys::Array>().ok()?;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(arr.get(i as u32).as_f64()? as f32);
    }
    Some(out)
}

/// 解析 `config.replay`(Plan 5D):`[{ t: number, raw: string }]` → `Vec<Record>`。缺省/空 → None。
fn get_replay(config: &JsValue, key: &str) -> Option<Vec<Record>> {
    let v = js_sys::Reflect::get(config, &JsValue::from_str(key)).ok()?;
    if v.is_undefined() || v.is_null() {
        return None;
    }
    let arr = js_sys::Array::from(&v);
    let mut records = Vec::with_capacity(arr.length() as usize);
    for item in arr.iter() {
        let t = js_sys::Reflect::get(&item, &JsValue::from_str("t"))
            .ok()
            .and_then(|x| x.as_f64())?;
        let raw = js_sys::Reflect::get(&item, &JsValue::from_str("raw"))
            .ok()
            .and_then(|x| x.as_string())?;
        records.push(Record { t, raw });
    }
    (!records.is_empty()).then_some(records)
}
