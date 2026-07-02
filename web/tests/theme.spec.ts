// theme.spec.ts — Plan 26① ThemeTokens E2E:换主题像素级生效;默认主题零回归由
// visual.spec 黄金帧(V1/V3;maxDiffPixelRatio 容差)守护,此处不重复。
//
// 剪裁区内存在**常驻微动画**(分隔线/shaderbox 时钟,~1-2k px)→ 先测两帧间的"环境噪声底",
// 再用相对阈值断言:换主题 ≫ 噪声底;不换主题 ≈ 噪声底。
import { test, expect } from "@playwright/test";
import { PNG } from "pngjs";
import { bootVisible } from "./helpers";

// 正文区(同 visual.spec:裁掉左侧动效头像列)。
const CLIP = { x: 64, y: 8, width: 520, height: 200 };

function pixelDiffCount(a: Buffer, b: Buffer): number {
  const pa = PNG.sync.read(a);
  const pb = PNG.sync.read(b);
  let diff = 0;
  for (let i = 0; i < pa.data.length; i += 4) {
    if (
      Math.abs(pa.data[i] - pb.data[i]) > 8 ||
      Math.abs(pa.data[i + 1] - pb.data[i + 1]) > 8 ||
      Math.abs(pa.data[i + 2] - pb.data[i + 2]) > 8
    ) {
      diff += 1;
    }
  }
  return diff;
}

test("set_theme 局部覆盖:下一帧像素生效;非法 JSON 忽略不崩;空覆盖恢复默认", async ({ page }) => {
  await bootVisible(page); // showcase settled
  // 铺一大片程序化选区(同 visual V3)→ 剪裁区内有数千 px 的 selection 高亮作为"被主题化的面"。
  await page.evaluate(() => window.__chat.set_selection(new Uint32Array([0, 6, 220])));
  await page.waitForTimeout(200);
  const shot = () => page.screenshot({ clip: CLIP });

  // 0) 环境噪声底:同主题两帧的像素差(常驻微动画)。
  const f0 = await shot();
  await page.waitForTimeout(200);
  const f1 = await shot();
  const ambient = pixelDiffCount(f0, f1);

  // 1) 换主题(夸张 code_bg/quote_bar/card_bg → 装饰区大面积变色)→ 远超噪声底。
  await page.evaluate(() =>
    window.__chat.set_theme(
      JSON.stringify({
        selection: [1.0, 0.1, 0.05, 0.85], // 选区蓝 → 亮红:选区覆盖面数千 px 必大变
        head_rule: [1.0, 0.9, 0.1, 1.0],
        code_bg: [0.5, 0.05, 0.05, 0.9],
      }),
    ),
  );
  await page.waitForTimeout(200);
  const themed = await shot();
  const changed = pixelDiffCount(f1, themed);
  expect(changed, `换主题应大改装饰像素(噪声底 ${ambient})`).toBeGreaterThan(ambient * 2 + 500);

  // 2) 非法 JSON:warn + 忽略 → 差异回落到噪声量级。
  await page.evaluate(() => window.__chat.set_theme("{ not json"));
  await page.waitForTimeout(200);
  const still = await shot();
  expect(pixelDiffCount(themed, still), "非法 JSON 不应改变主题").toBeLessThan(ambient * 3 + 300);

  // 3) 空覆盖 = 全默认 → 恢复原观感(与初始帧差 ≈ 噪声底)。
  await page.evaluate(() => window.__chat.set_theme("{}"));
  await page.waitForTimeout(200);
  const restored = await shot();
  expect(pixelDiffCount(f0, restored), "空覆盖应恢复默认观感").toBeLessThan(ambient * 3 + 300);
});
