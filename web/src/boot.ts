// boot.ts — Plan 25 PR-A:共享装配。canvas/wasm init + ChatCanvas 构建 + 标准每帧泵
// (overlay/复制/文本层/选区/查找/Dock)+ 字体载入。`main.ts`(实时/演示)与 `/chat`(剧本)复用,
// **不复制粘贴**。param 相关(bench/debug/msdf/demo/film/SSE)留各自入口。

import init, { ChatCanvas } from "../pkg/infinite_chat_wasm.js";
import { layout, measure, FONT_SIZE } from "./layout-bridge";
import { rasterize } from "./glyph-raster";
import { attachCanvasInput } from "./input";

export interface BootOpts {
  /** 预录事件流(replay/剧本经它进引擎的 Player 通道);无则实时/合成。 */
  replay?: { t: number; raw: string }[];
  serverUrl?: string;
  sessionId?: string;
  /** 挂 Cmd+F 查找条(默认挂)。 */
  findBar?: boolean;
}

export interface Booted {
  chat: ChatCanvas;
  canvas: HTMLCanvasElement;
  /** wasm 模块(读 `.memory` 线性内存,?bench 用)。 */
  wasmModule: Awaited<ReturnType<typeof init>>;
}

/** 装配 canvas + 引擎 + 渲染泵 + 字体。返回句柄供入口做各自的 param 特化。 */
export async function bootCanvas(opts: BootOpts): Promise<Booted> {
  const canvas = document.getElementById("chat") as HTMLCanvasElement;
  // HiDPI:后备缓冲 = CSS 尺寸 × devicePixelRatio;排版/光栅化同按设备像素(见 layout-bridge)。
  const dpr = window.devicePixelRatio || 1;
  const cssW = canvas.clientWidth || window.innerWidth;
  const cssH = canvas.clientHeight || window.innerHeight;
  canvas.width = Math.round(cssW * dpr);
  canvas.height = Math.round(cssH * dpr);

  const wasmModule = await init();

  const chat = new ChatCanvas(canvas, {
    layout,
    measure,
    rasterize,
    serverUrl: opts.serverUrl,
    sessionId: opts.sessionId,
    replay: opts.replay,
  });
  chat.set_math_em(FONT_SIZE); // 数学字号 = 正文字号(含 DPR);显示数学 ×1.3(Plan 12)
  chat.start();

  // 画布输入(滚轮/两指滚动/捏合缩放/拖拽平移)。
  attachCanvasInput(canvas, chat);

  // 图片嵌入(Plan 14 ③):轮询待解码图 → 浏览器解码/上传(重活在 JS)。
  {
    const { pumpImageLoads } = await import("./image-loader");
    setInterval(() => pumpImageLoads(chat), 120);
  }

  // Plan 26②:landmark + 读屏播报器(live region;按状态迁移粒度播,不逐 delta)。
  canvas.setAttribute("role", "main");
  canvas.setAttribute("aria-label", "对话画布");
  {
    const { mountAnnouncer } = await import("./announcer");
    mountAnnouncer(chat);
  }

  // 每帧泵:动图 overlay(Plan 14)+ 复制按钮(Plan 21)+ 文本层/选区(Plan 21)+ Dock(Plan 22)。
  {
    const { pumpEmbedOverlay } = await import("./embed-overlay");
    const { pumpCopyButtons } = await import("./copy-button");
    const { pumpTextLayer, attachSelection } = await import("./text-layer");
    const { pumpDock } = await import("./dock");
    attachSelection(chat);
    if (opts.findBar !== false) {
      const { mountFindBar } = await import("./find-bar");
      mountFindBar(chat);
    }
    const tick = () => {
      pumpEmbedOverlay(chat);
      pumpCopyButtons(chat);
      pumpTextLayer(chat, canvas);
      pumpDock(chat);
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  }

  // 保活:挂 window,避免 chat 被 GC(否则帧循环/监听悬空)。
  (window as unknown as { __chat: unknown }).__chat = chat;

  // 数学字体(Plan 12):异步预载(非阻塞)→ 就绪后 refresh_fonts 让已现公式重栅。
  {
    const { loadMathMsdf } = await import("./msdf");
    const { loadMathFonts } = await import("./math-fonts");
    Promise.all([
      loadMathMsdf(chat).catch((e) => console.error("[math-msdf] load failed", e)),
      loadMathFonts().catch((e) => console.error("[math-fonts] load failed", e)),
    ]).then(() => chat.refresh_fonts());
  }

  return { chat, canvas, wasmModule };
}
