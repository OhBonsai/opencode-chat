//! resilience(M0/M4)— Plan 22 P5 容错纯逻辑(F6/F8/F9/F10 的可测内核)。
//!
//! 这些是 §5 错误处理表里**与平台无关、可 native 单测**的判据/合并函数(CR1)。app/host 在
//! `ingest_events` 与 resync 处调用它们做决策;timer/I/O(F1/F2 wall-clock)留 TS(0031 §3.1)。

use std::collections::HashMap;

/// F9:错误名是否"配额 / 额度 / 限流"类 → 配额 Dock / 发送前预检(而非普通错误卡)。
#[must_use]
pub fn is_quota_error(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    [
        "quota",
        "rate",
        "limit",
        "insufficient",
        "balance",
        "429",
        "exceeded",
    ]
    .iter()
    .any(|k| n.contains(k))
}

/// F8:回合收尾时是否该补"兜底错误卡"——已发送(awaiting)但末条 assistant **无任何回复、无 finish、
/// 无 error**(如 503 耗尽却没 `message.updated`)。三者皆无 + 处于等待 → 注卡,别静默卡住。
#[must_use]
pub fn should_bottom_out(awaiting: bool, had_response: bool, had_error: bool) -> bool {
    awaiting && !had_response && !had_error
}

/// F6:**非破坏**合并——把 `incoming`(服务端快照)并入 `existing`(现有),按 `time`(created)升序、
/// id 去重、**双方都保留**(live 的本地 part 不因快照缺失而丢)。`incoming` 的 time 权威(覆盖)。
/// 返回合并后的有序 id 列表。元素 = `(id, time_created)`。
#[must_use]
pub fn merge_ordered(existing: &[(String, f64)], incoming: &[(String, f64)]) -> Vec<String> {
    let mut time: HashMap<String, f64> = HashMap::new();
    let mut first_seen: HashMap<String, usize> = HashMap::new();
    for (i, (id, t)) in existing.iter().chain(incoming).enumerate() {
        time.insert(id.clone(), *t); // incoming 在后 → 覆盖 → 权威
        first_seen.entry(id.clone()).or_insert(i);
    }
    let mut ids: Vec<String> = time.keys().cloned().collect();
    // 按 (time 升序, 首见序) → 稳定且时间有序;NaN/相等回退首见序。
    ids.sort_by(|a, b| {
        time[a]
            .partial_cmp(&time[b])
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| first_seen[a].cmp(&first_seen[b]))
    });
    ids
}

/// F10:真用户消息到达时,是否应移除占位 `temp-` 消息(文本匹配才回显移除;否则按 assistant 处理)。
#[must_use]
pub fn temp_should_replace(temp_text: &str, real_text: &str) -> bool {
    !temp_text.is_empty() && temp_text == real_text
}

#[cfg(test)]
mod tests {
    use super::{is_quota_error, merge_ordered, should_bottom_out, temp_should_replace};

    #[test]
    fn f9_quota_classification() {
        assert!(is_quota_error("RateLimitError"));
        assert!(is_quota_error("InsufficientQuota"));
        assert!(is_quota_error("HTTP 429"));
        assert!(is_quota_error("balance exceeded"));
        assert!(!is_quota_error("ProviderAuthError"));
        assert!(!is_quota_error("APIError"));
    }

    #[test]
    fn f8_bottom_out_only_when_silent() {
        assert!(
            should_bottom_out(true, false, false),
            "等待中无回复无错 → 注卡"
        );
        assert!(!should_bottom_out(true, true, false), "有回复 → 不注");
        assert!(
            !should_bottom_out(true, false, true),
            "有真错 → 不注(走真错卡)"
        );
        assert!(!should_bottom_out(false, false, false), "没发送 → 不注");
    }

    #[test]
    fn f6_merge_non_destructive_time_ordered() {
        // existing 有本地 live part p3(快照缺它);incoming 权威时间 → 合并后按时间有序、p3 保留。
        let existing = vec![("p1".to_owned(), 10.0), ("p3".to_owned(), 30.0)];
        let incoming = vec![("p1".to_owned(), 10.0), ("p2".to_owned(), 20.0)];
        let merged = merge_ordered(&existing, &incoming);
        assert_eq!(
            merged,
            vec!["p1", "p2", "p3"],
            "时间有序 + 双方保留(p3 不丢)"
        );
    }

    #[test]
    fn f6_merge_idempotent() {
        let a = vec![("p1".to_owned(), 1.0), ("p2".to_owned(), 2.0)];
        assert_eq!(merge_ordered(&a, &a), merge_ordered(&a, &a));
        assert_eq!(merge_ordered(&a, &a), vec!["p1", "p2"]);
    }

    #[test]
    fn f10_temp_replace_on_text_match() {
        assert!(
            temp_should_replace("hello", "hello"),
            "文本匹配 → 移除 temp 回显"
        );
        assert!(!temp_should_replace("hello", "world"), "不匹配 → 不移除");
        assert!(!temp_should_replace("", "world"), "空 temp → 不移除");
    }
}
