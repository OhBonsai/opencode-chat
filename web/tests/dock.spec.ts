// dock.spec.ts — Plan 22 P4 E2E(E4):权限/反问 Dock 弹出 + 应答端到端;Dock 阻塞 turn。
import { test, expect } from "@playwright/test";
import { assertWebGpu } from "./helpers";

test.use({ permissions: ["clipboard-read", "clipboard-write"] });

test("E4 permission Dock 弹出 + 应答解阻", async ({ page }) => {
  await page.goto("/?replay=showcase&noinput", { waitUntil: "domcontentloaded" });
  await assertWebGpu(page);
  await page.waitForFunction(() => !!(window as unknown as { __chat?: unknown }).__chat, null, {
    timeout: 60_000,
  });

  // 注入权限请求(= TS transport 收到 permission.asked)→ FSM Blocked → Dock 弹出。
  await page.evaluate(() =>
    window.__chat.push_event(
      JSON.stringify({ type: "permission.asked", properties: { sessionID: "s" } }),
    ),
  );
  await expect
    .poll(() => page.evaluate(() => window.__chat.session_status()), { timeout: 10_000 })
    .toBe("blocked:permission");
  await page.locator(".session-dock").waitFor({ state: "visible", timeout: 10_000 });
  await expect(page.locator(".dock-allow")).toBeVisible();

  // 点"允许" → 解阻(FSM 离开 blocked)+ Dock 收起。
  await page.locator(".dock-allow").click();
  await expect
    .poll(() => page.evaluate(() => window.__chat.session_status()), { timeout: 10_000 })
    .not.toBe("blocked:permission");
  await page.locator(".session-dock").waitFor({ state: "hidden", timeout: 10_000 });
});

test("E4 question Dock 弹出 + 应答", async ({ page }) => {
  await page.goto("/?replay=showcase&noinput", { waitUntil: "domcontentloaded" });
  await assertWebGpu(page);
  await page.waitForFunction(() => !!(window as unknown as { __chat?: unknown }).__chat, null, {
    timeout: 60_000,
  });
  await page.evaluate(() =>
    window.__chat.push_event(
      JSON.stringify({ type: "question.asked", properties: { sessionID: "s" } }),
    ),
  );
  await expect
    .poll(() => page.evaluate(() => window.__chat.session_status()), { timeout: 10_000 })
    .toBe("blocked:question");
  await page.locator(".session-dock").waitFor({ state: "visible", timeout: 10_000 });
  await page.locator(".dock-allow").click(); // "回答"
  await expect
    .poll(() => page.evaluate(() => window.__chat.session_status()), { timeout: 10_000 })
    .not.toBe("blocked:question");
});
