// find-bar.ts — Plan 21 P3(0030 步骤 4):自建页内查找(Cmd+F),跨**全历史**。
//
// 原生 Cmd+F 只覆盖 DOM 里的可见行(虚拟化 → 屏外块无 DOM)→ 查不到历史(0030 §7.7)。改自建:
// `chat.find(query)`(core 扫 Store 源文本,含屏外/Warm 块)定位命中所在 view → `scroll_to` 跳过去
// → 块 promote 回 Hot 后,在 `visible_text_runs` 的显示文本里定位 query → `set_selection` 精确选中。

interface FindHost {
  find(query: string): string;
  scroll_to(view: number): void;
  set_selection(flat: Uint32Array): void;
  visible_text_runs(): string;
}

interface Hit {
  view: number;
  char: number;
}
interface Run {
  block: number;
  char0: number;
  text: string;
}

const seg =
  typeof Intl !== "undefined" && "Segmenter" in Intl
    ? new Intl.Segmenter(undefined, { granularity: "grapheme" })
    : null;
const graphemeLen = (s: string) => (seg ? [...seg.segment(s)].length : s.length);
function graphemeIndex(text: string, cu: number): number {
  if (!seg || cu <= 0) return Math.min(cu, text.length);
  let n = 0;
  for (const s of seg.segment(text)) {
    if ((s as { index: number }).index >= cu) break;
    n += 1;
  }
  return n;
}

/** 在命中 view 跳到位后,于其可见行中定位 query 显示文本并选中。块从 Warm promote→Hot→重排上屏
 *  在重载流式下可能要 1–数秒,故持续重试至命中(上限 ~600 帧 ≈ 10s)再放弃,避免"跳到了却没选中"。 */
function selectMatch(host: FindHost, view: number, query: string): void {
  let tries = 0;
  const tick = () => {
    let runs: Run[];
    try {
      runs = JSON.parse(host.visible_text_runs()) as Run[];
    } catch {
      runs = [];
    }
    const run = runs.find((r) => r.block === view && r.text.includes(query));
    if (run) {
      const cu = run.text.indexOf(query);
      const start = run.char0 + graphemeIndex(run.text, cu);
      const end = start + graphemeLen(query);
      host.set_selection(new Uint32Array([view, start, end]));
      return;
    }
    if (tries++ < 600) requestAnimationFrame(tick); // 等块 promote + 重排上屏(重载下可能较久)
  };
  requestAnimationFrame(tick);
}

/** 挂 Cmd+F 查找条。返回卸载函数。 */
export function mountFindBar(host: FindHost): () => void {
  const bar = document.createElement("div");
  bar.className = "find-bar";
  bar.style.cssText =
    "position:fixed;top:10px;left:50%;transform:translateX(-50%);z-index:9998;display:none;" +
    "gap:6px;align-items:center;background:rgba(28,31,40,0.95);border:1px solid rgba(255,255,255,0.15);" +
    "border-radius:8px;padding:6px 8px;backdrop-filter:blur(6px);font-size:13px;color:#cdd3df";
  const input = document.createElement("input");
  input.type = "text";
  input.placeholder = "查找全历史…";
  input.className = "find-input";
  input.style.cssText =
    "background:rgba(0,0,0,0.3);border:1px solid rgba(255,255,255,0.12);border-radius:5px;" +
    "color:#e6e9f0;padding:3px 7px;outline:none;width:200px";
  const count = document.createElement("span");
  count.className = "find-count";
  count.style.cssText = "min-width:54px;text-align:right;opacity:0.75;font-variant-numeric:tabular-nums";
  bar.append(input, count);
  document.body.appendChild(bar);

  let hits: Hit[] = [];
  let idx = 0;

  const refresh = () => {
    const q = input.value;
    hits = q ? (JSON.parse(host.find(q)) as Hit[]) : [];
    idx = 0;
    count.textContent = hits.length ? `1/${hits.length}` : q ? "0/0" : "";
    if (hits.length) jump();
    else host.set_selection(new Uint32Array(0));
  };
  const jump = () => {
    const h = hits[idx];
    if (!h) return;
    count.textContent = `${idx + 1}/${hits.length}`;
    host.scroll_to(h.view);
    selectMatch(host, h.view, input.value);
  };
  const open = () => {
    bar.style.display = "flex";
    input.focus();
    input.select();
  };
  const close = () => {
    bar.style.display = "none";
    host.set_selection(new Uint32Array(0));
  };

  const onKeyDown = (e: KeyboardEvent) => {
    if ((e.ctrlKey || e.metaKey) && (e.key === "f" || e.key === "F")) {
      e.preventDefault();
      open();
    } else if (e.key === "Escape" && bar.style.display !== "none") {
      close();
    }
  };
  input.addEventListener("input", refresh);
  input.addEventListener("keydown", (e) => {
    if (e.key === "Enter") {
      e.preventDefault();
      if (!hits.length) return;
      idx = e.shiftKey ? (idx - 1 + hits.length) % hits.length : (idx + 1) % hits.length;
      jump();
    } else if (e.key === "Escape") {
      close();
    }
  });
  document.addEventListener("keydown", onKeyDown);
  return () => {
    document.removeEventListener("keydown", onKeyDown);
    bar.remove();
  };
}
