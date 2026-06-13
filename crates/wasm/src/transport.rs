//! transport(M1)— SSE 接入(gloo-net EventSource)。
//!
//! 订阅 `serverUrl + "/api/event"`,把每条 `data` **原文**入队(不在 JS/此处解析,BR1);
//! `Connection::poll` 取队列交 core 解码。Plan1 不做重连/看门狗(留 Plan2,0003)。

use std::cell::RefCell;
use std::collections::VecDeque;
use std::rc::Rc;

use futures::StreamExt;
use gloo_net::eventsource::futures::EventSource;
use opencode_chat_core::{Connection, RawEvent};
use wasm_bindgen_futures::spawn_local;

pub(crate) struct SseConnection {
    queue: Rc<RefCell<VecDeque<String>>>,
}

impl SseConnection {
    /// 连接并开始把事件抽进队列。`base` 是 server 根地址(如 `http://localhost:4096`);
    /// 内部拼出 SSE 端点。也兼容直接传入已带 `/event`、`/api/event` 路径的完整 URL。
    pub(crate) fn connect(base: &str) -> Result<Self, String> {
        let base = base.trim_end_matches('/');
        let event_url = if base.ends_with("/event") {
            base.to_string()
        } else {
            // 这版 opencode 路由无 `/api` 前缀(实测 session 端点为 `/session`)。
            format!("{base}/event")
        };
        tracing::info!(target: "M1", "连接 SSE: {event_url}");
        let mut es =
            EventSource::new(&event_url).map_err(|e| format!("EventSource 创建失败: {e:?}"))?;
        // opencode 默认以 "message" 事件投递信封 JSON。
        let stream = es
            .subscribe("message")
            .map_err(|e| format!("subscribe 失败: {e:?}"))?;
        let queue = Rc::new(RefCell::new(VecDeque::new()));
        let q = queue.clone();

        spawn_local(async move {
            // 把 es move 进来保活;stream 结束(连接关闭)即退出。
            let _es = es;
            let mut stream = stream;
            tracing::info!(target: "M1", "SSE 已连接,等待事件…");
            let mut first = true;
            while let Some(item) = stream.next().await {
                match item {
                    Ok((_ty, msg)) => {
                        if let Some(data) = msg.data().as_string() {
                            if first {
                                let head: String = data.chars().take(160).collect();
                                tracing::info!(target: "M1", "首个 SSE 事件: {head}");
                                first = false;
                            }
                            q.borrow_mut().push_back(data);
                        }
                    }
                    Err(e) => tracing::warn!(target: "M1", "SSE 事件错误: {e:?}"),
                }
            }
            tracing::info!(target: "M1", "SSE 流结束");
        });

        Ok(Self { queue })
    }
}

impl Connection for SseConnection {
    fn poll(&mut self) -> Vec<RawEvent> {
        self.queue
            .borrow_mut()
            .drain(..)
            .map(RawEvent::new)
            .collect()
    }
}
