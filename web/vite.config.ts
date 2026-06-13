import { defineConfig } from "vite";
import wasm from "vite-plugin-wasm";
import topLevelAwait from "vite-plugin-top-level-await";

// wasm() 让 wasm-pack 产物可被 import;pkg 排除出依赖预打包(它是本地生成的 ESM)。
// pretext(@chenglou/pretext)是发布的 npm 包,走正常 dependencies + 预打包,无需 alias。
export default defineConfig({
  plugins: [wasm(), topLevelAwait()],
  optimizeDeps: {
    exclude: ["pkg", "opencode-chat-wasm"],
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
