# Phase 1 实施总结(plan1-build-guide 落地记录)

- 日期:2026-06-13
- 范围:[plan1-build-guide.md](./plan1-build-guide.md) 全部 5 个 Phase(A–E)
- 状态:**代码与工具链全绿,且已接通真实 opencode 跑通端到端**(deepseek-v4-pro 逐字淡入、HiDPI 锐利);仅 ≥60fps/手感等主观项待长期观察
- 提交:`29d989a feat: Plan1 最小原型`(83 files,初始提交)
- 配套:[plan1-minimal-prototype.md](./plan1-minimal-prototype.md)、[../architecture.md](../architecture.md)、[../dev-practices.md](../dev-practices.md)

---

## 1. 一句话

从空仓库搭起 **Rust(core/render/wasm)+ WebAssembly + wgpu + Vite harness** 的最小可跑链路:
合成/真实 SSE 事件 → 归一化对账 → 逐 grapheme 匀速整流 → pretext 排版 → glyph 图集 →
WebGPU 实例化绘制 + 着色器淡入。约 **2300 行 Rust + 4 个 TS 桥**,22 个 native 测试。

---

## 2. 交付物(实际文件树)

```
crates/
├── core/                      # 平台无关内核,零 wasm/web-sys/wgpu(CR1)
│   ├── src/{lib,seam,protocol,store,smoother,content,app,frame,record,support,fsm}.rs
│   └── tests/replay.rs        # 确定性重放守门
├── render/                    # wgpu 后端,无 web-sys(CR3)
│   └── src/{lib,backend,atlas,scene,effects}.rs + shaders/glyph.wgsl
└── wasm/                      # 平台胶水,仅 wasm32 有内容(CR5)
    └── src/{lib,transport,layout_bridge,glyph_bridge,clock,observe}.rs
web/
├── {index.html,vite.config.ts,tsconfig.json,package.json}
└── src/{main,pretext-bridge,glyph-raster}.ts + wasm-pkg.d.ts
scripts/                          # opencode 联调(Node mjs)
└── {serve,dev-web,chat,api-paths}.mjs + README.md
```

> 每 crate 配 README + 顶层 `//!` mod doc(R5)。`fsm.rs` 为 Plan2 占位。

---

## 3. 各 Phase 落地

| Phase | 内容 | 关键实现 | 验证 |
|---|---|---|---|
| **A** 脚手架+空画布 | workspace 配置、三 crate 骨架、web harness、wgpu init+清屏、panic hook、rAF 循环 | `WebGpuBackend` 初始化(adapter/device/surface);`ChatCanvas::new/start` | `cargo build --workspace` ✓;`vite build` 打包 wasm+JS ✓ |
| **B** 静态串 | atlas/scene/glyph.wgsl、glyph 光栅化桥、pretext 排版桥 | shelf 装箱单纹理 + UV;实例化 TriangleStrip;measureText+Intl.Segmenter | `glyph.wgsl` naga 校验 ✓;tsc ✓ |
| **C** 合成流+平滑+淡入 | store/smoother/content/app/frame、effects、Player | 逐 grapheme 匀速(基线 200 字/秒 + backlog 追赶);`spawn_time` 着色器淡入 | core 18 单测 ✓(匀速/追赶/spawn 递增/对账) |
| **D** 接 opencode | transport(SSE)/clock/protocol、`ChatCanvas` 起流 | gloo-net EventSource 入队;`performance.now`;信封解码 text 子集 | **端到端通过**:真实 opencode SSE → deepseek-v4-pro 回复逐字淡入(详见 §9) |
| **E** 录制/重放 | record(Recorder/Player)、tests/replay.rs | jsonl 录像 + 虚拟时钟回放 | `--test replay` 3 测 ✓(两次重放 Store 逐字节一致 + jsonl 往返) |

---

## 4. 铁律落地对照

| 铁律 | 落点 |
|---|---|
| **CR1** core 零平台依赖 | core 仅 serde/thiserror/tracing/unicode-segmentation;wgpu/web-sys 全在 render/wasm |
| **CR2** 平台能力走 seam | `Connection/LayoutEngine/Clock/RenderSink` trait 注入,native 用 stub 可测 |
| **CR3** 后端 trait 选择 | `RenderBackend` trait;无 `cfg` 堆后端 |
| **CR4** 跨界平铺零拷贝 | layout 回 `Float32Array`;`GpuInstance` 为 `bytemuck::Pod` |
| **CR5** wasm 薄 API | 逻辑全在 core,`ChatCanvas` 只接平台 |
| **AR4** delta+updated 对账 | `Store::apply_part_updated` 全量覆盖;proptest 编码不变量 |
| **AR7** grapheme 单位 | smoother/app 用 `unicode-segmentation`,JS 用 `Intl.Segmenter` |
| **AR10** 每帧一次批调用 | app 整段文本一次 `layout` |
| **AR12** 未知→Ignored | protocol 未知 type/part 不 panic |
| **R8/R9** 确定性 | 时间由注入 `dt` 累加,无墙钟/随机 → 重放可复现 |

---

## 5. 卡口结果(全绿)

```
cargo fmt --all --check                                    ✓
cargo clippy --workspace --all-targets -- -D warnings      ✓ (native)
cargo clippy -p ...-wasm --target wasm32 -- -D warnings     ✓
cargo test --workspace                                      ✓ 22 测
cargo build -p ...-wasm --target wasm32-unknown-unknown     ✓
wasm-pack build + vite build                                ✓
```
> `cargo deny check` 未跑(cargo-deny 未装,plan 标注「可后置」)。

---

## 6. 与原计划的有据偏差

均已在代码注释 / plan §9 标注:

1. **`unsafe_code = forbid → deny`**:原注释要求 render/wasm 可局部 `#[allow]`,`forbid` 不可降级。
2. **frame 类型拆分**:core `FrameGlyph`(语义,无 UV)vs render `GpuInstance`(Pod);`LayoutResult` 只回位置,cluster 由 app reveal 提供(CR4)。
3. **`Engine<C,L,R>` 三参 + `now_ms` 累加**:`Clock` seam 在 wasm 帧循环/Recorder 用,保确定性。
4. **wasm 平台依赖移入 `[target.'cfg(wasm32)']` + 加 `wgpu`**:canvas→surface 胶水在 wasm(render 仍无 web-sys);crate 整体 `#![cfg(wasm32)]` 以便 native workspace 构建跳过。
5. **排版桥 Plan1 用 Canvas2D measureText**:自洽可跑、零字体打包;pretext 作 Plan2 同接口 drop-in。故 `web/package.json` 暂不依赖 pretext(npm 不支持 `link:`)。
6. **`wasm-opt = false`**:wasm-pack 自带 wasm-opt 过旧,不识别 rustc 新发的 bulk-memory/nontrapping-fptoint;release profile 已 LTO+`opt-level=z`。
7. **HiDPI 修正(真机回归)**:首版 dpr=1 导致 Retina 发虚;改为后备缓冲 = CSS×dpr、字体/排版/光栅化全按设备像素,1:1 锐利。同时接了窗口 `resize` 监听(Rust 侧驱动 `WebGpuBackend::resize` + `Engine::set_max_width`)。
8. **SSE 路径无 `/api` 前缀**:实测本机 opencode 路由是 `/event`、`/session`(decision 0001 记的 `/api/event` 是更早 build)。transport 默认连 `/event`,兼容已带路径的 URL。
9. **`message.part.delta` 无 `sessionID`**:实测 delta properties 仅 `{messageID,partID,field,delta}`,故 `PartDeltaProps.session_id` 改为 `#[serde(default)]`,否则每条 delta 解码失败被丢→空屏(Plan1 也不按 session 过滤)。
10. **帧循环/监听器自持(GC 安全)**:JS 端 `ChatCanvas` 句柄被 GC 后,`resize` 回调悬空抛 "closure invoked recursively or after being dropped"。修:Rust 侧 `on_resize.forget()` + `main.ts` 把实例挂 `window` 保活。

---

## 7. 如何运行

**纯逻辑验证(无需浏览器):**

```bash
cargo test --workspace                        # 22 测(core 18 + replay 3 + render naga 1)
cargo test -p opencode-chat-core --test replay
```

**合成流 demo(只需浏览器,无需服务端):**

```bash
cd web && npm run dev    # = wasm-pack build + vite;改了 Rust 需重跑本命令
# Chrome/Edge 113+(需 WebGPU)打开 http://localhost:5173 → 预设文字逐字淡入
```

**接真实 opencode(端到端,见 `scripts/README.md`):**

```bash
node scripts/serve.mjs                                    # 1) 起 opencode serve(4096)
node scripts/dev-web.mjs                                  # 2) 起前端
open "http://localhost:5173/?server=http://localhost:4096"   #    先开页面(连上 SSE!)
node scripts/chat.mjs                                     # 3) 多轮对话 → 逐字淡入
```

> ⚠️ Plan1 没接快照,SSE 只推"连上之后"的事件 → **必须先开页面再发消息**;刷新会丢历史。

---

## 8. 待补 / Plan2 入口

- **快照 catch-up(Plan1 主动跳过,见 plan §6)**:连上 SSE 先 `GET /session/:id/message` 拉历史灌进 store;补上后刷新/晚开页面也能看到完整对话,`?session=` 过滤也才生效。**这是当前最该补的一项。**
- **长期观察项**:≥60fps、长会话内存/帧率、合成流与真实 SSE 的淡入手感(plan §7 主观项)。
- **Plan2 自然延伸**(边界已留好):
  - fsm(Part/Turn/Tag 状态机、收尾看门狗)— `core/fsm.rs` 占位
  - content 全量(markdown/标签/高亮)、pretext 真排版接入、视口裁剪/滚动、WebGL2/Canvas2D 降级
  - SSE 重连/心跳看门狗(0003)、录像落盘到 `spec/diagnose/replays/` + `/replay-debug` HUD

---

## 9. 真实 opencode 联调实录(Phase D 端到端)

接 `~/w/agentscode/opencode`(serve 于 4096)跑通,期间校正了若干"文档 vs 实测"的差异:

| 项 | 实测(本机 build) | 处理 |
|---|---|---|
| SSE 端点 | `GET /event`(无 `/api`) | transport 默认 `/event`(§6.8) |
| 建 session | `POST /session` → `{id}` | `chat.mjs` 探测 `/session` 优先 |
| 发消息 | `POST /session/{id}/message` = `session.prompt`,**同步阻塞**直到回完,返回 `{info,parts}` | `chat.mjs` 打印 `🤖` 回复 |
| 发消息 body | `parts` 必填;`model` 选填但**无默认配置时跑空** | 显式传 `model:{providerID,modelID}` |
| delta 形 | `{messageID,partID,field,delta}`,**无 sessionID** | 解码 `sessionID` 改可选(§6.9) |
| text part | `{id,sessionID,messageID,type:"text",text,...}` | `Part::Text` 解码 OK,多余字段忽略 |

**联调脚本(`scripts/`,已提交)**:`serve.mjs` / `dev-web.mjs` / `chat.mjs`(复用 session 多轮、显式 model、默认 `aliyuntokenplan/deepseek-v4-pro`)/ `api-paths.mjs`(dump OpenAPI 排查)。

**结论**:真实 SSE → store 对账 → smoother 整流 → WebGPU 淡入,首轮对话端到端跑通。剩余体验毛刺集中在"快照/时序"(§8 首项)。
