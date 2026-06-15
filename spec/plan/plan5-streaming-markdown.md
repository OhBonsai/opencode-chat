# Plan 5(streaming markdown 还原:0016 机制 + 0017 落地 + 重放验证)

- **状态(2026-06-15)**:**已落地**(v1)。5A 机制(retained morph Scene)+ 集成、5B/5C 驱动与语法(复用块冻结 + 逐帧重解析,机制 join 兜底)、5D 重放 harness + c01–c10 case 集均就位。留尾(policy/升级):GPU 双态(路 A)、exit 淡出、块内分块冻结、行边界 cadence+插值延迟、缓动/时长 policy 大表、截图回归(5D4)。详见各相位末与 [0016 §9]/[0017 §9]。
- 日期:2026-06-15
- 范围:让**所有 markdown 语法的渲染都满足 streaming 形变规则**(不跳变,位移/缩放走补间),并用 **web 重放 SSE 的 case 集**验证。机制见 [0016](../decision/0016-streaming-morph-render-model.md),markdown 落地见 [0017](../decision/0017-markdown-streaming-landing.md)。
- 前置:0016 / 0017 / 0005(块冻结·settle)/ 0010(pulldown-cmark)/ 0011(quad·SDF 图元)/ 0014(表格,首个消费者);Plan 4(排版/装饰/调试器)已落地。
- 相位:**5A 机制层 → 5B 驱动层 → 5C 全语法 streaming 规则(随 5B)→ 5D 重放验证**;一相位 ≈ 一/数 PR,末尾过卡口。

## 0. 定位

Plan 4 把"能渲染、观感对、可调试"收口。**Plan 5 = 让 streaming 过程本身不跳变**:LLM 逐字到达、后续 token 改变已渲染内容几何(表格列变宽、`**` 闭合加粗、setext 回溯升级)时,**全部以 translate/scale 补间过渡**,并以可重复的重放 case 验证每种 markdown 语法都守这条规则。

**In**:0016 机制(双关键帧 retained scene)、0017 驱动(提交前沿/保守预测/行边界 tick)、全 markdown 构造的 streaming 行为、重放 harness + case 集、lygia shader 复用约定。
**Out(→ 后续/TODO2)**:具体动画美学调参(缓动/时长大表,policy 层)、乐观预测、math 行内盒(0013)、宽表局部滚动(0014 分叉)、编辑/乱序内容 diff 匹配。

**铁律**:content→layout→render 契约不动(0001 §2.2)/ AR10 每帧一次跨界 / 小包体 / **opinionated 单实现**(中英文 + markdown,[TODO 锚点](../../TODO.md))。

---

## 约定:shader 复用 lygia —— **许可感知,分两类**

仓库已放 `./lygia`(WGSL 版),作 **参考库**。WGSL 无 `#include`,任何复用只能**代码复制**。

**⚠ 许可前提**:lygia 是 **[Prosperity Public License 3.0](../../lygia/LICENSE.md)——非商用**(商用仅 30 天试用)。本项目目标是可嵌入/可分发的 npm 组件,**把 lygia 代码复制进发行产物 = 商用场景踩线**。故:

- **要发行的 shader 代码:不直接抄 lygia**。我们真正需要的几个函数(缓动曲线、`aastep`、圆角矩形 SDF)都是教科书级短函数,**自己实现几行**即可,反而避免许可负担——抄 lygia 收益极小。
  - 0016 tween 缓动 → 自写 `cubicOut/sineInOut`(标准公式,几行);可**参照** `lygia/animation/easing/*` 但不复制。
  - rect/glyph 抗锯齿 → 现有 `fwidth+smoothstep` 已够(对照 `lygia/math/aastep.wgsl` 思路);圆角矩形现 `sd_round_box` 已自写。
- **仅 lygia 提供、自写成本高的复杂件**(如 JFA `morphological/jumpFlood.wgsl` 生 SDF):**先标记为"参考/原型期非商用"**,真要进发行版前评估——重写、换 MIT/BSD 等价实现、或确认许可。**不默认抄进产物。**
- 任何确实复制的片段:文件头注明出处 + Prosperity license 行;整库不入构建产物(已 gitignore 评估)。

> 即:lygia 当**读参考、学算法**,发行 shader 以**自有实现**为主——既守小包体,也避开非商用许可。

---

## Phase 5A — 机制层(0016:双关键帧 retained scene)

> 域:`render`(scene/backend/shader)。落地 0016 的 `Scene` + `RenderNode` + join + 插值,**与内容无关**。

**任务**
- 5A1 **数据结构**:`Geom{pos,size,alpha}` / `RenderNode{载荷,current,past:Option,t_start,phase}` / `Scene{HashMap<NodeId,RenderNode>,dur_ms}`(0016 §4)。
- 5A2 **join**:`Scene::commit(layout, now)` enter/update/exit,**past 取当前显示态**(打断不回跳,0016 §4.4)。
- 5A3 **插值 + 发射**:`instances(now)` 走 `f(past,current,t)`;**路 B(CPU mix)** 先行,上传单态(不改实例结构);`past=None` 静止旁路零成本(0016 §4.5/§5)。
- 5A4 **缓动从 lygia**:CPU 侧移植 `lygia/animation/easing` 的 cubicOut/sineInOut(Rust 端口或拷 wgsl 待路 A);`dur_ms`/`ease` 作 policy 参数(默认值占位,0016 §8)。
- 5A5 **身份**:layout 输出每字块带 `glyph_idx`,app 赋 `block_seq` → `NodeId`(0016 §4.6,契约最小扩展)。
- 5A6 **块冻结衔接**:settle(节点全 `past=None`)后再冻结;冻结块不进 `Scene`(0016 §6)。

**DoD**:同一 `NodeId` 两次 commit 几何不同 → 显示端补间无 snap;过渡中再 commit 不回跳;exit 节点 t=1 清除;冻结/静止块零保留态、零额外开销;`?debug` 帧统计无回归。

---

## Phase 5B — 驱动层(0017:提交前沿 + 保守预测)

> 域:`core`(content/app:活动块、tick、重解析)。满足 0016 §7 上游契约。

**任务**
- 5B1 **活动块界定**:活动区 = 最后一个未闭合块(= 0005 未冻结块);前缀块已提交、冻结(0017 §2)。
- 5B2 **行边界 tick**:活动块每完成一行 → 重解析活动块(pulldown **原样** = 保守预测)→ 重排 → `Scene::commit`(0017 §3/§4)。
- 5B3 **提交前沿含 lookahead**:setext underline / 表头分隔行 / list 松紧 触发活动块整块重解析(0017 §2)。
- 5B4 **插值延迟(可选)**:渲染滞后一行,保证补间跨两个完整关键帧(0017 §4)。
- 5B5 **引用链接逃逸口**:`[ref]: url` 跨块解析 → 一次性非动画 restyle(0017 §7)。

**DoD**:活动块逐行重排经 5A 平滑过渡;闭合 → settle → 冻结;前缀块不再重解析;每 tick 重解析 O(块大小),长会话无累积开销。

---

## Phase 5C — 全 markdown 语法的 streaming 规则

> 域:`content`/layout。**逐构造**确保其流式变化映射到 0016 的 enter/update/exit,无跳变。这是"对所有 markdown 语法满足 streaming 规则"的落点。

**构造 × streaming 行为(规格表)**

| 构造 | streaming 中的变化 | 映射(0016) | 备注 |
|---|---|---|---|
| 纯文本 | 逐字追加 | enter(fade/scale in) | 基础 |
| `**粗** / *斜*` | 闭合瞬间字面→样式,字宽变 | size delta → update | 依赖 per-role 度量(4A4) |
| `` `行内码` `` | 闭合→等宽 + chip 底 | update + rect enter | chip = FrameRect |
| `[链接](url)` | 闭合→链接色;引用式跨块 | update / 一次性 restyle | 逃逸口 0017 §7 |
| 段落 | 追加 + 折行;长高把下方块下移 | enter + 下方块 translate | 跨块下移必须补间 |
| 标题 `#..` | 行首 `#` 数确定级别 | enter(定级后) | |
| 标题 setext | 下一行 `===/---` 回溯升级 | 段整体 update(字号 scale) | lookahead 重解析 |
| 列表 | 逐项追加;松/紧由空行定 | item enter;紧→松 = 行距 update | |
| 引用块 `>` | 追加;左条随高增长 | enter + 左条 rect scale | |
| 代码围栏 ``` | open 前当段落,close→代码块 | 批量 update + 底 rect enter | 块归类 |
| 表格 | 逐行;新行撑宽列 → 右侧列/旧行右移 | pos delta → update(单调增宽) | 核心,见 0014 |
| `---` hr | 闭合成线 | rect enter | |
| GFM Alert | `[!NOTE]` 识别→类型色条+底 | update + rect enter | |

**DoD**:上表每行都有对应 case(5D)且无跳变;字重/等宽闭合的 size delta 正确(不压扁);表格列单调增宽、旧行平滑右移;跨块下移平滑。

---

## Phase 5D — 重放验证 harness + case 集

> 域:web(harness)+ fixtures。**复用现成 `Player`/`Record{t,raw}`**(synthetic 已用),不新增连接类型(勿增实体)。模拟 SSE、**不连 opencode**。

**机制**
- 事件格式 = opencode 信封(实测):`{"type":"message.part.delta","properties":{"sessionID","messageID","partID","field":"text","delta":"…"}}`。
- **fixture** = 一段话切成带时间戳的 text delta 序列(TS 编写,`{t, delta}` 由 harness 包成上面的信封 → `Record{t,raw}`)。
- main.ts 加 `?replay=<case>`:不传 serverUrl,改 fetch `web/public/cases/<case>.json` → 构 `Player` 喂 `ChatCanvas`(替代 `synthetic()`)。可选 `?speed=`、`?step` 单事件步进。

**任务**
- 5D1 **harness**:`web/src/replay.ts` 载 case → 包信封 → 经 `ChatCanvas` replay 配置喂 `Player`;支持速率/步进。
- 5D2 **case 集**:`web/public/cases/*.json`,逐条覆盖 5C 规格表(见下)。
- 5D3 **`?verify` 标尺**(并 [TODO V]):叠 baseline/行盒/字盒自绘几何(复用 4C3),肉眼/截图比对。
- 5D4 **截图快照回归**:每 case 存参考图,改动后 diff(只守"这一种观感"不回退,opinionated)。

**case 集(初版)**
- `c01-plaintext` 逐字追加(enter 基础)
- `c02-bold-close` `**…**` 闭合重着色(update size)
- `c03-inline-code` 行内码 + chip
- `c04-list` 列表逐项 + 紧→松
- `c05-fence` 代码围栏 open→close(块归类 + 底色)
- `c06-table` 表格逐行列变宽(**核心**,旧行右移)
- `c07-setext` setext 标题回溯升级
- `c08-quote-alert` 引用块 + GFM Alert
- `c09-mixed-long` 综合长 passage(多块 + 下方下移)
- `c10-cjk` 中英混排 + CJK 禁则 under streaming

**DoD**:每 case 重放**全程无跳变**(肉眼 + 截图回归);`?debug` 无掉帧、atlas 不 thrash;`NodeId` 稳定(过渡中再 commit 不回跳);闭合块 settle 后冻结;5C 规格表逐行被 case 覆盖。

---

## 节奏与门控

- **5A 先行**(机制是地基);**5B 紧随**(喂 5A);**5C 随 5B 增量补**(每补一个构造加一个 case);**5D 贯穿**(每相位用重放验收)。
- 建议顺序:**5A → 5B → (5C+5D 交替补构造/补 case)**。
- 每相位末:`cargo fmt/clippy/test` + `wasm-pack build` + `tsc` 过卡口;**核心验收 = 重放 case 全程无跳变 + 无掉帧**;高风险用 subagent 跑截图回归。
- 完成后:动画美学调参(缓动/时长 policy)、GPU 双态(路 A)、math 行内盒等从 [TODO2]/[TODO] 取。

## 关联

- 机制:[0016](../decision/0016-streaming-morph-render-model.md) · 驱动:[0017](../decision/0017-markdown-streaming-landing.md) · 表格:[0014](../decision/0014-table-two-pass-layout.md) · 块冻结:[0005](../decision/0005-turn-aggregation-and-settlement.md)。
- 验证视图 [TODO V] / 垂直度量 [TODO T](../../TODO.md);字块 move 设计 `spec/design/thinking.md §2`。
- lygia:`./lygia`(WGSL)= **参考库**(Prosperity 非商用许可);发行 shader 以自有实现为主,见上「shader 复用 lygia」约定。
