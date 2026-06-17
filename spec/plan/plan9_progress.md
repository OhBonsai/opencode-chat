# Plan 9 进度(递归揭示:节点树上的块内时序 —— 方案 A)

- 状态(2026-06-17):**已落地(v1,9.0/9A/9B/9C/9D/9F + 9E 卡口)**。揭示从"全局 tier 队列"改为
  **嵌套集上的递归排程**(方案 A:文档序 + 每容器 ordering + Full 表就绪门)。全卡口绿
  (native+wasm clippy 0、108 core 测试、wasm-pack、tsc)。
- 一句话:`resolve_tree` 在 0020 节点树上 DFS——**tier = 顶层块文档序**(块间自上而下、不抢位),
  **delay_ms = 每容器 ordering 累加的编排**(骨架先行 / 逐项 / 逐行 / 逐字);表格 3 风格 = Table/
  TableRow 的 ordering 预设;Full 表加**内容门**(等闭合才揭)。

## 各相位落地

| 相位 | 落地 | 文件 |
|---|---|---|
| **9.0** NodeKind 补全 | `NodeKind` 增 `MathDisplay`/`ThematicBreak`/`HtmlBlock`;`block_node_kind` 逐一映射(去 `_=>Paragraph`);`is_nodespawn`(无字块=ThematicBreak/Embed) | `nodes.rs`、`content.rs` |
| **9A** 递归地基 | `resolve_tree` + `resolve_node`(DFS,文档序 tier,delay 累加,叶 glyph 锚、无字块 NodeSpawn);**删** Plan 8 `resolve`/`resolve_doc`/`resolve_into` + `RevealStyle`/`Stage`/`Selector`/`Dep`/`EaseId` + `table_style`/`text_style`/`skeleton_style`(0→1 不留并行旧路);`schedule` 改调 `resolve_tree` | `reveal.rs`、`app.rs` |
| **9B** ordering 数据 | `Ordering{Sequential, SkeletonThenChildren}` + `ordering_for(kind, table)`(每 NodeKind 默认 + 表格 3 预设) | `reveal.rs` |
| **9C** 骨架接入 | 复用既有:`block_decorations` 的 `FramePanel.reveal` 由该表已释放 cell 驱动(整表满框 / 行框逐行长大);代码/引用骨架 = 即时 `FrameRect` 底 + 字带 `SKELETON_LEAD` 延迟(底先于字) | `app.rs`、`frame.rs`、`panel.wgsl` |
| **9D** 列表/嵌套 | List/ListItem `Sequential{ITEM_GAP}`,嵌套 List 递归 → **逐层逐项深度优先** | `reveal.rs` |
| **9F** 就绪门 + 合成 | `resolve_tree(open_block)`:Full 表在"仍流入的末块"时整体 hold(`tier=MAX`),闭合(后续块到达 / turn 收尾)才揭——`spawn=max(内容门, 编排)` 的内容门下限;行框/原始不受门 | `reveal.rs`、`app.rs` |
| **9E** 卡口 + cases | g-table / g-nest / g-mixed / g-choreo 已建并进 `?debug` 下拉(人工验);native 渲染数据回归锁定文档序/骨架/门 | `web/public/cases/g-*.json`、`reveal.rs`/`app.rs` 测试 |

## 验收(DoD §8)对照

1. **块间文档序**:✓ tier = 顶层块文档序 → 靠后块绝不抢到靠前块前(`resolve_tree_tiers_are_document_order_no_cross_block_jump`)。
2. **块内嵌套**:✓ 列表逐项 / 嵌套逐层(`resolve_tree_nested_list_depth_first_doc_order`)、表格按预设(整表骨架→表头→body)。
3. **隔离**:✓ 表格风格只改 Table/TableRow ordering,段落/列表/代码各自 ordering 不动。
4. **无拖累**:✓ 文档序 tier + 共享配额按位置消费(同 `schedule`,块内不跨块抢)。
5. **就绪门**:✓ Full 表等闭合才揭(`resolve_tree_full_table_held_while_open`);行框到行即起。**部分**:逐行/逐格 的 layout 门(列宽就绪)见"取舍"。
6. **编排压过内容**:✓ `spawn=释放时刻+delay_ms`,内容瞬到时仍按 delay 编排(g-choreo 用 transport 看)。
7. **块 kind 完备自洽**:✓ 分隔线/公式/HTML 独立 NodeKind;无字块 `is_nodespawn`;补 kind 只增 `ordering_for`/`block_node_kind` 查找行,递归管线不改。
8. 确定性可重放 / 纯文本不回归(text ordering=`Sequential{0}` 零延迟)/ 限速·放慢·transport 不变 / 卡口全绿:✓。

## v1 取舍(留后续)

- **layout 就绪门(列宽)**:9F 现只接**内容门**(整表等闭合)。`layout_gate(table_panels)` 已备但未接进 `resolve_tree`——列宽来自 JS 像素两趟(native 无 `table_panels`),接进去会让 native 永久 hold 且无法本地验证。逐行/逐格的"几何稳定才画框"留后续(与 0019 §5 双门的布局门一支同源)。
- **方案 B(真并行 cell / 各块独立时间轴)**:本 plan 只做 A;整表"cell 并行"现用 `CELL_GAP` 小 gap 近似(肉眼接近并行),真并行随方案 B。
- **Embed 跨层 web 淡入(× 0022)**:`is_nodespawn(Embed)` + NodeSpawn 接口已留;DOM 叠加层的 CSS 淡入待 0022。
- **节奏美学**(rap flow 微定时,research/reveal-rhythm):正交,后接。
- **HtmlBlock**:NodeKind + 映射已备,但 vendored jcode 当前不产 `BlockKind::Html`(当段落),故实际不出现。

## 卡口命令

```
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy --workspace --target wasm32-unknown-unknown -- -D warnings
cargo test --workspace
(cd crates/wasm && wasm-pack build --target web --out-dir ../../web/pkg)
(cd web && npx tsc)
```

## 浏览器人工验(9E)

`?debug` 选 g-table / g-nest / g-mixed / g-choreo;表格风格下拉切 整表/行框/原始;**内容门**看实时重放(回放 0.1×~0.25×),**编排**用 transport 播放器拖 scrubber。逐 case 对照 plan9 §9 表的 "✅" 列:块间自上而下、块内各自编排、切风格只动表格、整表等闭合、行框逐行、编排压过内容、无跨块拖累。
