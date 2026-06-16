# Plan 6(markdown 全 SDF 化:装饰/容器统一成参数化 SDF 面板图元)

- **状态(2026-06-16)**:**6A + 6B 已落地**(SDF 面板图元 + storage buffer 数据通道 + 表格收编 + #5 真竖网格 + AO/圆角/表头底,端到端 WebGPU 工作)。**留尾**:WebGL2 兜底(data texture)、6C 收编其余装饰、6D 接 0016 过渡、6E 接 0019 揭示、6F 选中/hover/发光 + 截图回归。承接 [0018](../decision/0018-sdf-panel-decoration-primitive.md) + [onedraw §7](../research/onedraw-analysis.md)。
- 日期:2026-06-15
- 范围:把**当前一堆零散 `FrameRect` 画的块装饰**(代码块底 / 行内码 chip / 引用·Alert 左条+底 / hr / 标题线 / 表格框·网格·表头底)**全部收敛成一个参数化 SDF 面板图元**(`panel.wgsl`),数据驱动(小 storage buffer + 增量 `update`),并补上 #5 真竖直网格 + AO/圆角/选中。与 [0016](../decision/0016-streaming-morph-render-model.md) 过渡、[0019](../decision/0019-reveal-gating-and-choreography.md) 揭示同步。
- 前置:[0018](../decision/0018-sdf-panel-decoration-primitive.md)(方向 + 数据分档定调)/ [0011](../decision/0011-gpu-text-as-sdf-primitive.md)(文字 = SDF quad)/ 0016 / 0019 / 0014 + [plan5 §5F](plan5-streaming-markdown.md)(表格像素两趟,产 `colX/rowY`)/ `crates/render/src/shaders/rect.wgsl`(现圆角矩形 SDF,本图元的退化情形)。
- 相位:**6A 图元 + 数据通道 → 6B 表格满血 → 6C 收编所有块装饰 → 6D 接 0016 过渡 → 6E 接 0019 揭示 → 6F 效果入口 + 验证**;一相位 ≈ 一/数 PR,末尾过卡口。

## 0. 定位

"markdown 全 SDF 化"分两半:
- **文字半边**:glyph 已经是 SDF 图元(0011 单通道 SDF / 0015 MSDF;三源 + 回退在 [TODO K′](../../TODO.md))。**本 Plan 不碰文字管线**。
- **容器/装饰半边(= 本 Plan)**:现在用 `FrameRect`(平的实心/描边矩形)画的所有块装饰,迁成**一个参数化 fragment SDF 图元**——框/横竖网格/AO/圆角/底色/选中全在 shader 里几行 SDF 算出来,行列/选中只是参数。

两半合起来 = **整个 markdown 渲染面都由 SDF 图元产出**(字 = SDF glyph,容器 = SDF panel)。本 Plan 是作者"**后面很多效果都从这条路走**"(`design/thinking.md §4`)的落地起点:落成图元后,选中 / hover / 发光 / 容器特效都只是**加参数 + 几行 SDF**,不加图元类型。

**In**:`panel.wgsl`(rect.wgsl 升级)+ `PanelInstance` + 小 storage buffer 数据通道(照搬 OneDraw 命令+扁平参数+索引模型)+ IQ `sdRoundBox` + 网格/AO/选中 SDF + 收编全部 `FrameRect` 装饰 + 接 0016 过渡 + 接 0019 揭示 + WebGL2 兜底。
**Out(→ 后续/TODO2)**:文字管线本身(K′);tile binning + 层级 region + indirect 的"单 draw 画全部"(OneDraw 满血,长期,§见 onedraw §7 长期);3D/复杂光照;math/图片/mermaid 等 embed 容器(0007/0013,embed FSM 那条线)。

**铁律**:content→layout→render 契约不动(0001 §2.2)/ AR10 每帧一次跨界 / 小包体 / **文字不进 shader**(0018 §8:panel 只画容器,glyph 画字,共用同源 `colX/rowY`)/ **opinionated 单实现**。

---

## 约定:shader 复用许可(承 plan5)

- **shadertoy(CC-BY-NC)/ lygia(Prosperity 非商用)= 借手法、自写几行,不抄进发行物**。本 Plan 要的 `sdRoundBox` / `aastep` / 到网格线距离 / 软 AO 都是教科书级短函数,自写。AO 参考 shadertoy `NX23DV` 的 SDF→AO 思路但重写。
- **OneDraw(zlib)= 可借代码**:命令流 + 扁平 `draw_data` + 索引 的**数据模型直接照搬**(标注出处);IQ SDF 公式自写/借皆可(zlib 友好)。详见 [research/onedraw-analysis](../research/onedraw-analysis.md)。

---

## Phase 6A — 图元 + 数据通道(panel.wgsl 最小骨架)— ✅ 已落地(2026-06-16)

> 域:`render`(shader/scene/backend)+ `core`(frame 数据)。落 0018 §1/§2/§4 的图元本体 + 数据传输。

**任务**

- [x] **`crates/render/src/shaders/panel.wgsl`**:rect.wgsl 升级。fragment = `sd_round_box` + `fwidth` AA → 圆角外框 + 底色 + 表头底 + 横竖网格(从 storage 读占比)+ AO(内阴影)。naga 校验。
- [x] **`PanelInstance`**(scene.rs):`pos/size/radius` + 参数索引(`param_offset`/`param_len`)+ `flags`(grid/ao);`FrameRect` 并行保留(6C 删)。
- [x] **数据通道(小 storage buffer)**:`panel-params` storage buffer 装全部面板扁平参数(fill/line/header 色 + lineW/ao/headerRatio + nCols/nRows + colRatios/rowRatios);shader 按 `param_offset` 索引取;`write_buffer` 整写、容量不足才重建+重绑(增量改脏区可后续)。
- [~] **WebGL2 兜底**:storage buffer 在 WebGL2 无 SSBO → 后端按 `max_storage_buffers_per_shader_stage` **门控:不支持则面板管线 = None,降级不画**(WebGPU 主路工作)。**data-texture 兜底留后续**(0018 §2)。
- [x] **绘制顺序**:panel 在 rect 之前、glyph 之前(背景),同相机/裁剪/实例化。

**卡口**:`cargo fmt/clippy/test`(core+render 89,含 panel.wgsl naga)✓;`wasm-pack` + `tsc` ✓。**截图肉眼对留用户本机**(沙箱无 GPU)。

## Phase 6B — 表格满血(收编 + #5 真网格 + AO)— ✅ 已落地(2026-06-16)

> 域:`core`(app.block_decorations 表格段)+ `render`(panel fragment 网格/AO)+ web(colX 回传)。

**任务**

- [x] **网格参数同源 `colX/rowY`(逐表)**:`placeTable` 直接给出**整表面板几何** `{x,y,w,h,headerBottom,cols,rows}`(块内相对 px)→ `layout()` 收集**每个**表格(同块多表不再只取首个)→ 扁平 `tables` Float32Array → wasm `decode_table_panels` → `LayoutResult.table_panels` → `BlockCache.table_panels` → `block_decorations` **逐表**产一个 `FramePanel`(归一化 col/row/header_ratio)。字与框同源 colX → #5 竖线精确对齐。
  - **修(2026-06-16):同块多表合并 bug**。原实现从 glyph AABB 反推几何 + 把同块多表(如 `c06-all` 5 表共一 part)合并成一个巨框:行线跨满最宽表、仅首表头着色、竖线用首表列宽套全表 → 错位。改为 layout 逐表给精确几何、逐表一个面板。
- [x] **fragment 网格 + 表头底 + 圆角外框**:panel.wgsl `PANEL_GRID` 走 colRatios/rowRatios 画线,`header_ratio` 填表头底。
- [x] **AO**:`PANEL_AO` 到边距离内阴影(自写)。
- [x] **删表格 FrameRect**:`block_decorations` 表格分支改产 1 个 `FramePanel`(border + header + 行线 + 列线 + AO),去掉旧的 N 条 FrameRect + "竖线暂不画"的 #5 妥协注释。

**卡口**:卡口全绿;**重放 `c06*` 截图对(连续竖线/AO/圆角/CJK 对齐)留用户本机**。

## Phase 6C — 收编所有块装饰(FrameRect 退役)

> 域:`core`(app.block_decorations 全段)。把剩余装饰逐类迁到 panel 图元;`FrameRect` 仅余调试框(或一并迁,留作 6F 决定)。

**任务**(每类 = 一个参数化 panel)

- [ ] **代码块底**:圆角矩形底 → panel(`fillColor=CODE_BG, radius=6`);可顺手加极淡 AO/描边。
- [ ] **行内码 chip**:逐行聚合的圆角底 → panel(小圆角);保留现"同行连续延展、跨行 flush"聚合逻辑,只换产出。
- [ ] **引用 / Alert**:左条(细 panel)+ Alert 整块淡底(圆角 panel,类型色)→ 两个 panel 参数;颜色仍走 `theme::alert_bg/alert_bar/QUOTE_BAR`。
- [ ] **hr**:整宽细线 → panel(退化:无网格、极薄)。
- [ ] **标题线**:H1/H2 底部细线 → panel。
- [ ] **退役 FrameRect 装饰路径**:`block_decorations` 输出从 `Vec<FrameRect>` 改为 `Vec<PanelInstance>`(或并存期后删 FrameRect 装饰用法);`frame.rs`/`lib.rs` 相应调整。调试框去留单列(6F)。

**卡口**:卡口全绿;重放 `c01–c10` 全 case 截图与迁移前对拍(装饰位置/颜色不变,观感升级);确认无 FrameRect 装饰残留(调试框除外)。

## Phase 6D — 接 0016 过渡(框几何也补间)

> 域:`render`(panel 参数插值)+ `core`。streaming 中列宽/行高/块高随揭示变 → **字走 0016 morph,框的 `colRatios/box` 也要补间**,否则字滑框 snap(0018 §5)。

**任务**

- [ ] **路 B(v1 推荐,与 0016 CPU mix 同精神)**:CPU 每帧把 past→current 插值后的面板参数 `update` 进 storage buffer;面板过渡 = 0016 的"非 glyph 通道",节奏与 0016 对齐(同 t/ease)。
- [ ] **面板身份**:面板按块(`block_seq`)给稳定 id,供 past↔current 配对(类比 0016 NodeId,粒度到块/面板)。
- [ ] (留尾)**路 A**:shader 收 past+current 两套参数 + t 自插值(热点后升级)。

**卡口**:卡口全绿;重放表格增量长列 + 代码块增量长高:框跟字平滑长大、无 snap(截图逐帧/慢放 `?speed`)。

## Phase 6E — 接 0019 揭示(骨架先行)

> 域:`core`(reveal 调度器 → panel)。承 [0019](../decision/0019-reveal-gating-and-choreography.md):风格 2/3 的"先画框/网格、再填字" = panel 作为 **Frame/Grid stage** 先入场,glyph stage 随后。

**任务**

- [ ] **panel 作 Stage 产出物**:0019 调度器对 `Frame`/`Grid` selector 产出 panel 的 `(past,current,alpha)` 端点(经 6D 通道喂);`layout_gate` 满足(列宽定)即可精确画框。
- [ ] **骨架 alpha 入场**:框/网格先 fade-in(panel alpha),字 stage 延迟随后(0019 `offset_ms`)。
- [ ] **表格三风格联调**:原始/行框/全表三风格(0019 配置表三行)下 panel 入场时序正确;并入调试面板"风格"下拉。

**卡口**:卡口全绿;三风格重放观感符合预期(框先于字、节奏可调慢)。

## Phase 6F — 效果层入口 + 验证

> 域:`render` + `web`(调试器)。把"效果 = 参数"兑现,补回归。

**任务**

- [ ] **选中 / hover**:cell/块 index → SDF 子矩形 `sd_round_box` → 高亮/抬起/AO(纯参数,无新图元)。表格 cell 选中先行。
- [ ] **发光/容器特效**(可选,示范"加参数不加图元"):一个 glow 参数走通,证明效果层入口。
- [ ] **WebGL2 兜底验证**:data-texture/uniform 路与 WebGPU 路观感一致(无 SSBO 环境)。
- [ ] **截图快照回归(5D4)**:全 case panel 化前后对拍纳入 `?verify` 黄金样张;面板参数边界(超列上限截断、半截表)用例。

**卡口**:卡口全绿;`?verify` 通过;WebGL2/WebGPU 双路一致。

---

## 风险 / 取舍

- **变长数据**:行列分隔变长,塞不进定长 vertex 属性 → 必走 storage buffer(0018 §2)；WebGL2 无 SSBO → data texture 兜底是必做项,不是可选。
- **overdraw**:单 panel quad 内每像素 O(cols) 网格循环 → 需 tight quad 防 overdraw(表格列少,够;海量装饰再上 OneDraw tile binning,长期)。
- **与 6D/6E 的耦合顺序**:6B/6C 可在静态(冻结块)下先跑通观感;6D(过渡)、6E(揭示)依赖 0016/0019,可在其就绪后接——故 6A–6C 不阻塞于 0019。
- **FrameRect 退役**:调试框(4C3)是否一并迁 panel 待定(6F);保留 FrameRect 作调试专用也可,避免调试器耦合效果层。

## 与 ADR 的对应

| 相位 | 兑现 |
|---|---|
| 6A | 0018 §1/§2/§4(图元 + 小 storage buffer + AA);onedraw §7 短期路线 |
| 6B | 0018 §3/§4 + 5E.1 #5(真竖网格)+ §1 同源 colX/rowY |
| 6C | 0018 §6(装饰层收敛 —— ADR 真正价值)+ 0011 装饰=SDF quad 收口 |
| 6D | 0018 §5(框几何随 0016 补间) |
| 6E | 0019(gate×choreography:骨架先行)+ 0017 §10 |
| 6F | 0018 §3(选中/hover/发光=参数)+ `design/thinking.md §4`(效果层入口) |

> 长期(本 Plan 之外):装饰/效果爆炸后,照 OneDraw **tile binning + 层级 region + predicate/scan + indirect** 做"单 draw call 画全部装饰/效果",WebGL2 留实例化兜底(onedraw §7 长期)。
