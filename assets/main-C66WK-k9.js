const __vite__mapDeps=(i,m=__vite__mapDeps,d=(m.f||(m.f=["assets/debug-panel-DzykNQ2O.js","assets/boot-Bfd1wDNf.js","assets/replay-config-BGpffVO3.js","assets/style-panel-B4f0Zw69.js"])))=>i.map(i=>d[i]);
import { _ as d, b as I, __tla as __tla_0 } from "./boot-Bfd1wDNf.js";
Promise.all([
  (() => {
    try {
      return __tla_0;
    } catch {
    }
  })()
]).then(async () => {
  async function A() {
    const n = new URLSearchParams(location.search), i = n.get("server") ?? void 0, c = n.get("session") ?? void 0, l = !i, { loadReplayConfig: p } = await d(async () => {
      const { loadReplayConfig: e } = await import("./replay-config-BGpffVO3.js");
      return {
        loadReplayConfig: e
      };
    }, []), _ = p(), y = n.get("replay"), h = y ?? _.case ?? (l ? "showcase" : void 0), w = Number(n.get("speed") ?? "") || _.speed || 1;
    let m;
    if (h) try {
      m = await (await d(async () => {
        const { loadCase: e } = await import("./replay-Cvqaq-72.js");
        return {
          loadCase: e
        };
      }, [])).loadCase(h, w);
    } catch (e) {
      console.warn(`[replay] \u52A0\u8F7D\u5931\u8D25,\u8DF3\u8FC7\u91CD\u653E: ${h}`, e);
    }
    const v = n.has("bench");
    let u = 0;
    if (v) {
      const e = n.get("lines") ?? "10k", s = Number(n.get("spread") ?? "") || 100;
      try {
        const a = await (await fetch(`/infinite-chat/replays/longsession-${e}.json`)).json();
        m = a.map((t) => ({
          t: t.t * s,
          raw: t.raw
        })), u = a.length, console.info(`[bench] longsession-${e}: ${a.length} turns, spread=${s}ms`);
      } catch (r) {
        console.warn("[bench] \u8F7D\u957F\u4F1A\u8BDD\u5931\u8D25(\u5148\u8DD1 node scripts/gen-longsession.mjs):", r);
      }
    }
    const { chat: o, wasmModule: g } = await I({
      replay: m,
      serverUrl: i,
      sessionId: c
    });
    if (i) {
      const { SseClient: e } = await d(async () => {
        const { SseClient: s } = await import("./sse-client-UdbDDm5a.js");
        return {
          SseClient: s
        };
      }, []);
      new e({
        url: `${i}/event`,
        onEvent: (s) => o.push_event(s)
      }).start();
    }
    if (!n.has("noinput") && !l) {
      const { mountChatInput: e, parseModel: s } = await d(async () => {
        const { mountChatInput: a, parseModel: t } = await import("./chat-input-CDyUGpCJ.js");
        return {
          mountChatInput: a,
          parseModel: t
        };
      }, []), r = s(n.get("model") ?? "aliyuntokenplan/qwen3.7-max");
      e({
        serverUrl: i ?? "http://localhost:4096",
        sessionId: c,
        model: r,
        canvasLive: !!i,
        parent: document.body
      });
    }
    {
      const e = n.get("theme");
      if (e) try {
        const s = await fetch(`/infinite-chat/themes/${e}.json`);
        if (!s.ok) throw new Error(`HTTP ${s.status}`);
        o.set_theme(await s.text());
      } catch (s) {
        console.warn(`[theme] \u8F7D\u5165\u5931\u8D25,\u7528\u9ED8\u8BA4: ${e}`, s);
      }
    }
    if (n.has("debug")) {
      const e = document.createElement("div");
      e.style.cssText = "position:fixed;top:8px;right:8px;z-index:9999;display:flex;flex-direction:column;gap:8px;align-items:flex-end", document.body.appendChild(e);
      const { mountDebugPanel: s } = await d(async () => {
        const { mountDebugPanel: a } = await import("./debug-panel-DzykNQ2O.js");
        return {
          mountDebugPanel: a
        };
      }, __vite__mapDeps([0,1,2]));
      s(o, e);
      const { mountStylePanel: r } = await d(async () => {
        const { mountStylePanel: a } = await import("./style-panel-B4f0Zw69.js");
        return {
          mountStylePanel: a
        };
      }, __vite__mapDeps([3,1]));
      r(o, e);
    }
    if (n.has("msdf") || n.has("asciimsdf")) {
      const { loadMsdf: e } = await d(async () => {
        const { loadMsdf: a } = await import("./boot-Bfd1wDNf.js").then(async (m2) => {
          await m2.__tla;
          return m2;
        }).then((t) => t.h);
        return {
          loadMsdf: a
        };
      }, []), s = "/infinite-chat/", r = n.has("msdf") ? s + "fonts/lxgw-msdf" : s + "fonts/ascii-msdf";
      e(o, r).catch((a) => console.warn("[msdf] load skipped (\u56DE\u9000 TinySDF)", a));
    }
    if (n.has("verify")) {
      let e = 0;
      const s = setInterval(() => {
        o.set_debug_geometry(true), ++e > 20 && clearInterval(s);
      }, 200);
    }
    if (n.has("gallery")) {
      let e = 0;
      const s = setInterval(() => {
        o.set_shaderbox_gallery(true), ++e > 20 && clearInterval(s);
      }, 200);
    }
    if (v) {
      o.set_stream_rate(1e9), o.set_reveal_cps(Number.POSITIVE_INFINITY), n.has("sizefold") && o.set_bench_fold_width(true), n.has("novirt") && o.set_virtualize(false);
      const e = [];
      let s = -1, r = 0;
      const a = setInterval(() => {
        var _a;
        const t = o.stats(), x = g.memory.buffer.byteLength, b = {
          turns: t.retainedViews,
          storeChars: t.storeChars,
          retainedGlyphs: t.retainedGlyphs,
          retainedNodes: t.retainedNodes,
          frameGlyphs: t.glyphsVisible,
          fps: Math.round(t.fps),
          frameMsAvg: Number(t.frameMsAvg.toFixed(2)),
          wasmMiB: Number((x / 1048576).toFixed(1)),
          phAdvance: Number(t.phAdvance.toFixed(2)),
          phBfLayout: Number(t.phBfLayout.toFixed(2)),
          phBfGrid: Number(t.phBfGrid.toFixed(2)),
          phBfEmit: Number(t.phBfEmit.toFixed(2)),
          phBfTotal: Number(t.phBfTotal.toFixed(2)),
          phAdvIngest: Number(t.phAdvIngest.toFixed(2)),
          phAdvRoles: Number(t.phAdvRoles.toFixed(2)),
          phAdvReveal: Number(t.phAdvReveal.toFixed(2)),
          phAdvEnsure: Number(t.phAdvEnsure.toFixed(2)),
          phAdvSchedule: Number(t.phAdvSchedule.toFixed(2)),
          tierHot: t.tierHot,
          tierWarm: t.tierWarm,
          rebuilds: t.rebuilds
        };
        if (e.push(b), console.table([
          b
        ]), (u > 0 ? t.retainedViews >= u : t.retainedGlyphs > 0) && t.retainedGlyphs === s) {
          if (++r >= 8) {
            const f = [
              Object.keys(e[0]).join(","),
              ...e.map((E) => Object.values(E).join(","))
            ].join(`
`);
            window.__benchCSV = f, console.log(`[bench] done \u2014 CSV (window.__benchCSV):
${f}`), (_a = navigator.clipboard) == null ? void 0 : _a.writeText(f).catch(() => {
            }), clearInterval(a);
          }
        } else r = 0, s = t.retainedGlyphs;
      }, 1e3);
    }
    if (console.info("[harness] ChatCanvas started", {
      mode: i ? `live: ${i}` : "synthetic demo"
    }), l && N(), l && !y) {
      const { mountFilm: e } = await d(async () => {
        const { mountFilm: s } = await import("./scenes-BXUVMCrq.js");
        return {
          mountFilm: s
        };
      }, []);
      e(o);
    }
  }
  function N() {
    const n = "/infinite-chat/", i = document.createElement("div");
    i.style.cssText = "position:fixed;top:0;left:0;right:0;z-index:9998;display:flex;gap:14px;align-items:center;padding:8px 14px;font:13px/1.4 system-ui,sans-serif;color:#cdd3e0;background:linear-gradient(#0d0f17ee,#0d0f1700);pointer-events:none;";
    const c = (l, p) => `<a href="${p}" style="pointer-events:auto;color:#3df5d0;text-decoration:none;border:1px solid #3df5d066;border-radius:6px;padding:3px 9px">${l}</a>`;
    i.innerHTML = '<b style="color:#fff;letter-spacing:.3px">infinite-chat</b><span style="opacity:.6">intro film \xB7 SDF / \u6D41\u5F0F / \u65E0\u9650\u753B\u5E03</span><span style="flex:1"></span>' + c("\u{1F4AC} \u5B8C\u6574\u5BF9\u8BDD", `${n}chat/`) + c("\u{1F3A8} Icon Gallery", `${n}gallery.html`) + c("\u{1F4C4} Markdown demo", "?replay=showcase&speed=0.5") + c("GitHub", "https://github.com/OhBonsai/infinite-chat"), document.body.appendChild(i);
  }
  A().catch((n) => console.error("[harness] \u521D\u59CB\u5316\u5931\u8D25", n));
});
