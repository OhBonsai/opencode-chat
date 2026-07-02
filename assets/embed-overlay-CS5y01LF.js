let n = null;
const s = /* @__PURE__ */ new Map();
function c() {
  return n || (n = document.createElement("div"), n.style.cssText = "position:fixed;inset:0;z-index:50;pointer-events:none;overflow:hidden", document.body.appendChild(n)), n;
}
function a(r) {
  let i;
  try {
    i = JSON.parse(r.frame_embeds());
  } catch {
    return;
  }
  const d = c(), o = window.devicePixelRatio || 1, l = /* @__PURE__ */ new Set();
  for (const t of i) {
    l.add(t.key);
    let e = s.get(t.key);
    e || (e = document.createElement("img"), e.src = t.url, e.style.cssText = "position:absolute;object-fit:contain;will-change:transform", d.appendChild(e), s.set(t.key, e)), e.style.left = `${t.x / o}px`, e.style.top = `${t.y / o}px`, e.style.width = `${t.w / o}px`, e.style.height = `${t.h / o}px`;
  }
  for (const [t, e] of s) l.has(t) || (e.remove(), s.delete(t));
}
export {
  a as pumpEmbedOverlay
};
