//! protocol(M2)— opencode 事件解码(Plan1 仅 text 子集)。
//!
//! 字符串只过界一次,在此完成解码。未知事件/Part 类型 → `Ignored`/`Other`,**不 panic**
//! (AR12,向前兼容:服务端加类型不致崩)。
//!
//! 协议出处:`packages/core/src/v1/session.ts`、opencode server event handler(见
//! plan1-build-guide §6)。

use serde::Deserialize;

/// SSE 信封:`{id, type, properties}`。先解信封,再按 `type` 派发 `properties`。
#[derive(Debug, Clone, Deserialize)]
pub struct Envelope {
    #[serde(default)]
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub properties: serde_json::Value,
}

/// 解码后的强类型事件(Plan1 子集)。
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// 热路径文本增量(append-only)。
    PartDelta {
        session_id: String,
        message_id: String,
        part_id: String,
        field: String,
        delta: String,
    },
    /// 全量对账:以 `part` 为准(AR4)。
    PartUpdated { part: Part, time: f64 },
    /// 最小占位(Plan1 不读细节)。
    MessageUpdated,
    /// 会话状态(idle/busy/retry),Plan1 仅识别不处理。
    SessionStatus,
    /// 首发握手。
    Connected,
    /// 10s 心跳。
    Heartbeat,
    /// 未知事件类型(AR12)。
    Ignored,
}

/// Part 类型(Plan1 只认 `text`,其余 → `Other`)。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type")]
pub enum Part {
    #[serde(rename = "text")]
    Text {
        id: String,
        #[serde(rename = "messageID")]
        message_id: String,
        text: String,
    },
    /// Plan1 其余 part 类型一律忽略(AR12)。
    #[serde(other)]
    Other,
}

#[derive(Debug, Deserialize)]
struct PartDeltaProps {
    // 实测 opencode 的 message.part.delta 不带 sessionID(仅 messageID/partID/field/delta);
    // 设默认以兼容带/不带两种 build(Plan1 也不按 session 过滤)。
    #[serde(rename = "sessionID", default)]
    session_id: String,
    #[serde(rename = "messageID")]
    message_id: String,
    #[serde(rename = "partID")]
    part_id: String,
    field: String,
    delta: String,
}

#[derive(Debug, Deserialize)]
struct PartUpdatedProps {
    part: Part,
    #[serde(default)]
    time: f64,
}

/// 解码失败(信封损坏 / 已知类型的 properties 损坏)。未知类型不算失败(→ `Ignored`)。
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("envelope 解析失败: {0}")]
    Envelope(serde_json::Error),
    #[error("事件 {kind} 的 properties 解析失败: {source}")]
    Properties {
        kind: String,
        source: serde_json::Error,
    },
}

/// 把一条 SSE `data` 原文解码成 [`Event`]。
///
/// - 信封损坏 → `Err`(由调用方 `warn!` + 跳过)。
/// - 未知 `type` → `Ok(Event::Ignored)`(AR12)。
pub fn decode(raw: &str) -> Result<Event, ProtocolError> {
    let env: Envelope = serde_json::from_str(raw).map_err(ProtocolError::Envelope)?;
    let props = |e: serde_json::Error| ProtocolError::Properties {
        kind: env.kind.clone(),
        source: e,
    };
    let event = match env.kind.as_str() {
        "message.part.delta" => {
            let p: PartDeltaProps = serde_json::from_value(env.properties).map_err(props)?;
            Event::PartDelta {
                session_id: p.session_id,
                message_id: p.message_id,
                part_id: p.part_id,
                field: p.field,
                delta: p.delta,
            }
        }
        "message.part.updated" => {
            let p: PartUpdatedProps = serde_json::from_value(env.properties).map_err(props)?;
            Event::PartUpdated {
                part: p.part,
                time: p.time,
            }
        }
        "message.updated" => Event::MessageUpdated,
        "session.status" => Event::SessionStatus,
        "server.connected" => Event::Connected,
        "server.heartbeat" => Event::Heartbeat,
        _ => Event::Ignored,
    };
    Ok(event)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::panic)] // reason: 测试用 panic! 表达失败分支,符合 R1(测试可放开)
    use super::*;

    #[test]
    fn decodes_text_delta() {
        let raw = r#"{"id":"e1","type":"message.part.delta","properties":{
            "sessionID":"s1","messageID":"m1","partID":"p1","field":"text","delta":"你好"}}"#;
        let ev = decode(raw).expect("decode");
        assert_eq!(
            ev,
            Event::PartDelta {
                session_id: "s1".into(),
                message_id: "m1".into(),
                part_id: "p1".into(),
                field: "text".into(),
                delta: "你好".into(),
            }
        );
    }

    #[test]
    fn unknown_type_is_ignored_not_error() {
        // AR12:服务端加新类型不该让我们崩或报错。
        let raw = r#"{"id":"e2","type":"some.future.event","properties":{"x":1}}"#;
        assert_eq!(decode(raw).expect("decode"), Event::Ignored);
    }

    #[test]
    fn unknown_part_type_becomes_other() {
        let raw = r#"{"type":"message.part.updated","properties":{
            "part":{"type":"tool","id":"t1"},"time":1.0}}"#;
        match decode(raw).expect("decode") {
            Event::PartUpdated { part, .. } => assert_eq!(part, Part::Other),
            other => panic!("期望 PartUpdated,得到 {other:?}"),
        }
    }

    #[test]
    fn heartbeat_and_connected() {
        assert_eq!(
            decode(r#"{"type":"server.heartbeat","properties":{}}"#).expect("d"),
            Event::Heartbeat
        );
        assert_eq!(
            decode(r#"{"type":"server.connected","properties":{}}"#).expect("d"),
            Event::Connected
        );
    }

    #[test]
    fn broken_envelope_is_error() {
        assert!(decode("not json").is_err());
    }
}
