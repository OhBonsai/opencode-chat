function d(t, e) {
  return t === e ? null : e === "awaiting" ? "\u5DF2\u53D1\u9001,\u7B49\u5F85\u56DE\u590D" : e === "streaming" ? t === "idle" || t === "awaiting" ? "\u6B63\u5728\u56DE\u590D" : null : e === "blocked:permission" ? "\u5DE5\u5177\u8BF7\u6C42\u6743\u9650,\u9700\u8981\u786E\u8BA4" : e === "blocked:question" ? "\u52A9\u624B\u6709\u4E00\u4E2A\u95EE\u9898,\u9700\u8981\u56DE\u7B54" : e === "errored" ? "\u51FA\u9519\u4E86" : e === "stopped" ? "\u5DF2\u505C\u6B62" : e === "idle" && t !== "" && t !== "idle" ? "\u56DE\u590D\u5B8C\u6210" : null;
}
function f(t) {
  return t.startsWith("blocked:") ? "assertive" : "polite";
}
function m(t, e, i, n = 1500) {
  return e === t.lastMsg && i - t.lastAt < n ? [false, t] : [true, { lastMsg: e, lastAt: i }];
}
function p(t, e = document.body) {
  const i = document.createElement("div");
  i.className = "sr-announcer", i.setAttribute("aria-live", "polite"), i.setAttribute("aria-atomic", "true"), i.style.cssText = "position:absolute;width:1px;height:1px;margin:-1px;padding:0;overflow:hidden;clip:rect(0 0 0 0);white-space:nowrap;border:0", e.appendChild(i);
  let n = "", a = { lastMsg: "", lastAt: 0 }, r = 0;
  const u = () => {
    const s = t.session_status(), o = d(n, s);
    if (o) {
      const [l, c] = m(a, o, performance.now());
      l && (a = c, i.setAttribute("aria-live", f(s)), i.textContent = o);
    }
    n = s, r = requestAnimationFrame(u);
  };
  return r = requestAnimationFrame(u), () => {
    cancelAnimationFrame(r), i.remove();
  };
}
export {
  f as livenessOf,
  p as mountAnnouncer,
  m as shouldEmit,
  d as statusAnnouncement
};
