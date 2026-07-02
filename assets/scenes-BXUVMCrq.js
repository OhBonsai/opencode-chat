var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);
class k {
  constructor(e, i, s) {
    __publicField(this, "totalMs");
    __publicField(this, "marks");
    __publicField(this, "clock", 0);
    __publicField(this, "rate", 1);
    __publicField(this, "playing", false);
    __publicField(this, "starts", []);
    __publicField(this, "sceneIdx", -1);
    __publicField(this, "fired", /* @__PURE__ */ new Set());
    __publicField(this, "onUpdate");
    this.scenes = e, this.chat = i, this.teaser = s;
    let o = 0;
    this.marks = e.map((n) => {
      const r = { id: n.id, title: n.title, startMs: o, durationMs: n.durationMs };
      return this.starts.push(o), o += n.durationMs, r;
    }), this.totalMs = o;
  }
  ctx() {
    return { rate: this.rate, teaser: this.teaser, call: (e, ...i) => {
      try {
        const s = this.chat[e];
        typeof s == "function" && s(...i);
      } catch (s) {
        console.warn(`[film] ${e} \u5931\u8D25`, s);
      }
    } };
  }
  play() {
    this.playing = true, this.ctx().call("set_paused", false), this.emit();
  }
  pause() {
    this.playing = false, this.ctx().call("set_paused", true), this.emit();
  }
  toggle() {
    this.playing ? this.pause() : this.play();
  }
  setRate(e) {
    this.rate = e, this.emit();
  }
  seek(e) {
    this.clock = Math.max(0, Math.min(e, this.totalMs));
    const i = this.sceneAt(this.clock);
    this.enterScene(i, true), this.emit();
  }
  tick(e) {
    if (!this.playing) return;
    this.clock += e * this.rate, this.clock >= this.totalMs && (this.clock = this.totalMs, this.playing = false);
    const i = this.sceneAt(this.clock);
    i !== this.sceneIdx && this.enterScene(i, false);
    const s = this.scenes[i];
    if (s == null ? void 0 : s.cues) {
      const o = this.clock - this.starts[i];
      for (let n = 0; n < s.cues.length; n++) {
        const r = `${i}:${n}`;
        o >= s.cues[n].at && !this.fired.has(r) && (this.fired.add(r), s.cues[n].fn(this.ctx()));
      }
    }
    this.emit();
  }
  sceneAt(e) {
    let i = 0;
    for (let s = 0; s < this.starts.length; s++) e >= this.starts[s] && (i = s);
    return i;
  }
  enterScene(e, i) {
    var _a, _b;
    this.sceneIdx = e, this.teaser.hide();
    const s = this.scenes[e];
    for (let o = 0; o < (((_a = s == null ? void 0 : s.cues) == null ? void 0 : _a.length) ?? 0); o++) {
      const n = `${e}:${o}`, r = this.clock - this.starts[e];
      i && s.cues[o].at <= r ? this.fired.add(n) : this.fired.delete(n);
    }
    (_b = s == null ? void 0 : s.enter) == null ? void 0 : _b.call(s, this.ctx());
  }
  emit() {
    var _a;
    (_a = this.onUpdate) == null ? void 0 : _a.call(this, { clock: this.clock, total: this.totalMs, sceneIdx: this.sceneIdx, playing: this.playing, rate: this.rate });
  }
}
const w = (t) => {
  const e = Math.floor(t / 1e3);
  return `${Math.floor(e / 60)}:${String(e % 60).padStart(2, "0")}`;
};
function M(t, e = document.body) {
  const i = document.createElement("div");
  i.style.cssText = "position:fixed;left:0;right:0;bottom:0;z-index:9997;display:flex;align-items:center;gap:14px;padding:12px 18px;background:linear-gradient(#0d0f1700,#0d0f17ee);font:12px/1 'JetBrains Mono',monospace;color:#cdd3e0;user-select:none;";
  const s = document.createElement("button");
  s.style.cssText = "all:unset;cursor:pointer;width:30px;height:30px;border-radius:50%;border:1px solid #3df5d066;color:#3df5d0;text-align:center;line-height:30px;flex:0 0 auto;", s.textContent = "\u25B6";
  const o = document.createElement("div");
  o.style.cssText = "position:relative;flex:1;height:18px;cursor:pointer;display:flex;align-items:center;";
  const n = document.createElement("div");
  n.style.cssText = "position:absolute;left:0;right:0;height:3px;background:#222838;border-radius:2px;";
  const r = document.createElement("div");
  r.style.cssText = "position:absolute;left:0;height:3px;width:0;background:#3df5d0;border-radius:2px;";
  const h = document.createElement("div");
  h.style.cssText = "position:absolute;width:11px;height:11px;border-radius:50%;background:#3df5d0;left:0;transform:translateX(-50%);", o.append(n, r, h);
  for (const a of t.marks) {
    const l = document.createElement("div"), d = a.startMs / t.totalMs * 100;
    l.title = a.title, l.style.cssText = `position:absolute;left:${d}%;width:2px;height:9px;background:#7c8499;opacity:.6;transform:translateX(-50%);`, o.appendChild(l);
  }
  const c = document.createElement("span");
  c.style.cssText = "flex:0 0 auto;opacity:.7;min-width:74px;text-align:right;";
  const f = document.createElement("div");
  f.style.cssText = "display:flex;gap:6px;flex:0 0 auto;";
  const u = {};
  for (const a of [0.5, 1, 2]) {
    const l = document.createElement("button");
    l.style.cssText = "all:unset;cursor:pointer;padding:3px 7px;border-radius:5px;font-size:11px;color:#9aa3b5;border:1px solid #ffffff14;", l.textContent = `${a}\xD7`, l.onclick = () => t.setRate(a), u[a] = l, f.appendChild(l);
  }
  i.append(s, o, c, f), e.appendChild(i), s.onclick = () => t.toggle();
  const g = (a) => {
    const l = o.getBoundingClientRect(), d = Math.max(0, Math.min(1, (a.clientX - l.left) / l.width));
    t.seek(d * t.totalMs);
  };
  let m = false;
  o.addEventListener("mousedown", (a) => {
    m = true, g(a);
  }), window.addEventListener("mousemove", (a) => {
    m && g(a);
  }), window.addEventListener("mouseup", () => {
    m = false;
  }), t.onUpdate = (a) => {
    const l = a.total ? a.clock / a.total : 0;
    r.style.width = `${l * 100}%`, h.style.left = `${l * 100}%`, s.textContent = a.playing ? "\u23F8" : "\u25B6", c.textContent = `${w(a.clock)} / ${w(a.total)}`;
    for (const d of [0.5, 1, 2]) {
      const b = a.rate === d;
      u[d].style.color = b ? "#3df5d0" : "#9aa3b5", u[d].style.borderColor = b ? "#3df5d066" : "#ffffff14";
    }
  };
}
function v(t = document.body) {
  const e = document.createElement("div");
  e.style.cssText = "position:fixed;left:50%;top:46%;transform:translate(-50%,-50%) scale(0.96);z-index:9990;min-width:260px;max-width:360px;padding:18px 20px;border:1px solid #3df5d055;border-radius:12px;background:#1a1040ee;color:#e8eaf2;font:14px/1.5 system-ui,sans-serif;opacity:0;transition:opacity .35s ease,transform .35s cubic-bezier(0.34,1.56,0.64,1);pointer-events:none;", e.innerHTML = `<div style="font:700 10px/1 'JetBrains Mono',monospace;letter-spacing:.24em;color:#3df5d0;margin-bottom:8px">COMING SOON</div><div class="t-title" style="font-size:16px;font-weight:600;margin-bottom:6px"></div><div class="t-body" style="color:#bfeee2;opacity:.85"></div>`, t.appendChild(e);
  const i = e.querySelector(".t-title"), s = e.querySelector(".t-body");
  return { show(o, n) {
    i.textContent = o, s.textContent = n, e.style.opacity = "1", e.style.transform = "translate(-50%,-50%) scale(1)";
  }, hide() {
    e.style.opacity = "0", e.style.transform = "translate(-50%,-50%) scale(0.96)";
  } };
}
const x = (t) => new Promise((e) => setTimeout(e, t));
async function y(t, e, i = 900) {
  const s = window.devicePixelRatio || 1, o = window.innerWidth * s / 2, n = window.innerHeight * s * 0.42, r = 26, h = Math.pow(e, 1 / r);
  for (let c = 0; c < r; c++) t.call("zoom_at", h, o, n), await x(20);
  await x(i);
  for (let c = 0; c < r; c++) t.call("zoom_at", 1 / h, o, n), await x(20);
}
const p = (t, e, i) => {
  t.call("set_reveal_cps", e), t.call("set_reveal_slow", i), t.call("restart_reveal");
}, E = [{ id: "open", title: "\u5F00\u573A", durationMs: 4e3, enter: (t) => p(t, 12, 1) }, { id: "scale-glimpse", title: "\u89C4\u6A21\xB7\u60CA\u9E3F", durationMs: 4e3, cues: [{ at: 400, fn: (t) => void y(t, 0.5, 1400) }] }, { id: "stream", title: "\u6D41\u5F0F", durationMs: 5e3, enter: (t) => p(t, 9, 0.5), cues: [{ at: 2600, fn: (t) => p(t, 46, 1) }] }, { id: "sdf", title: "SDF\xB7\u6570\u5B66", durationMs: 5e3, enter: (t) => p(t, 28, 1), cues: [{ at: 700, fn: (t) => void y(t, 3, 1300) }] }, { id: "rich", title: "\u5BCC\u5185\u5BB9", durationMs: 5e3, enter: (t) => p(t, 24, 1) }, { id: "fx", title: "\u7279\u6548\xB7\u8DEF\u7EBF\u56FE", durationMs: 5e3, enter: (t) => t.call("set_shaderbox_gallery", true), cues: [{ at: 1600, fn: (t) => {
  t.call("set_shaderbox_gallery", false), t.teaser.show("Agent \u53D1\u5149\u8EAB\u4EFD", "assistant \u7684 glow-orb \u8EAB\u4EFD\u73AF \xB7 \u6D41\u5F0F\u65F6\u52A0\u901F\u8109\u51B2(plan16 \xA72.6)");
} }, { at: 3e3, fn: (t) => t.teaser.show("3D \u76F8\u673A \xB7 raymarch SDF", "\u5149\u6805 SDF + raymarch \u6DF7\u5408\u6E32\u67D3,\u6587\u5B57\u8D70\u8FDB\u4E09\u7EF4(\u51B3\u7B56 0024)") }, { at: 4200, fn: (t) => t.teaser.show("\u591A\u5B9E\u4F8B\u540C\u6B65 \xB7 a11y \u955C\u50CF", "\u591A\u6807\u7B7E\u9875\u72B6\u6001\u540C\u6B65(0008)\xB7 canvas \u7684\u65E0\u969C\u788D DOM \u955C\u50CF") }] }, { id: "resilient", title: "\u5BB9\u9519", durationMs: 5e3, enter: (t) => t.teaser.show("\u65AD\u7F51\u4E0D\u614C \xB7 \u5237\u65B0\u4E0D\u4E22\u5386\u53F2", "\u5F31\u7F51\u4E22\u5305:\u5DF2\u4E0A\u5C4F\u5185\u5BB9\u7EB9\u4E1D\u4E0D\u52A8;EventSource \u81EA\u52A8\u91CD\u8FDE + \u5FEB\u7167 resync \u5BF9\u8D26(\u51B3\u7B56 0003)") }, { id: "scale-peak", title: "\u89C4\u6A21\xB7\u5168\u7206\u53D1", durationMs: 4e3, cues: [{ at: 300, fn: (t) => void y(t, 0.32, 1800) }] }, { id: "outro", title: "\u6536\u5C3E \xB7 Chat", durationMs: 3e3, enter: (t) => {
  p(t, 30, 1), t.teaser.show("\u4E00\u6761\u6C38\u4E0D\u7ED3\u675F\u7684\u5BF9\u8BDD", "infinite-chat \xB7 Rust \xB7 WebAssembly \xB7 WebGPU \xB7 React/Vue import");
} }];
function C(t) {
  const e = v(), i = new k(E, t, e);
  M(i);
  let s = performance.now(), o = true;
  const n = (r) => {
    o && (i.tick(r - s), s = r, requestAnimationFrame(n));
  };
  return requestAnimationFrame((r) => {
    s = r, requestAnimationFrame(n);
  }), i.play(), () => {
    o = false;
  };
}
export {
  E as SCENES,
  C as mountFilm
};
