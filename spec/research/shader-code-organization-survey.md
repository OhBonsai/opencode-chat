# Shader 代码组织调研 —— egui / GPUI(Zed)/ Bevy / Flutter Impeller

- 日期:2026-06-18
- 目的:为本系统(infinite-chat,SDF-only,WebGPU/WebGL2)决定 shader 文件组织方式提供外部参照。
- 触发问题:复用 `FrameRect` 是否会耦合?WGSL 能否模块化?markdown 组件该不该各开一个文件?
- 结论先行:本系统形态最接近 **GPUI**(per-primitive SDF + 数据驱动 Scene)。应走"**base 形状函数库 + 按图元/组件分文件**"路线;模块化用 **build 期拼接**(`include_str!` 组合)起步,需求长大再上 `naga_oil`/WESL。

---

## 0. 根约束:为什么"组织方式"会成为一个问题

两条物理事实决定了所有人的选择:

1. **WGSL / GLSL 本身没有 `import`/`#include`。** `wgpu`/WebGPU 的 `createShaderModule` 只吃**整个文件字符串**。要跨文件复用,要么把所有代码塞进巨型单文件,要么在 build 期用预处理器拼起来。([naga_oil README](https://github.com/bevyengine/naga_oil);[WESL spec](https://github.com/wgsl-tooling-wg/wesl-spec))
2. **GPU 的真正编译单元是 pipeline,不是文件。** 一条 pipeline = 一个编译好的 shader module + 一套固定的 vertex/instance buffer 布局。"分几个文件"是**源码组织**问题;"分几条 pipeline / 几个 entry point"才是**运行时**问题。两者可以解耦。

所以业界的分歧不在"要不要复用形状函数"(都要),而在**用什么机制复用**、以及**一条 pipeline 画一种图元还是多种**。下面四个项目正好覆盖了从"最简单"到"最工程化"的整条谱系。

---

## 1. egui —— 单文件单 shader(极简端)

**怎么做**:`egui-wgpu` 全部渲染只有一个 [`egui.wgsl`](https://github.com/emilk/egui/blob/main/crates/egui-wgpu/src/egui.wgsl),一条 pipeline,vertex+fragment 两个 entry。所有 UI(矩形、文字、线、图标)都被上层拍平成**带纹理的三角形**送进来;shader 只做一件事:采样纹理 × 顶点色,加 sRGB/dither 处理。([emilk/egui](https://github.com/emilk/egui))

**为什么**:egui 是 immediate-mode、定位"易用",把复杂度全压在 CPU 端的 tessellation(`epaint`),GPU 端故意保持哑而薄。圆角、裁剪等都在 CPU 算成几何或用 texture。

**好处 / 代价**:零组织成本、零预处理、移植极易(同一个 shader 跑遍所有后端);代价是**放弃了 GPU 端的 SDF 表现力**(圆角/阴影/抗锯齿质量受限于 CPU 几何),不适合"一切皆 SDF"。

> 对本系统的意义:这是我们**不选**的极端 —— 我们要的恰恰是 GPU 端 SDF 表现力。但它证明了"单文件"在小范围内完全够用,不要过早上模块系统。

---

## 2. GPUI / Zed —— per-primitive SDF + 数据驱动 Scene(本系统的镜子)

**怎么做**:GPUI 不做"通用 2D 图形库",而是**为每一种已知图元手写一个专用 shader**:rectangle、shadow、glyph、icon/image、path、underline。每个图元一对 vertex/fragment entry,**instanced draw**(一次 draw 画一批同类图元)。CPU 端把一帧拆成数据驱动的 `Scene { layers: Vec<Layer> }`,每个 `Layer` 是 `{ shadows, rectangles, glyphs, icons, images }` 几个**同类图元的扁平数组**;renderer 按固定顺序(先所有 shadow → 再所有 rect → 再所有 glyph…)批量画。([Zed blog: rendering UI at 120fps](https://zed.dev/blog/videogame))

形状本身全是 **SDF**:`rect_sdf` 用对称性 + 勾股 + `max` 求圆角矩形距离;阴影用 Evan Wallace 的 erf 闭式解;文字只栅化 alpha 通道进 atlas,着色时乘任意色(省得每色存一份)。共享小函数(`to_device_position`/`rect_sdf`/`gaussian`/`erf`)放在同一文件顶部被各 entry 调用。

**文件组织**:历史上 Metal 版是一个 `shaders.metal`;迁到 wgpu 后是 `gpui/src/platform/.../shaders.wgsl`,**一个文件、多个 entry point、共享 helper**。注意 Zed 2025 年从 Blade 换回 wgpu,核心动机是 Blade 在 NVIDIA/Wayland 上崩溃 + 维护停滞,而**同一份 WGSL 能编到 Vulkan/Metal/DX12**(跨平台可移植性,正是当初选 WGSL 的理由)。([UBOS: Zed switches to wgpu](https://ubos.tech/news/zed-editor-switches-graphics-library-from-blade-to-wgpu-for-better-performance/))

**为什么**:Zed 把 UI 当游戏渲染 —— 现实里 2D UI 只分解成"矩形、阴影、文字、图标、图像"几种图元,与其做通用矢量引擎,不如每种图元一个最优 shader,数据驱动地批量喂 GPU。

**好处 / 代价**:每图元 shader 最优、instanced 批量 → 120fps;数据驱动 Scene 让上层(`Element` trait,layout 仿 Flutter:约束下行、尺寸上行)与渲染彻底解耦。代价:新图元 = 新 shader + 新 buffer 布局 + 新 draw,有固定开销(所以图元集合**刻意保持小**)。

> 对本系统的意义:**这就是我们的架构原型。** 我们已有 glyph/panel/rect 三条 pipeline + 数据驱动的 FrameGlyph/FramePanel/FrameRect 扁平数组 + 固定绘制顺序 —— 与 GPUI 的 Scene/Layer 同构。差别只在我们把 SDF 表现力推得更满(panel 参数化、reveal、动画 profile)。**结论:沿 GPUI 的"每图元/组件一个 shader + 共享 SDF helper"走,是被验证过的路。**

---

## 3. Bevy —— naga_oil 模块化 `#import` 组合(最工程化端)

**怎么做**:Bevy 用自研 [`naga_oil`](https://github.com/bevyengine/naga_oil)("naga Organised Integration Library")作为 **WGSL 预处理器**。shader 文件用 `#define_import_path` 声明自己是个模块,别处用 `#import` 引入,还支持 `#define`/条件编译。`naga_oil` 在 build/load 期把这些自定义指令(主要靠 regex 解析)解析、拼接、去重,最后 emit 一个完整 WGSL/naga 模块交给 wgpu。([naga_oil lib.rs](https://lib.rs/crates/naga_oil))

**为什么**:Bevy 的 PBR 材质系统极其庞大,大量 shader 共享一大坨公共代码(光照、坐标变换、bindings)。robtfm 在 [PR #5703](https://github.com/bevyengine/bevy/pull/5703) 重做导入模型,目的就是**作用域控制 + 代码复用**。官方还点出一个性能动机:**多个小 shader 共享大块 import 时,模块化组合比"每个 shader 都重新解析整份源码"更快**。

**好处 / 代价**:真正的模块系统(命名空间、去重、条件编译),适合几十上百个 shader 的大型材质库;代价是引入一个非标准预处理层(自定义语法、regex 解析的脆弱性、调试时行号错位、热重载路径问题见 [issue #16509](https://github.com/bevyengine/bevy/issues/16509))。

> 对本系统的意义:**现在用不上,但它是我们的"长大路径"。** 当 base/markdown 文件多到拼接管不动、或需要条件编译(WebGPU vs WebGL2 分支)时,naga_oil 是成熟答案。

---

## 4. Flutter Impeller —— 离线 AOT 编译 + 预编译图元 shader(另一种"为什么")

**怎么做**:Impeller 放弃 Skia 的运行时 SkSL,改用 GLSL 4.6 作者书写,经内置 **`impellerc`** 编译器在**引擎构建期**离线转成各后端(Metal/Vulkan)的 shader + 反射文件,打包进引擎。运行时已持有全部预编译好的"小而简单"的 shader,按需组合,**不触发驱动端编译**。([Flutter docs: Impeller](https://docs.flutter.dev/perf/impeller);[Shader compilation jank](https://liudonghua123.github.io/flutter_website/perf/shader/))

**为什么**:Skia 把 shader 生成+编译塞进帧工作流,首次遇到某效果时驱动编译可达数百 ms,而一帧只有 16ms → 卡顿(jank)。Impeller 用 **AOT** 把编译挪到 build 期根治。

**好处 / 代价**:消除首帧 shader 卡顿、启动即满速;代价是构建链复杂、shader 集合需预先穷举。

> 对本系统的意义:提醒一个**性能维度** —— shader 编译时机。我们 shader 少、`include_str!` 在 wasm build 期就编进二进制,天然规避运行时编译卡顿;但若将来运行时**动态生成** shader(按组件拼),要警惕首用编译开销,倾向预编译固定集合。

---

## 5. 标准化方向:WESL(WGSL Extended)

社区注意到大家(Bevy、Use.GPU、TypeGPU…)各自造轮子解决"WGSL 没有模块",于是合力做 [**WESL**](https://wesl-lang.dev/)("weasel"):WGSL 超集,加 `import` 语句、`@if` 条件编译、cargo/npm 上的 shader 包(library)。`.wesl` 文件在喂给 `createShaderModule` 前先翻译回标准 WGSL。0.2 已支持 import + 条件编译 + cargo 包。([wesl-spec](https://github.com/wgsl-tooling-wg/wesl-spec);[wesl-rs](https://github.com/wgsl-tooling-wg/wesl-rs))

> 对本系统的意义:**未来可能取代 naga_oil 成为通用标准。** 现在不动,但若以后要模块化,优先看 WESL(标准 > 私有预处理器)。

---

## 6. 谱系对比

| 项目 | 一条 pipeline 画几种图元 | 复用机制 | shader 文件组织 | 主要驱动 | 表现力 |
|---|---|---|---|---|---|
| **egui** | 1 条画全部(拍平成纹理三角) | 不需要(单文件) | 单 `egui.wgsl` | 易用、薄 GPU 层 | 低(CPU 几何) |
| **GPUI/Zed** | 每图元一对 entry(批量 instanced) | 同文件共享 helper fn | 单 `shaders.wgsl` 多 entry | 120fps、数据驱动、跨平台 | 高(per-primitive SDF) |
| **Bevy** | 每材质一条,共享大量公共模块 | `naga_oil` `#import` 预处理 | 多 `.wgsl` 模块 + 组合 | 大型材质库、作用域、复用 | 高(可组合) |
| **Flutter Impeller** | 小而简的预编译图元 shader 组合 | 离线 `impellerc` + 反射 | GLSL 源 → AOT 产物 | 消除 shader 编译卡顿 | 高 |

**三条贯穿的设计共识:**

1. **形状函数必复用,组件/图元不复用。** 所有人都把 `sdRoundBox`/`gaussian`/坐标变换抽成共享小函数;但每种**图元**(rect≠shadow≠glyph)都是独立 entry/shader,不互相借壳 —— 这正是"复用 FrameRect 会耦合"的业界答案:**复用底层 SDF 函数,而非复用上层图元**。
2. **数据驱动 + 批量。** CPU 把一帧描述成图元的扁平数组,GPU instanced 批量画,固定顺序(GPUI 的 Scene/Layer)。
3. **模块系统按规模上。** 小项目单文件(egui);中项目同文件多 entry + 共享 helper(GPUI);大项目才上预处理器(Bevy/naga_oil)或标准(WESL)。**不要为 3 个文件引入预处理器。**

---

## 7. 对本系统(infinite-chat)的结论

我们当前:glyph / panel / rect 三条 pipeline + 数据驱动 FrameGlyph/FramePanel/FrameRect 扁平数组 + 固定绘制顺序 —— **已经是 GPUI 模型**。问题点(用户提出)是 `sd_round_box` 在 rect.wgsl 与 panel.wgsl **重复**,且新增 markdown 组件(复选框等)若借 FrameRect 会耦合。

**采纳:**

1. **抽 `base/` 形状函数库**(无 entry point):`sd_round_box`/`sd_circle`/`sd_seg`/`op_outline`/`smin` 等,去重 rect+panel。对应业界共识①(复用函数不复用图元)。
2. **markdown 组件各开文件**(`markdown/box.wgsl` 复选框、`markdown/slider.wgsl` …),即使初始代码相似 —— 符合 0→1 准则(组件迟早分化,先分文件省将来拆分),也符合 GPUI"每图元独立"的边界。
3. **GPU 单元收敛**:不给每个 markdown 组件单开 pipeline(会爆),而是**一条 "markdown widget" pipeline + component-id 参数**在 shader 内分派到对应组件 SDF。对应共识②(数据驱动批量)。
4. **模块化机制用 build 期 `include_str!` 拼接**(`concat!(base, component)` 再 `create_shader_module`,`naga::parse_str` 照样校验),零新依赖,契合"实现简约"。需求长大再换 `naga_oil` / WESL(共识③)。

目标文件树:

```
shaders/
  base/      sdf.wgsl(共享形状/算子) · glyph.wgsl · rect.wgsl · panel.wgsl
  markdown/  widget.wgsl(入口,按 component-id 分派,拼 base/sdf.wgsl) · box.wgsl · slider.wgsl
```

---

## Sources

- Zed:[Leveraging Rust and the GPU to render UIs at 120 FPS](https://zed.dev/blog/videogame) · [Blade→wgpu 迁移](https://ubos.tech/news/zed-editor-switches-graphics-library-from-blade-to-wgpu-for-better-performance/)
- egui:[emilk/egui](https://github.com/emilk/egui) · [egui.wgsl](https://github.com/emilk/egui/blob/main/crates/egui-wgpu/src/egui.wgsl)
- Bevy:[naga_oil](https://github.com/bevyengine/naga_oil) · [lib.rs/naga_oil](https://lib.rs/crates/naga_oil) · [PR #5703 import model](https://github.com/bevyengine/bevy/pull/5703) · [issue #16509](https://github.com/bevyengine/bevy/issues/16509)
- Flutter Impeller:[docs.flutter.dev/perf/impeller](https://docs.flutter.dev/perf/impeller) · [Shader compilation jank](https://liudonghua123.github.io/flutter_website/perf/shader/)
- WESL:[wesl-lang.dev](https://wesl-lang.dev/) · [wesl-spec](https://github.com/wgsl-tooling-wg/wesl-spec) · [wesl-rs](https://github.com/wgsl-tooling-wg/wesl-rs)
</content>
</invoke>
