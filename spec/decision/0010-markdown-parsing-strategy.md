# 决策记录 0010:markdown 解析策略 —— pulldown-cmark(jcode)vs 手写 nom(warp)

- 日期:2026-06-14
- 状态:已采纳(沿用 jcode-render-core / pulldown-cmark;warp 的 nom + 行级流式 diff 备案为参考)
- 前置:0004(markdown 语义层)、0009(文字渲染引擎)、[plan2 H](../plan/plan2-usable-chat.md)
- 来源:调研 `~/w/agentscode/warp`(Warp 终端)如何实现 markdown,与我们已采用的
  jcode-render-core(pulldown-cmark)对比

## 1. 背景

Plan 2 H 已采用 vendored `jcode-render-core`(内部 pulldown-cmark)解析 markdown(见 0009)。
为校准选型,调研了 Warp 终端的 markdown 实现,发现它走的是**完全不同的路线**:手写 nom
解析器 + 行导向富文本模型 + 行级流式 diff。本 ADR 记录三方对比与"是否要改"的结论。

## 2. 三方实现对比

| | Warp | jcode-render-core(我们采用) | 我们的封装 |
|---|---|---|---|
| 解析器 | **手写 `nom`** 组合子(`crates/markdown_parser`,~6600 行含测试) | `pulldown-cmark` 0.12 | 经 jcode 适配成 `StyledSpan`+角色 |
| HTML | `html5ever` + `markup5ever_rcdom` | pulldown-cmark 内联 HTML 透传 | 暂忽略 |
| 模型 | 行导向 `FormattedText` = `VecDeque<FormattedTextLine>`(Heading/TaskList/CodeBlock/Table/Image/缩进行…) | 块导向 `Document` = `Vec<Block>`(每 Block 含 `StyledLine`) | 展平成 `Vec<StyledSpan>` + `\n` |
| 行内样式 | `FormattedTextFragment{ text, styles }`,styles = weight/italic/strikethrough/inline_code/**hyperlink** | `StyledSpan{ text, role, fill, attrs }` | `StyledSpan{ text, role }` |
| 可点链接 | ✅ `hyperlink + Action` 点击派发 | ✘(只有 Link 角色,无交互) | ✘ |
| GFM 表格 | `parse_markdown_with_gfm_tables`(feature `markdown_tables`) | `Options::ENABLE_TABLES` → `Block::table` | 渲染成 " │ " 分隔行 |
| **流式** | **整段重 parse → 行级 diff**:`compute_formatted_text_delta(old,new)` 算 `common_prefix_lines` + `new_suffix`,只替换变化的尾部行 | 无内建 diff(前端处理) | **排版层块冻结** + `remend` 尾部补全防闪 |
| 开关 | feature flags(tables/mermaid/images/agent-mode 各自可开关) | — | — |

## 3. 关键差异:流式更新策略

两条思路都解决"流式 markdown 不全量重渲染",但层级不同:

- **Warp(解析后行级 diff)**:每次内容变化 → 重 parse 整段 → 与上一版 `FormattedText` 按行算
  公共前缀,只更新尾部变化行(`is_noop()` 判无变化)。优点:精确知道"哪几行变了",利于做
  局部更新/动画;代价:每次重 parse 全文。
- **我们(排版层块冻结,Plan2 G)**:已完成块冻结(不重 parse 也不重排),只对正在生长的尾部
  块重 parse + 重排;`remend` 对尾部未闭合语法主动补全防闪。优点:连重 parse 都只发生在尾部块;
  与渲染缓存同边界。

二者等效解决问题,我们的更省(不重 parse 已完成块)。Warp 的行级 diff 在"想精确驱动逐行
入场动画"时更直接。

## 4. 决策

**沿用 jcode-render-core(pulldown-cmark),不改用 warp 式手写 nom 解析器。** 理由:

1. **省自维护**:warp 6600 行手写解析器换来对终端特性/GFM/HTML/可点链接的完全掌控;我们用
   成熟库 + jcode 的中立模型即可,markdown 语义已够"可用"(标题/粗斜/代码/表格/列表)。
2. **流式已解决**:我们的块冻结 + remend 已覆盖流式不闪 + 不全量重排,不需要 warp 的行级 diff
   机制(且我们更省)。
3. **接口缝不变**:content→layout→render 契约不动(0001 §2.2),换解析器只动 content 内部,
   未来要换不被锁死。

## 5. 可借鉴项(记录,非本 ADR 强制实施)

- **可点超链接**:warp 的 `FormattedTextStyles.hyperlink` + `hyperlinks() -> Vec<(Range, Hyperlink)>`
  + `Action` 点击派发,是"点链接/点引用跳转"的现成参考。我们的 `StyleRole::Link` 目前只上色无
  交互;Plan 3 做 input/选区/hit-test 时,可按此补 hyperlink 数据 + 命中派发。
- **行级 diff**:若 Plan 3 要做"逐行/逐块精确入场动画"或"超长输出的最小重绘",可参考
  `compute_formatted_text_delta` 在我们的块层加一个 diff(当前块冻结已够,暂不需要)。
- **feature flag 化**:warp 把 tables/mermaid/images/agent-mode 各自 feature 门控,便于裁剪体积;
  我们做 embed(Plan 3)时可借鉴按特性裁剪。

## 5.1 三方案四维深挖与"自定义语法"边界(2026-06-14 补)

配套调研见 [markdown-parser-comparison](../research/markdown-parser-comparison.md)(性能/兼容性/开发成本/效果四维量化,把"三套方案"重定义为覆盖完整设计空间的三条路线:**事件流 pulldown / 手写 nom / 全 AST comrak·markdown-rs**)。量化要点:

- **性能**:我们的真实负载是"每帧重 parse 几 KB 尾部块",pull 迭代器零 AST 分配最贴合;comrak 的 AST 在公开基准里约 **7× 慢于 pulldown**(1Password markdown-benchmarks),在高频路径上是净负担。
- **兼容性**:comrak 的"100% 逐字节对齐 GitHub"、手写的"想加就加"换来的能力,当前 0004 的子集都不需要;pulldown 的 `Options` 已能开齐 表格/删除线/任务列表/脚注/**数学(`ENABLE_MATH`,0.11 起)**。
- **开发成本**:A=0 增量;B≈6600 行自维护正确性;C=一次适配重写 + 体积/性能实测。

**关键边界——"自定义语法"不走解析器,走标签层(0006)。** 评估中"comrak 全 GFM+MDX/frontmatter、逐字节对齐、手写想加就加"三点很诱人,但逐一对"流式 LLM 聊天渲染"核对后:math 已在 A;MDX/frontmatter 聊天正文用不上;逐字节对齐对乖 LLM 输出无回报;而唯一对定制产品真有价值的"自定义指令/标签",**应在 0006 的 pre-markdown segmenter + 标签注册表里加,而非换 markdown 解析器**:

- 块/区域级(`<thinking>`、`:::note` 容器):segmenter 多认一种开启符,复用 hold 区 + FSM + 注册表(0006 §5.1 补)。
- 行内角标(`@提及`、引用角标):pulldown 正常 parse 后加一道 `StyledSpan` 后处理扫描,把命中片段改成 `Chip`/`Link` role(0006 §5.2 补);标准 `[^1]` 脚注可直接用 pulldown `ENABLE_FOOTNOTES`,再按 §5(可点链接)补跳转数据。

故"想加就加 / 罕见语法"在**标签层**已可满足,不构成换 B/C 的理由;只有"魔改核心 markdown 文法"或"逐字节对齐 GitHub"才触发下面的重评估。

## 6. 重新评估触发条件

- 需要终端/编辑器级的**可编辑 markdown**或对解析有 pulldown-cmark 满足不了的定制(罕见语法、
  自定义指令)→ 再评估手写解析器;
- 需要**可点交互**(链接/引用/脚注跳转)→ 借鉴 warp 的 hyperlink+Action(Plan 3 input 范畴)。

## 7. 来源 / 链接

- Warp markdown:`~/w/agentscode/warp/crates/markdown_parser`(`parse_markdown`/`FormattedText`/
  `compute_formatted_text_delta`)、`crates/editor/src/content/markdown.rs`(消费)
- 我们采用:[`vendor/jcode-render-core`](../../vendor/README.md)、`crates/core/src/content.rs`
- 四维深挖:[markdown-parser-comparison](../research/markdown-parser-comparison.md)(基准数据与来源在该文 §9)
- 相关:0004(markdown 语义层)、0006(标签层 = 自定义语法落点)、0009(渲染引擎)、Plan 2 G(块冻结)/H(markdown)
