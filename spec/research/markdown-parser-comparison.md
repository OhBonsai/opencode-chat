# 调研:markdown 解析三套方案深度对比(0010 配套)

- 日期:2026-06-14
- 关联:[决策 0010](../decision/0010-markdown-parsing-strategy.md)、[决策 0004](../decision/0004-markdown-and-embeds.md)(语义层)、[决策 0009](../decision/0009-text-rendering-engine.md)(渲染引擎)、[0001 §2.2](../decision/0001-canvas-architecture.md)(接口契约)
- 目的:在**不改变现有大思路**的前提下,对三条 markdown 解析路线做"性能 / 兼容性 / 开发成本 / 效果"四维深挖,给 0010 的结论补足量化依据,并标定何时该重开决策。

---

## 0. 先钉死"大思路不变"的含义

任何方案都必须落在同一个缝里——即 `crates/core/src/content.rs` 的:

```rust
pub fn parse_markdown(src: &str) -> Vec<StyledSpan>   // 输入累积原文,输出带角色的 span
```

不变量(评估三套方案都以"能否塞进这个缝、不动其它层"为前提):

1. **接口契约不动**:content→layout→render 只传 `StyledSpan{ text, role }`,输出的是**语义角色**(Bold/Code/Heading/Link/Quote/ListMarker…)不是像素/坐标(0001 §2.2、0004 §2)。换解析器只动 `content` 内部。
2. **解析器只活在 `content`(core)层**:必须 **wasm-safe**——零阻塞依赖(无 `std::fs`、无 C 库、无 oniguruma/resvg)、不打包字体(守 BR5,0009)。当前 `jcode-render-core` 依赖只有 `pulldown-cmark + serde + unicode-width`,这是基线。
3. **流式不在解析器里解决**:防闪靠**排版层块冻结 + `remend` 尾部主动补全**(0004 §5、content.rs `remend()`),淡入靠 GPU `spawn_time`(0009)。解析器**不需要**自带行级 diff——这正是 0010 不抄 warp 的核心论据,三套方案评估时都不把"内建 diff"算成加分项。
4. **只重 parse 尾部脏块**:已完成块冻结后不再 parse,parse 调用永远只发生在"正在生长的尾部块"上(0004 §5)。所以**单次 parse 的绝对延迟**比"全文吞吐"更重要——尾部块通常只有几百字节~几 KB。

下面三套方案,都是"`parse_markdown` 内部换什么"的候选。

---

## 1. 三套方案定义

| | 方案 A:pulldown-cmark(现状) | 方案 B:手写 nom 组合子(warp 式) | 方案 C:AST 库(comrak / markdown-rs) |
|---|---|---|---|
| 范式 | **pull-parser**:事件迭代器 `Event` 流,无 AST | **递归下降 + 组合子**:自定义文法 → 自定义富文本模型 | **建 AST**:解析成完整语法树节点,再遍历 |
| 代表实现 | `pulldown-cmark 0.12`(经 `jcode-render-core` 适配) | Warp `crates/markdown_parser`(~6600 行含测试,`nom`) | `comrak`(cmark-gfm 移植)/ `markdown-rs`(wooorm,状态机 + mdast) |
| 我们怎么接 | `Event` 流 → `Block`/`StyledSpan`(已实现,markdown.rs) | 自写文法 → 自定义模型 → 再映射 `StyledSpan` | `Node`/`mdast` 树 → 遍历 → `StyledSpan` |
| 状态 | **已采纳** | 0010 备案(参考) | 本调研新增的"第三路"——介于"成熟库"与"全手写"之间 |

> 说明:0010 原文把表格画成 "Warp / jcode / 我们的封装" 三列,但第三列是**适配层**不是另一个解析器。本调研把"第三套方案"重定义为**真正的第三条解析路线**:用更重的 AST 库(comrak / markdown-rs)。这样三套方案恰好覆盖完整设计空间——**事件流 / 手写文法 / 全 AST**,对比才有意义。

---

## 2. 性能

### 2.1 硬数据(吞吐 / 单次 parse)

公开基准把范式差异量化得很清楚:

- **1Password `markdown-benchmarks`**(同机、10 万次迭代,越小越快):
  - pulldown-cmark(Rust):**3.785s**
  - comrak(Rust):**26.623s**(≈ pulldown 的 **7×**)
  - md4c(C,空回调):1.088s(参照系,纯 C 流式)
- **ferromark** 自称 309 MiB/s、"faster than pulldown-cmark and md4c";pulldown-cmark 紧随其后,均在**数百 MiB/s** 量级。
- 范式归因(CommonMark 社区与各 README 共识):**流式/事件解析器解析期几乎不分配**(md4c 回调、pulldown 的 pull 迭代器);**AST 解析器要分配大量节点**(comrak 用 `typed_arena` + 大量 `RefCell`),这是 7× 差距的根因。

### 2.2 映射到我们的真实负载

我们的负载**不是**"把 1MB 文档转 HTML",而是**每帧对几 KB 的尾部块重 parse**。两点结论:

- **A(pulldown)**:pull 迭代器零 AST 分配,单次几 KB parse 在**微秒级**,且 `remend` 只在前面拼几个闭合符,开销可忽略。完美契合"高频小块重 parse"。
- **C(comrak)**:7× 慢 + 每次建/弃整棵 arena 树。在"全文转换"场景可接受,但在我们"每帧重 parse 尾部块"的高频路径上,GC/分配压力被放大,**最不适配**。markdown-rs 是状态机(每字节记位置 + 建 mdast),比 comrak 轻但仍重于 pulldown,且为"忠实复刻参考解析器"牺牲了速度。
- **B(nom)**:理论上 zero-copy 组合子可以很快;但 warp 的实际策略是**每次内容变化重 parse 整段** + 解析后行级 diff(`compute_formatted_text_delta`)。若照搬,等于"全文重 parse",比我们"只重 parse 尾部块"更费;若只对尾部块跑自写 nom,性能可与 A 同级,但 diff 机制就白写了(我们用块冻结已解决,0010 §3)。

### 2.3 wasm 包体(性能的另一面)

| | 依赖面 | wasm 体积倾向 |
|---|---|---|
| A pulldown-cmark | `pulldown-cmark + serde + unicode-width`(现状) | **最小**,无 C/无正则引擎 |
| B 手写 nom | `nom`(+ 可能 `html5ever`/`markup5ever_rcdom` 若要 HTML,如 warp) | nom 本身轻;但 warp 为 HTML 引入 html5ever 会显著增重——**按需裁剪才小** |
| C comrak / markdown-rs | comrak 依赖较多(含 entity 表等);markdown-rs 纯 Rust 无 C 依赖、wasm-friendly(作者本就做 wasm) | comrak 偏大;markdown-rs 中等 |

**性能小结**:对"高频小块重 parse + 小包体"这个真实约束,**A ≥ B(若只 parse 尾部块)> C**。C 的 AST 开销在我们的高频路径上是净负担,而它换来的好处(见兼容/效果)我们当前用不满。

---

## 3. 兼容性

这里"兼容性"含三层:**(a) markdown 规范覆盖**、**(b) wasm/工程兼容**、**(c) 与我们既有机制(remend / 块冻结 / StyledSpan)兼容**。

### 3.1 规范覆盖(CommonMark / GFM)

| 能力 | A pulldown-cmark | B 手写 nom(warp) | C comrak / markdown-rs |
|---|---|---|---|
| CommonMark 基础 | ✅ 基于 CommonMark spec | ⚠️ 取决于自己写多全(warp 覆盖终端所需子集) | ✅ comrak **100% cmark-gfm 对齐**;markdown-rs 号称比多数实现更忠实(额外数千测试) |
| GFM 表格 / 删除线 / 任务列表 / 脚注 / 数学 | ✅ 都有,靠 `Options` 标志开关(`ENABLE_TABLES` 等) | ⚠️ warp 有 `markdown_tables` feature;其余要自己补 | ✅ comrak 五大 GFM 扩展齐全;markdown-rs 还有 MDX/frontmatter/math |
| "和 GitHub 渲染一模一样" | 接近但非逐字节 | 不保证 | ✅ comrak 是 GitHub cmark-gfm 的移植,**逐字节对齐**是它的卖点 |
| 罕见语法 / 自定义指令 | 受库能力限制 | ✅ 想加就加(完全掌控) | comrak 扩展点有限;markdown-rs 支持 MDX 等 |

要点:**我们当前只需要"标题/粗斜/行内与块代码/列表/引用/表格/数学"这个子集**(0004 §3、content.rs 已覆盖)。pulldown-cmark 的 `Options` 已能开齐这些。comrak/markdown-rs 的"100% 对齐 / 罕见语法"是**当前用不满的余量**;手写 nom 的"想加就加"则要自己背覆盖率。

### 3.2 wasm / 工程兼容

- **A**:纯 Rust、零阻塞依赖,wasm 一等公民——**已在产**,基线。
- **B**:nom 纯 Rust 可 wasm;但若像 warp 一样为内联 HTML 引 `html5ever + markup5ever_rcdom`,要确认其 wasm 可编(通常可,但增重)。我们 0004 已决定**内联 HTML 暂忽略**,所以这块可不引。
- **C**:markdown-rs 作者本就维护 wasm 场景,兼容好;comrak 可 wasm 但依赖更多、体积更大,需实测。

### 3.3 与既有机制兼容(最关键的一层)

- **remend(尾部补全)**:针对**未闭合行内/块语法**临时补闭合符再 parse(content.rs)。它对解析器**无侵入**——任何把 `&str` 吃进去的解析器都兼容。✅ A/B/C 都行。
- **块冻结 + 只重 parse 尾部块**:需要解析器"对一小段文本 parse 出块/行/span 结构"。A 的 `Document{ blocks }` 天然贴合;C 的 AST 也能遍历出块边界;B 要保证自写模型能划出"完整块 vs 生长块"边界(warp 的 `FormattedTextLine` 行模型其实更偏行级,块级 checkpoint 要自己定)。
- **StyledSpan 角色映射**:A 已有 `map_role`(JRole→StyleRole);C 的 mdast/Node 也能映射;B 要把自写 styles(weight/italic/strike/inline_code/hyperlink)映回我们的 `StyleRole`。三者皆可,**A 已完成、零增量**。

**兼容性小结**:规范覆盖 **C > A > B(B 取决于工时)**;但**与我们既有机制的兼容** **A 最顺(已接通)**,B/C 都要重做适配层。而我们**当前需要的规范子集 A 已满足**,C 的余量用不满。

---

## 4. 开发成本

| | 接入成本(一次性) | 维护成本(长期) | 风险 |
|---|---|---|---|
| **A pulldown-cmark** | **0**(已在产,content.rs + jcode markdown.rs) | **低**:跟随上游升版即可,规范正确性由社区背 | 极低 |
| **B 手写 nom** | **高**:warp 用 **~6600 行**(含测试)才换来终端级掌控;我们要么照搬要么自写一套文法 + 模型 + 测试 | **高**:CommonMark 边界 case 极多,正确性、GFM 扩展、回归测试全自己背 | 高(易和 spec 不一致) |
| **C comrak / markdown-rs** | **中**:换 crate + 重写"AST→StyledSpan"遍历(替换现有 `emit_block`/`map_role`),工作量类似一次适配重写 | **中**:依赖第三方,但 API 比 pulldown 的事件流更"树状"好遍历;comrak 偏重 | 中(体积/性能需实测) |

成本的本质对照(沿用 0010 §4 的判断):**手写 nom 的 6600 行,是用"自维护"换"完全掌控"**;我们既不需要终端特性、也不需要逐字节 GitHub 对齐,**为掌控付 6600 行不划算**。comrak/markdown-rs 是"花一次适配重写,换更强规范/扩展",但**当前规范子集 A 已够**,这笔重写也暂无回报。

---

## 5. 效果(渲染产出质量)

注意:**最终视觉效果主要由 layout(pretext)+ render(wgpu)决定,不由解析器决定**(0009)。解析器影响的是"语义识别得对不对、全不全"。

| 效果维度 | A | B | C |
|---|---|---|---|
| 基础富文本(粗/斜/码/标题/列表/引用) | ✅ 已达"可用" | ✅(自写) | ✅ |
| 表格 | ✅ 已渲染成 `│` 分隔行(content.rs) | ✅ warp 有 | ✅ |
| 罕见 / 边界 markdown 正确率 | 好(社区级) | 取决于自写 | **最好**(C 逐字节对齐参考实现) |
| **可点超链接 + 跳转**(`hyperlink + Action`) | ✘(`Link` 角色只上色,无交互) | ✅ warp 原生有 | ✘(库给结构,交互仍需自做) |
| 逐行/逐块精确入场动画 | 块级够用;行级需自加 diff | ✅ warp 行级 diff 现成 | 需自做 |
| 流式不闪 | ✅ remend + 块冻结 | ✅(warp 行级 diff,另一条路) | ✅ remend 同样适用 |

关键:**warp 在"效果"上唯一真正领先的是"可点链接"与"行级 diff 动画"**——但 0010 已判定这俩属于 **Plan 3(input/选区/hit-test)范畴**,且**与解析器选型解耦**:可点链接需要的是 `hyperlink + Action` 的**数据 + 命中派发**,我们完全可以在保留 pulldown 的前提下,给 `StyleRole::Link` 补 URL 数据 + hit-test(不必换解析器)。

---

## 6. 四维总评

| 维度 | A pulldown-cmark(现状) | B 手写 nom(warp) | C AST 库(comrak/markdown-rs) |
|---|---|---|---|
| 性能(高频小块重 parse + 小包体) | ★★★★★ | ★★★★(仅 parse 尾部块时);★★(照搬全文重 parse) | ★★(AST 分配,≈7× 慢) |
| 兼容性(规范覆盖) | ★★★★(子集已够) | ★★(自背覆盖率) | ★★★★★(100% 对齐) |
| 兼容性(wasm + 既有机制) | ★★★★★(已接通) | ★★★ | ★★★★(markdown-rs 好,comrak 待测) |
| 开发成本 | ★★★★★(0 增量) | ★(~6600 行自维护) | ★★★(一次适配重写) |
| 效果 | ★★★★(差可点链接,可后补) | ★★★★★(可点链接 + 行级动画现成) | ★★★★ |

---

## 7. 结论(与 0010 一致,并补强)

**继续用方案 A(pulldown-cmark / jcode-render-core),不改 B、不上 C。** 量化后的理由:

1. **性能上 A 最贴合真实负载**:我们是"每帧重 parse 几 KB 尾部块",pull 迭代器零 AST 分配是微秒级;C 的 AST 在高频路径上是 ~7× 净负担;B 若照搬 warp 的全文重 parse 反而更费。
2. **兼容性的"余量"我们用不满**:C 的 100% 逐字节对齐、B 的"想加就加",换来的能力当前 0004 的子集都不需要;而 A 的 `Options` 已能开齐 表格/删除线/任务列表/脚注/数学。
3. **开发成本差距悬殊**:A = 0 增量;B = ~6600 行自维护正确性;C = 一次适配重写 + 体积/性能实测。在"无新需求"时这两笔投入无回报。
4. **效果上 A 的唯一短板(可点链接)与解析器解耦**:可在保留 A 的前提下,Plan 3 给 `Link` 角色补 URL + hit-test,不需要为它换解析器。
5. **大思路全程不动**:三套方案都只在 `parse_markdown` 内部替换;真正决定"流式不闪 / 淡入 / 布局质量"的是块冻结 + remend + pretext + wgpu,与解析器选型正交。

## 8. 重新评估触发条件(在 0010 基础上量化)

- **要"和 GitHub 逐字节一致"或踩到 pulldown 的 spec 边界 bug** → 评估 **C(comrak)**(接受 ~7× 解析慢 + 体积,因为正确性优先)。
- **要终端/编辑器级可编辑 markdown 或罕见自定义指令** → 评估 **B(手写 nom)**(接受 6600 行级自维护)。
- **要可点链接 / 引用跳转 / 脚注跳转** → **不换解析器**,按 warp 的 `hyperlink + Action` 思路给 `StyleRole::Link` 补数据 + hit-test(Plan 3 input)。
- **要逐行精确入场动画 / 超长输出最小重绘** → 在块层加一个 `compute_formatted_text_delta` 式 diff(当前块冻结已够,暂不需要)。

---

## 9. 来源

- 1Password markdown 基准(pulldown 3.785s / comrak 26.623s / md4c 1.088s):<https://github.com/1Password/markdown-benchmarks>
- ferromark(309 MiB/s,"faster than pulldown-cmark and md4c"):<https://github.com/sebastian-software/ferromark>
- pulldown-cmark(pull parsing、高性能低内存、`Options` GFM 扩展):<https://github.com/pulldown-cmark/pulldown-cmark> / <https://docs.rs/pulldown-cmark/latest/pulldown_cmark/struct.Options.html> / <https://lib.rs/crates/pulldown-cmark>
- comrak(100% cmark-gfm 对齐、AST via typed_arena + RefCell、"非最快"):<https://github.com/kivikakk/comrak> / <https://crates.io/crates/comrak>
- markdown-rs(状态机 + mdast、忠实复刻参考解析器、MDX/math/frontmatter、wasm):<https://github.com/wooorm/markdown-rs>
- "MD4C 为何快" / 流式 vs AST 分配讨论:<https://talk.commonmark.org/t/why-is-md4c-so-fast-c/2520>
- 仓库内:[0010](../decision/0010-markdown-parsing-strategy.md)、[0004](../decision/0004-markdown-and-embeds.md)、[0009](../decision/0009-text-rendering-engine.md)、`crates/core/src/content.rs`、`vendor/jcode-render-core/`
