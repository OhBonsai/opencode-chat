# 调研:图片渲染(warp / jcode)+ 对本项目 SVG/PNG 分流与 streaming-svg 的启示

- 日期:2026-06-19
- 目的:为本项目「图片嵌入」相位(TODO O / [0007](../decision/0007-rich-media-embeds.md) / [0022](../decision/0022-dom-overlay-layer.md) / thinking §1 图片 / plan13 `Embed` 叶子)定方向——**SVG(矢量)与 PNG(位图)分流**,并为作者明确想要的 **streaming-svg** 找先例 + 判断可行路径。
- 方法:实读 `~/w/agentscode/warp`(Warp 终端,GPU/wgpu 桌面端)与 `~/w/agentscode/jcode`(TUI 终端,ratatui/CPU)源码,带 file:line。
- 关联:[0013 数学渲染](../decision/0013-math-latex-rendering.md)(同一"矢量 vs 不透明纹理"取舍的已决先例)、[0011 一切皆 SDF](../decision/0011-gpu-text-as-sdf-primitive.md)、[plan12 数学一等 SDF](../plan/plan12-math-sdf-first-class.md)。

---

## 0. 一句话结论

- **warp 是唯一同形态参照**(GPU 自渲染 + wgpu + textured quad);**jcode 几乎不可借**——它是 CPU TUI,靠**终端图片协议**(Kitty / iTerm2 OSC1337 / Sixel)把图丢给终端模拟器画,本项目自己就是"终端模拟器",这条路不存在。
- **warp 与 jcode 都把 SVG 栅格化**(`usvg` 解析 → `resvg`+`tiny_skia` 渲成位图 → 当普通位图上屏);**两者都没有 streaming-svg、没有渐进/增量图像**。
- 故 **streaming-svg 在两个参照里都无先例 = 差异化点**;而且它和本项目刚为数学做过的决策([0013] 翻转 B / Plan 12:LaTeX → 矢量 SDF 字形进 quad 管线,**否决不透明纹理**)**同构**——要 streaming/缩放锐利/逐元素动画,就**不能走"栅格成不透明纹理"**那条路。

---

## 1. warp(GPU 桌面终端 · 主要参照)

形态与本项目一致:Rust + wgpu,自己画 UI,图片 = textured quad。

**① 表示 / 解析**
- markdown 图片:`crates/markdown_parser/src/lib.rs:334` `FormattedImage{ alt_text, source, title }`;nom 解析 `parse_image*`。
- 解码后统一为 `crates/warpui_core/src/image_cache.rs:459` 的 **`ImageType`**:
  ```rust
  enum ImageType { Svg{svg: Rc<usvg::Tree>}, StaticBitmap{..}, AnimatedBitmap{..}, Unrecognized }
  ```
  → **SVG 单独一支、保留为矢量树**;位图另一支。**无终端图片协议**(自渲染,不需要)。

**② 位图路径(PNG/JPEG/WebP/GIF)**
- `image` crate 解码(`image_cache.rs:320-365`)→ CPU 可选按显示尺寸 resize(`resize_image` 576-620,`CacheOption::BySize` 缓存 / `Original` 交 GPU 缩放)→ GPU 纹理 `Rgba8Unorm`(`rendering/wgpu/renderer/image.rs:240`,`queue.write_texture`)→ `texture_cache`(per-asset,10 帧淘汰)。

**③ SVG(关键)**
- 解析:`usvg::Tree::from_data`(`image_cache.rs:273`),**保留 `Rc<usvg::Tree>`**。
- 上屏:`svg_image(svg, bounds, fit)`(`image_cache.rs:551`)用 `resvg::render` + `tiny_skia::Pixmap` **按目标 bounds 栅格化成 RGBA 位图** → 之后与位图无异走 GPU 纹理。
- **要点**:矢量树留着、**按显示尺寸重栅**(换 bounds 重新 raster)→ 不同缩放下相对锐利;但**最终上屏仍是不透明位图纹理**,不是矢量几何/SDF。`resvg`+`tiny_skia` = CPU 栅格。

**④ 流式 / 增量**
- 只有**加载态 FSM**(`elements/image.rs:461` `Loading / FailedToLoad / Loaded` + 超时占位 + 完成 `repaint_after_load`);动图 = 多帧。**无下载中渐进解码、无"图边长边改"的增量重栅**。

**⑤ 布局 / reflow**
- `FitType{Contain|Cover|Stretch}`(`image_cache.rs:438`);图片 = 块级、占整行(`FormattedTextLine::Image` 当 LineBreak);**加载期用 `ConstrainedBox` 预留高度防 reflow**(lightbox.rs:151)。

**⑥ GPU 渲染**
- scene `Image{bounds, asset, opacity, corner_radius}`(`scene.rs:108`);**instance 化 textured quad**(`renderer/image.rs`,`image_shader.wgsl`,复用 quad mesh,per-asset 一张纹理,**无 atlas**)——**与 glyph 同套 instance 渲染范式**。

## 2. jcode(TUI 终端 · 形态不同,大部分 N/A)

CPU/ratatui,**不自己画像素**,靠终端协议让宿主终端画。

- **终端图片协议**:`crates/jcode-terminal-image`——`ImageProtocol{Kitty, ITerm2, Sixel, None}` + 环境探测(`display.rs:22`),`display_kitty/iterm2/sixel`(`display.rs:271`)发 base64 + 转义序列;`ratatui-image` 当 widget。**本项目用不上**(我们=自渲染 GPU,不是往终端吐转义)。
- **markdown 拍平**:render-core 把 `![alt](url)` **拍成文本** `[image: alt](url)`(`jcode-render-core/src/markdown.rs:659`);`BlockKind` **无 Image 变体**(`model.rs:159`)。= 本项目 `vendor/jcode-render-core` 现状同源。
- **位图**:`image` crate 解码 → PNG 落盘缓存 → ratatui-image → 协议序列;惰性 header 嗅探尺寸(PNG IHDR)做占位。
- **SVG**:**仅 mermaid 用**——`usvg`+`resvg`+`tiny_skia` 栅成 **PNG**(`jcode-tui-mermaid/src/mermaid_svg.rs:265`)→ 走位图协议。**无矢量上屏、无 streaming-svg**。
- **流式**:占位 + 可见才解码(`ui_inline_image.rs:86` materialize-on-visible)+ payload LRU;**无渐进/增量 API**。
- **GPU**:**无**(TUI 纯 CPU/终端);桌面 GPU 渲染器是另一条线,本次未涉图片。

> jcode 的可借项只有两点:① markdown 图片"先拍平成文本占位"的兜底(本项目 vendor 已有);② SVG→resvg→raster 的 mermaid 套路(与 warp 同,见 §3)。终端协议整条**否决**(形态不符)。

## 3. 共性 + 对本项目的可借 / 不可借

| 维度 | warp | jcode | 本项目取舍 |
|---|---|---|---|
| 自渲染 GPU textured quad | ✅(主参照) | ❌(终端协议) | **借 warp**:Embed = textured quad,instance 同 glyph |
| 位图解码 | `image` crate(Rust) | `image` crate | **改走浏览器**(`Image`/`OffscreenCanvas`,0007;wasm 零打包、白嫖浏览器解码) |
| SVG | usvg+resvg→位图纹理 | usvg+resvg→PNG | **两者都栅格;本项目要分流**(见 §4/§5) |
| 加载 FSM + 占位防 reflow | ✅ | ✅(占位) | **借**:= 0007 embed FSM + plan13 `Embed` 叶子预留盒 |
| FitType / 尺寸 | Contain/Cover/Stretch | fit + cell 比例 | **借** Contain 默认 + max-width(plan13 盒) |
| 终端图片协议 | ❌ | ✅ | **否决**(自渲染不需要) |
| streaming-svg / 渐进 | ❌ | ❌ | **无先例 → 自研(§5)** |

## 4. SVG vs PNG 分流(本项目)

作者要求"两种分开处理"。落到本项目(GPU 自渲染 + 无限缩放画布 + 想要流式):

- **PNG / 位图(非矢量)**:浏览器解码 → `ImageBitmap` → 上传 GPU 纹理 quad(0007)。简单。缩放靠纹理采样(放大会糊,可按显示尺寸 mipmaps/重采样,同 warp `BySize`)。走 plan13 `Embed` 叶子(`reportSize` 报固有/宽高比 → Taffy 预留盒,防 reflow)。**FSM**:Placeholder(估高)→ Loading → Ready(纹理 swap)→ Failed。
- **SVG / 矢量**:有两条路,**这正是 §5 的分叉**。warp/jcode 都选了"栅格成纹理"(最简单);但本项目有无限缩放 + streaming-svg 诉求 + 已为数学走过矢量原生(0013),所以 SVG 不能简单照抄"栅格成不透明纹理"。

## 5. ★ streaming-svg:两参照都没有 → 与数学(0013/Plan12)同构的取舍

**为什么 warp/jcode 没有**:它们把 SVG **栅格成一张不透明位图**。不透明位图是"死"的——无法逐路径着色、无法 tween、无法"边到边长"。这与 [0013 §8] / thinking §2 对数学的判断**一模一样**:"opaque 纹理(0013 C)里的字形无法 tween"。

**本项目刚做过这个决策(Plan 12)**:LaTeX **不**走 MathJax→SVG→纹理,而是 **RaTeX 排版 → 矢量 → 每符号一个 SDF atlas 字形进 quad 管线**(缩放锐利、逐符号上色、可逐字动画/跨式 morph)。**streaming-svg 之于 SVG,正是 math-as-SDF 之于 LaTeX。**

故 SVG 两条路,对应能不能 streaming:

| 路 | 做法 | 缩放 | streaming-svg / 动画 | 代价 |
|---|---|---|---|---|
| **A 栅格纹理**(warp/jcode) | usvg→resvg→位图→纹理(或浏览器 `<img>`/canvas 栅格) | 重栅才锐利,否则糊 | **不能**(不透明,逐元素无身份) | 最低,现成 |
| **B 矢量原生**(本项目方向) | 解析 SVG → 路径**三角化**(lyon)或 **SDF** → 进现有 quad/SDF 管线,逐 path 一个图元 | **任意缩放锐利**(同 0011) | **能**:逐 path 有身份 → 增量加 path = 流式长出;path 参数 lerp = morph(0016) | 高:要 SVG path → 几何/SDF 的 tessellator |

**streaming-svg 的本质**(承 thinking §1C/§2):LLM 流式吐 SVG 文本时,`<path>`/`<rect>`/`<circle>` 一个个到达 → **逐图元揭示 + reflow**(viewBox/路径随后续 token 变),而不是"等 `</svg>` 闭合再一次性栅格一张图"。这**只有路 B 做得到**——路 A 每次变都得整张重栅(warp 的"换 bounds 重栅"是缩放重栅,不是内容增量),且重栅是不透明替换 = 闪,违背北极星(thinking §3 / 0019)。

**落点(若走 B)**:复用本项目已有的 SDF/quad 管线与 0016 补间——
- SVG 基本图元(rect/circle/line/round-rect)本就是 SDF(`shaders/base/sdf.wgsl` 已有 `sd_round_box`/`sd_circle`/`sd_seg`,0026);
- 任意 `<path>` 贝塞尔 → 要么 **lyon 三角化**成 mesh(新管线),要么 **路径 SDF**(贵);
- 逐 path = 一个 0020 节点 / FrameWidget(0026 组件图元已有"按 component-id 分派"的扩展位)→ 流式增量 spawn + 0016 reflow + 0025 逐元素 anim,与 markdown 其它结构同源。

## 6. 接本项目现有决策

- **0007**:已写"图片/静态 SVG → 纹理 quad;**流式 SVG → 可变纹理(节流重光栅化)**"。本调研**修正**该条:可变纹理(= 路 A 节流重栅)能做"内容变了重画一张",但**做不到逐元素流式/缩放锐利/可 morph**;真 streaming-svg 须路 B(矢量原生)。建议 0007 之上新开 ADR 记此翻转(类比 0013 §8 对数学的翻转 B)。
- **0022 DOM overlay**:第三条路——**SVG 直接交浏览器 DOM**(`<svg>` 绝对定位叠画布,相机同步)。零栅格、浏览器原生矢量、甚至 SMIL/CSS 动画白嫖;代价 = 不在 canvas 内、与 SDF 特效/morph 体系隔离、事件/层级另管。**适合"复杂交互 SVG / 一次性展示"**,不适合"要和文字同台 morph/流式的 SVG"。→ 与路 B 分工:**展示用 0022,流式/动画一等公民用路 B**。
- **plan13 `Embed` 叶子**:无论 A/B/DOM,图片都作 Taffy `Embed` 叶子(`reportSize` measure → 预留盒防 reflow);plan13 §4 已留位。
- **0013 / Plan 12**:直接复用其"矢量→SDF/quad 一等公民"的范式与教训(确定性、按 hash 缓存、ok=false 兜底回退栅格)。

## 7. 建议 + 开放问题

**建议(分流 + 分期)**:
1. **PNG/位图**:走浏览器解码 → 纹理 quad + Embed 叶子 + FSM(0007 原案,low-risk,先做)。
2. **SVG 展示态(静态/复杂)**:短期可走**栅格纹理**(浏览器 `<img>`/canvas,= 路 A,最省)或 **0022 DOM overlay**(矢量、白嫖动画)。
3. **SVG streaming/动画一等公民**:走**路 B 矢量原生**(SVG 图元 → SDF/三角化 → quad 管线,逐 path 身份 + 0016/0025),= 与数学同构的"非不透明纹理"路线。**这是 streaming-svg 的唯一出路**,也是差异化点,排在 markdown 结构流式(thinking §1)成熟之后。

**开放问题**:
- 路 B 的 `<path>` 贝塞尔上屏:**lyon 三角化 mesh** vs **路径 SDF** vs **混合**(基本形 SDF + 复杂 path 三角化)?需 spike(类比 plan12 §0 RaTeX spike)。
- SVG 解析器:`usvg`(纯 Rust、wasm 安全,warp/jcode 同款)只解析不栅格——可只取其**解析/规范化**(拿到路径/形状/变换),栅格那步换成路 B,**复用 usvg 当"SVG 的 RaTeX"**。
- 流式 SVG 的歧义/确认延迟(thinking §1A):`<svg` 未闭合时怎么判定、占位尺寸(viewBox 早到?)。
- 是否新开 **ADR**:"SVG 矢量原生 vs 栅格纹理"(翻 0007 流式 SVG 条;承 0013 范式)。建议落地前先开。

---

> 参照源码:warp `crates/{markdown_parser, warpui_core/src/image_cache.rs, warpui/src/rendering/wgpu/renderer/image.rs}`;jcode `crates/{jcode-terminal-image, jcode-tui-mermaid/src/mermaid_svg.rs, jcode-render-core/src/markdown.rs}` + `terminal-capabilities.md`。
