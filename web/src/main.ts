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

  const wasmModule = await init(); // Plan 18:`wasmModule.memory` 读 wasm 线性内存(?bench)

  // 正文用浏览器系统字体栈(零打包,见 layout-bridge SANS/MONO),无需加载自带字体。
  // 固定字形的"文字当图片"走离线 MSDF(0011 §3.5 / TODO K′)。

  const params = new URLSearchParams(location.search);
  const serverUrl = params.get("server") ?? undefined;
  const sessionId = params.get("session") ?? undefined;
  // GitHub Pages 静态演示:CI 用 VITE_DEMO=1 构建 → 无 server 时默认重放 showcase 会话,
  // 并挂一条链接栏(画廊/GitHub),关掉会发不出去的输入框。本地 dev 不受影响。
  const demo = import.meta.env.VITE_DEMO === "1" && !serverUrl;

  // 重放(Plan 5D):case + speed 存 localStorage,在 ?debug 面板里选择(见 debug-panel)。
  // URL 的 ?replay/?speed 仍可临时覆盖(便于分享链接);否则用保存的配置。
  const { loadReplayConfig } = await import("./replay-config");
  const stored = loadReplayConfig();
  // demo 主页默认放介绍片(film 导演驱动,内容用 showcase 文档作导览);?replay=<case> 仍可指定。
  const explicitReplay = params.get("replay");
  const replayName = explicitReplay ?? stored.case ?? (demo ? "showcase" : undefined);
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

  // Plan 18 `?bench=longsession&lines=<tag>&spread=<ms>`:载合成长会话 records(多 turn)直接喂
  // replay,绕过 loadCase(单 part)。`spread` 拉开各 turn 到达间隔 → 浏览器侧能采到增长曲线。
  const benchMode = params.has("bench");
  let benchTarget = 0; // 期望 turn 数(= records 数)→ 采样器据此判「全载完」,防中途卡顿误判 done
  if (benchMode) {
    const tag = params.get("lines") ?? "10k";
    const spread = Number(params.get("spread") ?? "") || 100;
    try {
      const r = await fetch(`${import.meta.env.BASE_URL}replays/longsession-${tag}.json`);
      const recs = (await r.json()) as { t: number; raw: string }[];
      replay = recs.map((x) => ({ t: x.t * spread, raw: x.raw }));
      benchTarget = recs.length;
      console.info(`[bench] longsession-${tag}: ${recs.length} turns, spread=${spread}ms`);
    } catch (e) {
      console.warn(`[bench] 载长会话失败(先跑 node scripts/gen-longsession.mjs):`, e);
    }
  }

  const chat = new ChatCanvas(canvas, { layout, measure, rasterize, serverUrl, sessionId, replay });
  chat.set_math_em(FONT_SIZE); // 数学字号 = 正文字号(含 DPR);显示数学 ×1.3 = H3(Plan 12)
  chat.start();

  // Plan 22 P0:服务端实时流由 TS SSE 客户端接(韧性在 TS:重连/心跳/僵尸/cache-bust),每条
  // data 原文 → chat.push_event(解码在 Rust)。引擎侧用空 QueueConnection,不再在 Rust 内开 SSE。
  if (serverUrl) {
    const { SseClient } = await import("./sse-client");
    new SseClient({
      url: `${serverUrl}/event`,
      onEvent: (raw) => chat.push_event(raw),
    }).start();
  }

  // Plan 13 §5:调试输入框直接 POST `/session/{id}/message` 实时对话(回包走现有 Rust SSE 渲染,
  // 零 wasm/core 改动)。**总是挂载**(立即可见;会话首发时惰性建),serverUrl 缺省用本地 opencode
  // 默认端口 4096。?model= 覆盖模型(同 scripts/chat.mjs)。?noinput 可关掉(纯看渲染时)。
  if (!params.has("noinput") && !demo) {
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
  // 动图 DOM overlay(Plan 14 ⑥)+ 复制按钮(Plan 21 P1)+ 文本层(Plan 21 P2):每帧同步到相机位置。
  {
    const { pumpEmbedOverlay } = await import("./embed-overlay");
    const { pumpCopyButtons } = await import("./copy-button");
    const { pumpTextLayer, attachSelection } = await import("./text-layer");
    const { mountFindBar } = await import("./find-bar");
    const { pumpDock } = await import("./dock");
    attachSelection(chat); // selectionchange → set_selection(节流 rAF)
    mountFindBar(chat); // Plan 21 P3:Cmd+F 跨全历史查找
    const tick = () => {
      pumpEmbedOverlay(chat);
      pumpCopyButtons(chat);
      pumpTextLayer(chat, canvas);
      pumpDock(chat); // Plan 22 P4:权限/反问 Dock(据会话态弹/收)
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
  // MSDF 锐利字形(0015)。**正文 ASCII 默认不挂 MSDF**:正文走系统比例字体,挂等宽/异体
  // MSDF 会让 advance 与排版不符(等宽 atlas → URL 等被拉成等宽、错位)。需要锐利 ASCII 时:
  //   ?asciimsdf 挂比例烘集 ascii-msdf(须用比例字体烘,见 npm run bake:ascii);
  //   ?msdf 挂全集 lxgw-msdf(含 CJK)。未命中 → TinySDF 回退。
  if (params.has("msdf") || params.has("asciimsdf")) {
    const { loadMsdf } = await import("./msdf");
    const b = import.meta.env.BASE_URL;
    const msdfBase = params.has("msdf") ? b + "fonts/lxgw-msdf" : b + "fonts/ascii-msdf";
    loadMsdf(chat, msdfBase).catch((e) => console.warn("[msdf] load skipped (回退 TinySDF)", e));
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
  // Plan 18 ?bench 采样器:每 1s 读 stats + wasm 线性内存,累积 CSV;内容停增长(3 拍稳)→ 导出。
  // 主指标 retained_glyphs / wasm 内存随 turn 增长(before)。CSV 落 window.__benchCSV + 复制剪贴板。
  if (benchMode) {
    // 到达整流 + 显示揭示都不限速 → 内容随 Player 释放即时载满(测稳态规模/内存,非揭示节奏)。
    chat.set_stream_rate(1e9);
    chat.set_reveal_cps(Number.POSITIVE_INFINITY);
    // Plan 19 P1 A/B:?sizefold → sizes 退回每帧 fold(P1 前行为),对照缓存的 fps 收益。
    if (params.has("sizefold")) chat.set_bench_fold_width(true);
    // Plan 19 P2 A/B:?novirt → 关虚拟化(全程 Hot,不释放屏外几何),对照 retained 释放收益。
    if (params.has("novirt")) chat.set_virtualize(false);
    type Row = Record<string, number>;
    const rows: Row[] = [];
    let lastGlyphs = -1;
    let stable = 0;
    const id = setInterval(() => {
      const s = chat.stats();
      const wasmBytes = (wasmModule as unknown as { memory: WebAssembly.Memory }).memory.buffer
        .byteLength;
      const row: Row = {
        turns: s.retainedViews,
        storeChars: s.storeChars,
        retainedGlyphs: s.retainedGlyphs,
        retainedNodes: s.retainedNodes,
        frameGlyphs: s.glyphsVisible,
        fps: Math.round(s.fps),
        frameMsAvg: Number(s.frameMsAvg.toFixed(2)),
        wasmMiB: Number((wasmBytes / 1048576).toFixed(1)),
        // Plan 19 §2 per-phase 归因(上帧瞬时,ms)。
        phAdvance: Number(s.phAdvance.toFixed(2)),
        phBfLayout: Number(s.phBfLayout.toFixed(2)),
        phBfGrid: Number(s.phBfGrid.toFixed(2)),
        phBfEmit: Number(s.phBfEmit.toFixed(2)),
        phBfTotal: Number(s.phBfTotal.toFixed(2)),
        phAdvIngest: Number(s.phAdvIngest.toFixed(2)),
        phAdvRoles: Number(s.phAdvRoles.toFixed(2)),
        phAdvReveal: Number(s.phAdvReveal.toFixed(2)),
        phAdvEnsure: Number(s.phAdvEnsure.toFixed(2)),
        phAdvSchedule: Number(s.phAdvSchedule.toFixed(2)),
        // Plan 19 P2 工作集。
        tierHot: s.tierHot,
        tierWarm: s.tierWarm,
        rebuilds: s.rebuilds,
      };
      rows.push(row);
      console.table([row]);
      // 全部 turn 已材料化(retainedViews 达标)且驻留几何稳定 3 拍 → 导出。retainedViews 达标这一
      // 条防「载入中途卡顿(headless jank)冻住 stats → 误判 done」(否则会在两三个 turn 时早退)。
      const fullyLoaded = benchTarget > 0 ? s.retainedViews >= benchTarget : s.retainedGlyphs > 0;
      if (fullyLoaded && s.retainedGlyphs === lastGlyphs) {
        // 全载后再多采 8 拍稳态(turns 恒定行)→ 给 fps/帧时取中位+P95 足够样本(headless 抖)。
        if (++stable >= 8) {
          const head = Object.keys(rows[0]).join(",");
          const csv = [head, ...rows.map((r) => Object.values(r).join(","))].join("\n");
          (window as unknown as { __benchCSV: string }).__benchCSV = csv;
          console.log(`[bench] done — CSV (window.__benchCSV):\n${csv}`);
          navigator.clipboard?.writeText(csv).catch(() => {});
          clearInterval(id);
        }
      } else {
        stable = 0;
        lastGlyphs = s.retainedGlyphs;
      }
    }, 1000);
  }
  console.info("[harness] ChatCanvas started", {
    mode: serverUrl ? `live: ${serverUrl}` : "synthetic demo",
  });

  // GitHub Pages 演示链接栏(仅 VITE_DEMO 构建):标题 + 画廊 + markdown demo + GitHub。
  if (demo) mountDemoBar();
  // 主页(默认 film,无显式 ?replay)→ 挂介绍片:九幕导演 + 播放器进度条(plan17)。
  if (demo && !explicitReplay) {
    const { mountFilm } = await import("./film/scenes");
    mountFilm(chat);
  }
}

/// 顶部演示链接栏(纯 DOM,零依赖)。base 用 import.meta.env.BASE_URL 适配 Pages 子路径。
function mountDemoBar() {
  const base = import.meta.env.BASE_URL;
  const bar = document.createElement("div");
  bar.style.cssText =
    "position:fixed;top:0;left:0;right:0;z-index:9998;display:flex;gap:14px;align-items:center;" +
    "padding:8px 14px;font:13px/1.4 system-ui,sans-serif;color:#cdd3e0;" +
    "background:linear-gradient(#0d0f17ee,#0d0f1700);pointer-events:none;";
  const link = (label: string, href: string) =>
    `<a href="${href}" style="pointer-events:auto;color:#3df5d0;text-decoration:none;` +
    `border:1px solid #3df5d066;border-radius:6px;padding:3px 9px">${label}</a>`;
  bar.innerHTML =
    `<b style="color:#fff;letter-spacing:.3px">infinite-chat</b>` +
    `<span style="opacity:.6">intro film · SDF / 流式 / 无限画布</span>` +
    `<span style="flex:1"></span>` +
    link("🎨 Icon Gallery", `${base}gallery.html`) +
    link("📄 Markdown demo", `?replay=showcase&speed=0.5`) +
    link("GitHub", "https://github.com/OhBonsai/infinite-chat");
  document.body.appendChild(bar);
}

main().catch((e) => console.error("[harness] 初始化失败", e));
