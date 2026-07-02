// dock.ts — Plan 22 P4(0030 §3.4 / 0022 overlay):权限 / 反问 Dock。
//
// 会话被阻塞(FSM `Blocked{permission|question}`)时弹一个 DOM 浮层让用户应答;应答 → `reply_*`
// 解阻(host 另行 POST 真实 reply)。读 `session_status()`(core 单一真相),不在 JS 自存阻塞态。
// Dock 阻塞 turn:在它消失前会话保持 blocked(core 侧 FSM 已保证)。

interface DockChat {
  session_status(): string;
  reply_permission(): void;
  reply_question(): void;
}

let dock: HTMLDivElement | null = null;
let shownFor = ""; // 当前已弹的阻塞态(避免每帧重建)
let prevFocus: HTMLElement | null = null; // Plan 26②:打开前的焦点,关闭时还原

function ensureDock(): HTMLDivElement {
  if (dock) return dock;
  dock = document.createElement("div");
  dock.className = "session-dock";
  // Plan 26②:阻塞性对话 → alertdialog(读屏立即感知);焦点进出由 pumpDock 管理。
  dock.setAttribute("role", "alertdialog");
  dock.setAttribute("aria-modal", "true");
  dock.setAttribute("aria-label", "需要确认");
  dock.style.cssText =
    "position:fixed;left:50%;bottom:24px;transform:translateX(-50%);z-index:9997;display:none;" +
    "gap:10px;align-items:center;background:rgba(28,31,40,0.96);border:1px solid rgba(255,255,255,0.18);" +
    "border-radius:10px;padding:10px 14px;backdrop-filter:blur(6px);font-size:13px;color:#e6e9f0;" +
    "box-shadow:0 6px 24px rgba(0,0,0,0.4)";
  document.body.appendChild(dock);
  return dock;
}

function render(chat: DockChat, kind: "permission" | "question"): void {
  const d = ensureDock();
  d.innerHTML = "";
  const label = document.createElement("span");
  label.className = "dock-label";
  label.textContent = kind === "permission" ? "工具请求权限" : "助手有一个问题";
  d.appendChild(label);

  const mkBtn = (text: string, primary: boolean, onClick: () => void) => {
    const b = document.createElement("button");
    b.type = "button";
    b.className = primary ? "dock-allow" : "dock-deny";
    b.textContent = text;
    b.style.cssText =
      "cursor:pointer;font-size:12px;padding:5px 12px;border-radius:6px;border:1px solid " +
      (primary ? "rgba(90,150,255,0.6);background:rgba(60,110,220,0.85)" : "rgba(255,255,255,0.18);background:rgba(50,54,64,0.85)") +
      ";color:#fff";
    b.addEventListener("click", onClick);
    return b;
  };

  if (kind === "permission") {
    d.appendChild(mkBtn("允许", true, () => chat.reply_permission()));
    d.appendChild(mkBtn("拒绝", false, () => chat.reply_permission()));
  } else {
    d.appendChild(mkBtn("回答", true, () => chat.reply_question()));
  }
  d.style.display = "flex";
}

/** 一帧:据会话态弹/收 Dock(main rAF 调)。 */
export function pumpDock(chat: DockChat): void {
  const status = chat.session_status();
  const want = status.startsWith("blocked:") ? status : "";
  if (want === shownFor) return; // 态未变 → 不重建
  const wasOpen = shownFor !== "";
  shownFor = want;
  if (want === "blocked:permission" || want === "blocked:question") {
    // Plan 26②:记录当前焦点 → 弹 Dock → 焦点入首按钮(键盘/读屏直达)。
    if (!wasOpen) prevFocus = document.activeElement as HTMLElement | null;
    render(chat, want === "blocked:permission" ? "permission" : "question");
    dock?.querySelector<HTMLButtonElement>("button")?.focus();
  } else if (dock) {
    dock.style.display = "none";
    // 焦点还原到打开前位置(还在文档里才还原)。
    if (wasOpen && prevFocus?.isConnected) prevFocus.focus();
    prevFocus = null;
  }
}
