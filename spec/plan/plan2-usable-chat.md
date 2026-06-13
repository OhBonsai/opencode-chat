# Plan 2:可用的 Markdown 对话(详细任务)

- 配套:[plan1-build-guide.md](./plan1-build-guide.md)、[phase1_progress.md](./phase1_progress.md)、[../architecture.md](../architecture.md)、[../dev-practices.md](../dev-practices.md)
- 接口真相:[../knowledge/opencode.md](../knowledge/opencode.md)(动 M1/M2 先读)
- 主题:**从"流式文字可见" → "正确、可用、可滚动的 markdown 对话"**
- 相位编号续 Plan 1(A–E)→ **F–J**;一个 Phase ≈ 一个 PR,每相位末 `/dev-wrap` 过卡口。

---

## 0. Plan 2 要解决的 Plan 1 遗留

| Plan 1 现状 | Plan 2 目标 | 相位 |
|---|---|---|
| 刷新丢历史 / 晚开页面看不到 / `?session=` 不生效 | 快照 catch-up + session 过滤 | **F** |
| 每帧对每个 part 全文重排;无滚动;长对话变慢 | 滚动 + 视口裁剪 + 块冻结 | **G** |
| 纯文本直通(measureText 占位,无 markdown) | jcode markdown 语义 + 真 pretext 排版 + 代码高亮 | **H** |
| 多条 assistant 消息散开;无收尾;忘了 idle 会卡 | Part/Turn FSM + 回合聚合 + 收尾看门狗 | **I** |
| 单连接,断了不恢复 | 重连 + 心跳看门狗 + 对账强化 | **J** |

## 1. 范围

**In(F–J)**:快照/过滤、滚动/裁剪/块冻结、markdown+高亮+真 pretext、fsm/turn/收尾、重连/心跳。

**Out(留 Plan 3+)**:SDF + 富 shader 效果(发光/描边/溶解)、embed(图片/mermaid/SVG/卡片)、
内嵌标签(`<thinking>`)、选区/复制、无障碍 DOM 镜像、WebGL2/Canvas2D 降级、像素对齐相机的
DOM overlay、多标签同步、Worker 化。

**全程铁律红线**:AR1 渲染只读状态 / AR4 delta+updated 对账 / AR5 FSM 投影 / AR6 catch-up 零动画 /
AR7 grapheme / AR10 每帧一次跨界 / AR11 聚合即投影 / CR1 core 零平台依赖 / R8-R9 确定性。

---

## Phase F — 快照 catch-up + session 过滤

> 域:M1 transport · M2 protocol · M3 store。解锁"刷新不丢 / 晚开能看 / 按 session 渲染"。

**任务**
- F1 `knowledge/opencode.md` 核对快照端点:`GET /session/{id}/message?limit&before` → `[{info,parts}]`(已记)。
- F2 M1:`ChatCanvas` 启动时,**先开 SSE 入缓冲 → 拉快照(catch-up)→ 回放缓冲 → 转 live**(0003 §4 时序)。
  - 快照通过 seam 拉:新增 `trait Snapshot { fn fetch(&self, session_id) -> Vec<RawMessage>; }` 或复用 Connection 扩展;native 测试用 stub。
- F3 M2:补 `message.part.updated` 全量 part 的 catch-up 解码(text 子集即可,其余 Ignored)。
- F4 M3:`Store::apply_snapshot(messages)` 批量灌入(catch-up 模式,**零动画**:spawn_time 直接置"过去",不触发淡入,AR6)。
- F5 M2/M3:**session 过滤**——按 `sessionID` 只渲染目标会话(Plan1 全局渲染)。delta 实测无 sessionID
  (knowledge §6),故过滤要靠 `partID→messageID→sessionID` 的归属(updated/snapshot 带 sessionID 建立映射)。`?session=` 生效。

**DoD**
- 刷新页面 → 完整历史立即呈现(无逐字动画,catch-up);
- 先发消息后开页面 → 仍能看到已发生的回复;
- `?session=ses_X` 只渲染该会话;
- `cargo test`:快照灌入 + 回放缓冲不重不漏的 proptest(AR4 不变量扩展到含快照)。

---

## Phase G — 滚动 + 视口裁剪 + 块冻结

> 域:M11 input · M8 scene · M3/M13。**根治每帧全量重排 + 兑现长对话性能(核心卖点)**。

**任务**
- G1 M11:滚动状态 `scroll_offset`(core 持有);wheel/touch 事件 JS 收集每帧批量喂 wasm;
  自绘滚动条(两个矩形,非 DOM,见 0002 §6 讨论)。锚底:仅在底部才跟随 + 手势区分(阈值 ~48px)。
- G2 M8/M13:**块高度缓存** keyed by (block_id, width);**已完成块冻结**——布局结果 + glyph instance
  缓存,不再每帧重排(0002 §6)。只有"正在生长的尾部块"每帧重排。
- G3 M8:**视口裁剪**——glyph 按 y 有序,二分出可见区间 + overscan 一屏,只 build/draw 该范围。
- G4 M8:屏外块释放 instance 只留"高度+文本",滚回重排重建(pretext layout 纯算术,便宜)。
- G5 M13:`build_frame` 改为"冻结块直接取缓存 instance + 尾部块重排",不再全量。

**DoD**
- 长文档可滚,滚动 ≥60fps,锚底/手势正确;
- 10k+ 行合成会话:**fps 与内存基本平坦**(对照 Plan1 的劣化曲线,benchmark 记录,testing §3.4);
- 每帧跨界/重排量只与"尾部增长 + 可见区间"成正比,与总长无关;
- 测试:块冻结后 instance 字节级稳定(不变块不重算)的断言。

---

## Phase H — Markdown 语义 + 真 pretext + 代码高亮

> 域:M6 content · M7 layout。opencode 消息都是 md,这是"可用"的关键。

**任务**
- H1 M6:vendor `jcode-render-core`(0004 §3),`parse_markdown(text) -> Document`(BlockKind + StyledSpan)。
- H2 M6:**块级增量**——checkpoint = 最后一个完整 markdown 块;已完成块冻结,只重 parse 尾部生长块
  (与 G2 块冻结同边界,0004 §5)。
- H3 M6:remend 式尾部块"主动补全"未闭合语法(`**`/```/`$$`),消除闪烁(0004 §5.1)。
- H4 M7:**用真 pretext 替换 measureText**(Plan1 占位退场)——`StyledSpan → rich-inline fragment(role+attrs→font)→ pretext layout`(0004 §4)。接口不变,只换 `web/pretext-bridge.ts` 实现。
  - **pretext 接入(已定)**:`@chenglou/pretext` 是发布的 npm 包,已写进 `web/package.json` 的
    `dependencies`(`^0.0.8`,运行时依赖);`npm i` 即可,`import { ... } from "@chenglou/pretext"` /
    `"@chenglou/pretext/rich-inline"`。无 alias、无 `link:`/`file:`、无本地 build。
- H5 M6:代码高亮 syntect(`fancy-regex` 纯 Rust 特性,避 wasm 坑,0004 §6);按 (code_hash, lang) 缓存;
  输出样式 → 同 StyledSpan 管线。
- H6 render:glyph 支持**按 StyleRole 上色**(粗/斜/code/link/dim 等)——instance 加 style/color,glyph.wgsl 采样时应用。

**DoD**
- 真实 opencode markdown 回复正确渲染:标题/粗斜/列表/代码块(高亮)/行内 code/链接样式;
- 流式中代码块未闭合不闪烁(H3 验证);
- 中英混排 + emoji 仍锐利(pretext 度量与 measureText 一致性回归);
- 已完成块不重 parse(与 G2 协同);
- 测试:markdown 任意截断点补全后可解析、已完成块不变的 proptest(AR9 同族)。

---

## Phase I — Part/Turn FSM + 回合聚合 + 收尾

> 域:M4 fsm · M13 app。把散开的多消息聚成"一来一回",并解决"忘了 idle 卡死"。

**任务**
- I1 M4:Part FSM(Born→Streaming→Settling→Settled,投影语义 AR5);live/catch-up 双模式(AR6)。
- I2 M4:**Turn 聚合**(纯投影,AR11)——user 锚点到下一个 user 锚点之间的 assistant 消息合成一个回合;
  扁平化 part 流跨消息拼接;噪音 part(step-start/finish/snapshot/patch)不渲染(0005 §2)。
- I3 M4:**收尾判定**(0005 §4)——多信号收敛(session.status idle / message completed / step-finish)+
  **看门狗**(soft 8s→Stalled 表观降级;hard 30s→强制 settle 可复活);心跳区分"模型停了"vs"连接死了"。
- I4 M2/M4:扩 protocol 认 `reasoning`/`tool` part + `session.status`/`session.idle`/`message.updated`(knowledge §3.1)。
- I5 M13/render:tool part 按 status 渲染(pending/running/completed/error)可折叠;reasoning 区折叠;
  Turn 收尾触发收尾表现(光标隐等,效果占位即可,富效果留 Plan3)。

**DoD**
- 一句"修个 bug"→ 思考+工具+正文聚成**一个连续回合**,工具调用可折叠;
- 模型不发 idle 时 30s 内自动收尾解禁(不永久 loading);连接僵死则走重连(Phase J)不误判收尾;
- 测试:收尾判定矩阵逐行 case(idle 丢失/忘了 idle 有心跳/连接死/超时可复活)+ Turn 边界乱序重投影一致(AR11)。

---

## Phase J — 容错:重连 + 心跳看门狗 + 对账强化

> 域:M1 transport · M3 store。让弱网/重连下不丢不错不卡。

**任务**
- J1 M1:SSE 断线重连(EventSource 自带 + 我方退避);重连后**拉快照与本地逐 part diff,只对差异 part catch-up 修复**(0003 §3.4,不整体重置避免闪烁)。
- J2 M1:心跳看门狗——>~25s 无任何事件(含 heartbeat)→ 主动断开重连(0003 §3.5);与 I3 的"模型停了"看门狗区分清楚(一个看 part 活动,一个看连接活动)。
- J3 M3:对账强化——孤儿 delta(part 未 Born)按 partID 有上限缓冲,updated 到达回放(0003 §3.3,保住流式动画);超限丢弃等对账。
- J4 M2:未知/新增 part 类型稳健 Ignored 回归(AR12);protocol 版本漂移容忍。

**DoD**
- 联调中拔网/重启 server → 自动重连 + 快照修复,画面不闪不错;
- 注入丢/重/乱序事件(录像故障注入)→ 最终状态 == 快照状态(AR4 总不变量);
- 心跳停 → 重连而非收尾;part 活动停 → 收尾而非重连;
- 测试:故障注入套件(testing §2.5)+ 孤儿 delta 缓冲回放 case。

---

## 2. 依赖与推进顺序

```
F(快照/过滤) → G(滚动/裁剪/块冻结) → H(markdown/pretext/高亮) → I(fsm/turn/收尾) → J(容错)
```
- F 先行:它建立 session 归属映射,后续都受益;且解决最痛的"刷新丢历史"。
- G 在 H 前:块冻结/裁剪的"块"边界,H 的 markdown 块解析要复用同一边界——先把块基础设施立起来,H 往里填语义。
- I 依赖 H(reasoning/tool part 要能渲染)与 G(回合作为裁剪/驱逐单元)。
- J 收口,贯穿前面留的 seam(Snapshot/Connection)。

> 可并行的小块:H5 高亮、J3 孤儿缓冲相对独立,可插空做。

---

## 3. 验证点记录(Plan 2 收尾填,喂 Plan 3)

| 验证点 | 预期 | 实测 |
|---|---|---|
| 长对话(10k 行)fps/内存曲线 | 平坦(vs Plan1 劣化) | |
| 块冻结后尾部重排开销 | 只与尾部增长成正比 | |
| pretext 真排版 vs measureText 度量差 | 无可见回归 | |
| 流式 markdown 闪烁 | remend 后消除 | |
| 忘了 idle 的收尾延迟 | ≤30s 自动解禁 | |
| 重连快照修复 | 不闪不错,状态收敛快照 | |

---

## 4. 风险与回退

- **pretext 接入(H4)**:已是 npm 直接依赖(`@chenglou/pretext ^0.0.8`,在 `web/package.json` 的
  `dependencies`),`npm i` 即用,无 alias/无本地 build。仓库内 `pretext/` 本地副本与构建无关(gitignore,保持忽略)。
  先在纯 JS 跑通 pretext layout 再接 wasm 桥。
- **块冻结/裁剪(G)与 markdown 增量(H)耦合**:务必共用同一"块边界"定义,否则两套缓存打架——
  G 先定义块边界数据结构,H 复用。
- **收尾看门狗阈值(I3)**:8s/30s/25s 为初值,联调按真实模型节奏调(慢模型可能 >30s 仍在想)。
- **任一 Phase 不通**:回到上一个可跑态,不堆积(同 Plan1 §8)。

---

## 5. Plan 3 预告(本计划明确不做)

SDF 字形(运行时 TinySDF/ESDT,移植 infinite-canvas-tutorial L15)+ 富 shader 效果(发光/描边/溶解,
依赖 SDF)、embed(图片→mermaid→卡片,0007 三层)、内嵌标签(0006)、选区/复制 + 无障碍镜像、
WebGL2/Canvas2D 降级(0003 §5)、像素对齐相机 DOM overlay(0007 §3)。其中 **SDF + 效果**建议先写
`decision/0009-glyph-atlas-representation.md`(位图→SDF 的 atlas 接口抽象)再动手。
