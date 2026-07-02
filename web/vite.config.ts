import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

// Node 全局(tsconfig types:[] 未引 @types/node;config 在 Node 下由 vite/esbuild 执行)。
declare const process: { env: Record<string, string | undefined> };

// wasm() 让 wasm-pack 产物可被 import;pkg 排除出依赖预打包(它是本地生成的 ESM)。
// pretext(@chenglou/pretext)是发布的 npm 包,走正常 dependencies + 预打包,无需 alias。
export default defineConfig({
  // GitHub Pages 子路径:项目页托管在 https://<user>.github.io/<repo>/,资源/路由需带 base。
  // CI 里 `PAGES_BASE=/infinite-chat/ npm run build`;本地 dev/实时默认 "/" 不受影响。
  // 运行时 fetch(cases/fonts)统一用 import.meta.env.BASE_URL(= 此 base),见 replay/msdf/math-fonts。
  base: process.env.PAGES_BASE || "/",
  plugins: [wasm(), topLevelAwait()],
  // Plan 25:多页 —— 主 harness(/)+ 剧本回放页(/chat/)。dev 下 vite 按路径直接服务
  // chat/index.html;build 产 dist/chat/index.html(Pages 子路径 /infinite-chat/chat/)。
  build: {
    rollupOptions: {
      input: {
        main: new URL("index.html", import.meta.url).pathname,
        chat: new URL("chat/index.html", import.meta.url).pathname,
      },
    },
  },
  optimizeDeps: {
    exclude: ["pkg", "infinite-chat-wasm"],
  },
  server: {
    port: 5173,
    // 用 ?server=/opencode 走这个代理 → 同源,绕开 CORS;Vite 转发到本地 opencode。
    // SSE(/opencode/event)也走代理。改端口就改 target。
    proxy: {
      "/opencode": {
        target: "http://localhost:4096",
        changeOrigin: true,
        rewrite: (p) => p.replace(/^\/opencode/, ""),
      },
    },
  },
});
