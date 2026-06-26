// text-layer.ts — Plan 21 P2(0030 步骤 2):虚拟化透明 DOM 文本层。
//
// 在 canvas 之上叠一层**仅含可见行**的透明 `<span>`(原生可选中/可 Cmd+C/可 Cmd+F);视觉高亮交给
// GPU(core 据字符区间发 SELECTION FrameRect 画在文字下),DOM `::selection` 透明 → 不双高亮、不遮文字。
//
// 关键不变量(0030 §7):
//  - **虚拟化**:span 数 ∝ 可见行(读 `visible_text_runs()`,仅 Hot 可见块)→ DOM 不随历史涨。
//  - **不遮画布**:容器 `pointer-events:none`,仅 span `auto`(空白处事件落到 canvas)。滚轮转发回 canvas。
//  - **暗主题**:`color-scheme: only light` 防强制反色;选区色走引擎,不继承浏览器蓝。
//  - **字符序正确即可**(不追像素对齐,绕开 PDF.js scaleX 痛点):DOM 选区→字符区间用 grapheme 计数映射。

interface TextHost {
  visible_text_runs(): string;
  set_selection(flat: Uint32Array): void;
}

interface ScreenRun {
  block: number;
  char0: number;
  x: number;
  y: number;
  w: number;
  h: number;
  text: string;
}

let layer: HTMLDivElement | null = null;
const spans = new Map<string, HTMLSpanElement>(); // key = `${block}:${char0}`
const seg =
  typeof Intl !== "undefined" && "Segmenter" in Intl
    ? new Intl.Segmenter(undefined, { granularity: "grapheme" })
    : null;

/** UTF-16 code-unit 偏移 → 显示字形(grapheme)下标。CJK/emoji 下 DOM 偏移≠字形数,需换算。 */
function graphemeIndex(text: string, cuOffset: number): number {
  if (cuOffset <= 0) return 0;
  if (!seg) return Math.min(cuOffset, text.length); // 退化:近似按 code unit
  let n = 0;
  for (const s of seg.segment(text)) {
    if ((s as { index: number }).index >= cuOffset) break;
    n += 1;
  }
  return n;
}

function ensureLayer(canvas: HTMLCanvasElement): HTMLDivElement {
  if (layer) return layer;
  // 一次性注入 ::selection 透明(DOM 不画高亮,交 GPU)。
  const style = document.createElement("style");
  style.textContent =
    ".text-layer ::selection{background:transparent}.text-layer ::-moz-selection{background:transparent}";
  document.head.appendChild(style);

  layer = document.createElement("div");
  layer.className = "text-layer";
  // 容器全屏但 pointer-events:none(空白落 canvas);仅 span auto。z 在画布上、复制按钮/面板下。
  layer.style.cssText =
    "position:fixed;inset:0;z-index:52;overflow:hidden;pointer-events:none;color-scheme:only light";
  // 滚轮转发回 canvas(文本层在画布之上,否则滚动失效)。span 上的 wheel 冒泡到此。
  layer.addEventListener(
    "wheel",
    (e) => {
      canvas.dispatchEvent(
        new WheelEvent("wheel", {
          deltaX: e.deltaX,
          deltaY: e.deltaY,
          ctrlKey: e.ctrlKey,
          clientX: e.clientX,
          clientY: e.clientY,
          bubbles: true,
          cancelable: true,
        }),
      );
      e.preventDefault();
    },
    { passive: false },
  );
  document.body.appendChild(layer);
  return layer;
}

/** 一帧:把可见行同步成透明 span(main rAF 调,同 embed-overlay)。 */
export function pumpTextLayer(host: TextHost, canvas: HTMLCanvasElement): void {
  let runs: ScreenRun[];
  try {
    runs = JSON.parse(host.visible_text_runs()) as ScreenRun[];
  } catch {
    return;
  }
  const root = ensureLayer(canvas);
  const dpr = window.devicePixelRatio || 1;
  const seen = new Set<string>();
  for (const r of runs) {
    const key = `${r.block}:${r.char0}`;
    seen.add(key);
    let span = spans.get(key);
    if (!span) {
      span = document.createElement("span");
      span.dataset.block = String(r.block);
      span.dataset.char0 = String(r.char0);
      // 透明文字、可选中、保留空白;auto 命中以供选区。字号≈行高,宽设为行宽(近似字符映射)。
      span.style.cssText =
        "position:absolute;white-space:pre;color:transparent;user-select:text;-webkit-user-select:text;" +
        "pointer-events:auto;cursor:text;margin:0;padding:0;overflow:hidden";
      root.appendChild(span);
      spans.set(key, span);
    }
    if (span.textContent !== r.text) span.textContent = r.text;
    // 设备像素 → CSS 像素。
    span.style.left = `${r.x / dpr}px`;
    span.style.top = `${r.y / dpr}px`;
    span.style.width = `${r.w / dpr}px`;
    span.style.height = `${r.h / dpr}px`;
    span.style.fontSize = `${(r.h / dpr) * 0.82}px`;
    span.style.lineHeight = `${r.h / dpr}px`;
  }
  // 回收滚出视口 / 卸载的行。
  for (const [k, span] of spans) {
    if (!seen.has(k)) {
      span.remove();
      spans.delete(k);
    }
  }
}

/** 把当前 DOM 选区映成 `[(block,start,end)]` 扁平三元组(按块聚合;grapheme 下标)。 */
function selectionToRanges(): Uint32Array {
  const sel = window.getSelection();
  if (!sel || sel.isCollapsed || sel.rangeCount === 0) return new Uint32Array(0);
  const range = sel.getRangeAt(0);
  // 每块取被选 span 的最小起点 / 最大终点(块内显示字形序;v1 行级足够)。
  const perBlock = new Map<number, { start: number; end: number }>();
  for (const [, span] of spans) {
    if (!range.intersectsNode(span)) continue;
    const block = Number(span.dataset.block);
    const char0 = Number(span.dataset.char0);
    const text = span.textContent ?? "";
    const node = span.firstChild;
    // 该 span 内被选的 code-unit 区间:端点落在本 span 则取其 offset,否则整行入选。
    let cuStart = 0;
    let cuEnd = text.length;
    if (node && range.startContainer === node) cuStart = range.startOffset;
    if (node && range.endContainer === node) cuEnd = range.endOffset;
    const gStart = char0 + graphemeIndex(text, cuStart);
    const gEnd = char0 + graphemeIndex(text, cuEnd);
    const cur = perBlock.get(block);
    if (cur) {
      cur.start = Math.min(cur.start, gStart);
      cur.end = Math.max(cur.end, gEnd);
    } else {
      perBlock.set(block, { start: gStart, end: gEnd });
    }
  }
  const flat: number[] = [];
  for (const [block, { start, end }] of perBlock) {
    if (end > start) flat.push(block, start, end);
  }
  return new Uint32Array(flat);
}

function escapeHtml(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;");
}

/** 纯文本 → 简单 HTML(逐行 <div>),供富文本复制(text/html)保留换行结构。 */
function toHtml(text: string): string {
  const lines = text.split("\n").map((l) => `<div>${escapeHtml(l) || "<br>"}</div>`);
  return `<div>${lines.join("")}</div>`;
}

/** 富文本复制:同时写 text/plain + text/html(Safari 友好:同步构造 ClipboardItem);失败回退 writeText。 */
function richCopy(text: string): void {
  const w = navigator.clipboard;
  if (!w) return;
  try {
    const item = new ClipboardItem({
      "text/plain": new Blob([text], { type: "text/plain" }),
      "text/html": new Blob([toHtml(text)], { type: "text/html" }),
    });
    void w.write([item]).catch(() => void w.writeText(text).catch(() => {}));
  } catch {
    void w.writeText(text).catch(() => {});
  }
}

/** 选区是否落在文本层内(只接管文本层的复制,放行输入框等原生复制)。 */
function selectionInTextLayer(sel: Selection | null): boolean {
  if (!sel || sel.rangeCount === 0) return false;
  const a = sel.anchorNode;
  const el = a instanceof Element ? a : a?.parentElement;
  return !!el?.closest(".text-layer");
}

/** 挂选区监听:selectionchange(节流到 rAF)→ set_selection;copy → 写选区纯文本。返回卸载函数。 */
export function attachSelection(host: TextHost): () => void {
  let scheduled = false;
  const onChange = () => {
    if (scheduled) return;
    scheduled = true;
    requestAnimationFrame(() => {
      scheduled = false;
      // 仅当 DOM 选区落在文本层内才同步核心选区:
      //  - 在层内且非空 → 映成字符区间设入(用户拖选)。
      //  - 在层内且折叠(点了一下)→ 清空(取消选区)。
      //  - 不在层内(焦点在查找框/别处、或无选区)→ **不动**核心选区,避免误清 find 的程序化选区。
      if (!selectionInTextLayer(window.getSelection())) return;
      host.set_selection(selectionToRanges());
    });
  };
  // 透明文本层的复制(Plan 21 P3 富文本:同时给 text/plain + text/html,提保真),两条路保可靠:
  //  ① `copy` 事件 setData(真浏览器内菜单/快捷键触发的标准路径)。
  //  ② `keydown` Ctrl/Cmd+C 显式 clipboard.write([ClipboardItem]) —— 透明文本层在部分环境(含
  //     headless)不触发原生 `copy` 事件,故快捷键路兜底(Ctrl+C 是用户手势 → 写剪贴板获授权)。
  const onCopy = (e: ClipboardEvent) => {
    const sel = window.getSelection();
    if (!selectionInTextLayer(sel) || !sel || sel.isCollapsed) return;
    const text = sel.toString();
    e.clipboardData?.setData("text/plain", text);
    e.clipboardData?.setData("text/html", toHtml(text));
    e.preventDefault();
  };
  const onKey = (e: KeyboardEvent) => {
    if (!(e.ctrlKey || e.metaKey) || (e.key !== "c" && e.key !== "C")) return;
    const sel = window.getSelection();
    if (!selectionInTextLayer(sel) || !sel || sel.isCollapsed) return;
    e.preventDefault(); // 阻止浏览器默认复制(仅 text/plain)覆盖我们写入的富文本
    richCopy(sel.toString());
  };
  document.addEventListener("selectionchange", onChange);
  document.addEventListener("copy", onCopy);
  document.addEventListener("keydown", onKey);
  return () => {
    document.removeEventListener("selectionchange", onChange);
    document.removeEventListener("copy", onCopy);
    document.removeEventListener("keydown", onKey);
  };
}
