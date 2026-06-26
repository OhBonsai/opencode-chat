// copy.spec.ts — Plan 21 P1(0030 步骤 1)E2E:每条可见消息的复制按钮。
import { test, expect } from "@playwright/test";
import { bootVisible, dragSelectFirstLine, readTurns, type ScreenTurn } from "./helpers";

test.use({ permissions: ["clipboard-read", "clipboard-write"] });

test("E1 复制按钮写剪贴板", async ({ page }) => {
  await bootVisible(page);
  const turns = await readTurns(page);
  expect(turns.length, "应有可见消息").toBeGreaterThan(0);

  // 点首个复制按钮 → 剪贴板应得某条可见消息的渲染纯文本(非空)。
  await page.locator(".copy-btn").first().click();
  const clip = await page.evaluate(() => navigator.clipboard.readText());
  expect(clip.length, "剪贴板非空").toBeGreaterThan(0);
  expect(
    turns.map((t: ScreenTurn) => t.text),
    "剪贴板内容 = 某条可见消息渲染纯文本",
  ).toContain(clip);
});

test("E2 复制按钮随相机跟手", async ({ page }) => {
  await bootVisible(page);
  const dpr = await page.evaluate(() => window.devicePixelRatio || 1);

  // showcase 是单条长消息(p1)→ 视口顶恒一条可见消息,按钮在其右上角。用**水平**平移(不被纵向
  // 锚底取消,且按钮 top 不变 → 始终可见)验跟手:按钮 x 随 core 上报的屏幕 x 一致变化。
  const turns = await readTurns(page);
  expect(turns.length, "应有可见消息").toBeGreaterThan(0);
  const m = turns[0];
  const sel = `.copy-btn[data-turn-id="${m.id}"]`;
  const rightBtn = (right: number) => right / dpr - 52; // 复制 copy-button.ts 的摆位公式

  const boxBefore = await page.locator(sel).boundingBox();
  expect(Math.abs((boxBefore?.x ?? -999) - rightBtn(m.x + m.w)), "按钮初始贴合右上角").toBeLessThan(4);
  const xCoreBefore = m.x;

  await page.evaluate(() => window.__chat.pan_by(240, 0));
  await page.waitForTimeout(200);

  const t = (await readTurns(page)).find((x) => x.id === m.id);
  expect(t, "该消息仍可见").toBeTruthy();
  const boxAfter = await page.locator(sel).boundingBox();

  expect(Math.abs((boxAfter?.x ?? -999) - rightBtn(t!.x + t!.w)), "按钮跟随 core 上报位置").toBeLessThan(4);
  expect(Math.abs(t!.x - xCoreBefore), "相机平移后 core 上报位置确有变化").toBeGreaterThan(20);
  expect(Math.abs((boxAfter?.x ?? 0) - (boxBefore?.x ?? 0)), "相机平移后按钮位置确有变化").toBeGreaterThan(20);
});

test("E8 富文本复制含 html(P3)", async ({ page }) => {
  await bootVisible(page);
  await dragSelectFirstLine(page);
  await page.keyboard.press("Control+C");
  await page.waitForTimeout(250); // 等 clipboard.write([ClipboardItem]) 异步落地
  // clipboard.read() 的 ClipboardItem 应同时含 text/plain 与 text/html(richCopy 写入)。
  const types = await page.evaluate(async () => {
    const items = await navigator.clipboard.read();
    return items.flatMap((i) => i.types);
  });
  expect(types, "含纯文本").toContain("text/plain");
  expect(types, "含 HTML(富文本保真)").toContain("text/html");
});
