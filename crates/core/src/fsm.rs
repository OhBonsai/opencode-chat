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
}
