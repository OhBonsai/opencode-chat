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
mod observe;
mod transport;

use std::cell::RefCell;
use std::rc::Rc;

use infinite_chat_core::{Clock, Connection, Engine, FrameData, Player, Record, RenderSink};
use infinite_chat_render::{EffectProfile, RenderBackend, WebGpuBackend};
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
#[allow(dead_code)] // reason: Phase 2 MSDF 源接入后使用
const KIND_MSDF: u32 = 2;

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

    /// 解析某字形的源 kind(0015 §2.2)。Phase 1:Bitmap 模式 → 位图;其余 → TinySDF
    /// (MSDF 源 Phase 2 接入,届时 Auto/ForceMSDF 命中烘集走 MSDF)。
    fn resolve_kind(&self, _cluster: &str) -> u32 {
        match self.glyph_mode {
            GlyphMode::Bitmap => KIND_BITMAP,
            _ => KIND_TINYSDF,
        }
    }
}

impl RenderSink for GpuSink {
    fn submit(&mut self, frame: &FrameData) {
        self.backend.atlas_begin_frame();
        self.src_counts = [0; 4];
        let mut instances = Vec::with_capacity(frame.glyphs.len());
        for g in &frame.glyphs {
            // 源解析(0015):决定该字走位图/TinySDF/MSDF。
            let kind = self.resolve_kind(&g.cluster);
            self.src_counts[kind as usize] += 1;
            // atlas 按 (font_gen, kind, style, cluster) 分桶:粗/斜/code 与不同源是不同 tile;
            // font_gen/kind 变化让 key 失配触发重栅(render 与此处同 key)。
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
            instances.push(infinite_chat_render::GpuInstance {
                pos: g.pos,
                size: g.size,
                uv: a.slot.uv(),
                spawn_time: g.spawn_time,
                style: g.style,
                layer: a.slot.page,
                kind,
            });
        }
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
        if let Err(e) = self.backend.draw(
            &instances,
            &rects,
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
    rasterize_fn: js_sys::Function,
    server_url: Option<String>,
    session_id: Option<String>,
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
        let rasterize_fn = get_fn(&config, "rasterize")?;
        Ok(Self {
            canvas,
            layout_fn,
            rasterize_fn,
            server_url: get_str(&config, "serverUrl"),
            session_id: get_str(&config, "sessionId"),
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
        let rasterize_fn = self.rasterize_fn.clone();
        let server_url = self.server_url.clone();
        let session_id = self.session_id.clone();
        let state = self.state.clone();
        let raf = self.raf.clone();
        let stats_cell = self.stats.clone();
        let paused_flag = self.paused.clone();
        let step_flag = self.step.clone();
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = init_and_run(
                canvas,
                layout_fn,
                rasterize_fn,
                server_url,
                session_id,
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
    rasterize_fn: js_sys::Function,
    server_url: Option<String>,
    session_id: Option<String>,
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

    let conn: Box<dyn Connection> = match &server_url {
        Some(url) => Box::new(SseConnection::connect(url)?),
        None => Box::new(synthetic()),
    };
    let layout = LayoutBridge::new(layout_fn);
    let sink = GpuSink {
        backend,
        rasterize_fn,
        profile: EffectProfile::Full,
        font_gen: 0,
        glyph_mode: GlyphMode::Auto,
        src_counts: [0; 4],
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
                    atlas_used: used,
                    atlas_cap: cap,
                    atlas_evict: evict,
                    cam_zoom: app.engine.camera().zoom(),
                    src_counts: app.engine.sink().source_counts(),
                };
                if debug {
                    tracing::info!(target: "perf",
                        "fps={fps:.0} frame_ms(avg={avg:.1} max={:.1}) dropped={perf_dropped} glyphs={}/{} blocks={}/{} atlas={used}/{cap} evict={evict}",
                        perf_max_ms, st.frame_glyphs, st.total_glyphs, st.visible_blocks, st.total_blocks);
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

    // 滚轮 → 滚动(Phase G)。挂在 canvas 上(元素级 wheel 默认非 passive),preventDefault
    // 能阻止页面滚动。deltaY 为设备无关像素,直接喂 scroll_by(world unit = 设备 px,见缩放)。
    let state_w = state.clone();
    let on_wheel = Closure::wrap(Box::new(move |e: web_sys::WheelEvent| {
        e.prevent_default();
        let dpr = web_sys::window().map_or(1.0, |w| w.device_pixel_ratio());
        if let Some(app) = state_w.borrow_mut().as_mut() {
            if e.ctrl_key() {
                // ctrl+滚轮 = 围绕指针缩放(Plan 3 L)。
                let factor = if e.delta_y() < 0.0 { 1.1 } else { 1.0 / 1.1 };
                let sx = (f64::from(e.offset_x()) * dpr) as f32;
                let sy = (f64::from(e.offset_y()) * dpr) as f32;
                app.engine.zoom_by(factor, sx, sy);
            } else {
                app.engine.scroll_by((e.delta_y() * dpr) as f32);
            }
        }
    }) as Box<dyn FnMut(web_sys::WheelEvent)>);
    let _ = canvas.add_event_listener_with_callback("wheel", on_wheel.as_ref().unchecked_ref());
    on_wheel.forget();

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
    const TEXT: &str = "你好!我是 opencode 渲染引擎 🚀 正在逐字淡入 streaming text.";
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
