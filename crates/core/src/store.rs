//! store(M3)— 世界状态唯一真相,Plan1 最小三表 + 对账(AR4)。
//!
//! - `delta` 乐观追加;`message.part.updated` 到来时**全量覆盖**该 part 文本并清累积,
//!   以服务端为准(AR4:丢字能自愈)。
//! - 一切按 part_id upsert,首见即记录顺序;幂等(R8/确定性:同序列 → 同状态)。

use std::collections::HashMap;

use crate::protocol::Part;

/// 单个 text part 的累积状态。
#[derive(Debug, Clone, PartialEq, Eq)]
struct PartRow {
    message_id: String,
    /// 当前文本 = delta 累积(对账后被全量覆盖)。
    text: String,
}

/// 归一化文档表(Plan1 仅 text part)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Store {
    /// part_id 首见顺序(渲染按此纵向堆叠)。
    order: Vec<String>,
    parts: HashMap<String, PartRow>,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    /// 文本增量追加(AR4 的乐观路径)。非 `text` field 忽略(Plan1)。
    pub fn apply_delta(&mut self, part_id: &str, field: &str, delta: &str) {
        if field != "text" {
            return;
        }
        let row = self.ensure(part_id, "");
        row.text.push_str(delta);
    }

    /// 全量对账(AR4):以 `part.updated` 为准,覆盖文本并清累积。
    pub fn apply_part_updated(&mut self, part: &Part) {
        if let Part::Text {
            id,
            message_id,
            text,
        } = part
        {
            let row = self.ensure(id, message_id);
            row.text.clone_from(text);
        }
        // 非 text part(Other)Plan1 忽略(AR12)。
    }

    /// 某 part 的当前文本。
    pub fn part_text(&self, part_id: &str) -> Option<&str> {
        self.parts.get(part_id).map(|r| r.text.as_str())
    }

    /// 按首见顺序遍历 (part_id, text)。
    pub fn parts_in_order(&self) -> impl Iterator<Item = (&str, &str)> {
        self.order
            .iter()
            .filter_map(move |id| self.parts.get(id).map(|r| (id.as_str(), r.text.as_str())))
    }

    /// 用于断言/对账的快照(part_id, text),按顺序。
    pub fn snapshot(&self) -> Vec<(String, String)> {
        self.parts_in_order()
            .map(|(id, text)| (id.to_owned(), text.to_owned()))
            .collect()
    }

    fn ensure(&mut self, part_id: &str, message_id: &str) -> &mut PartRow {
        if !self.parts.contains_key(part_id) {
            self.order.push(part_id.to_owned());
            self.parts.insert(
                part_id.to_owned(),
                PartRow {
                    message_id: message_id.to_owned(),
                    text: String::new(),
                },
            );
        }
        self.parts.get_mut(part_id).expect("just inserted above") // reason: 上面已确保存在;非生产 panic 路径
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delta_appends_in_order() {
        let mut s = Store::new();
        s.apply_delta("p1", "text", "Hel");
        s.apply_delta("p1", "text", "lo");
        assert_eq!(s.part_text("p1"), Some("Hello"));
    }

    #[test]
    fn non_text_field_ignored() {
        let mut s = Store::new();
        s.apply_delta("p1", "reasoning", "x");
        assert_eq!(s.part_text("p1"), None);
    }

    #[test]
    fn part_updated_overwrites_lost_delta() {
        // AR4:delta 丢了一截,updated 全量对账后自愈。
        let mut s = Store::new();
        s.apply_delta("p1", "text", "Hel"); // 假设 "lo" 的 delta 丢了
        s.apply_part_updated(&Part::Text {
            id: "p1".into(),
            message_id: "m1".into(),
            text: "Hello world".into(),
        });
        assert_eq!(s.part_text("p1"), Some("Hello world"));
    }

    #[test]
    fn order_is_first_seen() {
        let mut s = Store::new();
        s.apply_delta("b", "text", "B");
        s.apply_delta("a", "text", "A");
        s.apply_delta("b", "text", "B2");
        let ids: Vec<_> = s.parts_in_order().map(|(id, _)| id.to_owned()).collect();
        assert_eq!(ids, vec!["b", "a"]);
    }

    use proptest::prelude::*;

    proptest! {
        // T7/AR4 不变量:无论中途追加了多少任意 delta,一次全量对账后,该 part 文本
        // 必等于对账值——丢字/串字都能自愈。
        #[test]
        fn reconciliation_is_authoritative(
            deltas in prop::collection::vec("[\\PC]{0,8}", 0..12),
            authoritative in "[\\PC]{0,32}",
        ) {
            let mut s = Store::new();
            for d in &deltas {
                s.apply_delta("p", "text", d);
            }
            s.apply_part_updated(&Part::Text {
                id: "p".into(),
                message_id: "m".into(),
                text: authoritative.clone(),
            });
            prop_assert_eq!(s.part_text("p"), Some(authoritative.as_str()));
        }
    }
}
