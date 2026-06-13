# 决策记录 0007:富媒体嵌入与 DOM/GPU 像素对齐

- 日期:2026-06-13
- 状态:已采纳(原型验证前)
- 前置:0001(架构,React 控件 + GPU 画布)、0002(管线 + 效果)、0004(embed block)
- 范围:图片/SVG/卡片等富内容的渲染分层;是否引入 egui;DOM overlay 同步缝的解法

## 1. 结论:不引入 egui 做内容

egui 与两个根基冲突:React 管控件(0001)、统一字体引擎(pretext/DOM 都走浏览器
字体引擎;egui 自带弱排版正是 0001 否决 cosmic-text 的同一理由)。引入 egui =
第二套 UI 范式 + 第二套输入/焦点/IME + 第二个字形 atlas + 与自定义 wgpu pass 抢
z-order/裁剪。代价远超收益。egui 仅留作开发期调试面板(0001 已定)。

富内容按交互程度分三层,各用合适机制。

## 2. 三层 embed 模型

| 内容 | 机制 | 框架 |
|---|---|---|
| 图片 / mermaid / 静态 SVG | 纹理 quad(浏览器光栅化,0004) | 无 |
| 流式 SVG | 可变纹理(节流重光栅化) | 无 |
| 只读产出卡片 | 自绘原语(圆角矩形 + pretext 文字 + 图标纹理) | 无 |
| 交互卡片 / HTML artifact | DOM overlay(React) | React |
| 调试面板 | egui(仅开发期) | egui |

### 2.1 纹理嵌入(被动视觉,默认,覆盖多数)

图片、mermaid、静态/流式 SVG、位图图表。走 0004 embed block:浏览器光栅化 →
纹理 quad,在 GPU 场景里当 opaque box 参与布局,自动获得 shader/动画/裁剪。

**流式 SVG**:可变纹理的 embed——SVG 每次增长/变化 → 浏览器重新光栅化
(Image/OffscreenCanvas)→ 更新纹理,按节流频率刷新(非每帧)。无需 UI 框架。

### 2.2 原生卡片(自绘原语,只读产出的甜点区)

聊天里多数"卡片"是只读展示:文件 diff 卡、搜索结果卡、工具产出摘要卡。用已有
原语拼:圆角矩形 + pretext 文字 + 图标纹理。**完全融入场景**:与正文一起滚动、
一起被 shader/动画/裁剪处理(0002),字体与正文一致,零同步问题。这是"渲染成
卡片"的主力,不需要 egui 也不需要 DOM。

### 2.3 DOM overlay(真交互卡片,少数)

仅当卡片需真交互(按钮、输入框、内部滚动、live 状态)或本身是 HTML artifact 时:
画布拥有布局、每帧上报该 embed 盒子(位置+尺寸),React 渲染绝对定位 overlay 跟随。
相比 egui 的优势:真文字选择/复制、可访问性、IME、链接、原生输入,Tauri 下一致。

代价:DOM overlay 与 GPU 画布在两个合成层,惯性滚动时可能滞后一帧(同步缝)。
解法见 §3。因主内容是文字、始终纯 GPU,交互卡片偶发,此缝不影响核心体验。

## 3. 像素对齐相机:消除 DOM↔GPU 同步缝

参考:Eemeli Haakana,Codrops 2025-06,
"How to Create Responsive and SEO-friendly WebGL Text"
(https://tympanus.net/codrops/2025/06/05/how-to-create-responsive-and-seo-friendly-webgl-text/,
代码 https://github.com/ehaakana/codrops-text-demo)。

### 3.1 核心技法

把 GPU 相机配置成 **1 世界单位 = 1 CSS 像素**(可见区域精确等于视口像素尺寸)。
于是 DOM overlay 与 GPU 画布**共享同一像素坐标系**:

- 相机/滚动移动时无需逐元素 JS 投影
- overlay 不是"追"画布,而是与画布用同一套像素坐标,被同一滚动量一起平移
- 惯性滚动不再有滞后缝

我们本就是 2D 滚动画布,这个相机设定极自然。它把 0007 §2.3 的同步缝从"已知缺陷"
降为"已解决"。

### 3.2 与 Haakana 方案的关键区别

Haakana 方案:**DOM 是真相、WebGL 是镜像**(为 SEO/可访问性,文字主体仍是 DOM)。
我们 0001 相反:**正文是 GPU 真相、DOM 只做控件**——因为要百万字符流式 +
per-glyph shader,不可能让 DOM 持有全部正文。

故不照搬其"DOM 主、GPU 镜像"模型,只借**像素对齐相机设定**这一条,用于消除我们
DOM overlay 卡片(§2.3)与 GPU 正文之间的同步缝。

### 3.3 与无障碍镜像的关系

Haakana 验证了"DOM 做无障碍真相 + GPU 做视觉"可行,印证 0002 待办里的隐藏 DOM
镜像(供屏幕阅读器 / Cmd+F)。我们的镜像可参考其 box 对齐做法。

## 4. 观望:HTML-in-Canvas API

Chrome origin trial(`getElementTransform` / `drawElementImage`)可把真 DOM 直接
画进 WebGL/WebGPU 纹理并保持可交互、可访问。若成熟,交互卡片层会多第四种实现。
但仍实验阶段,跨浏览器与 Tauri/WKWebView 均不可用,现仅观望,不进当前设计。

参考:https://developer.chrome.com/blog/html-in-canvas-origin-trial

## 5. 管线归属

- 被动 embed(图片/SVG/mermaid/图表):浏览器光栅化 → 纹理(0004),wasm 持元数据
- 只读卡片:自绘原语(rect + pretext + 图标纹理),纯 GPU 场景
- 交互卡片:DOM overlay,像素对齐相机(§3)消除同步缝
- 相机统一为 world unit = CSS pixel,贯穿正文、卡片、overlay、无障碍镜像
