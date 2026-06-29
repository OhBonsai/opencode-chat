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
    /// 删 part(message.part.removed)。
    PartRemoved {
        session_id: String,
        message_id: String,
        part_id: String,
    },
    /// 消息元信息更新:带 `info.{id, role, sessionID}`。**live 角色的唯一来源**(delta/part.updated
    /// 不带 role)→ store 据此 `messageID→role`,chat 左右分栏(Plan 13 §4.3)。字段缺省 = 空串。
    /// `error` = `info.error`(若有)→ 驱动 `Errored`(Plan 22 §5 / F4)。
    MessageUpdated {
        message_id: String,
        role: String,
        session_id: String,
        error: Option<String>,
    },
    /// 会话状态:`idle` / `busy` / `retry`(回合主信号)。
    SessionStatus { status: String },
    /// 子会话创建(parent==当前 → 登记;过滤/abort 范围,Plan 22 §2)。
    SessionCreated {
        session_id: String,
        parent_id: String,
    },
    /// 会话元信息更新(标题等)。
    SessionUpdated { session_id: String, title: String },
    /// 会话级错误(ghost-abort/配额/错误卡;F3/F4/F9)。`name`+`data`(原样)供错误分类。
    SessionError {
        session_id: String,
        name: String,
        data: serde_json::Value,
    },
    /// 上下文压缩:整列失效 → 重拉 getMessages(Plan 22 §2)。
    SessionCompacted { session_id: String },
    /// 实例销毁:流将关 → 等重连(Plan 22 §2)。
    InstanceDisposed,
    /// 权限请求 / 应答(Blocked + Dock,Plan 22 §2/§3.4)。`payload` 原样承载(shape 未冻结)。
    PermissionAsked {
        session_id: String,
        payload: serde_json::Value,
    },
    PermissionReplied {
        session_id: String,
        payload: serde_json::Value,
    },
    /// 提问请求 / 应答 / 拒绝(Blocked + Dock)。
    QuestionAsked {
        session_id: String,
        payload: serde_json::Value,
    },
    QuestionReplied {
        session_id: String,
        payload: serde_json::Value,
    },
    QuestionRejected {
        session_id: String,
        payload: serde_json::Value,
    },
    /// 首发握手(→ 触发 resync,0031 §5.4)。
    Connected,
    /// ~10s 心跳(僵尸看门狗喂时间戳)。
    Heartbeat,
    /// 未知事件类型(AR12,向前兼容)。
    Ignored,
}

/// Part 类型(Plan 22 §3:全分类联合,带结构化载荷)。未知类型 → `Other`(AR12)。
/// 结构不确定的子载荷(tool 的 `state`)以 `serde_json::Value` 原样承载 → 不因服务端加字段而碎。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type")]
pub enum Part {
    #[serde(rename = "text")]
    Text {
        id: String,
        #[serde(rename = "messageID")]
        message_id: String,
        text: String,
        /// TextPart 带 sessionID(snapshot/updated 来源);delta 路径无此字段。
        #[serde(rename = "sessionID", default)]
        session_id: String,
    },
    /// 推理 / 思考区(0006):正文同 text 走 delta(字符串字段)。
    #[serde(rename = "reasoning")]
    Reasoning {
        id: String,
        #[serde(rename = "messageID", default)]
        message_id: String,
        #[serde(default)]
        text: String,
        #[serde(rename = "sessionID", default)]
        session_id: String,
    },
    /// 工具调用(0007/0018):`tool` = 工具名;`state` = `{status,input,output,metadata,...}` 原样载荷。
    #[serde(rename = "tool")]
    Tool {
        id: String,
        #[serde(rename = "messageID", default)]
        message_id: String,
        #[serde(rename = "sessionID", default)]
        session_id: String,
        #[serde(default)]
        tool: String,
        /// 工具状态原样 JSON(status/input/output/metadata.filediff…),P22 兜底 dump,P23 漂亮卡。
        #[serde(default)]
        state: serde_json::Value,
    },
    /// 文件附件 / 嵌入(0007)。
    #[serde(rename = "file")]
    File {
        id: String,
        #[serde(rename = "messageID", default)]
        message_id: String,
        #[serde(rename = "sessionID", default)]
        session_id: String,
        #[serde(default)]
        filename: String,
        #[serde(default)]
        mime: String,
        #[serde(default)]
        url: String,
    },
    /// 上下文压缩通知(0026:分隔线 + 标签)。
    #[serde(rename = "compaction")]
    Compaction {
        #[serde(default)]
        id: String,
        #[serde(rename = "messageID", default)]
        message_id: String,
        #[serde(rename = "sessionID", default)]
        session_id: String,
    },
    /// 其余 part 类型(step-start/step-finish/snapshot/patch/agent/…)= 噪音,不承载不渲染(0002/AR12)。
    #[serde(other)]
    Other,
}

impl Part {
    /// part 的 id(`Other` 无 id → None)。
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        match self {
            Part::Text { id, .. }
            | Part::Reasoning { id, .. }
            | Part::Tool { id, .. }
            | Part::File { id, .. }
            | Part::Compaction { id, .. } => Some(id.as_str()),
            Part::Other => None,
        }
    }
}

/// 快照里的一条消息(`GET /session/{id}/message` 元素,Phase F catch-up)。
#[derive(Debug, Clone, PartialEq)]
pub struct SnapshotMessage {
    pub session_id: String,
    pub message_id: String,
    pub role: String,
    /// 该消息的 text part(Plan2 子集;其余 part 类型忽略)。
    pub text_parts: Vec<TextPartData>,
}

/// 快照里一个 text part 的最小数据。
#[derive(Debug, Clone, PartialEq)]
pub struct TextPartData {
    pub part_id: String,
    pub text: String,
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

/// `message.updated` 的 `properties.info`(只抽 chat 需要的 id/role/sessionID;其余忽略)。
#[derive(Debug, Deserialize)]
struct MessageUpdatedProps {
    info: MessageInfo,
}

#[derive(Debug, Deserialize)]
struct MessageInfo {
    #[serde(default)]
    id: String,
    #[serde(default)]
    role: String,
    #[serde(rename = "sessionID", default)]
    session_id: String,
    /// `info.error`(若有)→ 错误终止(F4)。原样保留作分类。
    #[serde(default)]
    error: serde_json::Value,
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
    #[error("快照解析失败: {0}")]
    Snapshot(serde_json::Error),
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
        "message.updated" => {
            // info 可能缺省/损坏:role 是辅助信息,缺了退默认(Assistant),不让整事件失败(AR12)。
            match serde_json::from_value::<MessageUpdatedProps>(env.properties) {
                Ok(p) => Event::MessageUpdated {
                    message_id: p.info.id,
                    role: p.info.role,
                    session_id: p.info.session_id,
                    error: error_string(&p.info.error),
                },
                Err(_) => Event::MessageUpdated {
                    message_id: String::new(),
                    role: String::new(),
                    session_id: String::new(),
                    error: None,
                },
            }
        }
        "session.status" => {
            // properties.status 可能是 {type:"busy"} 或直接字符串 "busy"。
            let status = env
                .properties
                .get("status")
                .and_then(|s| {
                    s.get("type")
                        .and_then(serde_json::Value::as_str)
                        .or_else(|| s.as_str())
                })
                .unwrap_or_default()
                .to_owned();
            Event::SessionStatus { status }
        }
        "session.idle" => Event::SessionStatus {
            status: "idle".to_owned(),
        },
        "message.part.removed" => Event::PartRemoved {
            session_id: str_prop(&env.properties, "sessionID"),
            message_id: str_prop(&env.properties, "messageID"),
            part_id: str_prop(&env.properties, "partID"),
        },
        "session.created" => Event::SessionCreated {
            session_id: info_str(&env.properties, "id")
                .unwrap_or_else(|| str_prop(&env.properties, "sessionID")),
            parent_id: info_str(&env.properties, "parentID").unwrap_or_default(),
        },
        "session.updated" => Event::SessionUpdated {
            session_id: info_str(&env.properties, "id")
                .unwrap_or_else(|| str_prop(&env.properties, "sessionID")),
            title: info_str(&env.properties, "title").unwrap_or_default(),
        },
        "session.error" => Event::SessionError {
            session_id: str_prop(&env.properties, "sessionID"),
            name: env
                .properties
                .get("error")
                .and_then(|e| e.get("name"))
                .and_then(serde_json::Value::as_str)
                .or_else(|| {
                    env.properties
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                })
                .unwrap_or_default()
                .to_owned(),
            data: env.properties,
        },
        "session.compacted" => Event::SessionCompacted {
            session_id: str_prop(&env.properties, "sessionID"),
        },
        "server.instance.disposed" => Event::InstanceDisposed,
        "permission.asked" | "permission.updated" => Event::PermissionAsked {
            session_id: str_prop(&env.properties, "sessionID"),
            payload: env.properties,
        },
        "permission.replied" => Event::PermissionReplied {
            session_id: str_prop(&env.properties, "sessionID"),
            payload: env.properties,
        },
        "question.asked" => Event::QuestionAsked {
            session_id: str_prop(&env.properties, "sessionID"),
            payload: env.properties,
        },
        "question.replied" => Event::QuestionReplied {
            session_id: str_prop(&env.properties, "sessionID"),
            payload: env.properties,
        },
        "question.rejected" => Event::QuestionRejected {
            session_id: str_prop(&env.properties, "sessionID"),
            payload: env.properties,
        },
        "server.connected" => Event::Connected,
        "server.heartbeat" => Event::Heartbeat,
        _ => Event::Ignored,
    };
    Ok(event)
}

/// 取 `properties.<key>` 字符串(缺/非串 → 空)。under-specified 事件的健壮取值(AR12)。
fn str_prop(props: &serde_json::Value, key: &str) -> String {
    props
        .get(key)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// 取 `properties.info.<key>` 字符串(session.created/updated 的元信息在 `info` 里)。
fn info_str(props: &serde_json::Value, key: &str) -> Option<String> {
    props
        .get("info")
        .and_then(|i| i.get(key))
        .and_then(serde_json::Value::as_str)
        .map(str::to_owned)
}

/// `info.error` 投影成可读串(对象取 `name`,字符串原样,空/null → None)。
fn error_string(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) if !s.is_empty() => Some(s.clone()),
        serde_json::Value::Object(_) => v
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
            .or_else(|| Some(v.to_string())),
        _ => None, // null / 空串 / 其它 → 无错误
    }
}

#[derive(Debug, Deserialize)]
struct SnapInfo {
    id: String,
    #[serde(rename = "sessionID", default)]
    session_id: String,
    #[serde(default)]
    role: String,
}

#[derive(Debug, Deserialize)]
struct SnapMsg {
    info: SnapInfo,
    #[serde(default)]
    parts: Vec<Part>,
}

/// 解析快照响应 `[{ info, parts }]`(`GET /session/{id}/message`,Phase F)。
///
/// 只抽 text part(Plan2 子集),message_id/session_id 取自 `info`。
pub fn parse_snapshot(raw: &str) -> Result<Vec<SnapshotMessage>, ProtocolError> {
    let msgs: Vec<SnapMsg> = serde_json::from_str(raw).map_err(ProtocolError::Snapshot)?;
    Ok(msgs
        .into_iter()
        .map(|m| {
            let text_parts = m
                .parts
                .into_iter()
                .filter_map(|p| match p {
                    Part::Text { id, text, .. } => Some(TextPartData { part_id: id, text }),
                    // 非 text part(reasoning/tool/file/compaction/Other):快照最小子集暂只抽 text;
                    // 完整承载走 live part.updated(P1 store carriage)。
                    _ => None,
                })
                .collect();
            SnapshotMessage {
                session_id: m.info.session_id,
                message_id: m.info.id,
                role: m.info.role,
                text_parts,
            }
        })
        .collect())
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
    fn message_updated_carries_role_for_live_split() {
        // Plan 13:live 左右分栏靠 message.updated 的 info.role(delta 不带)。
        let raw = r#"{"type":"message.updated","properties":{
            "info":{"id":"m7","sessionID":"sX","role":"user","time":{"created":1}}}}"#;
        assert_eq!(
            decode(raw).expect("decode"),
            Event::MessageUpdated {
                message_id: "m7".into(),
                role: "user".into(),
                session_id: "sX".into(),
                error: None,
            }
        );
    }

    #[test]
    fn message_updated_missing_info_degrades_not_errors() {
        // info 缺省/损坏 → 退空(AR12:不让整事件失败)。
        let raw = r#"{"type":"message.updated","properties":{}}"#;
        assert_eq!(
            decode(raw).expect("decode"),
            Event::MessageUpdated {
                message_id: String::new(),
                role: String::new(),
                session_id: String::new(),
                error: None,
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
        // step-start/snapshot/patch 等噪音 part → Other(不承载不渲染,0002/AR12)。
        let raw = r#"{"type":"message.part.updated","properties":{
            "part":{"type":"step-start","id":"s1"},"time":1.0}}"#;
        match decode(raw).expect("decode") {
            Event::PartUpdated { part, .. } => assert_eq!(part, Part::Other),
            other => panic!("期望 PartUpdated,得到 {other:?}"),
        }
    }

    #[test]
    fn decode_roundtrip_all_events() {
        // Plan 22 N1:§2 每类事件/Part 解码到正确变体;未知 → Ignored/Other(AR12)。
        type Check = fn(&Event) -> bool;
        let cases: &[(&str, Check)] = &[
            (
                r#"{"type":"message.part.removed","properties":{"sessionID":"s","messageID":"m","partID":"p"}}"#,
                |e| matches!(e, Event::PartRemoved { part_id, .. } if part_id == "p"),
            ),
            (
                r#"{"type":"session.created","properties":{"info":{"id":"sub","parentID":"s"}}}"#,
                |e| matches!(e, Event::SessionCreated { session_id, parent_id } if session_id=="sub" && parent_id=="s"),
            ),
            (
                r#"{"type":"session.updated","properties":{"info":{"id":"s","title":"Hi"}}}"#,
                |e| matches!(e, Event::SessionUpdated { title, .. } if title == "Hi"),
            ),
            (
                r#"{"type":"session.error","properties":{"sessionID":"s","error":{"name":"ProviderAuthError"}}}"#,
                |e| matches!(e, Event::SessionError { name, .. } if name == "ProviderAuthError"),
            ),
            (
                r#"{"type":"session.compacted","properties":{"sessionID":"s"}}"#,
                |e| matches!(e, Event::SessionCompacted { session_id } if session_id == "s"),
            ),
            (
                r#"{"type":"server.instance.disposed","properties":{}}"#,
                |e| matches!(e, Event::InstanceDisposed),
            ),
            (
                r#"{"type":"permission.asked","properties":{"sessionID":"s","permissionID":"x"}}"#,
                |e| matches!(e, Event::PermissionAsked { session_id, .. } if session_id == "s"),
            ),
            (
                r#"{"type":"question.asked","properties":{"sessionID":"s"}}"#,
                |e| matches!(e, Event::QuestionAsked { .. }),
            ),
            (
                r#"{"type":"message.updated","properties":{"info":{"id":"m","role":"assistant","error":{"name":"APIError"}}}}"#,
                |e| matches!(e, Event::MessageUpdated { error: Some(n), .. } if n == "APIError"),
            ),
            (r#"{"type":"future.unknown.event","properties":{}}"#, |e| {
                matches!(e, Event::Ignored)
            }),
        ];
        for (raw, ok) in cases {
            let ev = decode(raw).expect("decode");
            assert!(ok(&ev), "解码不符: {raw} → {ev:?}");
        }
    }

    #[test]
    fn decode_all_part_variants() {
        // N1(Part):reasoning/tool/file/compaction 解码到对应变体,载荷保留;噪音 → Other。
        let part = |json: &str| -> Part {
            let raw = format!(
                r#"{{"type":"message.part.updated","properties":{{"part":{json},"time":1.0}}}}"#
            );
            match decode(&raw).expect("decode") {
                Event::PartUpdated { part, .. } => part,
                other => panic!("期望 PartUpdated: {other:?}"),
            }
        };
        assert!(matches!(
            part(r#"{"type":"reasoning","id":"r","text":"想"}"#),
            Part::Reasoning { text, .. } if text == "想"
        ));
        assert!(matches!(
            part(r#"{"type":"tool","id":"t","tool":"bash","state":{"status":"running","input":{"cmd":"ls"}}}"#),
            Part::Tool { tool, .. } if tool == "bash"
        ));
        assert!(matches!(
            part(r#"{"type":"file","id":"f","filename":"a.png","mime":"image/png","url":"http://x"}"#),
            Part::File { filename, .. } if filename == "a.png"
        ));
        assert!(matches!(
            part(r#"{"type":"compaction","id":"c"}"#),
            Part::Compaction { .. }
        ));
        assert_eq!(part(r#"{"type":"snapshot","id":"x"}"#), Part::Other);
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

    #[test]
    fn parses_snapshot_text_parts() {
        let raw = r#"[
            {"info":{"id":"m1","sessionID":"sX","role":"user"},
             "parts":[{"type":"text","id":"p1","messageID":"m1","text":"问题"}]},
            {"info":{"id":"m2","sessionID":"sX","role":"assistant"},
             "parts":[{"type":"step-start"},
                      {"type":"text","id":"p2","messageID":"m2","text":"答复"}]}
        ]"#;
        let snap = parse_snapshot(raw).expect("snapshot");
        assert_eq!(snap.len(), 2);
        assert_eq!(snap[0].session_id, "sX");
        assert_eq!(snap[0].message_id, "m1");
        assert_eq!(
            snap[0].text_parts,
            vec![TextPartData {
                part_id: "p1".into(),
                text: "问题".into()
            }]
        );
        // 噪音 part(step-start)被滤掉,只留 text。
        assert_eq!(snap[1].text_parts.len(), 1);
        assert_eq!(snap[1].text_parts[0].text, "答复");
    }
}
