var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);
import { b as E } from "./boot-Bfd1wDNf.js";
import { mountScriptedInput as T } from "./chat-input-CDyUGpCJ.js";
const w = ["allow", "deny", "answer"];
function m(e) {
  return typeof e == "object" && e !== null && !Array.isArray(e);
}
function N(e) {
  if (!m(e)) return { ok: false, error: "\u5267\u672C\u5FC5\u987B\u662F\u5BF9\u8C61", index: -1 };
  const t = m(e.meta) ? e.meta : {}, o = e.track;
  if (!Array.isArray(o)) return { ok: false, error: "\u7F3A track \u6570\u7EC4", index: -1 };
  const a = [];
  for (let r = 0; r < o.length; r++) {
    const c = o[r];
    if (!m(c)) return { ok: false, error: "\u6307\u4EE4\u5FC5\u987B\u662F\u5BF9\u8C61", index: r };
    if (typeof c.dt != "number" || !Number.isFinite(c.dt) || c.dt < 0) return { ok: false, error: "dt \u5FC5\u987B\u662F \u22650 \u7684\u6709\u9650\u6570", index: r };
    const d = ["user", "event", "dock"].filter((n) => c[n] !== void 0);
    if (d.length !== 1) return { ok: false, error: `\u6307\u4EE4\u987B\u6070\u542B user|event|dock \u4E4B\u4E00(\u73B0 ${d.length} \u4E2A)`, index: r };
    if (c.user !== void 0) {
      const n = c.user;
      if (!m(n) || typeof n.text != "string") return { ok: false, error: "user.text \u5FC5\u987B\u662F\u5B57\u7B26\u4E32", index: r };
      if (n.cps !== void 0 && (typeof n.cps != "number" || n.cps <= 0)) return { ok: false, error: "user.cps \u5FC5\u987B\u662F\u6B63\u6570", index: r };
    }
    if (c.event !== void 0) {
      const n = c.event;
      if (!m(n) || typeof n.type != "string" || n.type.length === 0) return { ok: false, error: "event.type \u5FC5\u987B\u662F\u975E\u7A7A\u5B57\u7B26\u4E32", index: r };
      if (n.properties !== void 0 && !m(n.properties)) return { ok: false, error: "event.properties \u5FC5\u987B\u662F\u5BF9\u8C61", index: r };
    }
    if (c.dock !== void 0 && !w.includes(c.dock)) return { ok: false, error: `dock \u5FC5\u987B\u662F ${w.join("/")} \u4E4B\u4E00`, index: r };
    a.push(c);
  }
  return { ok: true, script: { meta: t, track: a } };
}
function C(e, t = 1) {
  const o = t > 0 ? t : 1;
  let a = 0;
  return e.map((r) => (a += r.dt / o, { ...r, at: a }));
}
function _(e) {
  return e.length ? e[e.length - 1].at : 0;
}
class A {
  constructor(t, o, a = {}) {
    __publicField(this, "timeline");
    __publicField(this, "total");
    __publicField(this, "driver");
    __publicField(this, "virtual", 0);
    __publicField(this, "cursor", 0);
    __publicField(this, "playing", false);
    __publicField(this, "speed", 1);
    __publicField(this, "lastNow", null);
    __publicField(this, "gated", false);
    this.timeline = C(t.track, 1), this.total = _(this.timeline), this.driver = o;
  }
  duration() {
    return this.total;
  }
  position() {
    return this.virtual;
  }
  isPlaying() {
    return this.playing;
  }
  play() {
    this.playing = true, this.lastNow = null;
  }
  pause() {
    this.playing = false;
  }
  setSpeed(t) {
    t > 0 && (this.speed = t);
  }
  tick(t) {
    var _a, _b, _c, _d;
    if (!this.playing || this.gated) {
      this.lastNow = t;
      return;
    }
    this.lastNow === null && (this.lastNow = t), this.virtual += (t - this.lastNow) * this.speed, this.lastNow = t, this.fireDue(false), (_b = (_a = this.driver).onProgress) == null ? void 0 : _b.call(_a, Math.min(this.virtual, this.total), this.total), this.cursor >= this.timeline.length && (this.playing = false, (_d = (_c = this.driver).onDone) == null ? void 0 : _d.call(_c));
  }
  seekForward(t) {
    var _a, _b;
    t < this.virtual || (this.virtual = t, this.fireDue(true), (_b = (_a = this.driver).onProgress) == null ? void 0 : _b.call(_a, Math.min(this.virtual, this.total), this.total));
  }
  fireDue(t) {
    for (; this.cursor < this.timeline.length && this.timeline[this.cursor].at <= this.virtual; ) this.fire(this.timeline[this.cursor], t), this.cursor += 1;
  }
  fire(t, o) {
    if (t.user) {
      const a = this.driver.typeUser(t.user, o);
      a && !o && (this.gated = true, a.finally(() => {
        this.gated = false, this.lastNow = null;
      }));
    } else t.event ? this.driver.pushEvent(JSON.stringify({ type: t.event.type, properties: t.event.properties ?? {} })) : t.dock && this.driver.dock(t.dock);
  }
}
const $ = (e) => {
  const t = Math.floor(e / 1e3);
  return `${Math.floor(t / 60)}:${String(t % 60).padStart(2, "0")}`;
}, P = [0.5, 1, 2, 4];
function D(e, t = document.body) {
  const o = document.createElement("div");
  o.className = "chat-player", o.style.cssText = "position:fixed;left:0;right:0;bottom:var(--input-h);z-index:9997;display:flex;align-items:center;gap:14px;padding:10px 18px;background:linear-gradient(#0d0f1700,#0d0f17cc);font:12px/1 'JetBrains Mono',monospace;color:#cdd3e0;user-select:none;";
  const a = document.createElement("button");
  a.className = "chat-player-toggle", a.style.cssText = "all:unset;cursor:pointer;width:30px;height:30px;border-radius:50%;border:1px solid #3df5d066;color:#3df5d0;text-align:center;line-height:30px;flex:0 0 auto;";
  const r = document.createElement("div");
  r.style.cssText = "position:relative;flex:1;height:18px;cursor:pointer;display:flex;align-items:center;";
  const c = document.createElement("div");
  c.style.cssText = "position:absolute;left:0;right:0;height:3px;background:#222838;border-radius:2px;";
  const d = document.createElement("div");
  d.style.cssText = "position:absolute;left:0;height:3px;width:0;background:#3df5d0;border-radius:2px;";
  const n = document.createElement("div");
  n.style.cssText = "position:absolute;width:11px;height:11px;border-radius:50%;background:#3df5d0;left:0;transform:translateX(-50%);", r.append(c, d, n);
  const p = document.createElement("span");
  p.style.cssText = "flex:0 0 auto;opacity:.7;min-width:74px;text-align:right;";
  const h = document.createElement("div");
  h.style.cssText = "display:flex;gap:6px;flex:0 0 auto;";
  let x = 1;
  const k = /* @__PURE__ */ new Map();
  for (const i of P) {
    const l = document.createElement("button");
    l.style.cssText = "all:unset;cursor:pointer;padding:3px 7px;border-radius:5px;font-size:11px;color:#9aa3b5;border:1px solid #ffffff14;", l.textContent = `${i}\xD7`, l.onclick = () => {
      x = i, e.setSpeed(i);
    }, k.set(i, l), h.appendChild(l);
  }
  o.append(a, r, p, h), t.appendChild(o), a.onclick = () => e.isPlaying() ? e.pause() : e.play();
  const f = (i) => {
    if (i >= e.position()) {
      e.seekForward(i);
      return;
    }
    const l = new URL(location.href);
    l.searchParams.set("at", String(Math.round(i))), location.assign(l.toString());
  }, g = (i) => {
    const l = r.getBoundingClientRect(), y = Math.max(0, Math.min(1, (i.clientX - l.left) / l.width));
    f(y * e.duration());
  };
  let s = false;
  r.addEventListener("mousedown", (i) => {
    s = true, g(i);
  }), window.addEventListener("mousemove", (i) => {
    s && g(i);
  }), window.addEventListener("mouseup", () => {
    s = false;
  });
  const u = () => {
    const i = e.duration(), l = i ? Math.min(1, e.position() / i) : 0;
    d.style.width = `${l * 100}%`, n.style.left = `${l * 100}%`, a.textContent = e.isPlaying() ? "\u23F8" : "\u25B6", p.textContent = `${$(Math.min(e.position(), i))} / ${$(i)}`;
    for (const [y, v] of k) {
      const b = x === y;
      v.style.color = b ? "#3df5d0" : "#9aa3b5", v.style.borderColor = b ? "#3df5d066" : "#ffffff14";
    }
    requestAnimationFrame(u);
  };
  requestAnimationFrame(u);
}
const F = 14, M = 400;
function S(e) {
  const t = document.createElement("div");
  t.style.cssText = "position:fixed;inset:0;display:flex;align-items:center;justify-content:center;color:#ffb4b4;font:14px/1.6 system-ui;white-space:pre-wrap;padding:24px;", t.textContent = e, document.body.appendChild(t);
}
async function j() {
  const e = new URLSearchParams(location.search), t = e.get("script") ?? "showcase-full", o = Number(e.get("speed") ?? "") || 1, a = Number(e.get("at") ?? "") || 0, r = "/infinite-chat/";
  let c;
  try {
    const s = await fetch(`${r}chats/${t}.json`);
    if (!s.ok) throw new Error(`HTTP ${s.status}`);
    c = await s.json();
  } catch (s) {
    S(`\u5267\u672C\u8F7D\u5165\u5931\u8D25: chats/${t}.json
${String(s)}`);
    return;
  }
  const d = N(c);
  if (!d.ok) {
    S(`\u5267\u672C\u975E\u6CD5(\u6761\u76EE ${d.index}): ${d.error}`);
    return;
  }
  const n = d.script;
  n.meta.title && (document.title = `${n.meta.title} \xB7 infinite-chat`);
  const { chat: p } = await E({ replay: [] });
  if (n.meta.theme) try {
    const s = n.meta.theme, u = typeof s == "string" ? await (await fetch(`${r}themes/${s}.json`)).text() : JSON.stringify(s);
    p.set_theme(u);
  } catch (s) {
    console.warn("[chat] \u4E3B\u9898\u8F7D\u5165\u5931\u8D25,\u7528\u9ED8\u8BA4", s);
  }
  const h = T(document.body), x = (s, u = 0) => {
    const i = document.querySelector(s);
    if (i) {
      i.click();
      return;
    }
    u < 120 ? requestAnimationFrame(() => x(s, u + 1)) : console.warn(`[chat] Dock \u6309\u94AE\u672A\u51FA\u73B0: ${s}`);
  }, k = { typeUser: async (s, u) => {
    const i = s.cps ?? F, l = s.holdMs ?? M;
    await h.typeText(s.text, i * o, u), u || await new Promise((y) => setTimeout(y, l / o)), h.flashSend(), p.note_send();
  }, pushEvent: (s) => p.push_event(s), dock: (s) => x(s === "deny" ? ".dock-deny" : ".dock-allow") }, f = new A(n, k);
  f.setSpeed(o), D(f), window.__player = f, a > 0 && (p.set_stream_rate(1e9), f.seekForward(a), setTimeout(() => p.set_stream_rate(200), 500)), f.play();
  const g = (s) => {
    f.tick(s), requestAnimationFrame(g);
  };
  requestAnimationFrame(g), console.info(`[chat] \u5267\u672C "${t}" \xB7 ${n.track.length} \u6761\u6307\u4EE4 \xB7 ${Math.round(f.duration() / 1e3)}s`);
}
j();
