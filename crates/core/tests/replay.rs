//! 确定性重放(Phase E DoD / T9):同一录像重放两次,最终 Store 与可见输出完全一致。
//!
//! 这是整个内核确定性的守门测试:只要 core 不偷碰墙钟/随机(R8/R9),给定相同录像
//! 与相同 dt 序列,结果必须逐字节可复现——断网也能复现 bug。

use opencode_chat_core::{CollectSink, Engine, MonospaceLayout, Player, Store};

/// 构造一段带乱序 + 对账的合成录像(模拟真实 SSE 噪声)。
fn recording() -> Vec<(f64, String)> {
    fn delta(t: f64, part: &str, d: &str) -> (f64, String) {
        (
            t,
            format!(
                r#"{{"type":"message.part.delta","properties":{{"sessionID":"s","messageID":"m","partID":"{part}","field":"text","delta":{d:?}}}}}"#
            ),
        )
    }
    fn updated(t: f64, part: &str, text: &str) -> (f64, String) {
        (
            t,
            format!(
                r#"{{"type":"message.part.updated","properties":{{"part":{{"type":"text","id":"{part}","messageID":"m","text":{text:?}}},"time":{t}}}}}"#
            ),
        )
    }
    vec![
        (0.0, r#"{"type":"server.connected","properties":{}}"#.into()),
        delta(10.0, "p1", "Hello "),
        delta(30.0, "p1", "世界"),
        (
            40.0,
            r#"{"type":"server.heartbeat","properties":{}}"#.into(),
        ),
        delta(60.0, "p1", " 🚀"),
        // 全量对账:补回一段可能在传输中丢失的字(AR4)。
        updated(80.0, "p1", "Hello 世界 🚀 done"),
        // 第二个 part(纵向堆叠)。
        delta(100.0, "p2", "second part"),
        // 未知事件类型:不得影响结果(AR12)。
        (
            110.0,
            r#"{"type":"some.future.event","properties":{"x":1}}"#.into(),
        ),
    ]
}

/// 跑一遍引擎到放完,返回 (store 快照, 末帧可见文本)。
fn run_once() -> (Vec<(String, String)>, String) {
    let player = Player::from_pairs(recording(), 16.0);
    let mut eng = Engine::new(
        player,
        MonospaceLayout::default(),
        CollectSink::default(),
        400.0,
        800.0,
    );
    // 足够多帧:覆盖所有记录 t(<=110ms,16ms 步进 ~7 帧到全部入场)+ 吐字时间。
    for _ in 0..200 {
        eng.frame(16.0);
    }
    let snapshot = eng.store().snapshot();
    let visible = eng.sink().visible_text();
    (snapshot, visible)
}

#[test]
fn replay_is_deterministic() {
    let (snap_a, vis_a) = run_once();
    let (snap_b, vis_b) = run_once();
    assert_eq!(snap_a, snap_b, "两次重放 Store 必须一致");
    assert_eq!(vis_a, vis_b, "两次重放可见输出必须一致");
}

#[test]
fn reconciliation_wins_over_delta() {
    // AR4:对账后 p1 文本以 updated 为准。
    let (snap, visible) = run_once();
    let store: std::collections::HashMap<_, _> = snap.into_iter().collect();
    assert_eq!(
        store.get("p1").map(String::as_str),
        Some("Hello 世界 🚀 done")
    );
    assert_eq!(store.get("p2").map(String::as_str), Some("second part"));
    // 可见文本最终应含两段全部字符。
    assert!(visible.contains("Hello 世界 🚀 done"), "got: {visible}");
    assert!(visible.contains("second part"), "got: {visible}");
}

#[test]
fn jsonl_roundtrip_preserves_determinism() {
    // 录像经 jsonl 序列化再解析,重放结果不变(Phase E 录像落盘场景)。
    use opencode_chat_core::Record;
    let records: Vec<Record> = recording()
        .into_iter()
        .map(|(t, raw)| Record { t, raw })
        .collect();
    let jsonl: String = records
        .iter()
        .map(|r| serde_json::to_string(r).expect("ser") + "\n")
        .collect();

    let run = |player: Player| {
        let mut eng = Engine::new(
            player,
            MonospaceLayout::default(),
            CollectSink::default(),
            400.0,
            800.0,
        );
        for _ in 0..200 {
            eng.frame(16.0);
        }
        eng.store().snapshot()
    };

    let from_mem = run(Player::new(records, 16.0));
    let from_disk = run(Player::from_jsonl(&jsonl, 16.0));
    assert_eq!(from_mem, from_disk);
    // Store 类型可直接相等(便于 Plan2 对账)。
    let _ = Store::new();
}
