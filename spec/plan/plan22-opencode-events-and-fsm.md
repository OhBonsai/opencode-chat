# Plan 22:完成 opencode 全事件 — 边界 / 渲染 / 状态转化 / 错误处理

- 日期:2026-06-24
- 状态:**P0–P5 全相完成(2026-06-29)**;native 247 测 + vitest 8 测 + Playwright(E3/E4)全绿经 `verify`/`test:unit`/`playwright`。F1–F12 容错全覆盖(F1 no-reply wall-clock + 真服务联调为人工 TODO §7.3)。落地 [0031](../decision/0031-event-fsm-resilience-and-js-rust-boundary.md)(韧性+边界)、扩 [0001](../decision/0001-canvas-architecture.md)/[0002](../decision/0002-event-driven-pipeline.md)/[0005](../decision/0005-turn-aggregation-and-settlement.md)。
  - **P0 传输边界**:`QueueConnection`(core,host 喂队列)+ `Engine` 注入队列 + wasm `push_event`;`web/src/sse-client.ts`(SSE:指数退避重连/连接超时/35s 僵尸自愈/cache-bust)替换 Rust 内 SSE,`main.ts` 服务端路径挂它喂 `push_event`。**E1(vitest 7 测:退避/超时/僵尸/cache-bust)✅**
  - **P1 全事件/Part 解码 + 承载**:`protocol.rs` `Event` 全 8 类 + `Part` 全分类(载荷 `serde_json::Value`);`store.rs` 分类 `PartRow` + `apply_part_removed` + `display_source`。未知→Ignored/Other(AR12)。**N1/N2 ✅**
  - **P2 SessionStatus FSM**:`SessionStatus`(8 态)+ `FsmInput` + `next_status`(纯函数穷尽 match)+ 派生量;**已接入 `ingest_events`**(全事件驱动)。**N4 ✅**
  - **P3 通用兜底渲染 + 分组**:`display_source` 兜底 markdown 经既有 `parse_markdown_nodes` 全管线 → **每个 part 都看得见**(标签+内容);tool 重写重渲;`group_message_parts`(三桶+context 折叠)。**N7/N8 + p3 集成 + E3(浏览器全 part 可见)✅**
  - **P4 错误处理 + Dock**:错误卡恒一张(F4)/ ghost-abort 不弹卡(F3)/ 停止冻结消息(F11)/ epoch(F12)/ 孤儿 view 清理;`web/src/dock.ts` 权限·反问 Dock(读 `session_status()`,`reply_*` 解阻)。wasm `session_status()`/`note_send()`/`stop_turn()`/`reply_permission|question()`/`push_event()`。**p4 重放(F3/F4/F11)+ p0 注入 + E4(Dock 弹出/应答)✅**
  - **P5 韧性**:FSM 全事件驱动 + 冻结集(F11)+ epoch(F12)+ `resilience.rs` 纯逻辑 **F6**(`merge_ordered` 非破坏时序合并)/ **F8**(`should_bottom_out` 无回复收尾注兜底卡,已接 idle)/ **F9**(`is_quota_error` 配额标注,已接 SessionError)/ **F10**(`temp_should_replace` 去重判据)。**F2**(僵尸自愈)由 `sse-client` 35s 僵尸表覆盖。**E2**(断连→重连→`server.connected`→resync)vitest 覆盖逻辑。
  - **唯一真 TODO(需真服务,人工;§7.3)**:**F1**(no-reply wall-clock 计时由 host TS 起,app 已置 `AwaitingAck` 供其轮询)+ 真实 opencode server 全事件联调冒烟。F10 去重判据已测,接 chat-input 发送侧(temp 用户消息)是 send 流程的小增量。
  - 备注:回合活跃/收尾由 `TurnTracker`(揭示收尾)+ `SessionStatus` FSM(生命周期态)并行,不回退。
  - **测试总账(全绿)**:native `verify` 247 测(N1/N2/N4/N7/N8 + p0 注入 + p3 集成 + p4 F3/F4/F11 + p5 F8/F9 + resilience F6/F8/F9/F10)、vitest 8 测(E1 退避/超时/僵尸/cache-bust + E2 重连 resync)、Playwright E3(全 part 兜底可见)+ E4(权限/反问 Dock)。
- 前置必读:`spec/knowledge/opencode.md`(接口真相)、[0031](../decision/0031-event-fsm-resilience-and-js-rust-boundary.md)、[0006](../decision/0006-inline-tags-and-extensibility.md)(reasoning/思考区)、[0007](../decision/0007-rich-media-embeds.md)(tool/file 卡片)、[0027](../decision/0027-code-block-viewport.md)(patch/diff)、[0029](../decision/0029-session-virtualization-and-glyph-working-set.md)(新块走 tier 虚拟化)。
- 目标(**= 搭起能跑通的「丑骨架」**):把"只渲染 assistant 纯文本"升级为"opencode 全事件/全 Part **正确解码 + 承载 + 状态 + 容错**,且**每个 part 都用最简方式渲染出来**"。四件事:① 全事件/全 Part 解码与承载;② **通用兜底渲染**(每 part = 标签 + 原始内容,text/markdown/JSON 皆可,**不做漂亮卡**);③ 状态转化(SessionStatus FSM);④ 错误处理(错误卡 + F1–F12 + 权限/提问)。TS/Rust 边界按 0031。
- **与 [Plan 23](./plan23-part-render-implementation.md) 的分工(可并行)**:**Plan 22 = 框架 + 通用兜底渲染器**(任何 part → 标签 + JSON/markdown,丑但完整);**Plan 23 = 每类专用漂亮渲染器**,经**渲染契约**(见 §3.2)注册进同一分派点逐个覆盖兜底。两份 plan **冻结契约后即可并行**:Plan 22 不依赖 Plan 23(兜底自洽),Plan 23 只对着契约写纯渲染函数、不碰事件/状态。
- 取向(扣 opencode 调研):**这正是 opencode 的 `PART_MAPPING` + `GenericTool` 兜底模式**——Plan 22 做分派器 + 兜底,Plan 23 做 specific 渲染器。

---

## 0. 现状盘点(为什么这是大工程)

| 层 | 现状 | 缺口 |
|---|---|---|
| protocol `Event` | 7 种(PartDelta/PartUpdated/MessageUpdated/SessionStatus/Connected/Heartbeat/Ignored) | 缺 8 类事件(见 §2) |
| protocol `Part` | **仅 `Text` + `Other`**(其余 `#[serde(other)]` 丢弃) | reasoning/tool/file/patch/step/compaction **全被丢** |
| store | `PartRow.text: String`(**纯文本**) | 无法承载非文本 part 的结构化载荷 |
| content/render | 只把 text → markdown → StyledSpan | 无 reasoning/tool/file/patch/step/compaction 渲染 |
| fsm | `TurnStatus` 4 态 | 无 AwaitingAck/Blocked/Stopped/Errored(0031) |
| transport | Plan1 stub(Rust,无韧性) | 移 TS + 重连/心跳/对账(0031) |

> 结论:这不是补几个分支,是**数据模型(store)从"文本"扩成"分类 part"+ 渲染从"文本"扩成"多种块"+ 状态从"收尾"扩成"全生命周期"**。分 6 相,每相一个可见的纵切。

---

## 1. 一句话架构(扣 0031 边界)

> **TS(transport.ts)产 raw 事件 + 连接生命周期 → Rust(protocol 解码 → store 分类承载 → fsm 状态转化 → content/render 出块)→ FrameData。** 错误/韧性判定全在 Rust(F1–F12);I/O/对话框在 TS。

---

## 2. opencode 全事件清单(完成度矩阵)

> 来源:`spec/knowledge/opencode.md` + 钉钉 §2.3。**每行 = 一个解码变体 + 一个状态/渲染效果 + 一个测试。**

| 事件 | 现状 | 落 | 解码(protocol) | 状态/渲染效果 |
|---|---|---|---|---|
| `server.connected` | 解码✅未接 | P0 | `Connected` | **触发 `resync`**(0031 §5.4) |
| `server.heartbeat` | 解码✅未接 | P0 | `Heartbeat` | 喂"活着"时间戳(TS 僵尸看门狗) |
| `server.instance.disposed` | ❌ | P0 | `InstanceDisposed` | 流将关→等重连 |
| `session.created` | ❌ | P1 | `SessionCreated{parent_id}` | `parent==当前` → 登记子会话(过滤/abort 范围) |
| `message.part.updated` | ✅ | P1 | `PartUpdated{part}` | 按 `part.id` upsert |
| `message.part.delta` | ✅ | P1 | `PartDelta` | 增量拼接 |
| `message.part.removed` | ❌ | P1 | `PartRemoved{part_id}` | 删 part |
| `message.updated` | ✅ | P2 | `MessageUpdated{info}` | 合并 info;带 error → `Errored`(§5) |
| `session.status` | ✅ | P2 | `SessionStatus{status}` | 回合主信号 idle/busy/retry |
| `session.idle`(旧式) | 解码归一✅ | P2 | →`SessionStatus{idle}` | 归一化(已做) |
| `session.error` | ❌ | P4 | `SessionError{name,data}` | ghost-abort/配额/错误卡(F3/F9/F4) |
| `session.compacted` | ❌ | P4 | `SessionCompacted` | 整列失效 → 重拉 `getMessages` |
| `session.updated` | ❌ | P1 | `SessionUpdated{title}` | 更会话元信息 |
| `permission.asked/replied` | ❌ | P4 | `Permission*` | `Blocked{permission}` + Dock |
| `question.asked/replied/rejected` | ❌ | P4 | `Question*` | `Blocked{question}` + Dock |

**铁律**:未知 `type` 仍 → `Ignored`(AR12,向前兼容 + 你将来扩自定义类型同样走这);字符串只过界解码一次(0001)。

---

## 3. 全 Part 承载 + 通用兜底渲染(本 plan 只做「丑骨架」)

> 把 `Part` 从 `Text|Other` 扩成完整分类联合,**store 承载结构化载荷**;**渲染只做通用兜底**(标签 + 原始内容),**漂亮卡留 Plan 23**。

### 3.1 兜底渲染规格(每 part → 一个标签块)

每个 part 渲染成**一个块**:**首行标签**说清「是什么(+状态)」+ **正文是原始内容**。一切走已有的 markdown→StyledSpan 路径,结构化载荷直接当代码块 dump。

| Part | 承载(store) | 兜底渲染(Plan 22) | Plan 23 接手 |
|---|---|---|---|
| `text` | text | markdown(已有) | 富排版/footer meta |
| `reasoning` | text | `[reasoning]` + markdown(可弱化色) | 思考区折叠 + 完成 auto-collapse(0006) |
| `tool` | `{tool,state{status,input,output,...}}` | `[tool:<name> · <status>]` + **input/output 的 JSON 代码块** | 工具卡 + 徽章 + 折叠 + per-tool 布局(0018,Plan 23) |
| `file` | `{filename,mime,url}` | `[file:<name>]` + url 文本 | 图片纹理 / 文件 chip(0007) |
| `compaction` | `{auto?,overflow?}` | **分隔线 + 标签"上下文已压缩"**(0026,简单) | — |
| `patch` / `step-start` / `step-finish` | **SKIP**(对齐 opencode reducer:不存不渲染;diff 走 tool 的 `metadata.filediff`,见 [opencode 调研](../research/opencode-desktop-part-rendering-and-interaction.md)§0) | 不渲染 | — |

> **修正(扣调研)**:`patch`/`step-start`/`step-finish` **不渲染**(opencode `SKIP_PARTS`);diff **挂在 tool part 的 `state.metadata.filediff`**,不是独立 patch 块;token/cost 不显示。

### 3.2 渲染契约(与 Plan 23 的并行边界)→ **已抽成 [ADR 0033](../decision/0033-part-render-contract.md) 并实现**

两份 plan 唯一的接触面 = **part 投影 + 渲染分派 registry + Block 输出**,**已冻结为独立 ADR 0033 并实现于 `crates/core/src/partrender.rs`**(`PartKind` / `RenderPart` / `RenderCtx` / `RenderFn` / `fallback_render` / `RenderRegistry` + 单测)。**本 plan 不再内联契约定义,以 0033 为单一真相源**(改契约双方都须知)。

- **Plan 22 职责**:`store` 据 part 填 `RenderPart` 投影;`content/build_frame` 建 `RenderRegistry::new()`(全兜底)经其出块(走 tier)。**只装兜底,不写 specific。**
- **Plan 23 职责**:`register(kind, specific)` 逐类覆盖,不碰事件/状态。
- **不变量(0033 §3)**:全兜底 = 对所有 part(含 `Unknown`)都能工作的完整(丑)UI、不 panic;specific 逐类覆盖,未覆盖走兜底 → **UI 始终完整,只是越来越漂亮**;`RenderFn` 纯函数(CR1/R8),兜底与 specific 共用渲染快照测试。

### 3.3 回合分组(结构,双方共用)

一轮 = 用户消息 + 其后连续 assistant 消息;`group_message_parts`(core 纯函数,CR1)把轮内 part 分**三桶**:辅助时间线(reasoning/tool/中间文本)/ 最终回复(尾部 text/file)/ (Plan 23)diff 摘要;**read/glob/grep/list 连续段折叠成 context 组**(降噪,采纳 opencode/调研)。**分组是结构契约,兜底与 specific 渲染器都按桶排布**;所有块走 0029 tier 虚拟化。

### 3.4 状态驱动 UI(render 读 SessionStatus)

- `AwaitingAck`/`Streaming`/`Retrying`/`Blocked` → "活跃"指示(自绘);
- `Blocked{permission|question}` → 弹 Dock(DOM overlay,0022);
- `Errored` → 错误卡(§5);`Stopped` → "已停止"标记。

---

## 4. 状态转化(SessionStatus FSM,落 0031 §5.2/5.3)

```rust
enum SessionStatus { Idle, AwaitingAck{sent_at}, Streaming{message_id},
                     Retrying{attempt}, Blocked{on}, Stalled, Stopped, Errored{error} }
fn next_status(cur, ev) -> SessionStatus   // 纯函数,穷尽 match,proptest
```

转移要点(每条配单测):发送 → `AwaitingAck`(起 no-reply timer,F1);首个 part → `Streaming`;`session.status:retry` → `Retrying`(F8);permission/question.asked → `Blocked`;reply → 回上一态;stop → `Stopped`(F11);`SessionError`/`MessageUpdated(error)` → `Errored`(F3/F4);idle/finish/hard-watchdog → `Idle`(收尾,加 ~300ms 防抖 F7)。**派生量**(is_active/can_send/streaming_id)全从 status 算,杜绝散落 boolean。**timer 即状态副作用**,集中在 controller 一处管理(dt_ms 驱动,R8)。

---

## 5. 错误处理

| 场景 | 处理 | F |
|---|---|---|
| idle 先于 error / 重复 | 合成卡 `error-` 前缀;真 error 到清陈旧卡 → **恒一张** | F4 |
| 503 耗尽无 message.updated | idle 收尾时末条 assistant 无 finish 无 error → 兜底注 APIError 卡 | F8 |
| ghost-abort(AbortError 无 part) | 删 `temp-` 用户消息、**不弹错误卡**、回 Idle | F3 |
| 额度耗尽(两路径) | `MessageUpdated`/`SessionError` 都抽原因 → 统一 `Blocked`/配额 Dock + 发送前预检 | F9 |
| temp 去重 | 真用户事件文本匹配 `temp-` 才回显移除,否则按 assistant | F10 |
| stop 冻结 | 冻结 id 集 + stoppedAt → 丢弃针对冻结消息事件,resync 也跳过;abort 父+子(TS POST) | F11 |
| no-reply / busy 假死 | AwaitingAck/hard-watchdog 到点 → **先 resync 再决策**(回完了补出不叠卡,否则注卡) | F1/F2 |
| compaction | 整列失效 → 触发重拉(TS fetch → resync) | — |

合成卡纯函数:`make_synthetic_error`、`upsert_error_card`(幂等)、`has_real_response_in_cycle`、`merge_server_messages`(非破坏 + 时间排序,F6)。全 Rust、native 可测。

---

## 6. 改动清单(精确到 file:符号)

**core(`crates/core`)**
- `protocol.rs`:扩 `Event`(§2 全 8 类)+ `Part`(§3 全分类联合,带载荷)+ 解码;未知→Ignored 回归。
- `store.rs`:`PartRow` 从纯文本 → **分类 part 模型**(text 累积 + 非文本结构化载荷);`apply_part_removed`、`SessionCreated`(子会话表)、`SessionUpdated`(元信息)、`merge_server_messages`(非破坏)。
- `fsm.rs`:`TurnStatus`→`SessionStatus` 联合 + `next_status` 纯函数 + timer 副作用集中。
- `partrender.rs`(**已实现,0033**):`PartKind`/`RenderPart`/`RenderCtx`/`RenderFn`/`fallback_render`/`RenderRegistry` 渲染契约 + 兜底。本 plan 直接复用,不重写。
- `store.rs`/`content.rs`:据 part 填 `RenderPart` 投影(kind_tag/text/payload_json);`group_message_parts`(三桶 + context 折叠);**漂亮 specific 渲染器属 Plan 23,本 plan 只用兜底。**
- `app.rs`:`ingest_events` 接全事件;`Connected`→resync、hard-watchdog→先对账;错误卡逻辑;子会话过滤;epoch 守卫(F12);build_frame 经 `RenderRegistry`(全兜底)出块(走 tier)。
- `seam.rs`:`Connection` 改 TS 喂队列契约 + intent 回传(reconcile/probe/abort)。

**wasm(`crates/wasm`)**
- `lib.rs`:`push_event(raw)`(TS 喂)、`take_intents()`(Rust→TS 的 reconcile/probe/abort)、`reply_permission`/`reply_question`、`stop`;`stats()` 加 SessionStatus。
- 删 `transport.rs` 的 SSE 实现(留 native stub 或移除)。

**web(`web/src`)**
- 新 `transport.ts`(照搬钉钉 sse-client:重连/超时/僵尸/cache-bust;connected→`chat.resync`;按 intent 做 probe/abort/fetch)。
- `permission-dock.ts`/`question-dock.ts`(DOM overlay,0022)。
- `main.ts`:挂 transport.ts(替代旧 Rust SSE 路径)+ intent pump。

**测试**:完整清单见 **§7**(自动化 N/E + 人工 TODO);每相 PR 必带对应 id 且全绿(FAIL_TO_PASS ∧ PASS_TO_PASS)。

---

## 7. 测试方案(自动化清单 + 人工 TODO;接 [Plan 20](./plan20-minimal-test-pipeline.md))

> 韧性 = 确定性重放(你的护城河)。给 Claude Code 直接执行:§7.1/§7.2 = 可自动化(每条 = 一个可写测试);§7.3 = 人工(需真机/真服务/人眼)。

### 7.1 Native 单元/属性测试(`cargo test`,最快最稳)

| # | 测试名 | 断言 | 相 |
|---|---|---|---|
| N1 | `decode_roundtrip_all_events` | §2 每类 Event/Part 解码 round-trip;未知 `type`/Part → `Ignored`/`Other`(AR12) | P1 |
| N2 | `store_part_upsert_removed_idempotent`(proptest) | 任意 delta/updated/removed 序列 → 三表幂等、有序 | P1 |
| N3 | `store_temp_dedup_and_merge`(proptest) | temp 文本匹配才回显移除(F10);非破坏 merge 按 `time.created` 有序(F6) | P4/P5 |
| N4 | `next_status_transitions` + 穷尽 match | `next_status` 每条转移单测;`SessionStatus` 全覆盖(编译期穷尽) | P2 |
| N5 | `fsm_failure_modes_replay` | **F1–F12 各一条事件序列重放**:F4 恒一错误卡 / F2 三分支(busy·idle有回复·idle无回复)/ F11 冻结丢弃 / F3 ghost-abort 删 temp 无卡 / F8 兜底卡 | P4/P5 |
| N6 | `replay_invariants`(proptest) | 跨随机事件序列:消息时间有序、错误卡 ≤1、temp 终去重或转正 | P5 |
| N7 | `group_message_parts_deterministic`(proptest) | 三桶分类 + context 折叠确定 | P3 |
| N8 | `fallback_conforms_to_contract`(**已实现,0033**) | 兜底渲染器过契约一致性闸:不 panic + 确定 + 内容不丢 | P3 |

### 7.2 TS / E2E(vitest + Playwright,复用现有 `playwright.config.ts`)

| # | 文件 › 用例 | 断言 | 相 |
|---|---|---|---|
| E1 | `transport.spec.ts`(vitest + fake EventSource) | 重连退避 `min(1000·2^n,60s)` / 连接超时 10s / 35s 僵尸自愈 / cache-bust | P0 |
| E2 | `resilience.spec.ts`(Playwright) | 断网 → 自动重连 → `server.connected` → `resync`(端到端) | P0/P5 |
| E3 | `render-fallback.spec.ts`(Playwright) | **全 part 兜底可见**:每类 part 渲染出 `[标签]` + 内容(截图/DOM 断言身份+内容都在) | P3 |
| E4 | `dock.spec.ts`(Playwright) | permission/question Dock 弹出 + 三选/答题 + reply 端到端;Dock 阻塞 turn | P4 |

### 7.3 人工测试(TODO — 需真机/真服务/人眼,不自动化)

- [ ] **真实 opencode server 联调**(全事件真流,非合成):覆盖 N5 难造的真错误/重试/压缩。
- [ ] **后台挂起 2 分钟回前台**(WebView 节流真机):验 F2 僵尸 + 重连对账。
- [ ] **状态指示 / Dock 观感**(主观,人眼):活跃指示、错误卡、阻塞态是否清楚。

### 7.4 机器可读输出(接 Plan 20)

- native → `cargo test`(或 nextest JUnit);Playwright → JUnit reporter;`verify` 聚合进 `TESTREPORT.md`。

### 7.5 验收门(= 自动化全绿 + 卡口)

| 相 | 必过自动化 |
|---|---|
| P0 | E1 · E2 |
| P1 | N1 N2 |
| P2 | N4 |
| P3 | N7 N8 · E3 |
| P4 | N3 · E4 |
| P5 | N5 N6 · E2 |

---

## 8. 分相落地(每相一个可见纵切;PR 切分)

| 相 | 范围 | 产出物 | 验收 |
|---|---|---|---|
| **P0 边界+transport** | `Connection`→TS 喂队列;`transport.ts`(重连/心跳/僵尸/cache-bust);`Connected`→resync | 断网自动重连、重连对账 | 断网→重连→`server.connected`→resync;transport vitest 绿 |
| **P1 全事件/Part 解码+承载** | 扩 `Event`/`Part` 全集;store 分类 part 模型;part.removed/created/updated/compacted | 全事件不再丢(承载,text-only 渲染保持) | 解码 round-trip 全绿;未知→Ignored |
| **P2 状态转化** | `SessionStatus`+`next_status`+timer 副作用;接全 status 事件;状态驱动指示 | 富 FSM,派生量从 status 算 | `next_status` 每转移单测;AwaitingAck/Retrying/Blocked 可达 |
| **P3 通用兜底渲染** | 渲染分派 registry + **兜底渲染器**(每 part = 标签 + markdown/JSON)+ 回合三桶分组 + context 折叠(走 tier) | **所有 part 都看得见**(标了身份、内容完整,丑) | 兜底渲染快照确定性;三桶分组确定;新块虚拟化(几何∝可见);冻结 §3.2 契约 |
| **P4 错误处理** | 错误卡去重/兜底/ghost-abort/配额/temp/stop;permission/question Dock | 错误/中止/权限正确 | F3/F4/F8/F9/F10/F11 重放绿;Dock 应答端到端 |
| **P5 韧性硬化** | F1/F2/F6/F12 reconcile 精炼 + epoch;全 F 测试闭环 + 埋点 | 全韧性覆盖 | F1–F12 全绿;断网/后台挂起/切会话端到端 |

> 估:每相 1–3 PR。**P0/P1 是地基**(边界 + 数据模型),**P3 = 丑骨架完整可见**(全 part 标签+内容,**冻结渲染契约 §3.2**)、**P4/P5 是韧性**。可按"先承载(P1)→先看见(P3 兜底)→再容错(P4/P5)"交付,中途任何相都端到端可跑。**P3 契约冻结后,[Plan 23](./plan23-part-render-implementation.md) 即可并行开发漂亮渲染器,不阻塞 P4/P5。**

---

## 9. 铁律与风险

- **CR1**:protocol/store/fsm/分组/错误卡全在 core(纯逻辑、native 可测);transport/Dock/I/O 在 TS。
- **R8 确定性**:事件→状态→渲染全纯函数;选区/相机/transport timer 不进重放内容;F1–F12 用录像重放当 oracle。
- **AR12 向前兼容**:未知事件/Part→Ignored;扩自定义类型 = 加变体+转移,不动骨架。
- **0029 虚拟化**:tool 卡/reasoning 区/diff 都是块 → 走 Hot/Warm tier,屏外释放;**别让长会话的工具卡撑爆内存**。
- **边界录制点不变**:TS 喂事件入口 = replay 入口 → transport 移 TS 不破重放(0031 §3)。
- **timer 双层**:确定性逻辑 timer(no-reply/看门狗/防抖)在 Rust 发 intent;wall-clock I/O timer(重连/僵尸)在 TS(0031 §3.1)。
- **风险**:store 的"文本→分类 part"是数据模型改动(影响虚拟化重建源、快照对账)——P1 必须配 proptest 守 upsert/merge 幂等 + 重建等价(接 0029 R8 不变量)。

---

参考:[0031](../decision/0031-event-fsm-resilience-and-js-rust-boundary.md)(韧性+边界)· `spec/knowledge/opencode.md`(接口真相)· 钉钉《OpenCode Sessions 干净实现计划》F1–F12([链接](https://alidocs.dingtalk.com/i/nodes/mweZ92PV6M9Lb2mxUMPa5RPaWxEKBD6p))· [0006](../decision/0006-inline-tags-and-extensibility.md)(reasoning 区)· [0007](../decision/0007-rich-media-embeds.md)(tool/file 卡)· [0027](../decision/0027-code-block-viewport.md)(patch diff)· [0029](../decision/0029-session-virtualization-and-glyph-working-set.md)(tier 虚拟化)· [Plan 20](./plan20-minimal-test-pipeline.md)(测试流水线)。
