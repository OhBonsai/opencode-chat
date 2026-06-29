// vitest.config.ts — Plan 22 P0:纯逻辑单测(transport SSE 客户端等)。
// 只收 src 下的 *.test.ts;tests/ 留给 Playwright(*.spec.ts),互不串台。
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    include: ["src/**/*.test.ts"],
    environment: "node",
  },
});
