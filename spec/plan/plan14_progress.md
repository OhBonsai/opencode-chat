# Plan 14 进度(图片嵌入 v1:textured quad + Embed 叶子 + FSM)

- 状态(2026-06-19):**①–⑥ 全相位 core/render/wasm/tsc 可验部分落地 + 测试通过**;纹理/动图**上屏**
  须人工 GPU + 浏览器 + 真 URL(plan §7 已预告)。整条管线(content→Embed→FSM→纹理 quad / DOM overlay)
  编译到 wasm32 + tsc 全绿。
- 沙箱约束(同 Plan 12/13):有 cargo(native + wasm32)+ tsc,无 GPU/浏览器 → 纹理上屏、占位→Ready
  reflow、动图 DOM 同步、真 URL 解码须人工实跑。本轮把所有纯 core/render(wgsl 解析)/tsc 可验的
  部分做完并测试。

## 已落地(验证)

| 相位 | 落地(file:符号) | 验证 |
|---|---|---|
| **① Embed 节点 + FSM** | vendor jcode `StyleRole::Image`(打包 `url\u{1f}alt`);`content.rs` `StyleRole::Image`(值 42)+ `EmbedRegion` + `parse_markdown_embeds` + emit alt 占位(url 不漏);`nodes.rs` 图片 span → `NodeKind::Embed` 叶;`embed.rs` `EmbedState`(Placeholder→Loading→Ready→Failed)+ `Embed` | cargo:image→alt 占位+url 隐藏、Embed 节点非 Run、无 alt 默认、FSM happy/terminal/no-regress、综合覆盖更新 |
| **② 纹理 quad 管线** | `frame.rs` `FrameImage`/`FrameEmbed` + `FrameData.images/.embeds`;`image.wgsl`(纹理 quad × alpha + 圆角 SDF);`render/scene.rs` `ImageInstance`;backend image pipeline(group0 globals/group1 per-image 纹理)+ `upload_image`(RGBA→sRGB 纹理→tex_id)+ 逐图绘 | cargo:`image_shader_is_valid_wgsl`(naga);wasm32 编译;**GPU 人工**:测试纹理上屏 |
| **③ 解码上传** | core `Engine.image_registry`(FSM 注册表)+ `take_pending_images`/`image_ready`/`image_failed`;build_frame:Ready 隐藏 alt 出 `FrameImage`/`FrameEmbed`;wasm `take_pending_images`/`upload_image_rgba`/`image_failed`;web `image-loader.ts`(fetch→`createImageBitmap` 首帧→RGBA→animated 判定→上传/失败)+ main 轮询 | cargo:ready→FrameImage+隐藏 alt、failed→alt 兜底;tsc;wasm32 编译(含 wgpu 管线);**人工**:真 URL 上屏 |
| **④ 0025 淡入** | `Embed.ready_at` + `Embed::alpha(now,fade)`;build_frame 喂 `FrameImage.alpha`(`IMAGE_FADE_MS`=200) | cargo:`alpha_fades_in_after_ready`(0→0.5→1,重入不重启);**GPU 人工**:淡入视觉 |
| **⑤ 动图检测 + 首帧静态** | `image-loader.ts` animated 判定(`ImageDecoder` 帧数 for GIF/WebP/APNG;SVG 文本嗅探 `<animate>`/SMIL/CSS);`createImageBitmap` 取首帧;`animated` 标志贯穿 JS→`Embed.animated`→build_frame 分流 | cargo:`animated_image_emits_frameembed_not_frameimage`;**人工**:GIF/动 SVG 显首帧静止 |
| **⑥ DOM overlay 播动画** | `FrameEmbed{key,url,rect}`;`GpuSink.last_embeds` + `ChatCanvas.frame_embeds()`(world→screen 经相机);`embed-overlay.ts`(按 key 复用 `<img>`、相机定位、回收);main rAF 轮询 | tsc;wasm32 编译;**人工**:GIF 循环 / SVG 动画上屏、随 pan/zoom 跟手、canvas 仍冻结 |

## 卡口状态(本轮)

- `cargo fmt --all --check` → **绿**(含 vendored jcode)。
- `cargo clippy --workspace --all-targets --all-features -D warnings` → **绿(0)**。
- `cargo test`(native)→ **绿**:core 148、render 19、jcode 19 等全过。
- `cargo build --target wasm32-unknown-unknown`(core + wasm)→ **绿**;`npm run build:wasm`(wasm-pack)→ **绿**。
- `cd web && tsc --noEmit` → **绿**。
- `wasm-pack test --headless --chrome` / GPU 上屏 → 人工卡口(沙箱无浏览器)。

## 待人工 GPU / 浏览器 / 真 URL 实跑(代码已就位)

- **纹理上屏**:`![](png/svg url)` → 浏览器解码 → textured quad,尺寸=解码自然尺寸,Ready alpha 淡入。
- **占位→Ready**:未就绪显 alt 文本,解码后纹理替之(同源/`data:` URL;跨域失败走 Failed 显 alt)。
- **动图**:GIF / 动 WebP / 动画 SVG → v1 首帧静止;DOM overlay `<img>` 循环播放、随 pan/zoom 跟手、canvas 那块冻结。

## 仍属 Plan 14 范围 / 关联 plan,单独排期

- **reportSize→Taffy reflow**(plan §2.1/④):图片解码后按自然尺寸**预留盒**、避免顶下文字重叠 —— 依赖
  **Plan 13 Tier C**(Embed 作带 measure 的 Taffy 叶子,见 [plan13_progress.md])。Tier C 落地前,图按自然
  尺寸绘于 alt 占位盒位,可能压邻行。**两者合并实现**(measure 同时驱动块内嵌套 + embed 预留)。
- **矢量原生 SVG / streaming-svg / 逐 path morph**(plan §6 / 调研路 B):另排,承 0013。
- **纹理 LRU 淘汰 / atlas / 跨域鉴权**(plan §7):v1 不淘汰(per-image 纹理),记基准超阈值再加。
