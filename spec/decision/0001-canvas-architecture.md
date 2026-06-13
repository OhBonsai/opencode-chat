# 决策记录 0001:对话画布整体架构

- 日期:2026-06-13
- 状态:已采纳(原型验证前)
- 范围:opencode-chat 的渲染画布、排版、数据传输与流式体验

## 1. 项目目标

做一个 LLM 对话画布程序:

- 控件层用 Web React 实现;对话内容区是一块自绘画布,由 wasm 驱动
- 数据源是 opencode server 的 SSE 事件流(固定格式)
- 追求极致性能与用户体验:完全控制文字渲染、大量文字 shader 效果与动画,特别是 streaming 的舒适感
- 桌面端用 Tauri + React + wasm 复用同一套代码

## 2. 核心选型决策

### 2.1 渲染:wasm(Rust)+ wgpu

- WebGPU 优先,**WebGL2 回退为必需**(Tauri 下 WKWebView 较新 macOS 才有 WebGPU,Linux webkitgtk 最落后;WebView2 没问题)
- 不用游戏引擎(Bevy/Godot 等):画布程序不需要 ECS 和游戏循环,太重
- 不用 gpui:**无 wasm/web 后端**(仅有社区讨论 zed#8203),直接出局
- 不用 egui 做 UI:React 已是 UI 框架。egui 仅可选用于开发期调试面板(`egui_wgpu` paint callback 叠加,发布时编译开关移除)

### 2.2 排版:pretext(JS 侧),不用 cosmic-text,不用 Rust 重写

pretext(`opencode-chat/pretext`,纯 TS)负责分段、bidi、换行、富文本 inline 流,以浏览器 `measureText` 为 ground truth,并逐浏览器校准(`accuracy/*.json`)。

选 pretext 的理由:

- **零字体资产成本**:画布以中文 LLM 对话为主,cosmic-text 需自行 ship 全覆盖中文字体(5–15MB)+ emoji 彩色字体(10MB+),且 LLM 输出无法提前子集化;pretext 路线由浏览器/系统字体白拿
- **字体回退链免维护**:中英混排、代码、emoji、其他文种的 fallback 浏览器已解决
- **与 React 半边视觉一致**:同一字体引擎,度量与渲染无细微差异
- **Tauri 复用成立**:桌面端画布仍跑在 webview 里,没有原生渲染层,pretext 路线 100% 复用,且按运行时引擎自动适应 WKWebView/WebView2 的度量差异
- API 形态契合流式:`prepare()`(贵,一次)/`layout()`(纯算术热路径)分离;`layoutNextLineRange()` + cursor 支持逐行增量;`rich-inline` 支持 code span / chip / `break:'never'`

不用 Rust 重写 pretext 的理由:

- 算法部分(分段/bidi/换行)重写 ≈ 重新发明 cosmic-text
- 测量部分无法重写:准确性来自浏览器字体引擎校准;Rust 侧要么度量与浏览器不一致(即 cosmic-text 路线),要么仍回调 `measureText`(测量源还在 JS)
- 热路径 `layout()` 本来就是纯算术,搬进 Rust 无可衡量收益;`prepare()` 瓶颈在 `measureText` 本身,换语言不变
- 代价是数月边缘 case 校准 + 永久维护一个 fork

**切换条件**(排版做成可替换模块,接口为"输入文本 run → 输出带位置的 fragment 列表"):

- 需要字形内效果(逐 glyph 路径变形等)→ 换 cosmic-text
- 放弃 Tauri 改原生壳 → 重新评估

### 2.3 文字渲染管线

```
SSE → 增量文本 → pretext 分行/测量(JS)
    → 文本 run + 位置 → OffscreenCanvas 将 run 光栅化进 GPU atlas
    → wasm/wgpu 拿 atlas UV + 位置做合成、动画、shader 效果
```

- 测量与光栅化同一字体引擎,宽度严格一致
- 每个 run/grapheme cluster 一个 instance,属性带 `spawn_time`、位置、atlas UV、样式 id;一次 instanced draw
- 效果全在 WGSL:`uniform time - spawn_time` 驱动逐字符淡入/上浮/溶解;样式 id 索引效果参数表
- 效果粒度为 grapheme cluster/run 级——对流式效果视觉上与 per-glyph 无区别
- atlas 做动态分配 + LRU 淘汰(中文字符集大);emoji 由 canvas 光栅化天然支持

## 3. 数据传输设计

### 3.1 opencode SSE 协议(源码确认)

- 端点:`GET /api/event`,统一信封 `{ id, type, properties }`(`id` 是事件 id,**不是序号**)
- 连接时首发 `server.connected`,每 10s 一个 `server.heartbeat`(活性检测)
- 快照:`GET /api/session/:sessionID/message`(cursor 分页;参考桌面端取值:首屏 80 条 / 历史 200 条)

关键事件:

| 事件 | properties | 用途 |
|---|---|---|
| `message.part.delta` | `{sessionID, messageID, partID, field, delta}` | **流式热路径**,纯文本增量,append-only |
| `message.part.updated` | `{sessionID, part, time}` | 全量 part,**对账依据** |
| `message.updated` / `message.removed` | `{sessionID, info / messageID}` | 消息级元数据 |

Part 按 `type` 区分 12 种:`text`、`reasoning`、`tool`(state 按 `status` 分 pending/running/completed/error)、`file`、`step-start`、`step-finish`、`subtask`、`retry`、`patch`、`snapshot`、`agent`、`compaction`。

类型定义出处:
- `packages/opencode/src/server/routes/instance/httpapi/handlers/event.ts`(SSE 信封)
- `packages/core/src/v1/session.ts`(Part/Message schema)
- `packages/opencode/src/session/message-v2.ts`(PartDelta 定义)

### 3.2 SSE 接入:wasm 直收

Rust 侧直接用 `gloo-net` 的 EventSource(或 web-sys 绑定),serde 解析(信封用 `#[serde(tag="type", content="properties")]`,Part 用 `#[serde(tag="type")]`),JS 不经手事件数据。需要自定义 header 时改用 fetch + ReadableStream 手动解析(`eventsource-stream`)。

### 3.3 快照 + 增量 + 对账(三层)

1. **启动**:先开 SSE(事件入缓冲)→ fetch 快照建文档 → 回放缓冲,避免订阅间隙丢事件
2. **热路径**:`part.delta` 直接进对应 part 的平滑器(按 partID 各一个 reveal 队列);只 shape 新增文本,新 glyph 打 `spawn_time`
3. **对账**:`part.updated` 全量与本地累积比对,一致忽略,不一致以它为准修复(opencode 桌面端同款模式:delta 乐观追加,updated 覆盖并清空累积)。信封无序号,断线重连后直接重拉快照

### 3.4 wasm ↔ pretext(JS)边界协议

数据量本身很小(每帧几十字符、几百浮点数),要避免的是模式错误:

1. **字符串只过界一次**:delta 文本传给 pretext 一次;`prepare()` 结果留 JS 侧 `Map<u32, Prepared>`,wasm 只持 u32 句柄,之后请求只传"句柄 + 宽度"
2. **结果平铺 TypedArray 零拷贝**:wasm 预分配结果缓冲,JS 用 `new Float32Array(wasm.memory.buffer, ptr, cap)` 直接写入(固定槽位,如 `[blockHandle, graphemeStart, graphemeEnd, x, y, width]`),返回条数。注意 memory grow 后视图失效,每次调用重建
3. **每帧最多一次跨界批调用**:平滑器决定本帧上屏字符 → 攒批 → 一次 `layout_batch()`;绝不逐 part 循环调 JS
4. **atlas 像素绕过 wasm**:`OffscreenCanvas → copyExternalImageToTexture` 直接进 GPU 纹理;wasm 只拿 UV 矩形
5. **缓存边界 = markdown 块边界**:已完成块的排版/instance 冻结(留在 GPU buffer),只有生长中的尾部块每帧重排;`prepare()` 只对尾部块增量重跑。稳态每帧过界数据只与新增字符数成正比

## 4. 流式体验设计

- **速率平滑器**(丝滑感最大单一来源):SSE token 突发进缓冲区,渲染侧按动态速率匀速吐字符,水位高加速、低减速。不要按 SSE 到达节奏直接上屏
- 每字符 80–120ms 淡入 + 2–3px 上移(shader 一行 mix)
- 布局 append-only;markdown 用增量解析(逐块喂),重排限制在当前块内
- 滚动跟随带惯性/弹簧;只在用户本就在底部时跟随;区分用户滚动与自动滚动

## 5. 参考:opencode 桌面端消息处理(packages/app,SolidJS)

### 可直接借鉴

- **归一化三表**:`message[sessionID][]`、`part[messageID][]`、`part_text_accum_delta[partID]`;有序数组一律二分插入
- **delta/updated 对账**:delta 追加 + 单独累积;`part.updated` 覆盖并清空累积
- **噪音过滤**:`patch`、`step-start`、`step-finish` 不渲染
- **滚动锚底**:仅当用户在底部(阈值 4px)才跟随;流式期 rAF 连续 90 帧锚底(对付高度测量延迟),空闲降 12 帧;wheel/touch 手势检测区分用户滚动
- **内存管理**:每 store 缓存 40 个 session(驱逐连 delta 累积一起清);session 列表按"前 N + 最近 4h 内 50 个 + 有 pending 权限"裁剪
- **乐观消息**:本地发送的消息单独存,与服务端分页二分合并去重
- **revert/中断/压缩**:revert 点之后消息从视图过滤;`MessageAbortedError` 处插分隔线;compaction 显示分隔符不回放

### 我们能超越的点

- 它**没有 delta 节流/平滑**,每个 delta 同步进 store,靠框架响应式合并——token 突发时上屏发蹦。我们有自己的渲染循环,可做速率平滑 + 按帧批量
- 流式动画只有一个 `TextReveal`(CSS 级字符渐显);我们是 GPU per-character shader
- 它的虚拟化是 DOM 行级(virtua);GPU 渲染天然不需要

关键文件:`src/context/global-sync/event-reducer.ts`(reducer/delta)、`src/pages/session/message-timeline.tsx`(虚拟化/锚底)、`src/context/directory-sync.ts`(分页/乐观合并)、`src/context/global-sync/session-cache.ts`(驱逐)。

## 6. 已否决的备选方案

| 方案 | 否决原因 |
|---|---|
| 游戏引擎(Bevy/Godot/...) | 不需要 ECS/游戏循环,过重 |
| gpui | 无 wasm/web 后端 |
| egui 做整体 UI | React 已承担 UI;egui 文字排版也满足不了完全控制 + shader 要求 |
| cosmic-text 全 wasm 排版 | 中文+emoji 字体资产 15–25MB、回退链自维护、与 DOM 侧度量不一致;Tauri 下"原生复用"优势不存在 |
| Rust 重写 pretext | 测量护城河无法移植;热路径已是纯算术;数月校准成本收益≈0 |
| CanvasKit/Skia wasm | 包体 ~7MB,逻辑仍在 JS 侧,shader 自由度不如自建管线 |

## 7. 后续(待办)

1. 最小原型:Rust(wasm)连真实 `opencode serve` 的 `/api/event`,解析 `part.delta` → 平滑器 → pretext 排版 → atlas 光栅化 → wgpu 实例化渲染,带一个淡入效果,验证整条链路
2. 排版模块接口定稿(可替换 pretext/cosmic-text)
3. 提前设计的坑:选择/复制(hit-test + 选区 + clipboard)、可访问性(隐藏 DOM 镜像,供屏幕阅读器与 Cmd+F)、跨平台字体差异(UI 字体打包 webfont,中文正文用系统字体)
4. 后期优化:EventSource + OffscreenCanvas 移入 Worker,数据从网络到渲染不经主线程
