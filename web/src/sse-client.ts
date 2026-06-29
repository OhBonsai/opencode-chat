// sse-client.ts — Plan 22 P0(0031 §3/§5.4):opencode SSE 客户端,**韧性在 TS**。
//
// 职责:连 `/event` → 把每条 `data` 原文喂 `onEvent`(host 转 `chat.push_event`,解码在 Rust);
// `server.connected` 触发 `onConnected`(→ resync,0031 §5.4)。韧性:指数退避重连、连接超时、
// 僵尸看门狗(久无任何消息=连接死了,自愈重连)、cache-bust(防代理/SW 缓存旧流)。
//
// **可测**:EventSource 与时钟/定时器全可注入(E1 vitest 用 fake);纯逻辑函数(backoff/cacheBust)单测。

export interface EventSourceLike {
  onmessage: ((e: { data: string }) => void) | null;
  onopen: (() => void) | null;
  onerror: (() => void) | null;
  close(): void;
}

export interface SseClientOpts {
  url: string;
  /** 每条 SSE data 原文(host → chat.push_event)。 */
  onEvent: (raw: string) => void;
  /** 收到 `server.connected` → resync(0031 §5.4)。 */
  onConnected?: () => void;
  /** EventSource 工厂(默认 window.EventSource;测试注入 fake)。 */
  makeEventSource?: (url: string) => EventSourceLike;
  now?: () => number;
  setTimer?: (cb: () => void, ms: number) => number;
  clearTimer?: (id: number) => void;
}

export const CONNECT_TIMEOUT_MS = 10_000;
export const ZOMBIE_MS = 35_000;
export const MAX_BACKOFF_MS = 60_000;

/** 重连退避:`min(1000·2^attempt, 60s)`(0031)。 */
export function backoffMs(attempt: number): number {
  return Math.min(1000 * 2 ** attempt, MAX_BACKOFF_MS);
}

/** cache-bust:追加 `t=<now>` 防代理/SW 缓存旧流(0031)。 */
export function cacheBust(url: string, t: number): string {
  return `${url}${url.includes("?") ? "&" : "?"}t=${t}`;
}

export class SseClient {
  private readonly o: Required<Omit<SseClientOpts, "onConnected">> & Pick<SseClientOpts, "onConnected">;
  private es: EventSourceLike | null = null;
  private attempt = 0;
  private connectTimer = 0;
  private zombieTimer = 0;
  private reconnectTimer = 0;
  private stopped = false;

  constructor(opts: SseClientOpts) {
    this.o = {
      makeEventSource: (u) => new EventSource(u) as unknown as EventSourceLike,
      now: () => Date.now(),
      setTimer: (cb, ms) => window.setTimeout(cb, ms),
      clearTimer: (id) => window.clearTimeout(id),
      ...opts,
    };
  }

  start(): void {
    this.stopped = false;
    this.connect();
  }

  stop(): void {
    this.stopped = true;
    this.es?.close();
    this.es = null;
    this.o.clearTimer(this.connectTimer);
    this.o.clearTimer(this.zombieTimer);
    this.o.clearTimer(this.reconnectTimer);
  }

  private connect(): void {
    if (this.stopped) return;
    const url = cacheBust(this.o.url, this.o.now());
    const es = this.o.makeEventSource(url);
    this.es = es;
    // 连接超时:10s 内没 open → 视为失败,重连。
    this.connectTimer = this.o.setTimer(() => this.reconnect(), CONNECT_TIMEOUT_MS);
    es.onopen = () => {
      this.attempt = 0; // 连上 → 重置退避
      this.o.clearTimer(this.connectTimer);
      this.armZombie();
    };
    es.onmessage = (e) => {
      this.armZombie(); // 任何消息(含 heartbeat)= 活着,重置僵尸表
      this.o.onEvent(e.data);
      if (isConnected(e.data)) this.o.onConnected?.();
    };
    es.onerror = () => this.reconnect();
  }

  /** 僵尸看门狗:35s 无任何消息 → 连接死了,重连(0031:用 heartbeat 区分模型停 vs 连接死)。 */
  private armZombie(): void {
    this.o.clearTimer(this.zombieTimer);
    this.zombieTimer = this.o.setTimer(() => this.reconnect(), ZOMBIE_MS);
  }

  private reconnect(): void {
    if (this.stopped) return;
    this.es?.close();
    this.es = null;
    this.o.clearTimer(this.connectTimer);
    this.o.clearTimer(this.zombieTimer);
    const delay = backoffMs(this.attempt);
    this.attempt += 1;
    this.reconnectTimer = this.o.setTimer(() => this.connect(), delay);
  }
}

/** data 是否 `server.connected`(只窥 type,解码仍在 Rust)。 */
function isConnected(raw: string): boolean {
  try {
    return (JSON.parse(raw) as { type?: string }).type === "server.connected";
  } catch {
    return false;
  }
}
