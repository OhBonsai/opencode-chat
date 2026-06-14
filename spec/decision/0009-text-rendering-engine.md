# 决策记录 0009:文字渲染引擎 —— 浏览器系统字体桥 vs glyphon/cosmic-text

- 日期:2026-06-14
- 状态:Plan 2 期已采纳;**Plan 3 起被 [0011](./0011-gpu-text-as-sdf-primitive.md) 演进**——BR5 放宽为"打包自带字体",文字改走自有 SDF 图元(TinySDF),glyphon 升级路径作废
- 前置:0001(画布架构,跨界/排版边界、BR5 零字体打包)、0004(markdown 管线)
- 来源:Plan 2 H 接入 jcode 后,排查 jcode **实际渲染**实现(`jcode-desktop`),发现其文字
  渲染走 glyphon/cosmic-text,与我们当前的"浏览器系统字体 + JS 桥"是两条不同路线

## 1. 背景

Plan 2 H 已 vendor `jcode-render-core`(后端中立 markdown 文档模型,见 [plan2 §5.5](../plan/plan2-usable-chat.md))。
但它只是**模型**(`parse_markdown -> Document`),不含实际像素渲染。继续排查 jcode 仓库,
真正画字的是 **`jcode-desktop`** crate(原生桌面 app:`winit` + `arboard`,**不跑 wasm**):

| 关切 | jcode-desktop | 本项目当前(Plan 1/2) |
|---|---|---|
| 文字渲染 | **glyphon 0.5**(cosmic-text 整形/排版/折行 + etagere atlas + wgpu `TextRenderer`) | 手搓单页 atlas(`render/atlas.rs`)+ 逐 grapheme 拼 quad(`render/scene.rs`) |
| 排版/折行 | cosmic-text 真整形 + `Wrap::Word` 自动折行 | JS `web/pretext-bridge.ts`(measureText 单字宽,无 shaping/折行) |
| 光栅化 | cosmic-text + swash → atlas | JS `web/glyph-raster.ts`(OffscreenCanvas `fillText`) |
| 字体来源 | `fontdb` 系统字体(原生机器有) | **浏览器系统字体**(Canvas2D),零打包(BR5) |
| 流式淡入 | CPU 侧改尾部 N 字颜色 alpha(`apply_streaming_tail_fade`,逐帧重算) | GPU 着色器 `time - spawn_time`(`glyph.wgsl`),CPU 零参与 |
| 平台 | 原生 | wasm/WebGPU |

关键文件(jcode 侧):`jcode-desktop/src/single_session_render.rs`(组 `glyphon::TextArea`)、
`.../single_session_render/text_style.rs`(`Document`/`StyledLine` → glyphon `Buffer`+`Attrs`,
StyleRole→颜色,opacity + tail-fade)。

## 2. 矛盾点:glyphon 在 wasm 需要打包字体

glyphon 的优势是**真排版**:正确的粗/斜/等宽字宽、CJK/emoji 字体回退、连字、bidi、自动
折行 —— 正好补上我们 measureText + 逐 grapheme 的全部短板(粗体按正文宽度量、无折行、
无 shaping)。

但 cosmic-text 靠 `fontdb` 加载字体:
- **原生**(jcode-desktop):能读系统字体,零成本。
- **wasm**:浏览器不暴露系统字体给 wasm/`fontdb`,**必须打包 `.ttf/.otf` 一起发**。要覆盖
  中文得带 CJK 字体(数 MB~十几 MB)。这直接违背 0001 的 **BR5「零字体打包、用浏览器系统
  字体」** —— 当初选 JS Canvas 桥就是为了白嫖浏览器系统字体。

附带:jcode 用 wgpu 0.19,本项目 wgpu 25;采用需换 glyphon 配套版本(≥0.6/0.7 线)。

## 3. 选项

### A. 切到 glyphon/cosmic-text(大改)
用 glyphon 替换 `render` 的 atlas + scene **和** web 的 `pretext-bridge`/`glyph-raster` 两个
JS 桥;打包 1~2 个字体(含 CJK 子集);移到配套 wgpu 版本;淡入从 GPU `spawn_time` 改 CPU
tail-fade(或保留 GPU 自绘 quad 思路但喂 cosmic-text 字形)。
- ➕ 文字质量最大化(shaping/折行/字宽/字体回退全对)
- ➖ 放弃 BR5(打包字体,包体涨数 MB)、改动面最大、丢掉 GPU 零参与淡入、wgpu/glyphon 版本绑定

### B. 保留浏览器系统字体 JS 桥,只增量搬 jcode 的适配逻辑(采纳)
继续用 Canvas2D 系统字体(零打包,守 BR5);把 jcode-desktop `text_style.rs` 里值得借鉴的
**适配逻辑**按需移植到我们现有管线:StyleRole→字体/颜色映射、**流式尾部淡入的分段思路**、
markdown 块的间距策略。不引入 cosmic-text/glyphon。
- ➕ 改动小、守 BR5、保留 GPU `spawn_time` 淡入、不绑 wgpu/glyphon 版本
- ➖ 仍无真 shaping/折行;粗/斜/等宽字宽是近似(measureText 按 body 字体),复杂脚本/bidi 不支持

### C. 只出 ADR,不动代码
即本文。

## 4. 决策

**采纳 B**:Plan 2/3 阶段**保留浏览器系统字体 + JS 桥**,不引入 glyphon/cosmic-text。理由:

1. **BR5 是 0001 的明确取舍**:零字体打包、用系统字体是库交付为"框架无关 wasm 组件"的卖点
   之一;为它打包数 MB CJK 字体在当前阶段不划算。
2. **现管线已可用**:Plan 2 已能渲染流式 markdown(标题/粗斜/代码/表格/列表),粗/斜靠
   光栅化时换字体(视觉正确,仅宽度近似),够"可用"。
3. **改动面与收益不匹配**:glyphon 是大改且绑版本;当前痛点(表格、收尾、滚动)已由 Plan 2
   F–J 解决,shaping/折行尚非阻塞。
4. jcode-desktop 是**原生** app,无法直接复用;能复用的只是**适配思路**,而非整套渲染。

可增量借鉴(B 的范畴,非本 ADR 强制):jcode 的 StyleRole→颜色/字体映射表、尾部淡入分段、
块间距策略 —— 这些不依赖 glyphon,可按需移植。

## 5. 后果与重新评估触发条件

**保留的局限**(已知,接受):
- 无真整形:复杂脚本(阿拉伯/印度系)、bidi、连字不支持;
- 折行靠我们自己(measureText 逐字),不如 cosmic-text 的 `Wrap::Word`;
- 粗/斜/等宽**字宽是近似**(layout 用 body 字体度量,光栅化换字体),宽字体可能轻微错位。

**glyphon 作为未来升级路径备案**——出现下列任一情况时重开此决策升级到 A:
- 需要正确支持复杂脚本 / bidi / 高质量折行(产品要做多语言);
- 可接受打包字体(例如已在打包其他资源,或做"自带字体"的发行档);
- 我们的手搓 atlas 在大字符集(CJK 全量)下内存/性能撑不住,需要 cosmic-text 的成熟字形缓存。
  届时:引 `glyphon`(配套 wgpu 25 的版本)+ 打包一个含 CJK 的可变字体;替换 `render` 的
  atlas/scene 与两个 JS 桥;淡入改 CPU tail-fade 或继续 GPU(给 cosmic-text 字形附 spawn_time)。

**不变量保持**:无论 A/B,content→layout→render 的接口契约(`StyledSpan`/角色、平铺位置、
`FrameGlyph`)不动 —— 0001 §2.2 "排版模块可替换"的设计就是为此留的缝,换 glyphon 只动 layout
桥 + render 后端,不动 core。

## 6. 来源 / 链接

- jcode 实际渲染:`~/w/agentscode/jcode/crates/jcode-desktop`(glyphon/cosmic-text/winit)
- 已 vendor 的模型:[`vendor/jcode-render-core`](../../vendor/README.md)
- 受影响接口缝:0001 §2.2(layout 可替换)、§3.4(跨界);BR5(零字体打包)
- 现管线:`crates/render/{atlas,scene}.rs`、`web/src/{pretext-bridge,glyph-raster}.ts`
