// sse-client.test.ts — Plan 22 P0 E1(vitest):SSE 客户端韧性单测(退避/超时/僵尸/cache-bust)。
import { describe, expect, it } from "vitest";
import {
  backoffMs,
  cacheBust,
  CONNECT_TIMEOUT_MS,
  MAX_BACKOFF_MS,
  SseClient,
  ZOMBIE_MS,
  type EventSourceLike,
} from "./sse-client";

// 手搓确定性调度器(可断言定时延迟,不依赖真实时钟)。
class FakeScheduler {
  t = 0;
  private id = 1;
  private timers: { id: number; at: number; cb: () => void }[] = [];
  set = (cb: () => void, ms: number): number => {
    const id = this.id++;
    this.timers.push({ id, at: this.t + ms, cb });
    return id;
  };
  clear = (id: number): void => {
    this.timers = this.timers.filter((x) => x.id !== id);
  };
  now = (): number => this.t;
  advance(ms: number): void {
    const end = this.t + ms;
    for (;;) {
      const due = this.timers
        .filter((x) => x.at <= end)
        .sort((a, b) => a.at - b.at)[0];
      if (!due) break;
      this.timers = this.timers.filter((x) => x !== due);
      this.t = due.at;
      due.cb();
    }
    this.t = end;
  }
}

class FakeES implements EventSourceLike {
  onmessage: ((e: { data: string }) => void) | null = null;
  onopen: (() => void) | null = null;
  onerror: (() => void) | null = null;
  closed = false;
  constructor(public url: string) {}
  close(): void {
    this.closed = true;
  }
}

function harness() {
  const sched = new FakeScheduler();
  const made: FakeES[] = [];
  const events: string[] = [];
  let connectedCount = 0;
  const client = new SseClient({
    url: "http://x/event",
    onEvent: (raw) => events.push(raw),
    onConnected: () => {
      connectedCount += 1;
    },
    makeEventSource: (u) => {
      const es = new FakeES(u);
      made.push(es);
      return es;
    },
    now: sched.now,
    setTimer: sched.set,
    clearTimer: sched.clear,
  });
  return { sched, made, events, client, connected: () => connectedCount };
}

describe("backoffMs", () => {
  it("指数退避并夹到 60s", () => {
    expect(backoffMs(0)).toBe(1000);
    expect(backoffMs(1)).toBe(2000);
    expect(backoffMs(3)).toBe(8000);
    expect(backoffMs(6)).toBe(MAX_BACKOFF_MS); // 64000 → 60000
    expect(backoffMs(20)).toBe(MAX_BACKOFF_MS);
  });
});

describe("cacheBust", () => {
  it("追加 t 参数(按已有 ? 选连接符)", () => {
    expect(cacheBust("http://x/event", 5)).toBe("http://x/event?t=5");
    expect(cacheBust("http://x/event?a=1", 5)).toBe("http://x/event?a=1&t=5");
  });
});

describe("SseClient", () => {
  it("连接带 cache-bust;消息喂 onEvent;server.connected 触发 onConnected", () => {
    const h = harness();
    h.client.start();
    expect(h.made).toHaveLength(1);
    expect(h.made[0].url).toContain("t=0");
    h.made[0].onopen?.();
    h.made[0].onmessage?.({ data: '{"type":"server.connected"}' });
    h.made[0].onmessage?.({ data: '{"type":"message.part.delta"}' });
    expect(h.events).toHaveLength(2);
    expect(h.connected()).toBe(1);
  });

  it("连接超时 10s 未 open → 重连", () => {
    const h = harness();
    h.client.start();
    expect(h.made).toHaveLength(1);
    h.sched.advance(CONNECT_TIMEOUT_MS); // 超时触发 reconnect(退避 backoff(0)=1000)
    h.sched.advance(1000);
    expect(h.made.length).toBeGreaterThanOrEqual(2);
    expect(h.made[0].closed).toBe(true);
  });

  it("35s 僵尸(无消息)→ 自愈重连", () => {
    const h = harness();
    h.client.start();
    h.made[0].onopen?.(); // 起僵尸表
    h.sched.advance(ZOMBIE_MS); // 久无消息 → reconnect
    h.sched.advance(1000); // 退避后重连
    expect(h.made.length).toBeGreaterThanOrEqual(2);
  });

  it("onerror → 指数退避重连(第二次更久)", () => {
    const h = harness();
    h.client.start();
    h.made[0].onerror?.(); // attempt 0 → 退避 1000
    h.sched.advance(1000);
    expect(h.made).toHaveLength(2);
    h.made[1].onerror?.(); // attempt 1 → 退避 2000
    h.sched.advance(1999);
    expect(h.made).toHaveLength(2); // 还没到
    h.sched.advance(1);
    expect(h.made).toHaveLength(3);
  });

  it("E2: 断连 → 重连 → server.connected → onConnected(resync)", () => {
    const h = harness();
    h.client.start();
    h.made[0].onopen?.();
    h.made[0].onmessage?.({ data: '{"type":"server.connected"}' }); // 首连
    expect(h.connected()).toBe(1);
    h.made[0].onerror?.(); // 断连
    h.sched.advance(1000); // 退避 → 重连
    expect(h.made).toHaveLength(2);
    h.made[1].onopen?.();
    h.made[1].onmessage?.({ data: '{"type":"server.connected"}' }); // 重连后再握手
    expect(h.connected()).toBe(2); // 每次 connected 都触发 resync(对账补漏)
  });

  it("stop 后不再重连", () => {
    const h = harness();
    h.client.start();
    h.client.stop();
    h.made[0].onerror?.();
    h.sched.advance(60_000);
    expect(h.made).toHaveLength(1);
    expect(h.made[0].closed).toBe(true);
  });
});
