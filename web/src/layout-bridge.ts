// layout-bridge(M7)— 带角色的文本 run → 平铺位置数组 [x,y,w,h]*N(每 grapheme 一组)。
//
// Canvas2D measureText + Intl.Segmenter,自洽可跑、零字体打包(用浏览器系统字体栈)。
// Plan 4A:词边界折行(拉丁不断词)+ CJK 每字可断 + 轻量禁则 + 按角色度量(粗/斜/code/标题分级)。
// 范围 = LTR(中英),不做 BiDi(0001 §2.2)。
//
// 契约:输入 runTexts[]/runRoles[](来自 Rust StyledSpan,与其 grapheme 顺序一致);
// 输出每 grapheme 一组 [x,y,w,h](含换行的零面积占位),app 据此回填 spawn_time。

import { msdfAdvancePx } from "./msdf";

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
    default: // Normal / Link / ListMarker / Rule(零墨)
      return `${FONT_SIZE}px ${SANS}`;
  }
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
): number {
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
  for (let r = 0; r < nRows; r++) {
    const cellLines: Array<Array<[number, number]>> = [];
    let maxLines = 1;
    for (let c = 0; c < region.rows[r].length; c++) {
      const [a, b] = cellRange(r, c);
      const lines = wrapRange(gs, a, b, colW[c]);
      cellLines.push(lines);
      if (lines.length > maxLines) maxLines = lines.length;
    }
    for (let c = 0; c < region.rows[r].length; c++) {
      const align = region.aligns[c] ?? 0;
      const lines = cellLines[c];
      for (let li = 0; li < lines.length; li++) {
        const [la, lb] = lines[li];
        const slack = colW[c] - advSum(la, lb);
        const off = align === 2 ? slack : align === 1 ? slack / 2 : 0;
        let penX = colX[c] + CELL_PAD + Math.max(0, off);
        const ly = y + li * LINE_HEIGHT;
        for (let k = la; k < lb; k++) {
          const g = gs[k];
          out[k * 4] = penX - g.off;
          out[k * 4 + 1] = ly - g.off;
          out[k * 4 + 2] = g.nl ? 0 : g.cell;
          out[k * 4 + 3] = g.nl ? 0 : g.cell;
          if (!g.nl) penX += g.adv;
        }
      }
    }
    y += maxLines * LINE_HEIGHT;
  }
  return y - top;
}

/// 排版:runTexts[i] 文本、runRoles[i] 角色;返回每 grapheme 一组 [x,y,w,h]。
export function layout(
  runTexts: string[],
  runRoles: Uint32Array,
  maxWidth: number,
  tables?: TableRegionJS[],
): Float32Array {
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
    out[idx * 4 + 1] = lineY - g.off;
    out[idx * 4 + 2] = g.cell;
    out[idx * 4 + 3] = g.cell;
    penX += g.adv;
    if (g.lineH > lineH) lineH = g.lineH;
  };
  let i = 0;
  while (i < gs.length) {
    // 5F:命中表格区 → 像素两趟摆位整块,跳过线性流(0014 B)。
    const hit = regionAt.get(i);
    if (hit) {
      lineY += placeTable(gs, out, hit.region, runStart, lineY, maxWidth);
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
  return out;
}
