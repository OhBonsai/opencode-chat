# opencode-chat-wasm

平台胶水层(M12 api / M1 transport)。薄 `#[wasm_bindgen]` 接口(CR5),业务逻辑全在
`core`;这里只接平台能力:canvas→wgpu surface、SSE、JS 排版/光栅化桥、rAF 帧循环。

## 关键点

- 整个 crate 仅 `wasm32` 目标有内容(`#![cfg(target_arch = "wasm32")]`)。native
  `cargo build --workspace` 把它当空 lib 跳过(平台代码无法在 native 链接)。
- **CR1 守护**:平台依赖(web-sys/gloo-net/wgpu/…)放在 `[target.'cfg(...wasm32)'.dependencies]`,
  不污染 native 构建;core/render 仍零 web-sys。
- **R8**:`clock.rs` 用 `performance.now()`;帧 `dt` 由其差分得出喂 `Engine::frame`。
- **BR1**:`transport.rs` 把 SSE `data` 原文直接入队,不在此/JS 侧解析。

## 模块

| 文件 | 职责 |
|---|---|
| `lib.rs` | `ChatCanvas`(`#[wasm_bindgen]`)+ GpuSink + rAF 循环 + 合成演示 |
| `transport.rs` | SSE 接入(gloo-net EventSource)→ `Connection` |
| `layout_bridge.rs` | 调 JS pretext 排版 → `LayoutEngine` |
| `glyph_bridge.rs` | 调 JS OffscreenCanvas 光栅化 grapheme |
| `clock.rs` | `performance.now()` → `Clock` |
| `observe.rs` | panic hook + tracing-wasm |

## 构建

```bash
# 真实编译验证(native workspace 构建不覆盖本 crate 平台代码):
cargo build -p opencode-chat-wasm --target wasm32-unknown-unknown

# 产出 npm 包(供 web/ harness):
cd ../../web && npm run build:wasm
```

## ChatCanvas API

```ts
new ChatCanvas(canvas, {
  layout:    (text, maxWidth) => Float32Array,                       // [x,y,w,h]*N
  rasterize: (cluster) => ({ data: Uint8Array, width, height }),     // RGBA8
  serverUrl?: string,   // 省略 → 合成流演示(Phase C)
  sessionId?: string,
}).start();
```
