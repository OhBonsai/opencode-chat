// helpers.ts — Plan 21 E2E 共享工具(headless Chromium + WebGPU,复用 playwright.config.ts)。
import { expect, type Page } from "@playwright/test";

export interface ChatHandle {
  set_paused(p: boolean): void;
  step(): void;
  seek_reveal(ms: number): void;
  pan_by(dx: number, dy: number): void;
  zoom_at(factor: number, sx: number, sy: number): void;
  set_stream_rate(cps: number): void;
  set_reveal_cps(cps: number): void;
  set_selection(flat: Uint32Array): void;
  visible_turns(): string;
  visible_text_runs(): string;
  stats(): Record<string, number>;
}

export interface ScreenTurn {
  id: number;
  role: string;
  x: number;
  y: number;
  w: number;
  h: number;
  text: string;
}

export interface ScreenRun {
  block: number;
  char0: number;
  x: number;
  y: number;
  w: number;
  h: number;
  text: string;
}

declare global {
  interface Window {
    __chat: ChatHandle;
  }
}

/** 确认 headless WebGPU 可用(否则引擎不出帧 → 早失败给清晰诊断,同 bench.spec.ts)。 */
export async function assertWebGpu(page: Page): Promise<void> {
  const ok = await page.evaluate(async () => {
    const g = (navigator as unknown as { gpu?: { requestAdapter(): Promise<unknown> } }).gpu;
    if (!g) return false;
    try {
      return !!(await g.requestAdapter());
    } catch {
      return false;
    }
  });
  expect(ok, "headless WebGPU adapter 必须可用(见 playwright.config flags)").toBeTruthy();
}

/** 等 `visible_turns()` 稳定:连续 3 次(各 ~300ms)逐字节相同 → 内容载完 + 揭示完 + 相机停。 */
export async function waitStable(page: Page): Promise<void> {
  const hist: string[] = [];
  await expect
    .poll(
      async () => {
        const cur = await page.evaluate(() => window.__chat.visible_turns());
        hist.push(cur);
        if (hist.length > 3) hist.shift();
        return hist.length === 3 && hist[0] === hist[1] && hist[1] === hist[2] && cur !== "[]";
      },
      { timeout: 90_000, intervals: [300] },
    )
    .toBe(true);
}

/** 启动 showcase → 加速载入 + 即时揭示 → 等稳定 → 滚到顶(脱离锚底)→ 再等稳定。引擎保持 live 但已 settle。 */
export async function bootVisible(page: Page): Promise<void> {
  await page.goto("/?replay=showcase&noinput", { waitUntil: "domcontentloaded" });
  await assertWebGpu(page);
  await page.waitForFunction(() => !!(window as unknown as { __chat?: unknown }).__chat, null, {
    timeout: 60_000,
  });
  await page.evaluate(() => {
    window.__chat.set_stream_rate(1e9);
    window.__chat.set_reveal_cps(1e9);
  });
  await waitStable(page);
  await page.evaluate(() => window.__chat.pan_by(0, -1e6)); // 滚到顶
  await waitStable(page);
}

/** 在首个可见文本行 span 上水平拖选其大部分文字(产生跨字符的 DOM 选区)。 */
export async function dragSelectFirstLine(page: Page): Promise<void> {
  const span = page.locator(".text-layer span").first();
  await span.waitFor({ state: "attached", timeout: 15_000 });
  const b = await span.boundingBox();
  if (!b) throw new Error("文本层 span 无 boundingBox");
  const cy = b.y + b.height / 2;
  await page.mouse.move(b.x + 3, cy);
  await page.mouse.down();
  await page.mouse.move(b.x + Math.max(20, b.width * 0.85), cy, { steps: 10 });
  await page.mouse.up();
}

export const readTurns = (page: Page) =>
  page.evaluate(() => window.__chat.visible_turns()).then((s) => JSON.parse(s) as ScreenTurn[]);
export const readRuns = (page: Page) =>
  page.evaluate(() => window.__chat.visible_text_runs()).then((s) => JSON.parse(s) as ScreenRun[]);
