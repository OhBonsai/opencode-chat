# TODO —— Plan 3 待办积压(K/L 之后)

- 维护说明:Plan 3 的 **K、L 已细化** 在 [spec/plan/plan3-canvas.md](spec/plan/plan3-canvas.md);本文件收**其余相位(M–S)+ Plan 2 欠账**,后续逐步细化/上提到正式 plan。
- 编号续 K、L → **M–S**;一个条目大致 ≈ 一个 Phase/PR。完成后从这里划走、补进 plan 文档。

---

## 相位积压(依赖:M–N 需 K/L 完成;O–S 需 L 的图元管线)

### M — 块装饰 quad + 表格对齐 + 词折行(追平 markdown 观感)
- [ ] 矩形/圆角图元:代码块底、行内码 chip、引用左条、表格网格+斑马+表头、hr、H1/H2 细线、GitHub Alert 左色条
- [ ] **拆 H1–H6 分级**(`content.rs` 现合并为一类 Heading)→ 逐级字号
- [ ] 表格**两趟列宽对齐**(max-content,padding 6px/13px,边框)
- [ ] **词边界折行 + CJK 禁则**(替换 `pretext-bridge` 逐 grapheme 贪婪折行)
- [ ] 固化 **github-theme 设计令牌**(字号/间距/色,见会话实测值)→ 喂 `StyleRole` 映射
- 参考:[0004](spec/decision/0004-markdown-and-embeds.md)、GitHub 令牌(`github-markdown-css`)
- 取舍:**自带字体 vs GitHub 字形一致 互斥**(0009/0011)——只追结构/间距/配色

### N — 富 shader 特效(依赖 SDF)
- [ ] SDF 发光 / 描边 / 溶解(fragment)
- [ ] 逐字 compute/vertex 动效(0011 §3.2);无状态时间效果走纯 VS(兼容冻结),有状态物理才 compute(先裁剪)
- 参考:[0007](spec/decision/0007-rich-media-embeds.md)、[0011 §3.2](spec/decision/0011-gpu-text-as-sdf-primitive.md)

### O — 嵌入块(图片 → mermaid → 卡片)
- [ ] 图片:浏览器解码 → 纹理 quad;mermaid:SVG → 浏览器光栅 → 纹理
- [ ] embed FSM:Placeholder → Loading → Ready → Failed;占位高度防 reflow;像素对齐
- [ ] wasm 只持元数据(尺寸/位置),重活交浏览器
- 参考:[0004 §7](spec/decision/0004-markdown-and-embeds.md)、[0007](spec/decision/0007-rich-media-embeds.md)

### P — 标签层 + 自定义语法
- [ ] pre-markdown segmenter + 标签注册表(hold 区、未知标签默认 Literal)
- [ ] `:::` 容器开启符(0006 §5.1);`<thinking>`/citation 区域 FSM
- [ ] 行内 chip:`@提及` / 引用角标(parse 后 span 后处理,0006 §5.2);`[^1]` 用 pulldown `ENABLE_FOOTNOTES`
- [ ] 安全:标签当数据,绝不当 HTML 执行
- 参考:[0006](spec/decision/0006-inline-tags-and-extensibility.md)、[0010 §5.1](spec/decision/0010-markdown-parsing-strategy.md)

### Q — input / 选区 / hit-test / 可点链接
- [ ] **CPU 基础盒模型**(§3.3④)做命中/选区/复制——不回读 GPU、不用正在动的 SDF
- [ ] 可点超链接(借 warp `hyperlink + Action`,0010 §5);脚注/引用跳转
- [ ] 选区跨折行、复制保真
- 参考:[0011 §3.3④](spec/decision/0011-gpu-text-as-sdf-primitive.md)、[0010](spec/decision/0010-markdown-parsing-strategy.md)

### R — 无障碍 DOM 镜像 + 渲染降级
- [ ] 可见内容 **DOM 镜像**(屏幕阅读器)——**可嵌入组件的硬需求,别拖到最后**;兼作"无 WebGPU 也无 WebGL2"的极端兜底
- [ ] **WebGL2 路专测**(已通过 `Backends::GL` 启用、自动兜底,但未测);处理其限制:**无 compute → 逐字 compute 特效降级为 vertex+fragment**(见 0011 §3.4)
- [ ] **Canvas2D 不做**(SDF/shader/compete 在 Canvas2D 上无意义;`RenderBackend` trait 留缝但不实现)
- 参考:[0003 §5](spec/decision/0003-fault-tolerance.md)、[0011 §3.4](spec/decision/0011-gpu-text-as-sdf-primitive.md)、`api` 模块(无障碍镜像)

### S — 公共 API + React/Vue 封装 + npm 打包
- [ ] 命令式 API / props / 事件 / 主题(`api` 模块)
- [ ] React、Vue 薄封装;`npm i` 即用
- [ ] **产物体积守门**(守"轻包体"原则)
- 参考:[0000](spec/decision/0000-overview.md)、README「交付形态」

---

## Plan 2 欠账(可插空,喂上面相位)

- [ ] **真 pretext per-role 精确度量**(Plan2.5):粗/斜/code 排版宽度(measureText 现按 body 度量)→ 随 M 的 SDF 度量一并解决
- [ ] **语法高亮**(H5):tree-sitter / syntect-fancy-regex → 接 M 的颜色管线(GitHub `.pl-*` 调色板可直接抄)
- [ ] **Turn 完整分组投影(AR11)+ 折叠 tool/reasoning**(I5)
- [ ] **10k 行真机 fps/内存 benchmark**(§3 一直挂着的待验项)
- [ ] 可视滚动条 + 块内 glyph 级裁剪细化(G 推迟项)
- [ ] 显式心跳 backoff 强制重连(J2,当前用周期 resync + 自动重连覆盖)

---

## 已决策、勿重开(背景锚点)

- 解析沿用 **pulldown-cmark**,不手写 nom、不上 comrak;自定义语法走标签层不动解析器([0010](spec/decision/0010-markdown-parsing-strategy.md))
- 文字 = **SDF 图元**,自有引擎、借算法(TinySDF)不借框架(否决 AntV G / egui / cosmic-text / glyphon)([0011](spec/decision/0011-gpu-text-as-sdf-primitive.md))
- **字体打包自带**(放宽 BR5),浏览器固定主战场([0009](spec/decision/0009-text-rendering-engine.md)→0011)
