const p = typeof Intl < "u" && "Segmenter" in Intl ? new Intl.Segmenter(void 0, { granularity: "grapheme" }) : null, f = (r) => p ? [...p.segment(r)].length : r.length;
function m(r, n) {
  if (!p || n <= 0) return Math.min(n, r.length);
  let t = 0;
  for (const o of p.segment(r)) {
    if (o.index >= n) break;
    t += 1;
  }
  return t;
}
function g(r, n, t) {
  let o = 0;
  const i = () => {
    let s;
    try {
      s = JSON.parse(r.visible_text_runs());
    } catch {
      s = [];
    }
    const a = s.find((l) => l.block === n && l.text.includes(t));
    if (a) {
      const l = a.text.indexOf(t), c = a.char0 + m(a.text, l), d = c + f(t);
      r.set_selection(new Uint32Array([n, c, d]));
      return;
    }
    o++ < 600 && requestAnimationFrame(i);
  };
  requestAnimationFrame(i);
}
function x(r) {
  const n = document.createElement("div");
  n.className = "find-bar", n.style.cssText = "position:fixed;top:10px;left:50%;transform:translateX(-50%);z-index:9998;display:none;gap:6px;align-items:center;background:rgba(28,31,40,0.95);border:1px solid rgba(255,255,255,0.15);border-radius:8px;padding:6px 8px;backdrop-filter:blur(6px);font-size:13px;color:#cdd3df";
  const t = document.createElement("input");
  t.type = "text", t.placeholder = "\u67E5\u627E\u5168\u5386\u53F2\u2026", t.className = "find-input", t.style.cssText = "background:rgba(0,0,0,0.3);border:1px solid rgba(255,255,255,0.12);border-radius:5px;color:#e6e9f0;padding:3px 7px;outline:none;width:200px";
  const o = document.createElement("span");
  o.className = "find-count", o.style.cssText = "min-width:54px;text-align:right;opacity:0.75;font-variant-numeric:tabular-nums", n.append(t, o), document.body.appendChild(n);
  let i = [], s = 0;
  const a = () => {
    const e = t.value;
    i = e ? JSON.parse(r.find(e)) : [], s = 0, o.textContent = i.length ? `1/${i.length}` : e ? "0/0" : "", i.length ? l() : r.set_selection(new Uint32Array(0));
  }, l = () => {
    const e = i[s];
    e && (o.textContent = `${s + 1}/${i.length}`, r.scroll_to(e.view), g(r, e.view, t.value));
  }, c = () => {
    n.style.display = "flex", t.focus(), t.select();
  }, d = () => {
    n.style.display = "none", r.set_selection(new Uint32Array(0));
  }, u = (e) => {
    (e.ctrlKey || e.metaKey) && (e.key === "f" || e.key === "F") ? (e.preventDefault(), c()) : e.key === "Escape" && n.style.display !== "none" && d();
  };
  return t.addEventListener("input", a), t.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      if (e.preventDefault(), !i.length) return;
      s = e.shiftKey ? (s - 1 + i.length) % i.length : (s + 1) % i.length, l();
    } else e.key === "Escape" && d();
  }), document.addEventListener("keydown", u), () => {
    document.removeEventListener("keydown", u), n.remove();
  };
}
export {
  x as mountFindBar
};
