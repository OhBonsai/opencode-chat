# Plan 1:最小功能原型

- 日期:2026-06-13
- 前置:architecture.md(§7 竖切)、testing-and-benchmark.md(§5)
- 目标:**竖切打通整条链路**,用最少模块验证"通信模式 + 帧率 + 流式体验"三个核心
  假设。不求功能全,只求那条主动脉跑起来。

---

## 一、成功标准(Definition of Done)

1. 连上本地 `opencode serve`,一条真实 assistant 文本回复**逐字符淡入**地渲染到
   GPU 画布上,稳态 ≥ 60fps。
2. 同一条会话能 **录制 + 确定性重放**(无需连服务端也能复现)。
3. wasm panic 有可读 backtrace,关键路径有 console 日志。
4. 整条链路按 architecture.md 的模块边界切分,不是一坨 main 函数。

达成即 Plan 1 完成,进入 Plan 2。

---

## 二、范围

### In scope(本期做)

- `transport`:EventSource 收 SSE(只处理文本相关事件)
- `protocol`:解码 `message.part.delta` / `message.part.updated`(text part)/
  `session.status`,其余 `Ignored`
- `store`:单 session、message/part 最小三表,delta 追加 + updated 对账
- `smoother`:固定速率逐 grapheme 吐字,打 spawn_time
- `content`:**纯文本直通**(先不接 markdown/标签/高亮)
- `layout`:pretext 桥,纯文本测量 + 换行,句柄 + 平铺数组回传
- `scene`:glyph atlas(浏览器 OffscreenCanvas 光栅化字形 → 纹理)+ instance buffer
- `effects`:**一个硬编码淡入**(spawn_time 驱动 alpha),无 profile 系统
- `render`:**仅 WebGPU**,像素对齐相机,无降级
- `app`:每帧编排循环
- `api`:`new ChatCanvas(canvas, {serverUrl, sessionId})` + `.start()`
- 可观测:tracing→console、panic hook、record/replay

### Out of scope(留给后续 plan)

markdown / 语法高亮 / 内嵌标签 / 图片 mermaid embed / turn 聚合 / 完整容错(重连
看门狗/快照对账先做最简)/ 多实例同步 / 滚动与视口裁剪 / 选区复制 / 无障碍镜像 /
WebGL2+Canvas2D 降级 / effects profile 系统 / Worker 化 / React/Vue 封装(先用裸
HTML harness)。

---

## 三、技术栈与脚手架

- **Rust crate** → wasm:`wasm-bindgen` + `wasm-pack`
- **GPU**:`wgpu`(WebGPU 后端)
- **SSE**:`gloo-net`(eventsource)+ `wasm-bindgen-futures`
- **解码**:`serde` + `serde_json`
- **分段**:pretext 已用 `Intl.Segmenter`,grapheme 复用其结果
- **日志**:`tracing` + `tracing-wasm`、`console_error_panic_hook`
- **前端 harness**:Vite + `vite-plugin-wasm`,一个 `index.html` + `<canvas>` +
  挂载 wasm(暂不引 React)
- **pretext**:作为 JS 依赖,wasm 通过 import 的 JS glue 调用
- **opencode**:本地 `opencode serve` 提供真实 `/api/event` 与快照

---

## 三.5 代码目录脚手架

### 可借鉴的成熟项目(同语言、最终编 wasm、非本功能)

| 项目 | 为何借鉴 |
|---|---|
| **Ruffle**(ruffle-rs/ruffle) | **最贴切**:Flash 模拟器 Rust→wasm,`core`(纯逻辑,零 wasm-bindgen)/ `render`(多后端 wgpu/webgl/canvas,trait 抽象)/ `web`(wasm-bindgen + npm 包)/ `desktop` 的切分,与我们"纯逻辑 + RenderBackend + wasm 组件 + npm"一一对应 |
| **Rerun**(rerun-io/rerun) | wgpu + egui 查看器,**native 与浏览器双跑**;core 逻辑 crate 零 GUI 依赖,viewer 是可选层;wasm-bindgen 版本锁定的做法 |
| **eframe_template**(emilk/eframe_template) | 最小 **cdylib + rlib 双目标**范式:单 lib.rs 跨平台,`cfg` 条件编译入口 |
| **Vello**(linebender/vello) | 纯 GPU 渲染逻辑零平台代码;examples 各为独立 workspace 成员 |

业界共识(被上述项目反复印证):**core 不碰 wasm-bindgen/web-sys/wgpu,保持 native
可测;平台能力走 trait,wasm-bindgen 层薄、只暴露公共 API;render 后端用 trait 不用
编译开关**。这和我们 architecture.md 的模块边界、0001 的"layout 可替换"、0003 的
"RenderBackend trait"完全合拍。

### 脚手架(workspace,对齐 architecture.md 13 模块)

```
opencode-chat/
├── Cargo.toml                   # workspace 根([workspace.dependencies] 统一版本)
├── rust-toolchain.toml
├── crates/
│   ├── core/                    # 纯逻辑,native 可测,★零 wasm-bindgen/web-sys/wgpu★
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── protocol.rs      # 事件/Part 解码
│   │       ├── store.rs         # 文档三表 + 对账
│   │       ├── fsm/             # part.rs / turn.rs / tag.rs(plan1: part 最小)
│   │       ├── smoother.rs      # 逐 grapheme 整流
│   │       ├── content/         # segmenter/markdown/highlight(plan1: 仅直通)
│   │       ├── input.rs         # 滚动/选区逻辑(plan1: 空)
│   │       ├── app.rs           # 帧编排(纯逻辑)
│   │       └── seam.rs          # ★平台缝 trait★(见下)
│   ├── render/                  # wgpu 渲染,native+wasm 双目标
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── backend.rs       # trait RenderBackend + WebGpuBackend(plan1)
│   │       ├── scene.rs         # instance buffer / 视口裁剪(plan1: 无裁剪)
│   │       ├── atlas.rs         # glyph atlas
│   │       ├── effects.rs       # tween + profile(plan1: 一个淡入)
│   │       └── shaders/*.wgsl
│   └── wasm/                    # cdylib:wasm-bindgen API + 平台缝实现
│       └── src/
│           ├── lib.rs           # #[wasm_bindgen] ChatCanvas(api 层)
│           ├── transport.rs     # Connection impl:EventSource(gloo-net)
│           ├── layout_bridge.rs # LayoutEngine impl:调 pretext(JS)
│           ├── glyph_bridge.rs  # 调 OffscreenCanvas 光栅化字形
│           └── observe.rs       # tracing-wasm + panic hook + record
├── web/                         # npm 包 + Vite harness
│   ├── package.json  vite.config.ts  index.html
│   ├── src/{main.ts, pretext-bridge.ts, glyph-raster.ts}
│   └── pkg/                     # wasm-pack 输出(.gitignore)
├── tests/                       # native 集成 + 录像重放
│   └── replays/*.jsonl
└── xtask/                       # 构建编排(wasm-pack 调用)
```

### 平台缝 trait(关键:让 core 保持 native 可测)

core 不直接依赖 EventSource / pretext / wgpu,而是定义 trait,由 wasm crate 注入
真实实现、native 测试注入 mock/replay 实现:

```rust
// core/src/seam.rs
pub trait Connection  { fn poll(&mut self) -> Vec<RawEvent>; }   // EventSource / Player(重放) / Mock
pub trait LayoutEngine{ fn layout(&mut self, spans:&[StyledSpan], width:f32) -> LayoutResult; } // pretext / stub
pub trait Clock       { fn dt(&self) -> f32; }                   // rAF / 固定(重放)
pub trait RenderBackend{ fn draw(&mut self, frame:&Frame); }     // wgpu /(后续)canvas2d

// core::app::Engine<C, L, R> 由组装方提供实现
```

收益:**录制/重放 = 一个 Player(Connection)+ 固定 Clock**(testing §0 的基石天然落地);
native 单测用 mock Connection + stub LayoutEngine(等宽度量)跑纯逻辑,无需浏览器。

### 工具链

- **wasm-pack + wasm-bindgen**(非 trunk):交付物是 **npm 包**,wasm-pack 直接产出
  npm 可用产物(对齐 Ruffle 的 `web/packages`);trunk 是给整页 app 的,不适合库
- `wasm-bindgen` **锁定精确版本**(Ruffle/Rerun 都这么做,它对版本敏感)
- `crates/wasm` 设 `crate-type = ["cdylib"]`;`core`/`render` 默认 rlib(可被 native
  测试 + 将来 native 壳复用)
- release profile:`opt-level="z"` + `lto=true` + `codegen-units=1` + `strip=true`(控包体)
- Vite + `vite-plugin-wasm` 加载 pkg;pretext 作 JS 依赖

### Rust lint 与代码质量门禁

集中在 workspace 根统一配置,所有 crate 继承,CI 强制。

- **`[workspace.lints]`**(Rust 1.74+,单点开关,各 crate `[lints] workspace = true`):
  ```toml
  # Cargo.toml(workspace 根)
  [workspace.lints.rust]
  unsafe_code = "forbid"          # 我们无需 unsafe;render 若需可在该 crate 局部放开
  missing_debug_implementations = "warn"
  rust_2018_idioms = "warn"
  unreachable_pub = "warn"

  [workspace.lints.clippy]
  all = { level = "warn", priority = -1 }
  pedantic = { level = "warn", priority = -1 }
  # 务实放开几条过吵的 pedantic:
  module_name_repetitions = "allow"
  must_use_candidate = "allow"
  # 热路径/正确性相关收紧:
  unwrap_used = "warn"            # 库代码禁裸 unwrap,错误显式处理(测试可 allow)
  panic = "warn"
  ```
- **rustfmt**:`rustfmt.toml` 固定风格(`edition=2021`、`max_width=100`、
  `imports_granularity="Module"`、`group_imports="StdExternalCrate"`),消除格式分歧
- **cargo-deny**:`deny.toml` 审查依赖许可证 / 安全公告(RustSec)/ 重复依赖——
  wasm 供应链 + 包体敏感,这条值得从一开始就有
- **wasm 相关**:`core` 用 `unsafe_code = "forbid"` 保证纯逻辑零 unsafe;
  WGSL shader 由 `naga` 在构建期校验(wgpu 自带)
- **CI 门禁**(对齐 testing §四):
  ```
  cargo fmt --all --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo deny check
  ```
  `-D warnings` 把 lint 升为硬错误,阻断合并。本地配 pre-commit 跑 fmt+clippy。

### Plan 1 填充范围

- `core`:protocol(仅 text)、store(最小)、smoother、app、seam——**建满**;
  fsm 仅 part 最小、content 直通、input 空
- `render`:backend(WebGPU)、atlas、scene(无裁剪)、effects(一个淡入)——**建满**
- `wasm`:transport、layout_bridge、glyph_bridge、observe、api——**建满**
- `web`:harness 三件套
- `tests`:一个重放一致性测试

其余 crate/模块**先建空文件占位**(标 `// TODO: plan N`),保证目录骨架完整、
后续 plan 往里填,不重构。

---

## 四、任务分解(按可独立验证的相位推进)

> 原则:每个相位结束都有**屏幕上看得见**的产出,绝不先攒模块后集成。

### Phase A — 脚手架 + 空画布(地基)

- A1 建 crate + wasm-pack + Vite harness,canvas 挂载
- A2 `render`:wgpu 初始化 WebGPU,每帧清一个背景色
- A3 `observe`:tracing→console + panic hook;`app` 起 rAF 帧循环
- ✅ 验收:画布显示纯色,console 有帧日志,故意 panic 能看到 Rust backtrace

### Phase B — 渲染一个静态字符串(渲染链路)

- B1 `layout`:pretext 桥,测量 + 换行一段定文本(`"Hello 你好 🚀"`),回传位置
- B2 `scene`:glyph atlas——JS 侧 OffscreenCanvas 光栅化每个 grapheme → 上传纹理;
  生成 GlyphInstance(pos/uv)
- B3 `render`:相机 world=CSS px;instanced draw 画出该字符串
- ✅ 验收:静态字符串位置正确、用浏览器系统字体、中英 emoji 都对(无字体打包)

### Phase C — 合成流 + 平滑器 + 淡入(体验核心)

- C1 `store`:最小三表 + delta 追加
- C2 合成事件发生器(record/replay 的 Player 雏形):按可控速率吐文本 delta
- C3 `smoother`:固定速率逐 grapheme reveal,打 spawn_time
- C4 `effects`:硬编码淡入(WGSL `alpha = smoothstep(0, fade, time-spawn_time)`)
- C5 `app`:把 store→smoother→content(直通)→layout→scene→effects→render 串成每帧
- ✅ 验收:文本从合成流匀速淡入,丝滑无抖动(**先用 mock 验证体验,解耦服务端**)

### Phase D — 接真实 opencode(通信验证)

- D1 `transport`:gloo-net EventSource 连 `/api/event`,事件入队
- D2 `protocol`:解码 text part 的 delta/updated + session.status
- D3 接 store;updated 做最简对账(覆盖);忽略其余事件类型
- D4 `api`:`new ChatCanvas(canvas, {serverUrl, sessionId}).start()`
- ✅ 验收:连本地 `opencode serve`,发一句话,assistant 回复逐字淡入上屏

### Phase E — 录制 / 重放(debug 地基)

- E1 `observe/record`:在 transport+input 出口记 `(t, event)`
- E2 Player:用录像驱动 app,dt 取录像值
- E3 一个 native `cargo test`:同录像重放两次,断言最终 store 状态一致
- ✅ 验收:录一条真实会话 → 断网重放出完全相同结果

---

## 五、里程碑顺序

```
A(空画布) → B(静态字) → C(合成流淡入) → D(真 opencode) → E(录制重放)
   地基          渲染链路       体验核心          通信验证        debug 地基
```

C 之后体验假设已验证;D 之后通信假设已验证;E 之后具备复现能力,可放心进 Plan 2。

---

## 六、本期要回答的关键问题(验证点)

1. **跨界开销**:wasm↔pretext 每帧调用,稳态是否真与"新增字符数"成正比?(埋点测)
2. **帧率**:合成流满速吐字 + 淡入,WebGPU 下能否稳 60fps?
3. **字形光栅化路径**:OffscreenCanvas 光栅化 grapheme → atlas 的延迟与正确性
   (尤其中文大字符集、emoji 彩色)
4. **平滑器手感**:base_rate(~200 字/秒)与缓冲追赶在真实流下是否舒适
5. **pretext 集成**:句柄管理 + 平铺数组零拷贝在 wasm 边界是否如设计顺畅

把这些数字/结论记进 Plan 1 收尾笔记,作为 Plan 2 的输入。

---

## 七、交付物

- 可跑的 wasm 组件 + HTML harness:连 opencode 流式淡入渲染
- record/replay + native 重放测试
- Plan 1 收尾笔记:上述 5 个验证点的实测结论 + 暴露的问题清单

---

## 八、明确不验证(避免范围蔓延)

长对话性能曲线、多 turn、容错完备性、降级、滚动——这些是 Plan 2+ 的事。Plan 1 只
证明"这条主动脉通,且流式体验对"。
