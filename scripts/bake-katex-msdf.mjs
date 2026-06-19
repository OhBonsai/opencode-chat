#!/usr/bin/env node
// bake-katex-msdf(Plan 12 ④)— 把 **RaTeX 用的 KaTeX 字体**(Computer Modern 系列)编译成 MSDF。
//
// RaTeX 排版发出的数学字形按 KaTeX 字族(Main/Math/Size1–4/AMS/Caligraphic/Fraktur…)分;每族一套
// 字形。本脚本对**每个真用到的字族**(`scripts/katex/charset/manifest.txt`,由 RaTeX 语料收集,见
// `crates/core/tests/dump_katex_charset.rs`)跑 msdf-bmfont,只烘该族实际出现的字符,产出 per-font
// BMFont MSDF(json + png)到 `web/public/fonts/katex-msdf/<Base>.{json,png}`(gitignore,可重生)。
//
// 用法:
//   1) 收集字符集(改语料后):cargo test -p infinite-chat-core --test dump_katex_charset -- --ignored
//   2) 编译:node scripts/bake-katex-msdf.mjs   (需 RaTeX/ 工作区 + 全局 msdf-bmfont)
//
// 依赖:msdf-bmfont-xml(npm i -g msdf-bmfont-xml);字体源 RaTeX/fonts/KaTeX_*.ttf。
//
// 运行时接入(Plan 12 ④,GPU 实跑,见 plan12_progress.md):这些 per-font atlas 需合并进**单页**
// (backend MSDF 是 D2Array,页尺寸须与 lxgw-msdf 一致)并按合成键 `role*0x110000 + codepoint` 索引;
// wasm `resolve` 对数学角色(26–40)查该键命中 MSDF、未命中回退 TinySDF。合并需 png 合成(pngjs)。

import { execFileSync } from "node:child_process";
import { mkdirSync, existsSync, readFileSync, renameSync, readdirSync, rmSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const FONTS = resolve(ROOT, "RaTeX/fonts");
const CHARSET = resolve(ROOT, "scripts/katex/charset");
const OUT = resolve(ROOT, "web/public/fonts/katex-msdf");
const SIZE = 42; // 每字形 MSDF 像素(与 lxgw 相近)
const RANGE = 4; // 距离场像素范围
const CLI = "msdf-bmfont";

if (!existsSync(FONTS)) fail(`找不到 ${FONTS} —— 需本地 RaTeX/ 工作区(见 plan12)。`);
const manifestPath = resolve(CHARSET, "manifest.txt");
if (!existsSync(manifestPath))
  fail(`找不到 ${manifestPath} —— 先跑:cargo test --test dump_katex_charset -- --ignored`);

const bases = readFileSync(manifestPath, "utf8").split("\n").map((s) => s.trim()).filter(Boolean);
mkdirSync(OUT, { recursive: true });
let ok = 0;
for (const base of bases) {
  const ttf = resolve(FONTS, `KaTeX_${base}.ttf`);
  const charset = resolve(CHARSET, `${base}.txt`);
  if (!existsSync(ttf)) { console.warn(`⚠ 缺字体 ${ttf}`); continue; }
  if (!existsSync(charset)) { console.warn(`⚠ 缺字符集 ${charset}`); continue; }
  // msdf-bmfont 以 font-face 名命名输出(KaTeX_<Base>.json / <Base>.png via -o)。先烘到临时前缀,再规整。
  const stem = resolve(OUT, base);
  try {
    execFileSync(
      CLI,
      [ttf, "-i", charset, "-m", "1024,1024", "-s", String(SIZE), "-r", String(RANGE),
       "-t", "msdf", "-f", "json", "-o", stem],
      { stdio: "pipe" },
    );
  } catch (e) {
    console.warn(`⚠ 烘 ${base} 失败:${e.message?.split("\n")[0]}`);
    continue;
  }
  // 规整产物名:msdf-bmfont 写 `KaTeX_<Base>.json`(font-face 名)+ `<Base>.png`(-o 名)。
  const facedJson = resolve(OUT, `KaTeX_${base}.json`);
  const wantJson = resolve(OUT, `${base}.json`);
  if (existsSync(facedJson)) renameSync(facedJson, wantJson);
  const meta = JSON.parse(readFileSync(wantJson, "utf8"));
  console.log(`✓ ${base}: ${meta.chars.length} 字形 · ${meta.common.scaleW}×${meta.common.scaleH} · 1 页`);
  ok++;
}
// 清理 msdf-bmfont 可能留下的杂项(.fnt 等)。
for (const f of readdirSync(OUT)) {
  if (!/\.(json|png)$/.test(f)) rmSync(resolve(OUT, f), { force: true });
}
console.log(`\n✓ 编译 ${ok}/${bases.length} 个 KaTeX 字族 → ${OUT}(per-font BMFont MSDF)`);
console.log("  运行时合并进单页 atlas + 合成键索引 = Plan 12 ④(GPU 实跑,见 plan12_progress.md)。");

function fail(m) { console.error(`✗ ${m}`); process.exit(1); }
