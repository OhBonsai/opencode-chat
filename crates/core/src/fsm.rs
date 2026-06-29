//! fsm(M4)— 回合收尾判定(Plan2 I)。
//!
//! Plan2 聚焦最痛的"模型忘了发 idle → 永久 loading":用**多信号收敛 + 看门狗**判定回合
//! 是否收尾(0005 §4),而非只看 idle。投影语义(AR5):状态由"最近活动时刻 + 会话信号
//! + 注入时间"实时算出,不存独立状态机;任意信号到达都能重算,漏事件不卡死。
//!
//! 确定性(R8):只依赖注入的 `now_ms`(由 dt 累加),无墙钟。
//!
//! Part/Turn 的完整分组投影(AR11,跨消息扁平化)留作后续;本期收尾判定按"当前回合"
//! (全局最近活动)处理,足以解禁 loading。

/// 回合状态(投影)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TurnStatus {
    /// 还没开始(无任何活动)。
    Idle,
    /// 进行中(busy 或刚有活动)。
    Active,
    /// 表观停滞(soft 超时无活动,但未收到收尾信号)——可降级表现,仍可复活。
    Stalled,
    /// 已收尾(收到 idle/完成信号,或 hard 看门狗强制)。新活动可复活回 Active。
    Settled,
}

/// soft 看门狗:静默多久判 Stalled(ms)。
const SOFT_MS: f64 = 8_000.0;
/// hard 看门狗:静默多久强制 Settle(ms)。
const HARD_MS: f64 = 30_000.0;

/// 回合收尾跟踪器(投影)。
pub struct TurnTracker {
    now_ms: f64,
    last_activity_ms: f64,
    has_activity: bool,
    /// 会话 busy(session.status busy/retry)。
    busy: bool,
    /// 显式收尾信号(session idle / 消息完成)。
    settled_signal: bool,
}

impl Default for TurnTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl TurnTracker {
    pub fn new() -> Self {
        Self {
            now_ms: 0.0,
            last_activity_ms: 0.0,
            has_activity: false,
            busy: false,
            settled_signal: false,
        }
    }

    /// 推进时间(每帧)。
    pub fn tick(&mut self, now_ms: f64) {
        self.now_ms = now_ms;
    }

    /// 有 part 活动(delta/updated 到达)——刷新活动时刻,清收尾信号(回合复活)。
    pub fn on_activity(&mut self, now_ms: f64) {
        self.last_activity_ms = now_ms;
        self.has_activity = true;
        self.settled_signal = false;
    }

    /// 会话 busy(开始/重试)。
    pub fn on_busy(&mut self) {
        self.busy = true;
        self.settled_signal = false;
    }

    /// 显式收尾(session idle / 消息完成)。
    pub fn on_settle_signal(&mut self) {
        self.busy = false;
        self.settled_signal = true;
    }

    /// 当前回合状态(投影:实时算)。
    pub fn status(&self) -> TurnStatus {
        if !self.has_activity {
            return TurnStatus::Idle;
        }
        if self.settled_signal {
            return TurnStatus::Settled;
        }
        let quiet = self.now_ms - self.last_activity_ms;
        if self.busy || quiet < SOFT_MS {
            TurnStatus::Active
        } else if quiet < HARD_MS {
            TurnStatus::Stalled
        } else {
            // hard 看门狗:模型忘了 idle 也强制解禁(0005 §4)。
            TurnStatus::Settled
        }
    }

    /// 是否还在"忙"(供宿主显示 loading;Stalled 也算未收尾但已降级)。
    pub fn is_settled(&self) -> bool {
        matches!(self.status(), TurnStatus::Settled | TurnStatus::Idle)
    }
}

// ───────────────────────── SessionStatus FSM(Plan 22 P2 / 0031 §5.2) ─────────────────────────
//
// 富生命周期状态机:把散落的 boolean(is_active/can_send/streaming_id…)收敛成**单一联合 + 纯
// 转移函数**(R8 可重放 oracle)。`TurnTracker`(上面)继续管"收尾 + 看门狗投影";本 FSM 管
// "会话生命周期态"(发送/流式/重试/阻塞/停止/错误),由 app/controller 据事件驱动、派生量从它算。

/// 阻塞来源(Blocked 的子类)。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocker {
    /// 工具权限请求(permission.asked)。
    Permission,
    /// 模型反问(question.asked)。
    Question,
}

/// 会话生命周期状态(0031 §5.2)。带数据的态用最小必要载荷。
#[derive(Debug, Clone, PartialEq)]
pub enum SessionStatus {
    /// 空闲(可发送)。
    Idle,
    /// 已发送、等首个回包(起 no-reply 计时,F1)。
    AwaitingAck { sent_at: f64 },
    /// 流式中(某 message 正在产出)。
    Streaming { message_id: String },
    /// 重试中(F8)。
    Retrying { attempt: u32 },
    /// 被阻塞(等权限/答题;Dock,§3.4)。`resume_message_id` = 解阻后回到的流式消息(空 = 回 Idle)。
    Blocked {
        on: Blocker,
        resume_message_id: String,
    },
    /// 表观停滞(soft 看门狗;可复活)。
    Stalled,
    /// 用户停止(F11)。
    Stopped,
    /// 错误终止(F3/F4)。
    Errored { error: String },
}

/// 驱动 FSM 的归一化输入(由 app 从 protocol `Event` + 计时器投影而来;**不是**裸协议事件)。
#[derive(Debug, Clone, PartialEq)]
pub enum FsmInput {
    /// 用户发送(起等待)。
    Send {
        now_ms: f64,
    },
    /// 首个 part 到达(进入流式)。
    FirstPart {
        message_id: String,
    },
    /// 一般活动(delta/updated):停滞/等待 → 复活流式;流式态不变。
    Activity,
    /// 会话 busy(session.status busy)。
    Busy,
    /// 重试(session.status retry,F8)。
    Retry {
        attempt: u32,
    },
    /// 收尾(session idle / 完成 / hard 看门狗)。
    Idle,
    /// 权限请求 / 反问 → 阻塞。
    PermissionAsked,
    QuestionAsked,
    /// 权限/反问已应答 → 解阻(回流式或 Idle)。
    Replied,
    /// 用户停止。
    Stop,
    /// 错误终止(SessionError / message.updated.error)。
    Error {
        error: String,
    },
    /// soft 看门狗 → Stalled。
    SoftTimeout,
}

impl SessionStatus {
    /// 流式消息 id(仅 `Streaming` 有)。
    #[must_use]
    pub fn streaming_id(&self) -> Option<&str> {
        match self {
            SessionStatus::Streaming { message_id } => Some(message_id.as_str()),
            _ => None,
        }
    }

    /// 是否"活跃"(回合进行中 → 显示活跃指示、禁止再发)。
    #[must_use]
    pub fn is_active(&self) -> bool {
        matches!(
            self,
            SessionStatus::AwaitingAck { .. }
                | SessionStatus::Streaming { .. }
                | SessionStatus::Retrying { .. }
                | SessionStatus::Blocked { .. }
                | SessionStatus::Stalled
        )
    }

    /// 是否可发送(空闲态)。派生量从 status 算,杜绝散落 boolean。
    #[must_use]
    pub fn can_send(&self) -> bool {
        matches!(
            self,
            SessionStatus::Idle | SessionStatus::Stopped | SessionStatus::Errored { .. }
        )
    }

    /// 解阻时回到的流式消息(`Blocked.resume_message_id`,或当前流式 id)。
    fn resume_id(&self) -> String {
        match self {
            SessionStatus::Streaming { message_id } => message_id.clone(),
            SessionStatus::Blocked {
                resume_message_id, ..
            } => resume_message_id.clone(),
            _ => String::new(),
        }
    }
}

/// 会话 FSM 转移(**纯函数,穷尽 match;R8 oracle**)。`cur` 当前态 + `input` 归一化输入 → 新态。
/// 转移要点见 Plan 22 §4 / 0031 §5.3:每条配单测(N4)。
#[must_use]
pub fn next_status(cur: &SessionStatus, input: &FsmInput) -> SessionStatus {
    use FsmInput as I;
    use SessionStatus as S;
    match input {
        I::Send { now_ms } => S::AwaitingAck { sent_at: *now_ms },
        I::FirstPart { message_id } => S::Streaming {
            message_id: message_id.clone(),
        },
        I::Activity => match cur {
            // 等待/停滞 → 复活为流式(沿用已知流式 id);Idle/终态被活动唤醒为流式。
            S::AwaitingAck { .. } | S::Stalled | S::Idle | S::Stopped | S::Errored { .. } => {
                S::Streaming {
                    message_id: cur.resume_id(),
                }
            }
            // 流式 / 重试 / 阻塞:单纯活动不改态。
            other => other.clone(),
        },
        I::Busy => match cur {
            S::Idle | S::Stopped | S::Errored { .. } => S::AwaitingAck { sent_at: 0.0 },
            other => other.clone(),
        },
        I::Retry { attempt } => S::Retrying { attempt: *attempt },
        I::Idle => S::Idle,
        I::PermissionAsked => S::Blocked {
            on: Blocker::Permission,
            resume_message_id: cur.resume_id(),
        },
        I::QuestionAsked => S::Blocked {
            on: Blocker::Question,
            resume_message_id: cur.resume_id(),
        },
        I::Replied => match cur {
            S::Blocked {
                resume_message_id, ..
            } if !resume_message_id.is_empty() => S::Streaming {
                message_id: resume_message_id.clone(),
            },
            S::Blocked { .. } => S::Idle,
            other => other.clone(),
        },
        I::Stop => S::Stopped,
        I::Error { error } => S::Errored {
            error: error.clone(),
        },
        I::SoftTimeout => match cur {
            S::AwaitingAck { .. } | S::Streaming { .. } | S::Retrying { .. } => S::Stalled,
            other => other.clone(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_before_any_activity() {
        let t = TurnTracker::new();
        assert_eq!(t.status(), TurnStatus::Idle);
    }

    #[test]
    fn active_while_streaming() {
        let mut t = TurnTracker::new();
        t.tick(100.0);
        t.on_activity(100.0);
        assert_eq!(t.status(), TurnStatus::Active);
    }

    #[test]
    fn explicit_idle_settles() {
        let mut t = TurnTracker::new();
        t.on_activity(100.0);
        t.on_settle_signal();
        t.tick(200.0);
        assert_eq!(t.status(), TurnStatus::Settled);
    }

    #[test]
    fn forgotten_idle_hard_watchdog_settles() {
        // 模型忘了发 idle,但 30s 静默后看门狗强制收尾(不永久 loading)。
        let mut t = TurnTracker::new();
        t.on_activity(0.0);
        t.tick(9_000.0);
        assert_eq!(t.status(), TurnStatus::Stalled); // soft 超时
        t.tick(31_000.0);
        assert_eq!(t.status(), TurnStatus::Settled); // hard 超时
        assert!(t.is_settled());
    }

    #[test]
    fn activity_revives_after_stall() {
        let mut t = TurnTracker::new();
        t.on_activity(0.0);
        t.tick(31_000.0);
        assert_eq!(t.status(), TurnStatus::Settled);
        // 新 part 到达 → 复活。
        t.on_activity(31_000.0);
        t.tick(31_100.0);
        assert_eq!(t.status(), TurnStatus::Active);
    }

    #[test]
    fn busy_keeps_active_even_if_quiet() {
        // session busy 但内容暂停(模型在想)→ 仍 Active,不误判停滞。
        let mut t = TurnTracker::new();
        t.on_activity(0.0);
        t.on_busy();
        t.tick(20_000.0);
        assert_eq!(t.status(), TurnStatus::Active);
    }

    // ───────── Plan 22 N4:SessionStatus next_status 每条转移单测 ─────────
    use super::{next_status, Blocker, FsmInput as I, SessionStatus as S};

    #[test]
    fn n4_send_then_first_part_then_idle() {
        let s = next_status(&S::Idle, &I::Send { now_ms: 100.0 });
        assert_eq!(s, S::AwaitingAck { sent_at: 100.0 });
        let s = next_status(
            &s,
            &I::FirstPart {
                message_id: "m1".into(),
            },
        );
        assert_eq!(
            s,
            S::Streaming {
                message_id: "m1".into()
            }
        );
        assert!(s.is_active() && !s.can_send());
        assert_eq!(s.streaming_id(), Some("m1"));
        assert_eq!(next_status(&s, &I::Idle), S::Idle);
        assert!(S::Idle.can_send());
    }

    #[test]
    fn n4_retry_and_soft_timeout_and_hard_idle() {
        assert_eq!(
            next_status(&S::AwaitingAck { sent_at: 0.0 }, &I::Retry { attempt: 2 }),
            S::Retrying { attempt: 2 }
        );
        // soft 看门狗:流式 → Stalled;活动复活 → 流式。
        let st = next_status(
            &S::Streaming {
                message_id: "m".into(),
            },
            &I::SoftTimeout,
        );
        assert_eq!(st, S::Stalled);
        assert_eq!(
            next_status(&st, &I::Activity),
            S::Streaming {
                message_id: String::new()
            }
        );
    }

    #[test]
    fn n4_block_on_permission_then_reply_resumes() {
        let streaming = S::Streaming {
            message_id: "m9".into(),
        };
        let blocked = next_status(&streaming, &I::PermissionAsked);
        assert_eq!(
            blocked,
            S::Blocked {
                on: Blocker::Permission,
                resume_message_id: "m9".into()
            }
        );
        assert!(blocked.is_active());
        // reply → 回到被阻塞前的流式消息(F:回上一态)。
        assert_eq!(
            next_status(&blocked, &I::Replied),
            S::Streaming {
                message_id: "m9".into()
            }
        );
    }

    #[test]
    fn n4_question_block_without_stream_replies_to_idle() {
        let blocked = next_status(&S::Idle, &I::QuestionAsked);
        assert_eq!(
            blocked,
            S::Blocked {
                on: Blocker::Question,
                resume_message_id: String::new()
            }
        );
        assert_eq!(next_status(&blocked, &I::Replied), S::Idle);
    }

    #[test]
    fn n4_stop_and_error_are_terminal_but_revivable() {
        assert_eq!(
            next_status(
                &S::Streaming {
                    message_id: "m".into()
                },
                &I::Stop
            ),
            S::Stopped
        );
        let err = next_status(
            &S::Streaming {
                message_id: "m".into(),
            },
            &I::Error {
                error: "APIError".into(),
            },
        );
        assert_eq!(
            err,
            S::Errored {
                error: "APIError".into()
            }
        );
        assert!(err.can_send(), "终态可再发送");
        // 新活动复活成流式(回合复活)。
        assert_eq!(
            next_status(&S::Stopped, &I::Activity),
            S::Streaming {
                message_id: String::new()
            }
        );
    }

    #[test]
    fn n4_busy_from_idle_awaits_else_keeps() {
        assert_eq!(
            next_status(&S::Idle, &I::Busy),
            S::AwaitingAck { sent_at: 0.0 }
        );
        let streaming = S::Streaming {
            message_id: "m".into(),
        };
        assert_eq!(next_status(&streaming, &I::Busy), streaming);
    }
}
