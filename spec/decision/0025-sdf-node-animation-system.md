# 决策记录 0025:SDF 节点动画系统 —— 控制层声明 × SDF 值层 × GPU per-instance 求值

- 日期:2026-06-17
- 状态:已采纳(模型 + 接口 + 边界定调;落地分相位见 [Plan 10](../plan/plan10-sdf-animation-system.md))
- 前置:[0016](0016-streaming-morph-render-model.md)(几何 morph)、[0018](0018-sdf-panel-decoration-primitive.md)(SDF 面板)、[0019](0019-reveal-gating-and-choreography.md)/Plan 9(reveal)、[0020](0020-content-node-identity-model.md)(节点身份)、[0015](0015-glyph-source-fallback.md)(字形源)、调研 [`animation-system-survey`](../research/animation-system-survey.md) + [`sdf-animation-system`](../research/sdf-animation-system.md)
- 触发:要给"一切皆 SDF"的渲染引入**元素动画系统**。不追求动画完整性,只解决"**SDF 约束下用什么结构/接口搭、能力与性能边界在哪**"。现状的 reveal fade、panel AO/reveal、0016 morph 是零散动画,需统一。

---

## 1. 决策

引入一套 **SDF 节点动画系统**,两层正交、求值在 GPU:

- **控制层(声明式,抄 GPUI/Flutter)**:每 node-kind / reveal 风格声明一组动画,按 **0020 node key** 续身份。**不**用 egui/GPUI 的"Id side-table"(我们是保留模式,天然有身份)。
- **值层(SDF 词汇)**:动画 tween 的是**距离场的参数**(变换 / 阈值 / 带宽 / outline / blend / mask / color / alpha),**不是顶点 + 合成 alpha**(那是 mesh/DOM 思路)。
- **求值(GPU per-instance)**:`t=(now-start)/dur → e=curve(id,t) → param=mix(from,to,e)`,在 shader 内作用于 SDF 求值。**不**像 egui/GPUI/Flutter 每帧 CPU 重建。
- **统一**:`glyph.wgsl` fade、`panel.wgsl` AO/`reveal`、`0016 morph` 收编为本系统在不同属性上的特例。

## 2. 接口(语言中立)

```
AnimTarget = Alpha | Scale | Translate | Rotate | Threshold | Band
           | Outline | Glow | BlendK | MixT | MaskProgress | Tint
CurveId    = Linear | SmoothStep | EaseIn | EaseOut | EaseInOut | Gain | ExpImpulse | CubicPulse
Anim       = { target, from, to, start_ms, dur_ms, curve, repeat }
Profile    = node-kind / reveal 风格 → Vec<Anim>   // 声明层(像 Flutter AnimatedX)
```
- **源类型约束**(SDF 直接进类型):**字(采样场)**合法子集 = Alpha/Scale/Translate/Rotate/Threshold/Band/Outline/MaskProgress/Tint(MixT 需两字形同 atlas,后置);**面板(解析场)**超集 = 再加 BlendK/Glow/smin 形变。
- **编排** = [Plan 9 `resolve`](../plan/plan9-recursive-reveal-tree.md) 给的 `start`(交错/骨架先行);per-element 动画 = 上面 Anim。"何时" 与 "怎么动" 分离。
- **编码**:定长(≤4 条/实例)走 instance 属性;变长走 storage(复用 panel params 通道)。
- **契约位置**:动画属 render/policy 层(挂 0020 节点),**不污染 content/layout**;几何变化仍走 measure/relayout + 0016。R8:`start` 走注入时钟、曲线纯函数 → seek/重放稳。

## 3. 能力边界(能 / 不能)

**能**:每元素进/出/态变(SDF 属性集)× 交错编排 × 任意缩放仍锐的 transform × SDF-native 多味淡入(alpha/阈值墨入/带宽模糊)/outline·glow draw-on/面板 smin·mix 形变/mask wipe × 确定可 seek。

**不做(v1)**:通用关键帧时间线(一条 Anim = 单段 from→to,多段串 stage);有状态物理/弹簧(要弹性用**解析阻尼当一条 curve**,不引入状态);字↔字 shape morph(atlas 成本,后置);布局驱动动画(归 0016 几何);跨元素约束/FLIP/Hero(超 0016 reflow);3D/raymarch(归 [0024](0024-3d-camera-and-raymarch-sdf.md));每元素无限并发 Anim(slot 有界)。

## 4. 性能边界(代价 / 上限)

**核心命题**:**声明一次 → GPU 按时间求值 → settled 块 CPU-idle**。动画成本 ≈ 现有渲染成本,可动**万级**可见字。
- GPU 求值近免费(几 mul/add + 一次 curve)。
- 代价驱动:**fill-rate**(glow/band 加宽、scale>1 暂态 overdraw)、**多采样**(字 morph 2× → 后置)、**带宽**(slot 数,N≤4 控上限,超走 storage)。
- **真正瓶颈在 CPU 端 `resolve`**:须**缓存 + 冻结**(仅内容/门变化时重算;settled 块不再碰 → GPU 独自按时间推进)。退回"每帧 CPU 重 resolve/重建"即上限。
- 确定性:时间走注入时钟;随机(微定时)必 seeded(R8/R9)。

## 5. 备选(不选)

- **每元素 CPU 每帧重建**(egui/GPUI/Flutter 模型):海量字扛不住 → 改 GPU per-instance 求值。
- **mesh/顶点 + 合成 alpha 动画**:丢 SDF 红利(任意缩放锐、形变/溶解/draw-on)→ 用 SDF 场参数。
- **引整套动画引擎/timeline 编辑器**:违"够用即止" → 固定小属性集 + 几条曲线(覆盖 90%)。

## 6. 落地

见 [Plan 10](../plan/plan10-sdf-animation-system.md):① 收编现有 + CurveId(进场 fade 加缓动 + scale,先全局 baked,glyph.wgsl 单文件)→ ② 属性集(scale/translate/threshold/band)→ ③ per-instance/per-element profile(plumbing)→ ④ 面板 smin 形变 → ⑤ 字 mix morph(后置)。

> 一句话:**控制层声明式(按 0020 身份)、值层 SDF 场参数(IQ/LYGIA 叶子)、求值 GPU per-instance;能力 = 固定 SDF 属性集上的单段进/出/态变 + 交错编排(确定可 seek),不做时间线/物理/布局动画;性能 = 声明一次 + GPU 按时间求值 + settled CPU-idle 可达万级,瓶颈在 resolve 缓存与冻结。**
