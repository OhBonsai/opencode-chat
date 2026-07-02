var __defProp = Object.defineProperty;
var __defNormalProp = (obj, key, value) => key in obj ? __defProp(obj, key, { enumerable: true, configurable: true, writable: true, value }) : obj[key] = value;
var __publicField = (obj, key, value) => __defNormalProp(obj, typeof key !== "symbol" ? key + "" : key, value);
const c = 1e4, h = 35e3, m = 6e4;
function o(t) {
  return Math.min(1e3 * 2 ** t, 6e4);
}
function n(t, i) {
  return `${t}${t.includes("?") ? "&" : "?"}t=${i}`;
}
class a {
  constructor(i) {
    __publicField(this, "o");
    __publicField(this, "es", null);
    __publicField(this, "attempt", 0);
    __publicField(this, "connectTimer", 0);
    __publicField(this, "zombieTimer", 0);
    __publicField(this, "reconnectTimer", 0);
    __publicField(this, "stopped", false);
    this.o = { makeEventSource: (e) => new EventSource(e), now: () => Date.now(), setTimer: (e, s) => window.setTimeout(e, s), clearTimer: (e) => window.clearTimeout(e), ...i };
  }
  start() {
    this.stopped = false, this.connect();
  }
  stop() {
    var _a2;
    this.stopped = true, (_a2 = this.es) == null ? void 0 : _a2.close(), this.es = null, this.o.clearTimer(this.connectTimer), this.o.clearTimer(this.zombieTimer), this.o.clearTimer(this.reconnectTimer);
  }
  connect() {
    if (this.stopped) return;
    const i = n(this.o.url, this.o.now()), e = this.o.makeEventSource(i);
    this.es = e, this.connectTimer = this.o.setTimer(() => this.reconnect(), 1e4), e.onopen = () => {
      this.attempt = 0, this.o.clearTimer(this.connectTimer), this.armZombie();
    }, e.onmessage = (s) => {
      var _a2, _b;
      this.armZombie(), this.o.onEvent(s.data), r(s.data) && ((_b = (_a2 = this.o).onConnected) == null ? void 0 : _b.call(_a2));
    }, e.onerror = () => this.reconnect();
  }
  armZombie() {
    this.o.clearTimer(this.zombieTimer), this.zombieTimer = this.o.setTimer(() => this.reconnect(), 35e3);
  }
  reconnect() {
    var _a2;
    if (this.stopped) return;
    (_a2 = this.es) == null ? void 0 : _a2.close(), this.es = null, this.o.clearTimer(this.connectTimer), this.o.clearTimer(this.zombieTimer);
    const i = o(this.attempt);
    this.attempt += 1, this.reconnectTimer = this.o.setTimer(() => this.connect(), i);
  }
}
function r(t) {
  try {
    return JSON.parse(t).type === "server.connected";
  } catch {
    return false;
  }
}
export {
  c as CONNECT_TIMEOUT_MS,
  m as MAX_BACKOFF_MS,
  a as SseClient,
  h as ZOMBIE_MS,
  o as backoffMs,
  n as cacheBust
};
