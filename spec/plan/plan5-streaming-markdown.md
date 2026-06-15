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
| 表格 | 逐行;新行撑宽列 → 右侧列/旧行右移 | pos delta → update(单调增宽) | **0014 A 已落地**(见 5E) |
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

## Phase 5E — 真表格(0014 A 等宽网格)— 已落地(2026-06-15)

> 域:`content`(emit_table)· `app`(装饰)· `theme` · `layout-bridge`/`glyph.wgsl`(等宽角色)。把占位的 `" │ "` 平铺换成对齐的真表格。

**任务**
- [x] **content.rs `emit_table`**:按列 max 显示宽(`display_width`,CJK/全角/emoji 计 2)空格补齐;新增角色 `TableCell`/`TableHeader`(as_u32=17/18);单元格间 `" │ "`(等宽 → `│` 竖线对齐成网格)。
- [x] **等宽角色**:`layout-bridge.fontForRole` 17/18 → MONO 同字重(表头表体对齐;`glyph-raster` 同走 `fontForRole`);`glyph.wgsl` 17/18 上色(表头略亮)。
- [x] **装饰**:`app.block_decorations` 由 `TableHeader` y 范围画表头淡底 + 底线,整表 y 范围画表尾外边线;`theme` 加 `TABLE_HEADER_BG`/`TABLE_RULE`。
- [x] **测试**:列对齐 / CJK 计 2 / 原始 `|`·`---` 不显形 / 角色正确(content.rs);`c06-table` 重放验流式列变宽。

**DoD**:表格列对齐(竖线成列)、表头有底+线;流式逐行到达时列变宽走 0016 补间不跳变(`?debug` 选 `c06-table`)。
**未做(边界,见 [0014](../decision/0014-table-two-pass-layout.md))**:竖直网格线 rect、斑马底、单元格内联格式、超宽列折行(B 比例两趟,另评审)。

### 5E.1 表格问题汇总(c06-all 实测,2026-06-15)

> `c06-all` 五段实测后的现状。✅ 达标 / ❌ 未达预期 / 🐛 真 bug。每条标根因 + 落点。

| # | 问题 | 现状 | 根因 | 落点 |
|---|---|---|---|---|
| 1 | **对齐方式**(`:--`/`:-:`/`--:`) | ✅ **已修**(2026-06-15) | jcode 表格模型丢了 per-列 `Alignment` | jcode 加 `table_align`(map pulldown `Alignment`)→ `emit_table` 按对齐左/右/居中补空 |
| 2 | **单元格内联格式**(`**粗**`/`*斜*`/`` `码` ``) | ✅ **已修(等宽 v1)** | jcode 把单元格压成纯字符串 | jcode 加 `table_spans`(cell=span 序列)→ `cell_role`:码→`Code`、粗/链接→`TableStrong`(等宽加粗)、**斜→`TableEm`(等宽斜体,2026-06-15 修 italic 误成 bold)**。`~~删~~` 删除线装饰留后续;比例体见 0014 B |
| 3 | **格内链接 URL 泄漏** | ✅ **已修** | 链接 ` (url)` 推进段落缓冲 → 表后另起一行 | jcode:`in_table` 时 End(Link) 不追加 URL;格内链接只显文字(Link 角色)|
| 4 | **raw 先闪后 snap** | ✅ **raw 抑制 + 框先行**(2026-06-15) | 0017 §3 逐帧原样揭示 | `is_pending_table` hold 成形中表格;确认后 #5 的网格/外框由 glyph 位置**即时绘制**(rect 无淡入)、单元格文字走 `spawn_time` **淡入** → "框先行、字填入"自然成立。刻意拉长 stagger 留 0017 §10 reveal 调度器 |
| 5 | **真竖直网格/边框 + AO** | ◐ 外框+行横线已落地;**连续竖线/AO → 走 0018** | B 已对齐(列 x 一致),连续竖线可画;但更优解 = **SDF 面板图元**(整框/网格/AO/选中一个 shader) | **改走 [0018 SDF 面板图元](../decision/0018-sdf-panel-decoration-primitive.md)**:colRatios/rowRatios 进小 storage buffer,fragment 画连续竖线 + AO + 圆角 + 选中(Zed 式数据驱动,deep research 验证)。现 FrameRect 网格过渡用 |
| 6 | **宽表溢出 / resize 折行** | ✅ **B 落地**(2026-06-15) | 之前 char-count A 行不可断;B 像素两趟可量可缩 | **接通 B**(content→layout 带 `TableRegion` sidecar + JS placeTable 两趟):列超 maxWidth → 按比例缩到 MINC + **格内受限折行**(`wrapRange`,行高=最多行数);resize 时 max_width 变→重排→表格折行塞下 |
| 9 | **表头底色不在表头** | ✅ **已修**(2026-06-15) | 表头底用**字形高**(≈字号)而非**整行高**(行距 1.4×)、贴字形顶 → 窄带浮在行内像落到下一行 | `app.block_decorations`:表头底改为**从表顶填到表头/首行分隔线**(用去重行顶 `tops[1]`),填满整个表头行 |
| 7 | **CJK 列错位** | ✅ **B 落地**(2026-06-15) | char-count 补白要求 2:1 字体,系统 mono CJK 回退非 2:1 → 错位 | **B 像素两趟(§5F)**:measureText 实测列宽,任意字体都对齐 |
| 8 | **字体切换表格不变** | ✅ **B 落地** | 原表格钉死 `MONO` | **B 随之解决**:像素量不要求等宽,表格用所选预设字体并跟随切换(§5F) |
| — | ASCII 列对齐 / 残缺行补空·超列丢弃 / 对齐方式 | ✅ 达标 | 0014 A + #1 | — |

**已修(2026-06-15)**:#1/#2/#3 经「**jcode 表格建模升级:cell=span 序列(`table_spans`)+ per-列 alignment(`table_align`)**」一次做完(additive,`table` 纯串保留向后兼容);`emit_table` 消费富数据 + 新角色 `TableStrong`(等宽加粗,5E.1 #2);content 4 测 + jcode parse 验证。
**留尾**:#4 phase2(header 骨架,reveal 策略)、#5(真网格/斑马,需列 x 坐标 = 扩 layout 契约)、#6(宽表溢出,需产品决策);**#7 CJK 错位 + #8 字体切换 = 留 TODO,方向「限制表格字体(LXGW 真 2:1)」**;#2 的删除线/比例体 + #7 的通用解 = **0014 B(像素两趟),评估后改动过大暂缓**。

### 5E.2 opencode TUI 对照(印证,2026-06-15)

读 `/Users/wp/w/agentscode/opencode/packages` 源码,对照其流式 markdown 实现:

- **流式 = remend healing + marked 重解析**(`ui/src/components/markdown-stream.ts`):`heal = remend(text, { linkMode: "text-only" })` 补全半截语法 → `marked.lexer` 分块 → 每 token 批次重渲染(Web 用 `morphdom` diff DOM;TUI 用 @opentui diff 终端缓冲)。唯一特判:**尾部未闭合代码围栏**单独实时渲染。**无逐字 move/补间**——靠 diff 重画。
  - 印证我们 0017 的 healing(remend)+ 活动块/raw 抑制思路;我们比它多一层 0016 逐字补间(更进一步)。
- **`remend` 的 `linkMode: "text-only"`**:opencode 用它让链接在流式中只显文字、不漏 URL —— **与我们 5E.1 #3 同解**(可对齐用法)。
- **CJK 表格对齐靠"真 2:1 等宽 + 列数补齐"**:TUI 是固定字符单元格网格,对齐 = `Bun.stringWidth`(wcwidth,CJK 计 2)补列 + **终端字体本就是真 2:1 等宽**。**坐实 5E.1 #7**:我们 char-count 补白方向没错,缺的只是"真 2:1 字体"——做 #7 时**钉 LXGW(真 2:1)即达 opencode 同款对齐**,无需上 B 像素两趟。
- 边界:`markdown-stream.ts`(分块/healing)已读全;真正把表格画成带边框单元格的 painter 在 **@opentui 框架层**(app 代码只产 markdown),不在 `tui/src`。

> 结论:**做 #7 直接走"限制字体 = LXGW"**(opencode 已验证此路对齐 CJK);B 像素两趟仍为通用解,非必需。

## Phase 5F — 表格像素两趟(0014 B)— 已落地(2026-06-15)

> **已接通端到端**:`content.rs`(emit_table 去补白/去 │、产 `TableRegion`;`parse_markdown_tables → (spans, tables)`)→ `LayoutEngine::layout(spans, tables, max_width)`(seam/support/app-test 三 impl 同步)→ `wasm/layout_bridge.rs`(`apply` 传第 4 参 `tables` = `[{rows,aligns}]`)→ `layout-bridge.ts`(`placeTable` 像素两趟 + `wrapRange` 格内折行 + 列缩到 MINC)。content 测试改为验 `TableRegion` 结构。**未做**:连续竖直网格线(#5,需 JS 回传 colX 给 app 画 rect);现表格 = 像素列间距 + 外框 + 行线 + 表头底(无 │)。下方为原方案存档 →


- **状态(2026-06-15)**:**落地方案已定**(本节);**JS 两趟摆位已落地**(`web/layout-bridge.ts::placeTable` + `layout()` 第 4 参 `tables?`,additive、tsc 绿、无 `tables` 时走 0014 A 旧路 → 零行为变化)。**待接**:Rust 侧 `content.rs` 产 `TableRegion` sidecar + `LayoutEngine` 通参 + `app` 透传 → 激活两趟;之后 `colX` 回传画 #5 网格。落地清单见 §末。

> 目标:用**像素实测对齐**取代"字符数补空格 + 假设 2:1 字体"。一次解决 **#7 CJK 对齐 + #8 字体跟随切换 + #5 连续竖线 + 任意字体**,并解锁格内折行。性能见下(≈现状,measureText 本就在跑)。与北极星正交协同(0017 §10 三层分工:reveal 策略 → **本布局** → 0016)。

### 契约(最小 sidecar,不破扁平 1:1)

glyph 仍按发射序留在扁平 run 数组(保 app 的 1:1 + 0016 `glyph_idx` 稳定);**额外旁传结构标注**:

```rust
// content.rs:parse_markdown 返回值加表格区
pub enum Align { Left, Center, Right }
pub struct TableRegion {
    pub rows: Vec<Vec<(u32, u32)>>, // rows[r][c] = 该格在 spans 数组里的 [start_run, end_run)
    pub aligns: Vec<Align>,         // 每列对齐(来自 jcode table_align)
}
// parse_markdown(src) -> (Vec<StyledSpan>, Vec<TableRegion>)
```

- **emit_table 改**:**不补空格、不发 ` │ ` 分隔 run**;每格按内容发 run(沿用现有 `cell_role` → TableCell/Strong/Em/Code);行间 `\n`;同时记每格 run 区间 + 对齐 → 一个 `TableRegion`。
- **LayoutEngine trait**:`layout(spans, tables, max_width)`(+1 参)。
- **wasm `layout_bridge.rs`**:把 `tables` 作第 4 个 JS 参(`Array<{rows:[[ [s,e] ]], aligns:number[]}>`)。
- **app.rs**:`parse_markdown` 拿 `(spans, tables)` 一并透传 layout;`BlockCache` 存 tables(脏判据不变)。

### 两趟(JS `layout-bridge`,操作已建的 `gs[]`)

run 区间 → grapheme 区间(建 `gs` 时记 run→grapheme 偏移)。adv **已在 `gs[k].adv` 量好**(measureText 本就跑过)→ 两趟只是**用已量数据**:

```
// 趟①:列宽 = 每列各格内容宽的 max(格宽 = 该格 gs 的 adv 之和)
colW[c] = max over rows( Σ gs[k].adv for k in cell(r,c) )
// 趟②:摆位(无补白空格)
colX[0]=tableLeft;  colX[c]=colX[c-1]+colW[c-1]+CELL_PAD*2+GRID_W
每行 r(rowTop = base + r*lineH):
  每列 c:  slack=colW[c]-cellW;  off = 右?slack : 中?slack/2 : 0
           penX = colX[c]+CELL_PAD+off
           逐 grapheme 写 out[k]=[penX-goff, rowTop-goff, cell, cell]; penX+=adv
```

- **性能**:measureText 次数 ≈ 现状(甚至更少,可"整格一次量");趟② 纯算术;layout 只在脏块跑、非每帧;渲染实例**更少**(无补白空格)。可加 `(格文本,role)→宽` 缓存,streaming 只量新格(后续优化)。
- **半截表格**:`rows` 只含已揭示行 → colW 随揭示增量增长 → 交 0016 平滑长大(支持北极星"骨架先行→列生长")。

### 渲染端(网格/边框)

- **#5 连续竖直网格线在 B 后自动成立**:列 x 一致 → 由 `colX` 派生全表高竖 rect(干净 cols+1 条)。两条路:(a) JS 把列边界 x 一并回传给 app;(b) app.rs 复用已注释的竖线代码——但需列 x,故选 (a) 更直接(layout 额外回传 `colX[]` per table)。外框/行横线/表头底沿用现状。
- 不再发 ` │ ` 字符(竖线 = rect)。

### 文件清单 + 建议分工(你并行改 Rust,避免撞)

| 文件 | 改动 | 建议归属 |
|---|---|---|
| `core/content.rs` | emit_table 去补白/分隔 + 产 `TableRegion`;`parse_markdown` 返回 `(spans, tables)` | **你(Rust)** |
| `core` LayoutEngine trait | `layout(spans, tables, max_width)` | 你/我 |
| `core/app.rs` | 透传 tables;BlockCache 存;#5 网格用回传 colX | 你/我 |
| `wasm/layout_bridge.rs` | tables → 第4参;接收 colX 回传 | **我(JS/bridge)** ⏳ 待 trait 改 |
| `web/layout-bridge.ts` | 两趟 + 回传 colX;去表格行 inTable 折行特判(B 接管) | **我(JS)** —— ✅ **两趟摆位已落地**(`placeTable` + `layout()` 第4参 `tables?`,additive、tsc 绿、无 `tables` 时走 0014 A 旧路);⏳ colX 回传(#5 网格)+ 去 inTable 特判 待 trait/bridge 通参后接 |

### 测试 / 验收

- content.rs:emit_table 产出无补白空格、`TableRegion` 行列 run 区间正确、aligns 正确。
- 重放 `c06-all`:CJK 列对齐(像素)、对齐方式生效、连续竖线干净、切字体表格跟随、宽表(配 #6 不折)、`?debug` 无掉帧。
- 半截:逐行揭示时列宽单调增长、0016 平滑(无 snap)。

### 非目标 / 留尾

- **格内折行**(超宽格在 colW 内换行):v1 可不做(单行格),趟② 预留;
- 宽表溢出仍按 **#6**(整行不折、溢出 pan);B 下也可改"按列折",另议。
- 删除线装饰(#2 留尾)正交,不在此。

### 落地清单(turnkey,按 §文件分工)

- [x] **JS 两趟摆位**(`web/layout-bridge.ts`,我):`placeTable`(趟① 列宽 = 列内格 adv 之和 max → 像素对齐;趟② 按 `aligns` 左/中/右摆位,无补白)+ `layout()` 第 4 参 `tables?` + `runStart[]`(run→grapheme 映射)+ 主循环命中表格区即两趟跳过线性流。**additive、tsc 绿、无 `tables` 行为不变**。
- [ ] **content.rs `emit_table`**(你/Rust):去补白/去 ` │ ` 分隔 run、每格按内容发 run;产 `TableRegion{rows:[start_run,end_run), aligns}`;`parse_markdown -> (spans, tables)`。
- [ ] **`LayoutEngine` trait + impls**(你/我):`layout(spans, tables, max_width)`;改 `LayoutBridge`(wasm)与 `MonospaceLayout`(support,忽略新参)。
- [ ] **`app.rs` 透传**(你/我):`(spans, tables)` 一并喂 layout;`BlockCache` 存 tables(脏判据不变)。
- [ ] **`wasm/layout_bridge.rs`**(我,待 trait 改):`tables` → 第 4 个 JS 参。
- [ ] **colX 回传 + #5 网格**(我):`LayoutResult` 扩 per-table `colX[]`;`layout-bridge.ts` 回传;`app.block_decorations` 由 colX 派生竖线 rect(cols+1 条)+ 去 ` │ ` 字符。
- [ ] **去 inTable 折行特判**(我):B 接管表格摆位后,移除 5E.1 #6 的 `inTable` 整行不折逻辑。
- [ ] **验收**:重放 `c06-all` —— CJK 像素列对齐 / 对齐方式 / 连续竖线 / 切字体跟随 / 半截列宽单调增长经 0016 平滑;`?debug` 无掉帧。

## 节奏与门控

- **5A 先行**(机制是地基);**5B 紧随**(喂 5A);**5C 随 5B 增量补**(每补一个构造加一个 case);**5D 贯穿**(每相位用重放验收)。
- 建议顺序:**5A → 5B → (5C+5D 交替补构造/补 case)**。
- 每相位末:`cargo fmt/clippy/test` + `wasm-pack build` + `tsc` 过卡口;**核心验收 = 重放 case 全程无跳变 + 无掉帧**;高风险用 subagent 跑截图回归。
- 完成后:动画美学调参(缓动/时长 policy)、GPU 双态(路 A)、math 行内盒等从 [TODO2]/[TODO] 取。

## 关联

- 机制:[0016](../decision/0016-streaming-morph-render-model.md) · 驱动:[0017](../decision/0017-markdown-streaming-landing.md) · 表格:[0014](../decision/0014-table-two-pass-layout.md) · 块冻结:[0005](../decision/0005-turn-aggregation-and-settlement.md)。
- 验证视图 [TODO V] / 垂直度量 [TODO T](../../TODO.md);字块 move 设计 `spec/design/thinking.md §2`。
- lygia:`./lygia`(WGSL)= **参考库**(Prosperity 非商用许可);发行 shader 以自有实现为主,见上「shader 复用 lygia」约定。
