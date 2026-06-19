# Plan 15 进度(代码块视口:行窗 + 边缘淡 + 双向滚动 + 行号 + 复制图标)

- 状态(2026-06-20):**①–⑥ 全相位 core/render/wasm/tsc 可验部分落地 + 测试通过**;淡入淡出 /
  滚动手感 / 复制图标上屏 / 横裁丝滑边须人工 GPU。进度详情即本文。
- 沙箱约束(同 Plan 12/13/14):有 cargo(native + wasm32)+ tsc + wgsl 解析,无 GPU/浏览器 →
  视觉(边缘淡、滚动跟手、图标、进窗补间、横裁部分像素)须人工实跑。

## 已落地(验证)

| 相位 | 落地(file:符号) | 验证 |
|---|---|---|
| **① 行窗 + 软裁 fade + tail** | `codeblock.rs`(新,纯逻辑:window_height/tail/clamp/edge_fade/culled/gutter_width);`app.rs` ensure_layouts 检测 CodeBlock 节点、超 6 行钉死窗高(块后内容上移)、缓存 `CodeView`;build_frame 按 scrollY 偏移、窗外 cull、边缘 fade(取字中心采样);`FrameGlyph.alpha`→`GpuInstance.alpha`→glyph.wgsl 乘静态 alpha | cargo:codeblock 5 测 + `>6 行→≤6 行窗+边缘淡`、`≤6 行→全显无淡`;wgsl 解析 |
| **② 行号 gutter** | `StyleRole::CodeLineNum`(值 43);content.rs 每代码行前置右对齐行号(等宽);glyph.wgsl 弱化灰;layout-bridge mono | cargo:`12 行→12 行号、右对齐 2 位` |
| **③ 复制图标** | `Engine.copy_icon_tex`/`set_copy_icon_tex`;build_frame 每代码块右上角钉 `FrameImage`(不随 scroll);wasm `load_copy_icon`;web `preloadCopyIcon`(栅格 `/copy.svg`)+ main 挂 | cargo:`载图标→FrameImage 在右上;未载→无`;tsc;wasm-pack |
| **④ 双向手动滚动** | `Engine.code_scroll`(key→scrollX/Y/following)+ `scroll_code_block` + `code_hit_rects`/`code_block_at`(`Rect::contains`);build_frame following→tail / 否则 clamp 手动;CodeBlock 字横移、行号 gutter 不动;wasm `code_block_at_screen`/`scroll_code_block`;input.ts wheel 命中代码块→滚块不滚画布(行累加平滑) | cargo:`命中+滚动态`;tsc |
| **⑤ 横向裁剪** | `CodeView.code_x0`(代码内容左界);build_frame 把 CodeBlock 字裁到 `[gutter 右, 盒右]`(CPU 整字粒度) | cargo:`超宽行盒外字横裁不发、右滚后露出` |
| **⑥ gutter 分隔 + 进窗补间 + 卡口** | block_decorations gutter 细竖线(`CODE_GUTTER_LINE`);代码底裁到窗高;进窗(盒高随行数 1→6 长大)走**既有 0016 盒位补间**(Plan 13⑤,无新代码) | 全卡口绿;**GPU 人工**:进窗平滑 |

## 卡口状态(本轮)

- `cargo fmt --all --check` → **绿**。
- `cargo clippy --workspace --all-targets --all-features -D warnings` → **绿(0)**。
- `cargo test`(native)→ **绿**:core 159、render 19 等全过。
- `cargo build --target wasm32`(core + wasm)→ **绿**;`npm run build:wasm`(wasm-pack)→ **绿**。
- `cd web && tsc --noEmit` → **绿**;render wgsl(naga 解析)→ **绿**。
- `wasm-pack test --headless --chrome` / GPU 上屏 → 人工卡口(沙箱无浏览器)。

## 待人工 GPU / 浏览器实跑(代码已就位)

- **行窗 + 边缘淡**:>6 行代码块只露 6 行,上淡出/下淡入(还有更多那侧)。
- **流式 tail**:代码逐行流入 → 自动跟最新 6 行;用户上滚脱离、滚回底复跟随。
- **双向滚动手感**:指针在代码块内 wheel → 滚块不滚画布;横纵都动;命中边界 clamp。
- **复制图标**:`copy.svg` 在每代码块右上角、不随 scroll、缩放清晰(**无交互**,§5 明确不做)。
- **进窗补间**:盒高 1→6 行经 0016 平滑长大。

## 仍属范围 / 后续(plan §5/§6)

- **复制交互 / 选中 / hover**(TODO Q + 剪贴板):图标先在、无行为。
- **逐字语法高亮**(research 另排,正交):代码字现单一 `CodeBlock` 色。
- **横裁丝滑边**:当前 CPU 整字裁(半出字整颗裁);GPU `set_scissor_rect` / shader x-clip 做部分像素裁是后续精修。
- **0018 面板化代码底**:当前圆角 `FrameRect` 底 + gutter 竖线;迁 SDF 面板(AO/描边)= 视觉精修,后续可选。
- **可见滚动条 / maxHeight 展开 / 软换行**:§5 明确不做。
