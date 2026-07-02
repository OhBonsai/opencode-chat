// script.test.ts — Plan 25 PR-A:剧本 schema 校验 + 时间轴展开(vitest,纯逻辑)。
import { describe, expect, it } from "vitest";
import { buildTimeline, parseScript, timelineDuration } from "./script";

describe("parseScript", () => {
  it("合法剧本三种指令都过", () => {
    const r = parseScript({
      meta: { title: "t", version: 1 },
      track: [
        { dt: 0, user: { text: "hi", cps: 14, holdMs: 500 } },
        { dt: 300, event: { type: "message.part.delta", properties: { delta: "a" } } },
        { dt: 100, dock: "allow" },
      ],
    });
    expect(r.ok).toBe(true);
    if (r.ok) expect(r.script.track).toHaveLength(3);
  });

  it("event 可省 properties;meta 可省", () => {
    const r = parseScript({ track: [{ dt: 0, event: { type: "server.connected" } }] });
    expect(r.ok).toBe(true);
  });

  it.each([
    [{ track: "x" }, -1, "缺 track 数组"],
    [{ track: [{ user: { text: "a" } }] }, 0, "dt"],
    [{ track: [{ dt: -1, user: { text: "a" } }] }, 0, "dt"],
    [{ track: [{ dt: 0 }] }, 0, "恰含"],
    [{ track: [{ dt: 0, user: {} }] }, 0, "user.text"],
    [{ track: [{ dt: 0, user: { text: "a", cps: 0 } }] }, 0, "user.cps"],
    [{ track: [{ dt: 0, event: { type: "" } }] }, 0, "event.type"],
    [{ track: [{ dt: 0, event: { type: "x" }, dock: "allow" }] }, 0, "恰含"],
    [{ track: [{ dt: 0, dock: "nope" }] }, 0, "dock"],
  ])("非法 %# → 指向下标 %o", (raw, index, needle) => {
    const r = parseScript(raw);
    expect(r.ok).toBe(false);
    if (!r.ok) {
      expect(r.index).toBe(index);
      expect(r.error).toContain(needle as string);
    }
  });
});

describe("buildTimeline", () => {
  it("dt(相对)累加成绝对时间", () => {
    const tl = buildTimeline([
      { dt: 0, dock: "allow" },
      { dt: 300, dock: "allow" },
      { dt: 100, dock: "allow" },
    ]);
    expect(tl.map((x) => x.at)).toEqual([0, 300, 400]);
  });

  it("speed 缩放", () => {
    const tl = buildTimeline([{ dt: 0, dock: "allow" }, { dt: 1000, dock: "allow" }], 2);
    expect(tl.map((x) => x.at)).toEqual([0, 500]);
    expect(timelineDuration(tl)).toBe(500);
  });

  it("空轨时长 0", () => {
    expect(timelineDuration(buildTimeline([]))).toBe(0);
  });
});
