//! record(M0)— 确定性录制 / 重放(testing §0)。
//!
//! - [`Recorder`] 包住任意 [`Connection`],把每次 poll 到的原始事件按墙钟时刻记成
//!   `(t, raw)`(时间走 [`Clock`] seam,R8)。
//! - [`Player`] 读回这些记录,实现 [`Connection`],按虚拟时间回放。重放只依赖记录内容
//!   与固定步进,不碰墙钟/随机,故**完全确定**(同录像 → 同最终 Store)。

use serde::{Deserialize, Serialize};

use crate::seam::{Clock, Connection, RawEvent};

/// 一条录制记录:到达时刻(ms)+ 原始事件文本。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub t: f64,
    pub raw: String,
}

/// 录制器:透明包住 `Connection`,旁路记录每个原始事件。
pub struct Recorder<C: Connection, K: Clock> {
    inner: C,
    clock: K,
    log: Vec<Record>,
}

impl<C: Connection, K: Clock> Recorder<C, K> {
    pub fn new(inner: C, clock: K) -> Self {
        Self {
            inner,
            clock,
            log: Vec::new(),
        }
    }

    /// 已录记录(只读)。
    pub fn records(&self) -> &[Record] {
        &self.log
    }

    /// 导出为 jsonl(每行一条 `Record`)。
    pub fn to_jsonl(&self) -> String {
        let mut out = String::new();
        for rec in &self.log {
            // serde_json 序列化单条记录不会失败(纯数据);失败则跳过该行而非 panic。
            if let Ok(line) = serde_json::to_string(rec) {
                out.push_str(&line);
                out.push('\n');
            }
        }
        out
    }
}

impl<C: Connection, K: Clock> Connection for Recorder<C, K> {
    fn poll(&mut self) -> Vec<RawEvent> {
        let events = self.inner.poll();
        let t = self.clock.now_ms();
        for ev in &events {
            self.log.push(Record {
                t,
                raw: ev.raw().to_owned(),
            });
        }
        events
    }
}

/// 重放器:按虚拟时间回放录像。也用作 Phase C 的合成事件源。
pub struct Player {
    /// 按 `t` 升序的记录。
    records: Vec<Record>,
    cursor: usize,
    virtual_now: f64,
    /// 每次 poll 推进的虚拟时间(ms)。决定回放节奏,确定性。
    step_ms: f64,
}

impl Player {
    /// 从已排序(或乱序,内部会稳定排序)的记录构造。
    pub fn new(mut records: Vec<Record>, step_ms: f64) -> Self {
        // 稳定排序保证同 t 记录顺序不变(确定性)。
        records.sort_by(|a, b| a.t.partial_cmp(&b.t).unwrap_or(std::cmp::Ordering::Equal));
        Self {
            records,
            cursor: 0,
            virtual_now: 0.0,
            step_ms,
        }
    }

    /// 便捷构造:`(t, raw)` 对。
    pub fn from_pairs(pairs: Vec<(f64, String)>, step_ms: f64) -> Self {
        let records = pairs
            .into_iter()
            .map(|(t, raw)| Record { t, raw })
            .collect();
        Self::new(records, step_ms)
    }

    /// 从 jsonl 解析(跳过空行;损坏行跳过而非 panic)。
    pub fn from_jsonl(text: &str, step_ms: f64) -> Self {
        let records = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str::<Record>(l).ok())
            .collect();
        Self::new(records, step_ms)
    }

    /// 是否已放完所有记录。
    pub fn is_exhausted(&self) -> bool {
        self.cursor >= self.records.len()
    }
}

impl Connection for Player {
    fn poll(&mut self) -> Vec<RawEvent> {
        self.virtual_now += self.step_ms;
        let mut out = Vec::new();
        while self.cursor < self.records.len() && self.records[self.cursor].t <= self.virtual_now {
            out.push(RawEvent::new(self.records[self.cursor].raw.clone()));
            self.cursor += 1;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedClock(f64);
    impl Clock for FixedClock {
        fn now_ms(&self) -> f64 {
            self.0
        }
    }

    #[test]
    fn player_releases_by_virtual_time() {
        let mut p = Player::from_pairs(
            vec![(0.0, "a".into()), (10.0, "b".into()), (100.0, "c".into())],
            16.0,
        );
        // 第 1 poll:virtual=16 → a,b 都到(t<=16),c 未到。
        let first: Vec<String> = p.poll().iter().map(|e| e.raw().to_owned()).collect();
        assert_eq!(first, vec!["a", "b"]);
        // 推进到 >=100。
        let mut got_c = false;
        for _ in 0..10 {
            if p.poll().iter().any(|e| e.raw() == "c") {
                got_c = true;
                break;
            }
        }
        assert!(got_c);
        assert!(p.is_exhausted());
    }

    #[test]
    fn recorder_roundtrips_through_jsonl() {
        let src = Player::from_pairs(vec![(0.0, "x".into()), (5.0, "y".into())], 1000.0);
        let mut rec = Recorder::new(src, FixedClock(42.0));
        // 放空源。
        while !rec_exhausted(&rec) {
            let got = rec.poll();
            if got.is_empty() {
                break;
            }
        }
        let jsonl = rec.to_jsonl();
        let replayed = Player::from_jsonl(&jsonl, 1000.0);
        // 重放出的原文应与录制内容一致。
        let raws: Vec<String> = replayed.records.iter().map(|r| r.raw.clone()).collect();
        assert_eq!(raws, vec!["x", "y"]);
    }

    fn rec_exhausted(rec: &Recorder<Player, FixedClock>) -> bool {
        rec.inner.is_exhausted()
    }
}
