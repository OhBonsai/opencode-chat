# Plan 23:part 渲染实现方案(tool / reasoning / file / diff / compaction)

- 日期:2026-06-24
- 状态:**核心落地 + 端到端接通(2026-06-29)**;R1–R4 渲染器实现并经 registry 接入 `build_frame`,tool/reasoning/compaction 漂亮卡可见;GPU 面板装饰 / DOM 热区 / R5–R6 待后续相;**消费 [ADR 0033 渲染契约](../decision/0033-part-render-contract.md)(已实现于 `crates/core/src/partrender.rs`)**,只实现 registry 的 **specific 漂亮渲染器**,**不碰事件/状态/store**。

> ## 进展(2026-06-29)
>
> **已落地(`crates/core/src/partspecific.rs`,纯 core / CR1 / R8,native 全绿)**:
> - **R1**:`reasoning_render`(合成 `💭 Thinking` 标题 + 弱化正文 + 折叠态)+ `compaction_render`(标签 + Rule 分隔线)。
> - **R2**:`tool_render`(`▸ name [status]` 标题徽章 + 状态分派 args/output;pending/running 隐藏 body;error 显错;工具名二级分派)。
> - **R3**:`diff_parse_lines`(纯函数)+ diff 块渲染(增/删/上下文角色 + `+x -y` 摘要);diff 挂 tool 的 `metadata.filediff`。
> - **R4**:`group_message_parts`(三桶 + read/glob/grep/list 连续段折叠成 context 组)。
> - **角色**:`StyleRole` 追加 51–57(Reasoning/ToolTitle/ToolArg/ToolOutput/ToolBadge/DiffAdded/DiffRemoved,数值稳定)+ `glyph.wgsl` 配色。
> - **注册**:`default_registry()` 注册 R1/R2(Tool 含 R3 二级分派),其余 kind 继续走兜底(UI 始终完整)。
> - **测试**:N2(diff proptest)· **N3 契约闸 ×3 渲染器** · N4(insta 快照 ×5)· N6(覆盖)。R4 分组复用 Plan 22 的 `partrender::group_message_parts`(Bucket)。
>
> **已接通端到端 + GPU 装饰(Plan 22 合并后,2026-06-29)**:
> - **registry 接缝**:`Engine` 持 `registry = default_registry()`;`store::render_part(part_id) → (PartKind, RenderPart)` 投影 reasoning/tool/compaction;`app.rs::ensure_layouts` 命中 specific → `registry.render()` 直出 StyledSpan + `flat_node_tree`(Run 叶喂 reveal 调度器,逐帧 quota 揭入);其余 kind 回退 Plan 22 `display_source` markdown。
> - **GPU 装饰(R2/R3)**:`block_decorations` 按角色发 —— tool/reasoning 卡底圆角面板(`CARD_BG`+`CARD_BORDER`)、diff 行底色带(`DIFF_ADD_BG`/`DIFF_DEL_BG`,逐行连续段);居字与其它 rect 之下。
> - **可见效果**:tool/reasoning/compaction 现以漂亮卡(`▸ bash [done]` / `💭 Thinking` / 压缩分隔线 + diff `+x -y` 摘要 + 增删行绿/红底)渲染,经既有流式 reveal + Plan 19 tier 虚拟化(registry 纯函数 → release↔rebuild 天然等价)。
> - **卡口全绿**:`scripts/verify.sh` = PASS(fmt / clippy / native-test **263** / wasm-build / tsc 全绿;tsc 需同步了 Plan 22 的 `wasm-pkg.d.ts` facade)。
>
> **未落地(后续富交互相 R5/R6)**:
> - **DiffChanges 条 widget**(5 格条,render shader)—— 当前用 `+x -y` 文本摘要 + 行底色覆盖功能;独立 widget 待做。
> - **DOM 热区**:折叠箭头 / task 跳转 / 链接 / 图片预览(R2 E2 / R5,0022/0030)。Todo/permission/question Dock 见 Plan 22 `web/src/dock.ts`。
> - **R5 file/task** 富媒体(图片纹理 0007)+ **E2E/视觉**(E1/E2/E3/V1,Playwright)。
- **与 Plan 22 并行(关键)**:Plan 22 产出 `RenderPart` 投影 + `RenderRegistry`(全兜底);**本 plan 只 `register(kind, specific)` 覆盖兜底**(`RenderFn = fn(PartKind, &RenderPart, &RenderCtx)->Vec<StyledSpan>`,纯函数 CR1/R8)。**唯一接触面 = registry 注册一行**(0006/0032/0033 数据驱动),冲突面极小。Plan 22 的 P4/P5(错误/韧性)与本 plan **同时进行不互阻**;本 plan 每做好一类就覆盖该 kind 的兜底,**未覆盖继续走兜底(丑但能看)→ UI 始终完整**(0033 §3 不变量)。
- 前置:[ADR 0033 渲染契约](../decision/0033-part-render-contract.md)(本 plan 的输入,已实现)、[opencode 渲染调研](../research/opencode-desktop-part-rendering-and-interaction.md)/[业界对照](../research/agent-ui-industry-survey.md)(设计依据)、[0031](../decision/0031-event-fsm-resilience-and-js-rust-boundary.md)(状态/边界)、[0018](../decision/0018-sdf-panel-decoration-primitive.md)(SDF 面板)、[0007](../decision/0007-rich-media-embeds.md)(嵌入)、[0006](../decision/0006-inline-tags-and-extensibility.md)(reasoning 区)、[0027](../decision/0027-code-block-viewport.md)(代码/diff 视口)、[0022](../decision/0022-dom-overlay-layer.md)(DOM Dock)、[0029](../decision/0029-session-virtualization-and-glyph-working-set.md)(tier 虚拟化)、[0030](../decision/0030-text-copy-selection-and-a11y-mirror.md)/[0032](../decision/0032-interaction-architecture-sdf-world-not-component-framework.md)(交互三层:GPU 视觉 + 命中层 + DOM 热区)。
- 目标:把 Plan 22 的「兜底标签块」逐类升级为**漂亮渲染:tool 卡 / reasoning 区 / file / diff / compaction**,**全走 tier 虚拟化、交互按 0030/0032 分工(GPU 视觉 + 命中层 + DOM 热区)**,数据模型对齐 opencode(SKIP patch/step,diff 挂 tool)。

---

## 0. 设计定调(扣调研三条)

1. **数据驱动两级分派**(照搬 opencode):`part.type` →(tool 再按 `tool` 名)→ 渲染器。core 里一张注册表,对齐你 0002/0006"注册表项不是代码分支"。
2. **SKIP_PARTS = {patch, step-start, step-finish}**:不存不渲染(对齐 opencode);diff 来自 **tool 的 `state.metadata.filediff`**,不是 patch part。usage 不显示(opencode 也不显示)。
3. **GPU 管视觉 / DOM 管交互**(0030):卡底/diff/文本 = GPU SDF 块;折叠箭头/点击/勾选 = DOM overlay 透明热区(复用 Plan 21 文本层 + embed-overlay 相机同步)。**所有 part 都是块 → 走 0029 Hot/Warm tier**。

---

## 1. 各 part 渲染映射(实现规格)

> 每个非文本 part = 一个**块**(进 PartView/BlockCache/tier)。视觉用既有图元,交互用 DOM 热区。

| part | 块结构(GPU) | 交互(DOM 热区) | 复用 |
|---|---|---|---|
| **text** | markdown→StyledSpan(已有)+ meta footer(小号 Dim:Agent·Model·时长) | 复制(Plan 21) | 已有 |
| **reasoning** | 思考区:`Reasoning`/Dim 角色 + 左条/弱底(SDF 面板) | (可选)折叠 | 0006 |
| **tool 卡(通用)** | **SDF 面板**(0018)卡底 + 圆角 + 状态徽章(pending/running/done/error 配色)+ title(spawn 微光对应 running)+ args/output 文本 | **折叠箭头热区** → 展开/收起(高度 0016 morph) | 0018 |
| tool: **bash** | 代码视口(0027)`$ cmd\n输出`;复制按钮 | 复制热区 / 滚动 | 0027 |
| tool: **edit/write/apply_patch** | 代码视口 + **增删行底色**(rect)+ **DiffChanges 条**(SDF widget:+x/-y 或 5 格条) | 折叠 / 多文件 Accordion 热区 | 0027/0018 |
| tool: **read/glob/grep/list** | **context 折叠组**:连续段合一个"Gathered context"面板 + 计数 | 展开热区 | 0018 |
| tool: **task(subagent)** | 卡 + agent 名(tone 配色)+ running spinner(shaderbox) | **点击跳子会话**热区 | 0018/0028 |
| tool: **webfetch/websearch** | 卡 + url 链接 | 链接点击热区 | — |
| tool: **error** | 红卡(SDF 面板 error 配色)+ 错误首段 | 展开看全文 | 0018 |
| **file**(user 附件) | 图片→纹理 quad(0007);文件→chip(面板+图标) | 图片点开预览(DOM) | 0007 |
| **compaction** | 分隔线(widget rule)+ 标签"上下文已压缩" | — | 0026 |
| **patch / step-*** | **不渲染**(SKIP) | — | — |
| **todowrite** | 时间线隐藏;Todo → DOM Dock | Dock(0022) | 0022 |

### 1.1 回合分组(三桶 + context 折叠)

`group_message_parts`(core 纯函数,CR1):turn 内 part →
- **辅助时间线**:reasoning / tool 卡 / 中间文本(弱化);**read/glob/grep/list 连续段折叠成一个 context 组**(降噪,采纳 opencode);
- **最终回复**:尾部 text / file(主体);
- **(turn 末)diff 摘要**:非 working 时聚合"N changed files"+ DiffChanges + 每文件折叠。

### 1.2 状态徽章(tool 卡)

读 `ToolState.status`(0031 已解码):`pending/running` → title spawn 微光 + 隐藏 args/output + 禁展开;`completed` → 显 output;`error` → 红卡。徽章配色走 `theme.rs` 令牌。

---

## 2. GPU/DOM 分工(扣 0030)

- **GPU(SDF 块,tier 虚拟化)**:卡底面板、圆角、状态色、diff 行底色、DiffChanges 条、文本、分隔线、图片纹理。
- **DOM overlay 热区(0022,仅可见块)**:折叠箭头、task 卡点击跳转、链接点击、图片预览、复制按钮、Todo Dock。复用 Plan 21 的"可见块透明 DOM 层 + 相机同步"机制,热区即透明 `<button>` 叠在卡的对应位置。
- **折叠动画**:展开/收起 = 块高度变化走 0016 morph(不跳变);折叠态只渲染 header 块。

---

## 3. 改动清单(file:符号)

**core(`crates/core`)**
- `protocol.rs`/`store.rs`:tool part 的 `state{status,input,output,title,metadata.filediff}` 结构化承载(0031 P1 已扩 Part;本plan 用其载荷)。
- `content.rs`/`nodes.rs`:`group_message_parts`(三桶 + context 折叠,纯函数);各 part → 块 + 角色(`Reasoning`/`ToolTitle`/`ToolArg`/`ToolOutput`/…StyleRole 追加,数值稳定);diff 解析(filediff → 增删行)。
- `app.rs::build_frame`:出 tool 卡块(面板 0018 + 文本 + 徽章)、reasoning 区、diff 块、context 组、compaction 线;折叠态控制(高度);**新块全走 tier**。
- `embed.rs`:file 图片纹理(0007)。
- `theme.rs`:tool 状态色 / 徽章 / diff 增删色令牌。
- 渲染分派注册表(`part.type`→`tool`)。

**render(`crates/render`)**
- `shaders/markdown/` 或 `panel.wgsl`:DiffChanges 条 widget;状态徽章(可复用 panel);diff 行底色走 rect。

**wasm/web**
- `lib.rs`:`visible_tool_hotspots()`(可见 tool 卡的折叠/点击热区屏幕盒,仿 frame_embeds)+ `toggle_tool(part_id)` / `tool_click(part_id)`。
- web:`tool-overlay.ts`(透明热区,相机同步;折叠/跳转/链接);Todo Dock(0022)。

**测试**:完整清单见 **§3.7**(含**契约一致性闸**:每个 specific 渲染器必过);每相 PR 必带对应 id 且全绿。

### 3.7 测试方案(自动化清单 + 人工 TODO)

> **关键闸:每个 specific 渲染器都必须过 [ADR 0033](../decision/0033-part-render-contract.md) 的契约一致性测试**(N3)——这是保护与 Plan 22 并行不互相破坏的验证。给 Claude Code 直接执行:§3.7.1/.2 自动化;.3 人工。

#### 3.7.1 Native(`cargo test`)

| # | 测试名 | 断言 | 相 |
|---|---|---|---|
| **N1** | `group_message_parts_buckets`(proptest) | 三桶分类 + context 折叠确定 | R4 |
| **N2** | `diff_parse_lines`(proptest) | `filediff → 增删行` 确定 = 朴素解析 | R3 |
| **N3** | **`<each>_render_conforms`** | **每个 specific 渲染器过 `crate::partrender::assert_renderfn_conforms`**(不 panic + 确定 + 内容不丢)——R1..R6 每注册一个渲染器加一条 | 全相 |
| **N4** | `<kind>_render_snapshot`(insta) | reasoning/tool/diff/compaction 渲染输出(StyledSpan/块几何)快照,随 status/折叠态确定 | 各相 |
| **N5** | `tool_card_release_rebuild_equiv`(proptest) | 屏外 tool 卡 release↔`ensure_layouts` 重建逐字节等价(0029 R8) | R2 |
| **N6** | `registry_coverage` | specific 注册后 `has_specific(kind)` 真 + 输出 ≠ 兜底(覆盖进度可测) | 各相 |

#### 3.7.2 E2E / 视觉(Playwright,定帧)

| # | 用例 | 断言 | 相 |
|---|---|---|---|
| **E1** | tool 卡渲染 | 徽章随 ToolState(running 微光/done/error 红);折叠态正确 | R2 |
| **E2** | 折叠热区点击展开 | 命中层路由点击 → 高度展开(0030/0032 GPU 视觉+DOM 热区) | R2 |
| **E3** | diff 块 | 增删底色 + DiffChanges 条可见;多文件折叠 | R3 |
| **V1** | 各 part 漂亮渲染黄金帧 | 定帧 `toHaveScreenshot`(reasoning/tool/diff/context 组) | 各相 |

#### 3.7.3 人工 TODO(需人眼/真机)

- [ ] 卡片/diff/SDF 墨团**观感**(主观;后续可接 LLM pairwise 判图)。
- [ ] 暗主题徽章/diff 配色对比。
- [ ] 子 agent 跳转/缩放手感、折叠点击命中体感(真设备)。

---

## 4. 分相落地(每相一个可见纵切;PR)

> **每相每注册一个 specific 渲染器,必加一条 N3 契约一致性测试**(`assert_renderfn_conforms`)。

| 相 | 范围 | 可见产出 | 验收(必过 id) |
|---|---|---|---|
| **R1 reasoning + compaction** | 思考区(0006)+ 分隔线(0026)+ "Thinking" 合成行 | 思考过程、压缩通知可见 | **N3** · N4 · V1 |
| **R2 tool 卡(通用)** | SDF 面板卡 + 状态徽章 + title/args/output + 折叠热区 | **工具调用可见**(bash/read/…通用卡) | **N3** · N4 N5 N6 · E1 E2 |
| **R3 diff(edit/write/apply_patch)** | 代码视口 + 增删底色 + DiffChanges 条;diff 挂 tool | **代码改动 diff 可见** | **N3** · N2 · E3 |
| **R4 context 折叠 + 三桶分组** | read/glob/grep/list 折叠成 context 组;turn 末 diff 摘要 | **降噪**:一堆检索折叠成一张卡 | N1 |
| **R5 file + task 跳转** | 图片纹理(0007)+ 文件 chip;task 卡点击跳子会话 | 附件/子 agent 可见可点 | **N3** · E2(跳转热区) |
| **R6 交互 Dock** | Todo/permission/question Dock(DOM overlay,0022)+ Blocked FSM(0031) | **授权/反问/待办**可交互 | Dock 阻塞 turn 端到端 |

> 次序:**R1/R2 是地基**(最常见的 reasoning+tool 卡),**R3 是高价值**(diff),**R4 降噪**,**R5/R6 富交互**。每相端到端可跑、带测试、FAIL_TO_PASS∧PASS_TO_PASS;**N3 契约闸贯穿全相**。

---

## 5. 铁律与风险

- **CR1**:`group_message_parts`/diff 解析/分派注册表全在 core(纯逻辑、native 可测);DOM 热区/Dock 在 web。
- **R8**:part→块→几何全纯函数;折叠态/选区是 presentation 输入,不进重放内容。
- **0029 虚拟化(硬约束)**:tool 卡/diff/context 组都是块 → Hot/Warm tier,屏外释放重几何。**长会话几十个工具卡不撑爆内存**(release↔rebuild 往返等价 proptest)。
- **0030 分工**:卡视觉 GPU、折叠/点击热区 DOM。别把整卡做成 DOM(否则毁虚拟化内存目标)。
- **SKIP 对齐**:patch/step part 不渲染;diff 挂 tool;不显 token/cost(对齐 opencode)。
- **数据驱动**:加新工具渲染 = 注册表加一项,不动 build_frame 骨架(0006)。
- **风险**:tool 卡是新块类型,影响虚拟化重建源 + 折叠态持久化(by part_id,append-only)——R2 必须配 release↔rebuild proptest。

---

参考:[opencode 渲染调研](../research/opencode-desktop-part-rendering-and-interaction.md)(逐类设计 + file:行号)· [Plan 22](./plan22-opencode-events-and-fsm.md)(数据模型,本plan 修正其 §3)· [0031](../decision/0031-event-fsm-resilience-and-js-rust-boundary.md) · [0018](../decision/0018-sdf-panel-decoration-primitive.md)/[0027](../decision/0027-code-block-viewport.md)/[0007](../decision/0007-rich-media-embeds.md)/[0006](../decision/0006-inline-tags-and-extensibility.md)/[0022](../decision/0022-dom-overlay-layer.md)/[0029](../decision/0029-session-virtualization-and-glyph-working-set.md)/[0030](../decision/0030-text-copy-selection-and-a11y-mirror.md)。
