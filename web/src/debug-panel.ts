// debug-panel(M12)— `?debug` 角落 DOM 浮层(Plan 4C2,DOM 面板 + 不引 egui,0012)。
//
// 轮询 ChatCanvas.stats() 渲染性能数字 + fps sparkline,带暂停/单步;atlas thrash 警告。
// 仅 ?debug 挂载,prod 零成本(不创建)。引擎自绘调试几何(块框/grid,4C3)走 flag,留后续。

import type { ChatCanvas } from "../pkg/infinite_chat_wasm.js";
import { mountTransport } from "./transport";
import { measureCacheStats } from "./layout-bridge";
import {
  REPLAY_CASES,
  loadReplayConfig,
  saveReplayConfig,
} from "./replay-config";

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
  shaderboxActive: number;
  shaderboxPixels: number;
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

const DEBUG_COLLAPSE_KEY = "infinite-chat.debugPanelCollapsed";

export function mountDebugPanel(chat: ChatCanvas, parent: HTMLElement = document.body): void {
  const panel = document.createElement("div");
  panel.style.cssText = [
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
  // 旧的 暂停/单步/reveal 速度 已并入下方 transport 播放器(视频播放器式:播放/调速/拖拽/单步)。
  const btn = (label: string, on: () => void) => {
    const b = document.createElement("button");
    b.textContent = label;
    b.style.cssText =
      "flex:1;font:11px ui-monospace,monospace;color:#cdd6f4;background:#313244;border:0;border-radius:4px;padding:3px;cursor:pointer";
    b.onclick = on;
    return b;
  };

  // 重放选择(Plan 5D):case + speed 存 localStorage,选完即 reload 生效(不进 URL)。
  // 慢放后用 ⏸/⏭ 单帧看过渡;↻ 重跑当前 case。
  const cfg = loadReplayConfig();
  const selCss =
    "flex:1;font:11px ui-monospace,monospace;color:#cdd6f4;background:#313244;border:0;border-radius:4px;padding:3px;cursor:pointer";
  const opt = (sel: HTMLSelectElement, value: string, label: string, on: boolean) => {
    const o = document.createElement("option");
    o.value = value;
    o.textContent = label;
    o.selected = on;
    sel.append(o);
  };

  const caseSel = document.createElement("select");
  caseSel.style.cssText = selCss;
  opt(caseSel, "", "▶ case: (none)", cfg.case == null);
  for (const name of REPLAY_CASES) opt(caseSel, name, name, cfg.case === name);

  const applyReplay = () => {
    saveReplayConfig({ case: caseSel.value || null, speed: cfg.speed || 1 }); // 速度交给下方 player
    location.reload(); // case 在启动时读取,故 reload 生效
  };
  caseSel.onchange = applyReplay;

  // 表格揭示风格(Plan 8E / 0019):与 case 同一行;**实时**生效(经 wasm 揭示调度器,无 reload)。
  // 整表骨架先行 / 行框 / 原始逐字;按下方 player 的 ▶/拖拽观看新风格。
  const REVEAL_STYLE_KEY = "infinite-chat.revealStyle"; // 0=raw 1=rowframe 2=full
  const styleNum = Number(localStorage.getItem(REVEAL_STYLE_KEY) ?? "2");
  const styleSel = document.createElement("select");
  styleSel.style.cssText = selCss;
  opt(styleSel, "2", "表: 整表骨架", styleNum === 2);
  opt(styleSel, "1", "表: 行框", styleNum === 1);
  opt(styleSel, "0", "表: 原始逐字", styleNum === 0);
  styleSel.onchange = () => {
    const v = Number(styleSel.value) || 0;
    localStorage.setItem(REVEAL_STYLE_KEY, String(v));
    chat.set_table_reveal_style(v);
  };
  chat.set_table_reveal_style(styleNum); // 挂载即应用持久化风格

  // case 选择 + 表格风格同一行(播放/调速/拖拽/单步交给下方 transport player)。
  const replayBar = document.createElement("div");
  replayBar.style.cssText = "display:flex;gap:6px;margin-top:6px";
  replayBar.append(caseSel, styleSel);

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

  // 字体 / 字形源切换已移入 style 面板(Render · font)。

  // 视频播放器式调试 transport(播放/暂停/调速/拖拽 scrubber/单步;回放揭示编排)。
  const transportWrap = document.createElement("div");
  mountTransport(chat, transportWrap);

  // 可收起:头部点击折叠正文(状态持久化)。
  const content = document.createElement("div");
  content.append(spark, body, replayBar, transportWrap, geoBar);
  let collapsed = localStorage.getItem(DEBUG_COLLAPSE_KEY) !== "0"; // 默认收起
  const hdr = header("debug");
  hdr.style.cssText += ";display:flex;justify-content:space-between;align-items:center;cursor:pointer";
  const caret = document.createElement("span");
  caret.style.cssText = "color:#7f849c";
  hdr.append(caret);
  const applyCollapse = () => {
    content.style.display = collapsed ? "none" : "";
    caret.textContent = collapsed ? "▸" : "▾";
  };
  hdr.onclick = () => {
    collapsed = !collapsed;
    localStorage.setItem(DEBUG_COLLAPSE_KEY, collapsed ? "1" : "0");
    applyCollapse();
  };
  panel.append(hdr, content);
  applyCollapse();
  parent.appendChild(panel);

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
      row("shaderbox", `${fmt(s.shaderboxActive)} (${fmt(s.shaderboxPixels)}px)`),
      row("atlas", `${fmt(s.atlasUsed)} / ${fmt(s.atlasCap)}`, thrash ? "#f38ba8" : undefined),
      row("evict", fmt(s.atlasEvict)),
      row("src B/T/M", `${fmt(s.srcBitmap)} / ${fmt(s.srcTinySdf)} / ${fmt(s.srcMsdf)}`),
      row("zoom", `${fmt(s.camZoom, 2)}×`),
      measureRow(),
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

/// Plan 13 §7⑦:measure 缓存命中率(Taffy 叶子量尺寸的复用率;高 = 折行/盒树重算近零成本)。
function measureRow(): string {
  const m = measureCacheStats();
  const total = m.hits + m.misses;
  const pct = total > 0 ? (100 * m.hits) / total : 0;
  return row("measure hit", `${pct.toFixed(0)}% (${m.size})`, pct < 50 ? "#f9e2af" : undefined);
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
