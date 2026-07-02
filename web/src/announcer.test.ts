// announcer.test.ts — Plan 26②:播报器纯逻辑(迁移→文案 / 节流去重 / liveness)。
import { describe, expect, it } from "vitest";
import { livenessOf, shouldEmit, statusAnnouncement, type EmitState } from "./announcer";

describe("statusAnnouncement", () => {
  it.each([
    ["idle", "awaiting", "已发送,等待回复"],
    ["awaiting", "streaming", "正在回复"],
    ["idle", "streaming", "正在回复"],
    ["streaming", "blocked:permission", "工具请求权限,需要确认"],
    ["streaming", "blocked:question", "助手有一个问题,需要回答"],
    ["streaming", "errored", "出错了"],
    ["streaming", "stopped", "已停止"],
    ["streaming", "idle", "回复完成"],
  ])("%s→%s → %s", (a, b, want) => {
    expect(statusAnnouncement(a, b)).toBe(want);
  });

  it("同态不播;非关键迁移不播", () => {
    expect(statusAnnouncement("idle", "idle")).toBeNull();
    expect(statusAnnouncement("blocked:permission", "streaming")).toBeNull(); // 解阻回流式:不噪
    expect(statusAnnouncement("", "idle")).toBeNull(); // 初始为 idle:不播
  });
});

describe("livenessOf", () => {
  it("阻塞态 assertive,其余 polite", () => {
    expect(livenessOf("blocked:permission")).toBe("assertive");
    expect(livenessOf("blocked:question")).toBe("assertive");
    expect(livenessOf("streaming")).toBe("polite");
    expect(livenessOf("idle")).toBe("polite");
  });
});

describe("shouldEmit", () => {
  const s0: EmitState = { lastMsg: "", lastAt: 0 };

  it("不同文案立即播;同文案窗口内去重、窗口外重播", () => {
    const [ok1, s1] = shouldEmit(s0, "正在回复", 1000);
    expect(ok1).toBe(true);
    const [ok2] = shouldEmit(s1, "正在回复", 1800); // <1500ms 同文案 → 抑制
    expect(ok2).toBe(false);
    const [ok3] = shouldEmit(s1, "回复完成", 1801); // 不同文案 → 播
    expect(ok3).toBe(true);
    const [ok4] = shouldEmit(s1, "正在回复", 2600); // >1500ms → 重播
    expect(ok4).toBe(true);
  });
});
