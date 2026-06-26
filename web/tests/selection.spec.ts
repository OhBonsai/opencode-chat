// selection.spec.ts — Plan 21 P2(0030 步骤 2)E2E:拖选 + GPU 高亮 + Cmd+C 复制 + 越界 clamp。
import { test, expect } from "@playwright/test";
import { bootVisible, dragSelectFirstLine } from "./helpers";

test.use({ permissions: ["clipboard-read", "clipboard-write"] });

test("E3 拖选产生 GPU 高亮", async ({ page }) => {
  await bootVisible(page);
  await dragSelectFirstLine(page);
  // selectionchange(节流 rAF)→ set_selection → 下帧 core 发选区 FrameRect;stats 每秒刷新 → poll。
  await expect
    .poll(() => page.evaluate(() => window.__chat.stats().selRects), { timeout: 8_000 })
    .toBeGreaterThan(0);
});

test("E4 Cmd+C 复制选中文本", async ({ page }) => {
  await bootVisible(page);
  await dragSelectFirstLine(page);
  const selText = await page.evaluate(() => window.getSelection()?.toString() ?? "");
  expect(selText.length, "拖选应产生非空选区文本").toBeGreaterThan(0);

  // CI 用 Control+C(非 Meta);透明文本层原生可复制 → 剪贴板 = 选区文本。
  await page.keyboard.press("Control+C");
  await page.waitForTimeout(250); // 等 clipboard.write 异步落地
  const clip = await page.evaluate(() => navigator.clipboard.readText());
  expect(clip, "剪贴板 = 选区渲染文本").toBe(selText);
});

test("E6 选区越界 clamp 到可见(v1)", async ({ page }) => {
  await bootVisible(page);
  // 程序化灌一个 end 远超文本长度的区间 → core 应 clamp,不报错,高亮数有界(≤ 可见字形)。
  await page.evaluate(() => window.__chat.set_selection(new Uint32Array([0, 0, 100_000])));
  await expect
    .poll(() => page.evaluate(() => window.__chat.stats().selRects), { timeout: 8_000 })
    .toBeGreaterThan(0);
  const n = await page.evaluate(() => window.__chat.stats().selRects);
  const storeChars = await page.evaluate(() => window.__chat.stats().storeChars);
  // clamp 不变量:end 越界被夹到块字形数 → 高亮数有界(≤ 本块内容量,绝非传入的 100000)。
  expect(n, "越界 end 被 clamp,不爆").toBeLessThanOrEqual(storeChars);
  expect(n, "高亮数有界但非空").toBeLessThan(100_000);
});
