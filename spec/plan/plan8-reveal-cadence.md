# Plan 8(reveal 节奏自主:0019 落地 —— gate × choreography)

- 状态(2026-06-17):**已落地(v1 简约子集,8A–8E 全过卡口)** —— 进度与取舍见 [plan8_progress.md](plan8_progress.md)。落地 **[0019](../decision/0019-reveal-gating-and-choreography.md)(reveal 门控 × 编排)**,实现作者**极端想要**的北极星:**阅读体验 > 实时性**——结构块绝不闪 raw、**骨架先行(框→填字)**、揭示节奏与 token 解耦(可限速 / 刻意放慢)。建在 **[Plan 7 / 0020 节点树](plan7-content-node-tree.md)**(已落地)之上:Selector 直接寻址节点(表/行/cell/列表项/run)。
- 日期:2026-06-16
- 前置(均已落地):**[0020](../decision/0020-content-node-identity-model.md) 节点树**(Selector 寻址)、[0016](../decision/0016-streaming-morph-render-model.md) `Scene`/`PanelScene`(几何过渡)、[0017](../decision/0017-markdown-streaming-landing.md) 提交前沿 + `is_pending_table`(raw 抑制雏形)、0005 块冻结、smoother(吐字到达)。
- 相位:**8A 就绪门 → 8B 揭示风格(数据)→ 8C 调度器 → 8D 骨架先行 + 全结构块 raw 抑制 → 8E 重放验证**;一相位 ≈ 一/数 PR,末过卡口。
- 准则(记忆 `design-0to1-no-backcompat`):**0→1 不留并行旧路**——调度器成为**唯一**揭示路径,**收编**现"grapheme 到达即 `spawn_time=now`"的即时揭示;终态结构完整、实现取简约子集。

## 0. 定位 / 北极星

token 只更新**内容真值**(谁到了);**何时、以多快、按什么顺序揭示**由调度器决定(可缓冲/排队/限速),不再 1:1 跟 SSE。结构块缓冲到"结构可判定"→ 先入场**骨架/容器**(表头框)→ 再**填内容**;节奏可**刻意放慢**让揭示被看见。三层正交协同(0017 §10):**① reveal 策略(本 plan)→ ② 布局(layout/表格两趟)→ ③ 0016 morph(过渡)**。

**In**:`RevealUnit` 就绪门(双门:内容/布局)、`RevealStyle` 数据(gate + stages,Selector over 0020 节点树)、reveal 调度器(节奏解耦 + 限速 + 放慢)、骨架先行、全结构块 raw 抑制、3 表格风格、重放/验证。
**Out**(后续):每类块的完整 style 数据库(v1 给少量默认 + 表格 3 风格)、乐观预测(0017 §3 默认不做)、每用户级偏好持久化、0016 exit 淡出(留尾)。

**铁律**:content→layout→render 契约不破 / AR10 / core 确定性可重放(调度器时间走注入 `dt_ms`,可重放)/ 揭示策略在 core(平台无关)/ 0016 机制不动(只喂端点)/ **opinionated 简约**。

---

## Phase 8A — 就绪门 `RevealUnit`(0019 §4.1,双门)

> 域:`core`。判"活动块成形到哪一级",驱动调度器何时可揭示。

**任务**
- [ ] `RevealUnit { Glyph, Line, Row, Block }`;`content_gate(active_block) -> RevealUnit`:泛化 `is_pending_table`——段落逐字、表格到行/整表、列表到项、围栏到闭合、公式/图片到闭合。
- [ ] `layout_gate`:布局就绪级(列宽定 = 可精确画框;0014 B 暴露)。**双门**:内容门驱动揭哪些字,布局门驱动框/网格能否精确画(0019 §5,化解"行框需全表列宽")。
- [ ] 单调:append-only ⇒ 门只增(0017 §6),调度器只前进、无回滚。
- 卡口:gate 单测(表格未到分隔行=未就绪;围栏闭合=Block;列表项闭合=Row;双门各自级别)。

## Phase 8B — 揭示风格 `RevealStyle`(0019 §4.2,数据驱动)

> 域:`core`。一个 style = 纯数据;Selector 在 0020 节点树上按 kind/range 查询。

**任务**
- [ ] 数据结构:
  ```
  RevealStyle { gate: RevealUnit, stages: Vec<Stage> }
  Stage { select: Selector, after: Dep, offset_ms, dur_ms, ease }
  Selector = ByKind(NodeKind) | Frame | Grid | Header | Cell(r,c) | RowGlyphs(n) | Glyphs
  Dep = ContentGate(RevealUnit) | LayoutGate(RevealUnit) | Stage(id, edge) | Now
  ```
  `Selector` 经 0020 `nodes_of_kind`/区间解析为节点集 → 端点(0019 §4.2)。
- [ ] **内置默认风格**(v1 简约,硬编码 + 可切):
  - 纯文本/行内:逐字 fade(实时感,速度可调)。
  - 结构块(列表/引用/代码/公式/图片):**骨架先行**(容器框 → 内容),不闪 raw。
  - 表格 **3 风格**(0019 §2 配置表三行):原始(逐行 raw 跟随)/ 行框(每行框→填字)/ 全表(整表→网格→表头→各 cell 并行)。
- 卡口:风格→stage 解析单测(Selector 命中节点、Dep 依赖正确);3 表格风格各产正确 stage 序。

## Phase 8C — 揭示调度器(0019 §4.3 + 北极星)

> 域:`core`。读门 → 单调激活依赖满足的 stage → 产 `spawn_time`(0016 §9 通道)/ 几何端点;**节奏与 token 解耦**。

**任务**
- [ ] **收编即时揭示**:删现"`spawn_time = revealed[j].1 / now`"即时路径;改由调度器按 stage 给每个节点/字 `spawn_time`(唯一揭示源,不并行)。
- [ ] **揭示时钟**:全局 `reveal_cps`(可设很慢)+ "放慢"开关;调度器以注入 `dt_ms` 推进(可重放),限速/排队释放,而非跟 smoother 的 token 率。
- [ ] **smoother 分工**:smoother 管"grapheme 到达 = 内容真值";调度器管"何时上屏 = 呈现"。二者串联,不重叠。
- [ ] 与 0016 对齐:stage 产的 enter/update 端点交 `Scene`/`PanelScene`(同 dur/ease);调度器只定"何时"。
- 卡口:调度器单测(限速:N ms 内揭示数受 reveal_cps 约束;放慢可见;时间注入可重放确定性);骨架 stage 在字 stage 之前。

## Phase 8D — 骨架先行 + 全结构块 raw 抑制(0019 §4.2 落地)

> 域:`core`(+ 复用 0018 面板 / 0016)。把策略落到具体块。

**任务**
- [ ] **骨架先行**:结构块 `layout_gate` 满足 → 先入场**容器面板**(0018 `FramePanel` / `PanelScene`,alpha fade)→ 字 stage 延后 `offset_ms`(表格 = 框/网格先,cell 字后;代码块 = 底先,字后;引用 = 左条先)。
- [x] **行内链接/图片 raw 抑制**(评审 #3,已落地):`content::strip_forming_link` —— 尾部 `](` 未闭合(链接/图片正在键入)则从开启的 `[`/`![` 处**只裁尾**,`)` 到达后整条(纯文本)再现,无 `[文字](ur` 闪。前文照常上屏(不像块级 `is_pending` 那样误藏已显文本)。单测 `forming_link_tail_suppressed`。
- [ ] **块级 raw 抑制**:`is_pending_table` 泛化到 列表 / 围栏 / 公式(`is_pending_structure` 已含表格 + 显示公式;列表/围栏暂保守不抑制)——成形中 hold,结构确认再揭示(0017 §10 收窄 §3)。
- [ ] 纯文本/行内不抑制(逐字,可调速)。
- 卡口:重放 `n-all`/`c06-all`:无 raw 闪;表格框先于字;代码底先于码;慢放可见骨架。

## Phase 8E — 重放验证 + 调试

> 域:`web`(重放 harness 已就位,5D)。

**任务**
- [ ] 调试面板加 **"reveal 风格" + "reveal 速度(可放慢)"** 下拉(并入 5D case/speed 旁)。
- [ ] 表格 3 风格切换重放对比;`n-all` 各结构骨架先行可视。
- [ ] `?verify` 黄金样张(并 [TODO V]):同 case × 不同速度/风格的揭示对拍。
- 卡口:全 case 无 raw 闪、无跳变(几何走 0016)、卡口全绿。

---

## 评审(取舍 / 风险 / 备选)

**为什么现在能做干净**:0020 节点树已落地 → Selector 直接寻址"表/行/cell/列表项/run",不必像早期设想那样按块类型写死专用选择子(0019 §4 通用形态可直接落)。0016/`PanelScene` 已能补间几何,调度器只管"何时",职责清。

**简约取舍(v1 砍掉,留后续)**:
- 完整每类块 style 数据库 → v1 少量内置默认 + 表格 3 风格;后续抽数据表/可配。
- 乐观预测(闭合前猜样式)→ 不做(0017 §3:保守预测 + 0016 补间已平滑)。
- 用户偏好持久化 / 每类块独立配速 → v1 全局 reveal_cps;后续按类。
- 0016 exit 淡出 → 留尾(同 0016)。

**风险**:
- **调度器 × smoother 语义重叠**:必须明确 smoother=到达、调度器=呈现,二者串联(8C 设计点);否则双重节奏打架。
- **raw 抑制误伤**:正文以 `|`/`-`/`>` 起头被误判结构 → 沿用"需结构确认信号"(is_pending_table 已有此控制)再揭示。
- **确定性可重放**:调度器时间必须走注入 `dt_ms`(R8/R9),否则重放/截图回归失效。
- **收编即时揭示的回归**:删旧 spawn 路径后,纯文本逐字仍要顺滑 → 8C 默认风格 = 逐字,等价现状(速度可调)。

**备选(不选)**:① 保留即时揭示 + 旁加调度器(并行双路)—— 违 0→1 准则、双节奏难调。② 把 reveal 塞进 0016 机制层 —— 机制应与内容无关(0016 §2),reveal 是上层 policy,分层更清。**故选"调度器为唯一揭示路径,喂 0016 端点"。**

**v1 落地后评审 3 项的处置**(2026-06-17):
- **#3 行内链接/图片 raw 抑制 —— 已做**(`strip_forming_link`,见 8D)。最小、真实修闪、纯 `core` 可原生测;关键是**裁尾而非块级 hold**,避免误藏已显前文。
- **#1 把 `content_gate`/`layout_gate`/`Dep` 接进 `schedule()` —— 暂不做**。当前**逐字增量重解析**已让"行随吐字逐行"等行为天然成立(每 tick 新 `TableRow` 节点出现→`resolve` 标记→释放),把门接进热路径的**行为增益有限**,却有回归顺滑默认路径的实风险,且本地无 cargo 难即时验证。`content_gate`/`Dep` 暂作**形式谓词 + 单测**保留;待要做"RowFrame 严格化/骨架强时序"时再接,并保留已冻结块快路径。
- **#2 面板帧淡入(pop→fade)—— 暂不做**。跨 crate 改 wgsl、ROI 最低,且无浏览器无法验证观感;并入"0016 exit/enter 淡出留尾"。

**验收(DoD)**:
1. 结构块 streaming 中**绝不闪 raw 源**(列表/围栏/公式/表格…)。
2. **骨架先行**:容器框先于内容字出现;表格 3 风格可切。
3. 揭示节奏**与 token 解耦**:可全局放慢、肉眼看见揭示过程。
4. 全程**无跳变**(几何走 0016/`PanelScene`);确定性可重放(注入时间)。
5. 纯文本逐字不回归(默认风格等价现状,速度可调);卡口全绿。

## 测试 / 验收(整套)

- **A 单元测试(`cargo test -p infinite-chat-core`)**:① `content_gate`/`layout_gate` 单调 + 各级别;② `RevealStyle`→stage 解析(Selector 命中 0020 节点、Dep 依赖、3 表格风格 stage 序);③ 调度器(限速/放慢/注入时间确定性、骨架 stage 早于字 stage);④ "门未达 → 该结构块无 raw 字揭示"。
- **B 渲染数据回归(native `CollectSink`,不需 GPU)**:断言 `build_frame` 的 `FrameGlyph.spawn_time` 顺序 = 调度器预期(骨架/字先后);结构块在 gate 前**不出 raw glyph**。
- **C `?replay` 可视(浏览器)**:`n-all`/`c06-all` × {3 表格风格} × {慢放 0.1×},肉眼验:无 raw 闪、框先字后、节奏可控。配 `?debug` 节点框看 Selector 选中。
- **卡口**:`fmt`/`clippy -D warnings`/`cargo test` 全绿;`wasm-pack build` + `tsc`。

## 依赖 / 与 ADR 对应

| 相位 | 兑现(0019) | 依赖(已落地) |
|---|---|---|
| 8A | §4.1 gate + §5 双门 | 0017 提交前沿 / is_pending_table / 0014 B |
| 8B | §4.2 RevealStyle/Selector | **0020 节点树** |
| 8C | §4.3 调度器 + 北极星(thinking §3)| 0016 spawn_time / smoother / 注入时钟 |
| 8D | §4.2 骨架先行 + 0017 §10 raw 抑制 | 0018 `FramePanel`/`PanelScene`、0016 |
| 8E | 5D 重放 + TODO V | 调试面板 |

> 后续(本 plan 外):每类块 style 数据库 / 可配、每类块配速、乐观预测、0016 exit 淡出、用户偏好持久化。北极星设计源:`design/thinking.md §1/§3`、记忆 `md-reveal-cadence-north-star`。
