// find.spec.ts — Plan 21 P3(0030 步骤 4)E2E:自建 Cmd+F 跨**全历史**命中并跳转选中。
import { test, expect } from "@playwright/test";
import { assertWebGpu } from "./helpers";

test.use({ permissions: ["clipboard-read", "clipboard-write"] });

test("E7 自建 Cmd+F 跨历史命中跳转并选中", async ({ page }) => {
  // 载 10k 长会话(200 turn,每 turn 一条消息;"Turn 0" 是首条 → 锚底时在屏外顶部)。
  await page.goto("/?bench&spread=0", { waitUntil: "domcontentloaded" });
  await assertWebGpu(page);
  await page.waitForFunction(() => !!(window as unknown as { __chat?: unknown }).__chat, null, {
    timeout: 60_000,
  });
  await page.locator(".text-layer span").first().waitFor({ state: "attached", timeout: 90_000 });
  // 等历史累积到 "Turn 0" 已存在且不在可见集(屏外)。
  await expect
    .poll(
      () =>
        page.evaluate(() => {
          const hits = JSON.parse(window.__chat.find("Turn 0")) as unknown[];
          const vis = (JSON.parse(window.__chat.visible_text_runs()) as { text: string }[]).some(
            (r) => r.text.includes("Turn 0"),
          );
          return hits.length > 0 && !vis; // 命中存在但当前屏外
        }),
      { timeout: 120_000 },
    )
    .toBe(true);

  // 打开查找条 → 输入 → 自建查找跳转(input 事件即跳到首个命中)。
  await page.keyboard.press("Control+F");
  await page.locator(".find-input").fill("Turn 0");

  // 命中词进入可见 + 被选中(scroll_to + selectMatch 生效)。
  await expect
    .poll(
      () =>
        page.evaluate(() =>
          (JSON.parse(window.__chat.visible_text_runs()) as { text: string }[]).some((r) =>
            r.text.includes("Turn 0"),
          ),
        ),
      { timeout: 15_000 },
    )
    .toBe(true);
  await expect
    .poll(() => page.evaluate(() => window.__chat.stats().selRects), { timeout: 15_000 })
    .toBeGreaterThan(0);
});
