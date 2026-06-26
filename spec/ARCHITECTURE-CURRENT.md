# infinite-chat · 当前架构梳理（实现态）

> 基于 2026-06 实际代码（`crates/{core,render,wasm}` + `web/`）通读整理，对照 `spec/architecture.md`
> 的 13 模块计划，标注**已落地**与**计划/实际差异**。目的：一图看懂整个项目的渲染流程、组件定位与边界。

---

## 0. 一句话定位

**用游戏引擎的手法做 LLM 对话渲染**：消费 opencode server 的 SSE 事件流，把流式吐出的对话
渲染成一块 GPU 画布上、可无限缩放平移、文字即 SDF 图元的「无限会话」。Rust 写核心 → 编译 WebAssembly
→ wgpu（WebGPU 优先、WebGL2 兜底）出像素 → 交付为框架无关的可嵌入组件。

本质：**一个"实体为文字的游戏客户端"**。fps/内存只与"可见的一屏"成正比，与历史总量无关。

---

## 1. 仓库布局：主代码 vs 参考库

```
opencode-chat/
├── crates/              ★ Rust 工作区(本项目核心)
│   ├── core/            平台无关内核(大脑) — 14 个 .rs，~9k 行
│   ├── render/          wgpu 渲染后端 — 5 .rs + WGSL，~2.5k 行
│   └── wasm/            #[wasm_bindgen] 薄胶水层 — 7 .rs，~1.6k 行
├── web/src/             ★ 前端 harness/宿主(TS) — ~2.7k 行
├── vendor/
│   └── jcode-render-core   vendored markdown 文档模型(pulldown-cmark 封装)
├── spec/                文档(决策 ADR / 架构 / plan / 知识)
├── scripts/             联调脚本(chat.mjs / bake-msdf.mjs 等)
└── RaTeX/ drei/ lygia/ onedraw/ infinite-canvas-tutorial/   ← 参考/依赖库(各有独立 .git)
```

**只有 `crates/` + `web/` + `vendor/jcode-render-core` 是本项目代码**。其余目录是参考实现或
git 依赖（`RaTeX` 数学排版按 rev 钉在 Cargo.toml，本地源不入库）。

### 四层职责与铁律

| 层 | crate / 目录 | 职责 | 关键铁律 |
|---|---|---|---|
| **内核** | `crates/core` | 平台无关的对话渲染大脑：协议、状态、节奏、内容、排版编排、每帧循环 | **CR1**：零 `wasm-bindgen`/`web-sys`/`wgpu` 依赖 → native 可测；**R8**：不碰墙钟，`dt_ms` 注入逐帧累加 → 确定性重放 |
| **渲染** | `crates/render` | wgpu 管线 + WGSL 着色器，吃 `FrameData` 出像素 | **CR3**：后端 trait 选择；**CR4**：instance 零拷贝；不依赖 web-sys，surface 由 wasm 注入 |
| **胶水** | `crates/wasm` | `#[wasm_bindgen]` 平台接缝：canvas→surface、JS 回调注入、`RenderSink` 实现 | **CR5**：业务零逻辑，只做平台接缝 |
| **宿主** | `web/src` | Vite harness：挂 canvas、init wasm、浏览器干重活(Canvas2D 排版/SDF 光栅/图片解码)、调试面板 | 薄 TS 桥；"让浏览器干重活" |

依赖方向单向：`web` → `wasm` → `{render, core}`，`render` → `core`。`core` 不依赖任何平台。

---

## 2. 每帧渲染流程（实现态真实步骤）

入口是 `core::Engine`（`app.rs`），每帧 `frame(dt_ms)` = `advance()`（推进模拟，不出图）+ `render_now()`（出一帧）。

### advance() —— 事件改状态（严格只写）

```
now_ms += dt_ms                          时钟逐帧累加(R8 确定性)
shaderbox_clock.tick(dt)                 shader 动效时钟 30fps 步进(与 rAF 解耦)
turn.tick(now_ms)                        回合收尾看门狗(soft 8s→Stalled / hard 30s→settle)
ingest_events()                          transport.drain → protocol.decode → store.apply(三表+对账)
enqueue_new_text()                       store 新文本 → smoother 各 part 入 reveal 队列
refresh_roles()                          从 store 校正样式角色(snapshot/resync 后才知)
reveal(dt)                               smoother:token 突发 → 匀速到达(grapheme 为单位)
ensure_layouts()                         content 解析尾块(冻结已完成块) → layout 桥排版 → 节点树
schedule(dt)                             RevealScheduler:唯一揭示路径,定每个字形 spawn_time
```

### render_now() —— 渲染只读状态

```
build_frame()                            装配 FrameData:可见集裁剪(SpatialGrid 查可见块)
                                         + glyph/rect/panel/widget/image/shaderbox/embed 实例
                                         + 块装饰(代码底/chip/引用条/标题线/分隔线)
                                         + 锚底平滑跟随(smooth-damp)
sink.submit(&frame)                      GpuSink(wasm):resolve 字源→atlas→几何 morph→backend.draw
```

一句话链路：

> **transport 收流 → protocol 解码 → store 落三表 → fsm 收尾 → smoother 整流 → content 解析尾块
> → layout 排版 → RevealScheduler 定 spawn_time → build_frame 裁剪装配 → GpuSink 上 atlas →
> wgpu instanced draw**

稳态下每帧跨界数据量只与"新增字符数"成正比，与文档总长无关。

---

## 3. 组件图（实际文件 → 职责定位）

### `crates/core`（内核·14 模块）

| 文件 | 计划模块 | 职责 |
|---|---|---|
| `app.rs` (3360) | M13 app | **每帧编排循环** + `build_frame` 装配 + 相机/锚底/滚动/缩放 + 块装饰 + 图片注册表。最重的文件，引擎中枢 |
| `content.rs` (1659) | M6 content | 标签扫描 + markdown 解析(经 jcode)+ `StyledSpan` 语义角色产出 + embed/table 区域 |
| `reveal.rs` (750) | — (新增) | **RevealScheduler：唯一揭示路径**。按风格/门/时钟释放 display 字形定 `spawn_time`，与 token 到达解耦 |
| `highlight.rs` (476) | M6 | 代码高亮(按 hash+lang 缓存) |
| `nodes.rs` (366) | M6/M8 | NodeTree：markdown 结构树(Heading/List/Table/Quote/CodeBlock…)，调试叠加与命中用 |
| `protocol.rs` (357) | M2 | Event 信封 + Part 类型 serde 解码；未知类型→`Ignored`(向前兼容) |
| `math.rs` (347) | — (新增) | 数学排版：RaTeX TeX→DisplayList→FrameGlyph/Rule(行内/显示数学) |
| `store.rs` (314) | M3 | 归一化文档三表(session/message/part)+ delta 乐观追加 + part.updated 全量对账 |
| `boxlayout.rs` (301) | — (新增) | taffy flexbox 盒子树(角色左右分栏 + 内容嵌套) |
| `shaderbox.rs` (246) | M9 | ShaderBox 画板("效果即数据"，icon/glow/raymarch) + 节流时钟 |
| `frame.rs` (216) | — | **FrameData 契约**：Glyph/Rect/Panel/Widget/Image/ShaderBox/Embed —— core↔render 的冻结接口 |
| `camera.rs` (180) | M10/M11 | 2D 相机(平移+缩放，world unit = CSS px) |
| `record.rs` (181) | — | Recorder/Player：录像重放(确定性) |
| `smoother.rs` (167) | M5 | 流式节奏整流，逐 grapheme 匀速吐字 |
| `fsm.rs` (167) | M4 | TurnTracker：回合聚合 + 收尾看门狗 |
| `seam.rs` (122) | M0 | **平台能力 trait 注入点**：Connection/LayoutEngine/RenderSink/Clock（CR2 接缝） |
| `spatial.rs` (115) | M8 | SpatialGrid：逐帧重建块 AABB，视口查可见块 |
| `codeblock.rs` `embed.rs` `theme.rs` `support.rs` | M6/M8 | 代码块滚动窗 / 嵌入 FSM / 主题色令牌 / native stub |

### `crates/render`（渲染·wgpu）

| 文件 | 模块 | 职责 |
|---|---|---|
| `backend.rs` (1265) | M10 | `WebGpuBackend`：所有 wgpu 管线初始化 + 单 pass 绘制 + 相机 globals |
| `morph.rs` (498) | M9 | Scene/PanelScene：流式几何形变保留态(past→current 补间，不跳变) |
| `atlas.rs` (457) | M8 | SDF/MSDF 图集：定长瓦片 page-pool + LRU 淘汰 |
| `scene.rs` (242) | M8 | GPU instance 结构定义(7 种 `#[repr(C)]` Pod 实例 + 顶点布局) |
| `effects.rs` (44) | M9 | EffectProfile / Globals(相机 uniform) |
| `shaders/` | M9/M10 | WGSL：`base/`(glyph·rect·panel·image·sdf) `markdown/`(box·rule·widget) `shaderbox/`(icons·glow_orb·raymarch·channel) |

### `crates/wasm`（胶水）

| 文件 | 模块 | 职责 |
|---|---|---|
| `lib.rs` (1105) | M12 api | `ChatCanvas` 宿主 API + **GpuSink**(实现 core::RenderSink，字源 resolve→atlas→morph→backend) |
| `layout_bridge.rs` (219) | M7 | LayoutEngine 实现：把排版请求转发 JS(Canvas2D)，零拷贝拿回位置 |
| `transport.rs` (98) | M1 | SseConnection(gloo-net EventSource)+ REST 快照拉取 |
| `glyph_bridge.rs` `msdf.rs` `clock.rs` `observe.rs` | M7/M0 | JS 光栅回调 / MSDF 字体元数据 / WebClock / tracing |

### `web/src`（宿主 harness）

| 文件 | 职责 |
|---|---|
| `main.ts` (223) | 入口：挂 canvas(HiDPI)→ init wasm → `new ChatCanvas` → start；调试开关(?debug/?replay/?gallery/?msdf)；demo reel 导演 |
| `layout-bridge.ts` (627) | **浏览器侧排版器**：Canvas2D measureText + Intl.Segmenter 折行(拉丁不断词/CJK 每字可断/禁则)+ 按角色度量。**唯一布局器** |
| `glyph-raster.ts` (127) | Canvas2D 光栅字形 → 覆盖率/TinySDF tile 交 atlas |
| `msdf.ts` `math-fonts.ts` | 离线 MSDF 图集加载 / KaTeX 数学字体 |
| `image-loader.ts` `embed-overlay.ts` | 浏览器解码图片上传纹理 / 动图 DOM overlay 跟随相机 |
| `transport.ts` `replay.ts` `chat-input.ts` | 合成流 / 录像重放 / 调试输入框直发 opencode |
| `debug-panel.ts` `style-panel.ts` `input.ts` | 帧统计 HUD / Figma 式样式面板 / 画布手势(滚/缩/拖) |

---

## 4. JS↔wasm 边界：谁干重活

设计原则"**让浏览器干重活，wasm 只持元数据**"。边界落在两处：

**排版桥（M7）**：core 产 `StyledSpan`(语义角色，非像素) → `layout_bridge`(wasm) 转发 →
`layout-bridge.ts`(JS) 用 Canvas2D + Intl.Segmenter 折行度量 → 平铺 `Float32Array [x,y,w,h]*N`
零拷贝回 wasm。**排版器刻意做成可替换**：契约是"StyledSpan in → 带位置 run out"，换 cosmic-text
只动这一模块。

**字形光栅**：GpuSink resolve 出"该字走哪条源"(位图/TinySDF/MSDF/RGBA emoji)，未命中则调
`glyph-raster.ts`(JS Canvas2D 画字)→ TinySDF 生成距离场 → 上 atlas。固定字形走离线 MSDF。

**图片/数学/SVG**：浏览器解码图片→上传纹理；RaTeX 在 Rust 算数学几何但字形仍走 MSDF/TinySDF；
动图交 DOM overlay。

---

## 5. GPU 绘制顺序（单 render pass，back→front）

`build_frame` 装配的 `FrameData` 经 GpuSink 交 `WebGpuBackend.draw`，**一个 render pass 内按 z-order**
依次 instanced draw（每次 `draw(0..4, 0..N)` = 一个 quad 实例化 N 份）：

```
1. panels      表格框/网格/AO(参数化 SDF，storage buffer 携变长参数)
2. rects        块装饰:代码块底 / 行内码 chip / 删除线 / 引用条 / 标题线 / 分隔线
3. widgets      markdown 组件(复选框等，box/rule SDF)
4. images       图片纹理(每图 per-image bind group)
5. shaderboxes  ShaderBox 画板(icons / glow_orb / raymarch / channel 管线)
6. glyphs       文字(SDF/MSDF，最上层；spawn_time 在 GPU 算淡入，CPU 零参与)
```

所有图元共用同一相机/视口裁剪；文字、矩形、图片共用统一 quad + 实例化管线（CR4 零拷贝）。
着色器淡入靠 `time - spawn_time` 在 GPU 算，逐字动画零 CPU。

---

## 6. 计划（spec 13 模块）vs 实现态差异

`spec/architecture.md` 是设计期的 M1–M13 划分；实现后有几处演进，看代码时需注意：

- **模块不等于文件夹**。计划里 transport/protocol/store/fsm 等都标"wasm 归属"，实际除
  `transport` 真在 `crates/wasm` 外，protocol/store/fsm/smoother/content 全在 **`crates/core`**
  (为了 native 可测，CR1)。wasm 只留真正的平台接缝(transport/layout 桥/glyph 桥/clock)。
- **新增 `RevealScheduler`（reveal.rs）**：计划里没有独立"揭示调度"，实现态把它抽成**唯一揭示路径**
  —— 字形上屏节奏由它统一定 `spawn_time`，与 smoother(内容到达)解耦。这是看流式动画时的关键中枢。
- **新增 `nodes`/`boxlayout`/`math`/`shaderbox`/`morph`**：结构树、taffy 盒布局、RaTeX 数学、
  ShaderBox 特效、几何形变保留态 —— 都是计划之外长出来的能力。
- **layout 在 JS**：计划标"JS+wasm"，实现态 627 行排版逻辑确实在 `web/src/layout-bridge.ts`，
  wasm 侧只是 219 行转发桥。
- **降级**：instance 已 `BROWSER_WEBGPU | GL`(WebGPU 优先、WebGL2 兜底，同一份代码已启用待专测)；
  Canvas2D 不做。WebGL2 无 compute，逐字 compute 特效为 WebGPU 专属。

---

## 7. 看代码的推荐入口

1. **`crates/core/src/app.rs`** —— `Engine::advance` + `build_frame`，引擎中枢，先看这两个方法。
2. **`crates/core/src/frame.rs`** —— `FrameData` 契约，core↔render 的接口冻结点。
3. **`crates/wasm/src/lib.rs`** —— `ChatCanvas`(宿主 API)+ `GpuSink::submit`(渲染落地)。
4. **`crates/render/src/backend.rs`** —— `WebGpuBackend.draw` 绘制顺序 + WGSL 管线。
5. **`web/src/main.ts`** + **`layout-bridge.ts`** —— 宿主编排 + 浏览器排版。
6. 设计动机/铁律 → `AGENTS.md` → `spec/decision/0000-overview.md` → `spec/architecture.md`。
