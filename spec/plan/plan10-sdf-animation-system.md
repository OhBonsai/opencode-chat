# Plan 10:SDF 节点动画系统 —— 整体框架 + 分期

- 日期:2026-06-17
- 状态:进行中(框架定稿;相位 1–2 v1 落地于 `glyph.wgsl`;3–5 待排)
- 前置:[0025](../decision/0025-sdf-node-animation-system.md)(决策)、调研 [`animation-system-survey`](../research/animation-system-survey.md);[0016](../decision/0016-streaming-morph-render-model.md)/[0018](../decision/0018-sdf-panel-decoration-primitive.md)/[0019](../decision/0019-reveal-gating-and-choreography.md)/Plan 9/[0020](../decision/0020-content-node-identity-model.md)
- 取向:不追求完整性;SDF 约束下用结构/接口搭动画,守 0025 的能力 / 性能边界。

---

## 1. 整体框架(四层)

```
① 声明层 Profile     node-kind / reveal 风格 → Vec<Anim>(进/出/态变)。声明式。
② 编排层 resolve     Plan 9 resolve + 就绪门 → 每节点具体 Anim + 绝对 start(交错)。CPU,缓存+冻结。
③ 编码层 seam        每实例 → flat(定长 slot ≤4 / 变长 storage,复用 panel params)。
④ 求值层 shader      t=(now-start)/dur → e=curve(id,t) → mix(from,to,e) → SDF 求值(变换/阈值/带宽/blend/mask/color/alpha)。GPU per-instance。
```
统一收编:`glyph.wgsl` fade、`panel.wgsl` AO/reveal、`0016 morph`。接口见 0025 §2(`AnimTarget`/`CurveId`/`Anim`/`Profile`)。

## 2. 分期

- **① 收编 + CurveId(进场 fade 加缓动 + scale)** — v1 在 `glyph.wgsl` **全局 baked**(单文件,无 buffer 布局改动,最小风险):进场 = 缓动 alpha + 绕字心 scale-in,由 `spawn_time`+`fade_ms` 驱动;settled(e=1)无影响;catch-up/瞬显跳过。**本相位落地,见 §3。**
- **② 属性集(scale / translate / threshold / band)** — v1 在 `glyph.wgsl` 加 scale(已)+ translate(rise,常量,默认 0 可开)+ 缓动;threshold/band 为同模式备选(片元)。**本相位 v1 落地(scale/translate/curve);threshold/band 留 profile 开关。**
- **③ per-instance / per-element profile(plumbing)** — 把"全局 baked"升为**每实例动画块**(curve_id + from/to + dur)经 `FrameGlyph`→`GpuInstance`→`glyph.wgsl`,由 reveal profile 按 node-kind/header 产出 → header≠body 可分。**需 GPU build 验证 vertex 布局,未做。**
- **④ 面板 smin / mix 形变** — `panel.wgsl` 解析场 blend(BlendK/MixT);收编 reveal 的 `reveal` mask 进统一求值。
- **⑤ 字↔字 mix morph(后置)** — 两字形同 atlas 同采,成本高。

## 3. 相位 1–2 v1(已落地:`glyph.wgsl`)

**做了什么**:进场动画 = **缓动 alpha(相位1)+ 绕字心 scale-in(相位2)+ translate 钩子**,全在顶点/片元里按 `(time - spawn_time)/fade_ms` 求值;曲线 = ease-out-cubic(snappy 入场)。常量集中在文件顶部便于调:`ANIM_ENTER_SCALE`(0.85)、`ANIM_ENTER_RISE`(0,可开"上浮")。
- **相位1**:alpha 从线性改为**缓动**(`ease_enter(t)`);并把进度 `e` 同时喂给 scale → "收编现有 fade 进统一求值 + 加 curve"。
- **相位2 核心**:scale-in(`mix(ANIM_ENTER_SCALE,1,e)` 绕中心)+ translate 钩子(`(1-e)*ANIM_ENTER_RISE`)。threshold/band 同模式,留相位3 用 profile 开。
- **不改**:`Globals`/instance 布局、Rust 侧、morph、reveal 调度 —— 故零回归风险面、单文件可验证。
- **边界**:本相位是**全局进场 profile**(所有揭示字同一条);**per-element 区分(header≠body)= 相位3**(需 per-instance plumbing)。与 0025 边界一致(先全局,后 per-instance)。

## 4. 验收

- 相位1–2:重放任意 case,字进场为**缓动淡入 + 轻微放大**(非线性硬淡);settled 字不抖;transport 慢放可见;切表格风格/速度不回归。卡口:`wasm-pack build` 后 `panel_shader_is_valid_wgsl` 类 wgsl 校验 + 肉眼。
- 相位3 起:header/body 可分别配 profile;threshold/band/translate 经 profile 开;万级字仍 60fps(GPU 求值 + resolve 缓存)。

## 5. 评审 / 风险

- **GPU 盲改风险**:相位1–2 刻意压在 `glyph.wgsl` 单文件、无 buffer 布局改 → 低风险、可视验证;相位3 的 per-instance 字段改 vertex 布局,**必须本地 GPU build 验证**(wgpu 布局严格),不盲上。
- **性能命脉**:相位3 起须**缓存 resolve + 冻结 settled**(否则每帧重算是上限,见 0025 §4);并修 Plan 9 review #1(NodeSpawn/换行 spawn 使活动 view 不冻结)。
- **morph 协同**:进场 scale 是片元/顶点暂态,叠在 0016 几何 lerp 之上(绕 morph 后中心),二者正交。
- **守界**:小属性集 + 几条曲线;布局动画交 0016;不做 timeline/物理/Hero。

> 框架 = 0025 四层;相位1–2 以"全局 baked 进场(缓动 alpha + scale)"在 `glyph.wgsl` 落地(安全、可验证);相位3 起做 per-instance/per-element profile(需 GPU build)。
