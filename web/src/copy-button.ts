// copy-button.ts — Plan 21 P1(0030 步骤 1):每条**可见消息**一个"复制"按钮。
//
// 每帧读 `chat.visible_turns()`(JSON,设备像素屏幕坐标)→ 按 `id` 复用一个浮层 `<button>`,摆在该
// 消息盒右上角;点击 → `navigator.clipboard.writeText(text)`(用户手势,满足 Clipboard 约束)→ 短暂
// "已复制 ✓"。滚出视口 / 卸载的按钮回收(同 `embed-overlay` 习惯)。**按钮数 ∝ 可见消息**(虚拟化,
// core 只吐 Hot 可见块)→ 不随历史增长。

interface CopyHost {
  visible_turns(): string;
}

interface ScreenTurn {
  id: number;
  turn: number;
  role: string;
  x: number;
  y: number;
  w: number;
  h: number;
  text: string;
}

let layer: HTMLDivElement | null = null;
const btns = new Map<number, HTMLButtonElement>();
const texts = new Map<number, string>(); // id → 最新渲染纯文本(点击时取)

function ensureLayer(): HTMLDivElement {
  if (!layer) {
    layer = document.createElement("div");
    // 覆盖全屏、容器本身不挡画布(pointer-events:none),按钮单独开 auto。z 在画布上、面板下。
    layer.style.cssText =
      "position:fixed;inset:0;z-index:55;pointer-events:none;overflow:hidden";
    document.body.appendChild(layer);
  }
  return layer;
}

function makeButton(id: number): HTMLButtonElement {
  const b = document.createElement("button");
  b.className = "copy-btn";
  b.type = "button";
  b.dataset.turnId = String(id); // 稳定身份(= view 下标)→ 跨帧/测试按 id 定位
  b.textContent = "复制";
  b.style.cssText =
    "position:absolute;pointer-events:auto;cursor:pointer;font-size:11px;line-height:1;" +
    "padding:3px 7px;border-radius:6px;border:1px solid rgba(255,255,255,0.18);" +
    "background:rgba(40,44,54,0.78);color:#cdd3df;backdrop-filter:blur(4px);" +
    "opacity:0.55;transition:opacity 0.12s;will-change:transform";
  b.addEventListener("mouseenter", () => (b.style.opacity = "1"));
  b.addEventListener("mouseleave", () => (b.style.opacity = "0.55"));
  b.addEventListener("click", () => {
    const text = texts.get(id) ?? "";
    void navigator.clipboard.writeText(text).then(
      () => flash(b, "已复制 ✓"),
      () => flash(b, "复制失败"),
    );
  });
  return b;
}

function flash(b: HTMLButtonElement, msg: string): void {
  b.textContent = msg;
  b.style.opacity = "1";
  window.setTimeout(() => {
    b.textContent = "复制";
    b.style.opacity = "0.55";
  }, 1100);
}

/** 一帧:把复制按钮同步到当前可见消息(main rAF 调,同 embed-overlay)。 */
export function pumpCopyButtons(host: CopyHost): void {
  let turns: ScreenTurn[];
  try {
    turns = JSON.parse(host.visible_turns()) as ScreenTurn[];
  } catch {
    return;
  }
  const root = ensureLayer();
  const dpr = window.devicePixelRatio || 1;
  const seen = new Set<number>();
  for (const t of turns) {
    seen.add(t.id);
    texts.set(t.id, t.text);
    let b = btns.get(t.id);
    if (!b) {
      b = makeButton(t.id);
      root.appendChild(b);
      btns.set(t.id, b);
    }
    // 设备像素 → CSS 像素(÷DPR)。摆消息盒右上角,略缩进。
    const right = (t.x + t.w) / dpr;
    const top = t.y / dpr;
    b.style.left = `${right - 52}px`;
    b.style.top = `${top + 2}px`;
  }
  // 回收滚出视口 / 卸载的按钮。
  for (const [id, b] of btns) {
    if (!seen.has(id)) {
      b.remove();
      btns.delete(id);
      texts.delete(id);
    }
  }
}
