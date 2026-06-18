# 0026 — 模块化 shader 组织 + markdown 组件图元

- 日期:2026-06-18
- 状态:已接受(框架);落地见 [Plan 11](../plan/plan11-modular-shader-and-markdown.md)
- 前置:[0011](0011-gpu-text-as-sdf-primitive.md)(字即 SDF)、[0018](0018-sdf-panel-decoration-primitive.md)(参数化 SDF 面板)、[0025](0025-sdf-node-animation-system.md)(SDF 动画系统)、调研 [`shader-code-organization-survey`](../research/shader-code-organization-survey.md)
- 取向:SDF-only 世界不变;在此约束下让 shader 代码**可复用、可扩展、可独立演化**,守住"实现简约 / 0→1 不背兼容"准则。

---

## 1. 背景与问题

当前 render 侧三条 pipeline(`glyph` / `panel` / `rect`)各自一个 `.wgsl`,经 `include_str!` 直接喂 `create_shader_module`。两个具体痛点:

1. **形状函数重复**:`sd_round_box` 在 `rect.wgsl` 与 `panel.wgsl` **各写一份**(将来 `sd_circle`/`sd_seg`/`smin` 还会继续重复)。
2. **markdown 组件无处安放**:要做 SDF 任务复选框、未来的滑块/进度/图标 morph 等"markdown 语义图元",若**借用** `FrameRect`(通用装饰矩形,HR 线/code 底/表格框都发它),组件就和所有装饰共享同一视觉语义与参数槽 —— 想给复选框加专属效果(勾选 morph、按下回弹)会牵动别处,反之亦然(耦合)。

根约束(见调研 §0):**WGSL 无原生 `import`**,`createShaderModule` 只吃整份字符串;且 **GPU 的编译单元是 pipeline 不是文件**("分几个文件"是源码问题,"分几条 pipeline"才是运行时问题,二者解耦)。

## 2. 外部参照(调研结论)

- **egui**:单文件单 shader,UI 拍平成纹理三角 —— 简单但放弃 GPU 端 SDF 表现力(我们不取)。
- **GPUI/Zed**:**每图元一个专用 SDF shader**(rect/shadow/glyph/icon)+ 数据驱动 `Scene{Layer{扁平数组}}` + instanced 批量 + 固定绘制顺序;共享 helper(`rect_sdf`/`gaussian`)放文件顶部。**本系统已与之同构**(glyph/panel/rect + FrameGlyph/FramePanel/FrameRect)。
- **Bevy `naga_oil`**:`#import`/`#define` 预处理器,为大型材质库服务 —— 我们的"长大路径",现在用不上。
- **Flutter Impeller**:离线 AOT 编译根治运行时 shader 卡顿 —— 提醒"编译时机"维度(我们 `include_str!` 在 wasm build 期编进二进制,天然规避)。
- **WESL**:WGSL 超集标准(import/条件编译),未来可能取代 naga_oil。

**三条业界共识**(直接定调本 ADR):

1. **复用底层 SDF 函数,不复用上层图元** —— 共享 `sdRoundBox`,但 rect≠shadow≠glyph 各自独立,不互相借壳。→ 回答痛点 2:复选框**不借** FrameRect。
2. **数据驱动 + instanced 批量 + 固定绘制顺序**。
3. **模块系统按规模上**:小→单文件;中→同文件多 entry + 共享 helper;大→才上预处理器。**不为 3 个文件引 naga_oil**。

## 3. 决策

### 3.1 分层文件树(源码组织)

```
crates/render/src/shaders/
  base/
    sdf.wgsl        # 纯形状/算子函数,无 entry:sd_round_box / sd_circle / sd_seg / op_outline / smin …
    glyph.wgsl      # 文字 pipeline(独立)
    rect.wgsl       # 通用装饰矩形 pipeline
    panel.wgsl      # 参数化面板 pipeline
  markdown/
    widget.wgsl     # markdown 组件 pipeline 入口:按 component-id 分派
    box.wgsl        # fn md_box(uv, p) -> f32   复选/确认框(第一个组件)
    slider.wgsl     # (后续)fn md_slider(...)
```

`base/` 放无 entry-point 的形状函数;每条 pipeline 的 `.wgsl` 与各 markdown 组件 `.wgsl` 在 build 期拼上 `base/sdf.wgsl`。

### 3.2 模块化机制:build 期 `include_str!` 拼接(不引预处理器)

```rust
// 伪代码:每条 pipeline 源 = base + 自身
const SDF: &str = include_str!("shaders/base/sdf.wgsl");
let rect_src   = format!("{SDF}\n{}", include_str!("shaders/base/rect.wgsl"));
let widget_src = format!("{SDF}\n{}\n{}", include_str!("shaders/markdown/box.wgsl"),
                                          include_str!("shaders/markdown/widget.wgsl"));
device.create_shader_module(/* source: widget_src */);
```

- `base/sdf.wgsl` 只含 `fn`,不含 entry/binding → 可被任意 pipeline 前置拼接,无符号冲突。
- 拼接后的字符串照样过 `naga::front::wgsl::parse_str` 校验(现有 `*_shader_is_valid_wgsl` 测试改为校验**拼接结果**)。
- 零新依赖、零自定义语法、行号偏移可控(base 在前,组件行号偏移 = base 行数,固定)。

**升级路径**:当 base 函数互相依赖成图、或需条件编译(WebGPU vs WebGL2 分支)时,换 `naga_oil`(成熟)或 WESL(标准)。现在不动。

### 3.3 markdown 组件 = 一条 widget pipeline + component-id 分派

**不**给每个 markdown 组件单开 pipeline(会爆 pipeline 数)。一条 `markdown/widget.wgsl`,instance 带 `component: u32`(box=0 / slider=1 / …),fragment 内 `switch component` 调对应 `md_box`/`md_slider` 求 SDF。新增组件 = 加一个 `markdown/<name>.wgsl` 的 `fn` + 一个 `switch` 分支 + 一个 id,**不动 buffer 布局、不增 pipeline**。

组件实例数据走一个新的扁平图元数组(类比 `FrameRect`/`FramePanel`):`FrameWidget { pos, size, component, params:[f32; N], color }`,经 seam → `GpuInstance` → widget pipeline。`params` 复用 panel 的"变长 slot"思路(定长 slot 起步,N≈8 够 box/slider)。

### 3.4 组件单文件原则(哪怕初始代码相似)

每个 markdown 组件单独 `.wgsl`,即使第一版代码和别的相似。理由:组件迟早分化(各加专属效果/动画 profile),一开始分文件省将来痛苦拆分,组件边界显式 —— 符合 0→1 准则与 GPUI"每图元独立"边界。

## 4. 后果

**收益**:形状函数单一出处(去重 rect/panel);markdown 组件与通用装饰解耦,可独立加效果;新组件成本低(一个 fn + 一个分支);零新构建依赖;与 GPUI 验证过的模型对齐;可平滑升级到 naga_oil/WESL。

**代价 / 风险**:
- **改了 build 期拼接 + 新增 widget pipeline(新 vertex buffer 布局)** → 必须本地 `wasm-pack build` + GPU 实跑验证(wgpu 布局严格,见 0025/Plan 10 §5 同类风险)。沙箱无 cargo,Claude 只能 `parse_str` 级校验 + 逐行审。
- 一条 widget pipeline 内 `switch` 分支随组件增多变长 —— 可接受(GPU 分支在同 draw 内同 component 时一致,instanced 按 component 排序可进一步省)。
- component 实例的 `params` 定长 slot 上限(N)需预留;超出再走 panel 式变长 storage。

## 5. 边界(不做)

- 不引 naga_oil / WESL(规模没到)。
- 不每组件单开 pipeline。
- 不把 glyph/rect/panel 合并成"超级 shader"(GPUI 的教训:每图元最优 > 通用引擎;保持图元集小而专)。
- markdown 中**纯文字语义**(脚注 `[^x]`、定义列表)**不做组件**,留文字层(见 Plan 11 §4)。

> 一句话:沿 GPUI 模型,把"共享 SDF 函数"抽进 `base/`、把"markdown 语义图元"作为独立组件收进一条 widget pipeline,用 `include_str!` 拼接做最简模块化;复用函数、不复用图元。
</content>
