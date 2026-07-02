let i = null;
const s = /* @__PURE__ */ new Map(), d = /* @__PURE__ */ new Map();
function x() {
  return i || (i = document.createElement("div"), i.style.cssText = "position:fixed;inset:0;z-index:55;pointer-events:none;overflow:hidden", document.body.appendChild(i)), i;
}
function y(n) {
  const t = document.createElement("button");
  return t.className = "copy-btn", t.type = "button", t.dataset.turnId = String(n), t.textContent = "\u590D\u5236", t.style.cssText = "position:absolute;pointer-events:auto;cursor:pointer;font-size:11px;line-height:1;padding:3px 7px;border-radius:6px;border:1px solid rgba(255,255,255,0.18);background:rgba(40,44,54,0.78);color:#cdd3df;backdrop-filter:blur(4px);opacity:0.55;transition:opacity 0.12s;will-change:transform", t.addEventListener("mouseenter", () => t.style.opacity = "1"), t.addEventListener("mouseleave", () => t.style.opacity = "0.55"), t.addEventListener("click", () => {
    const r = d.get(n) ?? "";
    navigator.clipboard.writeText(r).then(() => p(t, "\u5DF2\u590D\u5236 \u2713"), () => p(t, "\u590D\u5236\u5931\u8D25"));
  }), t;
}
function p(n, t) {
  n.textContent = t, n.style.opacity = "1", window.setTimeout(() => {
    n.textContent = "\u590D\u5236", n.style.opacity = "0.55";
  }, 1100);
}
function f(n) {
  let t;
  try {
    t = JSON.parse(n.visible_turns());
  } catch {
    return;
  }
  const r = x(), a = window.devicePixelRatio || 1, c = /* @__PURE__ */ new Set();
  for (const e of t) {
    c.add(e.id), d.set(e.id, e.text);
    let o = s.get(e.id);
    o || (o = y(e.id), r.appendChild(o), s.set(e.id, o));
    const l = (e.x + e.w) / a, u = e.y / a;
    o.style.left = `${l - 52}px`, o.style.top = `${u + 2}px`;
  }
  for (const [e, o] of s) c.has(e) || (o.remove(), s.delete(e), d.delete(e));
}
export {
  f as pumpCopyButtons
};
