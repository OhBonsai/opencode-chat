# 决策记录 0013:数学(LaTeX)渲染策略 —— 业界 Rust 怎么做 + 我们怎么选

- 日期:2026-06-14(**2026-06-18 重评:翻转为方案 B,见 §8**)
- 状态:**已修订** —— 原定 C 起步;现因"逐字 SDF 控制 + 提前渲染"成为优先级,**改 B(RaTeX,数学一等 SDF 公民)为主路**。落地见 [Plan 12](../plan/plan12-math-sdf-first-class.md)。
- 前置:0004(markdown 管线 / 嵌入)、0007(富媒体嵌入 / 像素对齐)、0010(解析沿用 pulldown-cmark)、0011(quad/SDF 图元)
- 来源:调研 `~/w/agentscode/{jcode,warp}` + Rust 数学排版生态

## 1. 背景

LLM 输出常含 LaTeX 数学:行内 `$...$`、显示 `$$...$$`。**解析侧已白嫖**——pulldown-cmark `ENABLE_MATH`(vendored jcode 已开)会吐 `InlineMath`/`DisplayMath`,拿到原文 TeX。**现状渲染侧不排版**:`content.rs` 把 Math 当 `Code` 显示原文 TeX。问题:要不要真排版(分式/上下标/根号/∑∫),怎么排。

## 2. 业界 Rust 怎么做(实证)

### 2.1 终端/桌面 markdown 渲染器:基本不排版

- **warp**:markdown 渲染**完全无视 math**——查无任何数学渲染代码(`tex/latex` 仅出现在文件扩展名表、测试夹具"一只叫 Rex 的狗"、语言检测词表、skill 文档)。
- **jcode**:解析了但**不排版**——render-core 标 `StyleRole::Math` + `BlockKind::MathDisplay` 携原文 TeX,`model.rs` 注释"the adapter frames it";前端只**上色/当 code**(TUI `StyleRole::Math => math_fg()`;桌面 `text_style.rs` 把 `Code | Math` 归一类当行内代码)。**全仓库无任何数学排版 crate**。
- **唯一可借鉴**:jcode 的 `escape_currency_dollars` 预处理——把 `$5`/`$5x` 货币转义,防被当行内数学。**我们 vendored 的 `parse_markdown` 内部已调用 → 已白嫖**(content.rs 加回归测试锁定)。

→ 结论:**两个"业界参考"都给不了数学真排版的现成实现**。

### 2.2 真排版的 Rust 选项

- **RaTeX**(纯 Rust):>99.5% KaTeX 语法覆盖,无 JS/WebView/DOM;解析 LaTeX → TeX 规则 → **吐扁平 display list(定位字形 + 横线)**→ 喂 Canvas2D / Skia / **自有矢量后端**;支持 WASM,与 KaTeX golden test 对齐。
- **Typst**:完整排版系统,自带数学引擎,可 math → SVG(`typub`/`mdbook-typst-math`);功能全但重。
- **(JS)KaTeX / MathJax**:浏览器排 → SVG/HTML;MathJax `tex2svg` 输出自包含矢量 SVG。

## 3. 选项对比(对本项目)

| 方案 | 真排版 | 包体 | 数学字形能否 SDF/缩放/特效 | 复用 |
|---|---|---|---|---|
| **A 不排版**(现状) | ✘(原文 TeX 当 code) | 0 | — | — |
| **B RaTeX 原生 → quad 管线** | ✅ | + RaTeX(wasm)+ **数学字体(MATH 表,数百 KB)** | ✅ **一等图元**(进 atlas,可 SDF/缩放/特效) | 走 content→layout→render |
| **C KaTeX/MathJax → SVG → 纹理(embed)** | ✅ | + 数学库(可懒加载,MathJax SVG 自包含不另需字体) | ✘ **opaque 纹理**(无逐字特效,极端缩放重栅) | 走 mermaid embed 路(0004 §7 / 0007) |

## 4. 决策

**C 起步(embed + 懒加载),B(RaTeX)作"数学一等公民"的升级路径。**

1. **v1 = C**:`$$` 显示数学 → MathJax/KaTeX 出 SVG → 浏览器光栅成纹理 → embed 块 quad(复用 mermaid 管线);行内 `$` → 带 baseline 的行内 embed 盒(4A 折行那侧补);按 TeX hash 缓存;**数学库懒加载**(不出现数学则零成本,守小包体)。
2. **升级到 B**:当需要"**数学字形像文字一样进 atlas、可缩放/可叠 SDF 特效**"(尤其 [TODO2] 的动画式 reflow:opaque 纹理里的字形没法 tween)→ 引 RaTeX,把 display list 映射成 quad 实例 + 打包数学字体。
3. **货币防误判**:`escape_currency_dollars` 已随 vendored jcode 生效(§2.1),content.rs 加回归测试。
4. **解析不变**:pulldown `ENABLE_MATH`;math 无论走 C 还是 B 都在 0001 §2.2 契约内(math = 特殊 run / embed),不动 core 解析。

## 5. 后果 / 重评估触发

- v1(C):数学能正确显示,但**进不了逐字特效、极端缩放要重栅**;数学库是懒加载的可选体积。
- **触发上 B(RaTeX)**:数学量大且要缩放/特效一等公民;或要把数学纳入"字块 move"动画([TODO2] 效果系统)。
- **触发上 Typst**:要服务端/离线 PNG 或更全的排版(罕见)。

## 6. 落点

LaTeX 本质是 **embed 子项**(图片→mermaid→**math**→卡片,0007 三层管线)→ [TODO O](../../TODO.md);外加 4A 折行补"行内 embed baseline 盒";升级到 RaTeX 时改走 quad 管线。

## 8. 重评(2026-06-18):翻转为 B —— 数学一等 SDF 公民

**触发**:本期优先级变为"**每个数学字形都是 SDF + Manim 级逐字控制 + 可提前渲染**"。深度调研见 [research/math-latex-sdf-rendering](../research/math-latex-sdf-rendering.md)(5 路检索 + 来源核实)。

**新结论**:C(SVG→纹理)与新优先级**互斥**(opaque 纹理无法逐字、缩放重栅),故**翻转:B(RaTeX)作 v1 主路**,C 退为极端 LaTeX 的兜底。

**关键证据**:
- **RaTeX 与本引擎同构**:纯 Rust、`ratex-wasm` 一等、原生吐**扁平 DisplayList(定位字形 + 横线)**→ 任意 2D 后端;自带 KaTeX 字体 TTF(可直接栅 SDF)。它的输出 1:1 映射本引擎 `FrameGlyph`/SDF atlas + `FrameRect`。活跃维护(v0.1.11,2026-05,>99.5% KaTeX 覆盖)。
- **KaTeX/MathJax 不进主路**:KaTeX **不给坐标**(竖直坐标只冻进 CSS `top`,水平坐标交浏览器行内排版——KaTeX issues #537/#587 实证);MathJax 是 JS + 重运行时。二者退为参考/兜底。
- **OT MATH 表不自研**:RaTeX 走 KaTeX 式"提取度量 + 烤死 TeX 常量"的 CM 字体路线,绑定 TeX 字体即够 TeX 级质量;字体无关(STIX/Cambria)与 MATH 表驱动是后续升级(评估 Typst-math)。
- **逐字控制 = 复用 0025**:定位字形 → 各自 SDF tile → GpuInstance → 走已有 per-instance 动画(逐字"写出"公式、跨式 morph)= Manim 模型,只是矢量路径换 SDF 实例。
- **提前渲染**:排版确定性 → 按 TeX hash 缓存 DisplayList,懒填 SDF atlas;不缓存渲染后 HTML/SVG(体积爆炸)。

**接入前唯一硬核实点**:`ratex-types::DisplayItem` 确切字段(字形 char/glyphId + x/y + 字号 + 字体 + rule 矩形)。

**落地**:[Plan 12](../plan/plan12-math-sdf-first-class.md)(解析→RaTeX 排版→映射 FrameGlyph/FrameRect→MSDF atlas→行内/显示接入→缓存→0025 动画→C 兜底)。

## 7. 来源 / 链接

- 实证:`~/w/agentscode/jcode`(render-core `StyleRole::Math`/`preprocess::escape_currency_dollars`、tui `render_core_adapter.rs`、desktop `text_style.rs` `Code|Math`)、`~/w/agentscode/warp`(无数学渲染)
- 已白嫖:`vendor/jcode-render-core/src/preprocess.rs`(`parse_markdown` 内部调用)
- 选项:[RaTeX](https://github.com/erweixin/RaTeX) / [RaTeX 站](https://ratex.lites.dev/) · [typub](https://crates.io/crates/typub) · KaTeX / MathJax `tex2svg`
- 相关:0004 / 0007(embed)、0011(quad/SDF)、[TODO2](../../TODO2.md)(动画式 reflow)
