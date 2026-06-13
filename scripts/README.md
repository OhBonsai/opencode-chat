# scripts/ · Plan 1 联调脚本(Node mjs)

让 opencode server 上有一个**正在产字的 session**,我们的画布(连全局 `GET /event`)
就把 `message.part.delta` 逐字淡入上屏。Node ESM(`.mjs`,需 Node ≥ 18 全局 `fetch`)。

> 本机这版 opencode 路由**无 `/api` 前缀**(实测 `/session`、`/event`)。`chat.mjs` 已按此
> 默认,并保留探测回退。

## 三步走(顺序重要)

Plan 1 没接快照 catch-up,SSE 只推"连上之后"的事件 —— **先开页面再发消息**:

```bash
# 终端 1:起服务
node scripts/serve.mjs                      # 默认 4096,PORT= 覆盖

# 终端 2:起前端 + 先开画布(连上 SSE)
node scripts/dev-web.mjs                     # 构建 wasm + 起 Vite
open "http://localhost:5173/?server=http://localhost:4096"

# 终端 3:多轮对话(每轮回复同步淡入画布)
node scripts/chat.mjs
```

## chat.mjs · 多轮对话

复用同一 session(保留上下文)+ REPL 循环。默认模型 `aliyuntokenplan/deepseek-v4-pro`。

```bash
node scripts/chat.mjs                        # 交互多轮(Ctrl-D 或 /exit 退出)
node scripts/chat.mjs "第一句"               # 先发一句再进多轮
node scripts/chat.mjs --once "只发一句"       # 单次(脚本/CI)
echo "你好" | node scripts/chat.mjs           # 管道单条
SESSION=ses_xxx node scripts/chat.mjs         # 续接已有会话
MODEL=deepseek/deepseek-chat node scripts/chat.mjs   # 换模型
```

每轮 `🤖` 一定打印回复(同步 `session.prompt`);画布只要第 2 步开着就同步淡入。

## 真实 API(核对源)

源:`~/w/agentscode/opencode/packages/sdk/openapi.json`(`session.prompt`)

| 动作 | 方法 路径 | body |
|---|---|---|
| 建 session | `POST /session` | `{}` → `{ id }` |
| 发消息 | `POST /session/{id}/message` | `{ parts:[{type:"text",text}], model:{providerID,modelID} }` → 同步返回 `{ info, parts }` |
| 事件流 | `GET /event` | SSE,`message.part.delta`(`{messageID,partID,field,delta}`,无 sessionID) |

## 脚本一览

| 脚本 | 作用 |
|---|---|
| `serve.mjs` | spawn `opencode serve --port $PORT` |
| `dev-web.mjs` | 构建 wasm + 起 Vite harness |
| `chat.mjs` | 多轮对话(复用 session,带模型) |
| `api-paths.mjs` | dump OpenAPI 路径,排查真实 API |

## 环境变量

| 变量 | 默认 | 说明 |
|---|---|---|
| `PORT` | 4096 | serve 端口 |
| `SERVER` | http://localhost:$PORT | server 基址 |
| `WEB_PORT` | 5173 | Vite harness 端口 |
| `MODEL` | aliyuntokenplan/deepseek-v4-pro | `provider/model` |
| `SESSION` | (新建) | 续接已有 session id |

## 排查

```bash
node scripts/api-paths.mjs                   # dump OpenAPI(/doc 或 /openapi.json)
```

- **没对话**:多半没传/传错 model →`chat.mjs` 会打印真实错误(如 `Model not found`)。
- **画布空但终端有回复**:时序/快照问题 → 先开页面再发;刷新会丢历史(Plan 1 未接快照)。
- **CORS**:浏览器(5173)连 opencode(4096)跨源,连不上按 F12 看 Console。
