# TODO —— 产品相位 backlog(Plan 3 K/L 之后)

- **近期可做项已拆出** → [plan4-polish](spec/plan/plan4-polish.md)(排版收口 + markdown 观感 + 基础调试器):含原 M(折行/分级/装饰/表格/令牌/pretext 清理)、调试器 P1 + 数据通道、per-role 度量。
- **streaming 形变** → [plan5-streaming-markdown](spec/plan/plan5-streaming-markdown.md)(0016 机制 + 0017 落地 + 全 markdown 语法 streaming 规则 + 重放验证 case)。
- **markdown 全 SDF 化** → [plan6-markdown-all-sdf](spec/plan/plan6-markdown-all-sdf.md)([0018] 落地:零散 FrameRect 装饰 → 一个参数化 SDF 面板图元 + storage buffer + #5 网格/AO/选中,接 0016/0019)。
- **★ reveal 节奏自主(北极星)** → [plan8-reveal-cadence](spec/plan/plan8-reveal-cadence.md):落地 [0019] gate×choreography——就绪门(双门)+ `RevealStyle` 数据(Selector over 0020 节点树)+ 调度器(节奏与 token 解耦/限速/可放慢)+ 骨架先行 + 全结构块 raw 抑制 + 表格 3 风格。建在 Plan 7(0020)之上;喂 0016/PanelScene;调度器为唯一揭示路径(收编即时 spawn)。**已落地 v1**。
  - **节奏"美感"升级(借 rap flow / groove / 语音韵律)** → [research/reveal-rhythm](spec/research/reveal-rhythm.md) + `design/thinking.md §5`:把匀速 cps 换成**节奏函数**(边界停顿 ∝ 节点层级 + 末延长 + 重音拍 + seeded 1/f 微定时),接进 Plan 8 调度器。大杠杆=结构性停顿(助读)。**思考中,未排期**。
- **内容节点树地基** → [plan7-content-node-tree](spec/plan/plan7-content-node-tree.md):**专注落地 [0020]** ——扁平 `Vec<Node>`(嵌套区间+parent+key)sidecar,收编 glyph(0016)/TableRegion(0014)/FramePanel(6D)身份,暴露查询 API,留下游接口(`Embed` kind 占位)。**简约 v1**(不上路径哈希/Taffy/任何消费者)。**下游各自后续**:reveal(0019)、Taffy(0023)、DOM embed(0022)、节点级 morph(0016 留尾)。
- **内容节点身份(scene-graph-lite)** → [0020](spec/decision/0020-content-node-identity-model.md):扁平 `Node` 表(嵌套区间 + parent 下标 + 路径哈希),统一 0016 NodeId / 0014 TableRegion / 0019 Selector;与 tile 分桶正交。**0019 reveal + 0016 节点级 morph 的前置**。
- **JS/Rust 边界 + 可配置渲染样式** → [0021](spec/decision/0021-js-rust-boundary-and-configurable-render.md):渲染样式全数据驱动(`Palette` 角色配色 + `RenderStyle`,shader 去写死、从 buffer 取)、布局/渲染两条生效路径固化、**pretext 作复杂 layout 预留**(契约固定可热替换)。落地清单见 0021 §8。`TableStyle`(Plan 6)是样板。
- **Taffy 盒子布局(chat 内容/页面)** → [0023](spec/decision/0023-taffy-box-layout.md):chat 内容是盒子树 → 用 **Taffy**(Zed/GPUI 同款,纯 Rust block/flex/grid)over [0020] 节点树;**叶子 measure 接 measureText(JS,护城河)/placeTable/reportSize**;Taffy(wasm)+ measure 回调 + 缓存;输出喂 0016 形变(reflow 不跳变);收编 build_frame 块堆叠 + placeTable 外层。落地清单见 0023 §10。**未排期**(chat 页面期做,与 0020 同期)。
- **DOM 叠加层(HTML 逃生口)** → [0022](spec/decision/0022-dom-overlay-layer.md)(调研底稿 [research/dom-overlay-layer](spec/research/dom-overlay-layer.md)):canvas 画不了整个 HTML 生态 → 在画布上叠**与相机同步的绝对定位 DOM box**(`NodeKind::Embed` 留位 + `world_to_screen`+scale 随缩放/平移 + light/dark 主题联动 + 事件钩子 + 尺寸回报重排)。范本 = drei `<Html>`/CSS3DRenderer/deck.gl。与"栅格化进 canvas"(0007/0013/TODO O)正交分流;接 0016(reflow 平滑)/0020(id)/0021(主题源)。落地清单见 0022 §9。**未排期**,面向少量交互/富媒体嵌入。
  - **⏳ 排期:待「非表格 markdown 全支持」之后再做**(见下「Plan 5 续 · 非表格 markdown 完备」)。它不阻塞 markdown 语法补全,反而该在语法齐全、节点种类稳定后一次性把 jcode `Document` 拍平时记区间建 `NodeTree`(0020 §9)。在此之前 0016/0014/0019 继续用现有 `(block_seq,glyph_idx)` / `TableRegion`。
- **愿景/上限层** → [TODO2](TODO2.md):效果系统(原 N)、画布产品化、极致规模、交互深度。
- 本文件 = **剩余产品相位 + Plan 2 欠账 + 决策锚点**;一条 ≈ 一个 Phase/PR,完成后上提到正式 plan。

---

## 表格遗留(2026-06-16;A 删除线已修,其余排队)

> 表格主体已落地(对齐/内联/链接 text-only/CJK 像素对齐/字体跟随/真竖网格 per-table 面板/AO/垂直对齐/实时 TableStyle setter)。删除线(A)已修。以下未做:

- **B 斑马纹 / 行底交替**:偶数数据行淡底,提升宽表可读。走 `TableStyle`(加 `zebra_fill`)+ `panel.wgsl`(按 row band 判奇偶填色)+ style 面板控件。实时、不重排。
- **C 宽表横向滚动(局部)**:超宽表在**表格内**横向滚,而非整画布 pan。**最重**,需:每表横向 `scrollX` 状态 + 表格框**裁剪**(wgpu scissor 分批 / 或 shader 按框 clip)+ 输入命中(指针/滚轮落在表格区 → 滚表不滚画布)。需单独设计(可能开 ADR / plan6 §)。
- **D 单元格选中 / hover 高亮**:cell index → SDF 子矩形高亮(0018 §3 / Plan 6F),纯参数无新图元。
- **E streaming 表格 reflow 不跳变 ✅(2026-06-16,Plan 6D)**:字本就走 0016 `Scene` 补间;新增并列 `PanelScene`(同 dur)给框/网格补间——`FramePanel.id=(block_seq,表序号)`,`submit` 提交面板几何 join,发射插值后的 box/header/col/row(col 逐元素、row 补前缀+新行出现)。色/AO/线宽不补间。留尾:exit 淡出(同 0016 路 A)。
- **F 骨架先行揭示**:先画框→再填字、可放慢(0017 §10 / 6E;排期在「非表格 markdown 全支持」后,与 [0019]/[0020] 同期)。
- **G 比例体强调(待核实)**:表格 cell 的 `**粗**`/`*斜*` 当前走等宽角色(TableStrong/TableEm + MONO 字体);B 像素两趟后已不要求等宽,可改跟随预设比例字体。先核实 `layout-bridge.fontForRole` 对表格角色是否仍强制 MONO,再定改不改。

---

## Plan 5 续(streaming markdown 未做项;总结见 [plan5_progress](spec/plan/plan5_progress.md))

> Plan 5 已落地:机制 [0016] + 驱动 [0017] + 重放 5D + 真表格 [0014 B](像素两趟/CJK 对齐/字体跟随/resize 折行)。以下为未做,按价值排:

- **★ reveal 节奏自主([0017 §10](spec/decision/0017-markdown-streaming-landing.md) 北极星;形式化 [0019](spec/decision/0019-reveal-gating-and-choreography.md))**:按 [0019] 四层(gate 就绪门 / plan 风格数据 / 调度器 / 0016 机制)实现——`RevealUnit` + `gate` 谓词 + 双门(内容/布局)+ `RevealStyle` 数据 + reveal 调度器(节奏与 token 解耦 / 限速 / **可刻意放慢**)+ **骨架先行**(表头框→填字,框走 [0018])+ **非表格结构块 raw 抑制**;表格三风格(原始/行框/全表)= 配置表三行。设计 `design/thinking.md §1/§3`。
- **★ SDF 面板/装饰图元([0018](spec/decision/0018-sdf-panel-decoration-primitive.md)) → 已拆成 [Plan 6](spec/plan/plan6-markdown-all-sdf.md)**:`panel.wgsl` + 小 storage buffer(命令+扁平参数,**照搬 onedraw 数据模型**,见 [research/onedraw-analysis](spec/research/onedraw-analysis.md))→ **#5 真竖直网格 + AO + 圆角 + 选中/hover**;再**收编所有块装饰**(代码块底/引用/Alert)。6A 图元+数据通道 → 6B 表格满血 → 6C 收编全部装饰 → 6D 接 0016 → 6E 接 0019 → 6F 效果入口。设计 `design/thinking.md §4`。
- **非表格 markdown 渲染质量**:列表(有序/嵌套/任务 `- [ ]`/松紧)、多级引用;代码块**语法高亮**(并 H5)。(**删除线 ✅ 2026-06-16**:`StyledSpan.strike` → 字中线 FrameRect,正文+表格通用。)
- **非表格重放 case 补全(5D)**:嵌套/有序/任务列表、围栏语言标注、转义、自动链接、脚注。
- **[0016] 机制留尾**:exit 淡出、GPU 双态(路 A)、policy 层(ease/dur 大表)、settle 后移出 Scene 内存优化。
- 截图快照回归(5D4);`?verify` 黄金样张(并 [V](#v--组件内观感验证视图opinionated非兼容性测试))。

---

## 产品相位(依赖 L 的图元管线 / plan4 的装饰与调试器)

### K′ — 双模/三源文字图元:位图(默认)+ SDF(特效)+ 离线 MSDF(文字当图片)

> 现状代码是初版"全 SDF";本相位落地三源 + 回退链 + 调试器切换。**完整方案见 [0015](spec/decision/0015-glyph-source-fallback.md)**;背景/性能账见 [0011 §3.5](spec/decision/0011-gpu-text-as-sdf-primitive.md)。

- [ ] **实例加 `kind: u32`**(0=位图覆盖率, 1=单通道 SDF, 2=MSDF, 3=RGBA emoji);片元按 kind 分支:bitmap `cov=tex.r` / SDF `smoothstep` / MSDF `median(r,g,b)` 再 smoothstep / RGBA 直采。
- [ ] **atlas 分页**:MSDF baked = 静态 RGB 页;运行时 R8(位图/TinySDF)动态页 + LRU;RGBA emoji 页。`layer` 选页、`kind` 选采样。
- [ ] **源解析器 + 回退链**(0015 §2.2):`Bitmap 模式 → 位图`;`SDF 模式 → MSDF 命中 ? MSDF : TinySDF(回退)`;emoji→RGBA。
- [ ] **离线 MSDF(LXGW 常用字)**:`msdf-atlas-gen` 烘 `lxgw-wenkai-v1.522/LXGWWenKaiMono-Light.ttf` 常用字集(ASCII+~3500 汉字)→ `lxgw-msdf.png(RGB)+json(metrics+coverage)`,放 `web/public/` 懒加载;coverage 建 Set 供 O(1) 判命中。
- [ ] **metrics 一致**(0015 §2.5):回退 TinySDF **也用 LXGW @font-face(子集 woff2)**光栅,advance/字形与 MSDF 同源;正文字体统一 LXGW。
- [ ] **调试器切换**(0015 §2.6 / 0012):`set_glyph_mode(Auto/Bitmap/ForceTinySDF/ForceMSDF)`;`FrameStats` 加逐源计数 {msdf/tinysdf/bitmap/rgba} → 面板看 MSDF 命中率调烘集。

### O — 嵌入块(图片 → mermaid → math → 卡片)
- [ ] 图片:浏览器解码 → 纹理 quad;mermaid:SVG → 浏览器光栅 → 纹理
- [ ] **math(LaTeX)**:`$$`→块、`$`→行内带 baseline 盒;v1 走 KaTeX/MathJax→SVG→纹理(embed,懒加载);货币防误判已白嫖(jcode `escape_currency_dollars`)。升级"数学一等图元/可动画"→ RaTeX 进 quad 管线。详见 [0013](spec/decision/0013-math-latex-rendering.md)。
- [ ] embed FSM:Placeholder → Loading → Ready → Failed;占位高度防 reflow;像素对齐
- [ ] wasm 只持元数据(尺寸/位置),重活交浏览器
- 参考:[0004 §7](spec/decision/0004-markdown-and-embeds.md)、[0007](spec/decision/0007-rich-media-embeds.md)、[0013](spec/decision/0013-math-latex-rendering.md)

### P — 标签层 + 自定义语法
- [ ] pre-markdown segmenter + 标签注册表(hold 区、未知标签默认 Literal)
- [ ] `:::` 容器开启符(0006 §5.1);`<thinking>`/citation 区域 FSM
- [ ] 行内 chip:`@提及` / 引用角标(parse 后 span 后处理,0006 §5.2);`[^1]` 用 pulldown `ENABLE_FOOTNOTES`
- [ ] 安全:标签当数据,绝不当 HTML 执行
- 参考:[0006](spec/decision/0006-inline-tags-and-extensibility.md)、[0010 §5.1](spec/decision/0010-markdown-parsing-strategy.md)

### Q — input / 选区 / hit-test / 可点链接
- [ ] **CPU 基础盒模型**(0011 §3.3④)做命中/选区/复制——不回读 GPU、不用正在动的 SDF
- [ ] 可点超链接(借 warp `hyperlink + Action`,0010 §5);脚注/引用跳转
- [ ] 选区跨折行、复制保真
- 参考:[0011 §3.3④](spec/decision/0011-gpu-text-as-sdf-primitive.md)、[0010](spec/decision/0010-markdown-parsing-strategy.md)

### R — 无障碍 DOM 镜像 + 渲染降级
- [ ] 可见内容 **DOM 镜像**(屏幕阅读器)——**可嵌入组件硬需求,别拖到最后**;兼作"无 WebGPU 也无 WebGL2"极端兜底
- [ ] **WebGL2 路专测**(已通过 `Backends::GL` 启用、自动兜底,未测);处理其限制:无 compute → 逐字 compute 特效降级 vertex+fragment(0011 §3.4)
- [ ] **Canvas2D 不做**(`RenderBackend` trait 留缝但不实现)
- 参考:[0003 §5](spec/decision/0003-fault-tolerance.md)、[0011 §3.4](spec/decision/0011-gpu-text-as-sdf-primitive.md)

### S — 公共 API + React/Vue 封装 + npm 打包
- [ ] 命令式 API / props / 事件 / 主题(`api` 模块)
- [ ] React、Vue 薄封装;`npm i` 即用
- [ ] **产物体积守门**(守"轻包体"原则)
- 参考:[0000](spec/decision/0000-overview.md)、README「交付形态」

### T — 字形垂直度量 / baseline(textMetrics 收口)

> 拆自 [0015 §2.5](spec/decision/0015-glyph-source-fallback.md) / [plan4_progress §7.5](spec/plan/plan4_progress.md):**水平 advance 已收口**(MSDF baked xadvance);**垂直度量(baseline / 行盒 / ascent-descent / 盒对齐)是独立工作面,且预判高频踩坑,单列**。范围仍 = 中英文(LTR),非通用排版。
>
> **现状(实现位置,2026-06-16 核)**:全在 `web/src/layout-bridge.ts`(measureText 路)——`FONT_SIZE=16×dpr`、`LINE_HEIGHT=ceil(FONT_SIZE×1.4)`(**硬编码**)、`roleScale`(标题倍率)、`advanceFor`(measureText 宽×scale,**无 letter-spacing**);水平 align = 表格 per-列(`placeTable` + `style-config.table.hAlign`),vAlign = 表格(`style-config.table.vAlign`)。**正文无 justify、无 letter-spacing 旋钮、无真 ascent/descent/baseline。**
> **已做(临时止血,2026-06-16)**:行内垂直 `(g.lineH-g.cell)/2` 把 glyph 在行高里居中(修"文字贴顶偏上",`layout-bridge.place` + 表格内放置)——是近似,非真 baseline。

- [ ] **MSDF baseline 真机校验**:`msdf_instance` 已用 baked `yoffset`(lib.rs ~203),真机看偏高/低 → 调竖直项(单旋钮)。
- [ ] **三源基线统一**:Canvas2D `textBaseline` 光栅(位图/TinySDF)与 SDF tile 内字模位置 + MSDF baked 盒,落同一基准(否则切源跳动)。
- [ ] **中英混排同基线**:西文 x-height/descender vs CJK 全角盒,坐同一基线不错层。
- [ ] **行盒来源统一**:现 `LINE_HEIGHT = 1.4×` 硬编码;ascent/descent/行高统一来源,避免不同 role(标题大字/行内码 chip/引用)行高跳动。
- [ ] **★ 捷径:LXGW/MSDF 路直接用 baked 度量**:`scripts/bake-msdf.mjs` 产的 BMFont json **已含** `common.lineHeight / base / 逐字 yoffset/xadvance`——"真字体度量"对 LXGW 路已现成。先用它把**行高/baseline 从常数 1.4 换成真值**(troika 同源思路);系统字体路退而求其次仍近似。**不引 troika**(绑 three.js,与 0001/0021 护城河冲突),只借手法。
- [ ] **style 面板旋钮(快赢,先手调观感)**:`lineHeight 系数 / letterSpacing / 正文 vAlign(top/center/bottom)` 接进 `style-config` + 面板(走 layout,refresh 重排),不必等真度量收口。
- [ ] **盒对齐**:行内码 chip / 标题 / Alert 标签 / 上下标的竖直居中与基线锚点。
- [ ] **math 行内盒 baseline**(O 的 `$…$` 依赖,见 0013)。
- [ ] **用真实字体度量替代 measureText 近似**(行高 / 字高 / baseline / 对齐 / 字距 kerning):现在只有 Canvas2D `measureText`(仅 advance 宽)+ 硬编码 `LINE_HEIGHT=1.4` + 方形 cell,**无 ascent/descent/cap-height/baseline、无 kerning、对齐靠近似**。
  - **参考 `troika-three-text`**(`./drei` 的 `<Text>` 即其薄封装:`src/core/Text.tsx` → `troika-three-text` 的 `TextMeshImpl`/`getTextRenderInfo`):用 **Typr 读字体表**拿 units-per-em / ascent / descent / cap-height / line-gap → **baseline 精确定位 + 行高**;支持 `letterSpacing`、`textAlign`、**锚点 `anchorX`/`anchorY`**(top/middle/baseline/bottom)、`maxWidth`+`overflowWrap`/`whiteSpace` 折行、`sdfGlyphSize` SDF。
  - 落点:把"字体度量真值"引入 layout-bridge(读 woff/ttf 表或借 troika 思路),让 advance/baseline/行高/对齐都来自字体,而非 measureText + 常数。与 [TODO V] 观感验证一起收。
- 参考:[0015 §2.5](spec/decision/0015-glyph-source-fallback.md)、[0013](spec/decision/0013-math-latex-rendering.md)、[troika-three-text](https://protectwise.github.io/troika/troika-three-text/) / drei `<Text>`([docs](https://drei.docs.pmnd.rs/abstractions/text),源码 `./drei/src/core/Text.tsx`)。

### V — 组件内「观感验证」视图(opinionated;非兼容性测试)

> **定位**:不追排版兼容性/能力,只锁定"**本作者认可的那一种实现**"的观感不回退。**范围 = 中英文 + markdown,仅此一条渲染路径**(↔ 已决策「opinionated 单实现」)。

- [ ] **内置黄金样张**:一份固定中英 markdown(标题 H1–H6 / 列表 / 引用 / 代码块 / 行内码 / Alert / 链接 / CJK 标点 / 中英混排),`?verify` 一键渲染。
- [ ] **标尺叠加**:复用 4C3 自绘几何画 baseline / 行盒 / 字盒,肉眼或截图比对"作者认可"的基准。
- [ ] **截图快照回归**:本地/CI 存一张参考图,改动后 diff(像素/感知),只守"这一种观感"不回退。
- [ ] **明确非目标**:不与浏览器/GitHub 逐像素对齐;不测 BiDi/复杂脚本;不测非 markdown 输入;不做多字体兼容矩阵。
- 参考:[T](#t--字形垂直度量--baselinetextmetrics-收口)(验证主要盯垂直度量)、[0012](spec/decision/0012-debugger-gui-html-vs-egui.md)(自绘几何复用)。

---

## SDF 动画系统([0025](spec/decision/0025-sdf-node-animation-system.md) / [Plan 10](spec/plan/plan10-sdf-animation-system.md))

- [x] 相位 1–2:进场缓动 alpha + scale-in(`glyph.wgsl`,全局 baked)。
- [x] 相位 3a/3b:per-element / per-instance profile(`anim` id 贯穿 `FrameGlyph→Sample→GpuInstance→shader`;core `enter_profile_id` 按 role + reveal 风格选,shader `enter_profile_by_id` 查表)。
- [ ] **相位 4:面板 smin / mix 形变**(仅**解析场** = 面板/装饰;字是采样场,见相位 5)。
  - [ ] **前置**:让面板/装饰图元在**一个片元里求多子形 + smin 组合**(现 `panel.wgsl` 一框一 `sdRoundBox`)。没具体效果需求前不必先建。
  - [ ] **消息气泡分组融合**(chat 首用,等气泡功能):连续同发送者多条气泡 `smin` 融成一体 + 头像/气泡接合 fillet。
  - [ ] **reveal "长出"形变**:整表骨架/行框由种子形 `mix`/`smin` 生长成完整框(比纯 scale/clip 更 SDF-native)。
  - [ ] **装饰接合**:代码块+语言标签 / 引用条+底 / 面板+角标 圆滑相接。
  - [ ] **多行选区·高亮**:跨行 rect 并集 → `smin` 圆角连续"墨团"(macOS 选区那种)。
  - [ ] **图标/控件 morph**:`mix(sdfA,sdfB,t)` 折叠箭头 / 勾选框 ✓ / loading 形变。
- [ ] **相位 5(后置):字↔字 morph** `mix(sdfA,sdfB,t)`(需两字形同 atlas 同采,2× 采样,贵)。
- [ ] 出场动画 + `threshold`/`band` 目标(只扩 `enter_profile_id` + profile 表,**不动 vertex 布局**)。
- [x] **性能命脉·冻结**(0025 §4):per-view `settled` 标志 —— 结算块每帧 O(1) 跳过(不扫 spawn / 不重 resolve);结算判定按**非换行**字已释放,**修 Plan 9 review #1**(换行/NodeSpawn 永不 spawn 致活动 view 不冻结)。
- [ ] **性能命脉·缓存(剩)**:活动(未结算)view 仍每帧 `resolve_tree`(1 个 view,O(块));内容不变的纯揭示帧可缓存 plan(`mem::take` 复用,key=(gcount,table_style,open_block))再省一层。
- 注:`smin`/`mix` = 形状融合/变形(解析场);reflow 用 **0016**(参数 lerp)。两个工具别混。深挖见 [研究](spec/research/animation-system-survey.md) / [sdf-animation](spec/research/sdf-animation-system.md)。

## 模块化 shader + markdown 组件([0026](spec/decision/0026-modular-shader-organization.md) / [Plan 11](spec/plan/plan11-modular-shader-and-markdown.md))

- [x] **相位①模块化地基**:`shaders/base/sdf.wgsl` 共享形状函数库(`sd_round_box`/`sd_seg`/`sd_circle`/`op_outline`/`op_smin`);glyph/rect/panel 移入 `base/`,rect/panel 去重 `sd_round_box`;`backend.rs::with_sdf` build 期前置拼接;wgsl 校验测试改拼接产物。
- [x] **相位②widget 图元 pipeline**:`shaders/markdown/widget.wgsl`(组件入口,按 component-id 分派)+ `box.wgsl`(复选框 SDF);`FrameWidget`(core)→ `WidgetInstance`(render,新 vertex 布局)→ widget pipeline(复用 globals bind);rect 后、glyph 前绘制。
- [x] **相位③任务复选框**:`StyleRole::TaskUnchecked/TaskChecked`(追加值 22/23,数值稳定);content 检测 `[ ]`/`[x]` → 零墨锚点 cell + 间隔;`block_decorations` 据锚点发 `FrameWidget` 方框(已勾叠 SDF 对勾,绿;未勾中性灰)。重放 case `g-tasks`。
- [ ] **本地确认**:`cargo test -p infinite-chat-{core,render}` + `wasm-pack build` + GPU 跑 `g-tasks`(改了 vertex 布局,wgpu 严格,必须实跑)。沙箱已过:wgsl 解析(markdown-widget/rect/glyph/base-sdf)+ tsc。
- [x] **相位④文字层**:脚注 `[^x]` → **行内小标记**(引用=去括号的 Link 色小号 `FootnoteRef`/值24;定义行首=弱化小号 `1.` `FootnoteDef`/值25;content.rs 后处理 Dim span + glyph 色 + roleScale 0.7/0.85);`---` HR 已迁 markdown widget(`WIDGET_RULE` component=1,`rule.wgsl`:**中间亮两端淡出渐变线**;dispatch 用 `select` 保导数均匀)。
- [x] **定义列表观感**:改 vendored jcode —— 术语去 bullet、内联文字置 `Strong`(粗体术语自成一行);定义去 `-> ` 箭头、改 4 空格缩进。`definition_list_term_bold_no_bullet_arrow` 验。
- [ ] **latex / image / html / link(各拆 1 步,另排)**:latex([0013])、image([0007]/[0022])、html(现 `JRole::Html→Normal` 当文字)、link(`Link` 角色已渲,交互需 DOM overlay/事件层)。本 plan 仅登记。
- [ ] **组件扩展**:`markdown/slider.wgsl` 等(加一个 `fn` + widget `switch` 分支 + component-id,不动布局);复选框勾选 `mix` morph 动画(接 Plan 10 §4 / 相位④图标 morph)。

## 可观测性(运行时;P1 + 数据通道已入 plan4 4C)

- [x] **节流帧统计**(`?debug`):每秒一行 `tracing target=perf` —— fps / 帧耗时 / 发射 vs 总 glyph / 可见 vs 总块 / atlas 占用·容量·淘汰。
- [ ] `performance.measure`(build_frame / draw)进 devtools Performance 面板。
- [ ] 关键路径计时 span(layout / rasterize / atlas alloc / draw)。
- [ ] **调试器 P2 质量组**:画布 glyph 包围盒/baseline/行框/按 role-kind 上色/atlas 纹理查看器;DOM 点选某字 → cluster/role/pos/size。
- [ ] **调试器 P3 效果上限组**:DOM 特效参数滑杆热调 + cps + FSM 徽章 + 事件日志;画布按 spawn_time 上色 / 生长尾高亮。
- 参考:[0012](spec/decision/0012-debugger-gui-html-vs-egui.md)。

## Plan 2 欠账(可插空)

- [ ] **语法高亮**(H5):tree-sitter / syntect-fancy-regex → 接颜色管线(GitHub `.pl-*` 调色板可抄)
- [ ] **Turn 完整分组投影(AR11)+ 折叠 tool/reasoning**(I5)
- [ ] **10k 行真机 fps/内存 benchmark**(一直挂着的待验项)
- [ ] 可视滚动条 + 块内 glyph 级裁剪细化(G 推迟项)
- [ ] 显式心跳 backoff 强制重连(J2,当前用周期 resync + 自动重连覆盖)

---

## 已决策、勿重开(背景锚点)

- 解析沿用 **pulldown-cmark**,不手写 nom、不上 comrak;自定义语法走标签层不动解析器([0010](spec/decision/0010-markdown-parsing-strategy.md))
- 文字 = **quad 图元**,自有引擎、借算法(TinySDF)不借框架(否决 AntV G / egui / cosmic-text / glyphon)([0011](spec/decision/0011-gpu-text-as-sdf-primitive.md))
- **正文用浏览器系统字体栈**(零打包,小包体);固定字形仅离线 MSDF([0009](spec/decision/0009-text-rendering-engine.md)→0011)
- **不引 pretext**,手搓 layout;**BiDi/RTL 非目标**([0001 §2.2 修订](spec/decision/0001-canvas-architecture.md))
- 调试器 = **DOM 面板 + 引擎自绘几何,否决 egui**([0012](spec/decision/0012-debugger-gui-html-vs-egui.md))
- **观感取向 = opinionated 单实现**:只支持**中英文 + markdown** 一条渲染路径;验证([V](#v--组件内观感验证视图opinionated非兼容性测试))只守"作者认可的观感"不回退,**不追排版兼容性/能力**(与 BiDi 非目标同源)。垂直度量收口见 [T](#t--字形垂直度量--baselinetextmetrics-收口)。
