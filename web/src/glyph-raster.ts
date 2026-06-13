// glyph-raster(M8)— 单个 grapheme → RGBA 位图(OffscreenCanvas 2D fillText)。
//
// 与 pretext-bridge 共用字体/行高,保证 layout 给出的 [w,h] 与位图尺寸一致(否则贴图
// 会拉伸)。文字用白色描边(深色画布上可见),emoji 自带彩色。
//
// 返回 { data: Uint8Array(RGBA), width, height },长度 = width*height*4。

import { FONT, LINE_HEIGHT, measureCluster } from "./pretext-bridge";

export interface GlyphBitmap {
  data: Uint8Array;
  width: number;
  height: number;
}

export function rasterize(cluster: string): GlyphBitmap {
  const width = measureCluster(cluster);
  const height = LINE_HEIGHT;
  const canvas = new OffscreenCanvas(width, height);
  const c = canvas.getContext("2d");
  if (!c) throw new Error("无法创建 2D 光栅化上下文");
  c.font = FONT;
  c.textBaseline = "top";
  c.fillStyle = "#ffffff";
  c.fillText(cluster, 0, 0);
  const img = c.getImageData(0, 0, width, height);
  // 复制出独立的 Uint8Array(共享 buffer 即可,Rust 侧会 to_vec 拷走)。
  return { data: new Uint8Array(img.data.buffer), width, height };
}
