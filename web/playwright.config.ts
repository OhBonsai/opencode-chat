// playwright.config.ts — Plan 18 浏览器侧规模/内存度量(fps + wasm 线性内存)的无人值守采集。
// 单测:`tests/bench.spec.ts` 驱动 `?bench` 页,读 `window.__benchCSV`。
//
// 关键:wgpu(wasm)默认只走 **WebGPU**(无 webgl feature)→ 需 Chromium 开 WebGPU。
// headless Chromium 在 macOS 经 Metal/Dawn 出图;下方 launch flags 显式打开 unsafe-webgpu。
import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: "./tests",
  timeout: 120_000, // 长会话载入 ~15s + 采样
  fullyParallel: false,
  // 串行单 worker:剪贴板是**全局单例**(E1/E4/E8 并行会互踩)、WebGPU/视觉帧需确定 → 强制 1 worker。
  workers: 1,
  // Plan 20/21 §3.5:机器可读输出 → verify/CI 可聚合。list 给人看,junit 给机器。
  reporter: [["list"], ["junit", { outputFile: "test-results/junit.xml" }]],
  use: {
    baseURL: "http://localhost:5173",
    headless: true,
    // 用**完整** Chromium 的 new-headless(非 headless-shell)→ 才有 WebGPU(Dawn/Metal)。
    channel: "chromium",
    launchOptions: {
      // WebGPU on headless Chromium(Mac=Metal/Dawn,Linux=Vulkan/SwiftShader)。
      args: [
        "--enable-unsafe-webgpu",
        "--enable-features=Vulkan",
        "--enable-webgpu-developer-features",
        "--ignore-gpu-blocklist",
        "--use-angle=default",
      ],
    },
  },
  // dev server:复用已开的(reuseExistingServer),否则现起(含 build:wasm,故 timeout 放宽)。
  webServer: {
    command: "npm run dev",
    url: "http://localhost:5173",
    timeout: 180_000,
    reuseExistingServer: true,
    stdout: "ignore",
    stderr: "pipe",
  },
});
