// a11y.spec.ts — Plan 26②(0030 步骤3)E2E:ARIA 镜像 / live region 播报 / Dock 焦点管理。
import { test, expect } from "@playwright/test";
import { assertWebGpu, bootVisible } from "./helpers";

test("ARIA 镜像:log 容器 + article 块(角色描述/posinset/setsize)", async ({ page }) => {
  await bootVisible(page);
  // 容器语义。
  const layer = page.locator(".text-layer");
  await expect(layer).toHaveAttribute("role", "log");
  await expect(layer).toHaveAttribute("aria-label", "对话");
  // 块级 article(虚拟化:仅可见块;posinset 保序)。
  const articles = page.locator('.text-layer [role="article"]');
  expect(await articles.count()).toBeGreaterThan(0);
  const first = articles.first();
  await expect(first).toHaveAttribute("aria-roledescription", /消息/);
  const pos = Number(await first.getAttribute("aria-posinset"));
  const size = Number(await first.getAttribute("aria-setsize"));
  expect(pos).toBeGreaterThan(0);
  expect(size).toBeGreaterThanOrEqual(pos);
  // landmark:画布 main。
  await expect(page.locator('canvas[role="main"]')).toBeAttached();
});

test("live region:状态迁移按粒度播报(不逐 delta);阻塞态 assertive", async ({ page }) => {
  await bootVisible(page); // showcase 耗尽 → 引擎安静
  const region = page.locator(".sr-announcer");
  await expect(region).toBeAttached();

  // 发送 → 播"等待回复";首包 → "正在回复"。
  await page.evaluate(() => {
    window.__chat.note_send();
  });
  await expect.poll(() => region.textContent(), { timeout: 5_000 }).toContain("等待回复");
  await page.evaluate(() =>
    window.__chat.push_event(
      JSON.stringify({
        type: "message.part.delta",
        properties: { messageID: "ma", partID: "pa", field: "text", delta: "hi" },
      }),
    ),
  );
  await expect.poll(() => region.textContent(), { timeout: 5_000 }).toContain("正在回复");

  // 权限请求 → assertive + 文案;idle 收尾 → "回复完成"。
  await page.evaluate(() =>
    window.__chat.push_event(JSON.stringify({ type: "permission.asked", properties: { sessionID: "s" } })),
  );
  await expect.poll(() => region.textContent(), { timeout: 5_000 }).toContain("权限");
  await expect(region).toHaveAttribute("aria-live", "assertive");
  await page.evaluate(() => {
    window.__chat.reply_permission();
    window.__chat.push_event(
      JSON.stringify({ type: "session.status", properties: { status: { type: "idle" } } }),
    );
  });
  await expect.poll(() => region.textContent(), { timeout: 5_000 }).toContain("回复完成");
  await expect(region).toHaveAttribute("aria-live", "polite");
});

test("Dock 焦点:打开入首按钮(alertdialog),应答后还原", async ({ page }) => {
  await page.goto("/?replay=showcase&noinput", { waitUntil: "domcontentloaded" });
  await assertWebGpu(page);
  await page.waitForFunction(() => !!(window as unknown as { __chat?: unknown }).__chat, null, {
    timeout: 60_000,
  });
  // 先给一个已知焦点(查找条输入框)。
  await page.keyboard.press("Control+F");
  await expect(page.locator(".find-input")).toBeFocused();

  // 权限请求 → Dock 弹出(alertdialog)且焦点在首按钮。
  await page.evaluate(() =>
    window.__chat.push_event(JSON.stringify({ type: "permission.asked", properties: { sessionID: "s" } })),
  );
  const dock = page.locator(".session-dock");
  await dock.waitFor({ state: "visible", timeout: 10_000 });
  await expect(dock).toHaveAttribute("role", "alertdialog");
  await expect(dock).toHaveAttribute("aria-modal", "true");
  await expect(page.locator(".dock-allow")).toBeFocused();

  // 应答 → Dock 收起 + 焦点还原到查找输入框。
  await page.locator(".dock-allow").click();
  await dock.waitFor({ state: "hidden", timeout: 10_000 });
  await expect(page.locator(".find-input")).toBeFocused();
});
