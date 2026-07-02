const n = ["c01-plaintext", "c02-bold-close", "c03-inline-code", "c04-list", "c05-fence", "c06-all", "c07-setext", "c08-quote-alert", "c09-mixed-long", "c10-cjk", "n-all", "g-table", "g-nest", "g-mixed", "g-choreo", "g-tasks", "g-md-all", "showcase", "reel"], l = "infinite-chat.replayConfig", t = { case: null, speed: 1 };
function s() {
  try {
    const c = localStorage.getItem(l);
    if (!c) return { ...t };
    const e = JSON.parse(c);
    return { case: typeof e.case == "string" && n.includes(e.case) ? e.case : null, speed: Number(e.speed) > 0 ? Number(e.speed) : 1 };
  } catch {
    return { ...t };
  }
}
function o(c) {
  try {
    localStorage.setItem(l, JSON.stringify(c));
  } catch {
  }
}
export {
  n as REPLAY_CASES,
  s as loadReplayConfig,
  o as saveReplayConfig
};
