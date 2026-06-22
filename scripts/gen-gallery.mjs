#!/usr/bin/env node
// gen-gallery —— 内联 spec/plan/{deck-icons,tool-icons}.frag → web/public/gallery.html。
// 自包含 WebGL2 页:两段动态 SDF 图标 —— ① ShaderBox deck(50,10×5) ② Tool icons(16,4×4)。
// 青/靛配色。用法:node scripts/gen-gallery.mjs
import { readFileSync, writeFileSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const deckFrag = readFileSync(resolve(ROOT, "spec/plan/deck-icons.frag"), "utf8");
const toolFrag = readFileSync(resolve(ROOT, "spec/plan/tool-icons.frag"), "utf8");

// deck:icon_id 0..49 行主序(顶→底,10 列)
const DECK = [
  "void","justice","strength","death","wall","temperance","branch","hanged man","high priestess","moon",
  "emperor","hierophant","tower","merge","hope","temple","summit","diamond","hermit","intuition",
  "stone","mountain","shadow","opposite","oak","ripples","empress","bundle","devil","sun",
  "star","judgement","wheel","vision","lovers","magician","link","holding","chariot","loop",
  "turning","trinity","cauldron","elders","core","inner truth","world","fool","enlighten","elements",
];
// tool:tool-icons.frag case 0..15 行主序
const TOOLS = [
  "read","write","edit","grep",
  "glob","shell","task","web fetch",
  "web search","todo","skill","patch",
  "question","plan·enter","plan·exit","invalid",
];

const labelCells = (arr, cols, fs) =>
  `<div class="labels" style="grid-template-columns:repeat(${cols},1fr);font-size:${fs}px">` +
  arr.map((l) => `<div>${l}</div>`).join("") + `</div>`;

const fragScript = (id, body) =>
  `<script type="x-shader/x-fragment" id="${id}">#version 300 es
precision highp float;
uniform vec2 iResolution;
uniform float iTime;
out vec4 outColor;

${body}

void main(){
  vec4 c; mainImage(c, gl_FragCoord.xy);
  float l = c.r;
  vec3 BG = vec3(0.102, 0.063, 0.251);
  vec3 FG = vec3(0.239, 0.961, 0.816);
  outColor = vec4(mix(BG, FG, clamp(l,0.0,1.0)), 1.0);
}
</script>`;

const html = `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1" />
<title>opencode · shaderbox icon gallery</title>
<style>
  :root { --fg:#3df5d0; --bg:#1a1040; --ink:#0d0a22; }
  html,body { margin:0; background:var(--ink); color:#e8eaf2;
    font:15px/1.5 system-ui,-apple-system,Segoe UI,Roboto,sans-serif; }
  .wrap { max-width:980px; margin:0 auto; padding:28px 20px 70px; }
  h1 { font-size:20px; letter-spacing:.4px; margin:0 0 4px; }
  h1 b { color:var(--fg); }
  .sub { opacity:.65; margin:0 0 24px; font-size:13px; }
  .sub a { color:var(--fg); text-decoration:none; }
  h2 { font-size:14px; font-weight:600; letter-spacing:.3px; margin:26px 0 10px; color:#cfe9e2; }
  h2 span { opacity:.5; font-weight:400; }
  .stage { position:relative; width:100%;
    border:1px solid #ffffff14; border-radius:12px; overflow:hidden; background:var(--bg); }
  .stage.deck { aspect-ratio:2/1; }
  .stage.tools { aspect-ratio:1/1; max-width:720px; }
  canvas { display:block; width:100%; height:100%; }
  .labels { position:absolute; inset:0; display:grid; pointer-events:none; }
  .labels div { display:flex; align-items:flex-end; justify-content:center; padding-bottom:3px;
    color:#bfeee2; opacity:.85; text-align:center; line-height:1.05; overflow:hidden; }
  .foot { margin-top:22px; font-size:12px; opacity:.55; }
  .foot a { color:var(--fg); text-decoration:none; }
</style>
</head>
<body>
<div class="wrap">
  <h1><b>opencode</b> · shaderbox icon gallery</h1>
  <p class="sub">纯片元 SDF 程序化图标,任意缩放锐利。青 #3DF5D0 / 靛 #1A1040。<a href="./">← 回引擎演示</a></p>

  <h2>ShaderBox deck <span>— 50 icon (PixelSpirit, plan16 §2.5)</span></h2>
  <div class="stage deck">
    <canvas id="deck"></canvas>
    ${labelCells(DECK, 10, 8)}
  </div>

  <h2>Tool icons <span>— 16 内置工具 (shader-first)</span></h2>
  <div class="stage tools">
    <canvas id="tools"></canvas>
    ${labelCells(TOOLS, 4, 11)}
  </div>

  <p class="foot">每个图标=一段 fragment shader。源:<a href="https://github.com/OhBonsai/infinite-chat">infinite-chat</a>。</p>
</div>

<script type="x-shader/x-vertex" id="vs">#version 300 es
void main(){ vec2 p=vec2((gl_VertexID<<1)&2, gl_VertexID&2); gl_Position=vec4(p*2.0-1.0,0.0,1.0); }
</script>
${fragScript("deck-fs", deckFrag)}
${fragScript("tool-fs", toolFrag)}

<script>
function makeStage(canvasId, fsId) {
  const cv = document.getElementById(canvasId);
  const gl = cv.getContext("webgl2", { antialias:true });
  if (!gl) { cv.parentElement.innerHTML =
    "<p style='padding:20px;color:#fbb'>此浏览器不支持 WebGL2(用 Chrome / Edge / Firefox)。</p>"; return; }
  const compile = (type, src) => { const s=gl.createShader(type); gl.shaderSource(s,src); gl.compileShader(s);
    if(!gl.getShaderParameter(s,gl.COMPILE_STATUS)) throw new Error(canvasId+": "+gl.getShaderInfoLog(s)); return s; };
  const prog = gl.createProgram();
  gl.attachShader(prog, compile(gl.VERTEX_SHADER, document.getElementById("vs").textContent.trim()));
  gl.attachShader(prog, compile(gl.FRAGMENT_SHADER, document.getElementById(fsId).textContent.trim()));
  gl.linkProgram(prog);
  if(!gl.getProgramParameter(prog, gl.LINK_STATUS)) throw new Error(gl.getProgramInfoLog(prog));
  gl.useProgram(prog);
  const uRes = gl.getUniformLocation(prog, "iResolution");
  const uTime = gl.getUniformLocation(prog, "iTime");
  gl.bindVertexArray(gl.createVertexArray());
  const dpr = Math.min(window.devicePixelRatio || 1, 2);
  function resize(){ const w=Math.round(cv.clientWidth*dpr), h=Math.round(cv.clientHeight*dpr);
    if(cv.width!==w||cv.height!==h){ cv.width=w; cv.height=h; } }
  const t0 = performance.now();
  function frame(){ resize(); gl.viewport(0,0,cv.width,cv.height); gl.useProgram(prog);
    gl.uniform2f(uRes, cv.width, cv.height); gl.uniform1f(uTime, (performance.now()-t0)/1000);
    gl.drawArrays(gl.TRIANGLES, 0, 3); requestAnimationFrame(frame); }
  requestAnimationFrame(frame);
}
try { makeStage("deck", "deck-fs"); } catch(e){ console.error(e); }
try { makeStage("tools", "tool-fs"); } catch(e){ console.error(e); }
</script>
</body>
</html>
`;

const out = resolve(ROOT, "web/public/gallery.html");
writeFileSync(out, html);
console.log(`wrote ${out} (${(html.length / 1024).toFixed(1)} KB, deck+tool inlined)`);
