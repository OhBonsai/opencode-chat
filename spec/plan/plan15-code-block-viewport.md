# Plan 15 — 代码块视口落地:行窗 + 边缘淡入淡出 + 双向滚动 + 行号 + 复制图标

- 状态(2026-06-20):**①–⑥ core/render/wasm/tsc 落地 + 测试通过(整链编译 wasm32);淡入淡出/
  滚动手感/图标/横裁须人工 GPU**。进度详情 → [plan15_progress.md](./plan15_progress.md)。
- 日期:2026-06-19
- 前置:[0027 代码块视口](../decision/0027-code-block-viewport.md)(决策)、[plan13 Taffy 叶子](./plan13-chat-box-layout.md)、[plan14 copy.svg→纹理](./plan14-image-embed.md)、[0018 面板](../decision/0018-sdf-panel-decoration-primitive.md)、[0011 per-glyph alpha](../decision/0011-gpu-text-as-sdf-primitive.md)、[research 代码高亮](../research/code-block-syntax-highlighting.md)(逐字色,正交)
- 一句话:落地 0027——代码块 = **6 行行窗**(maxHeight,0021 可配),超出**软裁剪 + 上淡出/下淡入**,流式自动 tail,左 **行号 gutter**,右上 **copy.svg 图标**(钉住),块内 **纵+横滚动**。复制交互/选中**不做**(图标先在)。

> 与高亮正交:本 plan 管**盒/裁剪/滚动/行号/图标**;逐字着色是另一条(research,可后接)。本 plan 代码字仍可先用单一 `CodeBlock` 色,高亮后续叠。

---

## 0. 现状

`StyleRole::CodeBlock` 整段平铺;`app.rs::block_decorations` 发一条底 rect(`code_block_emits_background_rect`);**无限高、无滚动、无行号、无复制**。wheel 全量 → 画布 `pan_by`(input.ts),无块内路由。

## 1. 数据流

```
content.rs:代码块行 → 注入行号字(CodeLineNum)+ 代码字(CodeBlock/高亮角色)
  → Taffy(plan13):代码块 = 叶子,measure 高 = min(N,6)·lineH + chrome(固定行窗)
  → build_frame:按块内 scrollY/scrollX 偏移内部内容 → 视口外 cull、边缘 fade alpha
                 + gutter(纵滚横不动)+ 底面板(0018)+ copy 图标 FrameImage(钉右上)
  → render:glyph(带 fade alpha)/ panel / image,横向 scissor 裁(长行)
input.ts:指针命中代码块 → wheel/drag 改该块 scrollX/Y(不滚画布);流式时 scrollY 自动 tail
```

## 2. 关键机制

### 2.1 行窗盒(固定高)
- Taffy 叶子 measure:`h = min(N, MAX_LINES=6)·lineH + pad`;**过 6 行盒高稳定**(不顶下文,plan13 §4.5 锚底友好)。`MAX_LINES` 走 0021 可配。
- 内部内容(N 行 × 行宽)独立内部坐标,**不进外层块流**;视口只露 6 行。

### 2.2 软裁剪 + 边缘 fade(per-glyph alpha)
- build_frame 对代码块内字:`y_in_view = y_content − scrollY·lineH`;**视口外(`<0` 或 `>6·lineH`)cull 不发**。
- fade band `FB ≈ 0.75·lineH`:
  ```
  a_top    = scrollY>0      ? clamp(y_in_view / FB, 0, 1) : 1   // 顶端不淡顶
  a_bottom = scrollY<maxY   ? clamp((viewH − y_in_view)/FB,0,1) : 1
  alpha *= min(a_top, a_bottom)
  ```
  → 上淡出、下淡入;仅"还有更多"那侧淡。复用 `FrameGlyph` 现有 alpha(0025 同通道),**无新管线**。

### 2.3 流式 tail
- 代码流入(活动块)→ `scrollY` 自动 = `max(0, N−6)`(跟最新 6 行);新行底部淡入、旧行顶部淡出。
- 用户上滚(2.5)→ 置 `following=false`,脱离 tail 看历史;滚回底 → 复跟随。

### 2.4 行号 gutter
- 左列宽 = `digits(N)·charW + pad`;每行一个行号(右对齐),角色 `CodeLineNum`(Dim 小号)。
- **纵向**:随 scrollY 滚(号绑行) + 同受 2.2 fade;**横向**:`scrollX` 不动(gutter 固定左,代码区横滚)。
- gutter 与代码间一条分隔(FramePanel 竖线或底面板内饰)。

### 2.5 双向滚动(块内状态 + 命中)
- core:`scroll: HashMap<node_key, (scrollX:f32, scrollY_lines:i32, following:bool)>`(按 0020 key)。
- input.ts:wheel/drag 时先 CPU 命中(指针 world 坐标 ∈ 哪个代码块视口 rect,0011 §3.3④ 基础盒)→ 命中则调 `chat.scroll_code_block(key, dx, dy)`,**不调 pan_by**;未命中走画布 pan。
- clamp:`scrollY ∈ [0, N−6]`、`scrollX ∈ [0, maxLineW − codeViewW]`。

### 2.6 横向裁剪(长行)
- 横向无 fade → **硬裁**:wgpu `set_scissor_rect` 到代码区(gutter 右、视口内)或 shader x-clip。= 表格 C 横滚同款机制,本 plan 顺带立。

### 2.7 复制图标
- 预载 `web/public/copy.svg` → 纹理(plan14 路);build_frame 发 `FrameImage` **钉代码块右上角**(world rect 跟盒、**不随 scroll**),在裁剪层之上。**无交互**(hover/点击/剪贴板 = TODO Q,defer)。
- **动效升级路(非本期,记原则)**:要 copy→✓ 点击反馈 / hover 脉冲时,**不做成动图文件**——升级为 SVG path → **SDF widget**(0026,同 checkbox `box.wgsl`)+ `mix(sdf_copy, sdf_check, t)` morph([Plan 10 §4 图标 morph]),**引擎程序化播**、缩放锐利、接 0021 主题色、零纹理上传。v1 静态纹理(0027 §3 已定)够用;内置图标动画一律走程序化 SDF,不用 GIF/动画 SVG(同 plan14 §2.5"内容动图 vs 内置图标动效")。

## 3. 相位

| 相位 | 交付(file:符号) | 验证 |
|---|---|---|
| **① 行窗盒 + 软裁剪 fade** | Taffy 叶子钳高(plan13)+ build_frame cull + 边缘 alpha(§2.2)+ tail(§2.3) | cargo:盒高=min(N,6)·lineH;视口外 cull;fade alpha 公式;tail scrollY=max(0,N−6)。**GPU 人工**:6 行窗 + 上下淡 + 流式 tail |
| **② 行号 gutter** | `CodeLineNum` 角色 + gutter 布局(§2.4) | cargo:行号字数/对齐;**GPU 人工**:号随纵滚、横滚固定、受 fade |
| **③ 复制图标** | copy.svg 预载纹理 + `FrameImage` 钉右上(§2.7) | tsc;**GPU 人工**:图标在右上、不随滚、缩放清晰 |
| **④ 双向手动滚动** | core scroll 状态 + 命中盒 + input.ts 路由(§2.5) | cargo:clamp;**人工**:指针在块内滚块不滚画布、横纵都动 |
| **⑤ 横向裁剪** | scissor / x-clip 代码区(§2.6) | **GPU 人工**:长行不溢出 gutter/视口 |
| **⑥ 底面板 + 进窗补间 + 卡口** | 底迁 0018 面板 + gutter 分隔;盒高 1→6 行 0016 补间 | 全卡口绿;**GPU 人工**:进窗平滑 |

> 沙箱可验:盒高/cull/fade/tail/clamp/行号(cargo)+ tsc + wgsl 解析。**淡入淡出、滚动手感、图标、横裁须人工 GPU。**

## 4. 测试用例提纲

- [ ] 正常:N≤6 行 → 盒高=N·lineH,无 fade、无滚动、无 tail。
- [ ] 正常:N>6 → 盒高=6·lineH;顶端 scrollY=0 不淡顶;底端不淡底;中间两侧淡。
- [ ] 流式:代码逐行流入 → scrollY 自动 tail=max(0,N−6),底部淡入;上滚 following=false 脱离。
- [ ] 行号:N=120 → gutter 宽=3 位;号随纵滚、横滚不动。
- [ ] 滚动:指针在代码块内 wheel → 块滚不滚画布;clamp 边界;块外 wheel → 画布 pan。
- [ ] 横向:超宽行 → 横滚 + 硬裁不溢出;gutter 横滚不动。
- [ ] 图标:copy.svg 在右上、不随 scroll、缩放清晰。

## 5. Scope · 不做什么

- ❌ **复制交互 / 选中 / hover**(TODO Q 输入层 + 剪贴板)——**图标先在,无行为**。
- ❌ 逐字语法高亮(research 另排;本 plan 代码字可先单一色)。
- ❌ 可见滚动条 ornament(先只滚动手感;条后续可选)。
- ❌ maxHeight 折叠/展开按钮(先固定 6 行窗;展开后续)。
- ❌ 软换行(代码默认不折、横滚;wrap 模式后续可选)。

## 6. Risk / Open

- **命中路由**(§2.5)= TODO Q 前哨:最小 CPU 盒命中即可,别提前做全选区/hit-test;与未来 Q 收口时合并。
- **横向 scissor 分批**:多代码块各一 scissor → draw 分批;先按块循环,基准看代价(同表格 C)。
- **fade 与 reveal(0019)叠加**:揭示 alpha × fade alpha 两者相乘是否自然(流式 tail 期都在动)——人工核。
- **Open**:① `MAX_LINES` 默认 6、可配范围?② 横滚用 scissor vs shader x-clip(与表格 C 一并定)?③ 行号是否随高亮主题变色(暂 Dim 固定)?④ tail 与"用户刚上滚又来新行"的抢滚策略(暂:用户滚过即停 tail,直到滚回底)。

## 7. Done

代码块 = 6 行行窗(0021 可配),超出软裁剪 + 上淡出/下淡入、流式自动 tail;左行号 gutter(纵滚横不动)+ 右上 copy.svg 图标(钉住);块内纵+横滚动(命中路由,不滚画布);底迁 0018 面板、进窗 0016 补间;复制交互/选中明确不做(图标在位)。卡口(cargo/clippy native+wasm、wasm-pack、tsc、wgsl 解析)全绿。

## 8. 关联

- decision:[0027](../decision/0027-code-block-viewport.md)(主)/ plan13(Taffy)/ plan14(copy.svg 纹理)/ [0018](../decision/0018-sdf-panel-decoration-primitive.md)(面板)/ [0011](../decision/0011-gpu-text-as-sdf-primitive.md)(alpha/命中)/ [0019](../decision/0019-reveal-gating-and-choreography.md)·[0016](../decision/0016-streaming-morph-render-model.md);高亮 research [code-block-syntax-highlighting](../research/code-block-syntax-highlighting.md)(正交,后接)。
- Code 入口:`app.rs`(block_decorations/build_frame/滚动状态)·`content.rs`(行号+CodeLineNum)·`frame.rs`(FrameImage/FramePanel)·`web/src/input.ts`(命中路由)·`web/src/image-loader.ts`(copy.svg 预载)·`web/public/copy.svg`。
