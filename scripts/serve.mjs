#!/usr/bin/env node
// 起 opencode server(Plan 1 联调用)。
// 用法:
//   node scripts/serve.mjs
//   PORT=4096 node scripts/serve.mjs
import { spawn } from "node:child_process";

const port = process.env.PORT ?? "4096";

console.log(`▶ 启动 opencode serve  ->  http://127.0.0.1:${port}`);
console.log("  (记下它实际打印的地址;Ctrl-C 停止)");

const child = spawn("opencode", ["serve", "--port", port], { stdio: "inherit" });

child.on("error", (e) => {
  if (e.code === "ENOENT") {
    console.error("✗ 未找到 opencode 命令。请先安装 opencode CLI。");
  } else {
    console.error(`✗ ${e.message}`);
  }
  process.exit(1);
});
child.on("exit", (code) => process.exit(code ?? 0));
