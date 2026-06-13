---
name: bridge-write
description: 写 web/ 下的薄 TS 桥(pretext 排版桥 / glyph 光栅化 / Vite harness)。触发场景:用户说 "/bridge-write"、"改前端桥"、"改 web/"、改 pretext-bridge / glyph-raster 时执行。
---

# /bridge-write · 写 web/ 薄桥

> 触发先 Read DEVMEM § 2。`web/` 只是薄胶水:把浏览器能力(pretext 排版、OffscreenCanvas
> 光栅化、EventSource)接给 wasm。**业务逻辑不在这里**(在 core)。

## 桥铁律 BR

| # | 铁律 | 来源 |
|---|---|---|
| **BR1** | 桥只搬运,禁解析业务事件;SSE 原始串直接给 wasm(JS 不 JSON.parse 再传) | 0003 |
| **BR2** | 跨界结果用 `new Float32Array(wasm.memory.buffer, ptr, cap)` 视图零拷贝;**memory grow 后必重建视图** | 0001 §3.4 |
| **BR3** | pretext `prepare()` 结果留 JS 侧 `Map<u32, Prepared>`,wasm 只持 u32 句柄 | 0001 §3.4 |
| **BR4** | 每帧最多一次 wasm↔pretext 批调用,禁逐 part 循环(同 AR10) | 0001 §3.4 |
| **BR5** | 字形光栅化用 OffscreenCanvas 画 grapheme → `copyExternalImageToTexture`;像素不经 wasm 内存 | 0004 §7 |
| **BR6** | TS strict;桥接口与 wasm 导出类型对齐,禁 `any` 糊弄 | — |
| **BR7** | 图片/SVG/mermaid 浏览器解码/光栅化 → 纹理,wasm 只拿尺寸+UV | 0007 |
| **BR8** | 吐字/分段用 `Intl.Segmenter`(grapheme),与 wasm 侧单位一致(同 AR7) | 0002 §4.1 |

## 文件

- `web/src/pretext-bridge.ts` — StyledSpan → rich-inline fragment → layout → 平铺数组
- `web/src/glyph-raster.ts` — OffscreenCanvas 光栅化 grapheme → 纹理上传
- `web/src/main.ts` — harness:挂 canvas、init wasm、转发输入事件
- `web/index.html`、`web/vite.config.ts`(vite-plugin-wasm)

## 流程

1. 读 wasm 侧对应导出(`#[wasm_bindgen]` 签名)+ 相关 decision
2. 写桥,逐条对照 BR 自检(尤其 BR2 视图重建、BR4 批调用)
3. `tsc --noEmit` + 在 harness 里跑通最小闭环

## 反模式

- ❌ 在 JS 侧解析/累积事件(违 BR1,那是 store 的事)
- ❌ 缓存 Float32Array 视图不在 grow 后重建(违 BR2,悬垂内存)
- ❌ 逐 part 调 pretext(违 BR4)
- ❌ 用 `any` 绕过类型对齐
