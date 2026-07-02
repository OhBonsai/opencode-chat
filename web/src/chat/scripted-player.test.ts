// scripted-player.test.ts — Plan 25 PR-A:调度器触发顺序/时序/暂停/倍速/前向 seek(vitest)。
import { describe, expect, it } from "vitest";
import { parseScript } from "./script";
import { ScriptedPlayer, type ScriptDriver } from "./scripted-player";

interface Fired {
  kind: "user" | "event" | "dock";
  detail: string;
  instant?: boolean;
}

function harness(track: unknown[]) {
  const r = parseScript({ track });
  if (!r.ok) throw new Error(`bad script: ${r.error}`);
  const fired: Fired[] = [];
  const driver: ScriptDriver = {
    typeUser: (u, instant) => {
      fired.push({ kind: "user", detail: u.text, instant });
    },
    pushEvent: (raw) => {
      fired.push({ kind: "event", detail: JSON.parse(raw).type });
    },
    dock: (a) => {
      fired.push({ kind: "dock", detail: a });
    },
  };
  return { player: new ScriptedPlayer(r.script, driver), fired };
}

const TRACK = [
  { dt: 0, user: { text: "hi" } },
  { dt: 300, event: { type: "message.part.delta" } },
  { dt: 100, event: { type: "session.status" } },
  { dt: 100, dock: "allow" },
];

describe("ScriptedPlayer", () => {
  it("按时间轴顺序在到点时触发", () => {
    const { player, fired } = harness(TRACK);
    player.play();
    player.tick(0); // virtual 0 → 首条 user(at 0)
    expect(fired.map((f) => f.kind)).toEqual(["user"]);
    player.tick(300); // at 300 → delta
    player.tick(400); // at 400 → status
    player.tick(500); // at 500 → dock
    expect(fired.map((f) => f.detail)).toEqual([
      "hi",
      "message.part.delta",
      "session.status",
      "allow",
    ]);
  });

  it("暂停冻结虚拟时钟,不触发", () => {
    const { player, fired } = harness(TRACK);
    player.play();
    player.tick(0);
    player.pause();
    player.tick(1000); // 暂停中不推进
    expect(fired).toHaveLength(1);
    player.play();
    player.tick(1000); // 恢复:lastNow 重置 → 从此刻起算,尚未到下一条(相对起点)
    player.tick(1300); // +300 → delta
    expect(fired.map((f) => f.detail)).toEqual(["hi", "message.part.delta"]);
  });

  it("倍速加快虚拟时钟", () => {
    const { player, fired } = harness(TRACK);
    player.setSpeed(10);
    player.play();
    player.tick(0);
    player.tick(50); // 50ms 实 × 10 = 500 虚 → user+delta+status+dock 全触发
    expect(fired).toHaveLength(4);
  });

  it("前向 seek 立即触发 ≤目标 的指令(user instant)", () => {
    const { player, fired } = harness(TRACK);
    player.seekForward(350); // ≤350:user(0)+delta(300)
    expect(fired.map((f) => f.detail)).toEqual(["hi", "message.part.delta"]);
    expect(fired[0].instant).toBe(true);
    expect(player.position()).toBe(350);
  });

  it("打字门:typeUser 返回 Promise → 时间轴冻结到 resolve(后续事件不越序)", async () => {
    const fired: string[] = [];
    let release!: () => void;
    const r = parseScript({ track: TRACK });
    if (!r.ok) throw new Error("bad");
    const player = new ScriptedPlayer(r.script, {
      typeUser: (u) => {
        fired.push(`user:${u.text}`);
        return new Promise<void>((res) => (release = res));
      },
      pushEvent: (raw) => {
        fired.push(`event:${JSON.parse(raw).type}`);
      },
      dock: () => {},
    });
    player.play();
    player.tick(0); // user@0 触发 → 门关
    player.tick(10_000); // 门内:时间不推进,后续事件不发
    expect(fired).toEqual(["user:hi"]);
    release(); // 打字完成 → 开门
    await Promise.resolve(); // 让 finally 跑
    player.tick(10_016); // lastNow 重置 → 从此刻续走
    player.tick(10_016 + 500); // +500ms → 其余三条按序触发
    expect(fired).toEqual([
      "user:hi",
      "event:message.part.delta",
      "event:session.status",
    ].concat([])); // dock 走 driver.dock,不进 fired 的 event 序列
  });

  it("onDone 在全部触发后回调一次", () => {
    const r = parseScript({ track: TRACK });
    if (!r.ok) throw new Error("bad");
    let done = 0;
    const player = new ScriptedPlayer(r.script, {
      typeUser: () => {},
      pushEvent: () => {},
      dock: () => {},
      onDone: () => (done += 1),
    });
    player.play();
    player.tick(0);
    player.tick(600); // 越过末点
    expect(done).toBe(1);
    expect(player.isPlaying()).toBe(false);
  });
});
