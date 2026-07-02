// script.ts — Plan 25 §1:/chat 剧本格式(指令时间轴)的类型 + 运行时校验 + 时间轴展开。
//
// 剧本 = 一串**指令**,每条带 `dt`(距上一条的毫秒,手插一条不用重排全轴)。三种指令:
//   ① user  —— 输入框逐字打出 → 停顿 → 发送(发送本身不产画布内容;紧随的 message.updated 事件才是气泡)
//   ② event —— opencode 事件**原样 JSON 对象**(player 序列化后 push_event,走真实事件路径)
//   ③ dock  —— 替观看者点真按钮(allow/deny/answer),走真 DOM click 代码路径
//
// 纯逻辑(无 DOM/wasm):类型 + `parseScript`(载入即校验,错误指向条目下标)+ `buildTimeline`
// (dt→绝对时间轴)。均 vitest 可测。

/** 用户轮:模拟打字 + 发送。 */
export interface UserInstr {
  /** 要打出的文本。 */
  text: string;
  /** 打字速率(字/秒);缺省由 player 定默认。 */
  cps?: number;
  /** 打完到"按发送"的停顿(ms)。 */
  holdMs?: number;
}

/** opencode 事件(原样对象;player 序列化后喂 push_event)。 */
export interface EventInstr {
  type: string;
  properties?: Record<string, unknown>;
}

/** Dock 应答动作。 */
export type DockAction = "allow" | "deny" | "answer";

/** 一条剧本指令(三选一;`dt` = 距上一条毫秒)。 */
export interface ScriptItem {
  dt: number;
  user?: UserInstr;
  event?: EventInstr;
  dock?: DockAction;
}

export interface ScriptMeta {
  title?: string;
  version?: number;
  /** Plan 26①:可选主题名(载 web/public/themes/<theme>.json)或内联 token 覆盖。 */
  theme?: string | Record<string, unknown>;
}

export interface ChatScript {
  meta: ScriptMeta;
  track: ScriptItem[];
}

/** 校验结果:成功带规范化 script;失败带人读错误 + 出错条目下标(-1 = 顶层错误)。 */
export type ParseResult =
  | { ok: true; script: ChatScript }
  | { ok: false; error: string; index: number };

const DOCK_ACTIONS: readonly DockAction[] = ["allow", "deny", "answer"];

function isObj(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

/** 解析 + 校验剧本(JSON 已 parse 的对象)。错误信息指向具体条目下标,便于手编定位。 */
export function parseScript(raw: unknown): ParseResult {
  if (!isObj(raw)) return { ok: false, error: "剧本必须是对象", index: -1 };
  const meta = isObj(raw.meta) ? (raw.meta as ScriptMeta) : {};
  const track = raw.track;
  if (!Array.isArray(track)) return { ok: false, error: "缺 track 数组", index: -1 };

  const items: ScriptItem[] = [];
  for (let i = 0; i < track.length; i++) {
    const it = track[i];
    if (!isObj(it)) return { ok: false, error: "指令必须是对象", index: i };
    if (typeof it.dt !== "number" || !Number.isFinite(it.dt) || it.dt < 0) {
      return { ok: false, error: "dt 必须是 ≥0 的有限数", index: i };
    }
    const kinds = ["user", "event", "dock"].filter((k) => it[k] !== undefined);
    if (kinds.length !== 1) {
      return { ok: false, error: `指令须恰含 user|event|dock 之一(现 ${kinds.length} 个)`, index: i };
    }
    if (it.user !== undefined) {
      const u = it.user;
      if (!isObj(u) || typeof u.text !== "string") {
        return { ok: false, error: "user.text 必须是字符串", index: i };
      }
      if (u.cps !== undefined && (typeof u.cps !== "number" || u.cps <= 0)) {
        return { ok: false, error: "user.cps 必须是正数", index: i };
      }
    }
    if (it.event !== undefined) {
      const e = it.event;
      if (!isObj(e) || typeof e.type !== "string" || e.type.length === 0) {
        return { ok: false, error: "event.type 必须是非空字符串", index: i };
      }
      if (e.properties !== undefined && !isObj(e.properties)) {
        return { ok: false, error: "event.properties 必须是对象", index: i };
      }
    }
    if (it.dock !== undefined && !DOCK_ACTIONS.includes(it.dock as DockAction)) {
      return { ok: false, error: `dock 必须是 ${DOCK_ACTIONS.join("/")} 之一`, index: i };
    }
    items.push(it as unknown as ScriptItem);
  }
  return { ok: true, script: { meta, track: items } };
}

/** 时间轴上一条已展开指令:`at` = 绝对时间(ms,从 0 起);`speed` 缩放已应用。 */
export interface TimelineItem extends ScriptItem {
  at: number;
}

/** dt(相对)→ 绝对时间轴。`speed>1` 快放(除 dt)。纯函数(vitest)。 */
export function buildTimeline(track: ScriptItem[], speed = 1): TimelineItem[] {
  const s = speed > 0 ? speed : 1;
  let t = 0;
  return track.map((it) => {
    t += it.dt / s;
    return { ...it, at: t };
  });
}

/** 剧本总时长(ms,含末条打字/停顿的粗估由 player 另算;此处仅时间轴末点)。 */
export function timelineDuration(timeline: TimelineItem[]): number {
  return timeline.length ? timeline[timeline.length - 1].at : 0;
}
