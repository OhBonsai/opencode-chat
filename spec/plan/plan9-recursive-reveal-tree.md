# Plan 9:递归揭示(节点树上的块内时序)—— 方案 A:文档序 + 容器 ordering

- 日期:2026-06-17
- 状态:**已落地(v1,9.0/9A/9B/9C/9D/9F + 9E 卡口)** —— 进度与取舍见 [plan9_progress.md](plan9_progress.md)。
- 前置:[0019](../decision/0019-reveal-gating-and-choreography.md)(揭示门 × 编排)、[0020](../decision/0020-content-node-identity-model.md)(嵌套集节点树)、[0016](../decision/0016-streaming-morph-render-model.md)(morph 淡入)、[Plan 8](plan8-reveal-cadence.md)(现揭示落地)、`crates/core/src/reveal.rs`、`crates/core/src/app.rs::schedule`
- 触发:现 `schedule()` 把整条消息排进**一个全局队列** `sort_by_key((tier, g))` + **共享配额** → 表格的 tier1/2 与列表/段落抢位、互相推迟,"整表骨架"观感上带累其他块(见对话排查)。诉求:**reveal 块内控制**——每个容器各自的时间轴,块间按**文档序**自上而下,互不拖累;并把"块内嵌套"(表格行/格、列表项/嵌套列表)纳入同一套**递归**编排。
- 设计准则:0→1,不兼容旧全局 tier 模型,一次替换到位;结构完整、实现简约。

---

## 1. 目标 / 非目标

**目标**
- 揭示改为**在 0020 嵌套集上递归**:容器按 ordering 揭示其子节点,子节点再递归;叶(Run/Glyph)逐字。
- **方案 A 语义**:每层容器**按文档序**(`range.start`)顺序揭示子项;块间严格自上而下(读序),块内各自编排。
- **块内不拖累**:释放序 = DFS 文档序,去掉跨块 tier 重排;一个块的揭示不抢、不推迟另一个块。
- 表格 3 风格收编为 **Table 容器的 ordering 预设**;列表/嵌套列表自动获得块内时序。
- **就绪门(逐容器)**:每个容器的揭示由其**内容门 + 布局门**把关(整表 = 整表闭合、行框 = 逐行到齐、文本 = 逐字);编排可在门之上**故意更慢**——`spawn = max(门满足时刻, 编排时刻)`。门是下限(没解析/没量宽画不出),编排是上限(刻意拉长)。
- **对完整块 kind 集自洽**:补全块 kind(分隔线/显示公式/HTML/Embed,§9.0)后递归仍只增查找表行;**有字块以 glyph 锚、无字块以节点锚(NodeSpawn)**,Embed 跨层到 web 淡入(§2.6)。

**非目标(留后)**
- 方案 B(各块并行独立时间轴)——本 plan 只做 A;ordering 抽象为 B 预留口子。
- 节奏美学(rap flow 微定时,research/reveal-rhythm)——正交,后接。
- 乐观预测、每用户持久化。

## 2. 核心模型:容器 ordering + 递归 resolve

把"揭示"定义为**容器对其子节点的排程**,递归到叶:

```
Ordering(每容器一个;数据,0019 §4.2 风格的泛化):
  Sequential { gap_ms }                  // 方案 A 默认:子项按文档序,逐个 + gap
  SkeletonThenChildren { frame, gap_ms } // 容器骨架(框/网格/底/左条)先现,再排子项
  // (方案 B 的 Parallel 预留,不在本 plan 落)
```

**递归 resolve(替换现 `resolve`/`resolve_doc`/表格专用 Selector):**
```
resolve_node(idx, cursor) -> cursor'         // cursor = 该容器内的 (tier, delay) 游标
  ordering = ordering_for(kind(idx))
  if SkeletonThenChildren: 记容器骨架 spawn = cursor;cursor.delay += frame.dur(骨架先行)
  for child in children(idx) by range.start:  // 文档序(嵌套集白送)
    if child 是叶(Run/Glyph): 该子区间逐字 tier=cursor.tier++/delay 递增
    else:                     cursor = resolve_node(child, cursor)  // 递归
    cursor.delay += gap_ms
  return cursor
```
- **tier = DFS 文档序 rank**(单调);`schedule` 的 `sort_by_key` 退化为**文档序**(= DFS),块间自然自上而下。
- **delay_ms** = 沿途累加(容器骨架 lead + 子项 gap),给"骨架先行 / 逐项 / 逐行"的相对时序;喂 `spawn_time`(0019 §4.3),morph(0016)按其淡入。
- **块内时间轴**:每容器的 cursor 是**局部**的;父不串子、兄弟按序——"块内控制"是结构直接推论(子树 = 连续区间,0020 §3A)。

**每 NodeKind 默认 ordering(方案 A):**

| 容器 | ordering | 效果 |
|---|---|---|
| Doc(根) | Sequential | 块与块自上而下(读序) |
| Paragraph / Heading | Sequential(叶) | 逐字 |
| List / ListItem | Sequential | 逐项(嵌套 List 递归 → 逐层逐项) |
| Quote / CodeBlock | SkeletonThenChildren | 左条/底先现 → 逐行字 |
| Table | **预设(下拉)** | 见 §3 |
| TableRow | Sequential | 行内 cell 逐个(行框/原始) |
| MathDisplay | SkeletonThenChildren | 公式框先现 → 源字(或整体 Embed 化) |
| HtmlBlock | Sequential(叶) | 逐字(或留 raw 容器) |
| ThematicBreak / Embed | **NodeSpawn(无字)** | 整块淡入,无逐字(见 §2.6) |

## 2.5 就绪门(content / layout gate)+ 合成(0019 §5 双门落地,补 Plan 8 推迟 #1)

**为什么必须有**:递归 ordering 只回答"子项以什么顺序/编排揭示",不回答"**何时允许开始**"。流式下容器内容是逐步到的——画整表骨架要先知道整张表的形状(行列全到 + 列宽量好),画行框只要那一行到齐。这就是**就绪门**,且**每种风格对门的要求不同**:

- **内容门 `content_ready(容器)`**:该容器内容成形到第几级(0017 提交前沿 + 0020 子树判定)。表格:`Row(n)`(已到 n 行)/ `Block`(已闭合)。
- **布局门 `layout_ready(容器)`**:几何稳定到第几级(列宽/行高定)。整表要全列定才能画准网格;行框只要该行量好。
- **风格声明门要求**(`Ordering` 携带,= 0019 `RevealStyle.gate`/`Dep`):
  - 整表骨架:门 = **内容 Block + 布局 Block**(整表闭合且列宽全定)→ 才放整表骨架。
  - 行框:门 = **逐行 内容 Row(i) + 布局 Row(i)**(该行到齐且量好)→ 逐行放。
  - 原始/文本:门 = **Glyph**(到一个字放一个,即时)。

**合成规则(关键)**:某容器/子项的揭示开始时刻 =
```
spawn_start = max( gate_satisfied_time,        // 内容/布局门满足(下限,内容是瓶颈时以它为准)
                   choreography_start_time )    // 父递归给的编排起点 + 本风格 offset(上限,刻意更慢)
```
- 内容快、编排慢 → 以编排为准("强制框 1s 绘制"成立)。
- 内容慢、编排快 → 以门为准(整表等到表闭合才出现)。
- `content_ready`/`layout_ready` **逐容器、单调只增**(append-only),递归求值;`schedule` 每帧用当前门值过滤"可释放"集合,再按文档序 + 配额释放。

**对流式 text 的最低绘制要求(即你的用例)**:`content_ready` 由 0017 提交前沿给——表格"闭合"= 它不再是活动块(后续块出现 / turn 结束)。整表门 = Block 故必须等闭合;行框门 = Row 故到一行即起。

## 2.6 无字块 + Embed 的揭示(块 kind 完备后的两个扩展点)

补全块 kind(§9.0)后会出现**没有 glyph 叶**的块,现有"以 glyph 为锚"的揭示对它们失效,补两条:

- **NodeSpawn(节点级 spawn)**:`resolve` 给**无字容器**(`ThematicBreak`/`Embed`/空块,以及"框先于字"里的框本身)一个**节点自身的 spawn_time**,独立于 glyph 区间。装饰/面板(0018 `FramePanel`/`FrameRect`、分隔线 Rule)的出现/淡入按它走;门取 **layout/资源就绪**(分隔线=布局定位即可;Embed=固有尺寸/资源到位)。glyph 路径不变——有字块照旧用 glyph spawn,无字块走 NodeSpawn,二者在同一 `spawn=max(门,编排)` 合成下统一。
- **Embed 跨层揭示(× 0022)**:`Embed` 的"揭示"= DOM 叠加框淡入(CSS),非 SDF。core 只产 **该 embed id 的 NodeSpawn 信号**(随 `FrameData` 出),web 层(0022 叠加层)据 id 配对、对 DOM box 施加淡入。core 不碰 DOM(CR1),接缝 = "id + spawn_time"。

> 一句话:**有字块以 glyph 锚、无字块以节点锚**,共用同一门 + 编排合成;Embed 再多一跳到 web 层做 CSS 淡入。

## 3. 表格 3 风格 = Table 容器 ordering 预设(收编 0019 §2)

| 下拉 | 门(content/layout) | Table.ordering | 观感 |
|---|---|---|---|
| 原始逐字 | Glyph | Sequential | 无骨架,逐格逐字(到字即起,纯文档序) |
| 行框 | 逐行 Row(i) | SkeletonThenChildren(每行框) | 逐行:该行到齐 → 行框先 → 该行字 |
| 整表骨架 | Block(整表闭合 + 列宽全定) | SkeletonThenChildren(整表网格) | 等整表闭合 → 网格 → 表头 → 各 cell |
- "整表"的"cell 并行"是**方案 B 在 Table 这层的局部用法**:本 plan 先用"gap 极小的 Sequential"近似(肉眼接近并行),真并行随方案 B 一起做。
- 面板(网格/框/底)接 §4:`FramePanel.reveal` 由该子树揭示进度驱动(已有字段,Plan 8 续)。

## 4. 面板/骨架接入(复用现有)

- 容器骨架(Table 网格、CodeBlock 底、Quote 左条)= `FramePanel`/`FrameRect`;其 `reveal`(纵向揭示比例,已落地)由**该容器子树**已释放字的范围驱动(`block_decorations` 现按表算,改为按容器算)。
- `SkeletonThenChildren` 的"frame 先现"= 容器骨架 spawn 早于子项 spawn(delay 差),不再无条件画满。

## 5. 调度器对接(`schedule`)

- `resolve_doc` → `resolve_tree`(递归);产 `GlyphPlan{tier(文档序), delay_ms, skeleton...}`。
- `cand.sort_by_key((tier,g))` 不变(tier 现=文档序 → 等价 DFS 序)。
- **配额**:仍是单一总速率(reveal_cps/slow,Plan 8 / transport),**按文档序消费** → 越靠前越早;表格在其文档位置消费,**不会**越到前面块去抢(根治"带累")。
- **门过滤**:每帧先按 `content_ready`/`layout_ready`(§2.5)过滤掉"门未满足"的容器子树(整表未闭合 → 整张表不进候选;行框某行未到 → 该行不进),再按文档序 + 配额释放;`spawn_time = max(门满足, 编排)`。
- 限速/放慢/`seek_reveal`/transport 播放器:**不改**(都跑在 `advance`/`schedule` 之上)。

## 6. 相位

- **9.0 NodeKind 补全(前置,使后续表一次覆盖完整 kind 集)**:`NodeKind` 增 `MathDisplay` / `ThematicBreak` / `HtmlBlock`(可选 `FootnoteDef` / `DefinitionList` / `DefinitionItem` / `TaskItem`);`content::block_node_kind` 去掉 `_ => Paragraph` 吞并,逐一映射(jcode 的 `BlockKind` 已给出这些);`Embed` 占位接口接通(0022,content 先产空 Embed / 预留)。单测:各构造解析 → 对应 NodeKind(分隔线/显示公式/HTML 不再当段落)。
- **9A 递归 resolve 地基**:`reveal.rs` 加 `Ordering`(含 `NodeSpawn` 无字分支,§2.6)+ `resolve_tree`(沿 children DFS,tier=文档序,delay 累加;有字块 glyph 锚、无字块节点锚);删全局 tier 阶梯 + 表格专用 `Selector::Header/Cell/RowGlyphs`(收编进递归)。`schedule` 改调 `resolve_tree`。单测:混排树 tier 单调=文档序;块内子树连续;无字块出 NodeSpawn。
- **9B ordering 数据 + 默认表**:每 NodeKind 默认 ordering;Table 3 预设(下拉值→Table/Row ordering)。单测:三预设的行/格/骨架时序顺序。
- **9C 骨架接入**:`block_decorations` 的 `reveal` 由"按表"泛化为"按容器子树进度";CodeBlock/Quote 骨架先行接 `SkeletonThenChildren`。
- **9D 列表/嵌套**:List/ListItem 逐项 + 嵌套 List 递归(现 `skeleton_style` 的整块一次性 → 改逐项)。单测:嵌套列表 tier 深度优先文档序。
- **9F 就绪门 + 合成(§2.5,补 Plan 8 #1)**:`content_ready`/`layout_ready` 逐容器(0017 提交前沿 + `TablePanel` 几何);`Ordering` 携门要求(整表=Block、行框=Row、文本=Glyph);`schedule` 每帧门过滤 + `spawn=max(门,编排)`。单测:整表门到 Block 才标、行框逐行标、内容瞬到时以编排为准。
- **9E 重放 / 人工验收**:见 §9(g-table / g-nest / g-mixed / g-choreo)。验"块间自上而下、块内各自编排、切表格风格只动表格、整表等闭合、行框逐行、编排压过内容、无跨块拖累"。卡口全绿。

## 7. 评审(取舍 / 风险 / 备选)

**简化**:tier 从"语义阶梯(text0/skeleton1/cell2)"改为"**文档序 rank**" —— 模型更简、更符合读序,且天然块内;编排全靠 delay(相对偏移),不靠 tier 抢序。
**风险**
- **reparse 稳定性**:递归依赖子树连续(append-only);流式 reparse 时 0020 已保前缀稳定 → OK(`check_invariants` 守)。
- **`children()` O(n) 扫**:深树 O(n·depth);可用"子树连续区间"切片优化,chat 规模先后置。
- **morph(0016)协同**:reveal 只定 spawn_time,几何/淡入仍 0016;键 `(block_seq<<32)|node_seq` 不变 → 无冲突。
- **transport/seek 协同**:`seek_reveal` 重跑 `advance` → 自动走新 `resolve_tree`,不需改。
- **"整表 cell 并行"近似**:本 plan 用小 gap 近似,真并行待方案 B。

**备选(不选)**:① 保留全局 tier 队列(现状)——跨块拖累、与"块内控制"相悖。② 仅给 `sort_by_key` 加 `block_seq`(`(block_seq,tier,g)`)——只解块间序,块内嵌套仍写死、表格/列表不统一;治标。**故选递归。**

## 8. 验收(DoD)

1. **块间**:严格文档序自上而下;靠后的块不抢到靠前块之前(根治"段落跳到列表前")。
2. **块内嵌套**:列表逐项 / 嵌套列表逐层 / 表格按预设(行框逐行、整表骨架先行)——各自时间轴。
3. **隔离**:切表格风格只改表格揭示,**不动**列表/段落/代码。
4. **无拖累**:某块揭示快慢不推迟其它块(同速率下按文档位置消费配额)。
5. **就绪门**:整表风格**等整表闭合**才揭(g-table 整表:表区空到末段到达);行框**到行即起**(逐行冒出);文本逐字。
6. **编排压过内容**:内容瞬到时(g-choreo)以编排时长为准(`spawn=max(门,编排)`)。
7. **块 kind 完备自洽**:分隔线 / 显示公式 / HTML / Embed 有独立 NodeKind;无字块走 NodeSpawn、Embed 经 web 淡入;补 kind 只增 ordering/gate 查找表行,递归管线不改(0019 "加一条,零改")。
8. 确定性可重放(注入时间);纯文本逐字不回归;限速/放慢/transport 不变;卡口全绿。

## 9. 人工验收 cases(你从 case 确认是否符合需求)

四个 case 已建(`web/public/cases/g-*.json`,已进 `?debug` 下拉)。**两种看法**:
- **内容门**(整表等闭合 / 行框逐行)= 看**实时重放**:debug 选 case + 把回放速度调慢(0.25× / 0.1×),肉眼看内容随时间到 + 揭示。
- **编排**(骨架→填、行框、逐字的动画)= 用 **transport 播放器**:拖 scrubber / 慢速播放回看(内容已加载,纯看编排)。
- 每个 case 先在**表格风格下拉**切 整表 / 行框 / 原始 各看一遍。

| case | 操作 | ✅ 符合需求的表现(实现 9 后) | ⚠️ 现状(未实现,作对照) |
|---|---|---|---|
| **g-table** | 选 case,回放 0.1×,切三种风格 | **整表**:表格区一直空,直到 ~2.4s 末段到达(表闭合)才出现网格→表头→cell。**行框**:行一/二/三在 0.9/1.4/1.9s 各自"框+字"冒出。**原始**:分隔行后逐格逐字。 | 三种都≈逐行冒出(门没接,风格不影响内容时机);整表**不**等闭合。 |
| **g-nest** | 选 case,回放 0.25× | 列表按文档序逐项;二级项跟在其一级父项之后(深度优先),块内自有节奏。 | 列表整块一次性揭(skeleton 粗粒度),无逐项。 |
| **g-mixed** | 选 case,回放 0.25×,整表风格 | 段落一→列表→表格→段落二**严格自上而下**;表格(整表)等段落二到达闭合才揭,段落二仍在其后;切风格只改表格、不动段落/列表。 | 可能出现后面块(tier0)抢到前面结构块(tier1)之前;切风格"带累"其它块。 |
| **g-choreo** | 选 case(内容瞬到),用 transport 0.25× 播放 | 内容门立即满足 → 所见即纯编排:整表=网格→表头→cell 有序淡入;**编排时长决定节奏**(内容不是瓶颈)。 | 全速一帧铺满,编排一闪而过(需 transport 才看得到,Plan 8 已支持)。 |

**判定**:逐 case 对照"✅ 列",符合即达 §8 DoD;不符合就指出哪条,迭代。这组 case 也作为 9E 的回归基线(实现前后对拍)。

---

> 一句话:**把揭示从"全局 tier 队列"改成"嵌套集上的递归排程(方案 A:文档序 + 每容器 ordering + 逐容器就绪门)"**——块内控制、块间读序、`spawn=max(门,编排)`;表格 3 风格收编为 Table 容器预设(整表=Block 门、行框=Row 门),列表/嵌套自动获得块内时序。
