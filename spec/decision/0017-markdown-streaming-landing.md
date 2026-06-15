# 决策记录 0017:markdown 在 streaming 形变机制上的落地 —— 提交前沿 + 保守预测/和解

- 日期:2026-06-15
- 状态:已采纳
- 前置:**[0016](0016-streaming-morph-render-model.md)(本篇是其驱动层)**、0005(块冻结/settle)、0010(pulldown-cmark / vendored jcode,非增量)、0014(表格两趟布局,首个消费者)
- 定位:[0016] 是与内容无关的渲染机制,要求驱动层提供「带稳定 id 的活跃区布局快照 + committed/active 区分 + 提交节奏」(0016 §7)。**本篇定义 markdown 流如何满足这三项**——即"何时产生 past→current 端点"。

---

## 1. 问题:0016 要的三样,markdown 怎么给

LLM 输出 markdown,逐 token 到达,且**已渲染内容可能因后续 token 改变几何**(表格新行撑宽列、`**` 闭合致加粗变宽)。要喂给 0016,需回答:

1. **哪些块已提交(冻结)、哪个是活跃区?** → §2 提交前沿。
2. **活跃区当前该渲染成什么?**(预测) → §3 保守预测。
3. **何时产生一份新快照?** → §4 行边界 + 插值延迟。
4. **字块的稳定 id 从哪来?** → §6 append-only。

## 2. 提交前沿 = 最后一个未闭合的块

**活动区 = 最后一个未闭合的块(就一个块)**;其之前的全部块 = **已提交 = 冻结**,永不重解析、永不重排。这就是"提交前沿"落到 markdown 的具体形态,**与 0005「块冻结/settle」是同一概念**——非新实体,只是把"未冻结块"重新表述为"唯一可变的活跃区"。

**markdown 的回溯坑(提交前沿要算准)**:少数构造由后续行决定前文解析,前沿要把它们算进去:

- **Setext 标题**:一行普通文字,下一行 `===`/`---` → 它回头变 H1(字号变)。
- **表头行**:一行普通文字,下一行 `|---|` → 它回头变表头。
- **list 松/紧**:取决于后续空行,影响间距。

这些都**在活动块内、由下一行触发** → 活动块整块重解析即覆盖(§3),无需特殊处理。唯一**跨块**回溯是引用式链接,单列 §5。

## 3. 保守预测 = 照当前文本原样解析

**每个 tick 只重解析活动块,用 pulldown-cmark 照当前(不完整)文本原样解析**。这天然就是"保守预测":pulldown 喂 `**foo` 本就当**字面文本**(闭合才出 emphasis)→ **零预测代码**;`**` 闭合那一刻解析翻转 = 自然产生 reconcile delta,交 0016 补间。

**为何够、为何不上增量解析器**:tree-sitter 那套(子树复用 / changed-ranges / scanner 状态快照)是为"编辑落在文件任意位置"设计的;我们 **append-only,改动永远在末尾**,只需"重解析活动块"——取其"提交前沿"观念,弃其引擎([tree-sitter](https://github.com/tree-sitter/tree-sitter))。pulldown 非增量也无妨:活动区永远一个块,每 tick 重解析 = O(块大小),与会话长度无关。

**乐观预测(闭合前先猜样式)默认不做**——它要写预测逻辑(增实体),且保守预测配 0016 补间后,样式"长出来"已平滑,收益不抵成本。

## 4. 提交节奏 = 行边界 + 插值延迟

- **快照节奏 = 行边界**:活动块每完成一行,重解析 + 重排 → 产出一份快照交 `Scene::commit`。
- **插值延迟(可选,Valve 同款)**:渲染滞后一行,保证每次补间都跨"前一行态 → 当前行态"两个完整关键帧([Source Networking](https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking))。这就是"等一行再渲染"的工程依据。

活动块闭合(下一块开启)→ 通知 0016 等 settle → 冻结、移出 Scene(0016 §6)。

## 5. 活动块内变化 → 映射到 0016 的 enter/update/exit

| 变化 | 触发 | 映射(0016) |
|---|---|---|
| 新字到达 | 新 token | enter |
| 闭合致重新着色(`**`→粗 / `` ` ``→等宽) | 语法闭合 | size delta → update(依赖 per-role 度量 4A4) |
| 表格列变宽 | 新行 | pos delta → update(见 0014) |
| 已知宽后重折行 | 列宽定 | pos delta → update |
| 块被重新归类(setext / 表头 / 围栏) | 下一行 lookahead | 活动块整块重解析 → enter/update 混合 |
| 行提交 | 下一块开启 | 冻结、移出 Scene |

全部落到 `(block_seq, glyph_idx)` 上 → 0016 的 join 兜底。**完整**。

## 6. 身份:append-only ⇒ `(block_seq, glyph_idx)` 稳定

0016 要稳定 id;markdown 流提供它的依据是 **LLM 输出只追加不改写**:第 `block_seq` 块的第 `idx` 个字块永远是同一逻辑字块,重排只改几何不改身份。

- **表格不需要特殊 id**:行/列只影响 layout 算出的几何,身份仍是线性 `glyph_idx`;新行 = 追加的新 `idx`,旧字块 id 不变。
- **edge:remend 尾部改写**:若尾部补全重写了已发字块(非纯追加),被改字块按 0016 的 exit+enter 处理(身份失配自然走生灭)。可编辑/乱序内容的 diff 匹配(Myers / TransformMatchingTex)**超出范围**。

## 7. 逃逸口:引用式链接

引用式链接 `[x][ref]` 的 `[ref]: url` 可能在**更后面的块**出现,回头改前面已冻结块的链接样式——唯一跨块回溯。**接受一次性非动画 restyle**(仅改链接色,极少见),不为它建机制;要更干脆则列为 opinionated 非目标。

## 8. 决策

markdown 流以**「提交前沿 + 保守预测/和解」**满足 0016 的上游契约:

1. 活动区 = 最后未闭合块(= 0005 未冻结块);前缀已提交、冻结(§2)。
2. 每 tick 只重解析活动块、pulldown 原样解析 = 保守预测,零预测代码(§3)。
3. 行边界产出快照 + 可选滞后一行(插值延迟)(§4)。
4. 身份 `(block_seq, glyph_idx)` 靠 append-only 稳定(§6)。
5. 引用链接跨块回溯 = 一次性 restyle,不建机制(§7)。

**理由**:三个现实约束(append-only / markdown 固定语法 / 2D 文字)把增量解析器、回滚日志、预测器全部消去,markdown 驱动只剩"冻结前缀 + 重解析活动块 + 行边界快照"一小撮,其余交 0016。

## 9. 落地清单(Plan 5,驱动层)— 已落地(Plan 5B/5C,2026-06-15)

- [x] 活动块界定(= 0005 未冻结块):`app.rs::ensure_layouts` 的脏判据(`revealed_len` 变)= 正在生长的块;已 settle 的前缀块复用缓存 = 冻结、不再重解析。
- [x] 每 tick 重解析活动块(`parse_markdown` 原样 = 保守预测)→ 重排 → 经 `FrameGlyph` 稳定 id 喂 `Scene::commit`。**cadence = 逐帧(逐字揭示)**,比"行边界"更细;机制 join 兜底,过渡平滑。
- [x] 提交前沿 lookahead(setext / 表头 / 围栏):整块(整 part)重解析天然覆盖——下一行到达即翻转。
- [x] 测试(`app.rs`):`**bold**` 闭合 → Bold 角色、无字面 `*`;`(block_seq,glyph_idx)` 跨帧稳定;setext `===` → 段升级 Heading。重放 case `c01–c10` 覆盖 5C 规格表(`web/public/cases/`)。
- [ ] (留尾)**行边界 cadence + 插值延迟**(§4,现逐帧足够平滑);**part 内分块冻结**(现按 part 粒度重解析,长单消息 O(消息),非会话长度);**引用链接逃逸口**(§7,跨 part 罕见)。

---

参考先例:[tree-sitter](https://github.com/tree-sitter/tree-sitter)(增量/错误恢复,取"提交前沿"观念弃引擎) · [Source Multiplayer Networking](https://developer.valvesoftware.com/wiki/Source_Multiplayer_Networking)(插值延迟 = 等一行)。渲染机制见 [0016](0016-streaming-morph-render-model.md)。
