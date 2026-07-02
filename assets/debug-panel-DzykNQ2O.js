import { m as N } from "./boot-Bfd1wDNf.js";
import { loadReplayConfig as j, REPLAY_CASES as D, saveReplayConfig as H } from "./replay-config-BGpffVO3.js";
const F = 60, V = [0.25, 0.5, 1, 2, 4];
function W(t, r) {
  t.set_reveal_cps(F), t.set_reveal_slow(1);
  let a = false, s = false, E = 1, l = 0, m = 4e3, x = 0, h = 0, p = -1;
  const g = document.createElement("div");
  g.style.cssText = "display:flex;flex-direction:column;gap:6px;margin-top:6px";
  const P = document.createElement("div");
  P.style.cssText = "display:flex;gap:6px;align-items:center";
  const T = (o) => {
    const f = document.createElement("button");
    return f.textContent = o, f.style.cssText = "flex:0 0 auto;min-width:30px;font:12px ui-monospace,monospace;color:#cdd6f4;background:#313244;border:0;border-radius:4px;padding:3px 8px;cursor:pointer", f;
  }, C = T("\u23EE"), u = T("\u25B6"), A = T("\u23EA"), w = T("\u23E9"), b = document.createElement("select");
  b.style.cssText = "flex:1;font:11px ui-monospace,monospace;color:#cdd6f4;background:#313244;border:0;border-radius:4px;padding:3px;cursor:pointer";
  for (const o of V) {
    const f = document.createElement("option");
    f.value = String(o), f.textContent = `${o}\xD7`, f.selected = o === 1, b.appendChild(f);
  }
  P.append(C, A, u, w, b);
  const d = document.createElement("input");
  d.type = "range", d.min = "0", d.max = String(m), d.value = "0", d.step = "1", d.style.cssText = "width:100%;accent-color:#89b4fa;cursor:pointer";
  const B = document.createElement("div");
  B.style.cssText = "font:11px ui-monospace,monospace;color:#9399b2", g.append(P, d, B), r.appendChild(g);
  const _ = 1e3 / 30, k = (o) => `${(o / 1e3).toFixed(2)}s`, $ = () => {
    B.textContent = `${k(l)} / ${k(m)}  ${s ? "\u25B6" : "\u23F8"} ${E}\xD7`;
  }, M = () => {
    var _a;
    const o = ((_a = t.stats()) == null ? void 0 : _a.glyphsTotal) ?? 0;
    m = Math.max(800, Math.round(o > 0 ? o / F * 1e3 + 800 : 4e3)), d.max = String(m);
  }, y = (o) => {
    l = Math.max(0, Math.min(m, o)), d.value = String(Math.round(l)), $();
  }, v = () => {
    a || (a = true, t.set_paused(true), M());
  }, n = () => {
    if (p < 0) return;
    const o = p;
    p = -1, t.seek_reveal(o), y(o);
  }, L = (o) => {
    if (!s) return;
    const f = Math.min(64, o - x) * E;
    if (x = o, p >= 0 && n(), l >= m) {
      i();
      return;
    }
    t.tick(f), y(l + f), h = requestAnimationFrame(L);
  }, e = () => {
    v(), !s && (s = true, u.textContent = "\u23F8", l >= m && (t.seek_reveal(0), y(0)), x = performance.now(), h = requestAnimationFrame(L));
  };
  function i() {
    s = false, u.textContent = "\u25B6", cancelAnimationFrame(h), $();
  }
  u.onclick = () => s ? i() : e(), C.onclick = () => {
    v(), i(), t.seek_reveal(0), y(0);
  }, w.onclick = () => {
    v(), i(), t.tick(_), y(l + _);
  }, A.onclick = () => {
    v(), i(), y(l - _), t.seek_reveal(l);
  }, b.onchange = () => {
    E = Number(b.value) || 1, $();
  }, d.addEventListener("input", () => {
    v(), i(), p = Number(d.value), requestAnimationFrame(n);
  });
  let S = 0;
  const R = setInterval(() => {
    M(), ++S >= 6 && (clearInterval(R), y(m));
  }, 400);
  $();
}
const I = "infinite-chat.debugPanelCollapsed";
function K(t, r = document.body) {
  const a = document.createElement("div");
  a.style.cssText = ["font:11px/1.5 ui-monospace,Menlo,Consolas,monospace", "color:#cdd6f4", "background:rgba(17,20,28,.86)", "border:1px solid #313244", "border-radius:6px", "padding:8px 10px", "min-width:188px", "backdrop-filter:blur(4px)", "user-select:none"].join(";");
  const s = document.createElement("canvas");
  s.width = 168, s.height = 26, s.style.cssText = "display:block;margin:4px 0;background:#11141c";
  const E = s.getContext("2d"), l = document.createElement("div"), m = (e, i) => {
    const S = document.createElement("button");
    return S.textContent = e, S.style.cssText = "flex:1;font:11px ui-monospace,monospace;color:#cdd6f4;background:#313244;border:0;border-radius:4px;padding:3px;cursor:pointer", S.onclick = i, S;
  }, x = j(), h = "flex:1;font:11px ui-monospace,monospace;color:#cdd6f4;background:#313244;border:0;border-radius:4px;padding:3px;cursor:pointer", p = (e, i, S, R) => {
    const o = document.createElement("option");
    o.value = i, o.textContent = S, o.selected = R, e.append(o);
  }, g = document.createElement("select");
  g.style.cssText = h, p(g, "", "\u25B6 case: (none)", x.case == null);
  for (const e of D) p(g, e, e, x.case === e);
  const P = () => {
    H({ case: g.value || null, speed: x.speed || 1 }), location.reload();
  };
  g.onchange = P;
  const T = "infinite-chat.revealStyle", C = Number(localStorage.getItem(T) ?? "2"), u = document.createElement("select");
  u.style.cssText = h, p(u, "2", "\u8868: \u6574\u8868\u9AA8\u67B6", C === 2), p(u, "1", "\u8868: \u884C\u6846", C === 1), p(u, "0", "\u8868: \u539F\u59CB\u9010\u5B57", C === 0), u.onchange = () => {
    const e = Number(u.value) || 0;
    localStorage.setItem(T, String(e)), t.set_table_reveal_style(e);
  }, t.set_table_reveal_style(C);
  const A = document.createElement("div");
  A.style.cssText = "display:flex;gap:6px;margin-top:6px", A.append(g, u);
  const w = document.createElement("div");
  w.style.cssText = "display:flex;margin-top:6px";
  let b = false;
  const d = m("\u25A6 geometry", () => {
    b = !b, t.set_debug_geometry(b), d.style.background = b ? "#585b70" : "#313244";
  });
  w.append(d);
  const B = document.createElement("div");
  W(t, B);
  const _ = document.createElement("div");
  _.append(s, l, A, B, w);
  let k = localStorage.getItem(I) !== "0";
  const $ = Y("debug");
  $.style.cssText += ";display:flex;justify-content:space-between;align-items:center;cursor:pointer";
  const M = document.createElement("span");
  M.style.cssText = "color:#7f849c", $.append(M);
  const y = () => {
    _.style.display = k ? "none" : "", M.textContent = k ? "\u25B8" : "\u25BE";
  };
  $.onclick = () => {
    k = !k, localStorage.setItem(I, k ? "1" : "0"), y();
  }, a.append($, _), y(), r.appendChild(a);
  const v = [], n = (e, i = 0) => e.toFixed(i);
  setInterval(() => {
    const e = t.stats();
    v.push(e.fps), v.length > s.width && v.shift(), G(E, s.width, s.height, v);
    const i = e.atlasCap > 0 && e.atlasUsed >= e.atlasCap * 0.98 && e.atlasEvict > 0;
    l.innerHTML = [c("fps", n(e.fps), e.fps < 50 ? "#f38ba8" : "#a6e3a1"), c("frame ms", `${n(e.frameMsAvg, 1)} / ${n(e.frameMsMax, 1)}`), c("dropped/s", n(e.dropped)), c("glyphs", `${n(e.glyphsVisible)} / ${n(e.glyphsTotal)}`), c("blocks", `${n(e.blocksVisible)} / ${n(e.blocksTotal)}`), c("shaderbox", `${n(e.shaderboxActive)} (${n(e.shaderboxPixels)}px)`), c("retained", `${n(e.retainedGlyphs)}g / ${n(e.retainedViews)}v / ${n(e.retainedNodes)}n`), c("store", `${n(e.storeChars)} chars`), c("tiers", `${n(e.tierHot)} hot / ${n(e.tierWarm)} warm \xB7 rebuild ${n(e.rebuilds)}`), c("phase ms", `lay ${e.phBfLayout} grid ${e.phBfGrid} emit ${e.phBfEmit} adv ${e.phAdvance}`), c("advance ms", `ing ${e.phAdvIngest} role ${e.phAdvRoles} rev ${e.phAdvReveal} ens ${e.phAdvEnsure} sch ${e.phAdvSchedule}`), c("atlas", `${n(e.atlasUsed)} / ${n(e.atlasCap)}`, i ? "#f38ba8" : void 0), c("evict", n(e.atlasEvict)), c("src B/T/M", `${n(e.srcBitmap)} / ${n(e.srcTinySdf)} / ${n(e.srcMsdf)}`), c("zoom", `${n(e.camZoom, 2)}\xD7`), q(), i ? '<div style="color:#f38ba8;margin-top:3px">\u26A0 atlas thrash</div>' : ""].join("");
  }, 500);
}
function Y(t) {
  const r = document.createElement("div");
  return r.textContent = t, r.style.cssText = "font-weight:bold;color:#89b4fa;letter-spacing:.5px", r;
}
function q() {
  const t = N(), r = t.hits + t.misses, a = r > 0 ? 100 * t.hits / r : 0;
  return c("measure hit", `${a.toFixed(0)}% (${t.size})`, a < 50 ? "#f9e2af" : void 0);
}
function c(t, r, a) {
  const s = a ? `color:${a}` : "";
  return `<div style="display:flex;justify-content:space-between"><span style="color:#7f849c">${t}</span><span style="${s}">${r}</span></div>`;
}
function G(t, r, a, s) {
  if (!t || (t.clearRect(0, 0, r, a), s.length < 2)) return;
  const E = Math.max(60, ...s);
  t.strokeStyle = "#89b4fa", t.lineWidth = 1, t.beginPath(), s.forEach((m, x) => {
    const h = x / (r - 1) * r, p = a - m / E * (a - 2) - 1;
    x === 0 ? t.moveTo(h, p) : t.lineTo(h, p);
  }), t.stroke();
  const l = a - 60 / E * (a - 2) - 1;
  t.strokeStyle = "rgba(166,227,161,.4)", t.beginPath(), t.moveTo(0, l), t.lineTo(r, l), t.stroke();
}
export {
  K as mountDebugPanel
};
