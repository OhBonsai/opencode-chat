// render-fallback.spec.ts — Plan 22 P3 E2E(E3):全 part 兜底可见(标签 + 内容)。
// 经 push_event 注入非文本 part(= TS transport 路径),验它们都渲染出来(身份标签 + 原始内容)。
import { test, expect } from "@playwright/test";
import { assertWebGpu } from "./helpers";

const partUpdated = (p: unknown) =>
  JSON.stringify({ type: "message.part.updated", properties: { part: p, time: 1 } });

test("E3 全 part 兜底可见(tool/reasoning/file)", async ({ page }) => {
  await page.goto("/?replay=showcase&noinput", { waitUntil: "domcontentloaded" });
  await assertWebGpu(page);
  await page.waitForFunction(() => !!(window as unknown as { __chat?: unknown }).__chat, null, {
    timeout: 60_000,
  });
  // 即时载入/揭示,注入非文本 part(各自独立 message → 独立 view:1=tool,2=reasoning,3=file)。
  await page.evaluate(
    ([tool, reasoning, file]) => {
      window.__chat.set_stream_rate(1e9);
      window.__chat.set_reveal_cps(1e9);
      window.__chat.push_event(tool);
      window.__chat.push_event(reasoning);
      window.__chat.push_event(file);
    },
    [
      partUpdated({
        type: "tool",
        id: "ztool",
        messageID: "zmsg-tool",
        tool: "bash",
        state: { status: "completed", input: { cmd: "ls -la" } },
      }),
      partUpdated({ type: "reasoning", id: "zrea", messageID: "zmsg-rea", text: "思考要点ZZZ" }),
      partUpdated({
        type: "file",
        id: "zfile",
        messageID: "zmsg-file",
        filename: "note.txt",
        url: "http://example/note.txt",
      }),
    ],
  );
  await page.waitForTimeout(300); // 上屏 + 排版

  // 每类 part 滚到其所在 view(showcase=0;tool=1,reasoning=2,file=3)→ 兜底渲染(标签 + 内容)可见。
  const seen = () => page.evaluate(() => window.__chat.visible_text_runs());
  const expectVisible = async (view: number, ...needles: string[]) => {
    await page.evaluate((v) => window.__chat.scroll_to(v), view);
    await page.waitForTimeout(120);
    const runs = await seen();
    for (const n of needles) expect(runs, `view ${view} 应含 "${n}"`).toContain(n);
  };
  await expectVisible(1, "tool:bash", "ls -la");
  await expectVisible(2, "reasoning", "思考要点ZZZ");
  await expectVisible(3, "file:note.txt");
});
