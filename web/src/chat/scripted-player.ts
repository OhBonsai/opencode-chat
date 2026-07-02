// scripted-player.ts — Plan 25 §2:剧本调度器。按时间轴驱动 driver(打字/推事件/点 Dock),
// 支持 播放/暂停/倍速/前向 seek(快放)。
//
// 设计要点:
// - **tick 驱动**(非内置 rAF)→ 纯逻辑可测:rAF 壳每帧调 `tick(performance.now())`;vitest 喂合成时间。
// - **虚拟时钟**:playing 时 `virtual += (now-lastNow)*speed`,倍速可中途改;暂停冻结 virtual。
// - **事件流累积不可逆**:前向 seek = 把 `at ≤ 目标` 的未触发指令**立即**触发(user 走 instant 秒填);
//   后向 seek 由宿主"重置引擎 + 从头快放"实现(R8 确定性保证结果一致),不在本类。
// - driver 与时钟全注入 → 与 DOM/wasm 解耦。

import { buildTimeline, timelineDuration, type ChatScript, type DockAction, type TimelineItem, type UserInstr } from "./script";

/** 宿主回调:player 只调这些,不碰 DOM/wasm 自身。 */
export interface ScriptDriver {
  /** 用户轮:打字机填输入框 → 停顿 → 发送。`instant`(seek 快放)= 秒填秒发,不逐字。
   * 返回 Promise 时,**时间轴暂停推进直到它 resolve**(打字门:后续事件——如用户气泡——必须
   * 等打字/发送完成才发,任何倍速/卡顿下顺序确定)。返回 void = 不门(旧行为/瞬时)。 */
  typeUser(u: UserInstr, instant: boolean): Promise<void> | void;
  /** 推一条 opencode 事件(已序列化的 JSON 原文)→ 走 push_event。 */
  pushEvent(rawJson: string): void;
  /** 替观看者点 Dock 真按钮。 */
  dock(action: DockAction): void;
  /** 进度回调(可选):`at` 当前虚拟时刻,`total` 总时长(ms)。 */
  onProgress?(at: number, total: number): void;
  /** 播完回调(可选)。 */
  onDone?(): void;
}

export interface PlayerOpts {
  /** 默认打字速率(字/秒),user.cps 缺省时用。 */
  defaultCps?: number;
}

export class ScriptedPlayer {
  private readonly timeline: TimelineItem[];
  private readonly total: number;
  private readonly driver: ScriptDriver;
  private virtual = 0;
  private cursor = 0; // 下一条待触发指令下标
  private playing = false;
  private speed = 1;
  private lastNow: number | null = null;
  /** 打字门(typeUser 返回的 Promise 未决):true = 冻结虚拟时钟,不推进不触发。 */
  private gated = false;

  constructor(script: ChatScript, driver: ScriptDriver, _opts: PlayerOpts = {}) {
    this.timeline = buildTimeline(script.track, 1); // 绝对时间轴(倍速在 tick 动态施加)
    this.total = timelineDuration(this.timeline);
    this.driver = driver;
  }

  /** 总时长(ms;时间轴末点)。 */
  duration(): number {
    return this.total;
  }

  /** 当前虚拟时刻(ms)。 */
  position(): number {
    return this.virtual;
  }

  isPlaying(): boolean {
    return this.playing;
  }

  play(): void {
    this.playing = true;
    this.lastNow = null; // 下一 tick 不把暂停期间算进 virtual
  }

  pause(): void {
    this.playing = false;
  }

  setSpeed(s: number): void {
    if (s > 0) this.speed = s;
  }

  /** 每帧推进(rAF 壳传 performance.now();vitest 传合成时间)。 */
  tick(now: number): void {
    if (!this.playing || this.gated) {
      this.lastNow = now; // 暂停/打字门期间不累积虚拟时间
      return;
    }
    if (this.lastNow === null) this.lastNow = now;
    this.virtual += (now - this.lastNow) * this.speed;
    this.lastNow = now;
    this.fireDue(false);
    this.driver.onProgress?.(Math.min(this.virtual, this.total), this.total);
    if (this.cursor >= this.timeline.length) {
      this.playing = false;
      this.driver.onDone?.();
    }
  }

  /** 前向快放到 `targetMs`:把 `at ≤ target` 的未触发指令**立即**触发(user 秒填)。 */
  seekForward(targetMs: number): void {
    if (targetMs < this.virtual) return; // 后向由宿主 reset+重放,本类不处理
    this.virtual = targetMs;
    this.fireDue(true);
    this.driver.onProgress?.(Math.min(this.virtual, this.total), this.total);
  }

  private fireDue(instant: boolean): void {
    while (this.cursor < this.timeline.length && this.timeline[this.cursor].at <= this.virtual) {
      this.fire(this.timeline[this.cursor], instant);
      this.cursor += 1;
    }
  }

  private fire(item: TimelineItem, instant: boolean): void {
    if (item.user) {
      const p = this.driver.typeUser(item.user, instant);
      if (p && !instant) {
        // 打字门:冻结时间轴直到打字+发送完成(顺序确定,与倍速/负载无关)。
        this.gated = true;
        void p.finally(() => {
          this.gated = false;
          this.lastNow = null; // 门内耗时不折算进虚拟时间
        });
      }
    } else if (item.event) {
      this.driver.pushEvent(JSON.stringify({ type: item.event.type, properties: item.event.properties ?? {} }));
    } else if (item.dock) {
      this.driver.dock(item.dock);
    }
  }
}
