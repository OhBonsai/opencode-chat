#!/usr/bin/env node
// record-chat.mjs — Plan 25 PR-D:真 opencode 会话 → /chat 剧本**草稿**转换器。
//
// 用法(录制;Ctrl+C 停止并落盘):
//   node scripts/record-chat.mjs --server http://localhost:4096 --session ses_xxx --out my-chat
//   → web/public/chats/my-chat.json(剧本草稿,人再精修节奏/删噪音)
//   → web/public/chats/my-chat.raw.json(原始 {t,raw} 录像,可重复转换)
//
// 用法(离线重转换,不连服务端):
//   node scripts/record-chat.mjs --from web/public/chats/my-chat.raw.json --session ses_xxx --out my-chat
//
// 转换规则(纯函数,vitest 于 web/src/chat/convert.test.ts 覆盖):
//   ① 过滤:只留目标 session 的事件;丢 server.connected/heartbeat 噪音。
//   ② user 升格:`message.updated(role=user)` 前插一条 `user` 打字指令(text 从其 text part 抽出)。
//   ③ dt 化:绝对时刻差分为相对 dt(手插改一条不用重排全轴)。

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

// ───────────────────────── 纯转换逻辑(可被 vitest import) ─────────────────────────

/** 事件是否属于目标 session(properties/part/info 三处任一命中;无 session 字段的留给白名单判断)。 */
export function eventSession(env) {
  const p = env?.properties ?? {};
  return p.sessionID ?? p.part?.sessionID ?? p.info?.sessionID ?? null;
}

/** 录制噪音(连接生命周期):不进剧本。 */
const NOISE_TYPES = new Set(["server.connected", "server.heartbeat", "server.instance.disposed"]);

/** ① 过滤:解析 raw → 只留目标 session(或无 session 标注的会话事件)+ 丢连接噪音。
 * 返回 `[{t, env}]`(env = 解析后的事件对象;解析失败的行丢弃)。 */
export function filterRecords(records, sessionId) {
  const out = [];
  for (const r of records) {
    let env;
    try {
      env = JSON.parse(r.raw);
    } catch {
      continue;
    }
    if (typeof env?.type !== "string" || NOISE_TYPES.has(env.type)) continue;
    const sid = eventSession(env);
    if (sessionId && sid && sid !== sessionId) continue; // 他会话 → 丢
    out.push({ t: r.t, env });
  }
  return out;
}

/** 从某 user 消息的 text part 事件里抽正文(message.part.updated 的 part.text 优先;无则拼 delta)。 */
export function userTextOf(items, messageId) {
  let assembled = "";
  for (const { env } of items) {
    const p = env.properties ?? {};
    if (env.type === "message.part.updated" && p.part?.messageID === messageId && p.part?.type === "text") {
      return p.part.text ?? "";
    }
    if (env.type === "message.part.delta" && p.messageID === messageId && p.field === "text") {
      assembled += p.delta ?? "";
    }
  }
  return assembled;
}

/** ②+③:过滤后的 `[{t, env}]` → 剧本草稿(user 升格 + dt 差分)。 */
export function toScript(items, { title = "recorded chat" } = {}) {
  // user 消息 id 集(message.updated role=user)。
  const userMsgs = new Set(
    items
      .filter(({ env }) => env.type === "message.updated" && env.properties?.info?.role === "user")
      .map(({ env }) => env.properties.info.id),
  );
  const insertedUser = new Set();
  const track = [];
  let prevT = items.length ? items[0].t : 0;
  for (const { t, env } of items) {
    const dt = Math.max(0, Math.round(t - prevT));
    prevT = t;
    // user 升格:该 user 消息的首个事件前插打字指令(dt 归打字条,原事件紧随)。
    const info = env.properties?.info;
    if (env.type === "message.updated" && info && userMsgs.has(info.id) && !insertedUser.has(info.id)) {
      insertedUser.add(info.id);
      const text = userTextOf(items, info.id);
      if (text) {
        track.push({ dt, user: { text, cps: 14, holdMs: 400 } });
        track.push({ dt: 600, event: env });
        continue;
      }
    }
    track.push({ dt, event: env });
  }
  return { meta: { title, version: 1 }, track };
}

// ───────────────────────── 录制(SSE)+ CLI ─────────────────────────

function parseArgs(argv) {
  const a = {};
  for (let i = 0; i < argv.length; i++) {
    if (argv[i].startsWith("--")) a[argv[i].slice(2)] = argv[i + 1];
  }
  return a;
}

async function record(server, onRaw) {
  const t0 = Date.now();
  const res = await fetch(`${server}/event`, { headers: { accept: "text/event-stream" } });
  if (!res.ok || !res.body) throw new Error(`SSE 连接失败 ${res.status}`);
  console.error(`[record] 已连 ${server}/event,Ctrl+C 停止…`);
  const reader = res.body.getReader();
  const dec = new TextDecoder();
  let buf = "";
  for (;;) {
    const { done, value } = await reader.read();
    if (done) break;
    buf += dec.decode(value, { stream: true });
    let idx;
    while ((idx = buf.indexOf("\n\n")) >= 0) {
      const chunk = buf.slice(0, idx);
      buf = buf.slice(idx + 2);
      for (const line of chunk.split("\n")) {
        if (line.startsWith("data:")) onRaw({ t: Date.now() - t0, raw: line.slice(5).trim() });
      }
    }
  }
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  const out = args.out ?? "recorded";
  const session = args.session ?? "";
  const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  const chatsDir = resolve(root, "web/public/chats");
  mkdirSync(chatsDir, { recursive: true });

  let records;
  if (args.from) {
    records = JSON.parse(readFileSync(resolve(args.from), "utf8"));
  } else {
    const server = args.server ?? "http://localhost:4096";
    records = [];
    const flush = () => {
      writeFileSync(resolve(chatsDir, `${out}.raw.json`), JSON.stringify(records, null, 1));
      const script = toScript(filterRecords(records, session), { title: out });
      writeFileSync(resolve(chatsDir, `${out}.json`), JSON.stringify(script, null, 1));
      console.error(
        `[record] ${records.length} 条 → chats/${out}.json(${script.track.length} 指令)+ ${out}.raw.json`,
      );
      process.exit(0);
    };
    process.on("SIGINT", flush);
    await record(server, (r) => {
      records.push(r);
      process.stderr.write(`\r[record] ${records.length} 条`);
    });
    flush();
    return;
  }
  const script = toScript(filterRecords(records, session), { title: out });
  writeFileSync(resolve(chatsDir, `${out}.json`), JSON.stringify(script, null, 1));
  console.error(`[convert] ${records.length} 条 → chats/${out}.json(${script.track.length} 指令)`);
}

// 仅作为 CLI 直跑时执行(被 vitest import 纯函数时不跑)。
if (process.argv[1] && import.meta.url === new URL(`file://${process.argv[1]}`).href) {
  main().catch((e) => {
    console.error(e);
    process.exit(1);
  });
}
