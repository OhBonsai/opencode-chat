# Plan 14 — 图片嵌入 v1:textured quad + Embed 叶子 + FSM(SVG/PNG 同路栅格)

- 日期:2026-06-19
- 前置:[plan13 §4 Embed 叶子](./plan13-chat-box-layout.md)(Taffy 叶子 + `reportSize` 预留盒)、[0007 富媒体嵌入](../decision/0007-rich-media-embeds.md)、[0011 §3.3 wasm 只持元数据](../decision/0011-gpu-text-as-sdf-primitive.md)、[调研 image-rendering-warp-jcode](../research/image-rendering-warp-jcode.md)(warp 范式 = textured quad)
- 一句话:图片走**最简单一条路**——浏览器解码/栅格成位图 → GPU **textured quad**;wasm 只持元数据(尺寸/位置);图片在 Taffy 里是一个 **`Embed` 叶子**(`reportSize` 预留盒防 reflow);生命周期一个 **FSM**(Placeholder→Loading→Ready→Failed)。**PNG 与 SVG v1 同路**(SVG 也交浏览器栅格,不做矢量)。

> 范围决策:作者明确「先做简单的」。**矢量原生 SVG / streaming-svg / 逐 path morph 全部 defer**——见调研 §5「路 B」,另排(承 0013 数学范式)。本 plan 只做 warp 那条已验证的"栅格成纹理"路。

---

## 0. 范围(明确边界)

- **v1 做**:静态位图(PNG/JPEG/WebP)+ 静态 SVG,**统一浏览器栅格 → 纹理 quad**;Embed 叶子 + FSM + 占位防 reflow。
- **v1 不做**(defer):矢量原生 SVG(路 B)、streaming-svg、**canvas 内逐帧动图合成**、纹理 atlas、缩放重栅(放大暂用纹理采样,可糊)、远程 URL 鉴权/CORS 细节。
- **动图(GIF / 动 WebP / APNG / 动画 SVG)做**:检测分流 → v1 首帧静态、② DOM overlay 让浏览器自己播(§2.5)。
- SVG v1(静态)= 当位图:浏览器 `Image`/`<img>` 原生栅格 SVG → `ImageBitmap`,与 PNG 之后无异。**缩放锐利/流式留给路 B**。

## 1. 数据流

```
markdown ![alt](url) → content.rs 产 Embed 节点(0020 NodeKind::Embed 已有)携 {url, alt, 估尺寸?}
   → Taffy:Embed = 叶子,measure = reportSize(已知宽高比/估高 → 预留盒,plan13 §4)
   → FSM(core 持状态 + JS 解码):Placeholder → Loading → Ready(纹理就绪)→ Failed
   → 静态:build_frame 发 FrameImage{pos,size,tex_id,alpha} → image pipeline textured quad
   → 动图(animated):v1 发首帧 FrameImage;② 改发 FrameEmbed{key,world_rect} → web DOM overlay 浏览器自播(§2.5)
```

wasm 只持 `{url, tex_id, w, h, state}`;**解码/栅格/纹理上传全在 JS**(0011 §3.3 / 0007:重活交浏览器)。

## 2. 三件套

### 2.1 Embed 叶子(plan13 Taffy)
- `NodeKind::Embed`(nodes.rs 已留位)→ Taffy 叶子。
- `measure`:已知宽高(markdown `=WxH` 或解码后回报)→ 该尺寸;未知 → **估高占位盒**(默认宽高比 / 一行高的占位条),避免真身到达跳变(thinking §1D)。
- 解码拿到真实尺寸 → `reportSize` 回报 → Taffy 重排该叶子 → 0016 平滑(plan13 §2.3 改尺寸走 measure)。

### 2.2 FSM(0007 embed FSM)
```
Placeholder  内容确认是图(`)` 到达)→ 占位框(估高)
  → Loading  发起浏览器解码(fetch/decode)
  → Ready    ImageBitmap 上传 GPU → tex_id 就绪 → FrameImage 出图(alpha 淡入,0025)
  → Failed   解码/网络失败 → 占位 + alt 文本兜底([image: alt],同 vendor 现状)
```
- core 持 `EmbedState`;JS 解码完成回调 `image_ready(node_key, tex_id, w, h)` / `image_failed(node_key)`。
- **不阻塞**:Loading 期渲占位,主循环照跑(异步)。

### 2.3 textured quad 管线(新,照 warp `renderer/image.rs`)
- `frame.rs` 加 `FrameImage{ pos:[f32;2], size:[f32;2], tex_id:u32, alpha:f32, radius:f32 }`;`FrameData.images: Vec<FrameImage>`。
- 新 image pipeline(`crates/render`):textured quad(复用 quad mesh + globals bind),**per-image 纹理 + sampler bind group**(非 atlas,warp 同款),`image.wgsl`:采 RGBA 纹理 × alpha,可选圆角(复用 `sd_round_box` clip)。instance 化(同 glyph/widget 范式)。
- 绘制次序:rect/panel 后、glyph 前(图作底,文字压上)或按盒次序——v1 简单:images 单独一 pass。

### 2.4 SVG = PNG 同路(v1 静态)
- content 不区分:`url` 后缀/`data:` MIME 判 SVG → 仍交 JS `Image` 解码(浏览器原生栅格 SVG)→ `ImageBitmap` → 纹理。**core/render 不知道矢量这回事**。
- 留尾:路 B(矢量原生)接入时,SVG 在 content 层分流到另一管线,**本 plan 的纹理路对 PNG 保持不变**(0→1 不为兼容妥协,但此处天然正交)。

### 2.5 动图分流(GIF / 动 WebP / APNG / 动画 SVG)— 实现

**判定(JS 解码时回报 `animated: bool`)**:
- GIF/WebP/APNG:WebCodecs `ImageDecoder` → `tracks[0].animated` 或 `frameCount > 1`。
- SVG:文本嗅探 `<animate`/`<animateTransform`/`<animateMotion`/`<script` 或 style 含 `animation`/`@keyframes` → 视为动画。
- 注:**动画 SVG ≠ streaming-svg**——后者是源码随 LLM 流式到达(路 B);本节只管"会自己动的图文件"。

**核心张力**:动图**永不 settle**(每帧变)→ **不能**塞 canvas 纹理路逐帧重传(毁块冻结 0025 §4 + 每帧 GPU 上传)。故**动的交浏览器自己播**。

**两段式实现**:
- **v1(本 plan,立即)= 首帧静态**:`animated=true` 也先**只取第一帧**当静态纹理走 §2.3(`ImageDecoder` decode 帧 0 / SVG 栅一帧)。看到静止首帧、不 jank、零新管线。FSM 进 `Ready{ animated:true }`,先 `dom:false`(画首帧)。
- **②(DOM overlay,使其真动)**:core 对 `animated` 的 Embed **不发 `FrameImage`**,改发 `FrameEmbed{ key, world_rect }`(每帧报世界矩形);web 维护一层 DOM(`<img>` GIF / `<svg>` 文本),按相机 `world_to_screen` + scale 定位叠在 canvas 上 → **浏览器原生播 GIF 循环 / SVG SMIL·CSS 动画**,零纹理上传、canvas 那块仍冻结。= [0022 DOM overlay] 的**最小切片**(只为动图,不上 0022 全套)。

**为什么不 canvas 内逐帧**:per-frame 纹理上传 + 该 embed 区永不冻结(一小块"永远活跃岛")。仅当"动图必须压在 SDF 特效下 / 同台 morph"才考虑,**默认否**(defer,§6)。

## 3. 改动清单(file:符号)

| 处 | 改动 |
|---|---|
| `crates/core/src/content.rs` | markdown Image → `Embed` 节点(携 url/alt/估尺寸),不再只拍平成文本(兜底文本留 Failed 态) |
| `crates/core/src/nodes.rs` | `NodeKind::Embed` 已有;Embed 叶子 measure 接 reportSize |
| `crates/core/src/app.rs` | `EmbedState` FSM + build_frame:Ready 发 `FrameImage`、否则占位;Embed 叶子尺寸进 Taffy |
| `crates/core/src/frame.rs` | 加 `FrameImage` + `FrameData.images`;**`FrameEmbed{key,world_rect}` + `FrameData.embeds`**(动图 DOM overlay) |
| `crates/render/*` | 新 image pipeline + `image.wgsl`(textured quad,per-image 纹理 bind) |
| `crates/wasm/src/lib.rs` | `load_image(url)->id` / `image_ready(key,bitmap,w,h,**animated**)` / `image_failed(key)`(仿 `load_msdf`);`frame.embeds` 暴露给 JS |
| `web/src/*`(新 `image-loader.ts`) | `fetch`+`ImageDecoder`/`createImageBitmap`(SVG 也吃)→ **判 animated** → 上传首帧纹理 → 回调 wasm;失败回 Failed |
| `web/src/*`(新 `embed-overlay.ts`) | **动图 DOM 层**:读 `frame.embeds` → 按相机 `world_to_screen`+scale 定位 `<img>`/`<svg>`(0022 最小切片) |

## 4. 相位

| 相位 | 交付 | 验证 |
|---|---|---|
| **① Embed 节点 + FSM(core)** | content Image→Embed;`EmbedState`;占位盒进 Taffy | cargo:`![](url)`→Embed 节点 + Placeholder 态 + 预留盒;失败→alt 兜底 |
| **② textured quad 管线(render)** | `FrameImage` + image pipeline + `image.wgsl` | 沙箱:wgsl 解析过;**GPU 人工**:一张测试纹理上屏 |
| **③ JS 解码上传** | `image-loader.ts` + wasm `load_image`/`image_ready` | tsc;**人工**:真 PNG/SVG URL → 上屏、尺寸对、占位→Ready 不跳 |
| **④ FSM 接 0016/0025** | reportSize→Taffy 重排→0016 平滑;Ready alpha 淡入(0025) | **GPU 人工**:解码完成 reflow 平滑、淡入 |
| **⑤ 动图检测 + 首帧静态** | JS `animated` 判定(ImageDecoder/SVG 嗅探)+ decode 帧 0;core FSM `Ready{animated}`(§2.5) | cargo:`animated` 标志贯穿;**人工**:GIF/动 SVG 显**首帧静止**、不崩 |
| **⑥ DOM overlay 播动画** | core 发 `FrameEmbed{world_rect}`;`embed-overlay.ts` DOM 层 + 相机同步(0022 最小切片) | tsc;**人工**:GIF 循环 / 动画 SVG SMIL 上屏、随 pan/zoom 跟手、canvas 仍冻结 |

> 沙箱可验:content/FSM/Taffy 尺寸 + `animated` 标志(cargo)+ wgsl 解析 + tsc。**纹理上屏、占位→Ready、reflow、动图 DOM 同步须人工 GPU + 真 URL。**

## 5. 测试用例提纲

- [ ] 正常:`![alt](png-url)` → Embed + Placeholder→Loading→Ready;尺寸=解码真值。
- [ ] 正常:SVG url / `data:image/svg+xml` → 同路 Ready(浏览器栅格)。
- [ ] 边界:无尺寸信息 → 估高占位,解码后 reportSize 重排不跳变(0016)。
- [ ] 错误:404 / 解码失败 / 非图 MIME → Failed → `[image: alt]` 文本兜底,不崩。
- [ ] 边界:图在流式中(`)` 未到)→ 不闪 raw `![..](..`(thinking §1A,hold 到确认)。
- [ ] 动图:多帧 GIF / 动 WebP → `animated=true`,v1 显首帧静止;动画 SVG(`<animate>`)→ 判定 animated。静态单帧 GIF → `animated=false` 走静态路。
- [ ] 动图②:DOM overlay 的 `<img>`/`<svg>` 随 pan/zoom 跟随;Failed/卸载时 DOM 元素清理不泄漏。

## 6. Scope · 不做什么

- ❌ 矢量原生 SVG / streaming-svg / 逐 path morph(调研路 B,另排;承 0013)。
- ❌ 缩放重栅(放大用纹理采样,可糊;路 B 才锐利)。
- ❌ **canvas 内逐帧动图合成**(per-frame 纹理上传)——动图走 DOM overlay(§2.5),不进 canvas 纹理逐帧。
- ❌ atlas / 远程鉴权 / 大图 LRU 淘汰(后续)。
- ❌ 点击放大 / lightbox / 通用交互(0022 全套或后续;本 plan 的 DOM overlay 只为动图显示)。

## 7. Risk / Open

- **纹理生命周期**:per-image 纹理无 atlas → 多图显存;v1 不淘汰,记基准,超阈值再加 LRU(warp 有先例)。
- **CORS / data-url**:远程图受跨域限制;v1 先支持同源 + `data:`,跨域失败走 Failed。
- **占位估高**:无 `=WxH` 时估高不准 → 解码后 reportSize 修正 + 0016 吸收(可接受)。
- **动图判定启发式**:`ImageDecoder` 不普及的浏览器回退(GIF 头嗅探 / 多 `IDAT`-APNG `acTL`);SVG animated 嗅探的误判(有 `<style>` 但无动画)——v1 容忍"误判成动图 → 走 DOM overlay"(显示无碍,只是没进 canvas 纹理)。
- **DOM overlay 依赖**:②阶段是 [0022] 的最小切片;若 0022 先落,则复用其 `world_to_screen`/主题/事件钩子,本 plan 只接动图 embed。
- **Open**:① content 层判 SVG/动图的依据(后缀 / MIME / 嗅探)?② 路 B 接入时 SVG 在 content 的分流点(本 plan 留尾,不预设)。③ 动图 embed 滚出视口时 DOM 元素回收策略(暂:不可见即 `display:none`,不销毁)。

## 8. Done

`![](url)` 产 Embed 叶子(Taffy 预留盒);PNG/SVG 经浏览器解码 → textured quad 上屏;FSM Placeholder→Loading→Ready→Failed(失败 alt 兜底);解码后 reportSize→0016 reflow 不跳、Ready alpha 淡入;**动图(GIF/动画 SVG)检测分流——v1 显首帧静态、② DOM overlay 让浏览器自播且 canvas 仍冻结**;卡口(cargo/clippy native+wasm、wasm-pack、tsc、wgsl 解析)全绿。

## 9. 关联

- decision:[0007](../decision/0007-rich-media-embeds.md)(embed FSM/纹理)/ [0011 §3.3](../decision/0011-gpu-text-as-sdf-primitive.md)(wasm 持元数据)/ [0016](../decision/0016-streaming-morph-render-model.md)(reflow)/ [0025](../decision/0025-sdf-node-animation-system.md)(淡入)/ [0022](../decision/0022-dom-overlay-layer.md)(交互/lightbox 备选);research [image-rendering-warp-jcode](../research/image-rendering-warp-jcode.md)(路 A 本 plan / 路 B 另排)。
- Code 入口:`content.rs`(Image→Embed)·`nodes.rs`(`NodeKind::Embed`)·`app.rs`(EmbedState + build_frame)·`frame.rs`(`FrameImage`/`FrameEmbed`)·`crates/render`(image pipeline + `image.wgsl`)·`crates/wasm/src/lib.rs`(load_image/animated,仿 `load_msdf`)·`web/src/image-loader.ts`(新,含 animated 判定)·`web/src/embed-overlay.ts`(新,动图 DOM 层)。
- DOM overlay:[0022](../decision/0022-dom-overlay-layer.md)(②阶段最小切片,动图自播 + 相机同步)。
