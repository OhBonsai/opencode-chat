# Plan 12 进度(数学 LaTeX 一等 SDF 公民)

- 状态(2026-06-19):**核心管线落地 + 验证(① ② core + ①-web 脚手架);GPU/集成相位待人工实跑**。
- 沙箱约束(plan12 §4 已预告):Claude 有 cargo(native+wasm 编译/测试已过),但**无 GPU/浏览器** →
  数学字形的视觉栅化、行内 baseline 盒、Path/MSDF、逐字动画**须人工 GPU 实跑**验收。本轮把所有
  **纯 core / tsc 可验**的部分做完并测试,GPU 相位给出精确接入点。

## §0 spike 闸门 —— **通过**

RaTeX(本地 `RaTeX/` gitignore 工作区,git dep `rev=1e5ae7d`)经核实:
- API 与 plan §0 一致:`ratex_parser::parse(&str)->Vec<ParseNode>` → `ratex_layout::layout(&nodes,&LayoutOptions)->LayoutBox` → `ratex_layout::to_display_list(&LayoutBox)->DisplayList`。
- **纯 Rust、wasm 安全**:依赖仅 serde/regex/thiserror/phf(字体度量 phf 编译期内嵌,无 fs)。
- **`cargo build -p infinite-chat-core` native + `--target wasm32-unknown-unknown` 均通过**(各 ~31s)。

## 已落地(验证)

| 相位 | 落地 | 验证 |
|---|---|---|
| **①-core** | `StyleRole` 追加数学字族角色(`MathMain/Bold/Italic/BoldItalic/Var/Ams/Size1–4/Cal/Frak/Sans/Script/Tt`,值 **26+,不移位**守 0001);`math::font_role`(RaTeX 字族串→角色)/`katex_font_base`(反向→KaTeX 文件名) | cargo 单测 |
| **②-core** | `crates/core/src/math.rs`:`layout_math(tex,display)->MathLayout{width,height,depth,glyphs:[MathGlyph{ch,role,size,dx,dy}],rules:[MathRule],ok}`(RaTeX DisplayList:GlyphPath→glyph、Line/Rect→rule;Path 暂略);`math_to_frame(...)->（FrameGlyph,FrameRect)`（em×math_px → world,baseline `dy-size`,glyph_idx=morph 身份） | **8 cargo 单测**:E=mc² 顺序+上标、分数线、∑/√、display vs inline 高度、非法 LaTeX→ok=false、world 摆放 |
| **①-web** | `layout-bridge.fontForRole` case 26–40 → `KaTeX_*` 字族 + `isMathRole`;`math-fonts.ts` 懒加载 16 个 KaTeX woff2(`FontFace`);`main.ts` 异步预载 + `refresh_fonts`;`scripts/copy-katex-fonts.mjs`(RaTeX→`web/public/fonts/katex/`,gitignore) | `tsc` 绿;`wasm-pack`(RaTeX→wasm)绿;copy 脚本 16/16 |
| **② 集成(data 层)** | `BlockCache.math: Vec<(区间, MathLayout)>`(ensure_layouts 算一次随块冻结缓存,= 相位⑤ 缓存雏形);`build_frame` 对 MathDisplay 区间**跳过 raw TeX 字形**、改 `math_to_frame` 出数学 SDF 字形 + 规则线;TeX 由该区间 display 字符拼回(无需改 content 契约)。spawn 随块揭示;`glyph_idx` 高位基避 morph 撞 | **cargo 单测** `display_math_emits_sdf_glyphs_not_raw_tex`:`$$E=mc^2$$`→ E(MathMain)/m·c(MathVar 斜体)数学字形,无 raw Code TeX |

## 待人工 GPU 实跑(代码已就位,需浏览器验视觉)

> 集成的 **data 层已落地 + native 测**(`$$…$$` 已产数学 SDF `FrameGlyph`);剩**视觉栅化**(KaTeX 字体上屏锐利)与下列须 **GPU/浏览器**验收(plan §4)。

- **② 视觉**:web 侧 `glyph-raster.rasterize` 已按 `fontForRole(26+)→KaTeX_*` 取字体,`main.ts` 预载 KaTeX woff2 + `refresh_fonts`;须浏览器看公式字形上屏、SDF 锐利、定位对(`MATH_PX=18`、baseline `dy-size` 近似可能要微调)。**已知 v1 限**:数学块高度仍用 JS 文本高(非 `math.total_height()`)→ 高公式(`\frac`/`\sum`)可能与邻块间距不准,待块高改用 math 高。
- **③ 行内 `$...$` baseline 盒**(plan 自评"最复杂"):core 先算盒尺寸(`MathLayout.width/depth`)→ 作不可断"宽字符"占位喂 JS 行排版 → build_frame 摆 math glyph 到盒原点、基线对齐 `depth`。当前只接显示数学 `$$…$$`。
- **④ MSDF(把 RaTeX 的 KaTeX 字体编译成 MSDF)**:
  - **已做(编译 + 工具)**:`crates/core/tests/dump_katex_charset.rs`(`#[ignore]` 辅助)按 RaTeX 语料收集**每个 KaTeX 字族实际用到的字符集** → `scripts/katex/charset/*.txt`(8 族,~150 字形,已提交)。`scripts/bake-katex-msdf.mjs` 跑 msdf-bmfont 逐族烘 → `web/public/fonts/katex-msdf/<Base>.{json,png}`(per-font BMFont MSDF,gitignore,**已跑通 8/8**)。
  - **待人工 GPU(运行时接入)**:backend MSDF 是 **D2Array**(页尺寸须统一 = lxgw 2048²),且 8 族 codepoint 重叠(Main 'A' ≠ Math 'A')→ 须把各族 glyph **合成进单页 atlas**(需 png 合成,如 pngjs)并按**合成键 `role*0x110000 + codepoint`**(role=StyleRole 26–40)索引;wasm `resolve` 已就绪(数学角色现走 TinySDF),改为对合成键查 MSDF(命中)→ 未命中回退 TinySDF;web 加 `load_math_msdf`(同 `loadMsdf` 路)。`msdf_node` 几何复用(数学 glyph 也是带 pos/size 的 `FrameGlyph`)。
  - **Path(根号/大定界符)**:多数定界符走 Size1–4 字形(已 atlas 化);`DisplayItem::Path` 矢量轮廓暂略。
  - 现状:数学走 **TinySDF + KaTeX woff2**(`resolve` 对角色 26–40 直接 TinySDF,跳过 lxgw MSDF 落空查),已可用、缩放略软;上面合并落地后即 MSDF 锐利。
- **⑤ 缓存**:按 TeX hash 缓存 `MathLayout`(排版确定性);atlas 按 `(role,char,size)` 现成 LRU。`math_to_frame` 已确定性,接缓存即可。
- **⑥ 逐字动画**:数学 glyph = `GpuInstance` + `anim` profile → Plan 10/0025:逐符号"写出"、跨式 morph。`FrameGlyph.anim` 字段已在。
- **⑦ 兜底**:`layout_math` 已返回 `ok=false`(RaTeX 不支持)→ 上游退 MathJax→SVG embed(0013 C)或显式占位。
- **glyph-raster / main.ts**:`glyph-raster.rasterize` 已按 `fontForRole(style)` 取字体 → 数学角色自动用 `KaTeX_*`(字体加载后);`main.ts` 在首个数学块出现时 `loadMathFonts().then(()=>chat.refresh_fonts())`。

## 卡口(本轮)
- `cargo test -p infinite-chat-core`(含 8 个 math 测试)、`clippy --all-targets -D warnings`(native + wasm32)全绿;`tsc` 绿。
- `cargo build -p infinite-chat-core --target wasm32-unknown-unknown` 通过(RaTeX wasm 安全确证)。

## 字体资产
KaTeX woff2 由 `scripts/copy-katex-fonts.mjs` 从 `RaTeX/platforms/web/fonts/` 复制到 `web/public/fonts/katex/`(gitignore,可重生,同 lxgw-msdf 策)。
