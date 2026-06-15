# Plan 5 进度(streaming markdown 还原)— 2026-06-15

> 对 [plan5-streaming-markdown](./plan5-streaming-markdown.md) 的落地总结。Plan 5 远超原定四相位,衍生出 3 篇 ADR(0016/0017/0018)+ 2 篇设计(thinking §3/§4)+ 1 篇研究(onedraw)。未做项见末尾「→ TODO」。

## 总览(一句话)

streaming markdown 的**机制层(0016)+ 驱动层(0017)+ 重放验证(5D)+ 真表格(0014 B 像素两趟)**已落地;**reveal 节奏自主(北极星)+ SDF 面板效果层(0018)+ 非表格语法完备**留作后续。

## 已落地

### 5A 机制层 = [0016](../decision/0016-streaming-morph-render-model.md) ✅
- `render/morph.rs`:retained keyed `Scene`(`NodeId(block_seq,glyph_idx)` + `RenderNode{current,past:Option,t_start,phase}`)+ `commit` join(**past 取显示态防回跳**)+ `instances` 线性插值(CPU mix 路 B)。
- `FrameGlyph` 带稳定 id;静止/冻结旁路零成本;7 个单测。
- v1 范围:几何(pos/size)补间;alpha 入场仍走 spawn_time。

### 5B/5C 驱动层 = [0017](../decision/0017-markdown-streaming-landing.md) ✅
- 活动区 = 最后未闭合块(= 0005 块冻结);每 tick 重解析活动块(pulldown 原样 = 保守预测)→ 经稳定 id 喂 Scene。
- **raw 抑制**(`content.rs::is_pending_table`):成形中表格 hold,不闪 `| a | b |`。
- 全 markdown 构造的 streaming 行为表(5C 规格表)。

### 5D 重放验证 ✅
- 复用 core `Player`/`Record`;`?replay=<case>` 喂预录 text-delta(模拟 SSE,不连 opencode)。
- 调试面板:**case 下拉 + speed 下拉**(localStorage 持久化,reload 生效)+ `↻` 重跑 + `?verify` 标尺。
- case `c01–c10` + 表格族 `c06 / c06b–f / c06-all`(覆盖对齐/CJK/内联/残缺/宽表)。

### 5E/5F 真表格(0014 A → **B 像素两趟**)✅
- **A(等宽)**:emit_table 等宽补齐 + 角色(TableCell/Header/Strong/Em/Sep)+ 装饰(表头底/行线/外框)。
- **升级**:per-列对齐(jcode `table_align`)、单元格内联(`table_spans` → 富 span)、链接 text-only 不漏 URL、italic→`TableEm`。
- **B 像素两趟(端到端接通,本会话)**:`parse_markdown_tables → (spans, TableRegion)` → `LayoutEngine::layout(spans, tables, max_width)`(seam/support/app-test 三 impl)→ wasm bridge `apply` 第4参 → `layout-bridge.placeTable` 像素两趟 + `wrapRange` **格内折行** + 列缩到 MINC。
  - **解决**:#1 对齐 / #2 内联 / #3 链接 / #4 raw 抑制 / **#7 CJK 像素对齐 / #8 字体跟随切换 / #2·#6 resize 折行塞下 / #9 表头底色填满整行**。
- 边界:**去 `│`**,现表格 = 像素列间距 + 外框 + 行线 + 表头底。

### 衍生(本会话沉淀)
- **设计**:`design/thinking.md §3`(★ markdown 节奏自主 / 阅读体验优先)、`§4`(★ SDF 效果层底座)。
- **ADR [0018](../decision/0018-sdf-panel-decoration-primitive.md)**:SDF 装饰/面板图元(参数化 shader 框 + 共享 storage buffer)——表格框/网格/AO/选中的去向。
- **研究** `research/onedraw-analysis.md`(zlib 可借的 GPU-driven SDF 渲染器,0018 满血参考)。
- 约定:lygia 非商用 = 参考不抄;TODO T 加 troika/drei 真字体度量参考;opencode TUI 对照(remend text-only + 终端 2:1 对齐)。

## 卡口(2026-06-15,全绿)

- `cargo fmt --check` ✓ / `cargo clippy --workspace --all-targets -D warnings` ✓(native + `--target wasm32`)。
- `cargo test --workspace` ✓:**88 测**(含 morph 7 + 表格 0014 B 结构测);`jcode-render-core` 19 测 ✓(additive `table_spans`/`table_align`)。
- `wasm-pack build --target web` ✓ / `cd web && npx tsc` ✓。
- 表格测试随 0014 B 改:验 `parse_markdown_tables` 的 `TableRegion`(run 区间 + 对齐)结构,而非旧的 `│` 字符。

## 未做 → 见 [TODO「Plan 5 续」](../../TODO.md)

- **reveal 节奏自主**(0017 §10 北极星):reveal 调度器(节奏与 token 解耦/限速/可放慢)+ 骨架先行(表头框→填字)+ **非表格结构块 raw 抑制**(列表/围栏/公式/图片/链接)。
- **#5 真竖直网格 + AO + 选中** → [0018](../decision/0018-sdf-panel-decoration-primitive.md) SDF 面板图元(panel.wgsl + storage buffer);**收编所有块装饰**到该图元。
- **非表格 markdown 完备**:列表渲染质量(有序/嵌套/任务 `- [ ]`/松紧)、删除线渲染、代码块语法高亮、多级引用;**补非表格 case**(嵌套/有序/任务列表、围栏语言、转义、自动链接、脚注)。
- **0016 留尾**:exit 淡出、GPU 双态(路 A)、policy 层(ease/dur 大表)、settle 后移出 Scene 内存优化。
- **垂直度量**(TODO T):真字体度量替代 measureText(行高/baseline/kerning,参考 troika)。
- 截图快照回归(5D4);`?verify` 黄金样张(TODO V)。
- 嵌入类(math/图片/mermaid = O)、可点链接(Q)、自定义标签(P)—— 原 TODO 既有。
