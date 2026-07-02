let e = null, d = "", i = null;
function u() {
  return e || (e = document.createElement("div"), e.className = "session-dock", e.setAttribute("role", "alertdialog"), e.setAttribute("aria-modal", "true"), e.setAttribute("aria-label", "\u9700\u8981\u786E\u8BA4"), e.style.cssText = "position:fixed;left:50%;bottom:24px;transform:translateX(-50%);z-index:9997;display:none;gap:10px;align-items:center;background:rgba(28,31,40,0.96);border:1px solid rgba(255,255,255,0.18);border-radius:10px;padding:10px 14px;backdrop-filter:blur(6px);font-size:13px;color:#e6e9f0;box-shadow:0 6px 24px rgba(0,0,0,0.4)", document.body.appendChild(e), e);
}
function b(n, r) {
  const t = u();
  t.innerHTML = "";
  const o = document.createElement("span");
  o.className = "dock-label", o.textContent = r === "permission" ? "\u5DE5\u5177\u8BF7\u6C42\u6743\u9650" : "\u52A9\u624B\u6709\u4E00\u4E2A\u95EE\u9898", t.appendChild(o);
  const l = (p, a, c) => {
    const s = document.createElement("button");
    return s.type = "button", s.className = a ? "dock-allow" : "dock-deny", s.textContent = p, s.style.cssText = "cursor:pointer;font-size:12px;padding:5px 12px;border-radius:6px;border:1px solid " + (a ? "rgba(90,150,255,0.6);background:rgba(60,110,220,0.85)" : "rgba(255,255,255,0.18);background:rgba(50,54,64,0.85)") + ";color:#fff", s.addEventListener("click", c), s;
  };
  r === "permission" ? (t.appendChild(l("\u5141\u8BB8", true, () => n.reply_permission())), t.appendChild(l("\u62D2\u7EDD", false, () => n.reply_permission()))) : t.appendChild(l("\u56DE\u7B54", true, () => n.reply_question())), t.style.display = "flex";
}
function f(n) {
  var _a;
  const r = n.session_status(), t = r.startsWith("blocked:") ? r : "";
  if (t === d) return;
  const o = d !== "";
  d = t, t === "blocked:permission" || t === "blocked:question" ? (o || (i = document.activeElement), b(n, t === "blocked:permission" ? "permission" : "question"), (_a = e == null ? void 0 : e.querySelector("button")) == null ? void 0 : _a.focus()) : e && (e.style.display = "none", o && (i == null ? void 0 : i.isConnected) && i.focus(), i = null);
}
export {
  f as pumpDock
};
