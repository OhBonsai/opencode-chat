// pretext-bridge(M7)— StyledSpan 文本 → 平铺位置数组 [x,y,w,h]*N。
//
// Plan1 用 Canvas2D measureText + Intl.Segmenter(grapheme)做单段排版,自洽可跑、零
// 字体打包(BR5,用系统字体)。pretext 是 Plan2 的排版引擎,接口不变(text+maxWidth →
// 每 grapheme 位置),届时只换本文件实现。
//
// 关键契约:**一个 grapheme 一组 [x,y,w,h]**,顺序须与 Rust 侧 unicode-segmentation 的
// grapheme 顺序一致(含换行的零宽占位),app 据此回填 spawn_time。

// HiDPI:所有几何都按设备像素算(canvas 后备缓冲也设成 设备像素,见 main.ts),
// 这样在 Retina 上 1:1 映射物理像素,文字锐利不发虚。
const DPR = typeof window !== "undefined" ? window.devicePixelRatio || 1 : 1;
const BASE_FONT_CSS_PX = 16;

export const FONT_SIZE = Math.round(BASE_FONT_CSS_PX * DPR);
export const LINE_HEIGHT = Math.ceil(FONT_SIZE * 1.4);

// SDF tile 几何(单一来源,glyph-raster 复用;**须与 Rust render::atlas::TILE_PX 一致**)。
// 字形在 tile 内按 FONT_PX 渲染、四周留 SDF_BUFFER;quad 用方形 footprint 采样整 tile,
// 故字形保持自然比例(不被 advance 宽压扁),advance 仅推进笔位。
export const TILE_PX = 64;
export const SDF_BUFFER = 8;
export const FONT_PX = TILE_PX - 2 * SDF_BUFFER;

// 正文用**浏览器系统字体栈**(零打包,小包体)。Canvas2D `ctx.font` = CSS font-family 语法 →
// 浏览器逐字形按优先级 fallback(Latin→中文→emoji→系统默认),无需自带字体。
// 跨端字形会因系统而异(macOS 苹方 / Windows 雅黑 / Linux Noto),接受;要固定字形用离线 MSDF(0011 §3.5)。
const SANS = `ui-sans-serif, system-ui, -apple-system, "Segoe UI", "PingFang SC", "Microsoft YaHei", "Noto Sans CJK SC", sans-serif`;
const MONO = `ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", "Noto Sans Mono CJK SC", monospace`;

/// 排版测量用的 body 字体(layout 不分角色,用 body 度量;粗/斜/code 的精确度量留 Plan2.5)。
export const FONT = `${FONT_SIZE}px ${SANS}`;

/// StyleRole 数值 → CSS 字体(与 Rust content::StyleRole as_u32 对应)。光栅化按此选字体。
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
    case 6: // Heading
      return `bold ${FONT_SIZE}px ${SANS}`;
    case 8: // Quote
      return `italic ${FONT_SIZE}px ${SANS}`;
    default: // Normal / Link / ListMarker
      return `${FONT_SIZE}px ${SANS}`;
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
  measureCtx.font = FONT;
  return measureCtx;
}

/** grapheme 宽度(px,向上取整,至少 1)。 */
export function measureCluster(cluster: string): number {
  return Math.max(1, Math.ceil(ctx().measureText(cluster).width));
}

/** 把整段文本排版成平铺 [x,y,w,h]*N(每 grapheme 一组)。
 *
 * quad = **方形 cell**(= tile footprint 缩放到显示尺寸),`uv` 在 render 侧取整 tile →
 * 字形保持自然比例,**不被 advance 宽压扁**(旧实现 w=advance 导致窄拉丁字横向压缩)。
 * advance 仅用于**推进笔位 + 折行判断**。pos = 笔位 - tile 内留白偏移,使墨迹落在笔位。 */
export function layout(text: string, maxWidth: number): Float32Array {
  const out: number[] = [];
  const s = FONT_SIZE / FONT_PX; // tile px → 显示 px 的均匀缩放
  const cell = TILE_PX * s; // 方形 quad 边长(世界 px)
  const off = SDF_BUFFER * s; // tile 左上 → 字形墨迹左上 的偏移
  let penX = 0;
  let lineY = 0;
  for (const { segment } of segmenter.segment(text)) {
    if (segment === "\n") {
      out.push(penX - off, lineY - off, 0, 0); // 零面积换行占位(保持 grapheme 顺序对齐)
      penX = 0;
      lineY += LINE_HEIGHT;
      continue;
    }
    const adv = measureCluster(segment);
    if (penX + adv > maxWidth && penX > 0) {
      penX = 0;
      lineY += LINE_HEIGHT;
    }
    out.push(penX - off, lineY - off, cell, cell);
    penX += adv;
  }
  return new Float32Array(out);
}
