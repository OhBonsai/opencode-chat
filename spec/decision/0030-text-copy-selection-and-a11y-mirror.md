# 决策记录 0030:canvas 文本可复制 / 选区 / 无障碍 —— DOM 管语义,GPU 管视觉

- 日期:2026-06-24
- 状态:**提议中(草案,待评审)**
- 前置:[0001](0001-canvas-architecture.md)(React 管控件 + GPU 画布)、[0007](0007-rich-media-embeds.md)/[0022](0022-dom-overlay-layer.md)(DOM overlay 相机同步,本篇复用其机制)、[0011 §3.3④](0011-gpu-text-as-sdf-primitive.md)(CPU 基础盒模型 hit-test)、[0012](0012-debugger-gui-html-vs-egui.md)(自绘几何)、[0025](0025-sdf-node-animation-system.md)(SDF 选区"墨团")、[0029](0029-session-virtualization-and-glyph-working-set.md)/[Plan 19](../plan/plan19-session-virtualization.md)(虚拟化/Hot tier,本篇 DOM 层必须复用)、README 原则 9(canvas 对读屏是黑盒,可嵌入组件必须配 DOM 镜像)、[TODO Q/R](../../TODO.md)(选区复制 / a11y 镜像)
- 定位:统一回答"**canvas 渲染的文字怎么可复制/可选/可搜/可读屏**"。结论:**一层(虚拟化的)透明 DOM 文本层承载语义(选区/复制/Cmd+F/读屏),选区高亮的视觉交 GPU 画在文字下面**;一层 DOM 同时兑现 TODO Q(复制)+ TODO R(a11y),不拆两套。基于 PDF.js / xterm.js / Google Docs / Figma 的业界实证(见 §10)。

---

## 1. 问题与本项目的简化红利

canvas/WebGPU 画的文字**没有 DOM 节点**,浏览器看到的是像素 → 原生选区、复制、Cmd+F、屏幕阅读器全部失效。这是所有 GPU 文本应用的通病。

**本项目的关键简化**:画布是**只读展示**(对话渲染),用户输入走**独立的 DOM 输入框**(`web/src/chat-input.ts`),**画布上没有文本编辑**。于是业界两大复杂度来源**本项目都不需要**:

- **不需要 on-canvas 编辑** → 排除 tldraw/Excalidraw 的"编辑期换入真 `<textarea>`/contenteditable"家族,也排除 **EditContext API**(它解决的是 canvas 编辑器的 IME/输入,Chromium-only)。
- 只需"**持久并行 DOM 承载选区/复制/读屏**"这一家族(PDF.js 透明文本层 / xterm.js a11y 树),且因只读,比通用编辑器简单得多。

## 2. 业界两大家族(实证)

| 家族 | 代表 | 机制 | 本项目 |
|---|---|---|---|
| **持久并行 DOM** | **PDF.js**(透明文本层)、**xterm.js**(off-screen a11y 树 + 自定义选区)、**Google Docs**(canvas + side-DOM a11y)、**Figma**(全自定义 + a11y 层) | canvas 画像素,平行一层 DOM 文本(可见但透明 / 或屏外)承载原生选区/复制/搜索/读屏 | ✅ **采用**(只读展示正合适) |
| **按需 DOM 编辑器** | **tldraw**(TipTap contenteditable)、**Excalidraw**(`<textarea>` overlay) | 静态画 canvas,**仅编辑期**换入真 DOM 编辑器 | ❌ 不需要(画布只读) |

两条业界经验直接定调本篇:

- **PDF.js**:透明文本层(`color:transparent` 的绝对定位 `<span>`),选区高亮靠**半透明 `::selection`**(`color-mix(AccentColor, transparent 50%)`)+ 强制文字 `transparent`,让 canvas 字透过高亮可读;并发现需 `color-scheme: only light` 防强制反色、`scaleX` 逐 run 对齐(对齐是其**长期痛点**)。
- **xterm.js(WebGL 渲染器)**:**选区画在文字下面**,用统一 `selectionForeground` + `minimumContrastRatio` 保证选中字可读,**官方明说"for accessibility reasons"**。这是"高亮不遮文字"的**最干净工业答案**,本篇据此定 §3。

## 3. 决策:三层分工

```
① DOM 透明文本层(仅可见块,虚拟化)  ── 管「语义」:原生选区 / 复制 / Cmd+F / 读屏。文字 transparent。
② GPU 选区高亮(画在文字下,rect pass) ── 管「视觉」:读 ① 的选中字符范围 → 用引擎自己的 placed 几何画高亮。
③ core 不变(CR1)                      ── 纯 web/host,确定性/R8 不受影响。
```

**关键洞察(绕开 PDF.js 的对齐地狱)**:因为**文字是我们自己渲染、精确几何 `placed` 在手**,选区高亮**用我们自己的几何画**(把 DOM 选区映射成字符偏移区间 → 取对应 `placed` 盒 → 发 `RectInstance`)→ **自动像素级对齐**。于是 ① 的 DOM 文本层**只需字符顺序正确**(决定"浏览器选中了哪些字"+复制内容+读屏),**不需要**和 canvas 字逐像素对齐——PDF.js 那套 `scaleX`/min-font-size 对齐折腾**本项目不用做**。

绘制顺序天然支持:render 的 pass 是 `panel→rect→widget→image→shaderbox→**glyph**`,**文字在最上层**。选区高亮作 `RectInstance` 在 rect pass(glyph 之前)→ **文字永远全不透明在上,任意多色都不被洗淡**,选区色走 `theme.rs` 令牌、暗主题可控,后续可升级成 [0025] 的 **SDF 圆角"墨团"**(跨行 rect 并集 `smin`)。

## 4. 为什么这是"更好的"(稳定性 / 性能 / 效果)

- **稳定性**:核心链路全用**跨浏览器 baseline API**——透明 DOM 文本层(选区/复制/读屏)、`Range.getClientRects()`(选区→几何桥,通用)、`navigator.clipboard.write()`(text/plain[+html],需用户手势)、隐藏 DOM 镜像(读屏唯一可移植答案)。**Chromium-only 的 EditContext / ariaNotify 只作渐进增强、绝非硬依赖**。"选区画文字下"有 xterm.js 工业验证。
- **性能**:① **DOM 文本层必须虚拟化 = 只镜像可见块**,直接复用 [0029]/[Plan 19] 的 **Hot tier**(可见窗)——绝不能塞 10k 行 DOM,否则正好毁掉刚做完的无限会话内存目标(PDF.js/xterm.js 都虚拟化:PDF.js 只渲可见页、xterm.js a11y 树每动画帧重建一次且仅推 ~20 行)。② `getClientRects()` 强制 reflow → **只在选区变化时读、不每帧**。③ GPU 高亮**零额外 pass**(复用 rect pass)。
- **效果**:文字永全不透明在最上层(多色不被高亮洗淡)、选区色暗主题可控、可做 SDF 墨团;复制内容是**渲染后纯文本**(去掉 `**`/`|` markdown 噪声),可附 `text/html` 或 markdown 源提保真。

## 5. 现代 Web API 逐个取舍(2025–2026 支持现状)

| API | 现状 | 本项目 |
|---|---|---|
| **透明 DOM 文本层 + `Range.getClientRects()`** | baseline,全浏览器多年 | ✅ **主路**:层载选区/复制/读屏,getClientRects 作选区→GPU 几何桥 |
| **Clipboard `write()`**(text/plain + text/html) | baseline(需安全上下文 + 用户手势;`clipboard-write` 权限名 Chromium-only,别据此 gate) | ✅ 复制用 |
| **CSS Custom Highlight API**(`CSS.highlights`) | Baseline 2025(Chrome105+/Safari17.2+/**Firefox140+ 2025-06**);**只高亮 DOM 文本,不高亮 canvas 像素** | 🟡 我们高亮在 GPU,故**不用它画选区**;可作"无 GPU 高亮"降级 / DOM 镜像内搜索高亮可选 |
| **EditContext API** | Chromium-only(Chrome121+),FF/Safari 无;解决 canvas **编辑**的 IME | ❌ 画布只读、无编辑 → **不需要** |
| **ariaNotify** | Chromium-only(Chrome141+,2025-10;ChromeOS 未支持) | 🟡 可选增强(读屏播报),fallback `aria-live` |
| **AOM 虚拟 a11y 树** | 实验/停滞,Blink 需 flag、WebKit 拟移除 | ❌ 不用;坚持隐藏 DOM 镜像 |

## 6. 与现有决策对接

- **一层 DOM 文本同时满足 TODO Q(复制)+ TODO R(a11y 镜像)**——别把"复制"走自建 GPU 选区、"a11y"另做一套;透明文本层兼任二者。原则 9 的"可嵌入组件硬需求"(读屏)即由此层兑现。
- **复用 [0022]/embed-overlay 的相机同步机制**:`embed-overlay.ts` 已证明"每帧把 DOM 元素按 `frame_embeds()` 屏幕坐标摆好"可行;文本层照搬(把 `<img>` 换成透明 `<span>`)。引擎新增 `visible_text_runs()`(可见块的屏幕盒 + 文本 + 字符→glyph 映射)即可,类比已有 `frame_embeds()`。
- **选区视觉接 [0025]**:GPU 高亮可直接用其规划的"多行选区 `smin` 圆角墨团";选区色入 `theme.rs` 令牌。
- **hit-test 用 [0011 §3.3④] CPU 基础盒**:已有 `code_block_at_screen` 的屏幕→世界换算可复用;但有了 DOM 文本层,选区命中**主要交给浏览器原生**,CPU 盒仅备用/特殊命中。

## 7. 关键不变量与护栏

1. **虚拟化(硬约束)**:DOM 文本层 = **Hot 可见块**,随滚动/相机重建(与 [0029] tier 同步)。**绝不全量 10k 节点**——否则毁无限会话内存。
2. **高亮用引擎几何**:选区→字符偏移→`placed` 盒→`RectInstance`,自动对齐;DOM 层只需**字符序正确**,不追像素对齐。
3. **选区不遮文字**:主路 GPU 画文字下;若某降级路径无 GPU 高亮,则半透明 `::selection`(PDF.js 式,`/0.3`)+ 文字 `transparent` 兜底。
4. **暗主题**:文本层 `color-scheme: only light` 防 OS 强制反色扰动透明/对齐;选区色走 theme,不继承浏览器默认蓝。
5. **复制内容**:默认渲染后纯文本(去 markdown 噪声);可选 `text/html` 或 markdown 源;**必须用户手势触发**(Clipboard 约束)。
6. **确定性(CR1/R8)**:全在 web/host 表现层,core 不碰 → 不影响录像重放。
7. **Cmd+F × 虚拟化冲突**(已知):屏外块无 DOM → 原生 Cmd+F 只覆盖可见。缓解:用 `Store` 全文建轻量索引自建 find(跳转+选中),或先接受"仅可见搜索"。**标记后续,非首期**。

## 8. 决策与不决定的

**采纳**:**透明虚拟化 DOM 文本层(语义:选区/复制/Cmd+F/读屏)+ GPU 画选区高亮于文字下(视觉,用引擎几何)+ core 不变**;复制走 Clipboard `write`;选区几何桥走 `getClientRects`;一层 DOM 兼任 Q 复制 + R a11y;DOM 文本层复用 [0029] Hot tier 虚拟化。

**理由**:画布只读 → 排除编辑类复杂度(EditContext / 编辑 overlay);全用 baseline 跨浏览器 API 保稳定;虚拟化 + GPU 高亮保性能;自渲染几何画高亮绕开 PDF.js 对齐痛点、并保文字永不被遮(xterm.js 验证),效果与暗主题/SDF 墨团兼容。

**不决定的(留实现/后续)**:DOM 文本层粒度(按行 span vs 按 run,首期按行足够);自建全文 find 的索引形态;复制是否带 `text/html`/markdown(首期纯文本);ariaNotify 增强(Chromium,选配);SDF 墨团选区的具体 [0025] 接法;选区跨虚拟化边界(选区一端在屏外 Warm 块)的处理(首期限可见窗内选区)。

## 9. 落地清单(渐进,接 Plan;先简后繁)

1. **Quick win — 每消息"复制"按钮**(host):从 engine 取该 turn 渲染后纯文本 → `clipboard.writeText`,用户点击触发。**半天,覆盖最高频需求,零选区。**
2. **MVP — 选区 + 复制**:`visible_text_runs()`(core/wasm 暴露可见块文本+几何)→ host 建**虚拟化透明文本层**(仅 Hot 块,复用 embed-overlay 相机同步)→ 原生选区 + Cmd+C 复制 → `getClientRects()` → **GPU 画高亮于文字下**(RectInstance)。
3. **a11y 镜像**:同一层补 ARIA(角色/顺序/`aria-posinset`/`setsize` 虚拟列表,xterm.js 式)→ 读屏可用;兑现 TODO R / 原则 9。
4. **增强(按需)**:SDF 墨团选区([0025])、自建 Cmd+F(Store 索引)、`text/html`/markdown 复制、ariaNotify(Chromium)。

## 10. 参考(Sources)

**实证实现**:[PDF.js text_layer_builder.css](https://raw.githubusercontent.com/mozilla/pdf.js/master/web/text_layer_builder.css)(透明文本层 / 半透明 `::selection` color-mix / `scaleX` 对齐 / `color-scheme:only light`)· [xterm.js Screen Reader Mode 设计文档](https://github.com/xtermjs/xterm.js/wiki/Design-Document:-Screen-Reader-Mode)(off-screen a11y 树 / 虚拟列表 / live region)· [xterm.js WebGL 渲染器 PR #1790](https://github.com/xtermjs/xterm.js/pull/1790)(**选区画文字下,for accessibility**)· [xterm.js minimumContrastRatio #4752](https://github.com/xtermjs/xterm.js/issues/4752)· [Google Docs canvas 渲染公告](https://workspaceupdates.googleblog.com/2021/05/Google-Docs-Canvas-Based-Rendering-Update.html)· [Figma: building a design tool on the web](https://www.figma.com/blog/building-a-professional-design-tool-on-the-web/)(自有 DOM/排版引擎 / clipboard = text/html 藏二进制)· [tldraw RichTextArea](https://tldraw.dev/reference/tldraw/RichTextArea)· [Excalidraw 内部粘贴 PR #10643](https://github.com/excalidraw/excalidraw/pull/10643)· [WebAIM: Google Docs canvas a11y 评论(二手)](https://webaim.org/blog/seismic-change-to-docs/)

**现代 Web API**:[EditContext API — MDN](https://developer.mozilla.org/en-US/docs/Web/API/EditContext_API)(Chromium-only,canvas 编辑 IME)· [Introducing EditContext — Chrome](https://developer.chrome.com/blog/introducing-editcontext-api)· [CSS Custom Highlight API — MDN](https://developer.mozilla.org/en-US/docs/Web/API/CSS_Custom_Highlight_API)(Baseline 2025;只高亮 DOM 文本)· [Clipboard write() — MDN](https://developer.mozilla.org/en-US/docs/Web/API/Clipboard/write)· [Range.getClientRects() — MDN](https://developer.mozilla.org/en-US/docs/Web/API/Range/getClientRects)· [Element.ariaNotify() — MDN](https://developer.mozilla.org/en-US/docs/Web/API/Element/ariaNotify)(Chrome 141+)· [AOM explainer — WICG](https://wicg.github.io/aom/explainer.html)(实验/停滞)

> 可信度:PDF.js/xterm.js 一手源码与设计文档、MDN/Chrome/WICG 平台文档、各厂商工程博客 = 高;Google Docs/Figma 的 a11y 内部细节部分依二手分析(WebAIM 等),已标注。
