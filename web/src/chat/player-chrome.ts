// player-chrome.ts — Plan 25 PR-B:/chat 播放器进度条(形态复用 film/player.ts)。
//
// 播/停、倍速、scrubber、时间。seek 语义(Plan 25 §6):事件流是累积状态,不可逆放——
// **前向** seek = player.seekForward(到点指令立即触发);**后向** seek = 带 `?at=<ms>` 重载,
// 页面启动时从头快放到目标点(R8 确定性 → 结果一致)。

import type { ScriptedPlayer } from "./scripted-player";

const fmt = (ms: number) => {
  const s = Math.floor(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
};

const SPEEDS = [0.5, 1, 2, 4];

export function mountPlayerChrome(player: ScriptedPlayer, parent: HTMLElement = document.body): void {
  const bar = document.createElement("div");
  bar.className = "chat-player";
  bar.style.cssText =
    "position:fixed;left:0;right:0;bottom:var(--input-h);z-index:9997;display:flex;align-items:center;" +
    "gap:14px;padding:10px 18px;background:linear-gradient(#0d0f1700,#0d0f17cc);" +
    "font:12px/1 'JetBrains Mono',monospace;color:#cdd3e0;user-select:none;";

  const btn = document.createElement("button");
  btn.className = "chat-player-toggle";
  btn.style.cssText =
    "all:unset;cursor:pointer;width:30px;height:30px;border-radius:50%;border:1px solid #3df5d066;" +
    "color:#3df5d0;text-align:center;line-height:30px;flex:0 0 auto;";

  const track = document.createElement("div");
  track.style.cssText =
    "position:relative;flex:1;height:18px;cursor:pointer;display:flex;align-items:center;";
  const rail = document.createElement("div");
  rail.style.cssText =
    "position:absolute;left:0;right:0;height:3px;background:#222838;border-radius:2px;";
  const fill = document.createElement("div");
  fill.style.cssText =
    "position:absolute;left:0;height:3px;width:0;background:#3df5d0;border-radius:2px;";
  const knob = document.createElement("div");
  knob.style.cssText =
    "position:absolute;width:11px;height:11px;border-radius:50%;background:#3df5d0;left:0;transform:translateX(-50%);";
  track.append(rail, fill, knob);

  const time = document.createElement("span");
  time.style.cssText = "flex:0 0 auto;opacity:.7;min-width:74px;text-align:right;";

  const speeds = document.createElement("div");
  speeds.style.cssText = "display:flex;gap:6px;flex:0 0 auto;";
  let curSpeed = 1;
  const speedBtns = new Map<number, HTMLButtonElement>();
  for (const r of SPEEDS) {
    const sb = document.createElement("button");
    sb.style.cssText =
      "all:unset;cursor:pointer;padding:3px 7px;border-radius:5px;font-size:11px;color:#9aa3b5;border:1px solid #ffffff14;";
    sb.textContent = `${r}×`;
    sb.onclick = () => {
      curSpeed = r;
      player.setSpeed(r);
    };
    speedBtns.set(r, sb);
    speeds.appendChild(sb);
  }

  bar.append(btn, track, time, speeds);
  parent.appendChild(bar);

  btn.onclick = () => (player.isPlaying() ? player.pause() : player.play());

  const seekTo = (targetMs: number) => {
    if (targetMs >= player.position()) {
      player.seekForward(targetMs);
      return;
    }
    // 后向:重载 + ?at=(启动时从头快放到目标,R8 确定性)。
    const u = new URL(location.href);
    u.searchParams.set("at", String(Math.round(targetMs)));
    location.assign(u.toString());
  };
  const seekFromEvent = (e: MouseEvent) => {
    const rect = track.getBoundingClientRect();
    const pct = Math.max(0, Math.min(1, (e.clientX - rect.left) / rect.width));
    seekTo(pct * player.duration());
  };
  let dragging = false;
  track.addEventListener("mousedown", (e) => {
    dragging = true;
    seekFromEvent(e);
  });
  window.addEventListener("mousemove", (e) => {
    if (dragging) seekFromEvent(e);
  });
  window.addEventListener("mouseup", () => {
    dragging = false;
  });

  // chrome 自己每帧刷新(读 player 状态;不依赖 driver 回调,避免与进度回调抢线)。
  const refresh = () => {
    const total = player.duration();
    const pct = total ? Math.min(1, player.position() / total) : 0;
    fill.style.width = `${pct * 100}%`;
    knob.style.left = `${pct * 100}%`;
    btn.textContent = player.isPlaying() ? "⏸" : "▶";
    time.textContent = `${fmt(Math.min(player.position(), total))} / ${fmt(total)}`;
    for (const [r, sb] of speedBtns) {
      const on = curSpeed === r;
      sb.style.color = on ? "#3df5d0" : "#9aa3b5";
      sb.style.borderColor = on ? "#3df5d066" : "#ffffff14";
    }
    requestAnimationFrame(refresh);
  };
  requestAnimationFrame(refresh);
}
