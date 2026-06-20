// layout-bridge(M7)— 带角色的文本 run → 平铺位置数组 [x,y,w,h]*N(每 grapheme 一组)。
//
// Canvas2D measureText + Intl.Segmenter,自洽可跑、零字体打包(用浏览器系统字体栈)。
// Plan 4A:词边界折行(拉丁不断词)+ CJK 每字可断 + 轻量禁则 + 按角色度量(粗/斜/code/标题分级)。
// 范围 = LTR(中英),不做 BiDi(0001 §2.2)。
//
// 契约:输入 runTexts[]/runRoles[](来自 Rust StyledSpan,与其 grapheme 顺序一致);
// 输出每 grapheme 一组 [x,y,w,h](含换行的零面积占位),app 据此回填 spawn_time。

import { msdfAdvancePx } from "./msdf";
import { getStyleConfig } from "./style-config";

const DPR = typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1;
const BASE_FONT_CSS_PX = 16;

export const FONT_SIZE = Math.round(BASE_FONT_CSS_PX * DPR);
export const LINE_HEIGHT = Math.ceil(FONT_SIZE * 1.4);

// SDF tile 几何(单一来源,glyph-raster 复用;须与 Rust render::atlas::TILE_PX 一致)。
export const TILE_PX = 128; // 64→128:源分辨率 ×2(FONT_PX→112),大字更锐(止血,见 0011 §6/0013)
export const SDF_BUFFER = 8;
export const FONT_PX = TILE_PX - 2 * SDF_BUFFER;

// 正文用浏览器系统字体栈(零打包)。Canvas2D 逐字形按优先级 fallback(Latin→中文→emoji)。
// 可在运行时切换字体族(调试器用,Plan 4C);切换后须 bump atlas 代 + 重排(见 ChatCanvas.refresh_fonts)。
export type FontPreset = "system" | "serif" | "rounded" | "mono";

const PRESETS: Record<FontPreset, { sans: string; mono: string }> = {
  system: {
    sans: `ui-sans-serif, system-ui, -apple-system, "Segoe UI", "PingFang SC", "Microsoft YaHei", "Noto Sans CJK SC", sans-serif`,
    mono: `ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", "Noto Sans Mono CJK SC", monospace`,
  },
  serif: {
    sans: `ui-serif, Georgia, Cambria, "Times New Roman", "Songti SC", "SimSun", "Noto Serif CJK SC", serif`,
    mono: `ui-monospace, SFMono-Regular, Menlo, "Noto Sans Mono CJK SC", monospace`,
  },
  rounded: {
    sans: `"SF Pro Rounded", ui-rounded, "Hiragino Maru Gothic ProN", "Yuanti SC", system-ui, sans-serif`,
    mono: `ui-monospace, SFMono-Regular, Menlo, "Noto Sans Mono CJK SC", monospace`,
  },
  mono: {
    // 全等宽:正文也走等宽栈(终端风)。
    sans: `ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Noto Sans Mono CJK SC", monospace`,
    mono: `ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Noto Sans Mono CJK SC", monospace`,
  },
};

let active: FontPreset = "system";
let SANS = PRESETS.system.sans;
let MONO = PRESETS.system.mono;

/// 切换字体预设(layout 与 raster 同走 `fontForRole`,故一处生效两处)。返回是否变更。
export function setFontPreset(name: FontPreset): boolean {
  const p = PRESETS[name];
  if (!p || name === active) return false;
  active = name;
  SANS = p.sans;
  MONO = p.mono;
  return true;
}

/// 当前预设。
export function currentFontPreset(): FontPreset {
  return active;
}

// 当前 glyph 源模式(与 wasm GlyphMode / debug-panel 一致;0015 §2.5)。
// 决定**量宽来源**:用 MSDF 的模式下,命中字用 LXGW baked xadvance(量宽==渲染源);
// 否则(bitmap/tinysdf/未命中)用 measureText(系统/preset)。切模式后须 refresh_fonts 重排。
let glyphMode = "auto";
/// 调试器切 glyph 源时同步给 layout(0015 §2.5 ⑦:切完须重排)。
export function setLayoutGlyphMode(mode: string): void {
  glyphMode = mode;
}
function usesMsdf(): boolean {
  return glyphMode === "auto" || glyphMode === "msdf";
}

/// 所有可选预设(供调试器枚举)。
export function fontPresets(): FontPreset[] {
  return Object.keys(PRESETS) as FontPreset[];
}

export const FONT = `${FONT_SIZE}px ${SANS}`;

/// StyleRole 数值 → CSS 字体(family + 粗细/斜体,FONT_SIZE 基准)。光栅化与度量按此选字体(4A4)。
export function fontForRole(role: number): string {
  switch (role) {
    case 1: // Bold
      return `bold ${FONT_SIZE}px ${SANS}`;
    case 2: // Italic
      return `italic ${FONT_SIZE}px ${SANS}`;
    case 3: // BoldItalic
      return `bold italic ${FONT_SIZE}px ${SANS}`;
    case 4: // Code
    case 5: // CodeBlock
    case 43: // CodeLineNum(代码行号:等宽 → 与代码列对齐,Plan 15 ②)
    case 44: // CodeKeyword(语法高亮 8 色,research 路 A;全等宽保列对齐)
    case 45: // CodeType
    case 46: // CodeFunc
    case 47: // CodeString
    case 48: // CodeComment
    case 49: // CodeNumber
    case 50: // CodePunct
      return `${FONT_SIZE}px ${MONO}`;
    case 6: // Heading (H1)
    case 10: // Heading2..6
    case 11:
    case 12:
    case 13:
    case 14:
      return `bold ${FONT_SIZE}px ${SANS}`;
    case 8: // Quote
      return `italic ${FONT_SIZE}px ${SANS}`;
    case 16: // AlertLabel(告警标签:加粗)
      return `bold ${FONT_SIZE}px ${SANS}`;
    case 17: // TableCell
    case 18: // TableHeader(同字重等宽 → 表头与表体列对齐;表头靠色/底/线区分,0014 A)
      return `${FONT_SIZE}px ${MONO}`;
    case 19: // TableStrong(表体强调:等宽**加粗** → 保列对齐 + 区分,5E.1 #2)
      return `bold ${FONT_SIZE}px ${MONO}`;
    case 20: // TableEm(表体斜体:等宽斜体 → 保列对齐,5E.1 #2)
      return `italic ${FONT_SIZE}px ${MONO}`;
    case 21: // TableSep(列分隔符:等宽,与列宽一致;render 据其 x 画竖线,5E.1 #5)
      return `${FONT_SIZE}px ${MONO}`;
    // ── 数学字形(Plan 12 / 0013 §8):值 26+ → KaTeX 字族(需 @font-face 加载 KaTeX woff2)。
    //    字号由 glyph-raster 按 FrameGlyph.size 覆盖(RaTeX 给的 em 倍率),故此处 FONT_SIZE 仅占位。
    case 26: // MathMain
      return `${FONT_SIZE}px KaTeX_Main`;
    case 27: // MathBold
      return `bold ${FONT_SIZE}px KaTeX_Main`;
    case 28: // MathItalic
      return `italic ${FONT_SIZE}px KaTeX_Main`;
    case 29: // MathBoldItalic
      return `bold italic ${FONT_SIZE}px KaTeX_Main`;
    case 30: // MathVar(数学变量:KaTeX_Math 斜体)
      return `italic ${FONT_SIZE}px KaTeX_Math`;
    case 31: // MathAms
      return `${FONT_SIZE}px KaTeX_AMS`;
    case 32: // MathSize1
      return `${FONT_SIZE}px KaTeX_Size1`;
    case 33: // MathSize2
      return `${FONT_SIZE}px KaTeX_Size2`;
    case 34: // MathSize3
      return `${FONT_SIZE}px KaTeX_Size3`;
    case 35: // MathSize4
      return `${FONT_SIZE}px KaTeX_Size4`;
    case 36: // MathCal
      return `${FONT_SIZE}px KaTeX_Caligraphic`;
    case 37: // MathFrak
      return `${FONT_SIZE}px KaTeX_Fraktur`;
    case 38: // MathSans
      return `${FONT_SIZE}px KaTeX_SansSerif`;
    case 39: // MathScript
      return `${FONT_SIZE}px KaTeX_Script`;
    case 40: // MathTt
      return `${FONT_SIZE}px KaTeX_Typewriter`;
    default: // Normal / Link / ListMarker / Rule(零墨)
      return `${FONT_SIZE}px ${SANS}`;
  }
}

/// 数学角色值(StyleRole 26+,与 core `crate::math` 一致);glyph-raster / 量宽据此判数学字形。
export function isMathRole(role: number): boolean {
  return role >= 26 && role <= 40;
}

/// 标题分级字号倍率(4A3):H1=2.0 … H6=0.9;非标题 1.0。
function roleScale(role: number): number {
  switch (role) {
    case 6:
      return 2.0; // H1
    case 10:
      return 1.6; // H2
    case 11:
      return 1.3; // H3
    case 12:
      return 1.15; // H4
    case 13:
      return 1.0; // H5
    case 14:
      return 0.9; // H6
    case 24:
      return 0.7; // FootnoteRef(脚注引用:小号,不抬基线)
    case 25:
      return 0.85; // FootnoteDef(脚注定义标记:略小)
    default:
      return 1.0;
  }
}

const segmenter = new Intl.Segmenter(undefined, { granularity: "grapheme" });

let measureCtx: OffscreenCanvasRenderingContext2D | null = null;
function ctx(): OffscreenCanvasRenderingContext2D {
  if (!measureCtx) {
    const c = new OffscreenCanvas(8, 8).getContext("2d");
    if (!c) throw new Error("无法创建 2D 测量上下文");
    measureCtx = c;
  }
  return measureCtx;
}

/// 按角色度量 advance(px)。**逐源一致(0015 §2.5)**:用 MSDF 的模式下,单码点且命中烘集的字
/// 用 LXGW baked xadvance(量宽 == 渲染那个 MSDF 字形的字体,等宽 → 与字重无关);否则
/// (bitmap/tinysdf/未命中/多码点 grapheme)退 `measureText(对应字体)`。标题乘分级倍率。
function advanceFor(cluster: string, role: number): number {
  if (usesMsdf()) {
    const cps = [...cluster];
    if (cps.length === 1) {
      const a = msdfAdvancePx(cps[0].codePointAt(0) ?? 0, FONT_SIZE);
      if (a != null) return a * roleScale(role);
    }
  }
  const c = ctx();
  c.font = fontForRole(role);
  return Math.max(1, c.measureText(cluster).width) * roleScale(role);
}

// CJK / 假名 / 谚文 / 全角:每字可断点。
function isCJK(ch: string): boolean {
  const cp = ch.codePointAt(0) ?? 0;
  return (
    (cp >= 0x4e00 && cp <= 0x9fff) || // CJK 统一
    (cp >= 0x3040 && cp <= 0x30ff) || // 平/片假名
    (cp >= 0xac00 && cp <= 0xd7a3) || // 谚文
    (cp >= 0xff00 && cp <= 0xffef) || // 全角
    (cp >= 0x3000 && cp <= 0x303f) // CJK 标点
  );
}
function isSpace(ch: string): boolean {
  return /\s/.test(ch);
}
// 行首禁则(不该出现在行首)→ 悬挂到上一行末(4A2 轻量)。
const HEAD_BAN = "。，、；：？！）」』】》〉”’%…·.,;:?!)]}";
function isHeadBan(ch: string): boolean {
  return HEAD_BAN.includes(ch);
}

interface G {
  cluster: string;
  role: number;
  adv: number;
  cell: number;
  off: number;
  lineH: number;
  nl: boolean;
  cjk: boolean;
  space: boolean;
  inTable: boolean; // 该 grapheme 属表格行 → 整行不折行(5E.1 #6)
}

/// 表格角色(TableCell/Header/Strong/Em/Sep,见 content StyleRole)→ 整行不可断。
function isTableRole(r: number): boolean {
  return r >= 17 && r <= 21;
}

// ───────── Phase 5F:表格像素两趟(0014 B)─────────
// 契约(plan5 §5F):content 产 TableRegion sidecar —— rows[r][c] = 该格在 spans 数组里的
// [startRun, endRun);aligns 每列对齐(0=Left / 1=Center / 2=Right)。layout 第 4 参收 `tables`。
// 像素实测对齐:列宽 = 各格内容 adv 之和的列内 max → 解决 #7 CJK 错位 / #8 字体跟随 / 任意字体。
export interface TableRegionJS {
  rows: Array<Array<[number, number]>>;
  aligns: number[];
}

const CELL_PAD = Math.round(8 * DPR); // 单元格左右内边距(px)
const GRID_W = Math.max(1, Math.round(DPR)); // 列间网格线预留宽(px)

/// 把 grapheme 区间 [a,b) 按 maxW 折成多行,返回每行 [start,end)。拉丁不断词、CJK 每字可断;
/// 空区间返回单空行。供表格格内受限折行(#2/#6)。
function wrapRange(gs: G[], a: number, b: number, maxW: number): Array<[number, number]> {
  const lines: Array<[number, number]> = [];
  let lineStart = a;
  let penX = 0;
  let i = a;
  while (i < b) {
    if (gs[i].nl) {
      lines.push([lineStart, i]);
      i++;
      lineStart = i;
      penX = 0;
      continue;
    }
    let j = i;
    let wordW = 0;
    if (gs[i].cjk) {
      j = i + 1;
      wordW = gs[i].adv;
    } else {
      while (j < b && !gs[j].cjk && !gs[j].nl) {
        wordW += gs[j].adv;
        j++;
        if (gs[j - 1].space) break;
      }
    }
    if (penX + wordW > maxW && penX > 0) {
      lines.push([lineStart, i]);
      lineStart = i;
      penX = 0;
    }
    penX += wordW;
    i = j;
  }
  if (lineStart < b || lines.length === 0) lines.push([lineStart, b]);
  return lines;
}

/// 把一个表格区按像素两趟摆位,写进 `out`(覆盖该区 grapheme),返回表格总高(px)。
/// `top` = 表格块顶 y(world);`runStart[r]` = run r 的首 grapheme 下标;`maxWidth` = 可用宽
/// (超出则列缩 + 格内折行,#2/#6)。
function placeTable(
  gs: G[],
  out: Float32Array,
  region: TableRegionJS,
  runStart: number[],
  top: number,
  maxWidth: number,
): { height: number; panel: TablePanelGeom } {
  const nRows = region.rows.length;
  const nCols = Math.max(region.aligns.length, ...region.rows.map((r) => r.length));
  const cellRange = (r: number, c: number): [number, number] => {
    const cell = region.rows[r][c];
    return cell ? [runStart[cell[0]], runStart[cell[1]]] : [0, 0];
  };
  const advSum = (a: number, b: number): number => {
    let w = 0;
    for (let k = a; k < b; k++) if (!gs[k].nl) w += gs[k].adv;
    return w;
  };
  // 趟①:自然列宽 = 列内各格 max-content 宽。
  const colW = new Array<number>(nCols).fill(0);
  for (let r = 0; r < nRows; r++)
    for (let c = 0; c < region.rows[r].length; c++) {
      const [a, b] = cellRange(r, c);
      colW[c] = Math.max(colW[c], advSum(a, b));
    }
  // 缩到 maxWidth(#2/#6):超出则按比例缩、floor 到 MINC(塞不下就轻微溢出)。
  const fixed = nCols * CELL_PAD * 2 + Math.max(0, nCols - 1) * GRID_W;
  const avail = maxWidth - fixed;
  const total = colW.reduce((s, w) => s + w, 0);
  if (total > avail && avail > 0) {
    const MINC = Math.round(4 * FONT_SIZE);
    const scaled = colW.map((w) => (w / total) * avail);
    let deficit = 0;
    let flex = 0;
    for (const w of scaled) {
      if (w < MINC) deficit += MINC - w;
      else flex += w - MINC;
    }
    for (let c = 0; c < nCols; c++) {
      if (scaled[c] < MINC) colW[c] = MINC;
      else colW[c] = deficit <= flex && flex > 0 ? scaled[c] - ((scaled[c] - MINC) / flex) * deficit : scaled[c];
    }
  }
  // 列 x(块内相对;render 侧据此画竖线网格,#5)。
  const colX = new Array<number>(nCols).fill(0);
  for (let c = 1; c < nCols; c++) colX[c] = colX[c - 1] + colW[c - 1] + CELL_PAD * 2 + GRID_W;
  // 趟②:逐行——每格在 colW[c] 内折行,行高 = 最多行数 × LINE_HEIGHT。
  let y = top;
  const rowTops: number[] = []; // 各行顶 y(块内相对;rowTops[0]=表顶,无线)
  for (let r = 0; r < nRows; r++) {
    rowTops.push(y);
    const cellLines: Array<Array<[number, number]>> = [];
    let maxLines = 1;
    for (let c = 0; c < region.rows[r].length; c++) {
      const [a, b] = cellRange(r, c);
      const lines = wrapRange(gs, a, b, colW[c]);
      cellLines.push(lines);
      if (lines.length > maxLines) maxLines = lines.length;
    }
    // 样式(style-config,web 层):垂直对齐(行内)+ 水平对齐覆盖(auto=用列对齐)。
    const st = getStyleConfig().table;
    const hOverride =
      st.hAlign === "auto" ? null : st.hAlign === "right" ? 2 : st.hAlign === "center" ? 1 : 0;
    for (let c = 0; c < region.rows[r].length; c++) {
      const align = hOverride ?? (region.aligns[c] ?? 0);
      const lines = cellLines[c];
      // 垂直对齐:本格文字"墨迹高度"在行带(maxLines×行高)内上/中/下放置。
      // 墨迹高 = 末行墨迹底 - 首行墨迹顶 ≈ (行数-1)×行高 + 单行墨迹(≈FONT_SIZE);
      // 行带 = maxLines×行高。二者之差 = 可用 slack(单行行也有 = 行距留白 LINE_HEIGHT-FONT_SIZE),
      // 故单行单元格也能上/中/下,不再恒等(修"vertical align 失效")。
      const contentH = (lines.length - 1) * LINE_HEIGHT + FONT_SIZE;
      const vSlack = Math.max(0, maxLines * LINE_HEIGHT - contentH);
      const vOff = st.vAlign === "top" ? 0 : st.vAlign === "bottom" ? vSlack : vSlack / 2;
      for (let li = 0; li < lines.length; li++) {
        const [la, lb] = lines[li];
        const slack = colW[c] - advSum(la, lb);
        const off = align === 2 ? slack : align === 1 ? slack / 2 : 0;
        let penX = colX[c] + CELL_PAD + Math.max(0, off);
        const ly = y + vOff + li * LINE_HEIGHT;
        for (let k = la; k < lb; k++) {
          const g = gs[k];
          out[k * 4] = penX - g.off;
          // 行内垂直居中(同 place):cell 在 LINE_HEIGHT 里居中,叠加格内 vAlign(vOff)。
          out[k * 4 + 1] = ly + (LINE_HEIGHT - g.cell) * 0.5 - g.off;
          out[k * 4 + 2] = g.nl ? 0 : g.cell;
          out[k * 4 + 3] = g.nl ? 0 : g.cell;
          if (!g.nl) penX += g.adv;
        }
      }
    }
    y += maxLines * LINE_HEIGHT;
  }
  // 整表面板几何(块内相对 px,0018 #5):盒子 + 内部竖/横网格线 + 表头底 → render 逐表画一个 SDF 面板。
  const cols: number[] = []; // 内部竖线 x(gap 中心)
  for (let c = 1; c < nCols; c++) cols.push(colX[c] - GRID_W * 0.5);
  const rows = rowTops.slice(1); // 内部横线 y(各数据行顶;首行=表顶不画)
  const boxW = colX[nCols - 1] + colW[nCols - 1] + 2 * CELL_PAD;
  const headerBottom = nRows >= 2 ? rowTops[1] : top; // 无数据行 → headerBottom=top(无表头底)
  const panel: TablePanelGeom = { x: 0, y: top, w: boxW, h: y - top, headerBottom, cols, rows };
  return { height: y - top, panel };
}

/// 排版:runTexts[i] 文本、runRoles[i] 角色;返回每 grapheme 一组 [x,y,w,h]。
/// 一个表格的面板几何(块内相对 px,0018 #5):盒子 + 内部竖/横网格线 + 表头底。
interface TablePanelGeom {
  x: number;
  y: number;
  w: number;
  h: number;
  headerBottom: number;
  cols: number[];
  rows: number[];
}

/// layout 返回:纯位置 `Float32Array`,或带各表格面板几何 `{ positions, tables }`(0018 #5)。
/// `tables` 扁平编码(每表连续):`[x, y, w, h, headerBottom, nCols, nRows, cols…, rows…]`,
/// 与 wasm `decode_table_panels` 对应。
export type LayoutOut = Float32Array | { positions: Float32Array; tables: Float32Array };

/// 把表格面板几何编码成扁平 `Float32Array`(见 `LayoutOut`)。
function encodeTablePanels(panels: TablePanelGeom[]): Float32Array {
  const flat: number[] = [];
  for (const p of panels) {
    flat.push(p.x, p.y, p.w, p.h, p.headerBottom, p.cols.length, p.rows.length, ...p.cols, ...p.rows);
  }
  return new Float32Array(flat);
}

export function layout(
  runTexts: string[],
  runRoles: Uint32Array,
  maxWidth: number,
  tables?: TableRegionJS[],
): LayoutOut {
  const S = FONT_SIZE / FONT_PX; // tile px → 显示 px
  // 1) 展平成带度量的 grapheme 列表(与 Rust grapheme 顺序一致)。`runStart[r]` = run r 的首
  //    grapheme 下标(5F 表格 sidecar 用 run 区间定位 cell)。
  const gs: G[] = [];
  const runStart: number[] = [];
  for (let r = 0; r < runTexts.length; r++) {
    runStart.push(gs.length);
    const role = runRoles[r] ?? 0;
    const scale = roleScale(role);
    for (const { segment } of segmenter.segment(runTexts[r])) {
      const nl = segment === "\n";
      gs.push({
        cluster: segment,
        role,
        adv: nl ? 0 : advanceFor(segment, role),
        cell: TILE_PX * S * scale,
        off: SDF_BUFFER * S * scale,
        lineH: Math.ceil(LINE_HEIGHT * scale),
        nl,
        cjk: !nl && isCJK(segment),
        space: !nl && isSpace(segment),
        inTable: false,
      });
    }
  }

  runStart.push(gs.length); // sentinel:末 run 之后(cell 区间 [s,e) 用 runStart[e])

  // 1.4) 5F 表格区索引(0014 B 像素两趟):按表格首 grapheme 下标登记 → 主循环命中即两趟摆位,
  //      跳过线性流。content 未供 `tables`(0014 A 路径)时为空,行为不变。
  const regionAt = new Map<number, { region: TableRegionJS; gEnd: number }>();
  if (tables) {
    for (const region of tables) {
      if (!region.rows.length || !region.rows[0].length) continue;
      const gStart = runStart[region.rows[0][0][0]];
      let gEnd = gStart;
      for (const row of region.rows) for (const [, e] of row) gEnd = Math.max(gEnd, runStart[e]);
      regionAt.set(gStart, { region, gEnd });
    }
  }

  // 1.5) 表格行标记(5E.1 #6):一行含任意表格角色 → 整行 inTable(后续不折行)。
  {
    let start = 0;
    for (let k = 0; k <= gs.length; k++) {
      if (k === gs.length || gs[k].nl) {
        let isTable = false;
        for (let m = start; m < k; m++) {
          if (isTableRole(gs[m].role)) {
            isTable = true;
            break;
          }
        }
        if (isTable) for (let m = start; m < k; m++) gs[m].inTable = true;
        start = k + 1;
      }
    }
  }

  // 2) 词级折行(拉丁不断词,CJK 每字可断,轻量禁则;表格行整行不断)。
  const out = new Float32Array(gs.length * 4);
  let penX = 0;
  let lineY = 0;
  let lineH = LINE_HEIGHT;
  const place = (g: G, idx: number) => {
    out[idx * 4] = penX - g.off;
    // 行内垂直居中:glyph cell(≈1.14×字号)在行高(LINE_HEIGHT≈1.4×)里居中,不再贴顶
    //(否则行距全留下方,代码块/段落看着偏上)。`(lineH-cell)/2` 把半个行距留到字上方。
    out[idx * 4 + 1] = lineY + (g.lineH - g.cell) * 0.5 - g.off;
    out[idx * 4 + 2] = g.cell;
    out[idx * 4 + 3] = g.cell;
    penX += g.adv;
    if (g.lineH > lineH) lineH = g.lineH;
  };
  const tablePanels: TablePanelGeom[] = []; // 每个表格一份面板几何(块内相对)→ 回传画 #5 网格(0018)
  let i = 0;
  while (i < gs.length) {
    // 5F:命中表格区 → 像素两趟摆位整块,跳过线性流(0014 B)。
    const hit = regionAt.get(i);
    if (hit) {
      const pt = placeTable(gs, out, hit.region, runStart, lineY, maxWidth);
      lineY += pt.height;
      tablePanels.push(pt.panel); // 每表一份(同块多表不合并,0018 #5)
      penX = 0;
      lineH = LINE_HEIGHT;
      i = hit.gEnd;
      continue;
    }
    const g = gs[i];
    if (g.nl) {
      out[i * 4] = penX - g.off;
      out[i * 4 + 1] = lineY - g.off;
      out[i * 4 + 2] = 0;
      out[i * 4 + 3] = 0;
      penX = 0;
      lineY += lineH;
      lineH = LINE_HEIGHT;
      i++;
      continue;
    }
    // 取一个"词":表格行整行不可断;CJK 单字;否则累加到空格(含)或下一个 CJK / 换行。
    let j = i;
    let wordW = 0;
    if (g.inTable) {
      // 表格行(5E.1 #6):整行作一个不可断"词" → 超宽溢到画布(可横向 pan),不拦腰折断。
      while (j < gs.length && !gs[j].nl) {
        wordW += gs[j].adv;
        j++;
      }
    } else if (g.cjk) {
      j = i + 1;
      wordW = g.adv;
    } else {
      while (j < gs.length && !gs[j].nl && !gs[j].cjk) {
        wordW += gs[j].adv;
        j++;
        if (gs[j - 1].space) break; // 词以空格收尾
      }
    }
    // 折行:词溢出且不在行首;行首禁则字则悬挂(不折)。
    if (penX + wordW > maxWidth && penX > 0 && !isHeadBan(g.cluster)) {
      penX = 0;
      lineY += lineH;
      lineH = LINE_HEIGHT;
    }
    for (let k = i; k < j; k++) place(gs[k], k);
    i = j;
  }
  // 有表格 → 回传 { positions, tables }(render 逐表画 #5 网格,0018);否则纯 Float32Array(向后兼容)。
  if (tablePanels.length > 0) {
    return { positions: out, tables: encodeTablePanels(tablePanels) };
  }
  return out;
}

/// measure 缓存(Plan 13 §4.2):key = 内容 + 角色 + 可用宽 → [w, h]。Taffy 排版期对同一叶子可能多次
/// measure(flex 解算),measureText 虽微秒级,命中缓存后近零。append-only 流下 key 自然稳定复用。
const measureCache = new Map<string, Float32Array>();
const MEASURE_CACHE_CAP = 4096; // 上限:超出清空(无限会话防泄漏;命中率基准用,§7⑦)
let measureHits = 0;
let measureMisses = 0;

/// measure 趟(Plan 13 §4.2):只量「这批 run 在可用宽 `availW` 下的目标尺寸」,返回 `[w, h]`。
/// **复用 `layout` 的同一套折行**(measure / layout 两趟必须一致)→ 从其 positions 取(最右墨边, 块高);
/// 结果按 (内容+角色+宽) 缓存。Taffy 叶子回调(wasm `measure_fn`)。
export function measure(runTexts: string[], runRoles: Uint32Array, availW: number): Float32Array {
  if (runTexts.length === 0) return new Float32Array([0, 0]);
  const key = `${runTexts.join("")}${Array.from(runRoles).join(",")}@${Math.round(availW)}`;
  const hit = measureCache.get(key);
  if (hit) {
    measureHits++;
    return hit;
  }
  measureMisses++;
  const ret = layout(runTexts, runRoles, availW);
  const positions = ret instanceof Float32Array ? ret : ret.positions;
  let maxX = 0;
  let maxY = 0;
  for (let k = 0; k + 3 < positions.length; k += 4) {
    const w = positions[k + 2];
    if (w > 0) maxX = Math.max(maxX, positions[k] + w);
    maxY = Math.max(maxY, positions[k + 1] + positions[k + 3]);
  }
  const size = new Float32Array([Math.min(maxX, availW), maxY]);
  if (measureCache.size >= MEASURE_CACHE_CAP) measureCache.clear();
  measureCache.set(key, size);
  return size;
}

/// measure 缓存命中率(`?debug` perf 行用,§7⑦ 基准入册)。
export function measureCacheStats(): { hits: number; misses: number; size: number } {
  return { hits: measureHits, misses: measureMisses, size: measureCache.size };
}
