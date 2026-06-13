#!/usr/bin/env node
// 构建 wasm + 起 Vite harness(画布前端)。
// 用法: node scripts/dev-web.mjs
import { spawn } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const web = join(root, "web");

function run(cmd, args) {
  return new Promise((resolve, reject) => {
    const c = spawn(cmd, args, { cwd: web, stdio: "inherit", shell: false });
    c.on("error", reject);
    c.on("exit", (code) => (code === 0 ? resolve() : reject(new Error(`${cmd} 退出码 ${code}`))));
  });
}

try {
  if (!existsSync(join(web, "node_modules"))) {
    console.log("▶ 安装 web 依赖 ...");
    await run("npm", ["install"]);
  }
  console.log(`▶ 构建 wasm + 起 Vite(默认 http://localhost:${process.env.WEB_PORT ?? "5173"})`);
  await run("npm", ["run", "dev"]);
} catch (e) {
  console.error(`✗ ${e.message}`);
  process.exit(1);
}
