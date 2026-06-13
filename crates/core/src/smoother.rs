//! smoother(M5)— 把突发 delta 整流成匀速逐字上屏。
//!
//! - **AR7**:吐字单位 = grapheme cluster,绝不按码点切(emoji/组合字不碎)。调用方
//!   传入已切好的 grapheme 序列。
//! - 匀速积分:`budget += rate * dt`;`rate = base * (1 + k*backlog)`(backlog 越高
//!   越追赶,0002 §4),封顶防瞬间倾泻。
//! - 确定性(R8/R9):全由注入的 `dt_ms` 驱动,无墙钟、无随机;同 dt 序列 → 同输出。

use std::collections::{HashMap, VecDeque};

/// 本帧吐出的一个字形。
#[derive(Clone, Debug, PartialEq)]
pub struct Revealed {
    pub part_id: String,
    pub cluster: String,
    /// 上屏时刻(ms)= 调用 update 时传入的 `now_ms`。
    pub spawn_time_ms: f32,
}

/// 逐 grapheme 整流器。
pub struct Smoother {
    /// 每个 part 一条 reveal 队列。
    queues: HashMap<String, VecDeque<String>>,
    /// part 入队顺序(跨 part 时先放先到的)。
    order: VecDeque<String>,
    /// 基线速率(grapheme/秒)。
    base_cps: f64,
    /// backlog 追赶系数。
    catchup_k: f64,
    /// 追赶倍率上限。
    max_mult: f64,
    /// 不足 1 个字形的预算余量。
    budget: f64,
}

impl Smoother {
    /// `base_cps` 基线吐字速率(plan1 基线 ~200 字/秒)。
    pub fn new(base_cps: f64) -> Self {
        Self {
            queues: HashMap::new(),
            order: VecDeque::new(),
            base_cps,
            catchup_k: 0.02,
            max_mult: 8.0,
            budget: 0.0,
        }
    }

    /// 入队新到的 grapheme(append-only)。
    pub fn push(&mut self, part_id: &str, graphemes: &[&str]) {
        if graphemes.is_empty() {
            return;
        }
        if !self.queues.contains_key(part_id) {
            self.order.push_back(part_id.to_owned());
            self.queues.insert(part_id.to_owned(), VecDeque::new());
        }
        let q = self.queues.get_mut(part_id).expect("just inserted"); // reason: 上面已确保存在
        q.extend(graphemes.iter().map(|g| (*g).to_owned()));
    }

    /// 待吐字形总数(backlog)。
    pub fn backlog(&self) -> usize {
        self.queues.values().map(VecDeque::len).sum()
    }

    /// 推进 `dt_ms`,返回本帧应上屏的字形(各打 `now_ms` 为 spawn_time)。
    pub fn update(&mut self, dt_ms: f64, now_ms: f64) -> Vec<Revealed> {
        let backlog = self.backlog();
        if backlog == 0 {
            self.budget = 0.0;
            return Vec::new();
        }
        let mult = (1.0 + self.catchup_k * backlog as f64).min(self.max_mult);
        let rate = self.base_cps * mult;
        self.budget += rate * (dt_ms / 1000.0);

        let mut out = Vec::new();
        while self.budget >= 1.0 {
            let Some((part_id, cluster)) = self.pop_next() else {
                // 队列空,余量不跨帧累积(避免空转后突然倾泻)。
                self.budget = 0.0;
                break;
            };
            self.budget -= 1.0;
            out.push(Revealed {
                part_id,
                cluster,
                spawn_time_ms: now_ms as f32,
            });
        }
        out
    }

    /// 按 part 入队顺序取下一个字形;空队列出队。
    fn pop_next(&mut self) -> Option<(String, String)> {
        while let Some(part_id) = self.order.front().cloned() {
            if let Some(q) = self.queues.get_mut(&part_id) {
                if let Some(cluster) = q.pop_front() {
                    return Some((part_id, cluster));
                }
            }
            // 该 part 暂时排空,移出轮转(后续 push 会重新入 order)。
            self.order.pop_front();
            self.queues.remove(&part_id);
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(s: &str) -> Vec<&str> {
        // 测试里简单按 char 切;真实 grapheme 切分见 app/support。
        s.split("").filter(|x| !x.is_empty()).collect()
    }

    #[test]
    fn reveals_at_constant_rate() {
        let mut sm = Smoother::new(100.0); // 100 字/秒 → 100ms 应吐 ~10 字
        sm.push("p1", &g("abcdefghijklmnop"));
        let out = sm.update(100.0, 0.0);
        // 100ms * 100cps = 10 基础预算,backlog 小幅追赶 → >=10
        assert!(out.len() >= 10, "got {}", out.len());
        assert!(out.iter().all(|r| r.part_id == "p1"));
    }

    #[test]
    fn deterministic_for_same_dt_sequence() {
        let run = || {
            let mut sm = Smoother::new(200.0);
            sm.push("p", &g("你好世界🚀这是一段流式文本"));
            let mut all = Vec::new();
            let mut t = 0.0;
            for _ in 0..20 {
                t += 16.0;
                all.extend(sm.update(16.0, t));
            }
            all
        };
        assert_eq!(run(), run());
    }

    #[test]
    fn drains_everything_eventually() {
        let mut sm = Smoother::new(50.0);
        let text = g("hello");
        sm.push("p", &text);
        let mut total = 0;
        let mut t = 0.0;
        for _ in 0..100 {
            t += 16.0;
            total += sm.update(16.0, t).len();
        }
        assert_eq!(total, 5);
        assert_eq!(sm.backlog(), 0);
    }

    #[test]
    fn empty_push_is_noop() {
        let mut sm = Smoother::new(100.0);
        sm.push("p", &[]);
        assert_eq!(sm.backlog(), 0);
    }
}
