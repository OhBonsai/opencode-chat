#!/usr/bin/env node
// Dump opencode server 的 OpenAPI 路径,确认真实 API 形态(路径随版本变)。
// 用法:
//   node scripts/api-paths.mjs
//   SERVER=http://localhost:4096 node scripts/api-paths.mjs

const port = process.env.PORT ?? "4096";
const server = (process.env.SERVER ?? `http://localhost:${port}`).replace(/\/$/, "");

async function tryFetch(ep) {
  try {
    const res = await fetch(`${server}${ep}`);
    if (!res.ok) return null;
    return await res.json();
  } catch { return null; }
}

console.log(`▶ 探测 ${server} 的 OpenAPI ...`);
let spec = null, hit = null;
for (const ep of ["/doc", "/openapi.json", "/openapi"]) {
  spec = await tryFetch(ep);
  if (spec) { hit = ep; break; }
}

if (!spec) {
  console.error("✗ /doc 与 /openapi.json 都拿不到。确认 server 起没起、端口对不对。");
  console.error(`  也可直接试: curl -N ${server}/api/event   (应保持连接、推 server.connected)`);
  process.exit(1);
}

console.log(`✓ 命中 ${server}${hit}`);
const paths = Object.keys(spec.paths ?? {}).sort();
if (paths.length === 0) {
  console.log("(无 paths 字段,原始片段:)");
  console.log(JSON.stringify(spec).slice(0, 2000));
} else {
  console.log("── paths ──");
  for (const p of paths) {
    const methods = Object.keys(spec.paths[p]).join(",").toUpperCase();
    console.log(`  ${methods.padEnd(18)} ${p}`);
  }
  // 高亮我们关心的
  console.log("\n── Plan 1 关心的 ──");
  for (const p of paths) {
    if (/\/(event|session)(\/|$)/.test(p) && /\/(event|message|session)$/.test(p)) {
      console.log(`  ${p}`);
    }
  }
}
