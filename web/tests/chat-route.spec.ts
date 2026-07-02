// chat-route.spec.ts — Plan 25:/chat 剧本回放页端到端。快放跑完剧本,断言里程碑序列。
// 剧本本身即 Plan 24 §4 想要的「全事件回归资产」。
import { test, expect } from "@playwright/test";
import { assertWebGpu } from "./helpers";

test("showcase-full 全事件谱:11 场里程碑顺序 + 终态 idle(Plan 25 §3)", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));

  await page.goto("/chat/?script=showcase-full&speed=12", { waitUntil: "domcontentloaded" });
  await assertWebGpu(page);
  await page.waitForFunction(() => !!(window as unknown as { __chat?: unknown }).__chat, null, {
    timeout: 60_000,
  });
  await page.evaluate(() => {
    window.__chat.set_stream_rate(1e9);
    window.__chat.set_reveal_cps(1e9);
  });

  // 快放下逐里程碑轮询(持久内容;Dock 瞬态由 dock.spec/mini 覆盖)。滚到底跟随最新。
  const runs = () => page.evaluate(() => window.__chat.visible_text_runs());
  const milestones = [
    "偶发超时", // 场1 用户气泡
    "连接池", // 场2 reasoning 正文
    "p99", // 场3 流式 markdown 表格
    "▸ read", // 场4 tool 三态(终态 done 卡)
    "指数退避", // 场6 diff 块(新增行)
    "perf-report", // 场7 file 附件
    "12 passed", // 场9 错误后重试 completed
    "上下文已压缩", // 场10 compaction
    "CHANGELOG", // 场11 二轮追问
  ];
  for (const m of milestones) {
    await expect.poll(runs, { timeout: 60_000, message: `里程碑未出现: ${m}` }).toContain(m);
  }
  // 终态 idle;全程无页面错误。
  await expect
    .poll(() => page.evaluate(() => window.__chat.session_status()), { timeout: 60_000 })
    .toBe("idle");
  expect(errors, `页面错误: ${errors.join("; ")}`).toHaveLength(0);
});

test("mini 剧本端到端:打字→用户气泡→流式→Dock 自动应答→工具卡→idle", async ({ page }) => {
  const errors: string[] = [];
  page.on("pageerror", (e) => errors.push(e.message));

  await page.goto("/chat/?script=mini&speed=8", { waitUntil: "domcontentloaded" });
  await assertWebGpu(page);
  await page.waitForFunction(() => !!(window as unknown as { __chat?: unknown }).__chat, null, {
    timeout: 60_000,
  });
  // 快放下即时揭示,免等吐字动画。
  await page.evaluate(() => {
    window.__chat.set_stream_rate(1e9);
    window.__chat.set_reveal_cps(1e9);
  });

  const runs = () => page.evaluate(() => window.__chat.visible_text_runs());

  // 里程碑 1:用户气泡上屏(剧本事件路径,非打字机本身)。
  await expect.poll(runs, { timeout: 30_000 }).toContain("测试怎么跑");
  // 里程碑 2:Dock 曾弹出并被剧本自动应答(阻塞态来去)。轮询捕捉 blocked 或直接看到已解除后的工具卡。
  // (dock allow 后 FSM 离开 blocked;若快放跳过了 blocked 窗口,则以工具卡为准。)
  // 里程碑 3:工具卡(bash · done)可见。
  await expect.poll(runs, { timeout: 30_000 }).toContain("▸ bash");
  // 里程碑 4:assistant 结语可见。
  await expect.poll(runs, { timeout: 30_000 }).toContain("run.mjs");
  // 里程碑 5:终态 idle(session.status idle 收尾)。
  await expect
    .poll(() => page.evaluate(() => window.__chat.session_status()), { timeout: 30_000 })
    .toBe("idle");
  // 播放器 chrome 存在;全程无页面错误。
  await expect(page.locator(".chat-player")).toBeVisible();
  expect(errors, `页面错误: ${errors.join("; ")}`).toHaveLength(0);
});
