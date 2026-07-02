let l = null;
const m = /* @__PURE__ */ new Map(), w = typeof Intl < "u" && "Segmenter" in Intl ? new Intl.Segmenter(void 0, { granularity: "grapheme" }) : null;
function b(n, t) {
  if (t <= 0) return 0;
  if (!w) return Math.min(t, n.length);
  let e = 0;
  for (const d of w.segment(n)) {
    if (d.index >= t) break;
    e += 1;
  }
  return e;
}
function v(n) {
  if (l) return l;
  const t = document.createElement("style");
  return t.textContent = ".text-layer ::selection{background:transparent}.text-layer ::-moz-selection{background:transparent}", document.head.appendChild(t), l = document.createElement("div"), l.className = "text-layer", l.setAttribute("role", "log"), l.setAttribute("aria-label", "\u5BF9\u8BDD"), l.style.cssText = "position:fixed;inset:0;z-index:52;overflow:hidden;pointer-events:none;color-scheme:only light", l.addEventListener("wheel", (e) => {
    n.dispatchEvent(new WheelEvent("wheel", { deltaX: e.deltaX, deltaY: e.deltaY, ctrlKey: e.ctrlKey, clientX: e.clientX, clientY: e.clientY, bubbles: true, cancelable: true })), e.preventDefault();
  }, { passive: false }), document.body.appendChild(l), l;
}
const h = /* @__PURE__ */ new Map();
function k(n, t) {
  let e = h.get(t);
  return e || (e = document.createElement("div"), e.style.display = "contents", e.setAttribute("role", "article"), e.dataset.ablock = String(t), n.appendChild(e), h.set(t, e)), e;
}
function A(n, t) {
  var _a;
  let e;
  try {
    e = JSON.parse(n.visible_text_runs());
  } catch {
    return;
  }
  const d = v(t), a = window.devicePixelRatio || 1, s = /* @__PURE__ */ new Set(), i = /* @__PURE__ */ new Set();
  for (const r of e) {
    const c = `${r.block}:${r.char0}`;
    s.add(c), i.add(r.block);
    let o = m.get(c);
    o || (o = document.createElement("span"), o.dataset.block = String(r.block), o.dataset.char0 = String(r.char0), o.style.cssText = "position:absolute;white-space:pre;color:transparent;user-select:text;-webkit-user-select:text;pointer-events:auto;cursor:text;margin:0;padding:0;overflow:hidden", k(d, r.block).appendChild(o), m.set(c, o)), o.textContent !== r.text && (o.textContent = r.text), o.style.left = `${r.x / a}px`, o.style.top = `${r.y / a}px`, o.style.width = `${r.w / a}px`, o.style.height = `${r.h / a}px`, o.style.fontSize = `${r.h / a * 0.82}px`, o.style.lineHeight = `${r.h / a}px`;
  }
  for (const [r, c] of m) s.has(r) || (c.remove(), m.delete(r));
  for (const [r, c] of h) i.has(r) || (c.remove(), h.delete(r));
  if (n.visible_turns) try {
    const r = JSON.parse(n.visible_turns()), c = new Map(r.map((u) => [u.id, u.role])), o = ((_a = n.stats) == null ? void 0 : _a.call(n).retainedViews) ?? -1;
    for (const [u, p] of h) {
      const g = c.get(u);
      p.setAttribute("aria-roledescription", g === "user" ? "\u7528\u6237\u6D88\u606F" : "\u52A9\u624B\u6D88\u606F"), p.setAttribute("aria-posinset", String(u + 1)), p.setAttribute("aria-setsize", String(o));
    }
  } catch {
  }
}
function C() {
  const n = window.getSelection();
  if (!n || n.isCollapsed || n.rangeCount === 0) return new Uint32Array(0);
  const t = n.getRangeAt(0), e = /* @__PURE__ */ new Map();
  for (const [, a] of m) {
    if (!t.intersectsNode(a)) continue;
    const s = Number(a.dataset.block), i = Number(a.dataset.char0), r = a.textContent ?? "", c = a.firstChild;
    let o = 0, u = r.length;
    c && t.startContainer === c && (o = t.startOffset), c && t.endContainer === c && (u = t.endOffset);
    const p = i + b(r, o), g = i + b(r, u), f = e.get(s);
    f ? (f.start = Math.min(f.start, p), f.end = Math.max(f.end, g)) : e.set(s, { start: p, end: g });
  }
  const d = [];
  for (const [a, { start: s, end: i }] of e) i > s && d.push(a, s, i);
  return new Uint32Array(d);
}
function S(n) {
  return n.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}
function x(n) {
  return `<div>${n.split(`
`).map((e) => `<div>${S(e) || "<br>"}</div>`).join("")}</div>`;
}
function E(n) {
  const t = navigator.clipboard;
  if (t) try {
    const e = new ClipboardItem({ "text/plain": new Blob([n], { type: "text/plain" }), "text/html": new Blob([x(n)], { type: "text/html" }) });
    t.write([e]).catch(() => void t.writeText(n).catch(() => {
    }));
  } catch {
    t.writeText(n).catch(() => {
    });
  }
}
function y(n) {
  var _a;
  if (!n || n.rangeCount === 0) return false;
  const t = n.anchorNode;
  return !!((_a = t instanceof Element ? t : t == null ? void 0 : t.parentElement) == null ? void 0 : _a.closest(".text-layer"));
}
function L(n) {
  let t = false;
  const e = () => {
    t || (t = true, requestAnimationFrame(() => {
      t = false, y(window.getSelection()) && n.set_selection(C());
    }));
  }, d = (s) => {
    var _a, _b;
    const i = window.getSelection();
    if (!y(i) || !i || i.isCollapsed) return;
    const r = i.toString();
    (_a = s.clipboardData) == null ? void 0 : _a.setData("text/plain", r), (_b = s.clipboardData) == null ? void 0 : _b.setData("text/html", x(r)), s.preventDefault();
  }, a = (s) => {
    if (!(s.ctrlKey || s.metaKey) || s.key !== "c" && s.key !== "C") return;
    const i = window.getSelection();
    !y(i) || !i || i.isCollapsed || (s.preventDefault(), E(i.toString()));
  };
  return document.addEventListener("selectionchange", e), document.addEventListener("copy", d), document.addEventListener("keydown", a), () => {
    document.removeEventListener("selectionchange", e), document.removeEventListener("copy", d), document.removeEventListener("keydown", a);
  };
}
export {
  L as attachSelection,
  A as pumpTextLayer
};
