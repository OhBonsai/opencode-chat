// style-panel(Plan 6)— Figma 式属性面板(web 层 DOM 浮层)。
//
// 两类样式、两条生效路径(见 ADR/讨论):
//  ① 布局类(table vAlign/hAlign):改 style-config → `chat.refresh_fonts()` 作废排版缓存 → 下帧重排。
//  ② 渲染类(table 颜色/AO/线宽/圆角):改 style-config → `chat.set_table_style(...)` 推进 wasm,
//     `block_decorations` 每帧读 → **下一帧即生效,无需重排/reload**。
// 面板与各分节可收起(持久化)。仅 ?debug 挂载。

import type { ChatCanvas } from "../pkg/infinite_chat_wasm.js";
import {
  currentFontPreset,
  type FontPreset,
  fontPresets,
  setFontPreset,
  setLayoutGlyphMode,
} from "./layout-bridge";
import { loadMsdf, msdfLoaded } from "./msdf";
import {
  getStyleConfig,
  setStyleConfig,
  THEME_TOKENS,
  type HAlign,
  type RGB,
  type RGBA,
  type TableRender,
  type VAlign,
} from "./style-config";

// 字形渲染方案(与 wasm GlyphMode / 0015 §2.6 一致;索引 = set_glyph_mode 入参)。
const GLYPH_MODES = ["auto", "bitmap", "tinysdf", "msdf"] as const;

const COLLAPSE_KEY = "infinite-chat.stylePanelCollapsed";

export function mountStylePanel(chat: ChatCanvas, parent: HTMLElement = document.body): void {
  // ① 布局类:作废排版缓存 → 下帧重排(refresh_fonts 顺带重栅,开销可接受)。
  const relayout = () => chat.refresh_fonts();
  // ② 渲染类:推 wasm,块装饰每帧读 → 下帧即生效。
  const pushRender = () => chat.set_table_style({ ...getStyleConfig().tableRender });
  // ③ 主题类(Plan 26①):token 覆盖 → `chat.set_theme(json)`,装饰 emit 每帧读 → 下帧即生效。
  const pushTheme = () => chat.set_theme(JSON.stringify(getStyleConfig().theme));
  // 挂载即把(可能来自 localStorage 的)渲染样式/主题覆盖各推一次,确保与引擎默认对齐。
  pushRender();
  if (Object.keys(getStyleConfig().theme).length) pushTheme();

  const panel = el(
    "div",
    [
      "font:11px/1.6 ui-monospace,Menlo,Consolas,monospace",
      "color:#cdd6f4",
      "background:rgba(17,20,28,.86)",
      "border:1px solid #313244",
      "border-radius:6px",
      "padding:8px 10px",
      "min-width:230px",
      "backdrop-filter:blur(4px)",
      "user-select:none",
    ].join(";"),
  );

  const body = el("div", "");
  let collapsed = localStorage.getItem(COLLAPSE_KEY) !== "0"; // 默认收起
  const caret = el("span", "color:#7f849c");
  const hdr = el(
    "div",
    "display:flex;justify-content:space-between;align-items:center;cursor:pointer;font-weight:bold;color:#89b4fa;letter-spacing:.5px",
  );
  const ttl = el("span", "");
  ttl.textContent = "style";
  hdr.append(ttl, caret);
  const applyCollapse = () => {
    body.style.display = collapsed ? "none" : "";
    caret.textContent = collapsed ? "▸" : "▾";
  };
  hdr.onclick = () => {
    collapsed = !collapsed;
    localStorage.setItem(COLLAPSE_KEY, collapsed ? "1" : "0");
    applyCollapse();
  };

  // —— Table · 布局(走重排)——
  const setTable = (patch: Partial<{ vAlign: VAlign; hAlign: HAlign }>) => {
    const c = getStyleConfig();
    setStyleConfig({ ...c, table: { ...c.table, ...patch } });
    relayout();
  };
  // —— Table · 渲染(走 set_table_style,实时)——
  const setRender = (patch: Partial<TableRender>) => {
    const c = getStyleConfig();
    setStyleConfig({ ...c, tableRender: { ...c.tableRender, ...patch } });
    pushRender();
  };
  const tr = () => getStyleConfig().tableRender;
  // —— Theme(走 set_theme,实时;Plan 26①)——
  const setThemeToken = (name: string, rgba: RGBA) => {
    const c = getStyleConfig();
    setStyleConfig({ ...c, theme: { ...c.theme, [name]: rgba } });
    pushTheme();
  };
  const themeGet = (name: string, dflt: RGBA): RGBA =>
    (getStyleConfig().theme[name] as RGBA | undefined) ?? dflt;

  // —— Render · 字体 / 字形源(原 debug 面板,移入 style)——
  // 字体切换:换预设 → bump atlas 代 + 重排(0015 §2.5)。
  const setFont = (name: string) => {
    if (setFontPreset(name as FontPreset)) chat.refresh_fonts();
  };
  // 字形源切换(auto/bitmap/tinysdf/msdf,0015 §2.6):量宽跟随渲染源;用 MSDF 的模式懒加载烘集。
  const usesMsdf = (name: string) => name === "auto" || name === "msdf";
  let glyphName = "auto";
  const setGlyph = (name: string) => {
    glyphName = name;
    setLayoutGlyphMode(name); // 量宽跟随渲染源:MSDF 命中字用 baked xadvance
    if (usesMsdf(name) && !msdfLoaded()) {
      loadMsdf(chat)
        .then(() => chat.refresh_fonts()) // 加载完 → baked advance 可用 → 重排
        .catch((e) => console.error("[msdf] load failed", e));
    }
    chat.set_glyph_mode(GLYPH_MODES.indexOf(name as (typeof GLYPH_MODES)[number]));
    chat.refresh_fonts(); // 切源 → advance 变 → 全量重排(0015 §2.5 ⑦)
  };

  body.append(
    section(
      "Theme",
      THEME_TOKENS.map(([name, label, dflt]) =>
        colorField(label, () => themeGet(name, dflt), (rgba) => setThemeToken(name, rgba)),
      ),
    ),
    section("Table · layout", [
      selectField(
        "text align ↕",
        [
          ["top", "top"],
          ["center", "center"],
          ["bottom", "bottom"],
        ],
        () => getStyleConfig().table.vAlign,
        (v) => setTable({ vAlign: v as VAlign }),
      ),
      selectField(
        "text align ↔",
        [
          ["auto", "auto (列对齐)"],
          ["left", "left"],
          ["center", "center"],
          ["right", "right"],
        ],
        () => getStyleConfig().table.hAlign,
        (v) => setTable({ hAlign: v as HAlign }),
      ),
    ]),
    section("Table · render", [
      colorField("line color", () => tr().lineColor, (rgba) => setRender({ lineColor: rgba })),
      colorField("header fill", () => tr().headerFill, (rgba) => setRender({ headerFill: rgba })),
      colorField(
        "AO color",
        () => [...tr().aoColor, 1] as RGBA,
        (rgba) => setRender({ aoColor: [rgba[0], rgba[1], rgba[2]] as RGB }),
      ),
      rangeField("line width", 0, 4, 0.5, () => tr().lineW, (n) => setRender({ lineW: n })),
      rangeField("AO strength", 0, 0.6, 0.02, () => tr().ao, (n) => setRender({ ao: n })),
      rangeField("AO width", 0, 30, 1, () => tr().aoWidth, (n) => setRender({ aoWidth: n })),
      rangeField("corner radius", 0, 16, 1, () => tr().radius, (n) => setRender({ radius: n })),
    ]),
    section("Render · font", [
      selectField(
        "font",
        fontPresets().map((p): [string, string] => [p, p]),
        () => currentFontPreset(),
        setFont,
      ),
      selectField(
        "glyph",
        GLYPH_MODES.map((m): [string, string] => [m, m]),
        () => glyphName,
        setGlyph,
      ),
    ]),
    section("List", [], "—— 待接(标记/缩进/松紧)"),
    section("Div", [], "—— 待接(容器内边距/底色)"),
  );

  panel.append(hdr, body);
  parent.appendChild(panel);
  applyCollapse();
}

// —— DOM 小工具 ——

function el(tag: string, css: string): HTMLElement {
  const e = document.createElement(tag);
  e.style.cssText = css;
  return e;
}

function el2(tag: string, css: string, text: string): HTMLElement {
  const e = el(tag, css);
  e.textContent = text;
  return e;
}

/// 可收起分节:标题 + 字段(空字段 + note = 占位)。
function section(title: string, fields: HTMLElement[], note?: string): HTMLElement {
  const wrap = el("div", "margin-top:8px;border-top:1px solid #262b38;padding-top:6px");
  let open = true;
  const caret = el("span", "color:#7f849c;margin-right:4px");
  caret.textContent = "▾";
  const head = el("div", "cursor:pointer;color:#a6adc8;font-weight:bold");
  head.append(caret, document.createTextNode(title));
  const inner = el("div", "margin-top:4px");
  for (const f of fields) inner.append(f);
  if (note) inner.append(el2("div", "color:#7f849c;font-style:italic", note));
  head.onclick = () => {
    open = !open;
    inner.style.display = open ? "" : "none";
    caret.textContent = open ? "▾" : "▸";
  };
  wrap.append(head, inner);
  return wrap;
}

function fieldRow(label: string): HTMLElement {
  const row = el(
    "div",
    "display:flex;justify-content:space-between;align-items:center;margin:3px 0;gap:8px",
  );
  row.append(el2("span", "color:#7f849c;flex:1", label));
  return row;
}

function selectField(
  label: string,
  options: Array<[value: string, text: string]>,
  get: () => string,
  set: (v: string) => void,
): HTMLElement {
  const row = fieldRow(label);
  const sel = document.createElement("select");
  sel.style.cssText =
    "font:11px ui-monospace,monospace;color:#cdd6f4;background:#313244;border:0;border-radius:4px;padding:2px 4px;cursor:pointer";
  const cur = get();
  for (const [value, text] of options) {
    const o = document.createElement("option");
    o.value = value;
    o.textContent = text;
    o.selected = value === cur;
    sel.append(o);
  }
  sel.onchange = () => set(sel.value);
  row.append(sel);
  return row;
}

/// 滑杆字段:label + range + 数值读出。
function rangeField(
  label: string,
  min: number,
  max: number,
  step: number,
  get: () => number,
  set: (n: number) => void,
): HTMLElement {
  const row = fieldRow(label);
  const r = document.createElement("input");
  r.type = "range";
  r.min = String(min);
  r.max = String(max);
  r.step = String(step);
  r.value = String(get());
  r.style.cssText = "width:90px";
  const out = el2("span", "color:#cdd6f4;width:30px;text-align:right", String(get()));
  r.oninput = () => {
    const n = Number(r.value);
    out.textContent = String(n);
    set(n);
  };
  row.append(r, out);
  return row;
}

/// 颜色字段:color picker(rgb)+ alpha 滑杆(a)。值为 0..1 RGBA。
function colorField(label: string, get: () => RGBA, set: (rgba: RGBA) => void): HTMLElement {
  const row = fieldRow(label);
  const picker = document.createElement("input");
  picker.type = "color";
  picker.value = rgbToHex(get());
  picker.style.cssText = "width:28px;height:18px;padding:0;border:0;background:none;cursor:pointer";
  const alpha = document.createElement("input");
  alpha.type = "range";
  alpha.min = "0";
  alpha.max = "1";
  alpha.step = "0.05";
  alpha.value = String(get()[3]);
  alpha.title = "opacity";
  alpha.style.cssText = "width:64px";
  const emit = () => {
    const [r, g, b] = hexToRgb(picker.value);
    set([r, g, b, Number(alpha.value)]);
  };
  picker.oninput = emit;
  alpha.oninput = emit;
  row.append(picker, alpha);
  return row;
}

function rgbToHex(c: RGBA): string {
  const h = (v: number) =>
    Math.max(0, Math.min(255, Math.round(v * 255)))
      .toString(16)
      .padStart(2, "0");
  return `#${h(c[0])}${h(c[1])}${h(c[2])}`;
}

function hexToRgb(hex: string): RGB {
  const n = parseInt(hex.slice(1), 16);
  return [((n >> 16) & 255) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255];
}
