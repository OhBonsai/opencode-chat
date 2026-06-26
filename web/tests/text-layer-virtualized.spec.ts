// text-layer-virtualized.spec.ts — Plan 21 P2(0030 §7.1 硬约束)E2E:文本层 DOM ∝ 可见,不随历史。
import { test, expect } from "@playwright/test";
import { assertWebGpu } from "./helpers";

test("E5 文本层 DOM 节点数 ∝ 可见(载 10k 仍 < 500)", async ({ page }) => {
  // ?bench 载 10k 合成长会话(main.ts:set_stream_rate/reveal 极大 → 即时载满);spread=0 = 全在 t0。
  await page.goto("/?bench&spread=0", { waitUntil: "domcontentloaded" });
  await assertWebGpu(page);
  await page.waitForFunction(() => !!(window as unknown as { __chat?: unknown }).__chat, null, {
    timeout: 60_000,
  });
  // 等文本层出现可见行(内容载入并上屏)。
  await page.locator(".text-layer span").first().waitFor({ state: "attached", timeout: 90_000 });
  // 等历史累积到足够大(replay 按虚拟时间增量载入)→ 总行数 >> 视口,虚拟化才有意义。
  await expect
    .poll(() => page.evaluate(() => window.__chat.stats().storeChars), { timeout: 120_000 })
    .toBeGreaterThan(40_000);
  await page.waitForTimeout(400); // 稳定虚拟化(锚底 → 仅末尾若干行在 DOM)

  const spanCount = await page.locator(".text-layer span").count();
  // 关键不变量:DOM span 数 ∝ 可见行,远小于历史总行数(40k+ 字符约数千行)。
  expect(spanCount, "文本层 span 数 ∝ 可见(>0)").toBeGreaterThan(0);
  expect(spanCount, "文本层 span 数 ∝ 可见(远小于总行数)").toBeLessThan(500);
});
