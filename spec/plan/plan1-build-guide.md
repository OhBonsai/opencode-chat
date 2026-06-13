# Plan 1 构建指南(逐文件可执行版)

- 配套:[plan1-minimal-prototype.md](./plan1-minimal-prototype.md)(目标/范围/验收)、[../architecture.md](../architecture.md)、[../dev-practices.md](../dev-practices.md)
- 用途:在 Claude Code 里按本指南逐 Phase 实现。每 Phase 给"建哪些文件 + 各文件要点 + 验收命令 + DoD"。

---

## 0. 在 Claude Code 里怎么用

- 起手 `/dev-start`:域选对应 Phase 的模块(如 Phase C 主要 M3/M5/M13)。
- 写 Rust 前 `/rust-write` 加载 AR/CR/R 铁律;写 wgpu 前 `/render-write`;写 `web/` 前 `/bridge-write`;写测试前 `/test-write`。
- 每 Phase 末 `/dev-wrap <task_id>` 过卡口提交。一个 Phase ≈ 一个 PR。
- **铁律红线**(全程):core 零 `wasm-bindgen/web-sys/wgpu`(CR1);core 零 `now()`/裸 `rand`,走 `Clock`/seed(R8/R9);跨界一帧一次批调用(AR10);delta 必配 updated 对账(AR4)。

---

## 1. 前置工具链

```bash
rustup default stable
rustup target add wasm32-unknown-unknown
cargo install wasm-pack            # 交付 npm 包
cargo install cargo-deny           # 供应链审查(可后置)
# web/ 用 node ≥ 20 + npm/pnpm
```

`rust-toolchain.toml`(仓库根):
```toml
[toolchain]
channel = "stable"
targets = ["wasm32-unknown-unknown"]
components = ["clippy", "rustfmt"]
```

---

## 2. 最终文件树(Plan 1 结束态)

```
opencode-chat/
├── Cargo.toml                      # workspace
├── rust-toolchain.toml
├── deny.toml
├── rustfmt.toml
├── crates/
│   ├── core/
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/
│   │       ├── lib.rs              # //! mod doc + re-export
│   │       ├── seam.rs             # Connection/LayoutEngine/Clock/RenderSink trait
│   │       ├── protocol.rs         # Event/Part(仅 text 子集)+ serde
│   │       ├── store.rs            # 三表最小 + delta 追加 + updated 对账
│   │       ├── smoother.rs         # 逐 grapheme 整流 + spawn_time
│   │       ├── content.rs          # 纯文本直通(StyledSpan = 一段 text)
│   │       ├── app.rs              # Engine<C,L,R> 每帧编排
│   │       ├── frame.rs            # GlyphInstance / FrameData(给 RenderSink)
│   │       └── record.rs           # Recorder / Player(Connection 实现)
│   ├── render/
│   │   ├── Cargo.toml
│   │   ├── README.md
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── backend.rs          # RenderBackend trait + WebGpuBackend
│   │       ├── atlas.rs            # glyph atlas(纹理 + UV 表)
│   │       ├── scene.rs            # GlyphInstance buffer(无裁剪)
│   │       ├── effects.rs          # 一个淡入(profile 占位)
│   │       └── shaders/glyph.wgsl  # 实例化 + 淡入
│   └── wasm/
│       ├── Cargo.toml              # crate-type = ["cdylib"]
│       └── src/
│           ├── lib.rs              # #[wasm_bindgen] ChatCanvas(api)
│           ├── transport.rs        # Connection impl:EventSource(gloo-net)
│           ├── layout_bridge.rs    # LayoutEngine impl:调 pretext(JS)
│           ├── glyph_bridge.rs     # 调 JS 光栅化
│           ├── clock.rs            # Clock impl:performance.now()
│           └── observe.rs          # tracing-wasm + panic hook
├── web/
│   ├── package.json
│   ├── vite.config.ts
│   ├── tsconfig.json
│   ├── index.html
│   └── src/
│       ├── main.ts                 # 挂 canvas + init wasm + 转发事件
│       ├── pretext-bridge.ts       # StyledSpan→pretext→平铺数组
│       └── glyph-raster.ts         # OffscreenCanvas 光栅化 grapheme→纹理
└── tests/                          # 在 core crate 内:tests/replay.rs
```

> 其余 architecture.md 模块(fsm/content 全量/input/embed…)本期**建空文件占位**,文件顶 `//! TODO: plan N`。

---

## 3. 配置文件(可直接抄)

### `Cargo.toml`(workspace 根)
```toml
[workspace]
members = ["crates/core", "crates/render", "crates/wasm"]
resolver = "2"

[workspace.package]
edition = "2021"
license = "MIT OR Apache-2.0"

[workspace.dependencies]
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tracing = "0.1"
# 平台相关(仅 wasm/render crate 用)
wasm-bindgen = "=0.2.100"          # ★锁定;wasm-pack/CLI 必须同版本
wasm-bindgen-futures = "0.4"
js-sys = "0.3"
web-sys = "0.3"
gloo-net = { version = "0.6", features = ["eventsource"] }
tracing-wasm = "0.2"
console_error_panic_hook = "0.1"
wgpu = "25"                        # ★用前确认最新稳定版,WebGPU+WebGL2 后端
bytemuck = { version = "1", features = ["derive"] }

[workspace.lints.rust]
unsafe_code = "forbid"             # render/wasm 局部确需可 `#![allow(unsafe_code)]` 并注释
rust_2018_idioms = "warn"
unreachable_pub = "warn"

[workspace.lints.clippy]
all = { level = "warn", priority = -1 }
pedantic = { level = "warn", priority = -1 }
module_name_repetitions = "allow"
must_use_candidate = "allow"
unwrap_used = "warn"
panic = "warn"

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
```

### `crates/core/Cargo.toml`(★零平台依赖)
```toml
[package]
name = "opencode-chat-core"
version = "0.1.0"
edition.workspace = true

[dependencies]
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tracing.workspace = true
unicode-segmentation = "1"         # 仅 native stub LayoutEngine 用

[dev-dependencies]
proptest = "1"

[lints]
workspace = true
```

### `crates/render/Cargo.toml`
```toml
[package]
name = "opencode-chat-render"
version = "0.1.0"
edition.workspace = true

[dependencies]
opencode-chat-core = { path = "../core" }
wgpu.workspace = true
bytemuck.workspace = true
tracing.workspace = true

[lints]
workspace = true
```

### `crates/wasm/Cargo.toml`
```toml
[package]
name = "opencode-chat-wasm"
version = "0.1.0"
edition.workspace = true

[lib]
crate-type = ["cdylib"]

[dependencies]
opencode-chat-core = { path = "../core" }
opencode-chat-render = { path = "../render" }
wasm-bindgen.workspace = true
wasm-bindgen-futures.workspace = true
js-sys.workspace = true
web-sys = { workspace = true, features = ["Window","Document","HtmlCanvasElement","Performance","EventSource","MessageEvent"] }
gloo-net.workspace = true
tracing.workspace = true
tracing-wasm.workspace = true
console_error_panic_hook.workspace = true

[lints]
workspace = true
```

### `rustfmt.toml`
```toml
edition = "2021"
max_width = 100
imports_granularity = "Module"
group_imports = "StdExternalCrate"
```

### `web/package.json`
```json
{
  "name": "opencode-chat-harness",
  "private": true,
  "type": "module",
  "scripts": {
    "build:wasm": "wasm-pack build ../crates/wasm --target web --out-dir ../../web/pkg",
    "dev": "npm run build:wasm && vite",
    "build": "npm run build:wasm && vite build"
  },
  "devDependencies": { "vite": "^5", "vite-plugin-wasm": "^3", "typescript": "^5" },
  "dependencies": { "@chenglou/pretext": "link:../pretext" }
}
```
> pretext 用本地路径(仓库内 `pretext/`)。`vite.config.ts` 加 `wasm()` 插件 + `optimizeDeps.exclude` 掉 pkg。

---

## 4. 核心契约(core 关键签名,先冻结再填充)

### `seam.rs` — 平台缝(让 core native 可测)
```rust
//! 平台缝:core 不直接依赖 EventSource/pretext/wgpu/时钟,由组装方注入。
pub trait Connection {
    /// 取自上次以来到达的原始事件(非阻塞)。Player 也实现它(重放)。
    fn poll(&mut self) -> Vec<RawEvent>;
}
pub trait LayoutEngine {
    /// 把样式 run 排版成带位置的 glyph;Plan1 纯文本。
    fn layout(&mut self, spans: &[StyledSpan], max_width: f32) -> LayoutResult;
}
pub trait Clock { fn now_ms(&self) -> f64; }          // R8:core 不碰 Instant::now
pub trait RenderSink { fn submit(&mut self, frame: &FrameData); }

pub struct RawEvent { pub raw: String }               // SSE data 原文,不在 JS 侧解析(BR1)
```

### `protocol.rs` — opencode 事件(Plan1 仅 text 子集)
```rust
#[derive(serde::Deserialize)]
pub struct Envelope { pub id: String, #[serde(rename="type")] pub kind: String, pub properties: serde_json::Value }

#[derive(serde::Deserialize)]
#[serde(tag = "type", content = "properties")]
pub enum Event {
    #[serde(rename="message.part.delta")]
    PartDelta { #[serde(rename="sessionID")] session_id: String,
                #[serde(rename="messageID")] message_id: String,
                #[serde(rename="partID")]    part_id: String,
                field: String, delta: String },
    #[serde(rename="message.part.updated")]
    PartUpdated { part: Part, time: f64 },
    #[serde(rename="message.updated")]   MessageUpdated { /* 最小 */ },
    #[serde(rename="session.status")]    SessionStatus { /* {type: idle|busy|retry} */ },
    #[serde(rename="server.connected")]  Connected,
    #[serde(rename="server.heartbeat")]  Heartbeat,
    #[serde(other)]                      Ignored,      // AR12
}

#[derive(serde::Deserialize)]
#[serde(tag = "type")]
pub enum Part {
    #[serde(rename="text")] Text { id:String, #[serde(rename="messageID")] message_id:String, text:String },
    // Plan1 其余 part 类型先 Ignored(用 #[serde(other)] 或外层跳过)
}
```

### `store.rs` — 对账(AR4)
```rust
pub struct Store { /* message[sid] / part[mid] / part_accum[pid] */ }
impl Store {
    pub fn apply_delta(&mut self, part_id:&str, field:&str, delta:&str) { /* 追加 text + part_accum */ }
    pub fn apply_part_updated(&mut self, part: Part) { /* 覆盖 + 清 part_accum;不一致以此为准 */ }
}
```

### `smoother.rs`(AR7 grapheme)
```rust
pub struct Smoother { /* 按 part_id 的 reveal 队列 + accumulator */ }
impl Smoother {
    pub fn push(&mut self, part_id:&str, graphemes:&[&str]) { /* 入队 */ }
    pub fn update(&mut self, dt_ms:f64, now_ms:f64) -> Vec<Revealed> { /* 匀速吐,打 spawn_time */ }
}
```

### `app.rs` — 每帧编排
```rust
pub struct Engine<C:Connection, L:LayoutEngine, R:RenderSink> { /* store, smoother, clock 等 */ }
impl<C,L,R> Engine<C,L,R> {
    pub fn frame(&mut self, dt_ms:f64) {
        // 1 conn.poll → Envelope→Event → store.apply (+ updated 对账)
        // 2 新增文本 → 取 grapheme(经 LayoutEngine/分段)→ smoother.push
        // 3 smoother.update(dt) → 本帧 reveal(spawn_time)
        // 4 content 直通 → layout.layout → 位置
        // 5 组 FrameData(GlyphInstance[]) → render.submit
    }
}
```

`record.rs`:`Recorder` 包住 `Connection` 记 `(t, raw)`;`Player` 实现 `Connection`,按 t 回放。

---

## 5. 分 Phase 实施

> 每 Phase:**建文件 → 实现 → 验收命令 → DoD 勾选**。Phase 间保持可跑。

### Phase A — 脚手架 + 空画布

**建**:workspace 全部配置(§3)、`crates/{core,render,wasm}` 骨架、`web/` harness、各空占位文件。
**实现要点**:
- `render`:wgpu init(`request_adapter`→`device/queue`→`surface`),每帧清色;Plan1 直接 WebGPU。
- `wasm/observe.rs`:`console_error_panic_hook::set_once()` + `tracing_wasm::set_as_global_default()`。
- `wasm/lib.rs`:`#[wasm_bindgen] pub struct ChatCanvas` + `new(canvas)` + 起 `requestAnimationFrame` 帧循环。
- `web/main.ts`:拿 `<canvas>` → `init()` wasm → `new ChatCanvas(canvas)`。

**验收**:
```bash
cargo build --workspace
cd web && npm i && npm run dev      # 打开页面
```
**DoD**:页面画布显示纯色;console 有帧日志;故意 `panic!` 能看到 Rust backtrace。

### Phase B — 渲染静态字符串

**建/改**:`render/atlas.rs`、`render/scene.rs`、`render/shaders/glyph.wgsl`、`wasm/glyph_bridge.rs`、`wasm/layout_bridge.rs`、`web/glyph-raster.ts`、`web/pretext-bridge.ts`。
**实现要点**:
- `pretext-bridge.ts`:导出 `layout(text, font, maxWidth) → Float32Array`(每 grapheme:`[x,y,w,glyphId]`)。Plan1 可先单段。
- `glyph-raster.ts`:`rasterize(grapheme, font) → {bitmap, w, h}`(OffscreenCanvas 2D `fillText`)。
- `atlas.rs`:接收位图上传到一张纹理,返回 UV;`GlyphInstance { pos:[f32;2], uv:[f32;4], glyph_id }`(`bytemuck::Pod`)。
- `glyph.wgsl`:实例化,采样 atlas,输出。
- 硬编码渲染 `"Hello 你好 🚀"`。

**验收**:页面正确显示该串,位置/字体正确,中英 emoji 都对。
**DoD**:静态串渲染正确(浏览器系统字体,零字体打包,BR5)。

### Phase C — 合成流 + 平滑器 + 淡入(体验核心)

**建/改**:`core/store.rs`、`core/smoother.rs`、`core/content.rs`、`core/app.rs`、`core/frame.rs`、`render/effects.rs`、`glyph.wgsl`(加淡入)、`core/record.rs`(Player 雏形)。
**实现要点**:
- 合成 `Player`:吐预设文本 delta(可控速率),实现 `Connection`。
- `Engine::frame` 串起 store→smoother→content→layout→render。
- `GlyphInstance` 加 `spawn_time`;`glyph.wgsl` 加 `alpha = smoothstep(0, fade, time - spawn_time)`,uniform 传 `time`。
- 平滑器逐 grapheme 匀速(基线 ~200 字/秒),按 backlog 微调。

**验收**:
```bash
cargo test -p opencode-chat-core        # store/smoother 单测
# 浏览器:合成流文本匀速淡入
```
**DoD**:文本从合成流匀速淡入、丝滑无抖动(先 mock 验证体验,解耦服务端)。

### Phase D — 接真实 opencode

**建/改**:`wasm/transport.rs`、`wasm/clock.rs`、`core/protocol.rs`(解码 text 子集)、`wasm/lib.rs`(`ChatCanvas::new(canvas, {serverUrl, sessionId})` + `.start()`)。
**实现要点**:
- `transport.rs`:`gloo_net::eventsource` 连 `serverUrl + "/api/event"`,事件入队;`Connection::poll` 取队列;原始串直传(BR1)。
- `clock.rs`:`performance.now()`。
- 本地起 `opencode serve` 拿 URL + sessionId。

**验收**:连本地 `opencode serve`,发一句话,assistant 回复逐字淡入。
**DoD**:真实 SSE → 逐字淡入上屏;心跳/未知事件不崩(AR12)。

### Phase E — 录制 / 重放

**建/改**:`core/record.rs`(Recorder 完整 + Player 正式)、`crates/core/tests/replay.rs`、`wasm/transport.rs`(挂 Recorder)。
**实现要点**:
- `Recorder` 包 `Connection` 记 `(t, raw)` 到 jsonl;`Player` 读 jsonl 重放,dt 取录像值。
- `tests/replay.rs`:同一录像重放两次,断言最终 `Store` 状态一致(确定性)。

**验收**:
```bash
cargo test -p opencode-chat-core --test replay
```
**DoD**:录一条真实会话 → 重放出完全相同结果;断网也能复现。

---

## 6. opencode 协议速查(实测,见 0001 §3.1)

- SSE:`GET /api/event`,信封 `{id, type, properties}`;首发 `server.connected`,每 10s `server.heartbeat`。
- 热路径 `message.part.delta`:`{sessionID, messageID, partID, field, delta}`(纯文本增量,append-only)。
- 对账 `message.part.updated`:`{part, time}`,part 按 `type` 分(Plan1 只认 `text`)。
- 状态 `session.status`:`{status:{type: idle|busy|retry,...}}`。
- 快照(Plan1 可暂不接):`GET /api/session/:sessionID/message`(cursor 分页)。
- 类型出处:`packages/core/src/v1/session.ts`、`packages/opencode/src/server/.../handlers/event.ts`。

---

## 7. 验证点记录(Plan1 收尾填,作为 Plan2 输入)

| 验证点 | 预期 | 实测 |
|---|---|---|
| 跨界开销与新增字符成正比 | 稳态与总长无关 | 架构已落地:每帧一次 layout 批调用(AR10),layout 返回平铺 `Float32Array`(CR4);稳态增量 = 新增 grapheme 数。运行时 fps 待浏览器实测(本环境无 GPU)。 |
| WebGPU 满速吐字 fps | ≥60 | 待浏览器实测。管线已就绪:实例化 TriangleStrip + 单次 instanced draw;`glyph.wgsl` 经 naga 构建期校验通过。 |
| 中文/emoji 光栅化正确性+延迟 | 正确,可接受 | grapheme 切分一致(JS `Intl.Segmenter` ↔ Rust `unicode-segmentation`);"Hi 你好"/emoji 顺序由 native 单测覆盖;像素正确性待浏览器。 |
| 平滑器手感(200 字/秒+追赶) | 舒适 | native 单测验证匀速 + backlog 追赶 + 确定性;手感主观项待浏览器。 |
| pretext 句柄+零拷贝边界 | 顺畅,grow 后视图重建无误 | Plan1 用 `measureText` 桥(同接口 text+maxWidth→位置),pretext 留 Plan2 drop-in;零拷贝 `Float32Array` 边界已通(`vite build` 通过)。 |

---

## 8. 风险与回退

- **wgpu/wasm-bindgen 版本漂移**:锁 wasm-bindgen,wgpu 用前确认最新稳定 + 浏览器 WebGPU 可用性;不可用先退 WebGL2 后端(wgpu 同 API)。
- **pretext 集成**:先在纯 JS 里跑通 `layout()` 再接 wasm,降低边界调试难度。
- **GPU 不可用**:Plan1 不做 Canvas2D 降级(留 Plan2),但 `RenderBackend` trait 边界要先留好(CR3)。
- **Phase 阻塞**:任一 Phase 不通,先回到上一个可跑态,不堆积。

---

## 9. 实施状态(Plan1 落地记录)

> 完整总结见 **[phase1_progress.md](./phase1_progress.md)**(文件树/铁律对照/运行方式/Plan2 入口)。

**已实现并通过验证**(`cargo` + `wasm-pack` + `vite` 工具链):

| Phase | 产物 | 验证 |
|---|---|---|
| A 脚手架 | workspace 配置 + 三 crate 骨架 + web harness | `cargo build --workspace` ✓;`vite build` 打包 wasm+JS ✓ |
| B 静态串 | atlas/scene/glyph.wgsl + glyph/layout 桥 | `glyph.wgsl` naga 校验 ✓;measureText 桥 tsc ✓ |
| C 合成流+平滑+淡入 | store/smoother/content/app/frame + effects | `cargo test -p opencode-chat-core` 18 单测 ✓(匀速/追赶/spawn_time 递增/对账) |
| D 接 opencode | transport(SSE)/clock/protocol/ChatCanvas | `cargo build -p ...-wasm --target wasm32` ✓;protocol 解码单测 ✓ |
| E 录制/重放 | record(Recorder/Player)+ tests/replay.rs | `cargo test --test replay` 3 测 ✓(两次重放 Store 逐字节一致 + jsonl 往返) |

**卡口全绿**:`cargo fmt --check`、`cargo clippy --workspace -- -D warnings`(native)、
`cargo clippy -p ...-wasm --target wasm32 -- -D warnings`、`cargo test`(22 测)。

**与原计划的有据偏差**(均已在代码注释/本文标注):
- `unsafe_code = forbid → deny`:原文注释要求 render/wasm 可局部 `#[allow]`,`forbid` 不可被降级,改 `deny`。
- core 的 `frame.rs` 用语义字形 `FrameGlyph`(不含 UV),render 侧才有 GPU `GlyphInstance`(Pod);`LayoutResult` 只回位置(CR4),cluster 文本由 app 侧 reveal 提供。
- `Engine<C,L,R>` 三参,`now_ms` 由注入 `dt` 累加(`Clock` seam 在 wasm 帧循环/Recorder 用),保确定性。
- wasm 平台依赖移入 `[target.'cfg(wasm32)']`,并加 `wgpu`(canvas→surface 胶水,render 保持无 web-sys);crate 整体 `#![cfg(wasm32)]` 以便 native workspace 构建跳过。
- web 排版桥 Plan1 用 Canvas2D `measureText`(自洽可跑、零字体打包),pretext 作 Plan2 同接口 drop-in;故 `web/package.json` 暂不依赖 pretext(npm 不支持 `link:`)。
- `wasm-opt = false`:wasm-pack 自带 wasm-opt 过旧,不识别 rustc 新发的 bulk-memory/nontrapping-fptoint;release profile 已 LTO+`opt-level=z`。
- **HiDPI 修正(真机回归)**:首版按 CSS 像素(dpr=1)渲染,Retina 上文字发虚;改为后备缓冲 = CSS×devicePixelRatio、字体/排版/光栅化全按设备像素,1:1 锐利。同时接窗口 `resize` 监听:Rust 侧直驱 `WebGpuBackend::resize` + `Engine::set_max_width`。

**待浏览器/真机验证**(本环境无 GPU/浏览器,无法跑):像素正确性、≥60fps、合成流与真实 SSE 的实际淡入手感、§7 主观项。运行方式见各 crate README 与 `web/src/main.ts` 顶部用法。
