# 决策记录 0019:reveal 门控 × 编排模型 —— style = (就绪门, 揭示编排) 的数据描述

- 日期:2026-06-15
- 状态:已采纳(模型定调;具体 style 表 / 缓动 / 时长留作上层 policy,见 §8)
- 前置:**[0017](0017-markdown-streaming-landing.md) §10(reveal 策略层方向,本篇是其形式化)**、[0016](0016-streaming-morph-render-model.md)(几何插值机制,本篇产出其端点)、0005(块冻结/settle)、0014(表格两趟布局,首个有"骨架先行"需求的块)、`design/thinking.md §1/§3`(按元素揭示编排 / 节奏自主北极星)
- 定位:0017 §10 钉下了"按块揭示编排 + 节奏自主"的**方向**;本篇把它**形式化为一个可被设计师改的数据模型**——把"什么时候够格渲染"和"以什么顺序/节奏揭示"从渲染机制里彻底剥出来,二者皆为可换的 policy,机制(0016)不动。

---

## 1. 触发:同一张表,设计师想要多种揭示风格

表格 streaming 入场,作者列出三种可能风格(且**会随设计师想法变**):

- **风格 1(原始)**:跟随 streaming text,解析到哪显示到哪,逐行铺 raw —— 现状,作者觉得丑。
- **风格 2(行框)**:每行的文字流完后,**先画行框,再把字填进框**。
- **风格 3(全表)**:整表流完后,**先画网格 → 再画表头 → 各 cell 并行填字**。

要害不是写三段代码,而是:**这三种风格在哪些维度上不同?如何抽象出一个 style 描述 + 渲染管线,使"加一种风格 = 加一行配置"而非改管线?** 其中每种风格对"需要看到多完整的内容才开始渲染"要求不同(风格 3 要整表、风格 2 要一行、风格 1 什么都不要)。

## 2. 工程定性:两个正交轴

三种风格 = **两个正交轴上的三个取值组合**,不是三件事:

- **轴 A — 就绪门控(gate)**:一个揭示单元要多"完整"才允许进入揭示。是对**解析状态的谓词**,数据驱动,来自 0017 的提交前沿。
- **轴 B — 揭示编排(choreography)**:门一开,元素以什么顺序、并行/串行、各带多少延迟被揭示。是**时间线 / stagger**,品味驱动。

| 风格 | 门控 A | 编排 B |
|---|---|---|
| 1 原始 | 逐 glyph(几乎无门) | glyph 解析即现,无编排 |
| 2 行框 | 整行完成 `RowComplete(n)` | 每行:框@0 → 字@框末 |
| 3 全表 | 整表完成 `TableComplete` | 网格@0 → 表头框/字 → 各 cell 并行填 |

一旦这么拆,"设计师改风格" = 改这张表的两列**数据**。这正是要找的抽象。

## 3. 业界先例(两轴各有成熟思路,弃引擎取观念)

**门控轴 ≈ React Suspense / 流式 SSR 的 reveal boundary + skeleton screen**:声明一个边界 + 骨架 fallback,数据"就绪"时把骨架换成内容;边界粒度 = 门控粒度。风格 2/3 的"先框后字"就是 skeleton-first。关键性质:数据源 **append-only ⇒ 门控单调只增(high-water mark)**,永不回退,可驱动只前进的状态机——与 0017 提交前沿同源([React Suspense](https://react.dev/reference/react/Suspense))。

**编排轴 ≈ 动画编排系统**:Framer Motion 的 `variants` + `staggerChildren`/`delayChildren`、GSAP timeline、After Effects/Lottie、Core Animation `CAAnimationGroup`、Unreal Sequencer / Unity Timeline;以及 Vue `<TransitionGroup>` 的 enter/leave/move(FLIP,thinking §2 已引)。共同点:**把"什么动"与"何时、按什么顺序动"分成数据**,本质是 **trigger(何时)与 transition(如何)分离**([Framer Motion orchestration](https://www.framer.com/motion/transition/))。

**结论**:本需求 = **Suspense 门控 + Framer-Motion variants stagger 编排**,底下接 0016 几何插值。两轴都弃引擎、取观念——单调门 + 数据化时间线,皆轻。

## 4. 四层模型(落在项目现有分层上)

```
① gate    就绪谓词    parse_state → Granularity         (core 解析侧;0017 提交前沿的细化)
② plan    揭示计划    style = 一组 Stage(纯数据)        (设计师只动这里)
③ sched   调度器      读 gate → 激活 Stage → 产 (past,current,alpha) 端点   (core 编排)
④ morph   机制        f(past, current, t) → 插值几何     (0016,完全不知 gate/plan)
```

> **节点身份依赖**:plan/stage 的 `Selector`(选 frame/grid/header/cell/row/run)需要**内容节点身份**(不是 glyph 下标)。该身份模型见 **[0020](0020-content-node-identity-model.md)**(嵌套区间 + parent 下标 + 路径哈希);本篇的 `Selector` = 在 0020 节点表上按 kind/range 查询。

### 4.1 ① gate —— 就绪粒度 `RevealUnit`(统一,不止表格)

门控对**任意块**复用一组单调递进的粒度:

```rust
/// 揭示就绪粒度:某块结构上已完成到哪一级。单调只增(append-only)。
pub enum RevealUnit {
    Glyph,          // 逐字(风格 1)
    Line,           // 整行/整逻辑行
    Row(u32),       // 表格第 n 行完成
    Block,          // 整块完成(整表/整代码块/整列表)
    Subtree,        // 含子结构整体完成(嵌套列表根)
}

/// 门控谓词:纯函数,对当前活动块解析状态算出已达到的最高就绪级。
fn gate(parse_state: &ActiveBlock) -> RevealUnit;
```

对表格:`Glyph`(风格 1)/ `Row(n)`(风格 2)/ `Block`(风格 3)。`content.rs::is_pending_table` 是它的雏形(只回答"是否成形")——本模型把它泛化成"成形到第几级"。门控在 core,保 CR1。

### 4.2 ② plan —— style = 一组 Stage(纯数据)

一个 style 完全由数据描述,不含代码:

```rust
pub struct RevealStyle {
    pub gate: RevealUnit,          // 揭示单元的就绪阈值(内容门)
    pub stages: Vec<Stage>,
}

pub struct Stage {
    pub select: Selector,          // 选谁:Frame | Grid | Header(字) | Cell(r,c) | RowGlyphs(n) ...
    pub after: Dep,                // 依赖:GateLevel(RevealUnit) | Stage(StageId).End | Now
    pub offset_ms: f32,            // 相对依赖满足点的延迟(stagger)
    pub dur_ms: f32,               // 该 stage 揭示时长
    pub ease: EaseId,              // 缓动(policy 引用)
    // 产出:被选中元素的 past/current 端点 + alpha,喂 ③→0016
}
```

三种风格 = 三条数据(示意):

```
风格1 = { gate: Glyph, stages: [ {RowGlyphs(*), after: GateLevel(Glyph), offset:0 } ] }
风格2 = { gate: Row, stages: [ {Frame(row n), after: GateLevel(Row(n)),        offset:0 },
                               {RowGlyphs(n), after: Stage(frame_n).End,        offset:0 } ] }
风格3 = { gate: Block, stages:[ {Grid,        after: GateLevel(Block),         offset:0 },
                               {Header,       after: Stage(grid).End,          offset:60 },
                               {Cell(*,*),    after: Stage(header).End,         offset:0, /*并行*/ } ] }
```

加风格 = 加一条 `RevealStyle`,管线零改动。

### 4.3 ③ sched —— 调度器(只前进的编排)

每 tick:读 `gate(active_block)` 得当前就绪级;激活所有依赖已满足(gate 达标或前置 stage 已结束)的 stage;为其选中元素产 `(NodeId, past, current, alpha)` 端点喂 `Scene::commit`(0016 §4.4)。因 gate 单调,stage 激活也单调——无回滚、无撤销。调度器在 core(0016 §7 契约的生产者),是 0017 §10 待做的 "reveal 调度器" 的本体。

### 4.4 ④ morph —— 机制(0016,不变)

stage 产出的端点照旧交 0016 补间;0016 不知 gate、不知 plan、不知节奏。本模型**完全不动 0016**。

## 5. 会咬人的子问题:布局就绪 ≠ 内容就绪(必须双门)

风格 2「先画行框再填字」有隐藏依赖:**框的几何(列宽)依赖整表内容**(0014 B 像素两趟,列宽全表共享)。单看一行算不出最终列宽。两条出路:

- (a) 行框先用**临时列宽**画,等整表完成列宽变化交 0016 morph 平滑收口(不 snap);
- (b) 承认**"布局就绪"门 ≠ "内容就绪"门**:文字揭示走行级内容门,列宽几何走表级布局门。

故模型采 **双门**:`content_gate`(内容完整度,驱动揭哪些字)与 `layout_gate`(几何稳定度,驱动框/网格能否精确画)。`Stage.after` 可引用任一门。这样风格 2 不是 hack,而是"文字门=行、几何门=表"的自然表达;风格 3 两门都要 `Block`。

```rust
pub enum Dep {
    ContentGate(RevealUnit),   // 内容完整到某级
    LayoutGate(RevealUnit),    // 几何稳定到某级(列宽定 = 可精确画框)
    Stage(StageId, StageEdge), // 兄弟 stage 起/止
    Now,
}
```

## 6. 与现有 ADR 的接合

- **0017**:§2 提交前沿(committed/active)= 门控的输入源,不变;§3 保守预测**继续收窄**——结构块由本模型的 gate+plan 接管(不再原样逐帧揭 raw);§4 cadence **被 plan 的 `offset/dur` 泛化吸收**(插值延迟、缓冲到结构成形 = 特例)。本篇是 0017 §10 的形式化,§10 的 [ ] 落地项迁移到本篇 §9。
- **0016**:零改动。plan 产端点、sched 喂 commit、morph 插值,职责链清晰。
- **0014**:B 的 `TableRegion` sidecar 提供 gate/plan 所需结构(几列、表头在哪、各 cell run 区间);B 必须能布局**半截表**(只表头/部分空格)——本就是增量重排常态,亦是 `layout_gate` 的实现支点。
- **0018**:风格 2/3 的"框/网格/AO"由 SDF 面板图元画;Stage 选中 `Frame/Grid` 时产出的就是面板图元实例的 (past,current,alpha)。两篇正交:0019 管**何时/按何序揭示**框,0018 管**框长什么样**。

## 7. 设计目标与边界

1. **风格 = 数据**:加/换风格 = 改 `RevealStyle` 表,不碰 sched/morph/layout。
2. **门控单调**:append-only ⇒ gate 只增,sched 只前进,无回滚(与 0017 §6 同源)。
3. **双门**:内容门与布局门分离(§5),消解"行框需全表列宽"这类依赖错配。
4. **机制不侵入**:0016/0014 不因风格增减而改;CR1(core 无平台依赖)保持。
5. **退化**:风格 1 = gate:Glyph + 单 stage,等价 0017 现状的逐字揭示,零额外成本。

## 8. 决策与不决定的

**采纳**:reveal = **四层(gate / plan / sched / morph)**;一个 **style = `RevealStyle{gate, stages:[Stage]}` 纯数据**;粒度统一为 `RevealUnit`;**双门**(content/layout);调度器单调激活、产 0016 端点。三种风格 = 配置表三行。

**理由**:把"何时够格揭示"(Suspense 式单调门)与"按何序/节奏揭示"(stagger 式数据时间线)从机制剥离,二者皆 policy。设计师改观感只动数据;append-only 让门单调、调度器无需回滚;双门消解布局/内容依赖错配;0016/0014/0018 各司其职不被风格污染。

**不决定的(留 policy / 后续)**:具体 `RevealStyle` 表内容(各块默认风格、`ease/dur/offset` 数值,见 [TODO V]);全局/每类块配速("故意放慢");风格的运行时切换粒度;非表格块(列表/代码/公式/图片)的 gate 细化与默认 plan(§9 列为后续);调试器"揭示速度"UI(并入重放面板)。

## 9. 落地清单(Plan 5 后续;承接 0017 §10)

- [ ] `RevealUnit` + `gate(active_block)` 谓词(core;表格先行:Glyph/Row/Block),泛化 `is_pending_table`。
- [ ] `layout_gate`:0014 B 暴露"列宽已定/半截表"就绪级。
- [ ] `RevealStyle`/`Stage`/`Dep` 数据结构 + 三种内置表格风格(1/2/3)作示例。
- [ ] reveal 调度器:读双门 → 单调激活 stage → 产 (NodeId,past,current,alpha) 喂 `Scene::commit`;与 token 解耦的揭示时钟(限速/缓冲)。
- [ ] 骨架先行(风格 2/3):Frame/Grid stage 经 0018 面板图元入场,字 stage 随后。
- [ ] 非表格块 gate + 默认 plan(列表/围栏/公式/图片/链接的 raw 抑制 → 骨架先行)。
- [ ] 重放 case:同一表格 × 三风格切换(并入调试面板"风格"下拉),观感对拍。
- [ ] 调试器"揭示速度/风格"配置(并入重放面板)。

---

参考先例:[React Suspense](https://react.dev/reference/react/Suspense) / 流式 SSR reveal boundary + skeleton screen(单调就绪门)· [Framer Motion orchestration](https://www.framer.com/motion/transition/)(`staggerChildren`/`delayChildren` = 数据化时间线)· GSAP timeline / Lottie / Core Animation group / Unreal Sequencer(what 动 vs when 动分离)· Vue `<TransitionGroup>` FLIP(enter/leave/move)。方向源 [0017 §10](0017-markdown-streaming-landing.md);几何机制 [0016](0016-streaming-morph-render-model.md);框/网格外观 [0018](0018-sdf-panel-decoration-primitive.md);设计取向 `design/thinking.md §1/§3`。
