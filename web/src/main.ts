// main.ts — harness 入口:挂 canvas → init wasm → new ChatCanvas → start。
//
// 用法:
//   npm run dev                         # 合成流演示(Phase C)
//   open http://localhost:5173/?server=http://localhost:4096&session=<id>   # 接真实 opencode(Phase D)

import init, { ChatCanvas } from "../pkg/infinite_chat_wasm.js";
import { layout } from "./pretext-bridge";
import { rasterize } from "./glyph-raster";

async function main() {
  const canvas = document.getElementById("chat") as HTMLCanvasElement;
  // HiDPI:后备缓冲 = CSS 尺寸 × devicePixelRatio(设备像素),CSS 显示尺寸仍是 100vw/vh,
  // 浏览器据此 1:1 映射物理像素 → 文字锐利。排版/光栅化同样按设备像素(见 pretext-bridge)。
  const dpr = window.devicePixelRatio || 1;
  const cssW = canvas.clientWidth || window.innerWidth;
  const cssH = canvas.clientHeight || window.innerHeight;
  canvas.width = Math.round(cssW * dpr);
  canvas.height = Math.round(cssH * dpr);

  await init();

  // 正文用浏览器系统字体栈(零打包,见 pretext-bridge SANS/MONO),无需加载自带字体。
  // 固定字形的"文字当图片"走离线 MSDF(0011 §3.5 / TODO K′)。

  const params = new URLSearchParams(location.search);
  const serverUrl = params.get("server") ?? undefined;
  const sessionId = params.get("session") ?? undefined;

  const chat = new ChatCanvas(canvas, { layout, rasterize, serverUrl, sessionId });
  chat.start();
  // 保活:挂到 window,避免 chat 被 GC 释放(否则帧循环/监听回调会悬空)。
  (window as unknown as { __chat: unknown }).__chat = chat;
  console.info("[harness] ChatCanvas started", {
    mode: serverUrl ? `live: ${serverUrl}` : "synthetic demo",
  });
}

main().catch((e) => console.error("[harness] 初始化失败", e));
