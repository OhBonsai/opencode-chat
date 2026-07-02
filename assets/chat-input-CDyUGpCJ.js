function w(e) {
  const s = e.indexOf("/");
  return s < 0 ? { providerID: "", modelID: e } : { providerID: e.slice(0, s), modelID: e.slice(s + 1) };
}
async function y(e, s) {
  const t = await fetch(`${e}/session`, { method: "POST", headers: { "content-type": "application/json" }, body: "{}" });
  if (!t.ok) throw new Error(`\u5EFA\u4F1A\u8BDD\u5931\u8D25 ${t.status} ${await t.text()}`);
  const n = await t.json();
  if (!n.id) throw new Error("\u5EFA\u4F1A\u8BDD\u54CD\u5E94\u7F3A id");
  return n.id;
}
function b() {
  const e = document.createElement("div");
  e.style.cssText = "position:fixed;left:0;right:0;bottom:0;z-index:9000;display:flex;gap:8px;align-items:flex-end;padding:10px 12px;background:rgba(20,22,28,0.82);backdrop-filter:blur(6px);border-top:1px solid rgba(255,255,255,0.08)";
  const s = document.createElement("textarea");
  s.placeholder = "\u8F93\u5165\u6D88\u606F,Enter \u53D1\u9001 / Shift+Enter \u6362\u884C\u2026", s.rows = 1, s.style.cssText = "flex:1;resize:none;max-height:30vh;min-height:22px;padding:8px 10px;border-radius:8px;border:1px solid rgba(255,255,255,0.12);background:rgba(0,0,0,0.35);color:#e8e8ea;font:14px/1.4 system-ui,sans-serif;outline:none";
  const t = document.createElement("button");
  return t.textContent = "\u53D1\u9001", t.style.cssText = "padding:8px 16px;border-radius:8px;border:none;cursor:pointer;color:#fff;background:#3b6fe0;font:600 14px system-ui,sans-serif", e.appendChild(s), e.appendChild(t), { bar: e, ta: s, btn: t };
}
function g(e) {
  let s = -1;
  const t = () => {
    const o = e.offsetHeight;
    document.documentElement.style.setProperty("--input-h", `${o}px`), o !== s && (s = o, window.dispatchEvent(new Event("resize")));
  }, n = new ResizeObserver(t);
  n.observe(e), t();
  for (const o of [300, 1200]) setTimeout(() => window.dispatchEvent(new Event("resize")), o);
  return () => {
    n.disconnect(), document.documentElement.style.setProperty("--input-h", "0px"), window.dispatchEvent(new Event("resize"));
  };
}
const m = "ic_pending_send";
function E(e) {
  let s = e.sessionId;
  const { bar: t, ta: n, btn: o } = b(), a = document.createElement("div");
  a.style.cssText = "position:fixed;left:12px;right:12px;bottom:64px;z-index:9001;color:#ffb4b4;background:rgba(60,16,16,0.92);border:1px solid #7a2a2a;border-radius:8px;padding:8px 12px;font:13px/1.45 system-ui,sans-serif;white-space:pre-wrap;display:none";
  const l = (r) => {
    a.textContent = r, a.style.display = "block";
  }, v = () => {
    a.style.display = "none";
  }, u = () => {
    n.style.height = "auto", n.style.height = `${n.scrollHeight}px`;
  }, d = (r) => r instanceof TypeError ? `\u8FDE\u4E0D\u4E0A opencode (${e.serverUrl})\u3002\u5148\u8D77\u670D\u52A1\u7AEF:node scripts/serve.mjs,\u6216 ?server= \u6307\u5B9A\u5730\u5740\u3002` : String(r);
  let c = false;
  const p = async () => {
    const r = n.value.trim();
    if (!r || c) return;
    c = true, n.disabled = true, o.disabled = true;
    const h = o.textContent;
    if (v(), !e.canvasLive) {
      o.textContent = "\u8FDE\u63A5\u4E2D\u2026", console.info("[chat-input] \u753B\u5E03\u672A\u8FDE\u670D\u52A1\u7AEF,\u91CD\u8FDE\u540E\u7EED\u53D1", { serverUrl: e.serverUrl });
      try {
        const i = s ?? await y(e.serverUrl);
        sessionStorage.setItem(m, r);
        const x = new URL(location.href);
        x.searchParams.set("server", e.serverUrl), x.searchParams.set("session", i), location.assign(x.toString());
      } catch (i) {
        l(`\u65E0\u6CD5\u8FDE\u63A5:${d(i)}`), console.error("[chat-input] \u8FDE\u63A5\u5931\u8D25", i), c = false, n.disabled = false, o.disabled = false, o.textContent = h;
      }
      return;
    }
    o.textContent = "\u53D1\u9001\u4E2D\u2026", console.info("[chat-input] \u53D1\u9001", { serverUrl: e.serverUrl, session: s, text: r });
    try {
      s || (s = await y(e.serverUrl));
      const i = await fetch(`${e.serverUrl}/session/${s}/message`, { method: "POST", headers: { "content-type": "application/json" }, body: JSON.stringify({ parts: [{ type: "text", text: r }], model: e.model }) });
      i.ok ? (n.value = "", u()) : l(`\u53D1\u9001\u5931\u8D25 ${i.status}: ${(await i.text()).slice(0, 300)}`);
    } catch (i) {
      l(`\u53D1\u9001\u5931\u8D25:${d(i)}`), console.error("[chat-input] \u53D1\u9001\u5931\u8D25", i);
    } finally {
      c = false, n.disabled = false, o.disabled = false, o.textContent = h, n.focus();
    }
  };
  n.addEventListener("input", u), n.addEventListener("keydown", (r) => {
    r.isComposing || r.keyCode === 229 || r.key === "Enter" && !r.shiftKey && (r.preventDefault(), p());
  }), o.addEventListener("click", () => void p()), e.parent.appendChild(t), e.parent.appendChild(a);
  const f = g(t);
  if (e.canvasLive) {
    const r = sessionStorage.getItem(m);
    r && (sessionStorage.removeItem(m), n.value = r, u(), setTimeout(() => void p(), 150));
  }
  return () => {
    f(), e.parent.removeChild(t), e.parent.removeChild(a);
  };
}
function I(e) {
  const { bar: s, ta: t, btn: n } = b();
  t.readOnly = true, t.placeholder = "", e.appendChild(s);
  const o = g(s), a = () => {
    t.style.height = "auto", t.style.height = `${t.scrollHeight}px`;
  };
  let l = 0;
  return { typeText: (d, c, p = false) => {
    if (window.clearInterval(l), p || c <= 0) return t.value = d, a(), Promise.resolve();
    const f = [...d];
    t.value = "", t.focus();
    let r = 0;
    return new Promise((h) => {
      l = window.setInterval(() => {
        r += 1, t.value = f.slice(0, r).join(""), a(), r >= f.length && (window.clearInterval(l), h());
      }, 1e3 / c);
    });
  }, flashSend: () => {
    const d = n.style.background;
    n.style.background = "#5a8bff", n.style.transform = "scale(0.94)", window.setTimeout(() => {
      n.style.background = d, n.style.transform = "", t.value = "", a();
    }, 160);
  }, unmount: () => {
    window.clearInterval(l), o(), e.removeChild(s);
  } };
}
export {
  y as ensureSession,
  E as mountChatInput,
  I as mountScriptedInput,
  w as parseModel
};
