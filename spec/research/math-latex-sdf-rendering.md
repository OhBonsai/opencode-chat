# LaTeX/KaTeX 数学渲染调研 —— SDF 引擎下的"每字皆 SDF + Manim 级控制"

- 日期:2026-06-18
- 目的:为本引擎(Rust→WASM→WebGPU/WebGL2,一切皆 SDF)选数学渲染路线;重评 [0013](../decision/0013-math-latex-rendering.md)(原定 "C: MathJax/KaTeX→SVG 纹理 embed 作 v1,RaTeX 作升级")——现在**逐字 SDF 控制 + 提前渲染**成为优先级。
- 方法:5 路并行检索 + 来源核实(KaTeX/Typst/RaTeX 源码、MS OpenType spec、HarfBuzz、MathJax/dvisvgm 文档)。来源见文末。
- 一句话结论:**翻转 0013 —— RaTeX 作 v1**(纯 Rust、WASM 一等、原生吐"定位字形 + 横线"扁平 display list、自带 KaTeX 字体)。它的输出**1:1 映射本引擎的 FrameGlyph/SDF atlas 管线**,每个数学字形都进 atlas 成 SDF 实例、可走 0025 逐实例动画(=Manim 级逐字控制)。**KaTeX/MathJax 不直接用于取坐标**(原因见 §2)。

---

## 1. 核心结论:三个问题的答案

**Q1「能不能每个数学字形都是 SDF?」→ 能,但取决于排版引擎是否吐"定位字形列表"。**
数学排版 = 把 TeX 源排成一串带 `(字形, x, y, 字号, 字体)` 的盒子 + 几条横线(分数线/根号)。只要引擎吐这个**扁平 display list**,每个字形就能像正文一样栅化进 SDF atlas(本引擎现成管线),成为一个 `FrameGlyph` 实例 → 自动获得逐字 SDF 控制、缩放锐利、可叠特效。能吐扁平列表的:**RaTeX / Typst-math / ReX**(Rust),或**解析 MathJax/dvisvgm 的 SVG**(取 `<path>` + transform)。**不能**:KaTeX 的 DOM 输出(§2)、把数学整体烤成一张纹理(0013 的 C,opaque,无法逐字)。

**Q2「Manim 级控制?」→ 是同一条路。** Manim 的本质 = LaTeX→SVG 路径(`dvisvgm --no-fonts`)→ 每个字形是独立 VMobject 路径 → 逐字平移/淡入/morph(`TransformMatchingTex`)。本引擎的等价物:定位字形 → 各自 SDF atlas tile → 各自 GpuInstance → 走已有的 0025 per-instance 动画(transform/threshold/morph)。**我们天然就是 Manim 模型**,只是把"矢量路径动画"换成"SDF 实例动画"。

**Q3「能提前渲染吗?」→ 能,且应该。** 排版是**确定性**的(KaTeX 官方:同输入恒同输出,可 Node 预渲染)。按 **TeX 源 hash 缓存 display list**,首次见到某字形再**懒栅化进 SDF atlas**(Mapbox 风格的按需填 atlas 是标准做法)。不缓存渲染后的 HTML/SVG(那个体积爆炸:KaTeX SSR 把页面从 ~50KB 撑到 ~1MB),只缓存紧凑的定位字形表。

---

## 2. KaTeX 的布局到底怎么做(以及为什么不能从它取坐标)

KaTeX **内部是一个 TeX 盒子模型排版引擎**,但**对外不给坐标**——这是关键。

**2.1 管线**:`parse`(LaTeX→ParseNode 树)→ `buildHTML`/`buildCommon`(注册表分发,逐节点建盒)→ domTree(定位盒子树)→ `.toMarkup()` 出 HTML+CSS。`__renderToHTMLTree` 可拿到内部树对象。构建阶段**无状态、确定性**(故可 SSR)。

**2.2 盒子模型(box-and-glue)**:每个布局节点带 `height`/`depth`/`maxFontSize`(em);`SymbolNode` 另带 `width`/`italic`/`skew`。容器(hlist)的高/深由子节点自底向上取 max(`sizeElementFromChildren`)。横向"胶"(glue)= `makeGlue` 发一个 `margin-right` 为 em 的 `mspace` span。

**2.3 竖直堆叠 = `makeVList`(整个竖直定位的枢纽)**:分数的分子/分母/分数线、上下标、重音都靠它;`buildHTML` 里被调用约 20 次。它**算出**精确竖直偏移,但**只写进 CSS** —— `childWrap.style.top = -pstrut - currPos - elem.depth`。即竖直坐标以 CSS 字符串存在 wrapper span 上,不作为结构化字段挂在节点上。

**2.4 字体度量**:`fontMetricsData.js`(由 `extract_ttfs.py` 从 TTF 用 fontTools 生成)= `字体→(码点→[depth, height, italic, skew, width])`(注意 depth 在前,全 em)。全局 TeX 参数 σ1–σ22/ξ8–ξ13 在 `fontMetrics.ts`(`axisHeight=0.25`、分数线厚 `defaultRuleThickness≈0.04` 等),按 text/script/scriptscript 分档。italic 渲染为 span 的 `margin-right`,skew 用于重音定位。

**2.5 原子与间距**:8 个原子类(mord/mop/mbin/mrel/mopen/mclose/mpunct/minner),间距表在 `spacingData.ts`(thin=3mu/med=4mu/thick=5mu;`spacings`[显示/文本] 与 `tightSpacings`[脚本] 两张 `[左类][右类]` 表)。`buildExpression` 据相邻原子类插 `makeGlue`;实现了 TeXbook 的 bin 取消规则。

**2.6 样式**:`Style.ts` 8 个实例(D/T/S/SS × cramped);样式转移是按 id 的静态查表(`sup`/`sub`/`fracNum`/`fracDen`/`cramp`);`isTight()` = size≥2;字号倍率经 `Options.sizeMultiplier` 级联(buildGroup 据父子倍率比缩放 height/depth)。

**2.7 具体构造**:分数线厚走 `makeLineSpan`(`border-bottom-width`),分子分母用 `makeVList` + `axisHeight`/`num1`/`denom1`;上下标读 `italic` 修正定位;定界符分层(小=Main、大=Size1–4、再大=拼叠或 SVG);可伸缩元素(箭头/宽重音/根号 vinculum)是**手写 SVG 路径**(`svgGeometry.js`),不是字形。

**2.8 ⚠ 致命点(issues #537/#587):KaTeX 不给你 `(字形, x, y, 字号)` 扁平表。**
- 竖直坐标:**算了但只冻进 CSS `top`**(要恢复得去 parse span 的 `top`/`pstrut`)。
- 横向坐标:**根本没算** —— 交给浏览器的行内排版引擎。维护者明说:水平方向"浏览器会自己摆一长串字形,我们只希望知道整体宽度";`\overbrace`/`\cancel`/`\widehat` 等正是因为**没有 `makeVList` 的水平对应物**而做不了。
- 维护者拒绝默认逐字绝对定位(会破坏组合字符/上下文字形/双向文本)。
- 结论:把 KaTeX 当"取坐标的布局核"**不可行**(除非渲染进隐藏 DOM 再 `getBoundingClientRect` 测量回读——脆且需 DOM)。**这正是为什么我们要用原生吐 display list 的引擎(RaTeX/Typst/ReX),而不是 KaTeX。**

---

## 3. 选项对比(再评估)

| 方案 | 纯 Rust/WASM | 输出 | 逐字定位坐标? | 字体/度量 | OT MATH | 维护 | 逐字 SDF/动画 |
|---|---|---|---|---|---|---|---|
| **RaTeX** | ✅ 核心纯 Rust,`ratex-wasm` npm 一等 | **扁平 DisplayList**(字形+横线)→ 任意 2D 后端 | **✅**(display items;wasm 出 JSON) | ✅ 自带 KaTeX 字体/度量(`ratex-font`) | ✘(KaTeX 式提取度量) | **活跃**(~1.3k★,v0.1.11 2026-05) | **✅ 直接** |
| **Typst-math** | ✅,但重 | `MathFragment`→`Frame`(`FrameItem::Text` 带逐字 Point/font/size + Shape 横线) | **✅** | ✅ New CM Math(经 `World`) | **✅** | 很活跃 | ✅ 但需拖入整个 Typst World+字体 |
| **ReX** | ✅ | 布局树/render-node(SVG 默认,后端可插) | **✅**(字形+rule) | ✅ 内置 XITS Math(OT MATH) | **✅** | **低/停滞**,自称非生产级 | ✅ 但最不成熟 |
| **MathJax→SVG** | JS(浏览器/Node,无 WASM) | 自包含 SVG(`<path>`+`<use>` transform) | **✅ 可解析回**(`fontCache:'none'` 出独立内联 path;坐标 1000/em y 翻转) | 路径内联(不发字体) | 间接 | 活跃 | △ 需解析 SVG→路径→自栅 SDF |
| **dvisvgm/Manim** | C++ + 整套 LaTeX(服务端/离线) | `--no-fonts` 出字形 path | **✅** | 真字体→路径 | 真 LaTeX | 活跃 | △ 非 WASM,重 |
| **0013 的 C:SVG→纹理** | JS | **opaque 纹理** | **✘** | — | — | — | **✘ 无法逐字** |

要点:
- **能"逐字 SDF + 动画"的前提 = 拿到逐字坐标。** 只有 RaTeX/Typst/ReX(原生)或解析 MathJax/dvisvgm 的 SVG 能给;**烤纹理(0013 C)直接出局**。
- **RaTeX 与本引擎同构**:它显式照搬 KaTeX 架构,只把"DOM `<span>` 树"换成"DisplayList → Canvas/Skia/**自有矢量后端**"。我们的"自有矢量后端"就是 SDF atlas + FrameGlyph。且它**自带 KaTeX 字体 TTF**,可直接喂我们的栅化器做 SDF tile。
- **Typst-math 质量最高**(真 OT MATH:高度相关 kern cut-in、伸缩拼叠、大运算符),但**重**:数学内部是 `pub(crate)`,实际用法是"编一个 `$...$` 小文档 → 走 `Frame` 树",还要实现 `World`、塞字体——为"排一个公式"拖入整个编译器。
- **ReX** 是精瘦纯 Rust + 一个 OT MATH 字体,但停滞、自称非生产级。

---

## 4. OpenType MATH 表(要不要)

- MATH 表 = **字体侧数据层**(三块:`MathConstants` ~57 个竖直常量、`MathGlyphInfo` 含逐字 italic/重音点/**高度相关 kern cut-in**、`MathVariants` 伸缩/拼叠 + 大运算符)。spec **只给数据、不给算法**(引擎自己写排版)。
- **需要它的场景**:把上下标"塞进"字形凹角(ω f、V A 的高度相关 cut-in)、伸缩定界符/根号/重音、大运算符按 `displayOperatorMinHeight` 选号。没有它只能缩放单字形(变形)或退化到分号字形/SVG。
- **谁用**:HarfBuzz(`hb-ot-math`,只读不排版)、MathML Core(Chrome 109+/Firefox/WebKit)、Typst、unicode-math(XeLaTeX/LuaLaTeX)。带 MATH 表的字体:Latin Modern Math、STIX Two Math、Cambria Math、New CM Math、XITS 等。
- **KaTeX 不用 MATH 表**(已证实):用自提取的 `fontMetricsData`(CM/AMS 移植)+ 手写 SVG 几何;把 italic/skew 内联进 5-tuple,把 TeX 常量烤进布局代码。
- **对我们的含义**:两条成熟架构——(a)**KaTeX 式**(提取度量 + 烤死 TeX 常量):**只要绑定 TeX/CM 字体就够 TeX 级质量**(RaTeX 走这条);(b)**MATH 表驱动**(字体无关,Typst/ReX 走这条)。我们**不需要**自己实现 MATH 表解析——选 RaTeX 即享其烤好的 CM 路线。最难"手搓"的是高度相关 cut-in 与伸缩拼叠(KaTeX 也用 SVG 绕开后者),所以**不自研排版**。

---

## 5. 字形 → SDF tile + 体积

- **栅化工具**:`msdfgen`/`msdf-atlas-gen` 同时吃 **TTF 字形和 SVG path**;**MSDF/MTSDF(RGB 中值重建)保留尖角**——数学符号(√ 钩、∫ 端点、分数线/括号尖)对尖角敏感,**应优先 MSDF 而非单通道 SDF**(tiny-sdf/sdf-glyph-foundry 仅单通道,会圆角)。msdfgen 是 bake-time C++ 但可作库/有 Rust 绑定;纯 Rust 的 `fdsm`(部分实现)可 WASM 运行时栅化。本引擎现有正文 atlas 管线即可复用:把数学字体字形按 `(font, glyph, size)` 当作新角色栅化进 atlas。
- **体积**:KaTeX 全字体 ~1 MiB(三格式)/ 仅 WOFF2 ~300–400KB[估];KaTeX JS ~70KB gzip。**字体是大头**。RaTeX 自带这些字体(`embed-fonts`),数学不出现则零成本(懒加载)。MathJax 不发字体但 JS 运行时重(几百 KB)+ 每式内联路径。
- **缓存**:KaTeX 确定性 → 按 TeX hash 缓存定位字形表;按需懒填 SDF atlas(标准实践)。**别缓存渲染后 HTML/SVG**(体积爆炸)。
- **逐字动画先例**:Manim 证实——有了定位轮廓,每个字形独立可寻址、可平移/淡入/morph。我们的 0025 per-instance 动画提供等价能力。

---

## 6. 建议(更新 0013)

**翻转 0013:RaTeX 作 v1(数学一等公民,直接进 SDF),不再先走 SVG 纹理。**

理由链:本期优先级已是**逐字 SDF 控制 + 提前渲染**;opaque SVG 纹理(0013 C)与此**互斥**(不能逐字、缩放重栅);而 RaTeX 的原生 display list **正好**是本引擎 FrameGlyph 的形状,且纯 Rust/WASM/自带字体/活跃维护——零阻抗接入。KaTeX/MathJax 因 §2(不给坐标)与体积/JS 依赖,退为"参考/兜底",不进主路。

落地骨架(留给 ADR/Plan 细化):
1. **解析不变**:pulldown `ENABLE_MATH` 已吐 `InlineMath`/`MathDisplay` 原文 TeX(content.rs 已收)。
2. **排版**:引 `ratex`(Rust)对 TeX 源产 DisplayList;**核实 `ratex-types::DisplayItem` 字段**(字形 char/glyphId + x/y + size + font + rule 矩形)——这是接入前唯一必须读源确认的点。
3. **映射**:DisplayList 的字形项 → `FrameGlyph`(世界坐标 = 行内基线 + 局部偏移;新增数学 `StyleRole`/字体角色);横线项 → `FrameRect`(分数线/根号 vinculum)或 markdown widget。
4. **栅化**:数学字体字形进现有 SDF atlas(MSDF 优先以保尖角);按 `(font, glyph, size)` 分桶。
5. **行内 vs 显示**:`$...$` 作带 baseline 的行内盒接入 4A 折行;`$$...$$` 作块(已有 `MathDisplay` BlockKind)。
6. **缓存 + 动画**:按 TeX hash 缓存 DisplayList;字形走 0025 进场/morph(逐字"写出"公式、跨式 morph 是后续亮点)。
7. **兜底**:RaTeX 未覆盖的极端 LaTeX → 退 MathJax→SVG embed(0013 C 作 fallback,不是主路)。

**升级/触发**:若要**字体无关**数学(STIX/Cambria)或更高保真伸缩拼叠 → 评估 Typst-math(经 `Frame` 取定位字形)或自研 MATH-表驱动;在此之前 RaTeX 的 CM 路线足够。

---

## Sources

**Rust 引擎**:[RaTeX repo](https://github.com/erweixin/RaTeX) · [RaTeX 站](https://ratex.lites.dev/) · [ratex-wasm npm](https://www.npmjs.com/package/ratex-wasm) · [Typst math fragment 源](https://github.com/typst/typst/blob/c98e9103/crates/typst-layout/src/math/fragment/mod.rs) · [Typst math DeepWiki](https://deepwiki.com/typst/typst/3.4-math-typesetting-elements) · [Typst World 抽象](https://deepwiki.com/typst/typst/2.3-world-abstraction-and-resource-loading) · [ReX](https://github.com/ReTeX/ReX) · [otfmath](https://github.com/cbreeden/otfmath)

**KaTeX 内部**:[overview](https://deepwiki.com/KaTeX/KaTeX/1-overview) · [rendering pipeline](https://deepwiki.com/KaTeX/KaTeX/2.3-rendering-pipeline) · [font metrics](https://deepwiki.com/KaTeX/KaTeX/4.2-font-metrics) · [font system](https://deepwiki.com/KaTeX/KaTeX/4-font-system) · [buildCommon.ts](https://raw.githubusercontent.com/KaTeX/KaTeX/main/src/buildCommon.ts) · [buildHTML.ts](https://raw.githubusercontent.com/KaTeX/KaTeX/main/src/buildHTML.ts) · [domTree.ts](https://raw.githubusercontent.com/KaTeX/KaTeX/main/src/domTree.ts) · [spacingData.ts](https://raw.githubusercontent.com/KaTeX/KaTeX/main/src/spacingData.ts) · [Style.ts](https://raw.githubusercontent.com/KaTeX/KaTeX/main/src/Style.ts) · [issue #537](https://github.com/KaTeX/KaTeX/issues/537) · [issue #587](https://github.com/KaTeX/KaTeX/issues/587)

**MathJax / dvisvgm**:[MathJax SVG output](https://docs.mathjax.org/en/latest/options/output/svg.html) · [MathJax server components](https://docs.mathjax.org/en/v4.0/server/components.html) · [mathjax-full](https://www.npmjs.com/package/mathjax-full) · [dvisvgm manpage](https://dvisvgm.de/Manpage/) · [dvisvgm repo](https://github.com/mgieseki/dvisvgm) · [Manim tex/svg](https://docs.manim.community/en/stable/reference/manim.mobject.svg.svg_mobject.VMobjectFromSVGPath.html) · [TransformMatchingTex](https://docs.manim.community/en/stable/reference/manim.animation.transform_matching_parts.TransformMatchingTex.html)

**OpenType MATH**:[MS OpenType MATH spec](https://learn.microsoft.com/en-us/typography/opentype/spec/math) · [HarfBuzz hb-ot-math](https://harfbuzz.github.io/harfbuzz-hb-ot-math.html) · [MathML Core explainer](https://w3c.github.io/mathml-core/docs/explainer.html) · [Chrome 109 MathML](https://frederic-wang.fr/2023/01/31/mathml-in-chrome-109/) · [unicode-math](https://github.com/latex3/unicode-math) · [MDN MathML fonts](https://developer.mozilla.org/en-US/docs/Mozilla/MathML_Project/Fonts)

**SDF / 体积**:[msdfgen](https://github.com/Chlumsky/msdfgen) · [msdf-atlas-gen](https://github.com/Chlumsky/msdf-atlas-gen) · [tiny-sdf](https://github.com/mapbox/tiny-sdf) · [fdsm(Rust)](https://crates.io/crates/fdsm) · [redblobgames SDF fonts](https://www.redblobgames.com/articles/sdf-fonts/) · [KaTeX font 文档](https://github.com/KaTeX/KaTeX/blob/main/docs/font.md) · [KaTeX SSR 体积讨论](https://news.ycombinator.com/item?id=27657305)

> 核实备注:RaTeX `DisplayItem` 确切字段未读源(接入前必读 `crates/ratex-types` + `ratex-wasm` JSON);Typst-math 单公式独立 WASM 用法、ReX crates.io 版本未直证;MathJax/tex-svg 精确 KB 未钉。结论不依赖这些细节。
