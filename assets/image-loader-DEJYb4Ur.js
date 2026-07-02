function d(a) {
  return /<animate(|Transform|Motion)\b|<script\b|@keyframes|animation\s*:/i.test(a);
}
async function f(a) {
  const n = globalThis.ImageDecoder;
  if (!n) return false;
  try {
    const t = new n({ data: await a.arrayBuffer(), type: a.type });
    await t.tracks.ready;
    const e = t.tracks.selectedTrack;
    return !!e && (e.animated === true || (e.frameCount ?? 1) > 1);
  } catch {
    return false;
  }
}
async function g(a) {
  const n = URL.createObjectURL(a);
  try {
    const t = new Image();
    t.src = n, await t.decode();
    const e = Math.max(1, Math.round(t.naturalWidth || 240)), r = Math.max(1, Math.round(t.naturalHeight || 140)), o = new OffscreenCanvas(e, r).getContext("2d");
    if (!o) throw new Error("\u65E0 2d context");
    o.drawImage(t, 0, 0, e, r);
    const c = o.getImageData(0, 0, e, r).data;
    return { rgba: new Uint8Array(c.buffer.slice(0)), w: e, h: r };
  } finally {
    URL.revokeObjectURL(n);
  }
}
async function l(a) {
  const n = await fetch(a);
  if (!n.ok) throw new Error(`HTTP ${n.status}`);
  const t = await n.blob(), e = t.type.includes("svg") || a.toLowerCase().endsWith(".svg");
  let r = false;
  e ? r = d(await t.text()) : r = await f(t);
  const { rgba: s, w: o, h: c } = await g(t);
  return { rgba: s, w: o, h: c, animated: r };
}
const i = /* @__PURE__ */ new Set();
function m(a) {
  let n;
  try {
    n = JSON.parse(a.take_pending_images());
  } catch {
    return;
  }
  for (const { key: t, url: e } of n) i.has(t) || (i.add(t), l(e).then(({ rgba: r, w: s, h: o, animated: c }) => {
    console.info(`[image-loader] \u89E3\u7801\u5B8C\u6210 ${e} ${s}\xD7${o} animated=${c}`), a.upload_image_rgba(t, r, s, o, c);
  }).catch((r) => {
    console.warn(`[image-loader] \u89E3\u7801\u5931\u8D25 ${e}:`, r), a.image_failed(t);
  }).finally(() => i.delete(t)));
}
export {
  m as pumpImageLoads
};
