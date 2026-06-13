# opencode 接口知识(本项目消费视角)

- 定位:**我们这个 wasm 对话组件消费 opencode HTTP/SSE 接口所需的全部知识的唯一真相**。
  动 M1 transport / M2 protocol / 联调脚本前**先读本文**,不要每次去翻 opencode 源码重推。
- 真相来源(本文据此提炼,版本漂移时回这里核对):
  - OpenAPI:`~/w/agentscode/opencode/packages/sdk/openapi.json`(权威 schema)
  - 事件定义:`packages/core/src/session/event.ts`、`packages/core/src/v1/session.ts`(Part/Message)
  - SSE handler:`packages/opencode/src/server/routes/instance/httpapi/handlers/event.ts`
- 刷新方法见 §7。最后核对:2026-06-13(对 `~/w/agentscode/opencode` 当前 build)。

---

## 1. 一张图:我们用到的最小面

```
建会话         POST /session                         → { id: "ses_..." }
发消息(同步)  POST /session/{id}/message            → { info, parts }  (阻塞到回完)
事件流(SSE)   GET  /event                           → text/event-stream
快照(catch-up) GET  /session/{id}/message?limit&before → [{ info, parts }]   (Plan2)
中止           POST /session/{id}/abort
```

热路径只有两个事件:`message.part.delta`(增量)+ `message.part.updated`(全量对账)。

---

## 2. ★ 路径前缀:有两套,按 build 实测 ★

同一服务器**同时**暴露**带 `/api` 前缀**和**不带前缀**两套路由(openapi 里都列了:
`/event` 与 `/api/event`、`/session` 与 `/api/session` 并存)。

- **本机当前 build 实测走无前缀**:`/event`、`/session`(decision 0001 记的 `/api/event`
  来自更早 build)。
- **结论**:transport 默认连 `/event`,但**必须可配**;URL 已带路径时原样用。联调前用
  `node scripts/api-paths.mjs` dump 一次确认。**不要把前缀写死。**

---

## 3. SSE:`GET /event`

- `Content-Type: text/event-stream`;每条 `event: message`,`data:` 为统一信封:
  ```json
  { "id": "evt_...", "type": "message.part.delta", "properties": { ... } }
  ```
- 连接首发 `server.connected`;之后每 ~10s 一条 `server.heartbeat`(活性检测,用它区分
  "模型停了" vs "连接死了",见 0005)。
- **全局流**:`/event` 推服务端**所有 session** 的事件,信封/properties 里带 `sessionID`
  用于过滤(但见 §6 实测:delta 上可能缺 sessionID)。`/global/event` 是更上层的全局总线。
- 服务端响应头:`Cache-Control: no-cache, no-transform`、`X-Accel-Buffering: no`(防代理缓冲)。

### 3.1 客户端要认的事件类型(v1 面,我们消费这套)

| type | properties | 用途 |
|---|---|---|
| `message.part.delta` | `{ sessionID, messageID, partID, field, delta }` | **热路径**,文本增量,append-only |
| `message.part.updated` | `{ sessionID, part, time }` | **全量对账**(part 完整) |
| `message.part.removed` | `{ sessionID, messageID, partID }` | 删 part |
| `message.updated` | `{ sessionID, info }` | 消息壳(role/model/cost/error…) |
| `message.removed` | `{ sessionID, messageID }` | 删消息 |
| `session.status` | `{ sessionID, status:{ type: idle\|busy\|retry, ... } }` | 忙/闲,驱动"思考中"+收尾 |
| `session.idle` | `{ sessionID }` | (旧)收尾信号,与 status 并存,别只依赖它(0005) |
| `session.created/updated/deleted` | `{ sessionID, info }` | 会话元数据(标题/revert…) |
| `session.error` | `{ sessionID, ... }` | 错误终止 |
| `session.diff` | `{ sessionID, ... }` | 文件 diff(暂不用) |
| `server.connected` / `server.heartbeat` / `server.instance.disposed` | `{}` | 连接生命周期 |

> 还有一整套 **`session.next.*`**(`text.delta`/`reasoning.delta`/`tool.*`/`step.*`/
> `compaction.*`):那是服务端**内部事件溯源层**,经 bridge 转换成上面的 v1 面再上线。
> **客户端不直接消费 `session.next.*`**,认上表 v1 事件即可。未知 type 一律 `Ignored`(AR12)。

### 3.2 Part 类型(`part.type`,共 12 种)

`text`、`reasoning`、`tool`、`file`、`step-start`、`step-finish`、`snapshot`、`patch`、
`agent`、`subtask`、`retry`、`compaction`。

公共字段 `{ id, sessionID, messageID, type }`。我们 Plan1 只认 `text`;`step-start/
step-finish/snapshot/patch` 是噪音不渲染(0002);`tool` 的 state 按 `status` 分
pending/running/completed/error(0002/0005)。

**只有字符串字段走 delta**(text/reasoning 的正文、tool 的 raw 输入);其余 part 靠
`message.part.updated` 全量推。→ 平滑器只服务 delta 通道。

---

## 4. 发消息:`POST /session/{id}/message`

- **同步阻塞**:请求直到 assistant 回完才返回 `{ info: AssistantMessage, parts: Part[] }`
  (流式过程同时通过 SSE 推 delta)。要异步用 `/session/{id}/prompt_async`。
- 请求体:
  ```json
  {
    "parts": [ { "type": "text", "text": "你好" } ],   // 必填;item 为 TextPartInput/FilePartInput/AgentPartInput/SubtaskPartInput
    "model": { "providerID": "...", "modelID": "..." }, // 选填,但无默认配置时不传会跑空
    "messageID": "msg_...",                              // 选填
    "agent": "...", "tools": {...}, "format": {...}      // 选填
  }
  ```
- **实测坑**:`model` 不传且服务端无默认 provider 配置时,回复为空。联调脚本显式传
  `model`(见 scripts,默认 `aliyuntokenplan/deepseek-v4-pro`)。

## 5. 其它端点

- **建会话** `POST /session`,body 可空 `{}`,可选 `{ parentID, title, agent, model }`,返回 `{ id: "ses_..." }`。
- **快照(Plan2 catch-up)** `GET /session/{id}/message`,query:`limit`、`before`(cursor 分页)、
  `directory`、`workspace`;返回 `[{ info: Message, parts: Part[] }]`。连上 SSE 后拉它补历史,
  解决"刷新丢历史 / 晚开页面看不到 / `?session=` 过滤"三件事。
- **中止** `POST /session/{id}/abort`。

---

## 6. ★ 本机实测 vs schema 的差异(易踩,优先看)★

| 项 | OpenAPI schema | 本机 build 实测 | 我们的处理 |
|---|---|---|---|
| 路径前缀 | `/api/*` 与 `/*` 并存 | 走**无前缀** `/event`、`/session` | transport 默认无前缀,可配;联调先 `api-paths.mjs` 确认 |
| `message.part.delta` 的 `sessionID` | schema 列了 `sessionID` | 运行时 delta **不带 sessionID** | `PartDeltaProps.session_id` 用 `#[serde(default)]`,否则每条 delta 解码失败→空屏 |
| 发消息返回 | `{ info, parts }` | 同步阻塞到回完才返回 | 脚本据此打印 `🤖`;画布靠 SSE |

> 这是 schema 与实际 build 的版本漂移。**以实测为准,但保留对 schema 的兼容**(可选字段而非删字段)。

---

## 7. 如何刷新本文(版本漂移时)

```bash
# 1) 起服务,dump 真实路径
node scripts/api-paths.mjs                      # 或 curl -s $SERVER/doc | jq '.paths|keys'

# 2) 从源仓库重取权威 schema(路径/字段)
cd ~/w/agentscode/opencode
jq -r '.paths|keys[]' packages/sdk/openapi.json                      # 全路径
jq '.paths["/session/{sessionID}/message"]' packages/sdk/openapi.json # 某端点详情
# 事件类型 + part.delta/updated 字段(解析 Event 联合):见 packages/core/src/session/event.ts

# 3) 用 SSE 抓真实事件样本核对字段
curl -N $SERVER/event        # 观察 server.connected / heartbeat / message.part.delta 真实 payload
```

更新本文后,若改动影响 M1/M2,顺手回填 decision 0001 的"实测修正"备注。

---

## 8. 渐进式加载约定(给 Claude 开发用)

- **Tier 0(常驻)**:AGENTS.md §4.1 一句话指针 + 最小面(§1 那 5 行)。
- **Tier 1(本文)**:动 M1 transport / M2 protocol / 联调脚本前**必读**;DEVMEM 强制约束已挂钩。
- **Tier 2(opencode 源仓库)**:仅在本文不够 / 版本漂移时,按 §7 回源仓库重取,然后**回填本文**——
  让知识沉淀在这里,而不是每次重查。
