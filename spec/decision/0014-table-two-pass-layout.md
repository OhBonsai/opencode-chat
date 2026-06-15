# 决策记录 0014:表格列对齐(两趟布局)—— 现状、选项、为何独立成相位

- 日期:2026-06-14
- 状态:已采纳;**A 等宽网格已落地(Plan 5,2026-06-15,见 §6)**;B 比例两趟仍为升级项
- 前置:0001 §2.2(content→layout→render 契约 / 每帧一次跨界 AR10)、0010(解析沿用 pulldown-cmark / vendored jcode)、0011(quad/SDF 图元)、Plan 4B(矩形装饰图元已就位)
- 触发:Plan 4B2「表格两趟列宽对齐」从 4B 拆出——它比 4B 其余项(引用条/Alert/hr/令牌)大一个量级,且有真实产品/工程取舍。

## 1. 背景与现状

LLM 输出常含 GFM 表格。**解析侧已就位**:vendored jcode 开了 `ENABLE_TABLES`,`parse_markdown` 吐 `BlockKind::Table`,原始单元格按行存在 `Block::table`(行主序,首行表头),**列布局显式留给前端适配器**(model.rs 注释 "column layout … performed by each front-end adapter")。

**现状渲染**(content.rs `emit_block`):表格被展平成「每行一行、单元格用 `" │ "` 分隔、表头 `Heading` 角色」的纯文本序列。**问题**:我们用**比例字体**(自带字体决策 0009/0011),`" │ "` 平铺下各行列**不对齐**——`Alice│30` 与 `Bob│25` 的竖线错位,观感差 GitHub 一截。

## 2. 难点 = 为什么不能顺手做

表格列对齐的本质是**两趟布局**:① 先量出每列的 max-content 宽;② 再按列宽定位每格 + 画网格/边框/斑马。这与我们现有管线有三处张力:

1. **比例字体需实测宽**:列宽 = 该列所有单元格渲染宽的最大值。宽度只有 **JS(Canvas `measureText`,在 `layout-bridge.ts`)**能准确量(Rust 侧无字体度量,CR1)。故两趟必须发生在 JS 排版桥里,且要把"这是表格、这些是单元格"的结构**穿过** content→layout 契约(现契约是扁平 `(text, role)` run 序列,不带二维表结构)。
2. **单元格内还要折行**:窄视口下长单元格要在列宽内**再折行**(4A 的词折行/CJK 禁则要按"列宽"而非"整行宽"跑),即一个**受限宽度的子布局**。
3. **网格/斑马/表头/边框 = 矩形装饰**:这些 4B 的矩形 quad 图元已能画(`FrameRect`),但要喂"每格/每列/每行的精确几何",依赖①②的产物。块冻结缓存也要按表格整体失效(AR10 不破)。

→ 一句话:**它不是"加个装饰",是给扁平文本管线补一条二维受限布局子路径**。规模、风险都明显高于 4B 其余项。

## 3. 选项

### A. 等宽网格(最省,Rust 侧可测)
表格整体改**等宽字体**,按列 max **显示宽**(CJK 计 2)用空格补齐对齐,再画斑马/表头/网格 rect。
- 优:纯 content.rs(Rust)可实现、可单测;无 JS reflow 风险;**列必对齐**。
- 劣:字形非 GitHub 比例体;超窄列不折行(或粗暴截断)。
- 适合:把「列对齐」这条 DoD 先稳稳拿下,字形观感暂让。

### B. 比例体实测两趟(最贴 GitHub,工程量大)
在 `layout-bridge.ts` 量每格 max-content → 两趟定列宽 + padding `6px/13px` + 边框;单元格内按列宽折行。
- 优:最贴 GitHub 结构/间距/配色。
- 劣:要扩 content→layout 契约带表结构;受限宽子布局 + 折行复用要重构 4A;风险高。
- 适合:作为 A 之后的升级,或表格观感成为硬需求时。

### C. 维持现状 `" │ "` 平铺(不做)
- 仅在表格极少出现时可接受;否则错位观感持续。

## 4. 决策

**方向:A → B 两步走**。

- **本期(Plan 4B 收口)**:**不做** A/B,表格维持现状 `" │ "` 平铺;4B 的矩形图元 + 引用条/Alert/hr/设计令牌**已交付**(见 [plan4_progress](../plan/plan4_progress.md))。
- **下一相位(独立)**:落 **A 等宽网格**作 v1——纯 Rust、可测、列必对齐、复用已就位的矩形图元画网格/斑马/表头。
- **B 比例体实测**列入升级项:仅当等宽网格的字形观感成为实测痛点时再起,且需先扩 content→layout 契约(带表结构)与受限宽子布局,单独评审。

**理由**:A 用最小风险拿下「列对齐 + 网格/斑马/表头」这条核心 DoD,且全程不碰跨界契约(AR10 安全);B 的契约扩张与折行重构不该夹在 4B 里仓促做。把表格从 4B 拆出独立成相位,是为了不让一个量级更大、有真实取舍的子系统拖累 4B 其余项的收口。

## 5. 影响

- 不影响已交付的 4B(矩形图元 / 引用条 / Alert / hr / 设计令牌)。
- content→layout 契约本期**不动**;A 落地时仅在 content.rs 内做等宽补齐(仍输出扁平 run),契约依旧不变——这也是选 A 作 v1 的关键好处。
- 设计令牌(0014 后续 A 用的网格线/斑马/表头色)并入 `crate::theme`(Plan 4B3 已建)。

## 6. 落地清单 — A 已落地(Plan 5,2026-06-15)

- [x] content.rs:`emit_table` 等宽网格——按列 max 显示宽(`display_width`,CJK/全角/emoji 计 2)空格补齐;新增角色 **`TableCell`/`TableHeader`**(末尾追加,as_u32=17/18),单元格间 `" │ "`(等宽 → 竖线列对齐)。单元格内联格式不解析(v1)。
- [x] layout-bridge `fontForRole`:17/18 → **MONO 同字重**(表头表体列对齐;光栅 `glyph-raster` 同走 `fontForRole`,自动一致)。
- [x] app.rs `block_decorations`:`TableHeader` y 范围 → **表头淡底 + 表头底线**;整表 y 范围 → 表尾外边线(复用 `FrameRect`)。
- [x] `theme.rs`:加 `TABLE_HEADER_BG`/`TABLE_RULE`;`glyph.wgsl::style_color` 加 17/18(表头略亮)。
- [x] 测试(content.rs):列对齐(每行首列等宽)、CJK 计 2 补齐、原始 `|`/`---` 不显形、首行 TableHeader 其余 TableCell。重放 `c06-table` 验流式列变宽。
- [x] **单元格内联格式 + per-列对齐 + 链接不泄漏**(5E.1 #1/#2/#3,2026-06-15):**additive 升级 vendored jcode** 表格建模——`Block` 加 `table_spans`(cell=span 序列)+ `table_align`(map pulldown `Alignment`),`table` 纯串保留向后兼容;`in_table` 时 End(Link) 不追加 ` (url)`(修 URL 泄漏)。`emit_table` 按 `align` 左/右/居中补空,`cell_role`:码→`Code`、粗/斜/链接文字→新角色 **`TableStrong`**(as_u32=19,**等宽加粗** → 保列对齐 + 区分);`layout-bridge`/`glyph.wgsl` 加 19。content 4 测。
- [~] **竖直网格线 / 斑马底**:未做(竖线由等宽 `│` 对齐替代;真网格/斑马需**列 x 坐标**,core 无 → 近 0014 B)。
- [x] **(升级 B,已落地 2026-06-15)** content→layout 带 `TableRegion` sidecar(run 区间 + 列对齐)+ JS `placeTable` 像素两趟 + `wrapRange` 单元格受限宽折行 + 列缩到 MINC。**端到端接通**:content.rs(`parse_markdown_tables`、emit_table 去补白)/ trait `layout(spans, tables, max_width)`/ wasm bridge `apply` 第4参 / layout-bridge.ts。**解决 CJK 像素对齐 + 字体跟随 + resize 折行塞下(任意字体)**;弃"限制字体 LXGW"。详见 [plan5 §5F](../plan/plan5-streaming-markdown.md)。
- [ ] (B 留尾)**连续竖直网格线**:需 JS 回传 `colX` 给 app 画全表高竖 rect(现:像素列间距 + 外框 + 行线 + 表头底,无 │);**删除线**装饰。

> v1 边界:等宽对齐 + **per-列对齐** + **单元格内联(等宽:码/强调)** + 链接只显文字 + 表头底/线 + `│` 竖线。**未做**:真竖直网格线 rect、斑马、删除线装饰、超宽列折行/比例体(属 B)。
