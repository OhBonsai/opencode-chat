// bench.spec.ts — Plan 18 §3.2/§4 浏览器侧规模/内存基线采集(无人值守)。
//
// 驱动 `?bench` 长会话页 → main.ts 采样器每 1s 写一行,内容停增长后导出 `window.__benchCSV`
// (含 fps / wasmMiB,native 测不到的两项)。本测把 CSV 落 `bench-results/` + 打印 + 基本断言。
import { test, expect } from "@playwright/test";
import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";

test("plan18 scale/memory before baseline (browser)", async ({ page }) => {
  const logs: string[] = [];
  page.on("console", (m) => logs.push(`[${m.type()}] ${m.text()}`));
  page.on("pageerror", (e) => logs.push(`[pageerror] ${e.message}`));

  await page.goto("/?bench&debug&spread=60", { waitUntil: "domcontentloaded" });

  // 先确认 WebGPU 可用(否则引擎起不来、采样器永不跑 → 早失败给清晰诊断)。
  const gpu = await page.evaluate(async () => {
    const g = (navigator as unknown as { gpu?: { requestAdapter(): Promise<unknown> } }).gpu;
    if (!g) return { ok: false, why: "navigator.gpu 缺失" };
    try {
      const a = await g.requestAdapter();
      return { ok: !!a, why: a ? "ok" : "requestAdapter 返回 null" };
    } catch (e) {
      return { ok: false, why: String(e) };
    }
  });
  console.log(`[bench] WebGPU adapter: ${gpu.ok ? "OK" : "FAIL — " + gpu.why}`);

  // 等采样器导出 CSV(内容停增长后置 window.__benchCSV);载入 ~15s,给足 100s。
  let csv = "";
  try {
    await page.waitForFunction(
      () => typeof (window as unknown as { __benchCSV?: string }).__benchCSV === "string",
      null,
      { timeout: 100_000 },
    );
    csv = await page.evaluate(() => (window as unknown as { __benchCSV: string }).__benchCSV);
  } catch {
    console.log("[bench] 未拿到 __benchCSV。最近控制台:\n" + logs.slice(-25).join("\n"));
    throw new Error("bench CSV 未生成(多半 WebGPU 未在 headless 起来,见上方 adapter 行)");
  }

  // 末帧 stats(直接读 window.__chat,交叉校验)。
  const finalStats = await page.evaluate(() => {
    const c = (window as unknown as { __chat?: { stats(): Record<string, number> } }).__chat;
    return c ? c.stats() : null;
  });

  const dir = resolve(process.cwd(), "bench-results");
  mkdirSync(dir, { recursive: true });
  const out = resolve(dir, "plan18-before-browser.csv");
  writeFileSync(out, csv);
  console.log(`\n[bench] CSV → ${out}\n${csv}\n`);
  if (finalStats) {
    console.log(
      `[bench] final: retainedGlyphs=${finalStats.retainedGlyphs} ` +
        `retainedViews=${finalStats.retainedViews} storeChars=${finalStats.storeChars} ` +
        `fps=${Math.round(finalStats.fps)}`,
    );
  }

  // 断言:CSV 多行、驻留几何随历史增长、末态远超一屏(before 无虚拟化)。
  const rows = csv.trim().split("\n");
  expect(rows.length).toBeGreaterThanOrEqual(3); // 表头 + ≥2 采样
  const head = rows[0].split(",");
  const gi = head.indexOf("retainedGlyphs");
  const vals = rows.slice(1).map((r) => Number(r.split(",")[gi]));
  expect(gi).toBeGreaterThanOrEqual(0);
  expect(vals[vals.length - 1]).toBeGreaterThan(vals[0]); // 增长
  expect(vals[vals.length - 1]).toBeGreaterThan(50_000); // 10k 行远超一屏
});
