// glyph-raster(M8)— 单个 grapheme → SDF tile(Plan 3 K)。
//
// 实现 TinySDF(Mapbox,MIT):Canvas2D fillText → alpha 位图 → 欧氏距离变换(EDT,
// Felzenszwalb 1D)→ R8 单通道距离场。0.5≈字形边缘(与 glyph.wgsl 的 smoothstep 对齐)。
// 固定 TILE 尺寸(SDF 缩放无关);返回长度 = TILE*TILE 的 Uint8Array。
//
// 字形画进方形 tile;render 侧用**方形 quad** 采样整 tile(见 layout-bridge.layout),
// 故保持自然比例不压缩。精确 per-glyph tile bbox / MSDF 拐角留后续(0011 §6 触发条件)。

import { fontForRole, TILE_PX as TILE, SDF_BUFFER as BUFFER } from "./layout-bridge";

// TILE / BUFFER 单一来源在 layout-bridge(layout 与光栅须几何一致;须与 Rust atlas::TILE_PX 一致)。
// ① 距离场半径(px):越大梯度越缓 → 细笔画峰值刚过 0.5 → 缩小显示又细又虚。
// 取 ≈ fontPx/6(48/6=8),峰值升到 ~0.69,细笔画饱满锐利。大范围发光/描边需更大范围时再单独留。
const RADIUS = 8;
const INF = 1e20;

let rasterCtx: OffscreenCanvasRenderingContext2D | null = null;
function ctx(): OffscreenCanvasRenderingContext2D {
  if (!rasterCtx) {
    const c = new OffscreenCanvas(TILE, TILE).getContext("2d", { willReadFrequently: true });
    if (!c) throw new Error("无法创建 SDF 光栅上下文");
    rasterCtx = c;
  }
  return rasterCtx;
}

// 1D 距离变换(Felzenszwalb & Huttenlocher)。
function edt1d(grid: Float64Array, offset: number, stride: number, length: number): void {
  const f = new Float64Array(length);
  const v = new Int32Array(length);
  const z = new Float64Array(length + 1);
  for (let q = 0; q < length; q++) f[q] = grid[offset + q * stride];
  v[0] = 0;
  z[0] = -INF;
  z[1] = INF;
  let k = 0;
  for (let q = 1; q < length; q++) {
    let s = (f[q] + q * q - (f[v[k]] + v[k] * v[k])) / (2 * q - 2 * v[k]);
    while (s <= z[k]) {
      k--;
      s = (f[q] + q * q - (f[v[k]] + v[k] * v[k])) / (2 * q - 2 * v[k]);
    }
    k++;
    v[k] = q;
    z[k] = s;
    z[k + 1] = INF;
  }
  k = 0;
  for (let q = 0; q < length; q++) {
    while (z[k + 1] < q) k++;
    const d = q - v[k];
    grid[offset + q * stride] = f[v[k]] + d * d;
  }
}

function edt(grid: Float64Array, w: number, h: number): void {
  for (let x = 0; x < w; x++) edt1d(grid, x, w, h); // 列
  for (let y = 0; y < h; y++) edt1d(grid, y * w, 1, w); // 行
}

// kind:0=位图覆盖率(alpha)/ 1=TinySDF(EDT 距离场)/ 3=RGBA 彩色 emoji(0015 §2.1/§7.2)。
// 统一返回 `TILE²×4` 的 RGBA8(动态图集已升 RGBA8):单色源把值塞 .r(`[v,v,v,255]`,shader 读 .r);
// emoji 直接返回 canvas 彩色像素。
export function rasterize(cluster: string, style: number, kind = 1): Uint8Array {
  const c = ctx();
  c.clearRect(0, 0, TILE, TILE);
  // 字号让字形落进 [BUFFER, TILE-BUFFER]。
  const fontPx = TILE - 2 * BUFFER;
  c.font = fontForRole(style).replace(/^\s*(bold |italic )*\d+px/, `$1${fontPx}px`);
  c.textBaseline = "top";
  c.fillStyle = "#ffffff";
  c.fillText(cluster, BUFFER, BUFFER);

  const img = c.getImageData(0, 0, TILE, TILE).data; // RGBA
  const n = TILE * TILE;

  // kind 3:彩色 emoji —— 直接返回 canvas 的 RGBA(emoji 字体本就彩色,fillStyle 无关)。
  if (kind === 3) {
    return new Uint8Array(img); // 拷贝出 Uint8ClampedArray 的字节
  }

  // 位图模式:覆盖率 = alpha;splat 进 RGBA 的 .r(+ a=255)。
  if (kind === 0) {
    const out = new Uint8Array(n * 4);
    for (let i = 0; i < n; i++) {
      const a = img[i * 4 + 3];
      out[i * 4] = a;
      out[i * 4 + 1] = a;
      out[i * 4 + 2] = a;
      out[i * 4 + 3] = 255;
    }
    return out;
  }

  const outer = new Float64Array(n);
  const inner = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    const a = img[i * 4 + 3] / 255;
    if (a === 1) {
      outer[i] = 0;
      inner[i] = INF;
    } else if (a === 0) {
      outer[i] = INF;
      inner[i] = 0;
    } else {
      const o = Math.max(0, 0.5 - a);
      const inn = Math.max(0, a - 0.5);
      outer[i] = o * o;
      inner[i] = inn * inn;
    }
  }
  edt(outer, TILE, TILE);
  edt(inner, TILE, TILE);

  const out = new Uint8Array(n * 4);
  for (let i = 0; i < n; i++) {
    // 有符号距离:外正内负;归一到 0..1,0.5 = 边缘。
    const d = Math.sqrt(outer[i]) - Math.sqrt(inner[i]);
    const v = Math.max(0, Math.min(255, Math.round(255 * (0.5 - d / (2 * RADIUS)))));
    out[i * 4] = v; // shader 读 .r
    out[i * 4 + 1] = v;
    out[i * 4 + 2] = v;
    out[i * 4 + 3] = 255;
  }
  return out;
}
