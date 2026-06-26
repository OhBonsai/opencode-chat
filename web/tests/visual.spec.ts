// visual.spec.ts — Plan 21 P2 视觉回归(0030):选区高亮黄金帧 + "高亮不遮文字"像素证。
//
// 钉死现有 headless Chromium + WebGPU(playwright.config.ts)。截图区**裁掉左侧头像 glow-orb 列**
// (那是 dynamic shaderbox,每帧脉动 → 非确定);裁出的正文 + 选区在 settled 后是静态的 → 可比对。
import { test, expect, type Page } from "@playwright/test";
import { PNG } from "pngjs";
import { bootVisible } from "./helpers";

// 正文区(排除最左 ~60px 头像列);settled 后静态。
const CLIP = { x: 64, y: 8, width: 520, height: 200 };

async function applySelection(page: Page, start: number, end: number): Promise<void> {
  await page.evaluate(
    ([s, e]) => window.__chat.set_selection(new Uint32Array([0, s, e])),
    [start, end],
  );
  await expect
    .poll(() => page.evaluate(() => window.__chat.stats().selRects), { timeout: 8_000 })
    .toBeGreaterThan(0);
  await page.waitForTimeout(120);
}

test("V1 选区高亮黄金帧", async ({ page }) => {
  await bootVisible(page);
  await applySelection(page, 6, 120); // 跨多行,确保墨团充满裁剪区(可见回归才有意义)
  // 首次跑生成基线;容差容 GPU 抗锯齿亚像素噪声(0030/Plan21 §3.3)。
  await expect(page).toHaveScreenshot("sel-highlight.png", {
    clip: CLIP,
    maxDiffPixelRatio: 0.02,
    animations: "disabled",
  });
});

test("V3 SDF 墨团黄金帧(P3 多行选区)", async ({ page }) => {
  await bootVisible(page);
  // 跨多行选区 → 逐行圆角墨团(P3)。范围足够大以覆盖正文区多行。
  await applySelection(page, 6, 220);
  await expect(page).toHaveScreenshot("ink-blob.png", {
    clip: CLIP,
    maxDiffPixelRatio: 0.02,
    animations: "disabled",
  });
});

test("V2 高亮不遮文字(像素证)", async ({ page }) => {
  await bootVisible(page);

  // 无选区帧。
  await page.evaluate(() => window.__chat.set_selection(new Uint32Array(0)));
  await page.waitForTimeout(120);
  const png0 = PNG.sync.read(await page.screenshot({ clip: CLIP }));

  // 有选区帧(覆盖正文区前若干字)。
  await applySelection(page, 0, 80);
  const png1 = PNG.sync.read(await page.screenshot({ clip: CLIP }));

  expect(png1.width).toBe(png0.width);
  expect(png1.height).toBe(png0.height);

  // 统计:正文是浅色字,暗底;选区是偏蓝半透明叠底。
  const isBright = (d: Buffer, i: number) => d[i] > 175 && d[i + 1] > 175 && d[i + 2] > 175;
  let bright0 = 0;
  let bright1 = 0;
  let blueShift = 0;
  for (let i = 0; i < png0.data.length; i += 4) {
    if (isBright(png0.data, i)) bright0 += 1;
    if (isBright(png1.data, i)) bright1 += 1;
    // 选区叠底 → 蓝通道相对无选区帧抬升(且非变暗)。
    if (png1.data[i + 2] - png0.data[i + 2] > 14) blueShift += 1;
  }

  expect(bright0, "正文区本应有浅色文字像素").toBeGreaterThan(50);
  expect(blueShift, "有选区帧应出现偏蓝高亮像素(高亮已渲染)").toBeGreaterThan(50);
  // 关键:文字像素在加了高亮后**基本保留**(若被高亮遮盖会大幅消失)→ 证文字画在高亮之上。
  expect(bright1, "高亮之上文字像素应基本保留(未被遮)").toBeGreaterThan(bright0 * 0.5);
});
