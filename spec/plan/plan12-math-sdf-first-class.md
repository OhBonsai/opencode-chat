# Plan 12 — 数学(LaTeX)一等 SDF 公民(方案 B,直到最终效果)

- 日期:2026-06-18
- 状态:规划(开工前必做 §0 spike 核实 RaTeX API)
- 前置:[0013](../decision/0013-math-latex-rendering.md)(§8 翻转为 B)、调研 [research/math-latex-sdf-rendering](../research/math-latex-sdf-rendering.md);[0001](../decision/0001-content-layout-render-contract.md) 契约、[0011](../decision/0011-gpu-text-as-sdf-primitive.md)(字即 SDF)、[0015](../decision/0015-glyph-source-fallback.md)(字形源/atlas)、[0016](../decision/0016-streaming-morph-render-model.md)、[0020](../decision/0020-content-node-identity-model.md)、[0025](../decision/0025-sdf-node-animation-system.md)/[Plan 10](plan10-sdf-animation-system.md)(动画)、[Plan 11](plan11-modular-shader-and-markdown.md)(widget/rect 图元)
- 取向:数学字形 = 和正文同级的 SDF 一等公民(进 atlas、缩放锐利、逐字可控/可动画),**不**烤纹理。排版用 **RaTeX**(纯 Rust)在 core 算出定位字形,JS 只负责栅化字形外观(复用现有 atlas 管线)。
- 最终效果:`$...$`/`$$...$$` 正确排版;每个符号是 SDF 实例;可逐字"写出"公式、跨式 morph;确定性 + 按 TeX hash 缓存;极端 LaTeX 退 MathJax→SVG 兜底。

---

## 0. RaTeX API(已读源核实,2026-06-18 —— 本地 `RaTeX/` 工作区)

源码读毕,接口完全契合,spike 闸门**通过**。确定的真相:

**公开管线**(三步,全在 `ratex-layout` + `ratex-parser`):
```rust
let nodes = ratex_parser::parse(latex)?;                 // &str → Vec<ParseNode>
let lbox  = ratex_layout::layout(&nodes, &LayoutOptions); // → LayoutBox(可设 style=Display/Text、color)
let dl    = ratex_layout::to_display_list(&lbox);          // → DisplayList(扁平、绝对坐标)
```

**`DisplayList`**(`ratex-types/src/display_item.rs`):`{ items: Vec<DisplayItem>, width, height, depth }`(f64;`total_height()=height+depth`)。

**`DisplayItem`**(4 变体,serde `tag="type"`):
- `GlyphPath { x, y, scale, font: String, char_code: u32, color }` ← **字形,主力**
- `Line { x, y, width, thickness, color, dashed }` ← 分数线/上划线
- `Rect { x, y, width, height, color }` ← `\colorbox` 底
- `Path { x, y, commands: Vec<PathCommand>, fill, color }` ← **根号 surd / 大定界符**(矢量路径)

**坐标系**(`to_display.rs` 注释明确):x 右增,**y 下增(屏幕坐标)**,原点 = 包围盒左上,**baseline 在 y=height**。单位 = KaTeX em(顶层 `to_display_list` 以 scale=1.0 起);乘"每 em 像素"得 px。`GlyphPath.y` = 该字基线 y,`scale` = 该字 em 字号倍率。

**字体名**(`FontId::as_str`,`ratex-font/src/font_id.rs`):`"Main-Regular"`/`"Math-Italic"`/`"AMS-Regular"`/`"Size1-Regular"`…`"Size4-Regular"`/`"Caligraphic-Regular"`/`"Fraktur-Regular|Bold"`/`"SansSerif-*"`/`"Script-Regular"`/`"Typewriter-Regular"` + CJK/Emoji 兜底。前缀加 `"KaTeX_"` 即对应字体文件。**`char_code` 已是该字体 cmap 内的码点**(RaTeX 的 `math_alpha` 已解析数学字母映射,如 Main-Bold cmap 用 'A' 而非 U+1D400)→ 直接 `char::from_u32(char_code)` 用对应 KaTeX 字体栅化即得正确字形。

**参考实现**:`ratex-wasm/src/lib.rs::render_latex` = parse→layout→to_display_list→`serde_json`(带 version 包裹 + NaN 清零),证明 **WASM 构建可行**且输出 DisplayList JSON。

**对 Plan 的影响**:接口比预想更干净——`GlyphPath`/`Line`/`Rect` 直接映射 `FrameGlyph`/`FrameRect`;**唯一新增工作 = `Path` 变体**(根号/大定界符的矢量路径)需路径渲染(见相位④)。无需再做 spike,直接进相位①。

---

## 1. 架构落点(content→layout→render 契约内)

数学是**特殊 run**:排版在 **core**(RaTeX 给绝对坐标),**绕过 JS 文本测量**(JS 只栅化字形)。

```
解析(pulldown ENABLE_MATH,已开)
  → content.rs:InlineMath/MathDisplay 携原文 TeX(已收;现当 Code 显示,改)
  → core math 模块:parse→layout→to_display_list → DisplayList{items,width,height,depth}(em,缓存 by TeX hash)
  → 行内:作 baseline 盒接入行排版(占 advance=width,基线对齐 depth);显示:作块(块高=height+depth)
  → build_frame:DisplayItem px=em×mathPx,world=盒原点+(x,y) →
       · GlyphPath → FrameGlyph(cluster=char_from_u32(char_code),math 角色按 font 选,spawn 同行揭示)
       · Line     → FrameRect(thin 线;分数线/上划线)
       · Rect     → FrameRect(实心;\colorbox)
       · Path     → 矢量路径渲染(根号 surd/大定界符;见相位④)
  → 栅化:math 字形按 (font, char_code, size) 进 SDF atlas(JS rasterize 用 KaTeX 字体文件;MSDF 可选)
  → 动画:字形走 0025 per-instance(进场/morph)
```

要点:数学字形的 `pos` 由 **core 直接写**(RaTeX 给绝对 em 坐标 × mathPx),不经 JS layout;JS 仅 `rasterize_fn(char, math_role, kind)` 出 tile。**GlyphPath/Line/Rect 复用现有 FrameGlyph/FrameRect + atlas/glyph/rect pipeline,无新管线;唯 Path 需新增路径渲染(相位④)。**

---

## 2. 分期

### 相位 ① 字体 + atlas 接通(地基)
- web:把 **KaTeX 字体**(woff2:KaTeX_Main/Math/AMS/Size1–4/Caligraphic/Fraktur/SansSerif/Script/Typewriter)放 `web/public/fonts/`,`main.ts` 启动 await 加载(数学不出现可懒加载,守体积)。
- `layout-bridge.ts` `fontForRole`:新增数学角色 → 对应 KaTeX 字族(如 `MathMain`/`MathItalic`/`MathSize1`…)。`roleScale` 数学按 RaTeX 给的 size,不走标题倍率。
- content.rs `StyleRole`:**追加**数学角色(`MathOrd`/`MathOp`/`MathRel`…或更简:`Math` + 字体维度另走字段)。**追加不移位**(守 0001 数值稳定)。glyph.wgsl `style_color` 加数学取色(中性/可配)。
- **验收**:手摆一个 FrameGlyph(math 角色 + KaTeX 字体字符)能栅化上屏、SDF 锐利。

### 相位 ② RaTeX → MathLayout(显示数学 `$$` 先行)
- 新 `crates/core/src/math.rs`(或 `crates/math`):`fn layout_math(tex: &str, display: bool) -> MathLayout`,内部 ratex → DisplayList → 映射。`MathLayout{ width, height, depth, glyphs: Vec<MathGlyph>, rules: Vec<MathRule> }`;`MathGlyph{ ch, font_role, size, dx, dy }`;`MathRule{ dx, dy, w, h }`(块内相对)。
- content.rs:`MathDisplay` 块不再当 Code → 标记为 math 块携 TeX;app.rs build_frame 对 math 块调 `layout_math(.., true)`,glyphs→FrameGlyph(world=块顶+dx/dy)、rules→FrameRect。块高 = MathLayout.height。
- **核 0001**:math 作"特殊 run/embed"在契约内,不破坏 core 解析。
- **验收**:`$$E=mc^2$$`、`$$\frac{a}{b}$$`、`$$\sum_{i=0}^n i$$`、`$$\sqrt{x}$$` 正确排版上屏,字形 SDF 锐利;分数线/根号 vinculum 是 FrameRect。

### 相位 ③ 行内数学 `$...$`(baseline 盒接入行流)
- 行内 math = 一个**带 baseline 的盒**:宽 = MathLayout.width,基线对齐(用 depth)。接入 4A 折行:把 math 盒当作一个不可断的"宽字符"占位,行排版预留其 advance,行高吃其 height/depth。
- content.rs 行内 `$` run → emit 一个 math 占位(承载 TeX + 盒尺寸);layout(JS)据盒宽预留空间;build_frame 把 math glyphs 摆到盒的 world 原点。
- **难点**:core 的 math 盒尺寸要在 JS 行排版**之前**算出(RaTeX 在 core,先算尺寸 → 传给 JS 占位)。或:行内 math 也由 core 接管该 run 的水平推进。
- **验收**:`质能方程 $E=mc^2$ 如上` 行内公式与文字基线对齐、不串行、折行正确。

### 相位 ④ `Path` 变体(根号/大定界符)+ MSDF 尖角保真
- **Path 渲染(新增,必做)**:`DisplayItem::Path{ x, y, commands: Vec<PathCommand>, fill, color }` 用于根号 surd 与大定界符的矢量轮廓。三选一:(a)**离线 msdfgen 把路径烤进 atlas**(当作"特殊字形"tile,复用 glyph pipeline,最省);(b)CPU 三角化 → 一个轻量 fill pipeline;(c)简单形状(竖线/根钩)退化成 `FrameRect`/markdown widget SDF。v1 取 (a) 或 (c);先确认 RaTeX 多数定界符其实走 `Size1–4` 字形(GlyphPath,已 atlas 化),仅极大者为 Path。
- **MSDF 尖角(推荐)**:数学符号尖角敏感(√钩/∫端/分数线端/括号尖)→ 单通道 SDF 会圆角。bake-time `msdf-atlas-gen`(吃 KaTeX TTF)预烤数学 atlas,或运行时纯 Rust `fdsm`。glyph.wgsl 已有 MSDF 分支(kind=2,median3)→ 数学字形标 kind=MSDF 即享。
- **验收**:`\sqrt{\frac{a}{b}}` 的大根号、`\left(...\right)` 的大括号正确成形;√、∫、大括号大缩放下边缘锐利无圆角。

### 相位 ⑤ 缓存 + 确定性
- 按 **TeX 源 hash** 缓存 `MathLayout`(排版确定性);atlas 按 `(math_font, char, size)` 懒填(现成 LRU)。不缓存渲染后产物。
- **验收**:同一公式重复出现/重排不重算 layout;perf 面板看 math 块命中缓存。

### 相位 ⑥ 逐字动画(最终效果,接 0025)
- 数学字形 = GpuInstance,带 `anim` profile → 走 Plan 10/0025:**逐字"写出"公式**(按 DisplayList 顺序/结构错时进场,reveal 节奏接 0019);**跨式 morph**(同符号在两公式间 `mix`,Manim `TransformMatchingTex` 的 SDF 版,接 Plan 10 §4/§5)。
- **验收**:重放慢放可见公式逐符号长出;两个相邻公式切换有符号级过渡(后续)。

### 相位 ⑦ 兜底 C(极端 LaTeX)
- RaTeX 未覆盖的构造 → 退 MathJax→SVG embed(0013 原 C,走 mermaid/embed 路,opaque 纹理,无逐字)。仅作 fallback,主路是 B。
- **验收**:故意喂 RaTeX 不支持的宏 → 不崩,降级为纹理 embed(或显式"未支持"占位)。

---

## 3. 验收总览(最终效果)
- `$...$` 行内与 `$$...$$` 显示数学正确排版;每个符号是 SDF 实例、缩放锐利、可逐字上色/动画。
- 公式可逐符号"写出";确定性 + hash 缓存;尖角 MSDF 保真;极端 LaTeX 有兜底。

## 4. 风险 / 评审
- **RaTeX API 未知**(§0 spike 是闸门);不过则回退 Typst-math/保持 C。
- **行内 baseline 盒**是最复杂处(core 算尺寸 ↔ JS 行排版的时序),显示数学先行降风险。
- **体积**:KaTeX 字体 ~300KB–1MiB,懒加载(无数学零成本);RaTeX wasm 增量待 spike 量。
- **沙箱无 cargo**:Claude 不能编 Rust;每相位本地 `cargo test` + `wasm-pack build` 确认;RaTeX 接入尤其要 GPU 实跑。
- **守界**:不自研 TeX 排版、不自研 OT MATH 表(用 RaTeX 的 CM 路线);字体无关/MATH 表驱动是后续(评估 Typst-math)。

## 5. 落点 / 拆分
- spike(§0)→ 相位①②(显示数学 MVP)→ ③(行内)→ ④(MSDF)→ ⑤(缓存)→ ⑥(动画,最终效果)→ ⑦(兜底)。各相位独立可验、可单独 commit。

> 框架 = 0013 §8;RaTeX 在 core 产定位字形 → 映射 FrameGlyph/FrameRect → 复用 SDF atlas → 0025 逐字动画。数学因此和正文同级:每符号一个 SDF 实例,Manim 级控制。spike 先行核实 RaTeX API。
