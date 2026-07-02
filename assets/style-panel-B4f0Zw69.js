import { g as f, T as j, f as G, c as W, s as C, a as H, d as I, l as $, e as D } from "./boot-Bfd1wDNf.js";
const M = ["auto", "bitmap", "tinysdf", "msdf"], O = "infinite-chat.stylePanelCollapsed";
function q(t, n = document.body) {
  const l = () => t.refresh_fonts(), c = () => t.set_table_style({ ...f().tableRender }), s = () => t.set_theme(JSON.stringify(f().theme));
  c(), Object.keys(f().theme).length && s();
  const o = u("div", ["font:11px/1.6 ui-monospace,Menlo,Consolas,monospace", "color:#cdd6f4", "background:rgba(17,20,28,.86)", "border:1px solid #313244", "border-radius:6px", "padding:8px 10px", "min-width:230px", "backdrop-filter:blur(4px)", "user-select:none"].join(";")), a = u("div", "");
  let r = localStorage.getItem(O) !== "0";
  const d = u("span", "color:#7f849c"), i = u("div", "display:flex;justify-content:space-between;align-items:center;cursor:pointer;font-weight:bold;color:#89b4fa;letter-spacing:.5px"), w = u("span", "");
  w.textContent = "style", i.append(w, d);
  const E = () => {
    a.style.display = r ? "none" : "", d.textContent = r ? "\u25B8" : "\u25BE";
  };
  i.onclick = () => {
    r = !r, localStorage.setItem(O, r ? "1" : "0"), E();
  };
  const _ = (e) => {
    const p = f();
    C({ ...p, table: { ...p.table, ...e } }), l();
  }, m = (e) => {
    const p = f();
    C({ ...p, tableRender: { ...p.tableRender, ...e } }), c();
  }, g = () => f().tableRender, R = (e, p) => {
    const x = f();
    C({ ...x, theme: { ...x.theme, [e]: p } }), s();
  }, A = (e, p) => f().theme[e] ?? p, F = (e) => {
    H(e) && t.refresh_fonts();
  }, P = (e) => e === "auto" || e === "msdf";
  let k = "auto";
  const L = (e) => {
    k = e, D(e), P(e) && !I() && $(t).then(() => t.refresh_fonts()).catch((p) => console.error("[msdf] load failed", p)), t.set_glyph_mode(M.indexOf(e)), t.refresh_fonts();
  };
  a.append(h("Theme", j.map(([e, p, x]) => v(p, () => A(e, x), (N) => R(e, N)))), h("Table \xB7 layout", [b("text align \u2195", [["top", "top"], ["center", "center"], ["bottom", "bottom"]], () => f().table.vAlign, (e) => _({ vAlign: e })), b("text align \u2194", [["auto", "auto (\u5217\u5BF9\u9F50)"], ["left", "left"], ["center", "center"], ["right", "right"]], () => f().table.hAlign, (e) => _({ hAlign: e }))]), h("Table \xB7 render", [v("line color", () => g().lineColor, (e) => m({ lineColor: e })), v("header fill", () => g().headerFill, (e) => m({ headerFill: e })), v("AO color", () => [...g().aoColor, 1], (e) => m({ aoColor: [e[0], e[1], e[2]] })), y("line width", 0, 4, 0.5, () => g().lineW, (e) => m({ lineW: e })), y("AO strength", 0, 0.6, 0.02, () => g().ao, (e) => m({ ao: e })), y("AO width", 0, 30, 1, () => g().aoWidth, (e) => m({ aoWidth: e })), y("corner radius", 0, 16, 1, () => g().radius, (e) => m({ radius: e }))]), h("Render \xB7 font", [b("font", G().map((e) => [e, e]), () => W(), F), b("glyph", M.map((e) => [e, e]), () => k, L)]), h("List", [], "\u2014\u2014 \u5F85\u63A5(\u6807\u8BB0/\u7F29\u8FDB/\u677E\u7D27)"), h("Div", [], "\u2014\u2014 \u5F85\u63A5(\u5BB9\u5668\u5185\u8FB9\u8DDD/\u5E95\u8272)")), o.append(i, a), n.appendChild(o), E();
}
function u(t, n) {
  const l = document.createElement(t);
  return l.style.cssText = n, l;
}
function S(t, n, l) {
  const c = u(t, n);
  return c.textContent = l, c;
}
function h(t, n, l) {
  const c = u("div", "margin-top:8px;border-top:1px solid #262b38;padding-top:6px");
  let s = true;
  const o = u("span", "color:#7f849c;margin-right:4px");
  o.textContent = "\u25BE";
  const a = u("div", "cursor:pointer;color:#a6adc8;font-weight:bold");
  a.append(o, document.createTextNode(t));
  const r = u("div", "margin-top:4px");
  for (const d of n) r.append(d);
  return l && r.append(S("div", "color:#7f849c;font-style:italic", l)), a.onclick = () => {
    s = !s, r.style.display = s ? "" : "none", o.textContent = s ? "\u25BE" : "\u25B8";
  }, c.append(a, r), c;
}
function T(t) {
  const n = u("div", "display:flex;justify-content:space-between;align-items:center;margin:3px 0;gap:8px");
  return n.append(S("span", "color:#7f849c;flex:1", t)), n;
}
function b(t, n, l, c) {
  const s = T(t), o = document.createElement("select");
  o.style.cssText = "font:11px ui-monospace,monospace;color:#cdd6f4;background:#313244;border:0;border-radius:4px;padding:2px 4px;cursor:pointer";
  const a = l();
  for (const [r, d] of n) {
    const i = document.createElement("option");
    i.value = r, i.textContent = d, i.selected = r === a, o.append(i);
  }
  return o.onchange = () => c(o.value), s.append(o), s;
}
function y(t, n, l, c, s, o) {
  const a = T(t), r = document.createElement("input");
  r.type = "range", r.min = String(n), r.max = String(l), r.step = String(c), r.value = String(s()), r.style.cssText = "width:90px";
  const d = S("span", "color:#cdd6f4;width:30px;text-align:right", String(s()));
  return r.oninput = () => {
    const i = Number(r.value);
    d.textContent = String(i), o(i);
  }, a.append(r, d), a;
}
function v(t, n, l) {
  const c = T(t), s = document.createElement("input");
  s.type = "color", s.value = K(n()), s.style.cssText = "width:28px;height:18px;padding:0;border:0;background:none;cursor:pointer";
  const o = document.createElement("input");
  o.type = "range", o.min = "0", o.max = "1", o.step = "0.05", o.value = String(n()[3]), o.title = "opacity", o.style.cssText = "width:64px";
  const a = () => {
    const [r, d, i] = Y(s.value);
    l([r, d, i, Number(o.value)]);
  };
  return s.oninput = a, o.oninput = a, c.append(s, o), c;
}
function K(t) {
  const n = (l) => Math.max(0, Math.min(255, Math.round(l * 255))).toString(16).padStart(2, "0");
  return `#${n(t[0])}${n(t[1])}${n(t[2])}`;
}
function Y(t) {
  const n = parseInt(t.slice(1), 16);
  return [(n >> 16 & 255) / 255, (n >> 8 & 255) / 255, (n & 255) / 255];
}
export {
  q as mountStylePanel
};
