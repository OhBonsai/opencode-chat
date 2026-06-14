// debug-panel(M12)— `?debug` 角落 DOM 浮层(Plan 4C2,DOM 面板 + 不引 egui,0012)。
//
// 轮询 ChatCanvas.stats() 渲染性能数字 + fps sparkline,带暂停/单步;atlas thrash 警告。
// 仅 ?debug 挂载,prod 零成本(不创建)。引擎自绘调试几何(块框/grid,4C3)走 flag,留后续。

import type { ChatCanvas } from "../pkg/infinite_chat_wasm.js";
import { setFontPreset, currentFontPreset, fontPresets } from "./layout-bridge";

// stats() 在 wasm-bindgen 生成的 .d.ts 里是 any;这里给个本地形状。
interface ChatStats {
  fps: number;
  frameMsAvg: number;
  frameMsMax: number;
  dropped: number;
  glyphsVisible: number;
  glyphsTotal: number;
  blocksVisible: number;
  blocksTotal: number;
  atlasUsed: number;
  atlasCap: number;
  atlasEvict: number;
  camZoom: number;
  paused: number;
  srcBitmap: number;
  srcTinySdf: number;
  srcMsdf: number;
  srcRgba: number;
}

// 字形渲染方案(与 wasm GlyphMode / 0015 §2.6 一致)。
const GLYPH_MODES = ["auto", "bitmap", "tinysdf", "msdf"] as const;

export function mountDebugPanel(chat: ChatCanvas): void {
  const panel = document.createElement("div");
  panel.style.cssText = [
    "position:fixed",
    "top:8px",
    "right:8px",
    "z-index:9999",
    "font:11px/1.5 ui-monospace,Menlo,Consolas,monospace",
    "color:#cdd6f4",
    "background:rgba(17,20,28,.86)",
    "border:1px solid #313244",
    "border-radius:6px",
    "padding:8px 10px",
    "min-width:188px",
    "backdrop-filter:blur(4px)",
    "user-select:none",
  ].join(";");

  const spark = document.createElement("canvas");
  spark.width = 168;
  spark.height = 26;
  spark.style.cssText = "display:block;margin:4px 0;background:#11141c";
  const sctx = spark.getContext("2d");

  const body = document.createElement("div");
  const bar = document.createElement("div");
  bar.style.cssText = "display:flex;gap:6px;margin-top:6px";
  const btn = (label: string, on: () => void) => {
    const b = document.createElement("button");
    b.textContent = label;
    b.style.cssText =
      "flex:1;font:11px ui-monospace,monospace;color:#cdd6f4;background:#313244;border:0;border-radius:4px;padding:3px;cursor:pointer";
    b.onclick = on;
    return b;
  };
  let paused = false;
  const pauseBtn = btn("⏸ pause", () => {
    paused = !paused;
    chat.set_paused(paused);
    pauseBtn.textContent = paused ? "▶ resume" : "⏸ pause";
  });
  bar.append(pauseBtn, btn("⏭ step", () => chat.step()));

  // 自绘调试几何(块 AABB / 视口框,4C3)。
  const geoBar = document.createElement("div");
  geoBar.style.cssText = "display:flex;margin-top:6px";
  let geo = false;
  const geoBtn = btn("▦ geometry", () => {
    geo = !geo;
    chat.set_debug_geometry(geo);
    geoBtn.style.background = geo ? "#585b70" : "#313244";
  });
  geoBar.append(geoBtn);

  // 字体切换(循环预设;切完 bump atlas 代 + 重排,4C)。
  const fontBar = document.createElement("div");
  fontBar.style.cssText = "display:flex;margin-top:6px";
  const presets = fontPresets();
  const fontBtn = btn(`🅰 font: ${currentFontPreset()}`, () => {
    const next = presets[(presets.indexOf(currentFontPreset()) + 1) % presets.length];
    if (setFontPreset(next)) chat.refresh_fonts();
    fontBtn.textContent = `🅰 font: ${currentFontPreset()}`;
  });
  fontBar.append(fontBtn);

  // 字形源方案切换(auto / bitmap / tinysdf / msdf,0015 §2.6)。
  const glyphBar = document.createElement("div");
  glyphBar.style.cssText = "display:flex;margin-top:6px";
  let glyphMode = 0;
  const glyphBtn = btn(`◐ glyph: ${GLYPH_MODES[glyphMode]}`, () => {
    glyphMode = (glyphMode + 1) % GLYPH_MODES.length;
    chat.set_glyph_mode(glyphMode);
    glyphBtn.textContent = `◐ glyph: ${GLYPH_MODES[glyphMode]}`;
  });
  glyphBar.append(glyphBtn);

  panel.append(header("debug"), spark, body, bar, geoBar, fontBar, glyphBar);
  document.body.appendChild(panel);

  const fpsHist: number[] = [];
  const fmt = (n: number, d = 0) => n.toFixed(d);

  const tick = () => {
    const s = chat.stats() as ChatStats;
    fpsHist.push(s.fps);
    if (fpsHist.length > spark.width) fpsHist.shift();
    drawSpark(sctx, spark.width, spark.height, fpsHist);

    const thrash = s.atlasCap > 0 && s.atlasUsed >= s.atlasCap * 0.98 && s.atlasEvict > 0;
    body.innerHTML = [
      row("fps", fmt(s.fps), s.fps < 50 ? "#f38ba8" : "#a6e3a1"),
      row("frame ms", `${fmt(s.frameMsAvg, 1)} / ${fmt(s.frameMsMax, 1)}`),
      row("dropped/s", fmt(s.dropped)),
      row("glyphs", `${fmt(s.glyphsVisible)} / ${fmt(s.glyphsTotal)}`),
      row("blocks", `${fmt(s.blocksVisible)} / ${fmt(s.blocksTotal)}`),
      row("atlas", `${fmt(s.atlasUsed)} / ${fmt(s.atlasCap)}`, thrash ? "#f38ba8" : undefined),
      row("evict", fmt(s.atlasEvict)),
      row("src B/T/M", `${fmt(s.srcBitmap)} / ${fmt(s.srcTinySdf)} / ${fmt(s.srcMsdf)}`),
      row("zoom", `${fmt(s.camZoom, 2)}×`),
      thrash ? `<div style="color:#f38ba8;margin-top:3px">⚠ atlas thrash</div>` : "",
    ].join("");
  };
  setInterval(tick, 500);
}

function header(t: string): HTMLElement {
  const h = document.createElement("div");
  h.textContent = t;
  h.style.cssText = "font-weight:bold;color:#89b4fa;letter-spacing:.5px";
  return h;
}

function row(k: string, v: string, color?: string): string {
  const c = color ? `color:${color}` : "";
  return `<div style="display:flex;justify-content:space-between"><span style="color:#7f849c">${k}</span><span style="${c}">${v}</span></div>`;
}

function drawSpark(
  c: CanvasRenderingContext2D | null,
  w: number,
  h: number,
  data: number[],
): void {
  if (!c) return;
  c.clearRect(0, 0, w, h);
  if (data.length < 2) return;
  const max = Math.max(60, ...data);
  c.strokeStyle = "#89b4fa";
  c.lineWidth = 1;
  c.beginPath();
  data.forEach((v, i) => {
    const x = (i / (w - 1)) * w;
    const y = h - (v / max) * (h - 2) - 1;
    if (i === 0) c.moveTo(x, y);
    else c.lineTo(x, y);
  });
  c.stroke();
  // 60fps 基准线
  const y60 = h - (60 / max) * (h - 2) - 1;
  c.strokeStyle = "rgba(166,227,161,.4)";
  c.beginPath();
  c.moveTo(0, y60);
  c.lineTo(w, y60);
  c.stroke();
}
