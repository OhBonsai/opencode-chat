# Plan 11 — 模块化 shader + markdown 组件 + 推进 markdown

- 日期:2026-06-18
- 状态:相位 ①②③ 代码落地(沙箱 wgsl 解析通过 + tsc 通过;**待本地 `cargo test` + `wasm-pack build` + GPU 实跑确认**);④ 文字层已验证(脚注/定义列表既已走文字层),latex/image/html/link 拆步登记待排。
- 前置:[0026](../decision/0026-modular-shader-organization.md)(决策)、调研 [`shader-code-organization-survey`](../research/shader-code-organization-survey.md);[0011](../decision/0011-gpu-text-as-sdf-primitive.md)/[0018](../decision/0018-sdf-panel-decoration-primitive.md)/[0020](../decision/0020-content-node-identity-model.md)/[0025](../decision/0025-sdf-node-animation-system.md)/[Plan 10](plan10-sdf-animation-system.md)
- 取向:SDF-only;复用底层 SDF 函数、不复用上层图元;模块化用 `include_str!` 拼接(不引预处理器)。
- **验证约束**:相位 1–2 改了 build 期拼接 + 新 widget pipeline(新 vertex 布局)→ **必须本地 `cargo test` + `wasm-pack build` + GPU 实跑**。沙箱无 cargo,只能 `parse_str` 级校验 + 逐行审。

---

## 1. 总览(四相)

```
① 模块化地基   抽 base/sdf.wgsl,rect/panel 去重,include_str! 拼接,校验改拼接结果。  纯重构,零行为变化。
② widget 图元   新 markdown/widget.wgsl pipeline + box.wgsl(复选框);FrameWidget 扁平数组贯穿 seam→GPU。
③ 任务复选框    content→layout→render 接通:StyleRole::Task* + 组件实例发射 + ✓ 渲染 + 动画 profile。
④ 推进 markdown 文字层(脚注/定义列表)+ HR 效果 + latex/image/html/link(4 步,后续)。
```

相位 1 是纯重构(可单独验证不回归),先落;2–3 是复选框主线;4 是后续 markdown 覆盖。

## 2. 相位 ① 模块化地基(纯重构)

**做什么**:把重复的形状函数抽进 `base/sdf.wgsl`,各 pipeline 源在 build 期前置拼接。

- 新建 `shaders/base/sdf.wgsl`:`sd_round_box`(从 rect/panel 抽出,**唯一一份**),预留 `sd_circle`/`sd_seg`/`op_outline`/`smin` 占位(用到再加)。仅 `fn`,无 entry/binding。
- `shaders/` 重排进 `base/`:`glyph.wgsl` / `rect.wgsl` / `panel.wgsl` 移入 `base/`,删掉各自的 `sd_round_box`。
- `backend.rs`:每条 pipeline 源改为 `format!("{}\n{}", SDF, include_str!(...))`(`SDF = include_str!("shaders/base/sdf.wgsl")`)。
- `*_shader_is_valid_wgsl` 测试:改为 `parse_str` **拼接后**字符串(保证拼接产物合法)。
- **验收**:`cargo test` 全绿;`wasm-pack build` 通;肉眼任意 case 与改前**像素级一致**(零行为变化)。

**风险**:`include!` 路径、拼接换行;符号重复(确保移走后无残留 `sd_round_box`)。低,可单独验证。

## 3. 相位 ② widget 图元(新 pipeline)

**做什么**:一条 markdown 组件 pipeline + 第一个组件(box),数据通路打通到 GPU。

- `shaders/markdown/box.wgsl`:`fn md_box(local, half, p) -> f32` —— 圆角方框轮廓(SDF round box 的 stroke 环),`p` 携带圆角/线宽/勾选进度等。
- `shaders/markdown/widget.wgsl`:widget pipeline 入口(vs/fs)。InstanceIn 含 `component: u32` + `params: vec4<f32>×K`(定长 slot,K≈2 → 8 个 f32 起步)。fs 内 `switch component { case 0: md_box(...) ... }`。拼 `base/sdf.wgsl`。
- `scene.rs`:`GpuWidget`/扩 `GpuInstance` —— 新 vertex buffer 布局(`@location`:pos/size/component/params…)。
- `frame.rs`:`FrameWidget { pos, size, component:u32, params:[f32; N], color }`(类比 FrameRect)。
- `seam.rs` + `morph.rs`:`FrameWidget` → `Sample`/`GpuWidget` 编码(component+params 进 flat)。
- `backend.rs`:建 widget pipeline + 每帧画 `widget` 批(绘制顺序:在 rect 之后、glyph 之前 or 之后,按"框在字下/勾在字上"定;**复选框框作背景 → 同 rect 时机**)。
- `wasm/lib.rs`:Sample 构造补 widget 字段。
- **验收**:能发一个测试 `FrameWidget` 画出空心圆角框;`wasm-pack build` + GPU 见框。

**风险**:**vertex buffer 布局改动 = wgpu 严格校验,必须本地 GPU build**(同 Plan 10 相位 3b 风险类)。

## 4. 相位 ③ 任务复选框(content→layout→render 接通)

**做什么**:把 GFM 任务项 `- [ ]` / `- [x]` 渲染成 SDF 复选框 + ✓。

- `content.rs`:`StyleRole` **追加** `TaskUnchecked` / `TaskChecked`(append 在 `TableSep` 之后 → 值 22/23,**不移动**既有数值,避免冲击 shader/`enter_profile_id`)。
- `content.rs` emit_block:检测 jcode 发的任务标记 span(`markdown.rs:559` 发 `"[x] "`/`"[ ] "`)。命中则**替换**为:一个 `TaskChecked`/`TaskUnchecked` 角色的锚点字格(✓ 或空)+ 一个 `Normal` 间隔 —— 只首格承载 task 角色(框锚点)。
- `app.rs` `block_decorations`:遇 `role == TaskUnchecked|TaskChecked` 的字格(世界 x0..x1,y0..y1 已算),发一个 `FrameWidget{ component=box, params=[勾选进度,圆角/线宽] }`(替代原计划的 FrameRect —— **不借通用图元**,见 0026 §1 痛点 2)。
- ✓ 标记:`TaskChecked` 的锚点字渲染 `✓`(走字 atlas)**或**由 `md_box` 内 SDF 直接画对勾(更纯 SDF,可 morph)。**v1 先用字形 ✓**,后续相位可换 SDF 对勾 + 勾选 morph 动画。
- `glyph.wgsl` style_color:加 `TaskChecked(23)` → accent 色(✓);`TaskUnchecked(22)` = 零墨锚点。
- 动画:`enter_profile_id` 可给复选框配 pop;勾选 morph 留相位④/Plan 10 §4。
- **验收**:`- [ ]`/`- [x]` 列表渲染出方框 + 勾选态可辨;reveal 正常;`cargo test` + GPU。

**测试锚点**:content.rs 已有 fixture(`content.rs:1123-1124` 的 `- [ ] / - [x]`)。

## 5. 相位 ④ 推进 markdown(后续,分步)

按"先简单、纯文字的先做、需 SDF/外部的拆步"排:

1. **脚注 `[^x]` / 定义列表 = 纯文字层**(0026 §5 边界):不做组件,确保解析后以普通/弱化文字角色落地、不串版。**仅文字角色 + reveal,无 GPU 图元**。(简单,先做)
2. **`---` HR 效果**:已是 SDF(`Rule` 锚点 → `block_decorations` 画 HR_RULE rect)。可迁到 widget/base 用 SDF 加效果(渐隐线、点划、流光)。**可选增强,非阻塞**。
3. **latex / image / html / link —— 4 个独立步骤**(各自单独排期,非本 plan 落地):
   - **latex**:见 [0013](../decision/0013-math-latex-rendering.md);MathDisplay 已有块类型,需排版引擎决策。
   - **image**:见 [0007](0007-rich-media-embeds.md)/[0022](0022-dom-overlay-layer.md);RGBA 纹理图元 or DOM overlay。
   - **html**:内联/块 HTML 的安全与降级策略(现 `JRole::Html=>Normal` 当文字)。
   - **link**:`Link` 角色已渲染;交互(hover/click)需 DOM overlay 或事件层。

   每步各自 ADR/plan,本 plan 仅登记拆分,不实现。

## 6. 验收总览

- 相位①:像素级零回归,`cargo test` + `wasm-pack build`。
- 相位②:测试 widget 画出空框,GPU 验证布局。
- 相位③:`- [ ]`/`- [x]` 出 SDF 复选框 + ✓,reveal/动画不回归。
- 相位④.1:脚注/定义列表文字正确落地不串版。

## 7. 风险 / 评审

- **GPU 盲改**:相位①无布局改(低险,可单验);相位②③改 vertex 布局,**必须本地 GPU build**,不盲上(同 Plan 10 §5)。
- **⚠ 导数均匀性坑(已踩)**:WebGPU/Tint **禁止在非均匀控制流里调 `fwidth`/`dpdx` 等导数**;naga(`cargo test`)较宽松**不报**,但 Chrome 一画就判 shader 非法 → 整 pass 失败**黑屏**。规避:组件 SDF 的导数一律 top-level **无条件**求,分支用 `select(...)`(表达式,非控制流);多组件分派**不要**用 `switch in.component { md_x() }`(选择子 per-instance flat = 非均匀),改为各组件无条件算 + `select` 选。`cargo test` 过 ≠ 浏览器过,新组件必 GPU 实跑。
- **数值稳定**:`StyleRole` 只 append 不重排,守 0001 契约"数值稳定";shader `style_color`/`enter_profile_id` 加分支不改既有 id。
- **守界**:组件集小而专;脚注/定义列表留文字层;latex/image/html/link 拆步另排,不在本 plan 膨胀。
- **沙箱限制**:Claude 不能编 Rust,只能 `parse_str` 校验 + 逐行审 + tsc;每相位末由用户本地 `cargo test` + `wasm-pack build` + GPU 确认。

> 框架 = 0026;相位①把共享 SDF 抽进 `base/`(纯重构、可单验),②建 markdown widget pipeline(需 GPU build),③接通 SDF 任务复选框,④推进剩余 markdown(文字层先行,latex/image/html/link 拆 4 步另排)。
</content>
