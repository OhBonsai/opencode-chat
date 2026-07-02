// announcer.ts — Plan 26②(0030 步骤3):读屏 live region 播报器。
//
// **不逐 delta 播报**(会淹没读屏)——按「会话状态变化」粒度播:发送/回复中/请求权限/提问/
// 出错/完成。permission/question 用 `assertive`(阻塞性,需立即知晓),其余 `polite`。
// 节流 + 去重是**纯函数**(vitest);DOM 侧只是把结果写进视觉隐藏的 live region。

/** 状态迁移 → 播报文案(null = 不播)。纯函数。 */
export function statusAnnouncement(prev: string, next: string): string | null {
  if (prev === next) return null;
  if (next === "awaiting") return "已发送,等待回复";
  if (next === "streaming") return prev === "idle" || prev === "awaiting" ? "正在回复" : null;
  if (next === "blocked:permission") return "工具请求权限,需要确认";
  if (next === "blocked:question") return "助手有一个问题,需要回答";
  if (next === "errored") return "出错了";
  if (next === "stopped") return "已停止";
  if (next === "idle" && prev !== "" && prev !== "idle") return "回复完成";
  return null;
}

/** 阻塞态用 assertive(立即打断播报);其余 polite。 */
export function livenessOf(status: string): "assertive" | "polite" {
  return status.startsWith("blocked:") ? "assertive" : "polite";
}

/** 节流 + 去重状态(纯数据)。 */
export interface EmitState {
  lastMsg: string;
  lastAt: number;
}

/** 同文案 `minGapMs` 内不重播(去重);不同文案立即播。纯函数:返回 [是否播, 新状态]。 */
export function shouldEmit(
  state: EmitState,
  msg: string,
  now: number,
  minGapMs = 1500,
): [boolean, EmitState] {
  if (msg === state.lastMsg && now - state.lastAt < minGapMs) return [false, state];
  return [true, { lastMsg: msg, lastAt: now }];
}

interface AnnouncerHost {
  session_status(): string;
}

/** 挂播报器:视觉隐藏 live region + 每帧观测状态迁移。返回卸载函数。 */
export function mountAnnouncer(host: AnnouncerHost, parent: HTMLElement = document.body): () => void {
  const region = document.createElement("div");
  region.className = "sr-announcer";
  region.setAttribute("aria-live", "polite");
  region.setAttribute("aria-atomic", "true");
  // 视觉隐藏但读屏可及(标准 sr-only 手法;display:none 会被读屏忽略,不可用)。
  region.style.cssText =
    "position:absolute;width:1px;height:1px;margin:-1px;padding:0;overflow:hidden;" +
    "clip:rect(0 0 0 0);white-space:nowrap;border:0";
  parent.appendChild(region);

  let prev = "";
  let emit: EmitState = { lastMsg: "", lastAt: 0 };
  let raf = 0;
  const tick = () => {
    const cur = host.session_status();
    const msg = statusAnnouncement(prev, cur);
    if (msg) {
      const [ok, next] = shouldEmit(emit, msg, performance.now());
      if (ok) {
        emit = next;
        region.setAttribute("aria-live", livenessOf(cur));
        region.textContent = msg;
      }
    }
    prev = cur;
    raf = requestAnimationFrame(tick);
  };
  raf = requestAnimationFrame(tick);

  return () => {
    cancelAnimationFrame(raf);
    region.remove();
  };
}
