// image-loader.ts — Plan 14 ③/⑤:图片解码 + 动图检测,把首帧 RGBA 上传给 wasm(→ GPU 纹理)。
//
// 流程:每帧轮询 `chat.take_pending_images()`(JSON `[{key,url}]`)→ fetch/decode → 取首帧 RGBA →
// 判 animated(§2.5)→ `chat.upload_image_rgba(key, rgba, w, h, animated)`;失败 → `chat.image_failed`。
// **重活全在浏览器**(0011 §3.3):core 只持元数据。SVG 与位图同路(浏览器原生栅格,§2.4)。

interface ImageHost {
  take_pending_images(): string;
  upload_image_rgba(
    key: string,
    rgba: Uint8Array,
    w: number,
    h: number,
    animated: boolean,
  ): void;
  image_failed(key: string): void;
}

/** SVG 源是否含动画(SMIL / CSS 关键帧 / 脚本)。启发式;误判成动图也只是走 DOM overlay(§2.5 容忍)。 */
function svgIsAnimated(svg: string): boolean {
  return /<animate(|Transform|Motion)\b|<script\b|@keyframes|animation\s*:/i.test(svg);
}

/** 用 `ImageDecoder`(若有)判位图是否多帧动图(GIF/动 WebP/APNG)。不支持 → false(当静态)。 */
async function bitmapIsAnimated(blob: Blob): Promise<boolean> {
  const Decoder = (globalThis as { ImageDecoder?: unknown }).ImageDecoder as
    | (new (init: { data: ArrayBuffer; type: string }) => {
        tracks: { ready: Promise<void>; selectedTrack?: { animated?: boolean; frameCount?: number } };
        completed?: Promise<void>;
      })
    | undefined;
  if (!Decoder) return false;
  try {
    const dec = new Decoder({ data: await blob.arrayBuffer(), type: blob.type });
    await dec.tracks.ready;
    const t = dec.tracks.selectedTrack;
    return !!t && (t.animated === true || (t.frameCount ?? 1) > 1);
  } catch {
    return false;
  }
}

/** 把 `ImageBitmap` 画到 OffscreenCanvas 取 RGBA 像素(首帧静态纹理源)。 */
function bitmapToRgba(bmp: ImageBitmap): { rgba: Uint8Array; w: number; h: number } {
  const w = bmp.width;
  const h = bmp.height;
  const cv = new OffscreenCanvas(w, h);
  const ctx = cv.getContext("2d");
  if (!ctx) throw new Error("无 2d context");
  ctx.drawImage(bmp, 0, 0);
  const data = ctx.getImageData(0, 0, w, h).data;
  return { rgba: new Uint8Array(data.buffer.slice(0)), w, h };
}

/** 解码一张图(url)→ {rgba, w, h, animated}。SVG 走文本嗅探动画 + Image 栅格;位图走 ImageDecoder 判帧。 */
async function decodeImage(
  url: string,
): Promise<{ rgba: Uint8Array; w: number; h: number; animated: boolean }> {
  const resp = await fetch(url);
  if (!resp.ok) throw new Error(`HTTP ${resp.status}`);
  const blob = await resp.blob();
  const isSvg = blob.type.includes("svg") || url.toLowerCase().endsWith(".svg");
  let animated = false;
  if (isSvg) {
    animated = svgIsAnimated(await blob.text());
  } else {
    animated = await bitmapIsAnimated(blob);
  }
  // 首帧静态(§2.5):createImageBitmap 取一帧(GIF/SVG 同路,浏览器原生栅格)。
  const bmp = await createImageBitmap(blob);
  const { rgba, w, h } = bitmapToRgba(bmp);
  bmp.close();
  return { rgba, w, h, animated };
}

const inFlight = new Set<string>();

/** 处理一轮待解码图片(从 host 领取 → 解码 → 上传/失败)。main 每帧或定时调。 */
export function pumpImageLoads(host: ImageHost): void {
  let pending: { key: string; url: string }[];
  try {
    pending = JSON.parse(host.take_pending_images()) as { key: string; url: string }[];
  } catch {
    return;
  }
  for (const { key, url } of pending) {
    if (inFlight.has(key)) continue;
    inFlight.add(key);
    decodeImage(url)
      .then(({ rgba, w, h, animated }) => host.upload_image_rgba(key, rgba, w, h, animated))
      .catch((e) => {
        console.warn(`[image-loader] 解码失败 ${url}:`, e);
        host.image_failed(key);
      })
      .finally(() => inFlight.delete(key));
  }
}
