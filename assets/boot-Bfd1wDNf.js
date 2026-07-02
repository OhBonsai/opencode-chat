let _n, z, cn, gn, sn, Ft, un, fn, $t, Ot, Bt, bn, an;
let __tla = (async () => {
  (function() {
    const e = document.createElement("link").relList;
    if (e && e.supports && e.supports("modulepreload")) return;
    for (const _ of document.querySelectorAll('link[rel="modulepreload"]')) o(_);
    new MutationObserver((_) => {
      for (const a of _) if (a.type === "childList") for (const s of a.addedNodes) s.tagName === "LINK" && s.rel === "modulepreload" && o(s);
    }).observe(document, {
      childList: true,
      subtree: true
    });
    function t(_) {
      const a = {};
      return _.integrity && (a.integrity = _.integrity), _.referrerPolicy && (a.referrerPolicy = _.referrerPolicy), _.crossOrigin === "use-credentials" ? a.credentials = "include" : _.crossOrigin === "anonymous" ? a.credentials = "omit" : a.credentials = "same-origin", a;
    }
    function o(_) {
      if (_.ep) return;
      _.ep = true;
      const a = t(_);
      fetch(_.href, a);
    }
  })();
  let rt, ot, Be;
  rt = "modulepreload";
  ot = function(n) {
    return "/infinite-chat/" + n;
  };
  Be = {};
  z = function(e, t, o) {
    let _ = Promise.resolve();
    if (t && t.length > 0) {
      document.getElementsByTagName("link");
      const s = document.querySelector("meta[property=csp-nonce]"), u = (s == null ? void 0 : s.nonce) || (s == null ? void 0 : s.getAttribute("nonce"));
      _ = Promise.allSettled(t.map((c) => {
        if (c = ot(c), c in Be) return;
        Be[c] = true;
        const f = c.endsWith(".css"), g = f ? '[rel="stylesheet"]' : "";
        if (document.querySelector(`link[href="${c}"]${g}`)) return;
        const d = document.createElement("link");
        if (d.rel = f ? "stylesheet" : rt, f || (d.as = "script"), d.crossOrigin = "", d.href = c, u && d.setAttribute("nonce", u), document.head.appendChild(d), f) return new Promise((h, M) => {
          d.addEventListener("load", h), d.addEventListener("error", () => M(new Error(`Unable to preload CSS for ${c}`)));
        });
      }));
    }
    function a(s) {
      const u = new Event("vite:preloadError", {
        cancelable: true
      });
      if (u.payload = s, window.dispatchEvent(u), !u.defaultPrevented) throw s;
    }
    return _.then((s) => {
      for (const u of s || []) u.status === "rejected" && a(u.reason);
      return e().catch(a);
    });
  };
  let i;
  const G = new Array(128).fill(void 0);
  G.push(void 0, null, true, false);
  function r(n) {
    return G[n];
  }
  let Z = G.length;
  function b(n) {
    Z === G.length && G.push(G.length + 1);
    const e = Z;
    return Z = G[e], G[e] = n, e;
  }
  const qe = typeof TextDecoder < "u" ? new TextDecoder("utf-8", {
    ignoreBOM: true,
    fatal: true
  }) : {
    decode: () => {
      throw Error("TextDecoder not available");
    }
  };
  typeof TextDecoder < "u" && qe.decode();
  let K = null;
  function Q() {
    return (K === null || K.byteLength === 0) && (K = new Uint8Array(i.memory.buffer)), K;
  }
  function l(n, e) {
    return n = n >>> 0, qe.decode(Q().subarray(n, n + e));
  }
  function A(n, e) {
    try {
      return n.apply(this, e);
    } catch (t) {
      i.__wbindgen_export_0(b(t));
    }
  }
  function O(n) {
    return n == null;
  }
  let P = 0;
  const ie = typeof TextEncoder < "u" ? new TextEncoder("utf-8") : {
    encode: () => {
      throw Error("TextEncoder not available");
    }
  }, _t = typeof ie.encodeInto == "function" ? function(n, e) {
    return ie.encodeInto(n, e);
  } : function(n, e) {
    const t = ie.encode(n);
    return e.set(t), {
      read: n.length,
      written: t.length
    };
  };
  function L(n, e, t) {
    if (t === void 0) {
      const u = ie.encode(n), c = e(u.length, 1) >>> 0;
      return Q().subarray(c, c + u.length).set(u), P = u.length, c;
    }
    let o = n.length, _ = e(o, 1) >>> 0;
    const a = Q();
    let s = 0;
    for (; s < o; s++) {
      const u = n.charCodeAt(s);
      if (u > 127) break;
      a[_ + s] = u;
    }
    if (s !== o) {
      s !== 0 && (n = n.slice(s)), _ = t(_, o, o = s + n.length * 3, 1) >>> 0;
      const u = Q().subarray(_ + s, _ + o), c = _t(n, u);
      s += c.written, _ = t(_, o, s, 1) >>> 0;
    }
    return P = s, _;
  }
  let N = null;
  function m() {
    return (N === null || N.buffer.detached === true || N.buffer.detached === void 0 && N.buffer !== i.memory.buffer) && (N = new DataView(i.memory.buffer)), N;
  }
  let J = null;
  function je() {
    return (J === null || J.byteLength === 0) && (J = new Uint32Array(i.memory.buffer)), J;
  }
  function at(n, e) {
    return n = n >>> 0, je().subarray(n / 4, n / 4 + e);
  }
  function ct(n) {
    n < 132 || (G[n] = Z, Z = n);
  }
  function ee(n) {
    const e = r(n);
    return ct(n), e;
  }
  const Le = typeof FinalizationRegistry > "u" ? {
    register: () => {
    },
    unregister: () => {
    }
  } : new FinalizationRegistry((n) => {
    i.__wbindgen_export_4.get(n.dtor)(n.a, n.b);
  });
  function Oe(n, e, t, o) {
    const _ = {
      a: n,
      b: e,
      cnt: 1,
      dtor: t
    }, a = (...s) => {
      _.cnt++;
      const u = _.a;
      _.a = 0;
      try {
        return o(u, _.b, ...s);
      } finally {
        --_.cnt === 0 ? (i.__wbindgen_export_4.get(_.dtor)(u, _.b), Le.unregister(_)) : _.a = u;
      }
    };
    return a.original = _, Le.register(a, _, _), a;
  }
  function xe(n) {
    const e = typeof n;
    if (e == "number" || e == "boolean" || n == null) return `${n}`;
    if (e == "string") return `"${n}"`;
    if (e == "symbol") {
      const _ = n.description;
      return _ == null ? "Symbol" : `Symbol(${_})`;
    }
    if (e == "function") {
      const _ = n.name;
      return typeof _ == "string" && _.length > 0 ? `Function(${_})` : "Function";
    }
    if (Array.isArray(n)) {
      const _ = n.length;
      let a = "[";
      _ > 0 && (a += xe(n[0]));
      for (let s = 1; s < _; s++) a += ", " + xe(n[s]);
      return a += "]", a;
    }
    const t = /\[object ([^\]]+)\]/.exec(toString.call(n));
    let o;
    if (t && t.length > 1) o = t[1];
    else return toString.call(n);
    if (o == "Object") try {
      return "Object(" + JSON.stringify(n) + ")";
    } catch {
      return "Object";
    }
    return n instanceof Error ? `${n.name}: ${n.message}
${n.stack}` : o;
  }
  function st(n, e) {
    const t = e(n.length * 4, 4) >>> 0;
    return je().set(n, t / 4), P = n.length, t;
  }
  function it(n, e) {
    const t = e(n.length * 1, 1) >>> 0;
    return Q().set(n, t / 1), P = n.length, t;
  }
  function ut(n, e) {
    i.__wbindgen_export_5(n, e);
  }
  function ft(n, e, t) {
    i.__wbindgen_export_6(n, e, b(t));
  }
  const be = [
    "clamp-to-edge",
    "repeat",
    "mirror-repeat"
  ], Re = [
    "zero",
    "one",
    "src",
    "one-minus-src",
    "src-alpha",
    "one-minus-src-alpha",
    "dst",
    "one-minus-dst",
    "dst-alpha",
    "one-minus-dst-alpha",
    "src-alpha-saturated",
    "constant",
    "one-minus-constant",
    "src1",
    "one-minus-src1",
    "src1-alpha",
    "one-minus-src1-alpha"
  ], bt = [
    "add",
    "subtract",
    "reverse-subtract",
    "min",
    "max"
  ], gt = [
    "uniform",
    "storage",
    "read-only-storage"
  ], dt = [
    "opaque",
    "premultiplied"
  ], ge = [
    "never",
    "less",
    "equal",
    "less-equal",
    "greater",
    "not-equal",
    "greater-equal",
    "always"
  ], lt = [
    "none",
    "front",
    "back"
  ], $e = [
    "nearest",
    "linear"
  ], wt = [
    "ccw",
    "cw"
  ], pt = [
    "uint16",
    "uint32"
  ], de = [
    "load",
    "clear"
  ], mt = [
    "nearest",
    "linear"
  ], ht = [
    "low-power",
    "high-performance"
  ], yt = [
    "point-list",
    "line-list",
    "line-strip",
    "triangle-list",
    "triangle-strip"
  ], xt = [
    "filtering",
    "non-filtering",
    "comparison"
  ], le = [
    "keep",
    "zero",
    "replace",
    "invert",
    "increment-clamp",
    "decrement-clamp",
    "increment-wrap",
    "decrement-wrap"
  ], vt = [
    "write-only",
    "read-only",
    "read-write"
  ], we = [
    "store",
    "discard"
  ], St = [
    "all",
    "stencil-only",
    "depth-only"
  ], At = [
    "1d",
    "2d",
    "3d"
  ], j = [
    "r8unorm",
    "r8snorm",
    "r8uint",
    "r8sint",
    "r16uint",
    "r16sint",
    "r16float",
    "rg8unorm",
    "rg8snorm",
    "rg8uint",
    "rg8sint",
    "r32uint",
    "r32sint",
    "r32float",
    "rg16uint",
    "rg16sint",
    "rg16float",
    "rgba8unorm",
    "rgba8unorm-srgb",
    "rgba8snorm",
    "rgba8uint",
    "rgba8sint",
    "bgra8unorm",
    "bgra8unorm-srgb",
    "rgb9e5ufloat",
    "rgb10a2uint",
    "rgb10a2unorm",
    "rg11b10ufloat",
    "rg32uint",
    "rg32sint",
    "rg32float",
    "rgba16uint",
    "rgba16sint",
    "rgba16float",
    "rgba32uint",
    "rgba32sint",
    "rgba32float",
    "stencil8",
    "depth16unorm",
    "depth24plus",
    "depth24plus-stencil8",
    "depth32float",
    "depth32float-stencil8",
    "bc1-rgba-unorm",
    "bc1-rgba-unorm-srgb",
    "bc2-rgba-unorm",
    "bc2-rgba-unorm-srgb",
    "bc3-rgba-unorm",
    "bc3-rgba-unorm-srgb",
    "bc4-r-unorm",
    "bc4-r-snorm",
    "bc5-rg-unorm",
    "bc5-rg-snorm",
    "bc6h-rgb-ufloat",
    "bc6h-rgb-float",
    "bc7-rgba-unorm",
    "bc7-rgba-unorm-srgb",
    "etc2-rgb8unorm",
    "etc2-rgb8unorm-srgb",
    "etc2-rgb8a1unorm",
    "etc2-rgb8a1unorm-srgb",
    "etc2-rgba8unorm",
    "etc2-rgba8unorm-srgb",
    "eac-r11unorm",
    "eac-r11snorm",
    "eac-rg11unorm",
    "eac-rg11snorm",
    "astc-4x4-unorm",
    "astc-4x4-unorm-srgb",
    "astc-5x4-unorm",
    "astc-5x4-unorm-srgb",
    "astc-5x5-unorm",
    "astc-5x5-unorm-srgb",
    "astc-6x5-unorm",
    "astc-6x5-unorm-srgb",
    "astc-6x6-unorm",
    "astc-6x6-unorm-srgb",
    "astc-8x5-unorm",
    "astc-8x5-unorm-srgb",
    "astc-8x6-unorm",
    "astc-8x6-unorm-srgb",
    "astc-8x8-unorm",
    "astc-8x8-unorm-srgb",
    "astc-10x5-unorm",
    "astc-10x5-unorm-srgb",
    "astc-10x6-unorm",
    "astc-10x6-unorm-srgb",
    "astc-10x8-unorm",
    "astc-10x8-unorm-srgb",
    "astc-10x10-unorm",
    "astc-10x10-unorm-srgb",
    "astc-12x10-unorm",
    "astc-12x10-unorm-srgb",
    "astc-12x12-unorm",
    "astc-12x12-unorm-srgb"
  ], Mt = [
    "float",
    "unfilterable-float",
    "depth",
    "sint",
    "uint"
  ], pe = [
    "1d",
    "2d",
    "2d-array",
    "cube",
    "cube-array",
    "3d"
  ], Ct = [
    "uint8",
    "uint8x2",
    "uint8x4",
    "sint8",
    "sint8x2",
    "sint8x4",
    "unorm8",
    "unorm8x2",
    "unorm8x4",
    "snorm8",
    "snorm8x2",
    "snorm8x4",
    "uint16",
    "uint16x2",
    "uint16x4",
    "sint16",
    "sint16x2",
    "sint16x4",
    "unorm16",
    "unorm16x2",
    "unorm16x4",
    "snorm16",
    "snorm16x2",
    "snorm16x4",
    "float16",
    "float16x2",
    "float16x4",
    "float32",
    "float32x2",
    "float32x3",
    "float32x4",
    "uint32",
    "uint32x2",
    "uint32x3",
    "uint32x4",
    "sint32",
    "sint32x2",
    "sint32x3",
    "sint32x4",
    "unorm10-10-10-2",
    "unorm8x4-bgra"
  ], Pt = [
    "vertex",
    "instance"
  ], De = typeof FinalizationRegistry > "u" ? {
    register: () => {
    },
    unregister: () => {
    }
  } : new FinalizationRegistry((n) => i.__wbg_chatcanvas_free(n >>> 0, 1));
  class kt {
    __destroy_into_raw() {
      const e = this.__wbg_ptr;
      return this.__wbg_ptr = 0, De.unregister(this), e;
    }
    free() {
      const e = this.__destroy_into_raw();
      i.__wbg_chatcanvas_free(e, 0);
    }
    push_event(e) {
      const t = L(e, i.__wbindgen_export_2, i.__wbindgen_export_3), o = P;
      i.chatcanvas_push_event(this.__wbg_ptr, t, o);
    }
    set_paused(e) {
      i.chatcanvas_set_paused(this.__wbg_ptr, e);
    }
    seek_reveal(e) {
      i.chatcanvas_seek_reveal(this.__wbg_ptr, e);
    }
    set_math_em(e) {
      i.chatcanvas_set_math_em(this.__wbg_ptr, e);
    }
    frame_embeds() {
      let e, t;
      try {
        const a = i.__wbindgen_add_to_stack_pointer(-16);
        i.chatcanvas_frame_embeds(a, this.__wbg_ptr);
        var o = m().getInt32(a + 4 * 0, true), _ = m().getInt32(a + 4 * 1, true);
        return e = o, t = _, l(o, _);
      } finally {
        i.__wbindgen_add_to_stack_pointer(16), i.__wbindgen_export_1(e, t, 1);
      }
    }
    image_failed(e) {
      const t = L(e, i.__wbindgen_export_2, i.__wbindgen_export_3), o = P;
      i.chatcanvas_image_failed(this.__wbg_ptr, t, o);
    }
    refresh_fonts() {
      i.chatcanvas_refresh_fonts(this.__wbg_ptr);
    }
    set_selection(e) {
      const t = st(e, i.__wbindgen_export_2), o = P;
      i.chatcanvas_set_selection(this.__wbg_ptr, t, o);
    }
    visible_turns() {
      let e, t;
      try {
        const a = i.__wbindgen_add_to_stack_pointer(-16);
        i.chatcanvas_visible_turns(a, this.__wbg_ptr);
        var o = m().getInt32(a + 4 * 0, true), _ = m().getInt32(a + 4 * 1, true);
        return e = o, t = _, l(o, _);
      } finally {
        i.__wbindgen_add_to_stack_pointer(16), i.__wbindgen_export_1(e, t, 1);
      }
    }
    reply_question() {
      i.chatcanvas_reply_question(this.__wbg_ptr);
    }
    restart_reveal() {
      i.chatcanvas_restart_reveal(this.__wbg_ptr);
    }
    session_status() {
      let e, t;
      try {
        const a = i.__wbindgen_add_to_stack_pointer(-16);
        i.chatcanvas_session_status(a, this.__wbg_ptr);
        var o = m().getInt32(a + 4 * 0, true), _ = m().getInt32(a + 4 * 1, true);
        return e = o, t = _, l(o, _);
      } finally {
        i.__wbindgen_add_to_stack_pointer(16), i.__wbindgen_export_1(e, t, 1);
      }
    }
    set_glyph_mode(e) {
      i.chatcanvas_set_glyph_mode(this.__wbg_ptr, e);
    }
    set_reveal_cps(e) {
      i.chatcanvas_set_reveal_cps(this.__wbg_ptr, e);
    }
    set_virtualize(e) {
      i.chatcanvas_set_virtualize(this.__wbg_ptr, e);
    }
    set_reveal_slow(e) {
      i.chatcanvas_set_reveal_slow(this.__wbg_ptr, e);
    }
    set_stream_rate(e) {
      i.chatcanvas_set_stream_rate(this.__wbg_ptr, e);
    }
    set_table_style(e) {
      i.chatcanvas_set_table_style(this.__wbg_ptr, b(e));
    }
    reply_permission() {
      i.chatcanvas_reply_permission(this.__wbg_ptr);
    }
    scroll_code_block(e, t, o) {
      const _ = L(e, i.__wbindgen_export_2, i.__wbindgen_export_3), a = P;
      i.chatcanvas_scroll_code_block(this.__wbg_ptr, _, a, t, o);
    }
    upload_image_rgba(e, t, o, _, a) {
      const s = L(e, i.__wbindgen_export_2, i.__wbindgen_export_3), u = P, c = it(t, i.__wbindgen_export_2), f = P;
      i.chatcanvas_upload_image_rgba(this.__wbg_ptr, s, u, c, f, o, _, a);
    }
    visible_text_runs() {
      let e, t;
      try {
        const a = i.__wbindgen_add_to_stack_pointer(-16);
        i.chatcanvas_visible_text_runs(a, this.__wbg_ptr);
        var o = m().getInt32(a + 4 * 0, true), _ = m().getInt32(a + 4 * 1, true);
        return e = o, t = _, l(o, _);
      } finally {
        i.__wbindgen_add_to_stack_pointer(16), i.__wbindgen_export_1(e, t, 1);
      }
    }
    set_debug_geometry(e) {
      i.chatcanvas_set_debug_geometry(this.__wbg_ptr, e);
    }
    take_pending_images() {
      let e, t;
      try {
        const a = i.__wbindgen_add_to_stack_pointer(-16);
        i.chatcanvas_take_pending_images(a, this.__wbg_ptr);
        var o = m().getInt32(a + 4 * 0, true), _ = m().getInt32(a + 4 * 1, true);
        return e = o, t = _, l(o, _);
      } finally {
        i.__wbindgen_add_to_stack_pointer(16), i.__wbindgen_export_1(e, t, 1);
      }
    }
    code_block_at_screen(e, t) {
      let o, _;
      try {
        const u = i.__wbindgen_add_to_stack_pointer(-16);
        i.chatcanvas_code_block_at_screen(u, this.__wbg_ptr, e, t);
        var a = m().getInt32(u + 4 * 0, true), s = m().getInt32(u + 4 * 1, true);
        return o = a, _ = s, l(a, s);
      } finally {
        i.__wbindgen_add_to_stack_pointer(16), i.__wbindgen_export_1(o, _, 1);
      }
    }
    set_bench_fold_width(e) {
      i.chatcanvas_set_bench_fold_width(this.__wbg_ptr, e);
    }
    set_shaderbox_gallery(e) {
      i.chatcanvas_set_shaderbox_gallery(this.__wbg_ptr, e);
    }
    set_table_reveal_style(e) {
      i.chatcanvas_set_table_reveal_style(this.__wbg_ptr, e);
    }
    constructor(e, t) {
      try {
        const s = i.__wbindgen_add_to_stack_pointer(-16);
        i.chatcanvas_new(s, b(e), b(t));
        var o = m().getInt32(s + 4 * 0, true), _ = m().getInt32(s + 4 * 1, true), a = m().getInt32(s + 4 * 2, true);
        if (a) throw ee(_);
        return this.__wbg_ptr = o >>> 0, De.register(this, this.__wbg_ptr, this), this;
      } finally {
        i.__wbindgen_add_to_stack_pointer(16);
      }
    }
    find(e) {
      let t, o;
      try {
        const s = i.__wbindgen_add_to_stack_pointer(-16), u = L(e, i.__wbindgen_export_2, i.__wbindgen_export_3), c = P;
        i.chatcanvas_find(s, this.__wbg_ptr, u, c);
        var _ = m().getInt32(s + 4 * 0, true), a = m().getInt32(s + 4 * 1, true);
        return t = _, o = a, l(_, a);
      } finally {
        i.__wbindgen_add_to_stack_pointer(16), i.__wbindgen_export_1(t, o, 1);
      }
    }
    step() {
      i.chatcanvas_step(this.__wbg_ptr);
    }
    tick(e) {
      i.chatcanvas_tick(this.__wbg_ptr, e);
    }
    start() {
      i.chatcanvas_start(this.__wbg_ptr);
    }
    stats() {
      const e = i.chatcanvas_stats(this.__wbg_ptr);
      return ee(e);
    }
    pan_by(e, t) {
      i.chatcanvas_pan_by(this.__wbg_ptr, e, t);
    }
    zoom_at(e, t, o) {
      i.chatcanvas_zoom_at(this.__wbg_ptr, e, t, o);
    }
    load_msdf(e) {
      try {
        const _ = i.__wbindgen_add_to_stack_pointer(-16);
        i.chatcanvas_load_msdf(_, this.__wbg_ptr, b(e));
        var t = m().getInt32(_ + 4 * 0, true), o = m().getInt32(_ + 4 * 1, true);
        if (o) throw ee(t);
      } finally {
        i.__wbindgen_add_to_stack_pointer(16);
      }
    }
    note_send() {
      i.chatcanvas_note_send(this.__wbg_ptr);
    }
    scroll_to(e) {
      i.chatcanvas_scroll_to(this.__wbg_ptr, e);
    }
    set_theme(e) {
      const t = L(e, i.__wbindgen_export_2, i.__wbindgen_export_3), o = P;
      i.chatcanvas_set_theme(this.__wbg_ptr, t, o);
    }
    stop_turn() {
      i.chatcanvas_stop_turn(this.__wbg_ptr);
    }
  }
  async function Tt(n, e) {
    if (typeof Response == "function" && n instanceof Response) {
      if (typeof WebAssembly.instantiateStreaming == "function") try {
        return await WebAssembly.instantiateStreaming(n, e);
      } catch (o) {
        if (n.headers.get("Content-Type") != "application/wasm") console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", o);
        else throw o;
      }
      const t = await n.arrayBuffer();
      return await WebAssembly.instantiate(t, e);
    } else {
      const t = await WebAssembly.instantiate(n, e);
      return t instanceof WebAssembly.Instance ? {
        instance: t,
        module: n
      } : t;
    }
  }
  function Et() {
    const n = {};
    return n.wbg = {}, n.wbg.__wbg_Window_9e7ea8667e28eb00 = function(e) {
      const t = r(e).Window;
      return b(t);
    }, n.wbg.__wbg_WorkerGlobalScope_0169ffb9adb5f5ef = function(e) {
      const t = r(e).WorkerGlobalScope;
      return b(t);
    }, n.wbg.__wbg_addEventListener_90e553fdce254421 = function() {
      return A(function(e, t, o, _) {
        r(e).addEventListener(l(t, o), r(_));
      }, arguments);
    }, n.wbg.__wbg_apply_36be6a55257c99bf = function() {
      return A(function(e, t, o) {
        const _ = r(e).apply(r(t), r(o));
        return b(_);
      }, arguments);
    }, n.wbg.__wbg_beginRenderPass_aefd0d9681a1f010 = function() {
      return A(function(e, t) {
        const o = r(e).beginRenderPass(r(t));
        return b(o);
      }, arguments);
    }, n.wbg.__wbg_buffer_09165b52af8c5237 = function(e) {
      const t = r(e).buffer;
      return b(t);
    }, n.wbg.__wbg_buffer_609cc3eee51ed158 = function(e) {
      const t = r(e).buffer;
      return b(t);
    }, n.wbg.__wbg_call_672a4d21634d4a24 = function() {
      return A(function(e, t) {
        const o = r(e).call(r(t));
        return b(o);
      }, arguments);
    }, n.wbg.__wbg_call_b8adc8b1d0a0d8eb = function() {
      return A(function(e, t, o, _, a) {
        const s = r(e).call(r(t), r(o), r(_), r(a));
        return b(s);
      }, arguments);
    }, n.wbg.__wbg_clientHeight_216178c194000db4 = function(e) {
      return r(e).clientHeight;
    }, n.wbg.__wbg_clientWidth_ce67a04dc15fce39 = function(e) {
      return r(e).clientWidth;
    }, n.wbg.__wbg_configure_86dd92dde48d105a = function() {
      return A(function(e, t) {
        r(e).configure(r(t));
      }, arguments);
    }, n.wbg.__wbg_createBindGroupLayout_f0635625a1a82bea = function() {
      return A(function(e, t) {
        const o = r(e).createBindGroupLayout(r(t));
        return b(o);
      }, arguments);
    }, n.wbg.__wbg_createBindGroup_043b06d20124f91e = function(e, t) {
      const o = r(e).createBindGroup(r(t));
      return b(o);
    }, n.wbg.__wbg_createBuffer_086a8bb05ced884a = function() {
      return A(function(e, t) {
        const o = r(e).createBuffer(r(t));
        return b(o);
      }, arguments);
    }, n.wbg.__wbg_createCommandEncoder_aa9ae9d445bb7abf = function(e, t) {
      const o = r(e).createCommandEncoder(r(t));
      return b(o);
    }, n.wbg.__wbg_createPipelineLayout_5cc7e994e46201c7 = function(e, t) {
      const o = r(e).createPipelineLayout(r(t));
      return b(o);
    }, n.wbg.__wbg_createRenderPipeline_47152f2f57b11194 = function() {
      return A(function(e, t) {
        const o = r(e).createRenderPipeline(r(t));
        return b(o);
      }, arguments);
    }, n.wbg.__wbg_createSampler_f3970b77a6f36963 = function(e, t) {
      const o = r(e).createSampler(r(t));
      return b(o);
    }, n.wbg.__wbg_createShaderModule_9ec201507fe4949e = function(e, t) {
      const o = r(e).createShaderModule(r(t));
      return b(o);
    }, n.wbg.__wbg_createTexture_09f18232c5ad6e69 = function() {
      return A(function(e, t) {
        const o = r(e).createTexture(r(t));
        return b(o);
      }, arguments);
    }, n.wbg.__wbg_createView_f7cd0a0356a46f3b = function() {
      return A(function(e, t) {
        const o = r(e).createView(r(t));
        return b(o);
      }, arguments);
    }, n.wbg.__wbg_devicePixelRatio_68c391265f05d093 = function(e) {
      return r(e).devicePixelRatio;
    }, n.wbg.__wbg_document_d249400bd7bd996d = function(e) {
      const t = r(e).document;
      return O(t) ? 0 : b(t);
    }, n.wbg.__wbg_draw_d38c9207eb049f56 = function(e, t, o, _, a) {
      r(e).draw(t >>> 0, o >>> 0, _ >>> 0, a >>> 0);
    }, n.wbg.__wbg_end_d54348baf0bf3b70 = function(e) {
      r(e).end();
    }, n.wbg.__wbg_error_7534b8e9a36f1ab4 = function(e, t) {
      let o, _;
      try {
        o = e, _ = t, console.error(l(e, t));
      } finally {
        i.__wbindgen_export_1(o, _, 1);
      }
    }, n.wbg.__wbg_fetch_a9bc66c159c18e19 = function(e) {
      const t = fetch(r(e));
      return b(t);
    }, n.wbg.__wbg_finish_db34a19c90c07af7 = function(e) {
      const t = r(e).finish();
      return b(t);
    }, n.wbg.__wbg_finish_e2d3808af76b422a = function(e, t) {
      const o = r(e).finish(r(t));
      return b(o);
    }, n.wbg.__wbg_from_2a5d3e218e67aa85 = function(e) {
      const t = Array.from(r(e));
      return b(t);
    }, n.wbg.__wbg_getContext_e9cf379449413580 = function() {
      return A(function(e, t, o) {
        const _ = r(e).getContext(l(t, o));
        return O(_) ? 0 : b(_);
      }, arguments);
    }, n.wbg.__wbg_getContext_f65a0debd1e8f8e8 = function() {
      return A(function(e, t, o) {
        const _ = r(e).getContext(l(t, o));
        return O(_) ? 0 : b(_);
      }, arguments);
    }, n.wbg.__wbg_getCurrentTexture_6ee19b05d6ba43ba = function() {
      return A(function(e) {
        const t = r(e).getCurrentTexture();
        return b(t);
      }, arguments);
    }, n.wbg.__wbg_getPreferredCanvasFormat_c56b5a9a243fe942 = function(e) {
      const t = r(e).getPreferredCanvasFormat();
      return (j.indexOf(t) + 1 || 96) - 1;
    }, n.wbg.__wbg_get_67b2ba62fc30de12 = function() {
      return A(function(e, t) {
        const o = Reflect.get(r(e), r(t));
        return b(o);
      }, arguments);
    }, n.wbg.__wbg_get_b9b93047fe3cf45b = function(e, t) {
      const o = r(e)[t >>> 0];
      return b(o);
    }, n.wbg.__wbg_get_e27dfaeb6f46bd45 = function(e, t) {
      const o = r(e)[t >>> 0];
      return O(o) ? 0 : b(o);
    }, n.wbg.__wbg_gpu_1b22165b67dd5a59 = function(e) {
      const t = r(e).gpu;
      return b(t);
    }, n.wbg.__wbg_height_838cee19ba8597db = function(e) {
      return r(e).height;
    }, n.wbg.__wbg_instanceof_Error_4d54113b22d20306 = function(e) {
      let t;
      try {
        t = r(e) instanceof Error;
      } catch {
        t = false;
      }
      return t;
    }, n.wbg.__wbg_instanceof_Float32Array_01dd91be3195315d = function(e) {
      let t;
      try {
        t = r(e) instanceof Float32Array;
      } catch {
        t = false;
      }
      return t;
    }, n.wbg.__wbg_instanceof_GpuAdapter_331cc7dcda68de8c = function(e) {
      let t;
      try {
        t = r(e) instanceof GPUAdapter;
      } catch {
        t = false;
      }
      return t;
    }, n.wbg.__wbg_instanceof_GpuCanvasContext_4ea475a10f693c29 = function(e) {
      let t;
      try {
        t = r(e) instanceof GPUCanvasContext;
      } catch {
        t = false;
      }
      return t;
    }, n.wbg.__wbg_instanceof_Response_f2cc20d9f7dfd644 = function(e) {
      let t;
      try {
        t = r(e) instanceof Response;
      } catch {
        t = false;
      }
      return t;
    }, n.wbg.__wbg_instanceof_Uint32Array_b8b88c093c0d7ff4 = function(e) {
      let t;
      try {
        t = r(e) instanceof Uint32Array;
      } catch {
        t = false;
      }
      return t;
    }, n.wbg.__wbg_instanceof_Uint8Array_17156bcf118086a9 = function(e) {
      let t;
      try {
        t = r(e) instanceof Uint8Array;
      } catch {
        t = false;
      }
      return t;
    }, n.wbg.__wbg_instanceof_Window_def73ea0955fc569 = function(e) {
      let t;
      try {
        t = r(e) instanceof Window;
      } catch {
        t = false;
      }
      return t;
    }, n.wbg.__wbg_isArray_a1eab7e0d067391b = function(e) {
      return Array.isArray(r(e));
    }, n.wbg.__wbg_label_7045a786095b1bab = function(e, t) {
      const o = r(t).label, _ = L(o, i.__wbindgen_export_2, i.__wbindgen_export_3), a = P;
      m().setInt32(e + 4 * 1, a, true), m().setInt32(e + 4 * 0, _, true);
    }, n.wbg.__wbg_length_3b4f022188ae8db6 = function(e) {
      return r(e).length;
    }, n.wbg.__wbg_length_6ca527665d89694d = function(e) {
      return r(e).length;
    }, n.wbg.__wbg_length_a446193dc22c12f8 = function(e) {
      return r(e).length;
    }, n.wbg.__wbg_length_e2d2a49132c1b256 = function(e) {
      return r(e).length;
    }, n.wbg.__wbg_limits_563f98195b4aab75 = function(e) {
      const t = r(e).limits;
      return b(t);
    }, n.wbg.__wbg_location_350d99456c2f3693 = function(e) {
      const t = r(e).location;
      return b(t);
    }, n.wbg.__wbg_log_0cc1b7768397bcfe = function(e, t, o, _, a, s, u, c) {
      let f, g;
      try {
        f = e, g = t, console.log(l(e, t), l(o, _), l(a, s), l(u, c));
      } finally {
        i.__wbindgen_export_1(f, g, 1);
      }
    }, n.wbg.__wbg_log_cb9e190acc5753fb = function(e, t) {
      let o, _;
      try {
        o = e, _ = t, console.log(l(e, t));
      } finally {
        i.__wbindgen_export_1(o, _, 1);
      }
    }, n.wbg.__wbg_mark_7438147ce31e9d4b = function(e, t) {
      performance.mark(l(e, t));
    }, n.wbg.__wbg_maxBindGroups_30d01da76ad53580 = function(e) {
      return r(e).maxBindGroups;
    }, n.wbg.__wbg_maxBindingsPerBindGroup_3dcdeb4a7de67a4a = function(e) {
      return r(e).maxBindingsPerBindGroup;
    }, n.wbg.__wbg_maxBufferSize_a3c3e79851bb49a7 = function(e) {
      return r(e).maxBufferSize;
    }, n.wbg.__wbg_maxColorAttachmentBytesPerSample_61daf47ae1b88dc2 = function(e) {
      return r(e).maxColorAttachmentBytesPerSample;
    }, n.wbg.__wbg_maxColorAttachments_f8f65390ed7c3dcd = function(e) {
      return r(e).maxColorAttachments;
    }, n.wbg.__wbg_maxComputeInvocationsPerWorkgroup_dbfa932a2c3d9ca0 = function(e) {
      return r(e).maxComputeInvocationsPerWorkgroup;
    }, n.wbg.__wbg_maxComputeWorkgroupSizeX_2a7fdde2d850eb69 = function(e) {
      return r(e).maxComputeWorkgroupSizeX;
    }, n.wbg.__wbg_maxComputeWorkgroupSizeY_ae6eb3af592e045d = function(e) {
      return r(e).maxComputeWorkgroupSizeY;
    }, n.wbg.__wbg_maxComputeWorkgroupSizeZ_df6389c6ad61aa20 = function(e) {
      return r(e).maxComputeWorkgroupSizeZ;
    }, n.wbg.__wbg_maxComputeWorkgroupStorageSize_d090d78935189091 = function(e) {
      return r(e).maxComputeWorkgroupStorageSize;
    }, n.wbg.__wbg_maxComputeWorkgroupsPerDimension_5d5d832c21854769 = function(e) {
      return r(e).maxComputeWorkgroupsPerDimension;
    }, n.wbg.__wbg_maxDynamicStorageBuffersPerPipelineLayout_0d5102fd812fe086 = function(e) {
      return r(e).maxDynamicStorageBuffersPerPipelineLayout;
    }, n.wbg.__wbg_maxDynamicUniformBuffersPerPipelineLayout_fd6efab6fa18099a = function(e) {
      return r(e).maxDynamicUniformBuffersPerPipelineLayout;
    }, n.wbg.__wbg_maxSampledTexturesPerShaderStage_4ffa7a7339d366d7 = function(e) {
      return r(e).maxSampledTexturesPerShaderStage;
    }, n.wbg.__wbg_maxSamplersPerShaderStage_776dbf5a1fdc58b1 = function(e) {
      return r(e).maxSamplersPerShaderStage;
    }, n.wbg.__wbg_maxStorageBufferBindingSize_4a81009504bfcacd = function(e) {
      return r(e).maxStorageBufferBindingSize;
    }, n.wbg.__wbg_maxStorageBuffersPerShaderStage_772149c39281f13c = function(e) {
      return r(e).maxStorageBuffersPerShaderStage;
    }, n.wbg.__wbg_maxStorageTexturesPerShaderStage_181856fa7bd31bd2 = function(e) {
      return r(e).maxStorageTexturesPerShaderStage;
    }, n.wbg.__wbg_maxTextureArrayLayers_c50110b7591a08e7 = function(e) {
      return r(e).maxTextureArrayLayers;
    }, n.wbg.__wbg_maxTextureDimension1D_8886fff72f64818a = function(e) {
      return r(e).maxTextureDimension1D;
    }, n.wbg.__wbg_maxTextureDimension2D_0e30b1b618696302 = function(e) {
      return r(e).maxTextureDimension2D;
    }, n.wbg.__wbg_maxTextureDimension3D_2f567b561a18a953 = function(e) {
      return r(e).maxTextureDimension3D;
    }, n.wbg.__wbg_maxUniformBufferBindingSize_50a7723e932bbd63 = function(e) {
      return r(e).maxUniformBufferBindingSize;
    }, n.wbg.__wbg_maxUniformBuffersPerShaderStage_cfac0560ee2b33a2 = function(e) {
      return r(e).maxUniformBuffersPerShaderStage;
    }, n.wbg.__wbg_maxVertexAttributes_6bd060b2025920cc = function(e) {
      return r(e).maxVertexAttributes;
    }, n.wbg.__wbg_maxVertexBufferArrayStride_b3c77c1ff836be9f = function(e) {
      return r(e).maxVertexBufferArrayStride;
    }, n.wbg.__wbg_maxVertexBuffers_b4635256105b2915 = function(e) {
      return r(e).maxVertexBuffers;
    }, n.wbg.__wbg_measure_fb7825c11612c823 = function() {
      return A(function(e, t, o, _) {
        let a, s, u, c;
        try {
          a = e, s = t, u = o, c = _, performance.measure(l(e, t), l(o, _));
        } finally {
          i.__wbindgen_export_1(a, s, 1), i.__wbindgen_export_1(u, c, 1);
        }
      }, arguments);
    }, n.wbg.__wbg_message_97a2af9b89d693a3 = function(e) {
      const t = r(e).message;
      return b(t);
    }, n.wbg.__wbg_minStorageBufferOffsetAlignment_989812b5a6a4b5e7 = function(e) {
      return r(e).minStorageBufferOffsetAlignment;
    }, n.wbg.__wbg_minUniformBufferOffsetAlignment_ff7899c34a8303e7 = function(e) {
      return r(e).minUniformBufferOffsetAlignment;
    }, n.wbg.__wbg_name_0b327d569f00ebee = function(e) {
      const t = r(e).name;
      return b(t);
    }, n.wbg.__wbg_navigator_0a9bf1120e24fec2 = function(e) {
      const t = r(e).navigator;
      return b(t);
    }, n.wbg.__wbg_navigator_1577371c070c8947 = function(e) {
      const t = r(e).navigator;
      return b(t);
    }, n.wbg.__wbg_new_018dcc2d6c8c2f6a = function() {
      return A(function() {
        const e = new Headers();
        return b(e);
      }, arguments);
    }, n.wbg.__wbg_new_405e22f390576ce2 = function() {
      const e = new Object();
      return b(e);
    }, n.wbg.__wbg_new_780abee5c1739fd7 = function(e) {
      const t = new Float32Array(r(e));
      return b(t);
    }, n.wbg.__wbg_new_78feb108b6472713 = function() {
      const e = new Array();
      return b(e);
    }, n.wbg.__wbg_new_80bf4ee74f41ff92 = function() {
      return A(function() {
        const e = new URLSearchParams();
        return b(e);
      }, arguments);
    }, n.wbg.__wbg_new_8a6f238a6ece86ea = function() {
      const e = new Error();
      return b(e);
    }, n.wbg.__wbg_new_9ffbe0a71eff35e3 = function() {
      return A(function(e, t) {
        const o = new URL(l(e, t));
        return b(o);
      }, arguments);
    }, n.wbg.__wbg_new_a12002a7f91c75be = function(e) {
      const t = new Uint8Array(r(e));
      return b(t);
    }, n.wbg.__wbg_new_e3b321dcfef89fc7 = function(e) {
      const t = new Uint32Array(r(e));
      return b(t);
    }, n.wbg.__wbg_newnoargs_105ed471475aaf50 = function(e, t) {
      const o = new Function(l(e, t));
      return b(o);
    }, n.wbg.__wbg_newwithbyteoffsetandlength_d97e637ebe145a9a = function(e, t, o) {
      const _ = new Uint8Array(r(e), t >>> 0, o >>> 0);
      return b(_);
    }, n.wbg.__wbg_newwithbyteoffsetandlength_f1dead44d1fc7212 = function(e, t, o) {
      const _ = new Uint32Array(r(e), t >>> 0, o >>> 0);
      return b(_);
    }, n.wbg.__wbg_newwithlength_bd3de93688d68fbc = function(e) {
      const t = new Uint32Array(e >>> 0);
      return b(t);
    }, n.wbg.__wbg_newwithstr_78e86e03c4ae814e = function() {
      return A(function(e, t) {
        const o = new Request(l(e, t));
        return b(o);
      }, arguments);
    }, n.wbg.__wbg_newwithstrandinit_06c535e0a867c635 = function() {
      return A(function(e, t, o) {
        const _ = new Request(l(e, t), r(o));
        return b(_);
      }, arguments);
    }, n.wbg.__wbg_now_2c95c9de01293173 = function(e) {
      return r(e).now();
    }, n.wbg.__wbg_now_d18023d54d4e5500 = function(e) {
      return r(e).now();
    }, n.wbg.__wbg_ok_3aaf32d069979723 = function(e) {
      return r(e).ok;
    }, n.wbg.__wbg_performance_7a3ffd0b17f663ad = function(e) {
      const t = r(e).performance;
      return b(t);
    }, n.wbg.__wbg_performance_c185c0cdc2766575 = function(e) {
      const t = r(e).performance;
      return O(t) ? 0 : b(t);
    }, n.wbg.__wbg_push_737cfc8c1432c2c6 = function(e, t) {
      return r(e).push(r(t));
    }, n.wbg.__wbg_querySelectorAll_40998fd748f057ef = function() {
      return A(function(e, t, o) {
        const _ = r(e).querySelectorAll(l(t, o));
        return b(_);
      }, arguments);
    }, n.wbg.__wbg_queueMicrotask_97d92b4fcc8a61c5 = function(e) {
      queueMicrotask(r(e));
    }, n.wbg.__wbg_queueMicrotask_d3219def82552485 = function(e) {
      const t = r(e).queueMicrotask;
      return b(t);
    }, n.wbg.__wbg_queue_0ffbb97537a0c4ed = function(e) {
      const t = r(e).queue;
      return b(t);
    }, n.wbg.__wbg_requestAdapter_f09d28b3f37de26c = function(e, t) {
      const o = r(e).requestAdapter(r(t));
      return b(o);
    }, n.wbg.__wbg_requestAnimationFrame_d7fd890aaefc3246 = function() {
      return A(function(e, t) {
        return r(e).requestAnimationFrame(r(t));
      }, arguments);
    }, n.wbg.__wbg_requestDevice_51509dadc50b2e9d = function(e, t) {
      const o = r(e).requestDevice(r(t));
      return b(o);
    }, n.wbg.__wbg_resolve_4851785c9c5f573d = function(e) {
      const t = Promise.resolve(r(e));
      return b(t);
    }, n.wbg.__wbg_search_c1c3bfbeadd96c47 = function() {
      return A(function(e, t) {
        const o = r(t).search, _ = L(o, i.__wbindgen_export_2, i.__wbindgen_export_3), a = P;
        m().setInt32(e + 4 * 1, a, true), m().setInt32(e + 4 * 0, _, true);
      }, arguments);
    }, n.wbg.__wbg_search_e0e79cfe010c5c23 = function(e, t) {
      const o = r(t).search, _ = L(o, i.__wbindgen_export_2, i.__wbindgen_export_3), a = P;
      m().setInt32(e + 4 * 1, a, true), m().setInt32(e + 4 * 0, _, true);
    }, n.wbg.__wbg_setBindGroup_a81ce7b3934585bf = function(e, t, o) {
      r(e).setBindGroup(t >>> 0, r(o));
    }, n.wbg.__wbg_setBindGroup_bb0c2c05b7c49401 = function() {
      return A(function(e, t, o, _, a, s, u) {
        r(e).setBindGroup(t >>> 0, r(o), at(_, a), s, u >>> 0);
      }, arguments);
    }, n.wbg.__wbg_setPipeline_78f8f6d440dddd25 = function(e, t) {
      r(e).setPipeline(r(t));
    }, n.wbg.__wbg_setVertexBuffer_b0d3128a04bfd766 = function(e, t, o, _, a) {
      r(e).setVertexBuffer(t >>> 0, r(o), _, a);
    }, n.wbg.__wbg_setVertexBuffer_edbff6ddb5055174 = function(e, t, o, _) {
      r(e).setVertexBuffer(t >>> 0, r(o), _);
    }, n.wbg.__wbg_set_10bad9bee0e9c58b = function(e, t, o) {
      r(e).set(r(t), o >>> 0);
    }, n.wbg.__wbg_set_65595bdd868b3009 = function(e, t, o) {
      r(e).set(r(t), o >>> 0);
    }, n.wbg.__wbg_set_bb8cecf6a62b9f46 = function() {
      return A(function(e, t, o) {
        return Reflect.set(r(e), r(t), r(o));
      }, arguments);
    }, n.wbg.__wbg_set_d23661d19148b229 = function(e, t, o) {
      r(e).set(r(t), o >>> 0);
    }, n.wbg.__wbg_seta_721deab95e136b71 = function(e, t) {
      r(e).a = t;
    }, n.wbg.__wbg_setaccess_b20bfa3ec6b65d05 = function(e, t) {
      r(e).access = vt[t];
    }, n.wbg.__wbg_setaddressmodeu_9c0b2104a94d10f3 = function(e, t) {
      r(e).addressModeU = be[t];
    }, n.wbg.__wbg_setaddressmodev_a9bedc188ff29608 = function(e, t) {
      r(e).addressModeV = be[t];
    }, n.wbg.__wbg_setaddressmodew_5774889145ce3789 = function(e, t) {
      r(e).addressModeW = be[t];
    }, n.wbg.__wbg_setalpha_2c7bdc9da833b6c2 = function(e, t) {
      r(e).alpha = r(t);
    }, n.wbg.__wbg_setalphamode_fc3528d234b1fefa = function(e, t) {
      r(e).alphaMode = dt[t];
    }, n.wbg.__wbg_setalphatocoverageenabled_314ce1ca1759b395 = function(e, t) {
      r(e).alphaToCoverageEnabled = t !== 0;
    }, n.wbg.__wbg_setarraylayercount_3c7942d623042874 = function(e, t) {
      r(e).arrayLayerCount = t >>> 0;
    }, n.wbg.__wbg_setarraystride_4b36d0822dea74a8 = function(e, t) {
      r(e).arrayStride = t;
    }, n.wbg.__wbg_setaspect_f06e234d0aacd1a6 = function(e, t) {
      r(e).aspect = St[t];
    }, n.wbg.__wbg_setattributes_382cc084e6792c33 = function(e, t) {
      r(e).attributes = r(t);
    }, n.wbg.__wbg_setb_f53c2f10173c804f = function(e, t) {
      r(e).b = t;
    }, n.wbg.__wbg_setbasearraylayer_a5b968338c5c56b6 = function(e, t) {
      r(e).baseArrayLayer = t >>> 0;
    }, n.wbg.__wbg_setbasemiplevel_e3288c2d851da708 = function(e, t) {
      r(e).baseMipLevel = t >>> 0;
    }, n.wbg.__wbg_setbeginningofpasswriteindex_35dcbf135e4f9d61 = function(e, t) {
      r(e).beginningOfPassWriteIndex = t >>> 0;
    }, n.wbg.__wbg_setbindgrouplayouts_8de6e109dd34a448 = function(e, t) {
      r(e).bindGroupLayouts = r(t);
    }, n.wbg.__wbg_setbinding_5276d6202fceba46 = function(e, t) {
      r(e).binding = t >>> 0;
    }, n.wbg.__wbg_setbinding_9e9ed8b6e1418176 = function(e, t) {
      r(e).binding = t >>> 0;
    }, n.wbg.__wbg_setblend_6828ff186670f414 = function(e, t) {
      r(e).blend = r(t);
    }, n.wbg.__wbg_setbuffer_1acdac44d9638973 = function(e, t) {
      r(e).buffer = r(t);
    }, n.wbg.__wbg_setbuffer_74b7b0adf855cf1a = function(e, t) {
      r(e).buffer = r(t);
    }, n.wbg.__wbg_setbuffers_53e83b7c7a5c95aa = function(e, t) {
      r(e).buffers = r(t);
    }, n.wbg.__wbg_setbytesperrow_9249690c44f83135 = function(e, t) {
      r(e).bytesPerRow = t >>> 0;
    }, n.wbg.__wbg_setclearvalue_f82fff01ed0b5c35 = function(e, t) {
      r(e).clearValue = r(t);
    }, n.wbg.__wbg_setcode_6b6ad02fc1705aa2 = function(e, t, o) {
      r(e).code = l(t, o);
    }, n.wbg.__wbg_setcolor_0df2c5f47a951ac1 = function(e, t) {
      r(e).color = r(t);
    }, n.wbg.__wbg_setcolorattachments_de625dd9a4850a13 = function(e, t) {
      r(e).colorAttachments = r(t);
    }, n.wbg.__wbg_setcompare_1b67d8112d05628e = function(e, t) {
      r(e).compare = ge[t];
    }, n.wbg.__wbg_setcompare_8fbddcdd4781f49a = function(e, t) {
      r(e).compare = ge[t];
    }, n.wbg.__wbg_setcount_e8b681b1185cf5da = function(e, t) {
      r(e).count = t >>> 0;
    }, n.wbg.__wbg_setcullmode_74bc6eaab528c94b = function(e, t) {
      r(e).cullMode = lt[t];
    }, n.wbg.__wbg_setdepthbias_cdcc35c6971d19cd = function(e, t) {
      r(e).depthBias = t;
    }, n.wbg.__wbg_setdepthbiasclamp_57801e26f66496d9 = function(e, t) {
      r(e).depthBiasClamp = t;
    }, n.wbg.__wbg_setdepthbiasslopescale_81699f807bd5a647 = function(e, t) {
      r(e).depthBiasSlopeScale = t;
    }, n.wbg.__wbg_setdepthclearvalue_9801aa9eff7645df = function(e, t) {
      r(e).depthClearValue = t;
    }, n.wbg.__wbg_setdepthcompare_53d249a136855bd8 = function(e, t) {
      r(e).depthCompare = ge[t];
    }, n.wbg.__wbg_setdepthfailop_2e4767995acd4c0a = function(e, t) {
      r(e).depthFailOp = le[t];
    }, n.wbg.__wbg_setdepthloadop_af0b0f05e83f6571 = function(e, t) {
      r(e).depthLoadOp = de[t];
    }, n.wbg.__wbg_setdepthorarraylayers_5d480fc05509ea0c = function(e, t) {
      r(e).depthOrArrayLayers = t >>> 0;
    }, n.wbg.__wbg_setdepthreadonly_a7b7224074e024d3 = function(e, t) {
      r(e).depthReadOnly = t !== 0;
    }, n.wbg.__wbg_setdepthstencil_2bb2fcea55783858 = function(e, t) {
      r(e).depthStencil = r(t);
    }, n.wbg.__wbg_setdepthstencilattachment_dcbd5b74e4350e16 = function(e, t) {
      r(e).depthStencilAttachment = r(t);
    }, n.wbg.__wbg_setdepthstoreop_40dfd99c7e42f894 = function(e, t) {
      r(e).depthStoreOp = we[t];
    }, n.wbg.__wbg_setdepthwriteenabled_4368a2fe5d258cb0 = function(e, t) {
      r(e).depthWriteEnabled = t !== 0;
    }, n.wbg.__wbg_setdevice_d372d6aa06f20cae = function(e, t) {
      r(e).device = r(t);
    }, n.wbg.__wbg_setdimension_268b2b7bfc3e2bb8 = function(e, t) {
      r(e).dimension = At[t];
    }, n.wbg.__wbg_setdimension_359b229ea1b67a77 = function(e, t) {
      r(e).dimension = pe[t];
    }, n.wbg.__wbg_setdstfactor_96e73b9eaedeb23e = function(e, t) {
      r(e).dstFactor = Re[t];
    }, n.wbg.__wbg_setendofpasswriteindex_71e7659a9d2a9d60 = function(e, t) {
      r(e).endOfPassWriteIndex = t >>> 0;
    }, n.wbg.__wbg_setentries_5941f16619f54d42 = function(e, t) {
      r(e).entries = r(t);
    }, n.wbg.__wbg_setentries_97a6ad10aa7fa4d1 = function(e, t) {
      r(e).entries = r(t);
    }, n.wbg.__wbg_setentrypoint_a858879f63ec2236 = function(e, t, o) {
      r(e).entryPoint = l(t, o);
    }, n.wbg.__wbg_setentrypoint_a8ce0b22c20548b0 = function(e, t, o) {
      r(e).entryPoint = l(t, o);
    }, n.wbg.__wbg_setfailop_d55bda42958efa98 = function(e, t) {
      r(e).failOp = le[t];
    }, n.wbg.__wbg_setformat_69ba449c0e080708 = function(e, t) {
      r(e).format = j[t];
    }, n.wbg.__wbg_setformat_713b9e90b13df6aa = function(e, t) {
      r(e).format = Ct[t];
    }, n.wbg.__wbg_setformat_76bcf93126fcdc9d = function(e, t) {
      r(e).format = j[t];
    }, n.wbg.__wbg_setformat_970299d3f84a8f20 = function(e, t) {
      r(e).format = j[t];
    }, n.wbg.__wbg_setformat_a8a60feb127f0971 = function(e, t) {
      r(e).format = j[t];
    }, n.wbg.__wbg_setformat_beb33029aea4cf8e = function(e, t) {
      r(e).format = j[t];
    }, n.wbg.__wbg_setformat_f6ec428901712514 = function(e, t) {
      r(e).format = j[t];
    }, n.wbg.__wbg_setfragment_0f23dfb67b3e84ab = function(e, t) {
      r(e).fragment = r(t);
    }, n.wbg.__wbg_setfrontface_c80337acd997f8c6 = function(e, t) {
      r(e).frontFace = wt[t];
    }, n.wbg.__wbg_setg_7eb6b5e67456a09e = function(e, t) {
      r(e).g = t;
    }, n.wbg.__wbg_sethasdynamicoffset_b34dfdba692a7959 = function(e, t) {
      r(e).hasDynamicOffset = t !== 0;
    }, n.wbg.__wbg_setheaders_834c0bdb6a8949ad = function(e, t) {
      r(e).headers = r(t);
    }, n.wbg.__wbg_setheight_433680330c9420c3 = function(e, t) {
      r(e).height = t >>> 0;
    }, n.wbg.__wbg_setheight_a7439239ff109215 = function(e, t) {
      r(e).height = t >>> 0;
    }, n.wbg.__wbg_setheight_da683a33fa99843c = function(e, t) {
      r(e).height = t >>> 0;
    }, n.wbg.__wbg_setlabel_1df8805b2aad72d7 = function(e, t, o) {
      r(e).label = l(t, o);
    }, n.wbg.__wbg_setlabel_460a52030d604dd7 = function(e, t, o) {
      r(e).label = l(t, o);
    }, n.wbg.__wbg_setlabel_57008c2e11276b5e = function(e, t, o) {
      r(e).label = l(t, o);
    }, n.wbg.__wbg_setlabel_66db708c47a585b2 = function(e, t, o) {
      r(e).label = l(t, o);
    }, n.wbg.__wbg_setlabel_68cd87490e02e1de = function(e, t, o) {
      r(e).label = l(t, o);
    }, n.wbg.__wbg_setlabel_76b058f0224eb49e = function(e, t, o) {
      r(e).label = l(t, o);
    }, n.wbg.__wbg_setlabel_89c327fa94d8076b = function(e, t, o) {
      r(e).label = l(t, o);
    }, n.wbg.__wbg_setlabel_969d6f8279c74456 = function(e, t, o) {
      r(e).label = l(t, o);
    }, n.wbg.__wbg_setlabel_a0c41069e355431e = function(e, t, o) {
      r(e).label = l(t, o);
    }, n.wbg.__wbg_setlabel_c14214ffbf6e5c4a = function(e, t, o) {
      r(e).label = l(t, o);
    }, n.wbg.__wbg_setlabel_ca2c132e2b646244 = function(e, t, o) {
      r(e).label = l(t, o);
    }, n.wbg.__wbg_setlabel_e6fab993e10f1dd3 = function(e, t, o) {
      r(e).label = l(t, o);
    }, n.wbg.__wbg_setlabel_f9a45e9ef445b781 = function(e, t, o) {
      r(e).label = l(t, o);
    }, n.wbg.__wbg_setlayout_67a29edc6247c437 = function(e, t) {
      r(e).layout = r(t);
    }, n.wbg.__wbg_setlayout_758d30edbd6ea91c = function(e, t) {
      r(e).layout = r(t);
    }, n.wbg.__wbg_setloadop_5644a3bf70f4f76c = function(e, t) {
      r(e).loadOp = de[t];
    }, n.wbg.__wbg_setlodmaxclamp_d80060a9922f9fe3 = function(e, t) {
      r(e).lodMaxClamp = t;
    }, n.wbg.__wbg_setlodminclamp_bee469ae69d038f0 = function(e, t) {
      r(e).lodMinClamp = t;
    }, n.wbg.__wbg_setmagfilter_f50646cfdc01700d = function(e, t) {
      r(e).magFilter = $e[t];
    }, n.wbg.__wbg_setmappedatcreation_0dc5796d4e90ab4b = function(e, t) {
      r(e).mappedAtCreation = t !== 0;
    }, n.wbg.__wbg_setmask_800b15ad78613be8 = function(e, t) {
      r(e).mask = t >>> 0;
    }, n.wbg.__wbg_setmaxanisotropy_83ac2a8bef9f9ec8 = function(e, t) {
      r(e).maxAnisotropy = t;
    }, n.wbg.__wbg_setmethod_3c5280fe5d890842 = function(e, t, o) {
      r(e).method = l(t, o);
    }, n.wbg.__wbg_setminbindingsize_20ca594cd6d93818 = function(e, t) {
      r(e).minBindingSize = t;
    }, n.wbg.__wbg_setminfilter_8ffc9e1ac6b4149f = function(e, t) {
      r(e).minFilter = $e[t];
    }, n.wbg.__wbg_setmiplevel_6f507098915add77 = function(e, t) {
      r(e).mipLevel = t >>> 0;
    }, n.wbg.__wbg_setmiplevelcount_5e59806cbcf116e9 = function(e, t) {
      r(e).mipLevelCount = t >>> 0;
    }, n.wbg.__wbg_setmiplevelcount_f896fe8cbb669df2 = function(e, t) {
      r(e).mipLevelCount = t >>> 0;
    }, n.wbg.__wbg_setmipmapfilter_037575f2e647f024 = function(e, t) {
      r(e).mipmapFilter = mt[t];
    }, n.wbg.__wbg_setmodule_4c73bb35cb0beb0b = function(e, t) {
      r(e).module = r(t);
    }, n.wbg.__wbg_setmodule_ca21130b3f66ea5d = function(e, t) {
      r(e).module = r(t);
    }, n.wbg.__wbg_setmultisample_4f57dcaa4144a62f = function(e, t) {
      r(e).multisample = r(t);
    }, n.wbg.__wbg_setmultisampled_0bb9fc1b577bf11a = function(e, t) {
      r(e).multisampled = t !== 0;
    }, n.wbg.__wbg_setoffset_67ee100819c122f2 = function(e, t) {
      r(e).offset = t;
    }, n.wbg.__wbg_setoffset_a8194a4fcfff8910 = function(e, t) {
      r(e).offset = t;
    }, n.wbg.__wbg_setoffset_d37e5fa34e9ded2e = function(e, t) {
      r(e).offset = t;
    }, n.wbg.__wbg_setoperation_173958551af7f4f2 = function(e, t) {
      r(e).operation = bt[t];
    }, n.wbg.__wbg_setorigin_e26b73e77b3e275d = function(e, t) {
      r(e).origin = r(t);
    }, n.wbg.__wbg_setpassop_070547fd6160a00d = function(e, t) {
      r(e).passOp = le[t];
    }, n.wbg.__wbg_setpowerpreference_1f3351e5d2acf765 = function(e, t) {
      r(e).powerPreference = ht[t];
    }, n.wbg.__wbg_setprimitive_ee18492ab93953bc = function(e, t) {
      r(e).primitive = r(t);
    }, n.wbg.__wbg_setqueryset_3b14f95f9bd114db = function(e, t) {
      r(e).querySet = r(t);
    }, n.wbg.__wbg_setr_a4e2f60e3466da86 = function(e, t) {
      r(e).r = t;
    }, n.wbg.__wbg_setrequiredfeatures_fc44bc3433300ee3 = function(e, t) {
      r(e).requiredFeatures = r(t);
    }, n.wbg.__wbg_setresolvetarget_c4b519cab7eb42b7 = function(e, t) {
      r(e).resolveTarget = r(t);
    }, n.wbg.__wbg_setresource_1659f5a29a2e0541 = function(e, t) {
      r(e).resource = r(t);
    }, n.wbg.__wbg_setrowsperimage_53ed2c633b1adfcc = function(e, t) {
      r(e).rowsPerImage = t >>> 0;
    }, n.wbg.__wbg_setsamplecount_e88d044f067a2241 = function(e, t) {
      r(e).sampleCount = t >>> 0;
    }, n.wbg.__wbg_setsampler_a778272f31d31ce5 = function(e, t) {
      r(e).sampler = r(t);
    }, n.wbg.__wbg_setsampletype_c0e25b966db74174 = function(e, t) {
      r(e).sampleType = Mt[t];
    }, n.wbg.__wbg_setsearch_609451e9e712f3c6 = function(e, t, o) {
      r(e).search = l(t, o);
    }, n.wbg.__wbg_setshaderlocation_985046f48e76573f = function(e, t) {
      r(e).shaderLocation = t >>> 0;
    }, n.wbg.__wbg_setsize_23676383c9c0732f = function(e, t) {
      r(e).size = t;
    }, n.wbg.__wbg_setsize_51616eaf8209c58b = function(e, t) {
      r(e).size = r(t);
    }, n.wbg.__wbg_setsize_5878aadcd23673cf = function(e, t) {
      r(e).size = t;
    }, n.wbg.__wbg_setsrcfactor_04ce8874f1bff5a8 = function(e, t) {
      r(e).srcFactor = Re[t];
    }, n.wbg.__wbg_setstencilback_4b20ecfcd4c4816a = function(e, t) {
      r(e).stencilBack = r(t);
    }, n.wbg.__wbg_setstencilclearvalue_7ba82e1993788f37 = function(e, t) {
      r(e).stencilClearValue = t >>> 0;
    }, n.wbg.__wbg_setstencilfront_1ca3b695f7c42f6a = function(e, t) {
      r(e).stencilFront = r(t);
    }, n.wbg.__wbg_setstencilloadop_b65c60a0077315cd = function(e, t) {
      r(e).stencilLoadOp = de[t];
    }, n.wbg.__wbg_setstencilreadmask_4f5b98747141e796 = function(e, t) {
      r(e).stencilReadMask = t >>> 0;
    }, n.wbg.__wbg_setstencilreadonly_9006a99a91d198e9 = function(e, t) {
      r(e).stencilReadOnly = t !== 0;
    }, n.wbg.__wbg_setstencilstoreop_4f00c5eca345c145 = function(e, t) {
      r(e).stencilStoreOp = we[t];
    }, n.wbg.__wbg_setstencilwritemask_e37a7214d84ace99 = function(e, t) {
      r(e).stencilWriteMask = t >>> 0;
    }, n.wbg.__wbg_setstepmode_7d58d75e6547a7a6 = function(e, t) {
      r(e).stepMode = Pt[t];
    }, n.wbg.__wbg_setstoragetexture_2987339fec972d54 = function(e, t) {
      r(e).storageTexture = r(t);
    }, n.wbg.__wbg_setstoreop_c62dd050b5806095 = function(e, t) {
      r(e).storeOp = we[t];
    }, n.wbg.__wbg_setstripindexformat_3e4893749b3f00b0 = function(e, t) {
      r(e).stripIndexFormat = pt[t];
    }, n.wbg.__wbg_settargets_0ef1de33af7253a6 = function(e, t) {
      r(e).targets = r(t);
    }, n.wbg.__wbg_settexture_2553e9c3ae6f7687 = function(e, t) {
      r(e).texture = r(t);
    }, n.wbg.__wbg_settexture_f62859f817324dd1 = function(e, t) {
      r(e).texture = r(t);
    }, n.wbg.__wbg_settimestampwrites_1995524c3a31cb8f = function(e, t) {
      r(e).timestampWrites = r(t);
    }, n.wbg.__wbg_settopology_3d9b2f0ffe2e350c = function(e, t) {
      r(e).topology = yt[t];
    }, n.wbg.__wbg_settype_0b59dd5f4721c490 = function(e, t) {
      r(e).type = xt[t];
    }, n.wbg.__wbg_settype_8c8bbfab4cf7e32e = function(e, t) {
      r(e).type = gt[t];
    }, n.wbg.__wbg_setusage_44ebc3b496e60ff4 = function(e, t) {
      r(e).usage = t >>> 0;
    }, n.wbg.__wbg_setusage_4cf7b16df5617a46 = function(e, t) {
      r(e).usage = t >>> 0;
    }, n.wbg.__wbg_setusage_c45cca4a5b9f8376 = function(e, t) {
      r(e).usage = t >>> 0;
    }, n.wbg.__wbg_setusage_e58b3c3ce83fbbda = function(e, t) {
      r(e).usage = t >>> 0;
    }, n.wbg.__wbg_setvertex_6144c56d98e2314a = function(e, t) {
      r(e).vertex = r(t);
    }, n.wbg.__wbg_setview_4bc3dfcbfc8a58ba = function(e, t) {
      r(e).view = r(t);
    }, n.wbg.__wbg_setview_8d0b0055b6ef07e3 = function(e, t) {
      r(e).view = r(t);
    }, n.wbg.__wbg_setviewdimension_afac48443b8fb565 = function(e, t) {
      r(e).viewDimension = pe[t];
    }, n.wbg.__wbg_setviewdimension_f5d4b5336a27d302 = function(e, t) {
      r(e).viewDimension = pe[t];
    }, n.wbg.__wbg_setviewformats_0cfe174ac882efaf = function(e, t) {
      r(e).viewFormats = r(t);
    }, n.wbg.__wbg_setviewformats_c566feb1da7b1925 = function(e, t) {
      r(e).viewFormats = r(t);
    }, n.wbg.__wbg_setvisibility_7245f1acbedb4ff4 = function(e, t) {
      r(e).visibility = t >>> 0;
    }, n.wbg.__wbg_setwidth_056381a7176ba440 = function(e, t) {
      r(e).width = t >>> 0;
    }, n.wbg.__wbg_setwidth_660ca581e3fbe279 = function(e, t) {
      r(e).width = t >>> 0;
    }, n.wbg.__wbg_setwidth_c5fed9f5e7f0b406 = function(e, t) {
      r(e).width = t >>> 0;
    }, n.wbg.__wbg_setwritemask_c381ff702509999c = function(e, t) {
      r(e).writeMask = t >>> 0;
    }, n.wbg.__wbg_setx_6e550cba86f408f0 = function(e, t) {
      r(e).x = t >>> 0;
    }, n.wbg.__wbg_sety_16ff3ff771600f8c = function(e, t) {
      r(e).y = t >>> 0;
    }, n.wbg.__wbg_setz_b2c09b24c996ee06 = function(e, t) {
      r(e).z = t >>> 0;
    }, n.wbg.__wbg_stack_0ed75d68575b0f3c = function(e, t) {
      const o = r(t).stack, _ = L(o, i.__wbindgen_export_2, i.__wbindgen_export_3), a = P;
      m().setInt32(e + 4 * 1, a, true), m().setInt32(e + 4 * 0, _, true);
    }, n.wbg.__wbg_static_accessor_GLOBAL_88a902d13a557d07 = function() {
      const e = typeof global > "u" ? null : global;
      return O(e) ? 0 : b(e);
    }, n.wbg.__wbg_static_accessor_GLOBAL_THIS_56578be7e9f832b0 = function() {
      const e = typeof globalThis > "u" ? null : globalThis;
      return O(e) ? 0 : b(e);
    }, n.wbg.__wbg_static_accessor_SELF_37c5d418e4bf5819 = function() {
      const e = typeof self > "u" ? null : self;
      return O(e) ? 0 : b(e);
    }, n.wbg.__wbg_static_accessor_WINDOW_5de37043a91a9c40 = function() {
      const e = typeof window > "u" ? null : window;
      return O(e) ? 0 : b(e);
    }, n.wbg.__wbg_status_f6360336ca686bf0 = function(e) {
      return r(e).status;
    }, n.wbg.__wbg_submit_252766c4e0945cee = function(e, t) {
      r(e).submit(r(t));
    }, n.wbg.__wbg_text_7805bea50de2af49 = function() {
      return A(function(e) {
        const t = r(e).text();
        return b(t);
      }, arguments);
    }, n.wbg.__wbg_then_44b73946d2fb3e7d = function(e, t) {
      const o = r(e).then(r(t));
      return b(o);
    }, n.wbg.__wbg_then_48b406749878a531 = function(e, t, o) {
      const _ = r(e).then(r(t), r(o));
      return b(_);
    }, n.wbg.__wbg_toString_5285597960676b7b = function(e) {
      const t = r(e).toString();
      return b(t);
    }, n.wbg.__wbg_toString_c813bbd34d063839 = function(e) {
      const t = r(e).toString();
      return b(t);
    }, n.wbg.__wbg_url_8f9653b899456042 = function(e, t) {
      const o = r(t).url, _ = L(o, i.__wbindgen_export_2, i.__wbindgen_export_3), a = P;
      m().setInt32(e + 4 * 1, a, true), m().setInt32(e + 4 * 0, _, true);
    }, n.wbg.__wbg_width_5dde457d606ba683 = function(e) {
      return r(e).width;
    }, n.wbg.__wbg_writeBuffer_3193eaacefdcf39a = function() {
      return A(function(e, t, o, _, a, s) {
        r(e).writeBuffer(r(t), o, r(_), a, s);
      }, arguments);
    }, n.wbg.__wbg_writeTexture_cd7877c213ee5704 = function() {
      return A(function(e, t, o, _, a) {
        r(e).writeTexture(r(t), r(o), r(_), r(a));
      }, arguments);
    }, n.wbg.__wbindgen_cb_drop = function(e) {
      const t = ee(e).original;
      return t.cnt-- == 1 ? (t.a = 0, true) : false;
    }, n.wbg.__wbindgen_closure_wrapper2376 = function(e, t, o) {
      const _ = Oe(e, t, 194, ut);
      return b(_);
    }, n.wbg.__wbindgen_closure_wrapper2409 = function(e, t, o) {
      const _ = Oe(e, t, 203, ft);
      return b(_);
    }, n.wbg.__wbindgen_debug_string = function(e, t) {
      const o = xe(r(t)), _ = L(o, i.__wbindgen_export_2, i.__wbindgen_export_3), a = P;
      m().setInt32(e + 4 * 1, a, true), m().setInt32(e + 4 * 0, _, true);
    }, n.wbg.__wbindgen_is_function = function(e) {
      return typeof r(e) == "function";
    }, n.wbg.__wbindgen_is_null = function(e) {
      return r(e) === null;
    }, n.wbg.__wbindgen_is_undefined = function(e) {
      return r(e) === void 0;
    }, n.wbg.__wbindgen_memory = function() {
      const e = i.memory;
      return b(e);
    }, n.wbg.__wbindgen_number_get = function(e, t) {
      const o = r(t), _ = typeof o == "number" ? o : void 0;
      m().setFloat64(e + 8 * 1, O(_) ? 0 : _, true), m().setInt32(e + 4 * 0, !O(_), true);
    }, n.wbg.__wbindgen_number_new = function(e) {
      return b(e);
    }, n.wbg.__wbindgen_object_clone_ref = function(e) {
      const t = r(e);
      return b(t);
    }, n.wbg.__wbindgen_object_drop_ref = function(e) {
      ee(e);
    }, n.wbg.__wbindgen_string_get = function(e, t) {
      const o = r(t), _ = typeof o == "string" ? o : void 0;
      var a = O(_) ? 0 : L(_, i.__wbindgen_export_2, i.__wbindgen_export_3), s = P;
      m().setInt32(e + 4 * 1, s, true), m().setInt32(e + 4 * 0, a, true);
    }, n.wbg.__wbindgen_string_new = function(e, t) {
      const o = l(e, t);
      return b(o);
    }, n.wbg.__wbindgen_throw = function(e, t) {
      throw new Error(l(e, t));
    }, n;
  }
  function It(n, e) {
    return i = n.exports, Ne.__wbindgen_wasm_module = e, N = null, J = null, K = null, i;
  }
  async function Ne(n) {
    if (i !== void 0) return i;
    typeof n < "u" && (Object.getPrototypeOf(n) === Object.prototype ? { module_or_path: n } = n : console.warn("using deprecated parameters for the initialization function; pass a single object instead")), typeof n > "u" && (n = new URL("/infinite-chat/assets/infinite_chat_wasm_bg-Cy_JI15k.wasm", import.meta.url));
    const e = Et();
    (typeof n == "string" || typeof Request == "function" && n instanceof Request || typeof URL == "function" && n instanceof URL) && (n = fetch(n));
    const { instance: t, module: o } = await Tt(await n, e);
    return It(t, o);
  }
  let ue = false, _e = null;
  const Ve = /* @__PURE__ */ new Map();
  let ve = 0;
  Ft = function() {
    return ue;
  };
  function Xe(n, e) {
    if (!ue || ve === 0) return null;
    const t = Ve.get(n);
    return t == null ? null : t * e / ve;
  }
  Bt = function(n, e = "/infinite-chat/fonts/lxgw-msdf") {
    if (ue) return Promise.resolve();
    if (_e) return _e;
    const t = e.replace(/[^/]+$/, "");
    return _e = (async () => {
      const o = await fetch(`${e}.json`).then((c) => c.json()), _ = o.chars;
      ve = o.info.size;
      const a = new Uint32Array(_.length), s = new Float32Array(_.length * 7);
      _.forEach((c, f) => {
        a[f] = c.id, s.set([
          c.x,
          c.y,
          c.width,
          c.height,
          c.xoffset,
          c.yoffset,
          c.page
        ], f * 7), Ve.set(c.id, c.xadvance);
      });
      const u = [];
      for (const c of o.pages) {
        const f = await fetch(`${t}${c}`).then((S) => S.blob()), g = await createImageBitmap(f), h = new OffscreenCanvas(g.width, g.height).getContext("2d");
        if (!h) throw new Error("MSDF \u89E3\u7801:\u65E0 2D \u4E0A\u4E0B\u6587");
        h.drawImage(g, 0, 0);
        const M = h.getImageData(0, 0, g.width, g.height).data;
        u.push(new Uint8Array(M.buffer.slice(0)));
      }
      n.load_msdf({
        atlasW: o.common.scaleW,
        atlasH: o.common.scaleH,
        fontSize: o.info.size,
        ids: a,
        cells: s,
        pixels: u
      }), ue = true, console.info(`[msdf] loaded ${_.length} glyphs / ${o.pages.length} pages`);
    })(), _e;
  };
  let ae = null, We = false;
  function Lt(n, e = "/infinite-chat/fonts/katex-msdf") {
    if (We) return Promise.resolve();
    if (ae) return ae;
    const t = e.replace(/[^/]+$/, "");
    return ae = (async () => {
      const o = await fetch(`${e}.json`).then((c) => c.json()), _ = o.chars, a = new Uint32Array(_.length), s = new Float32Array(_.length * 7);
      _.forEach((c, f) => {
        a[f] = c.id, s.set([
          c.x,
          c.y,
          c.width,
          c.height,
          c.xoffset,
          c.yoffset,
          c.page
        ], f * 7);
      });
      const u = [];
      for (const c of o.pages) {
        const f = await fetch(`${t}${c}`).then((S) => S.blob()), g = await createImageBitmap(f), h = new OffscreenCanvas(g.width, g.height).getContext("2d");
        if (!h) throw new Error("MSDF \u89E3\u7801:\u65E0 2D \u4E0A\u4E0B\u6587");
        h.drawImage(g, 0, 0);
        const M = h.getImageData(0, 0, g.width, g.height).data;
        u.push(new Uint8Array(M.buffer.slice(0)));
      }
      n.load_msdf({
        atlasW: o.common.scaleW,
        atlasH: o.common.scaleH,
        fontSize: o.info.size,
        ids: a,
        cells: s,
        pixels: u
      }), We = true, console.info(`[math-msdf] loaded ${_.length} glyphs`);
    })(), ae;
  }
  let ce, He;
  Ot = Object.freeze(Object.defineProperty({
    __proto__: null,
    loadMathMsdf: Lt,
    loadMsdf: Bt,
    msdfAdvancePx: Xe,
    msdfLoaded: Ft
  }, Symbol.toStringTag, {
    value: "Module"
  }));
  _n = [
    [
      "code_bg",
      "code bg",
      [
        0.1,
        0.11,
        0.16,
        0.75
      ]
    ],
    [
      "code_chip",
      "code chip",
      [
        0.18,
        0.19,
        0.26,
        0.7
      ]
    ],
    [
      "code_border",
      "code border",
      [
        0.32,
        0.36,
        0.46,
        0.85
      ]
    ],
    [
      "quote_bar",
      "quote bar",
      [
        0.42,
        0.46,
        0.56,
        0.9
      ]
    ],
    [
      "head_rule",
      "head rule",
      [
        0.24,
        0.27,
        0.33,
        0.9
      ]
    ],
    [
      "hr_rule",
      "hr rule",
      [
        0.82,
        0.86,
        0.94,
        1
      ]
    ],
    [
      "selection",
      "selection",
      [
        0.26,
        0.45,
        0.92,
        0.4
      ]
    ],
    [
      "card_bg",
      "card bg",
      [
        0.14,
        0.16,
        0.21,
        0.55
      ]
    ],
    [
      "card_border",
      "card border",
      [
        0.3,
        0.34,
        0.44,
        0.7
      ]
    ],
    [
      "diff_add_bg",
      "diff add",
      [
        0.22,
        0.45,
        0.27,
        0.35
      ]
    ],
    [
      "diff_del_bg",
      "diff del",
      [
        0.5,
        0.22,
        0.24,
        0.35
      ]
    ]
  ];
  ce = {
    table: {
      vAlign: "center",
      hAlign: "auto"
    },
    tableRender: {
      lineColor: [
        0.26,
        0.29,
        0.36,
        0.9
      ],
      headerFill: [
        0.16,
        0.18,
        0.24,
        0.6
      ],
      aoColor: [
        1,
        1,
        1
      ],
      lineW: 1,
      ao: 0.12,
      aoWidth: 10,
      radius: 4
    },
    theme: {}
  };
  He = "infinite-chat.styleConfig";
  function ze(n) {
    return JSON.parse(JSON.stringify(n));
  }
  let Se = Rt();
  function Rt() {
    try {
      const n = localStorage.getItem(He);
      if (!n) return ze(ce);
      const e = JSON.parse(n);
      return {
        table: {
          ...ce.table,
          ...e.table ?? {}
        },
        tableRender: {
          ...ce.tableRender,
          ...e.tableRender ?? {}
        },
        theme: {
          ...e.theme ?? {}
        }
      };
    } catch {
      return ze(ce);
    }
  }
  $t = function() {
    return Se;
  };
  an = function(n) {
    Se = n;
    try {
      localStorage.setItem(He, JSON.stringify(Se));
    } catch {
    }
  };
  const Pe = typeof window < "u" && window.devicePixelRatio || 1, Dt = 16, y = Math.round(Dt * Pe), $ = Math.ceil(y * 1.4), F = 128, te = 8, Wt = F - 2 * te, fe = {
    system: {
      sans: 'ui-sans-serif, system-ui, -apple-system, "Segoe UI", "PingFang SC", "Microsoft YaHei", "Noto Sans CJK SC", sans-serif',
      mono: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Liberation Mono", "Noto Sans Mono CJK SC", monospace'
    },
    serif: {
      sans: 'ui-serif, Georgia, Cambria, "Times New Roman", "Songti SC", "SimSun", "Noto Serif CJK SC", serif',
      mono: 'ui-monospace, SFMono-Regular, Menlo, "Noto Sans Mono CJK SC", monospace'
    },
    rounded: {
      sans: '"SF Pro Rounded", ui-rounded, "Hiragino Maru Gothic ProN", "Yuanti SC", system-ui, sans-serif',
      mono: 'ui-monospace, SFMono-Regular, Menlo, "Noto Sans Mono CJK SC", monospace'
    },
    mono: {
      sans: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Noto Sans Mono CJK SC", monospace',
      mono: 'ui-monospace, SFMono-Regular, "SF Mono", Menlo, Consolas, "Noto Sans Mono CJK SC", monospace'
    }
  };
  let Ae = "system", U = fe.system.sans, V = fe.system.mono;
  cn = function(n) {
    const e = fe[n];
    return !e || n === Ae ? false : (Ae = n, U = e.sans, V = e.mono, true);
  };
  sn = function() {
    return Ae;
  };
  let Me = "auto";
  un = function(n) {
    Me = n;
  };
  function zt() {
    return Me === "auto" || Me === "msdf";
  }
  fn = function() {
    return Object.keys(fe);
  };
  function Ke(n) {
    switch (n) {
      case 1:
        return `bold ${y}px ${U}`;
      case 2:
        return `italic ${y}px ${U}`;
      case 3:
        return `bold italic ${y}px ${U}`;
      case 4:
      case 5:
      case 43:
      case 44:
      case 45:
      case 46:
      case 47:
      case 48:
      case 49:
      case 50:
        return `${y}px ${V}`;
      case 6:
      case 10:
      case 11:
      case 12:
      case 13:
      case 14:
        return `bold ${y}px ${U}`;
      case 8:
        return `italic ${y}px ${U}`;
      case 16:
        return `bold ${y}px ${U}`;
      case 17:
      case 18:
        return `${y}px ${V}`;
      case 19:
        return `bold ${y}px ${V}`;
      case 20:
        return `italic ${y}px ${V}`;
      case 21:
        return `${y}px ${V}`;
      case 26:
        return `${y}px KaTeX_Main`;
      case 27:
        return `bold ${y}px KaTeX_Main`;
      case 28:
        return `italic ${y}px KaTeX_Main`;
      case 29:
        return `bold italic ${y}px KaTeX_Main`;
      case 30:
        return `italic ${y}px KaTeX_Math`;
      case 31:
        return `${y}px KaTeX_AMS`;
      case 32:
        return `${y}px KaTeX_Size1`;
      case 33:
        return `${y}px KaTeX_Size2`;
      case 34:
        return `${y}px KaTeX_Size3`;
      case 35:
        return `${y}px KaTeX_Size4`;
      case 36:
        return `${y}px KaTeX_Caligraphic`;
      case 37:
        return `${y}px KaTeX_Fraktur`;
      case 38:
        return `${y}px KaTeX_SansSerif`;
      case 39:
        return `${y}px KaTeX_Script`;
      case 40:
        return `${y}px KaTeX_Typewriter`;
      default:
        return `${y}px ${U}`;
    }
  }
  function Gt(n) {
    return n === 5 || n >= 43 && n <= 50;
  }
  function Ce(n) {
    switch (n) {
      case 6:
        return 2;
      case 10:
        return 1.6;
      case 11:
        return 1.3;
      case 12:
        return 1.15;
      case 13:
        return 1;
      case 14:
        return 0.9;
      case 24:
        return 0.7;
      case 25:
        return 0.85;
      default:
        return 1;
    }
  }
  const Ut = new Intl.Segmenter(void 0, {
    granularity: "grapheme"
  });
  let me = null;
  function qt() {
    if (!me) {
      const n = new OffscreenCanvas(8, 8).getContext("2d");
      if (!n) throw new Error("\u65E0\u6CD5\u521B\u5EFA 2D \u6D4B\u91CF\u4E0A\u4E0B\u6587");
      me = n;
    }
    return me;
  }
  function jt(n, e) {
    if (zt()) {
      const o = [
        ...n
      ];
      if (o.length === 1) {
        const _ = Xe(o[0].codePointAt(0) ?? 0, y);
        if (_ != null) return _ * Ce(e);
      }
    }
    const t = qt();
    return t.font = Ke(e), Math.max(1, t.measureText(n).width) * Ce(e);
  }
  function Nt(n) {
    const e = n.codePointAt(0) ?? 0;
    return e >= 19968 && e <= 40959 || e >= 12352 && e <= 12543 || e >= 44032 && e <= 55203 || e >= 65280 && e <= 65519 || e >= 12288 && e <= 12351;
  }
  function Vt(n) {
    return /\s/.test(n);
  }
  const Xt = "\u3002\uFF0C\u3001\uFF1B\uFF1A\uFF1F\uFF01\uFF09\u300D\u300F\u3011\u300B\u3009\u201D\u2019%\u2026\xB7.,;:?!)]}";
  function Ht(n) {
    return Xt.includes(n);
  }
  function Kt(n) {
    return n >= 17 && n <= 21;
  }
  const se = Math.round(8 * Pe), he = Math.max(1, Math.round(Pe));
  function Jt(n, e, t, o) {
    const _ = [];
    let a = e, s = 0, u = e;
    for (; u < t; ) {
      if (n[u].nl) {
        _.push([
          a,
          u
        ]), u++, a = u, s = 0;
        continue;
      }
      let c = u, f = 0;
      if (n[u].cjk) c = u + 1, f = n[u].adv;
      else for (; c < t && !n[c].cjk && !n[c].nl && (f += n[c].adv, c++, !n[c - 1].space); ) ;
      s + f > o && s > 0 && (_.push([
        a,
        u
      ]), a = u, s = 0), s += f, u = c;
    }
    return (a < t || _.length === 0) && _.push([
      a,
      t
    ]), _;
  }
  function Yt(n, e, t, o, _, a) {
    const s = t.rows.length, u = Math.max(t.aligns.length, ...t.rows.map((p) => p.length)), c = (p, C) => {
      const k = t.rows[p][C];
      return k ? [
        o[k[0]],
        o[k[1]]
      ] : [
        0,
        0
      ];
    }, f = (p, C) => {
      let k = 0;
      for (let T = p; T < C; T++) n[T].nl || (k += n[T].adv);
      return k;
    }, g = new Array(u).fill(0);
    for (let p = 0; p < s; p++) for (let C = 0; C < t.rows[p].length; C++) {
      const [k, T] = c(p, C);
      g[C] = Math.max(g[C], f(k, T));
    }
    const d = u * se * 2 + Math.max(0, u - 1) * he, h = a - d, M = g.reduce((p, C) => p + C, 0);
    if (M > h && h > 0) {
      const p = Math.round(4 * y), C = g.map((I) => I / M * h);
      let k = 0, T = 0;
      for (const I of C) I < p ? k += p - I : T += I - p;
      for (let I = 0; I < u; I++) C[I] < p ? g[I] = p : g[I] = k <= T && T > 0 ? C[I] - (C[I] - p) / T * k : C[I];
    }
    const S = new Array(u).fill(0);
    for (let p = 1; p < u; p++) S[p] = S[p - 1] + g[p - 1] + se * 2 + he;
    let w = _;
    const x = [];
    for (let p = 0; p < s; p++) {
      x.push(w);
      const C = [];
      let k = 1;
      for (let R = 0; R < t.rows[p].length; R++) {
        const [re, X] = c(p, R), H = Jt(n, re, X, g[R]);
        C.push(H), H.length > k && (k = H.length);
      }
      const T = $t().table, I = T.hAlign === "auto" ? null : T.hAlign === "right" ? 2 : T.hAlign === "center" ? 1 : 0;
      for (let R = 0; R < t.rows[p].length; R++) {
        const re = I ?? t.aligns[R] ?? 0, X = C[R], H = (X.length - 1) * $ + y, ke = Math.max(0, k * $ - H), et = T.vAlign === "top" ? 0 : T.vAlign === "bottom" ? ke : ke / 2;
        for (let oe = 0; oe < X.length; oe++) {
          const [Te, Ee] = X[oe], Ie = g[R] - f(Te, Ee), tt = re === 2 ? Ie : re === 1 ? Ie / 2 : 0;
          let Fe = S[R] + se + Math.max(0, tt);
          const nt = w + et + oe * $;
          for (let q = Te; q < Ee; q++) {
            const W = n[q];
            e[q * 4] = Fe - W.off, e[q * 4 + 1] = nt + ($ - W.cell) * 0.5 - W.off, e[q * 4 + 2] = W.nl ? 0 : W.cell, e[q * 4 + 3] = W.nl ? 0 : W.cell, W.nl || (Fe += W.adv);
          }
        }
      }
      w += k * $;
    }
    const E = [];
    for (let p = 1; p < u; p++) E.push(S[p] - he * 0.5);
    const v = x.slice(1), B = S[u - 1] + g[u - 1] + 2 * se, D = s >= 2 ? x[1] : _, Qe = {
      x: 0,
      y: _,
      w: B,
      h: w - _,
      headerBottom: D,
      cols: E,
      rows: v
    };
    return {
      height: w - _,
      panel: Qe
    };
  }
  function Zt(n) {
    const e = [];
    for (const t of n) e.push(t.x, t.y, t.w, t.h, t.headerBottom, t.cols.length, t.rows.length, ...t.cols, ...t.rows);
    return new Float32Array(e);
  }
  function Je(n, e, t, o) {
    const _ = y / Wt, a = [], s = [];
    for (let w = 0; w < n.length; w++) {
      s.push(a.length);
      const x = e[w] ?? 0, E = Ce(x);
      for (const { segment: v } of Ut.segment(n[w])) {
        const B = v === `
`;
        a.push({
          cluster: v,
          role: x,
          adv: B ? 0 : jt(v, x),
          cell: F * _ * E,
          off: te * _ * E,
          lineH: Math.ceil($ * E),
          nl: B,
          cjk: !B && Nt(v),
          space: !B && Vt(v),
          inTable: false
        });
      }
    }
    s.push(a.length);
    const u = /* @__PURE__ */ new Map();
    if (o) for (const w of o) {
      if (!w.rows.length || !w.rows[0].length) continue;
      const x = s[w.rows[0][0][0]];
      let E = x;
      for (const v of w.rows) for (const [, B] of v) E = Math.max(E, s[B]);
      u.set(x, {
        region: w,
        gEnd: E
      });
    }
    {
      let w = 0;
      for (let x = 0; x <= a.length; x++) if (x === a.length || a[x].nl) {
        let E = false;
        for (let v = w; v < x; v++) if (Kt(a[v].role)) {
          E = true;
          break;
        }
        if (E) for (let v = w; v < x; v++) a[v].inTable = true;
        w = x + 1;
      }
    }
    const c = new Float32Array(a.length * 4);
    let f = 0, g = 0, d = $;
    const h = (w, x) => {
      c[x * 4] = f - w.off, c[x * 4 + 1] = g + (w.lineH - w.cell) * 0.5 - w.off, c[x * 4 + 2] = w.cell, c[x * 4 + 3] = w.cell, f += w.adv, w.lineH > d && (d = w.lineH);
    }, M = [];
    let S = 0;
    for (; S < a.length; ) {
      const w = u.get(S);
      if (w) {
        const D = Yt(a, c, w.region, s, g, t);
        g += D.height, M.push(D.panel), f = 0, d = $, S = w.gEnd;
        continue;
      }
      const x = a[S];
      if (x.nl) {
        c[S * 4] = f - x.off, c[S * 4 + 1] = g - x.off, c[S * 4 + 2] = 0, c[S * 4 + 3] = 0, f = 0, g += d, d = $, S++;
        continue;
      }
      const E = x.inTable || Gt(x.role);
      let v = S, B = 0;
      if (E) for (; v < a.length && !a[v].nl; ) B += a[v].adv, v++;
      else if (x.cjk) v = S + 1, B = x.adv;
      else for (; v < a.length && !a[v].nl && !a[v].cjk && (B += a[v].adv, v++, !a[v - 1].space); ) ;
      !E && f + B > t && f > 0 && !Ht(x.cluster) && (f = 0, g += d, d = $);
      for (let D = S; D < v; D++) h(a[D], D);
      S = v;
    }
    return M.length > 0 ? {
      positions: c,
      tables: Zt(M)
    } : c;
  }
  const Y = /* @__PURE__ */ new Map(), Qt = 4096;
  let Ye = 0, Ze = 0;
  function en(n, e, t) {
    if (n.length === 0) return new Float32Array([
      0,
      0
    ]);
    const o = `${n.join("")}${Array.from(e).join(",")}@${Math.round(t)}`, _ = Y.get(o);
    if (_) return Ye++, _;
    Ze++;
    const a = Je(n, e, t), s = a instanceof Float32Array ? a : a.positions;
    let u = 0, c = 0;
    for (let g = 0; g + 3 < s.length; g += 4) {
      const d = s[g + 2];
      d > 0 && (u = Math.max(u, s[g] + d)), c = Math.max(c, s[g + 1] + s[g + 3]);
    }
    const f = new Float32Array([
      Math.min(u, t),
      c
    ]);
    return Y.size >= Qt && Y.clear(), Y.set(o, f), f;
  }
  bn = function() {
    return {
      hits: Ye,
      misses: Ze,
      size: Y.size
    };
  };
  const tn = 8, ne = 1e20;
  let ye = null;
  function nn() {
    if (!ye) {
      const n = new OffscreenCanvas(F, F).getContext("2d", {
        willReadFrequently: true
      });
      if (!n) throw new Error("\u65E0\u6CD5\u521B\u5EFA SDF \u5149\u6805\u4E0A\u4E0B\u6587");
      ye = n;
    }
    return ye;
  }
  function Ge(n, e, t, o) {
    const _ = new Float64Array(o), a = new Int32Array(o), s = new Float64Array(o + 1);
    for (let c = 0; c < o; c++) _[c] = n[e + c * t];
    a[0] = 0, s[0] = -ne, s[1] = ne;
    let u = 0;
    for (let c = 1; c < o; c++) {
      let f = (_[c] + c * c - (_[a[u]] + a[u] * a[u])) / (2 * c - 2 * a[u]);
      for (; f <= s[u]; ) u--, f = (_[c] + c * c - (_[a[u]] + a[u] * a[u])) / (2 * c - 2 * a[u]);
      u++, a[u] = c, s[u] = f, s[u + 1] = ne;
    }
    u = 0;
    for (let c = 0; c < o; c++) {
      for (; s[u + 1] < c; ) u++;
      const f = c - a[u];
      n[e + c * t] = _[a[u]] + f * f;
    }
  }
  function Ue(n, e, t) {
    for (let o = 0; o < e; o++) Ge(n, o, e, t);
    for (let o = 0; o < t; o++) Ge(n, o * e, 1, e);
  }
  function rn(n, e, t = 1) {
    const o = nn();
    o.clearRect(0, 0, F, F);
    const _ = F - 2 * te;
    o.font = Ke(e).replace(/^\s*(bold |italic )*\d+px/, `$1${_}px`), o.textBaseline = "top", o.fillStyle = "#ffffff", o.fillText(n, te, te);
    const a = o.getImageData(0, 0, F, F).data, s = F * F;
    if (t === 3) return new Uint8Array(a);
    if (t === 0) {
      const g = new Uint8Array(s * 4);
      for (let d = 0; d < s; d++) {
        const h = a[d * 4 + 3];
        g[d * 4] = h, g[d * 4 + 1] = h, g[d * 4 + 2] = h, g[d * 4 + 3] = 255;
      }
      return g;
    }
    const u = new Float64Array(s), c = new Float64Array(s);
    for (let g = 0; g < s; g++) {
      const d = a[g * 4 + 3] / 255;
      if (d === 1) u[g] = 0, c[g] = ne;
      else if (d === 0) u[g] = ne, c[g] = 0;
      else {
        const h = Math.max(0, 0.5 - d), M = Math.max(0, d - 0.5);
        u[g] = h * h, c[g] = M * M;
      }
    }
    Ue(u, F, F), Ue(c, F, F);
    const f = new Uint8Array(s * 4);
    for (let g = 0; g < s; g++) {
      const d = Math.sqrt(u[g]) - Math.sqrt(c[g]), h = Math.max(0, Math.min(255, Math.round(255 * (0.5 - d / (2 * tn)))));
      f[g * 4] = h, f[g * 4 + 1] = h, f[g * 4 + 2] = h, f[g * 4 + 3] = 255;
    }
    return f;
  }
  function on(n, e) {
    const t = () => window.devicePixelRatio || 1;
    let o = 0;
    const _ = (f) => {
      f.preventDefault();
      const g = t();
      if (f.ctrlKey) {
        const d = Math.exp(-f.deltaY * 0.01), h = n.getBoundingClientRect();
        e.zoom_at(d, (f.clientX - h.left) * g, (f.clientY - h.top) * g);
      } else {
        const d = n.getBoundingClientRect(), h = (f.clientX - d.left) * g, M = (f.clientY - d.top) * g, S = e.code_block_at_screen(h, M);
        if (S) {
          o += f.deltaY / $;
          const w = Math.trunc(o);
          o -= w, (w !== 0 || f.deltaX !== 0) && e.scroll_code_block(S, f.deltaX * g, w);
        } else e.pan_by(f.deltaX * g, f.deltaY * g);
      }
    };
    n.addEventListener("wheel", _, {
      passive: false
    });
    let a = false;
    const s = (f) => {
      f.button === 0 && (a = true, n.setPointerCapture(f.pointerId), n.style.cursor = "grabbing");
    }, u = (f) => {
      if (!a) return;
      const g = t();
      e.pan_by(-f.movementX * g, -f.movementY * g);
    }, c = (f) => {
      a && (a = false, n.hasPointerCapture(f.pointerId) && n.releasePointerCapture(f.pointerId), n.style.cursor = "");
    };
    return n.addEventListener("pointerdown", s), n.addEventListener("pointermove", u), n.addEventListener("pointerup", c), n.addEventListener("pointercancel", c), () => {
      n.removeEventListener("wheel", _), n.removeEventListener("pointerdown", s), n.removeEventListener("pointermove", u), n.removeEventListener("pointerup", c), n.removeEventListener("pointercancel", c);
    };
  }
  gn = async function(n) {
    const e = document.getElementById("chat"), t = window.devicePixelRatio || 1, o = e.clientWidth || window.innerWidth, _ = e.clientHeight || window.innerHeight;
    e.width = Math.round(o * t), e.height = Math.round(_ * t);
    const a = await Ne(), s = new kt(e, {
      layout: Je,
      measure: en,
      rasterize: rn,
      serverUrl: n.serverUrl,
      sessionId: n.sessionId,
      replay: n.replay
    });
    s.set_math_em(y), s.start(), on(e, s);
    {
      const { pumpImageLoads: u } = await z(async () => {
        const { pumpImageLoads: c } = await import("./image-loader-DEJYb4Ur.js");
        return {
          pumpImageLoads: c
        };
      }, []);
      setInterval(() => u(s), 120);
    }
    e.setAttribute("role", "main"), e.setAttribute("aria-label", "\u5BF9\u8BDD\u753B\u5E03");
    {
      const { mountAnnouncer: u } = await z(async () => {
        const { mountAnnouncer: c } = await import("./announcer-DQUuDfyE.js");
        return {
          mountAnnouncer: c
        };
      }, []);
      u(s);
    }
    {
      const { pumpEmbedOverlay: u } = await z(async () => {
        const { pumpEmbedOverlay: M } = await import("./embed-overlay-CS5y01LF.js");
        return {
          pumpEmbedOverlay: M
        };
      }, []), { pumpCopyButtons: c } = await z(async () => {
        const { pumpCopyButtons: M } = await import("./copy-button-CE6UyqNK.js");
        return {
          pumpCopyButtons: M
        };
      }, []), { pumpTextLayer: f, attachSelection: g } = await z(async () => {
        const { pumpTextLayer: M, attachSelection: S } = await import("./text-layer-J5KnxGEh.js");
        return {
          pumpTextLayer: M,
          attachSelection: S
        };
      }, []), { pumpDock: d } = await z(async () => {
        const { pumpDock: M } = await import("./dock-q0_usG0n.js");
        return {
          pumpDock: M
        };
      }, []);
      if (g(s), n.findBar !== false) {
        const { mountFindBar: M } = await z(async () => {
          const { mountFindBar: S } = await import("./find-bar-BQX65CJo.js");
          return {
            mountFindBar: S
          };
        }, []);
        M(s);
      }
      const h = () => {
        u(s), c(s), f(s, e), d(s), requestAnimationFrame(h);
      };
      requestAnimationFrame(h);
    }
    window.__chat = s;
    {
      const { loadMathMsdf: u } = await z(async () => {
        const { loadMathMsdf: f } = await Promise.resolve().then(() => Ot);
        return {
          loadMathMsdf: f
        };
      }, void 0), { loadMathFonts: c } = await z(async () => {
        const { loadMathFonts: f } = await import("./math-fonts-CSpAQE3e.js");
        return {
          loadMathFonts: f
        };
      }, []);
      Promise.all([
        u(s).catch((f) => console.error("[math-msdf] load failed", f)),
        c().catch((f) => console.error("[math-fonts] load failed", f))
      ]).then(() => s.refresh_fonts());
    }
    return {
      chat: s,
      canvas: e,
      wasmModule: a
    };
  };
})();
export {
  _n as T,
  z as _,
  __tla,
  cn as a,
  gn as b,
  sn as c,
  Ft as d,
  un as e,
  fn as f,
  $t as g,
  Ot as h,
  Bt as l,
  bn as m,
  an as s
};
