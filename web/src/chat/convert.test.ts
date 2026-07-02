// convert.test.ts — Plan 25 PR-D:录制→剧本草稿转换器纯函数(vitest)。
// 目标:过滤(session/噪音)、user 升格(text 抽取)、dt 差分、round-trip(事件原样保留)。
import { describe, expect, it } from "vitest";
import { filterRecords, toScript, userTextOf } from "../../../scripts/record-chat.mjs";
import { parseScript } from "./script";

const raw = (t: number, env: unknown) => ({ t, raw: JSON.stringify(env) });

const REC = [
  raw(0, { type: "server.connected", properties: {} }),
  raw(100, { type: "server.heartbeat", properties: {} }),
  raw(200, { type: "message.updated", properties: { sessionID: "s1", info: { id: "mu1", role: "user", sessionID: "s1" } } }),
  raw(260, { type: "message.part.updated", properties: { sessionID: "s1", part: { type: "text", id: "pu1", messageID: "mu1", sessionID: "s1", text: "帮我修个 bug" } } }),
  raw(300, { type: "message.updated", properties: { sessionID: "s2", info: { id: "zz", role: "user", sessionID: "s2" } } }), // 他会话
  raw(700, { type: "message.part.delta", properties: { sessionID: "s1", messageID: "ma1", partID: "pa1", field: "text", delta: "好的" } }),
  raw(1200, { type: "session.status", properties: { sessionID: "s1", status: { type: "idle" } } }),
];

describe("filterRecords", () => {
  it("丢连接噪音 + 他会话;保时序", () => {
    const items = filterRecords(REC, "s1");
    expect(items.map((i) => i.env.type)).toEqual([
      "message.updated",
      "message.part.updated",
      "message.part.delta",
      "session.status",
    ]);
    expect(items[0].t).toBe(200);
  });

  it("坏 JSON 行静默丢弃", () => {
    const items = filterRecords([{ t: 0, raw: "{oops" }, ...REC], "s1");
    expect(items).toHaveLength(4);
  });
});

describe("userTextOf", () => {
  it("优先 text part 全量;退化拼 delta", () => {
    const items = filterRecords(REC, "s1");
    expect(userTextOf(items, "mu1")).toBe("帮我修个 bug");
    expect(userTextOf(items, "ma1")).toBe("好的"); // 无全量 → 拼 delta
  });
});

describe("toScript", () => {
  it("user 升格(打字指令前插)+ dt 差分 + 产物过 schema 校验", () => {
    const script = toScript(filterRecords(REC, "s1"), { title: "t" });
    // 首条 = user 打字指令(升格);紧随其 message.updated。
    const track = script.track as { dt: number; user?: { text: string }; event?: { type: string } }[];
    expect(track[0].user?.text).toBe("帮我修个 bug");
    expect(track[1].event?.type).toBe("message.updated");
    // dt 差分:part.updated 与前一事件差 60ms;delta 与前差 440ms。
    const evTypes = track.filter((x) => x.event).map((x) => x.event!.type);
    expect(evTypes).toEqual(["message.updated", "message.part.updated", "message.part.delta", "session.status"]);
    const dts = track.map((x) => x.dt);
    expect(dts).toContain(440); // 700-260
    // round-trip:草稿直接过 /chat 剧本 schema(载入即可播)。
    const parsed = parseScript(script);
    expect(parsed.ok).toBe(true);
  });

  it("事件对象原样进 track(内容零损)", () => {
    const items = filterRecords(REC, "s1");
    const script = toScript(items);
    const track = script.track as { event?: { type: string; properties?: unknown } }[];
    const delta = track.find((x) => x.event?.type === "message.part.delta");
    expect(delta?.event?.properties).toEqual({
      sessionID: "s1",
      messageID: "ma1",
      partID: "pa1",
      field: "text",
      delta: "好的",
    });
  });
});
