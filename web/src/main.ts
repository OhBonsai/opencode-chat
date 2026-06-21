// main.ts — harness 入口:挂 canvas → init wasm → new ChatCanvas → start。
//
// 用法:
//   npm run dev                         # 合成流演示(Phase C)
//   open http://localhost:5173/?server=http://localhost:4096&session=<id>   # 接真实 opencode(Phase D)

import init, { ChatCanvas } from "../pkg/infinite_chat_wasm.js";
import { layout, measure, FONT_SIZE } from "./layout-bridge";
import { rasterize } from "./glyph-raster";
import { attachCanvasInput } from "./input";

async function main() {
  const canvas = document.getElementById("chat") as HTMLCanvasElement;
  // HiDPI:后备缓冲 = CSS 尺寸 × devicePixelRatio(设备像素),CSS 显示尺寸仍是 100vw/vh,
  // 浏览器据此 1:1 映射物理像素 → 文字锐利。排版/光栅化同样按设备像素(见 layout-bridge)。
  const dpr = window.devicePixelRatio || 1;
  const cssW = canvas.clientWidth || window.innerWidth;
  const cssH = canvas.clientHeight || window.innerHeight;
  canvas.width = Math.round(cssW * dpr);
  canvas.height = Math.round(cssH * dpr);

  await init();

  // 正文用浏览器系统字体栈(零打包,见 layout-bridge SANS/MONO),无需加载自带字体。
  // 固定字形的"文字当图片"走离线 MSDF(0011 §3.5 / TODO K′)。

  const params = new URLSearchParams(location.search);
  const serverUrl = params.get("server") ?? undefined;
  const sessionId = params.get("session") ?? undefined;

  // 重放(Plan 5D):case + speed 存 localStorage,在 ?debug 面板里选择(见 debug-panel)。
  // URL 的 ?replay/?speed 仍可临时覆盖(便于分享链接);否则用保存的配置。
  const { loadReplayConfig } = await import("./replay-config");
  const stored = loadReplayConfig();
  const replayName = params.get("replay") ?? stored.case ?? undefined;
  const speed = Number(params.get("speed") ?? "") || stored.speed || 1;
  // 重放是可选的:加载失败(case 不存在/非 JSON)→ 跳过走实时/合成,绝不连累 init。
  let replay: { t: number; raw: string }[] | undefined;
  if (replayName) {
    try {
      replay = await (await import("./replay")).loadCase(replayName, speed);
    } catch (e) {
      console.warn(`[replay] 加载失败,跳过重放: ${replayName}`, e);
    }
  }

  const chat = new ChatCanvas(canvas, { layout, measure, rasterize, serverUrl, sessionId, replay });
  chat.set_math_em(FONT_SIZE); // 数学字号 = 正文字号(含 DPR);显示数学 ×1.3 = H3(Plan 12)
  chat.start();

  // Plan 13 §5:调试输入框直接 POST `/session/{id}/message` 实时对话(回包走现有 Rust SSE 渲染,
  // 零 wasm/core 改动)。**总是挂载**(立即可见;会话首发时惰性建),serverUrl 缺省用本地 opencode
  // 默认端口 4096。?model= 覆盖模型(同 scripts/chat.mjs)。?noinput 可关掉(纯看渲染时)。
  if (!params.has("noinput")) {
    const { mountChatInput, parseModel } = await import("./chat-input");
    const model = parseModel(params.get("model") ?? "aliyuntokenplan/qwen3.7-max");
    mountChatInput({
      serverUrl: serverUrl ?? "http://localhost:4096",
      sessionId,
      model,
      // 画布只有带 ?server= 时才连 SSE;否则是合成 demo,发送前需重连(chat-input 内处理)。
      canvasLive: !!serverUrl,
      parent: document.body,
    });
  }
  // 画布输入(滚轮/触摸板两指滚动/捏合缩放/拖拽平移)在 web 层挂(Plan 6)。
  attachCanvasInput(canvas, chat);
  // 图片嵌入(Plan 14 ③):每 ~120ms 轮询待解码图 → 浏览器解码/上传(重活在 JS,core 持元数据)。
  {
    const { pumpImageLoads } = await import("./image-loader");
    // Plan 16 §2.7:代码块 copy 图标改走程序化 ShaderBox(不再预载 copy.svg 纹理)。
    setInterval(() => pumpImageLoads(chat), 120);
  }
  // 动图 DOM overlay(Plan 14 ⑥):每帧把动图 `<img>` 同步到相机位置(随 pan/zoom 跟手)。
  {
    const { pumpEmbedOverlay } = await import("./embed-overlay");
    const tick = () => {
      pumpEmbedOverlay(chat);
      requestAnimationFrame(tick);
    };
    requestAnimationFrame(tick);
  }
  // 保活:挂到 window,避免 chat 被 GC 释放(否则帧循环/监听回调会悬空)。
  (window as unknown as { __chat: unknown }).__chat = chat;

  // ?debug:挂调试面板(Plan 4C2)。按需加载,prod 零成本。
  if (params.has("debug")) {
    // 右上角竖排容器:debug 在上、style 在下,收起/展开自动重排(Plan 6)。
    const panels = document.createElement("div");
    panels.style.cssText =
      "position:fixed;top:8px;right:8px;z-index:9999;display:flex;flex-direction:column;gap:8px;align-items:flex-end";
    document.body.appendChild(panels);
    const { mountDebugPanel } = await import("./debug-panel");
    mountDebugPanel(chat, panels);
    // 样式属性面板(Figma 式;web 层调样式,不重编 wasm)。
    const { mountStylePanel } = await import("./style-panel");
    mountStylePanel(chat, panels);
  }
  // ?msdf:预载离线 MSDF 烘集(0015),默认 Auto 模式即命中常用字。非 prod 默认(小包体)。
  if (params.has("msdf")) {
    const { loadMsdf } = await import("./msdf");
    loadMsdf(chat).catch((e) => console.error("[msdf] preload failed", e));
  }
  // 数学字体(Plan 12 / 0013 §8):异步预载(非阻塞)。① **KaTeX MSDF atlas**(相位④)→ 数学字形
  // 任意缩放锐利无锯齿(resolve 对数学角色查合成键 MSDF);② KaTeX woff2 作 MSDF 未命中字的 TinySDF
  // 回退。两者就绪后 refresh_fonts 让已出现公式重栅。
  {
    const { loadMathMsdf } = await import("./msdf");
    const { loadMathFonts } = await import("./math-fonts");
    Promise.all([
      loadMathMsdf(chat).catch((e) => console.error("[math-msdf] load failed", e)),
      loadMathFonts().catch((e) => console.error("[math-fonts] load failed", e)),
    ]).then(() => chat.refresh_fonts());
  }
  // ?verify:开自绘几何标尺(复用 4C3 块/视口框,Plan 5D3),配 ?replay 看流式无跳变。
  // 引擎异步就绪,轮询几次让开关生效后停。
  if (params.has("verify")) {
    let tries = 0;
    const id = setInterval(() => {
      chat.set_debug_geometry(true);
      if (++tries > 20) clearInterval(id);
    }, 200);
  }
  // ?gallery:ShaderBox 画廊(Plan 16)— 视口钉一格栅,逐格一个内置 shader(50 icon +
  // glow_orb + raymarch),一屏肉眼验全盘 shader 上屏。引擎异步就绪 → 同 verify 轮询几次。
  if (params.has("gallery")) {
    let tries = 0;
    const id = setInterval(() => {
      chat.set_shaderbox_gallery(true);
      if (++tries > 20) clearInterval(id);
    }, 200);
  }
  console.info("[harness] ChatCanvas started", {
    mode: serverUrl ? `live: ${serverUrl}` : "synthetic demo",
  });
}

main().catch((e) => console.error("[harness] 初始化失败", e));
