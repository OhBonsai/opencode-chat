//! opencode-chat-wasm(M12 api / M1 transport / 平台胶水)。
//!
//! 薄 `#[wasm_bindgen]` 层(CR5):业务逻辑全在 core,这里只做平台接缝——从 canvas 建
//! wgpu surface、连 SSE、调 JS 排版/光栅化、跑 requestAnimationFrame 帧循环。
//!
//! 整个 crate 仅 `wasm32` 目标有内容;native `cargo build --workspace` 把它当空 lib
//! 跳过(平台代码无法在 native 链接)。真实编译验证:
//! `cargo build -p opencode-chat-wasm --target wasm32-unknown-unknown`。
#![cfg(target_arch = "wasm32")]

mod clock;
mod glyph_bridge;
mod layout_bridge;
mod observe;
mod transport;

use std::cell::RefCell;
use std::rc::Rc;

use opencode_chat_core::{Clock, Connection, Engine, FrameData, Player, Record, RenderSink};
use opencode_chat_render::{EffectProfile, RenderBackend, WebGpuBackend};
use wasm_bindgen::prelude::*;

use crate::clock::WebClock;
use crate::layout_bridge::PretextLayout;
use crate::transport::{fetch_snapshot, SseConnection};

type AppEngine = Engine<Box<dyn Connection>, PretextLayout, GpuSink>;
type SharedState = Rc<RefCell<Option<AppState>>>;
type RafHandle = Rc<RefCell<Option<Closure<dyn FnMut()>>>>;

/// 周期性对账间隔(ms,Phase J)。
const RESYNC_MS: f64 = 20_000.0;

/// 渲染汇:把 core 的语义字形按需光栅化进图集,再交后端绘制。
struct GpuSink {
    backend: WebGpuBackend,
    rasterize_fn: js_sys::Function,
    profile: EffectProfile,
}

impl GpuSink {
    fn resize(&mut self, width: u32, height: u32) {
        self.backend.resize(width, height);
    }
}

impl RenderSink for GpuSink {
    fn submit(&mut self, frame: &FrameData) {
        self.backend.atlas_begin_frame();
        let mut instances = Vec::with_capacity(frame.glyphs.len());
        for g in &frame.glyphs {
            // atlas 按 (style, cluster) 分桶:粗/斜/code 是不同 SDF tile(render 与此处同 key)。
            let key = opencode_chat_render::glyph_key(g.style, &g.cluster);
            self.backend.atlas_pin(&key);
            let a = self.backend.atlas_alloc(&key);
            if a.is_new {
                if let Some(sdf) =
                    glyph_bridge::rasterize_sdf(&self.rasterize_fn, &g.cluster, g.style)
                {
                    self.backend.atlas_upload(a.slot, &sdf);
                }
            }
            instances.push(opencode_chat_render::GpuInstance {
                pos: g.pos,
                size: g.size,
                uv: a.slot.uv(),
                spawn_time: g.spawn_time,
                style: g.style,
                layer: a.slot.page,
            });
        }
        if let Err(e) = self.backend.draw(
            &instances,
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
        })
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
        wasm_bindgen_futures::spawn_local(async move {
            if let Err(e) = init_and_run(
                canvas,
                layout_fn,
                rasterize_fn,
                server_url,
                session_id,
                state,
                raf,
            )
            .await
            {
                tracing::error!(target: "M13", "启动失败: {e}");
            }
        });
    }
}

async fn init_and_run(
    canvas: web_sys::HtmlCanvasElement,
    layout_fn: js_sys::Function,
    rasterize_fn: js_sys::Function,
    server_url: Option<String>,
    session_id: Option<String>,
    state: SharedState,
    raf: RafHandle,
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
    let layout = PretextLayout::new(layout_fn);
    let sink = GpuSink {
        backend,
        rasterize_fn,
        profile: EffectProfile::Full,
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
    *raf.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        let now = clock.now_ms();
        let dt = (now - *last.borrow()).max(0.0);
        *last.borrow_mut() = now;
        if let Some(app) = inner.borrow_mut().as_mut() {
            app.engine.frame(dt);
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
