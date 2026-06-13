//! store(M3)— 世界状态唯一真相,归一化三表 + 对账(AR4)+ 快照灌入 + session 归属。
//!
//! - `delta` 乐观追加;`message.part.updated` 全量覆盖该 part(AR4:丢字自愈)。
//! - `apply_snapshot` 批量灌历史(catch-up,Phase F):带 sessionID,建立 part/message→session 映射。
//! - **session 归属**:delta 实测不带 sessionID,靠 `partID→messageID→sessionID` 解析
//!   (snapshot/updated 建映射),供 `?session=` 过滤。
//! - 一切按 part_id upsert,首见即记录顺序;幂等(R8/确定性:同序列 → 同状态)。

use std::collections::HashMap;

use crate::protocol::{Part, SnapshotMessage};

/// 单个 text part 的累积状态。
#[derive(Debug, Clone, PartialEq, Eq)]
struct PartRow {
    message_id: String,
    /// 已知的归属 session(snapshot/updated 带;delta 不带 → 靠 message 映射补)。
    session_id: Option<String>,
    /// 当前文本 = delta 累积(对账后被全量覆盖)。
    text: String,
}

/// 归一化文档表(Plan2 text 子集)。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Store {
    /// part_id 首见顺序(渲染按此纵向堆叠)。
    order: Vec<String>,
    parts: HashMap<String, PartRow>,
    /// messageID → sessionID(snapshot/updated 建立),用于解析 delta 的归属。
    message_session: HashMap<String, String>,
}

impl Store {
    pub fn new() -> Self {
        Self::default()
    }

    /// 文本增量追加(AR4 乐观路径)。非 `text` field 忽略。`message_id` 用于 session 解析。
    pub fn apply_delta(&mut self, part_id: &str, message_id: &str, field: &str, delta: &str) {
        if field != "text" {
            return;
        }
        let known = self.message_session.get(message_id).cloned();
        let row = self.ensure(part_id, message_id);
        row.text.push_str(delta);
        if row.session_id.is_none() {
            row.session_id = known;
        }
    }

    /// 全量对账(AR4):以 `part.updated` 为准,覆盖文本;若带 sessionID 则建立映射。
    pub fn apply_part_updated(&mut self, part: &Part) {
        if let Part::Text {
            id,
            message_id,
            text,
            session_id,
        } = part
        {
            if !session_id.is_empty() {
                self.message_session
                    .insert(message_id.clone(), session_id.clone());
            }
            let sid = (!session_id.is_empty()).then(|| session_id.clone());
            let row = self.ensure(id, message_id);
            row.text.clone_from(text);
            if sid.is_some() {
                row.session_id = sid;
            }
        }
        // 非 text part(Other)忽略(AR12)。
    }

    /// 批量灌入快照历史(Phase F,catch-up)。带 sessionID,建立映射。
    pub fn apply_snapshot(&mut self, messages: &[SnapshotMessage]) {
        for msg in messages {
            self.message_session
                .insert(msg.message_id.clone(), msg.session_id.clone());
            for tp in &msg.text_parts {
                let row = self.ensure(&tp.part_id, &msg.message_id);
                row.text.clone_from(&tp.text);
                row.session_id = Some(msg.session_id.clone());
            }
        }
    }

    /// 某 part 的当前文本。
    pub fn part_text(&self, part_id: &str) -> Option<&str> {
        self.parts.get(part_id).map(|r| r.text.as_str())
    }

    /// 某 part 的归属 session(直接已知,或经 message 映射解析)。
    pub fn part_session(&self, part_id: &str) -> Option<&str> {
        let row = self.parts.get(part_id)?;
        match &row.session_id {
            Some(s) => Some(s.as_str()),
            None => self
                .message_session
                .get(&row.message_id)
                .map(String::as_str),
        }
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
                    session_id: None,
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
    use crate::protocol::TextPartData;

    fn text_part(id: &str, mid: &str, sid: &str, text: &str) -> Part {
        Part::Text {
            id: id.into(),
            message_id: mid.into(),
            text: text.into(),
            session_id: sid.into(),
        }
    }

    #[test]
    fn delta_appends_in_order() {
        let mut s = Store::new();
        s.apply_delta("p1", "m1", "text", "Hel");
        s.apply_delta("p1", "m1", "text", "lo");
        assert_eq!(s.part_text("p1"), Some("Hello"));
    }

    #[test]
    fn non_text_field_ignored() {
        let mut s = Store::new();
        s.apply_delta("p1", "m1", "reasoning", "x");
        assert_eq!(s.part_text("p1"), None);
    }

    #[test]
    fn part_updated_overwrites_lost_delta() {
        // AR4:delta 丢了一截,updated 全量对账后自愈。
        let mut s = Store::new();
        s.apply_delta("p1", "m1", "text", "Hel");
        s.apply_part_updated(&text_part("p1", "m1", "s1", "Hello world"));
        assert_eq!(s.part_text("p1"), Some("Hello world"));
        assert_eq!(s.part_session("p1"), Some("s1"));
    }

    #[test]
    fn order_is_first_seen() {
        let mut s = Store::new();
        s.apply_delta("b", "m", "text", "B");
        s.apply_delta("a", "m", "text", "A");
        s.apply_delta("b", "m", "text", "B2");
        let ids: Vec<_> = s.parts_in_order().map(|(id, _)| id.to_owned()).collect();
        assert_eq!(ids, vec!["b", "a"]);
    }

    #[test]
    fn snapshot_loads_history_with_session() {
        let mut s = Store::new();
        s.apply_snapshot(&[SnapshotMessage {
            session_id: "sX".into(),
            message_id: "m1".into(),
            role: "assistant".into(),
            text_parts: vec![TextPartData {
                part_id: "p1".into(),
                text: "历史回复".into(),
            }],
        }]);
        assert_eq!(s.part_text("p1"), Some("历史回复"));
        assert_eq!(s.part_session("p1"), Some("sX"));
    }

    #[test]
    fn delta_session_resolved_via_message_map() {
        // delta 不带 sessionID,但同 message 已被 snapshot/updated 建立映射 → 可解析。
        let mut s = Store::new();
        s.apply_snapshot(&[SnapshotMessage {
            session_id: "sX".into(),
            message_id: "m1".into(),
            role: "assistant".into(),
            text_parts: vec![],
        }]);
        s.apply_delta("p9", "m1", "text", "live");
        assert_eq!(s.part_session("p9"), Some("sX"));
    }

    use proptest::prelude::*;

    proptest! {
        // T7/AR4 不变量:无论中途追加多少任意 delta,一次全量对账后文本必等于对账值。
        #[test]
        fn reconciliation_is_authoritative(
            deltas in prop::collection::vec("[\\PC]{0,8}", 0..12),
            authoritative in "[\\PC]{0,32}",
        ) {
            let mut s = Store::new();
            for d in &deltas {
                s.apply_delta("p", "m", "text", d);
            }
            s.apply_part_updated(&text_part("p", "m", "s", &authoritative));
            prop_assert_eq!(s.part_text("p"), Some(authoritative.as_str()));
        }
    }
}
