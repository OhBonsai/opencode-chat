#!/usr/bin/env node
// 与 opencode 多轮对话:复用同一 session(保留上下文)→ assistant 流式回复 → SSE 推 delta → 画布逐字淡入。
//
// ⚠️ 顺序(Plan1 没接快照 catch-up):先开画布页面(连上 SSE),再发消息,否则流式 delta
//    在页面连上前就发完了,画布看不到(终端仍会打印回复)。
//
// 用法:
//   node scripts/chat.mjs                       # 交互式多轮(Ctrl-D 或 /exit 退出)
//   node scripts/chat.mjs "第一句"              # 先发一句,再进多轮
//   node scripts/chat.mjs --once "只发一句"     # 单次发送即退出(脚本/CI)
//   SESSION=ses_xxx node scripts/chat.mjs       # 续接已有会话
//   MODEL=aliyuntokenplan/deepseek-v4-pro node scripts/chat.mjs
//
// API(见 ~/w/agentscode/opencode/packages/sdk/openapi.json · session.prompt):
//   POST /session                      → { id }
//   POST /session/{id}/message  body { parts:[{type:"text",text}], model:{providerID,modelID} } → { info, parts }
//   GET  /event                        SSE,message.part.delta

import readline from "node:readline/promises";
import { stdin, stdout } from "node:process";

const argv = process.argv.slice(2);
const once = argv.includes("--once");
const firstPrompt = argv.filter((a) => a !== "--once")[0]; // 可选首句

const port = process.env.PORT ?? "4096";
const server = (process.env.SERVER ?? `http://localhost:${port}`).replace(/\/$/, "");
const webPort = process.env.WEB_PORT ?? "5173";

const modelStr = process.env.MODEL ?? "aliyuntokenplan/qwen3.7-max";
const slash = modelStr.indexOf("/");
const model = { providerID: modelStr.slice(0, slash), modelID: modelStr.slice(slash + 1) };

async function postJson(url, body) {
  const res = await fetch(url, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body ?? {}),
  });
  const text = await res.text();
  let json;
  try { json = JSON.parse(text); } catch { json = text; }
  if (!res.ok) throw new Error(`${res.status} ${res.statusText} @ ${url}\n${text.slice(0, 400)}`);
  return json;
}

// 复用 SESSION 或新建;返回 { id, prefix }。
async function resolveSession() {
  if (process.env.SESSION) {
    return { id: process.env.SESSION, prefix: process.env.PREFIX ?? "/session" };
  }
  for (const prefix of ["/session", "/api/session"]) {
    try {
      const j = await postJson(`${server}${prefix}`, {});
      const id = j?.id ?? j?.sessionID;
      if (id) return { id, prefix };
    } catch { /* 试下一个 */ }
  }
  throw new Error("建 session 失败 —— 先确认 `opencode serve` 在跑(node scripts/serve.mjs)");
}

function assistantText(resp) {
  return (resp?.parts ?? [])
    .filter((p) => p?.type === "text")
    .map((p) => p.text ?? "")
    .join("");
}

// 发一轮(同一 session → 带上下文)。
async function send(prefix, id, text) {
  const resp = await postJson(`${server}${prefix}/${id}/message`, {
    parts: [{ type: "text", text }],
    model,
  });
  const reply = assistantText(resp);
  if (reply.trim()) {
    console.log(`\n🤖 ${reply}\n`);
  } else {
    console.log(`\n⚠ 没拿到文本回复。响应:\n${JSON.stringify(resp, null, 2).slice(0, 1000)}\n`);
  }
}

const { id, prefix } = await resolveSession();
console.log(`✓ session = ${id}  (前缀 ${prefix})`);
console.log(`✓ 模型   = ${model.providerID}/${model.modelID}`);
console.log(`✓ 画布   : open "http://localhost:${webPort}/?server=${server}&session=${id}"`);
console.log(once ? "" : "  输入消息开始多轮对话(Ctrl-D 或 /exit 退出)。");

try {
  if (firstPrompt) {
    console.log(`\n🧑 ${firstPrompt}`);
    await send(prefix, id, firstPrompt);
  }
  if (once || !stdin.isTTY) {
    if (!firstPrompt && !stdin.isTTY) {
      // 从管道读单条
      const piped = await new Promise((r) => {
        let buf = "";
        stdin.on("data", (d) => (buf += d));
        stdin.on("end", () => r(buf.trim()));
      });
      if (piped) { console.log(`\n🧑 ${piped}`); await send(prefix, id, piped); }
    }
    process.exit(0);
  }

  const rl = readline.createInterface({ input: stdin, output: stdout });
  rl.setPrompt("🧑 > ");
  rl.prompt();
  for await (const line of rl) {
    const t = line.trim();
    if (t === "/exit" || t === "/quit") break;
    if (t) {
      try { await send(prefix, id, t); }
      catch (e) { console.error(`✗ ${e.message}`); }
    }
    rl.prompt();
  }
  rl.close();
  console.log("再见 👋");
} catch (e) {
  console.error(`✗ ${e.message}`);
  process.exit(1);
}
